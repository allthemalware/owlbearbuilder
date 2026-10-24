// SPDX-License-Identifier: GPL-3.0-or-later

use obb_core::character::{Save, Skill};
use obb_core::conditions::Condition;
use obb_core::dice::Rng;
use obb_core::encounter::{Combatant, EncounterTracker};
use obb_core::import::{import_character_auto, merge_update};
use obb_core::spells::spellbook;

const FIGHTER_4: &str = include_str!("fixtures/pb_fighter_lvl4.json");
const FIGHTER_5: &str = include_str!("fixtures/pb_fighter_lvl5.json");
const WIZARD_5: &str = include_str!("fixtures/pb_wizard_lvl5.json");

fn check_character(raw: &str, hp: i32, ac: i32, perception: i32, saves: [i32; 3], class_dc: i32) {
    let sheet = import_character_auto(raw).unwrap();
    // HP is ancestry HP plus level times class HP and Constitution modifier.
    assert_eq!(sheet.max_hp, hp);
    // AC follows the export's total; the shield bonus is tracked separately.
    assert_eq!(sheet.effective_ac(), ac);
    // Perception is rank bonus plus level plus Wisdom modifier.
    assert_eq!(sheet.perception_modifier(), perception);
    // Saves are proficiency plus level plus their linked ability modifier.
    assert_eq!(
        [
            sheet.save_modifier(Save::Fortitude),
            sheet.save_modifier(Save::Reflex),
            sheet.save_modifier(Save::Will),
        ],
        saves
    );
    // Class DC is 10 plus rank bonus, level and key ability.
    assert_eq!(sheet.class_dc(), class_dc);

    if sheet.class == "Fighter" {
        // Athletics and Intimidation are expert: +4 rank, level and their ability modifier.
        assert_eq!(sheet.skill_modifier("Athletics"), Some(4 + sheet.level + 4));
        assert_eq!(sheet.skill_modifier("Intimidation"), Some(4 + sheet.level));
        let expected_attacks: &[[i32; 3]] = if sheet.level == 4 {
            &[[14, 9, 4], [14, 10, 6]]
        } else {
            &[[16, 11, 6], [15, 11, 7]]
        };
        for (index, strike) in sheet.strikes.iter().enumerate() {
            // MAP is -5/-10 normally and -4/-8 for the agile shortsword.
            assert_eq!(
                [
                    sheet.strike_attack_modifier(index, 0).unwrap(),
                    sheet.strike_attack_modifier(index, 1).unwrap(),
                    sheet.strike_attack_modifier(index, 2).unwrap(),
                ],
                expected_attacks[index]
            );
            let expected_dice = if strike.name.contains("Striking") {
                2
            } else {
                1
            };
            assert_eq!(strike.damage.dice_count, expected_dice);
            let expected_sides = if strike.name.contains("Shortsword") {
                6
            } else {
                8
            };
            assert_eq!(strike.damage.die_sides, expected_sides);
        }
    } else {
        // Arcana is expert; Society is trained. Their modifier abilities are Int and Int.
        assert_eq!(sheet.skill_modifier("Arcana"), Some(4 + sheet.level + 4));
        assert_eq!(sheet.skill_modifier("Society"), Some(2 + sheet.level + 4));
    }
}

#[test]
fn imports_golden_sheets_and_derived_values() {
    check_character(FIGHTER_4, 56, 18, 9, [10, 10, 7], 20);
    check_character(FIGHTER_5, 68, 19, 10, [11, 11, 8], 21);
    check_character(WIZARD_5, 41, 19, 9, [8, 9, 11], 21);
}

#[test]
fn initiative_uses_the_seeded_d20_and_perception() {
    let sheet = import_character_auto(FIGHTER_5).unwrap();
    let mut tracker = EncounterTracker::new();
    tracker
        .add_combatant(Combatant::new("amiri", &sheet.id, &sheet.name, true))
        .unwrap();
    let (total, roll, _) = tracker
        .roll_initiative_for_character("amiri", &sheet, "perception", &mut Rng::from_seed(42))
        .unwrap();
    assert_eq!(
        total,
        i32::from(roll.natural_d20.unwrap()) + sheet.perception_modifier()
    );
}

#[test]
fn wizard_spellbook_heightens_prepared_spells() {
    let sheet = import_character_auto(WIZARD_5).unwrap();
    let spells = spellbook(&sheet);
    let find = |name: &str| spells.iter().find(|spell| spell.name == name).unwrap();
    let fireball = find("Fireball").details.as_ref().unwrap();
    assert_eq!(fireball.area.as_ref().unwrap().label(), "20-foot burst");
    assert_eq!(fireball.damage_expr.as_deref(), Some("6d6"));
    let breathe = find("Breathe Fire").details.as_ref().unwrap();
    assert_eq!(breathe.area.as_ref().unwrap().label(), "15-foot cone");
    assert_eq!(breathe.damage_expr.as_deref(), Some("2d6"));
    assert_eq!(
        find("Electric Arc")
            .details
            .as_ref()
            .unwrap()
            .damage_expr
            .as_deref(),
        Some("4d4")
    );
}

#[test]
fn level_up_merge_reports_build_changes_and_keeps_table_state() {
    let mut old = import_character_auto(FIGHTER_4).unwrap();
    old.current_hp = 30;
    old.temp_hp = 4;
    old.hero_points = 2;
    old.conditions = vec![Condition::Frightened(2)];
    old.linked_token_id = Some("tok-amiri".into());
    let changes = merge_update(&mut old, import_character_auto(FIGHTER_5).unwrap());

    assert!(
        changes.contains(&"Level: 4 -> 5".to_string()),
        "{changes:?}"
    );
    assert!(
        changes.contains(&"Max HP: 56 -> 68".to_string()),
        "{changes:?}"
    );
    assert!(
        changes.contains(&"New Strike: +1 Striking Longsword".to_string()),
        "{changes:?}"
    );
    assert_eq!(old.current_hp, 42);
    assert_eq!(old.conditions, vec![Condition::Frightened(2)]);
    assert_eq!(old.linked_token_id.as_deref(), Some("tok-amiri"));
    assert_eq!((old.temp_hp, old.hero_points), (4, 2));
    assert_eq!(
        old.skill_rank("Athletics"),
        Some(obb_core::character::ProfRank::Expert)
    );
    assert_eq!(
        old.skill_rank(Skill::Athletics.id()),
        Some(obb_core::character::ProfRank::Expert)
    );
}
