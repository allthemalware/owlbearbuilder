// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashSet;

use obb_core::aoe::parse_area_description;
use obb_core::dice::DiceExpr;
use obb_core::spell_data::COMMON_SPELLS;

#[test]
fn every_entry_is_well_formed() {
    let mut names = HashSet::new();
    for s in COMMON_SPELLS {
        assert!(names.insert(s.name.to_lowercase()), "duplicate {}", s.name);
        assert!(s.rank <= 10, "{}", s.name);
        assert!(!s.traditions.is_empty(), "{} has no tradition", s.name);
        for t in s.traditions {
            assert!(
                ["arcane", "divine", "occult", "primal"].contains(t),
                "{}: {t}",
                s.name
            );
        }
        assert!(
            ["1", "2", "3", "1 to 3", "1 or 2", "2 or 3", "reaction", "free"].contains(&s.actions),
            "{}: {}",
            s.name,
            s.actions
        );
        if let Some(a) = s.area {
            assert!(
                parse_area_description(a).is_some(),
                "{}: area {a:?}",
                s.name
            );
        }
        if let Some(d) = s.defense {
            assert!(
                [
                    "AC",
                    "Fortitude",
                    "Reflex",
                    "Will",
                    "basic Fortitude",
                    "basic Reflex",
                    "basic Will"
                ]
                .contains(&d),
                "{}: {d}",
                s.name
            );
        }
        for d in [s.damage, s.heighten_damage].into_iter().flatten() {
            assert!(DiceExpr::parse(d).is_ok(), "{}: damage {d:?}", s.name);
        }
        assert_eq!(
            s.heighten_every > 0,
            s.heighten_damage.is_some(),
            "{}: heightening",
            s.name
        );
        assert_eq!(
            s.damage.is_some(),
            s.damage_type.is_some(),
            "{}: damage type",
            s.name
        );
    }
    assert!(
        COMMON_SPELLS.len() >= 30,
        "only {} spells",
        COMMON_SPELLS.len()
    );
    for want in [
        "Fireball",
        "Breathe Fire",
        "Electric Arc",
        "Fear",
        "Grease",
        "Heal",
        "Harm",
        "Lightning Bolt",
        "Cone of Cold",
        "Bless",
        "Divine Wrath",
        "Dizzying Colors",
        "Ignition",
        "Void Warp",
        "Vitality Lash",
        "Force Barrage",
        "Runic Weapon",
        "Slow",
        "Haste",
        "Synesthesia",
        "Chain Lightning",
        "Frostbite",
        "Caustic Blast",
        "Daze",
        "Gouging Claw",
        "Noise Blast",
        "Thunderstrike",
        "Shield",
        "Command",
    ] {
        assert!(names.contains(&want.to_lowercase()), "missing {want}");
    }
}
