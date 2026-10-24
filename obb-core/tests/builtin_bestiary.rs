// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashSet;

use obb_core::bestiary_data::builtin_bestiary;

#[test]
fn six_starter_creatures_parse() {
    let all = builtin_bestiary();
    let names: Vec<&str> = all.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Goblin Warrior",
            "Orc Brute",
            "Skeleton Guard",
            "Kobold Scout",
            "Giant Rat",
            "Ogre Warrior"
        ]
    );
    let ids: HashSet<&str> = all.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids.len(), 6);
    for m in &all {
        assert!((-1..=3).contains(&m.level), "{} level {}", m.name, m.level);
        assert!(!m.strikes.is_empty(), "{} has no Strikes", m.name);
        assert!(
            m.strikes.iter().all(|s| !s.damage_expr.is_empty()),
            "{} Strike without damage",
            m.name
        );
        assert!(
            m.ac >= 12 && m.max_hp >= 4 && m.perception >= 0,
            "{} looks wrong",
            m.name
        );
        assert!(
            !m.traits.is_empty() && !m.speed.is_empty(),
            "{} missing traits or speed",
            m.name
        );
    }
    let skeleton = &all[2];
    assert!(
        skeleton.immunities.iter().any(|i| i.contains("death")),
        "skeletons are immune to death effects"
    );
}
