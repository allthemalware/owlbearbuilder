// SPDX-License-Identifier: GPL-3.0-or-later

use obb_core::character::*;
use obb_core::conditions::*;
use obb_core::dice::StrikeDamage;
use obb_core::json::JsonValue;

/// Level 5 fighter: Str +4, Dex +2, Con +2, Int +0, Wis +1, Cha +0.
fn fighter() -> CharacterSheet {
    let mut c = CharacterSheet::new("Valeros the Bold", 5);
    c.abilities = [4, 2, 2, 0, 1, 0];
    c.key_ability = Ability::Str;
    c.max_hp = 76;
    c.current_hp = 76;
    c.ac_base = 22;
    c.shield_bonus = 2;
    c.perception_rank = ProfRank::Expert;
    c.fort_rank = ProfRank::Expert;
    c.reflex_rank = ProfRank::Expert;
    c.will_rank = ProfRank::Trained;
    c.class_dc_rank = ProfRank::Trained;
    c.skill_ranks[Skill::Athletics as usize] = ProfRank::Expert;
    c.skill_ranks[Skill::Intimidation as usize] = ProfRank::Trained;
    c.lores.push(Lore {
        name: "Warfare".into(),
        rank: ProfRank::Trained,
    });
    c.item_bonuses.insert("athletics".into(), 1);
    c.strikes.push(WeaponStrike {
        name: "+1 Striking Longsword".into(),
        attack_bonus: 16,
        attack_ability: Ability::Str,
        is_ranged: false,
        damage: StrikeDamage {
            dice_count: 2,
            die_sides: 8,
            flat_bonus: 4,
            deadly: None,
            fatal: None,
        },
        damage_type: "slashing".into(),
        extra_damage: vec![],
        traits: vec!["versatile p".into()],
    });
    c.strikes.push(WeaponStrike {
        name: "Shortsword".into(),
        attack_bonus: 15,
        attack_ability: Ability::Dex,
        is_ranged: false,
        damage: StrikeDamage {
            dice_count: 1,
            die_sides: 6,
            flat_bonus: 4,
            deadly: None,
            fatal: None,
        },
        damage_type: "piercing".into(),
        extra_damage: vec![],
        traits: vec!["agile".into(), "finesse".into(), "versatile s".into()],
    });
    c.strikes.push(WeaponStrike {
        name: "Javelin".into(),
        attack_bonus: 13,
        attack_ability: Ability::Dex,
        is_ranged: true,
        damage: StrikeDamage {
            dice_count: 1,
            die_sides: 6,
            flat_bonus: 4,
            deadly: None,
            fatal: None,
        },
        damage_type: "piercing".into(),
        extra_damage: vec![],
        traits: vec!["thrown 30 ft".into()],
    });
    c.strikes.push(WeaponStrike {
        name: "Longbow".into(),
        attack_bonus: 13,
        attack_ability: Ability::Dex,
        is_ranged: true,
        damage: StrikeDamage {
            dice_count: 1,
            die_sides: 8,
            flat_bonus: 0,
            deadly: Some(10),
            fatal: None,
        },
        damage_type: "piercing".into(),
        extra_damage: vec![],
        traits: vec!["deadly d10".into(), "volley 30 ft".into()],
    });
    c
}

fn wizard() -> CharacterSheet {
    let mut c = CharacterSheet::new("Ezren", 3);
    c.abilities = [0, 2, 1, 4, 1, 0];
    c.key_ability = Ability::Int;
    c.spellcasting.push(SpellcastingEntry {
        name: "Wizard".into(),
        tradition: "arcane".into(),
        kind: "prepared".into(),
        ability: Ability::Int,
        rank: ProfRank::Trained,
        focus_points: 1,
        spells: vec![
            (0, vec!["Electric Arc".into()]),
            (2, vec!["Fireball".into()]),
        ],
    });
    c
}

#[test]
fn ranks_and_names() {
    assert_eq!(ProfRank::from_bonus(4), ProfRank::Expert);
    assert_eq!(ProfRank::from_bonus(5), ProfRank::Expert);
    assert_eq!(ProfRank::from_bonus(-2), ProfRank::Untrained);
    assert_eq!(ProfRank::from_bonus(12), ProfRank::Legendary);
    assert_eq!(ProfRank::Master.bonus(), 6);
    assert_eq!(ProfRank::parse("LEGENDARY"), Some(ProfRank::Legendary));
    assert_eq!(ProfRank::Expert.letter(), 'E');
    assert_eq!(proficiency_bonus(ProfRank::Untrained, 9), 0);
    assert_eq!(proficiency_bonus(ProfRank::Master, 9), 15);
    assert_eq!(Skill::parse("Thievery"), Some(Skill::Thievery));
    assert_eq!(Skill::Medicine.ability(), Ability::Wis);
    assert_eq!(Skill::Crafting.ability(), Ability::Int);
    assert_eq!(Save::parse("fort"), Some(Save::Fortitude));
    assert_eq!(Ability::parse("Charisma"), Some(Ability::Cha));
    assert_eq!(slugify("Valeros the Bold!"), "valeros-the-bold");
    assert_eq!(slugify("  --  "), "character");
}

#[test]
fn new_sheet_defaults() {
    let c = CharacterSheet::new("Kyra", 1);
    assert_eq!(c.id, "kyra");
    assert_eq!(c.size, "Medium");
    assert_eq!(c.speed, 25);
    assert_eq!(c.hero_points, 1);
    assert_eq!(c.ac_base, 10);
    assert!(c.skill_ranks.iter().all(|r| *r == ProfRank::Untrained));
}

#[test]
fn base_modifiers() {
    let c = fighter();
    assert_eq!(c.perception_modifier(), 10);
    assert_eq!(c.save_modifier(Save::Fortitude), 11);
    assert_eq!(c.save_modifier(Save::Reflex), 11);
    assert_eq!(c.save_modifier(Save::Will), 8);
    assert_eq!(c.skill_modifier("athletics"), Some(14));
    assert_eq!(c.skill_modifier("Intimidation"), Some(7));
    assert_eq!(c.skill_modifier("acrobatics"), Some(2));
    assert_eq!(c.skill_modifier("Warfare"), Some(7));
    assert_eq!(c.skill_modifier("warfare lore"), Some(7));
    assert_eq!(c.skill_modifier("Sailing Lore"), None);
    assert_eq!(c.skill_modifier("juggling"), None);
    assert_eq!(c.skill_rank("Athletics"), Some(ProfRank::Expert));
    assert_eq!(c.class_dc(), 21);
    assert_eq!(c.effective_ac(), 22);
}

#[test]
fn untrained_improvisation() {
    let mut c = fighter();
    c.feats.push("Untrained Improvisation".into());
    assert!(c.has_feat("untrained improvisation"));
    assert_eq!(c.skill_modifier("acrobatics"), Some(2 + 2));
    assert_eq!(c.assurance_total("acrobatics"), Some(10 + 2));
    c.level = 7;
    assert_eq!(c.skill_modifier("acrobatics"), Some(2 + 7));
    assert_eq!(c.skill_modifier("athletics"), Some(7 + 4 + 4 + 1));
}

#[test]
fn status_penalties_do_not_stack() {
    let mut c = fighter();
    c.conditions = vec![Condition::Frightened(2), Condition::Sickened(1)];
    assert_eq!(c.save_modifier(Save::Will), 6);
    assert_eq!(c.skill_modifier("athletics"), Some(12));
    c.conditions = vec![Condition::Frightened(1), Condition::Clumsy(3)];
    assert_eq!(c.save_modifier(Save::Reflex), 8);
    assert_eq!(c.save_modifier(Save::Fortitude), 10);
    assert_eq!(status_penalty(&c.conditions, Ability::Dex), -3);
    assert_eq!(status_penalty(&c.conditions, Ability::Str), -1);
    assert_eq!(status_penalty(&[], Ability::Cha), 0);
}

#[test]
fn armor_class_with_conditions_and_shield() {
    let mut c = fighter();
    c.conditions = vec![
        Condition::OffGuard,
        Condition::Frightened(2),
        Condition::Clumsy(1),
    ];
    assert_eq!(c.effective_ac(), 22 - 2 - 2);
    c.raise_shield();
    assert_eq!(c.effective_ac(), 22 - 2 - 2 + 2);
    assert_eq!(
        ac_circumstance(&[Condition::OffGuard, Condition::ShieldRaised(2)]),
        0
    );
    let mut no_shield = fighter();
    no_shield.shield_bonus = 0;
    no_shield.raise_shield();
    assert!(no_shield.conditions.is_empty());
}

#[test]
fn enfeebled_hits_strength_attacks_and_damage() {
    let mut c = fighter();
    c.conditions = vec![Condition::Enfeebled(2)];
    assert_eq!(c.strike_attack_modifier(0, 0), Some(14));
    assert_eq!(c.strike_attack_modifier(0, 1), Some(9));
    assert_eq!(c.strike_attack_modifier(0, 2), Some(4));
    assert_eq!(c.strike_damage(0).unwrap().flat_bonus, 2);
    // Finesse Strike rolled with Dex: Enfeebled spares the attack but not the melee damage.
    assert_eq!(c.strike_attack_modifier(1, 1), Some(11));
    assert_eq!(c.strike_damage(1).unwrap().flat_bonus, 2);
    // Thrown weapons add Strength to damage; bows don't.
    assert_eq!(c.strike_damage(2).unwrap().flat_bonus, 2);
    assert_eq!(c.strike_damage(3).unwrap().flat_bonus, 0);
    assert_eq!(c.strike_damage(3).unwrap().deadly, Some(10));
    assert_eq!(c.strike_attack_modifier(9, 0), None);
    assert!(c.strikes[1].is_agile());
    assert!(!c.strikes[0].is_agile());
    // Frightened and Sickened hit the attack roll but not damage, which isn't a check.
    c.conditions = vec![
        Condition::Frightened(2),
        Condition::Sickened(1),
        Condition::Enfeebled(1),
    ];
    assert_eq!(c.strike_attack_modifier(0, 0), Some(14));
    assert_eq!(c.strike_damage(0).unwrap().flat_bonus, 3);
}

#[test]
fn clumsy_hits_dex_attacks() {
    let mut c = fighter();
    c.conditions = vec![Condition::Clumsy(1)];
    assert_eq!(c.strike_attack_modifier(0, 0), Some(16));
    assert_eq!(c.strike_attack_modifier(1, 2), Some(15 - 1 - 8));
    assert_eq!(c.strike_attack_modifier(3, 0), Some(12));
}

#[test]
fn stupefied_hits_mental_checks_and_spells() {
    let mut c = fighter();
    c.conditions = vec![Condition::Stupefied(3)];
    assert_eq!(c.perception_modifier(), 7);
    assert_eq!(c.save_modifier(Save::Will), 5);
    assert_eq!(c.save_modifier(Save::Fortitude), 11);
    let mut w = wizard();
    assert_eq!(w.spell_dc(0), Some(19));
    assert_eq!(w.spell_attack_modifier(0), Some(9));
    w.conditions = vec![Condition::Stupefied(1)];
    assert_eq!(w.spell_dc(0), Some(18));
    assert_eq!(w.spell_attack_modifier(0), Some(8));
    assert_eq!(w.spell_dc(1), None);
}

#[test]
fn assurance_ignores_everything_but_proficiency() {
    let mut c = fighter();
    c.conditions = vec![Condition::Frightened(2), Condition::Enfeebled(1)];
    assert_eq!(c.assurance_total("Athletics"), Some(10 + 5 + 4));
    assert_eq!(c.assurance_total("acrobatics"), Some(10));
    assert_eq!(c.assurance_total("Warfare Lore"), Some(10 + 5 + 2));
    assert_eq!(c.assurance_total("nothing"), None);
}

#[test]
fn drained_lowers_fortitude_and_max_hp() {
    let mut c = fighter();
    c.conditions = vec![Condition::Drained(2)];
    assert_eq!(c.save_modifier(Save::Fortitude), 9);
    assert_eq!(c.effective_max_hp(), 76 - 10);
    c.current_hp = 50;
    c.heal(100);
    assert_eq!(c.current_hp, 66);
}

#[test]
fn hit_points() {
    let mut c = fighter();
    c.current_hp = 30;
    c.gain_temp_hp(5);
    c.gain_temp_hp(3);
    assert_eq!(c.temp_hp, 5);
    c.apply_damage(8);
    assert_eq!((c.temp_hp, c.current_hp), (0, 27));
    c.apply_damage(-4);
    assert_eq!(c.current_hp, 27);
    c.apply_damage(100);
    assert_eq!(c.current_hp, 0);
    c.heal(-3);
    assert_eq!(c.current_hp, 0);
}

#[test]
fn valued_conditions_keep_the_higher_value() {
    let mut list = Vec::new();
    gain_condition(&mut list, Condition::Frightened(1));
    gain_condition(&mut list, Condition::Frightened(3));
    gain_condition(&mut list, Condition::Frightened(2));
    gain_condition(&mut list, Condition::OffGuard);
    gain_condition(&mut list, Condition::OffGuard);
    gain_condition(&mut list, Condition::Sickened(1));
    assert_eq!(
        list,
        vec![
            Condition::Frightened(3),
            Condition::OffGuard,
            Condition::Sickened(1)
        ]
    );
    set_condition(&mut list, "frightened", Some(1)).unwrap();
    assert_eq!(list[0], Condition::Frightened(1));
    set_condition(&mut list, "Sickened", Some(0)).unwrap();
    assert!(!list.contains(&Condition::Sickened(1)));
    remove_condition(&mut list, "flat-footed");
    assert_eq!(list, vec![Condition::Frightened(1)]);
    assert!(set_condition(&mut list, "clumsy", None).is_err());
}

#[test]
fn turn_ticks() {
    let mut list = vec![
        Condition::Frightened(2),
        Condition::Sickened(1),
        Condition::ShieldRaised(2),
    ];
    tick_end_of_turn(&mut list);
    assert_eq!(
        list,
        vec![
            Condition::Frightened(1),
            Condition::Sickened(1),
            Condition::ShieldRaised(2)
        ]
    );
    tick_end_of_turn(&mut list);
    assert_eq!(
        list,
        vec![Condition::Sickened(1), Condition::ShieldRaised(2)]
    );
    tick_start_of_turn(&mut list);
    assert_eq!(list, vec![Condition::Sickened(1)]);
}

#[test]
fn condition_parsing_and_json() {
    assert_eq!(
        Condition::parse("Flat-Footed", None),
        Ok(Condition::OffGuard)
    );
    assert_eq!(
        Condition::parse("raised shield", None),
        Ok(Condition::ShieldRaised(2))
    );
    assert_eq!(
        Condition::parse("Stupefied", Some(2)),
        Ok(Condition::Stupefied(2))
    );
    assert!(Condition::parse("drained", Some(0)).is_err());
    assert!(Condition::parse("", None).is_err());
    let grabbed = Condition::parse(" Grabbed ", None).unwrap();
    assert_eq!(
        grabbed,
        Condition::Other {
            name: "Grabbed".into(),
            value: None
        }
    );
    assert_eq!(Condition::Frightened(2).label(), "Frightened 2");
    assert_eq!(Condition::OffGuard.label(), "Off-Guard");
    assert_eq!(Condition::ShieldRaised(2).label(), "Shield Raised (+2)");
    assert_eq!(
        Condition::Frightened(2).to_json().to_json_string(),
        r#"{"name":"frightened","value":2}"#
    );
    assert_eq!(
        Condition::OffGuard.to_json().to_json_string(),
        r#"{"name":"off-guard"}"#
    );
    for c in [
        Condition::Clumsy(1),
        Condition::OffGuard,
        Condition::ShieldRaised(3),
        Condition::Other {
            name: "Persistent Fire".into(),
            value: Some(2),
        },
    ] {
        assert_eq!(Condition::from_json(&c.to_json()), Ok(c));
    }
}

#[test]
fn sheet_json_round_trip() {
    let mut c = fighter();
    c.conditions = vec![Condition::Frightened(1), Condition::OffGuard];
    c.linked_token_id = Some("tok-123".into());
    c.feats.push("Power Attack".into());
    let json = c.to_json();
    assert_eq!(json.get_str("format"), Some("owlbearbuilder-character"));
    assert_eq!(json.get_i64("version"), Some(1));
    assert_eq!(json.get("abilities").unwrap().get_i64("str"), Some(4));
    assert_eq!(
        json.get("skill_ranks").unwrap().get_str("athletics"),
        Some("expert")
    );
    let back =
        CharacterSheet::from_json(&JsonValue::parse(&json.to_json_pretty()).unwrap()).unwrap();
    assert_eq!(back, c);
    let mut w = wizard();
    w.linked_token_id = None;
    assert_eq!(CharacterSheet::from_json(&w.to_json()).unwrap(), w);
}

#[test]
fn sheet_json_defaults_and_errors() {
    let minimal = JsonValue::parse(r#"{"name": "Merisiel", "level": 2}"#).unwrap();
    assert_eq!(
        CharacterSheet::from_json(&minimal).unwrap(),
        CharacterSheet::new("Merisiel", 2)
    );
    let missing = JsonValue::parse(r#"{"level": 2}"#).unwrap();
    assert!(CharacterSheet::from_json(&missing).is_err());
    let wrong = JsonValue::parse(r#"{"name": "M", "level": 2, "speed": "fast"}"#).unwrap();
    assert!(CharacterSheet::from_json(&wrong)
        .unwrap_err()
        .contains("speed"));
}
