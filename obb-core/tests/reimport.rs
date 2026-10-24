// SPDX-License-Identifier: GPL-3.0-or-later

use obb_core::character::{Ability, CharacterSheet, Lore, ProfRank, Skill, SpellcastingEntry};
use obb_core::conditions::Condition;
use obb_core::import::*;

fn level4() -> CharacterSheet {
    let mut c = CharacterSheet::new("Valeros", 4);
    c.abilities = [4, 2, 2, 0, 1, 0];
    c.max_hp = 52;
    c.current_hp = 30;
    c.temp_hp = 4;
    c.hero_points = 2;
    c.ac_base = 21;
    c.perception_rank = ProfRank::Expert;
    c.skill_ranks[Skill::Athletics as usize] = ProfRank::Expert;
    c.lores.push(Lore {
        name: "Warfare".into(),
        rank: ProfRank::Trained,
    });
    c.feats.push("Power Attack".into());
    c.conditions = vec![Condition::Frightened(2)];
    c.linked_token_id = Some("tok-1".into());
    c.player_name = "Sam".into();
    c
}

fn level5() -> CharacterSheet {
    let mut c = level4();
    c.id = "valeros-renamed".into();
    c.level = 5;
    c.max_hp = 64;
    c.current_hp = 64;
    c.temp_hp = 0;
    c.hero_points = 1;
    c.ac_base = 22;
    c.skill_ranks[Skill::Athletics as usize] = ProfRank::Master;
    c.lores.push(Lore {
        name: "Sailing".into(),
        rank: ProfRank::Trained,
    });
    c.feats.push("Sudden Charge".into());
    c.conditions.clear();
    c.linked_token_id = None;
    c.player_name = String::new();
    c.speed = 30;
    c.spellcasting.push(SpellcastingEntry {
        name: "Innate".into(),
        tradition: "arcane".into(),
        kind: "innate".into(),
        ability: Ability::Cha,
        rank: ProfRank::Trained,
        focus_points: 0,
        spells: vec![(0, vec!["Daze".into()])],
    });
    c
}

#[test]
fn merge_keeps_table_state_and_lists_changes() {
    let mut sheet = level4();
    let changes = merge_update(&mut sheet, level5());
    assert_eq!(
        &changes[..4],
        [
            "Level: 4 -> 5",
            "Max HP: 52 -> 64",
            "AC: 21 -> 22",
            "Perception: +9 -> +10"
        ]
    );
    assert!(
        changes.contains(&"Athletics: +12 -> +15 (Master)".to_string()),
        "{changes:?}"
    );
    assert!(
        changes.contains(&"Warfare Lore: +6 -> +7".to_string()),
        "{changes:?}"
    );
    assert!(
        changes.contains(&"New Lore: Sailing Lore +7 (Trained)".to_string()),
        "{changes:?}"
    );
    assert!(changes.contains(&"Speed: 25 -> 30".to_string()));
    assert!(changes.contains(&"New feat: Sudden Charge".to_string()));
    assert!(changes.contains(&"New spell: Daze".to_string()));
    assert!(
        !changes.iter().any(|c| c.starts_with("Fortitude")),
        "untrained saves move with level only if proficient: {changes:?}"
    );
    assert_eq!(sheet.id, "valeros");
    assert_eq!((sheet.level, sheet.max_hp, sheet.current_hp), (5, 64, 42));
    assert_eq!((sheet.temp_hp, sheet.hero_points), (4, 2));
    assert_eq!(sheet.conditions, vec![Condition::Frightened(2)]);
    assert_eq!(sheet.linked_token_id.as_deref(), Some("tok-1"));
    assert_eq!(sheet.player_name, "Sam");
    assert_eq!(
        sheet.skill_ranks[Skill::Athletics as usize],
        ProfRank::Master
    );
}

#[test]
fn merge_caps_hp_when_max_drops() {
    let mut sheet = level4();
    sheet.current_hp = 50;
    let mut smaller = level4();
    smaller.max_hp = 40;
    let changes = merge_update(&mut sheet, smaller);
    assert_eq!(changes, ["Max HP: 52 -> 40"]);
    assert_eq!(sheet.current_hp, 40);
    let mut same = level4();
    assert!(merge_update(&mut same, level4()).is_empty());
}

#[test]
fn auto_detects_formats() {
    let pb = r#"{"success": true, "build": {"name": "Kyra", "level": 2, "class": "Cleric"}}"#;
    assert_eq!(import_character_auto(pb).unwrap().class, "Cleric");
    let canonical = level4().to_json().to_json_string();
    assert_eq!(import_character_auto(&canonical).unwrap(), level4());
    let bare = r#"{"name": "Kyra", "level": 2, "abilities": {"str": 10}, "proficiencies": {}}"#;
    assert_eq!(import_character_auto(bare).unwrap().name, "Kyra");
    let err = import_character_auto(r#"{"hello": 1}"#).unwrap_err();
    assert!(err.contains("Pathbuilder"), "{err}");
    assert!(import_character_auto("nope").is_err());
}
