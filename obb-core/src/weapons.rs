// SPDX-License-Identifier: GPL-3.0-or-later
//! Traits of common weapons. Builder exports name the weapon and its dice but usually not its
//! traits, and the traits decide the multiple attack penalty (agile), which ability attacks
//! (finesse, ranged), and critical damage (deadly, fatal).

pub struct WeaponInfo {
    /// Base weapon name as printed in Player Core, e.g. `Composite Longbow`.
    pub name: &'static str,
    /// Lowercase traits with their values, e.g. `agile`, `deadly d10`, `thrown 20 ft`,
    /// `volley 30 ft`, `versatile p`, `two-hand d12`.
    pub traits: &'static [&'static str],
    /// Range increment in feet for ranged weapons. Melee weapons that can be thrown keep `None`
    /// here and carry a `thrown N ft` trait instead.
    pub range_ft: Option<u32>,
}

static WEAPONS: &[WeaponInfo] = &[
    WeaponInfo {
        name: "Fist",
        traits: &["agile", "finesse", "nonlethal", "unarmed"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Club",
        traits: &["thrown 10 ft"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Dagger",
        traits: &["agile", "finesse", "thrown 10 ft", "versatile s"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Gauntlet",
        traits: &["agile", "free-hand"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Light Mace",
        traits: &["agile", "finesse", "shove"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Longspear",
        traits: &["reach"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Mace",
        traits: &["shove"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Morningstar",
        traits: &["versatile p"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Sickle",
        traits: &["agile", "finesse", "trip"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Spear",
        traits: &["thrown 20 ft"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Spiked Gauntlet",
        traits: &["agile", "free-hand"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Staff",
        traits: &["two-hand d8"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Clan Dagger",
        traits: &["agile", "parry", "versatile b"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Katar",
        traits: &["agile", "deadly d6"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Bastard Sword",
        traits: &["two-hand d12"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Battle Axe",
        traits: &["sweep"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Bo Staff",
        traits: &["parry", "reach", "trip"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Falchion",
        traits: &["forceful", "sweep"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Flail",
        traits: &["disarm", "sweep", "trip"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Glaive",
        traits: &["deadly d8", "forceful", "reach"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Greataxe",
        traits: &["sweep"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Greatclub",
        traits: &["backswing", "shove"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Greatpick",
        traits: &["fatal d12"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Greatsword",
        traits: &["versatile p"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Guisarme",
        traits: &["reach", "trip"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Halberd",
        traits: &["reach", "versatile s"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Hatchet",
        traits: &["agile", "sweep", "thrown 10 ft"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Lance",
        traits: &["deadly d8", "jousting d6", "reach"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Light Hammer",
        traits: &["agile", "thrown 20 ft"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Light Pick",
        traits: &["agile", "fatal d8"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Longsword",
        traits: &["versatile p"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Main-gauche",
        traits: &["agile", "disarm", "finesse", "parry", "versatile s"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Maul",
        traits: &["shove"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Pick",
        traits: &["fatal d10"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Ranseur",
        traits: &["disarm", "reach"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Rapier",
        traits: &["deadly d8", "disarm", "finesse"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Sap",
        traits: &["agile", "nonlethal"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Scimitar",
        traits: &["forceful", "sweep"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Scythe",
        traits: &["deadly d10", "trip"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Shield Bash",
        traits: &[],
        range_ft: None,
    },
    WeaponInfo {
        name: "Shield Boss",
        traits: &["attached to shield"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Shield Spikes",
        traits: &["attached to shield"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Shortsword",
        traits: &["agile", "finesse", "versatile s"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Starknife",
        traits: &[
            "agile",
            "deadly d6",
            "finesse",
            "thrown 20 ft",
            "versatile s",
        ],
        range_ft: None,
    },
    WeaponInfo {
        name: "Trident",
        traits: &["thrown 20 ft"],
        range_ft: None,
    },
    WeaponInfo {
        name: "War Flail",
        traits: &["disarm", "sweep", "trip"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Warhammer",
        traits: &["shove"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Whip",
        traits: &["disarm", "finesse", "nonlethal", "reach", "trip"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Kukri",
        traits: &["agile", "finesse", "trip"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Dueling Sword",
        traits: &["finesse", "versatile p"],
        range_ft: None,
    },
    WeaponInfo {
        name: "Blowgun",
        traits: &["agile", "nonlethal", "reload 1"],
        range_ft: Some(20),
    },
    WeaponInfo {
        name: "Crossbow",
        traits: &["reload 1"],
        range_ft: Some(120),
    },
    WeaponInfo {
        name: "Heavy Crossbow",
        traits: &["reload 2"],
        range_ft: Some(120),
    },
    WeaponInfo {
        name: "Hand Crossbow",
        traits: &["reload 1"],
        range_ft: Some(60),
    },
    WeaponInfo {
        name: "Dart",
        traits: &["agile", "thrown"],
        range_ft: Some(20),
    },
    WeaponInfo {
        name: "Javelin",
        traits: &["thrown"],
        range_ft: Some(30),
    },
    WeaponInfo {
        name: "Sling",
        traits: &["propulsive", "reload 1"],
        range_ft: Some(50),
    },
    WeaponInfo {
        name: "Longbow",
        traits: &["deadly d10", "volley 30 ft"],
        range_ft: Some(100),
    },
    WeaponInfo {
        name: "Composite Longbow",
        traits: &["deadly d10", "propulsive", "volley 30 ft"],
        range_ft: Some(100),
    },
    WeaponInfo {
        name: "Shortbow",
        traits: &["deadly d10"],
        range_ft: Some(60),
    },
    WeaponInfo {
        name: "Composite Shortbow",
        traits: &["deadly d10", "propulsive"],
        range_ft: Some(60),
    },
];

/// Looks a weapon up by name, case-insensitive, ignoring surrounding whitespace and a trailing
/// parenthetical (`Longsword (cold iron)` → `Longsword`). `None` for unknown weapons.
pub fn weapon_info(name: &str) -> Option<&'static WeaponInfo> {
    let name = name
        .trim()
        .split_once('(')
        .map_or(name.trim(), |(base, _)| base.trim());
    WEAPONS
        .iter()
        .find(|weapon| weapon.name.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_ignores_case_whitespace_and_material() {
        assert_eq!(
            weapon_info("  lOnGsWoRd (cold iron) ").unwrap().name,
            "Longsword"
        );
    }

    #[test]
    fn names_are_unique() {
        for (index, weapon) in WEAPONS.iter().enumerate() {
            assert!(!WEAPONS[..index]
                .iter()
                .any(|other| other.name == weapon.name));
        }
    }
}
