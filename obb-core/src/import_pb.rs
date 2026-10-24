// SPDX-License-Identifier: GPL-3.0-or-later
//! Pathbuilder 2e "Export JSON" import.
//!
//! Mapping from the export's `build` object:
//! - `name`, `class`, `level`, `ancestry`, `heritage`, `background`, `sizeName` → identity
//!   (`size` defaults to `Medium` when `sizeName` is missing); `keyability` → `key_ability`.
//! - `abilities.{str..cha}`: ability scores (8 and up) become modifiers with
//!   `(score - 10).div_euclid(2)`; values of 7 or less are already modifiers (no legal score is
//!   below 8 and no modifier reaches 8), so they are taken as is.
//! - `attributes`: `max_hp = ancestryhp + bonushp + level * (classhp + con_mod +
//!   bonushpPerLevel)`; `speed = speed + speedBonus`. `current_hp` starts at `max_hp`.
//! - `acTotal.acTotal` → `ac_base`; `acTotal.shieldBonus` (a number, a numeric string, or
//!   null) → `shield_bonus`.
//! - `proficiencies` (rank bonuses 0/2/4/6/8): `perception`, `fortitude`, `reflex`, `will`,
//!   `classDC` and the 16 skill ids.
//! - `lores`: `[["Warfare", 2], ...]`.
//! - `mods`: `{"Athletics": {"Item Bonus": 1}, "Warfare Lore": {...}}`; only item bonuses are
//!   kept (status and circumstance entries there are situational toggles), keyed as in
//!   `CharacterSheet::item_bonuses`. Unknown stat names are ignored.
//! - Worn armor's `res` (`resilient`, `greaterResilient`, `majorResilient`) gives +1/+2/+3
//!   item bonus to all saves unless `mods` already gives a higher one.
//! - `feats` (`[[name, extra, type, level], ...]`) and `specials` (names) → `feats`, names
//!   only, in that order, without duplicates.
//! - `weapons`: one `WeaponStrike` each, see `strike_from_pathbuilder`.
//! - `spellCasters`: one `SpellcastingEntry` each: `name`, `magicTradition`, `ability`,
//!   `proficiency`, `focusPoints`, `spells: [{spellLevel, list}]`; `kind` is `innate` when
//!   `innate` is true, else `spellcastingType` (`prepared` / `spontaneous`).
//! - `focus: {<tradition>: {<ability>: {proficiency, focusCantrips, focusSpells}}}` → one
//!   `focus` entry per tradition/ability pair named `Focus Spells`, with `focus_points` from
//!   the build's `focusPoints`; focus cantrips at rank 0 and focus spells at rank
//!   `(level + 1) / 2`, because focus spells are always cast at half level rounded up.
//! - `id = slugify(name)`; `player_name` stays empty (the export doesn't carry it).

use crate::character::{Ability, Lore, ProfRank, Skill, SpellcastingEntry};
use crate::character::{CharacterSheet, WeaponStrike};
use crate::dice::StrikeDamage;
use crate::json::JsonValue;

/// Parses an export. Accepts `{"success": true, "build": {...}}` or the bare build object.
/// Errors: invalid JSON, `"success": false`, no build object, missing `name` or `level`.
pub fn import_pathbuilder_json(raw_json: &str) -> Result<CharacterSheet, String> {
    let doc = JsonValue::parse(raw_json).map_err(|error| error.to_string())?;
    import_pathbuilder(&doc)
}

/// Same as `import_pathbuilder_json` on an already parsed document.
pub fn import_pathbuilder(doc: &JsonValue) -> Result<CharacterSheet, String> {
    if doc.get_bool("success") == Some(false) {
        return Err("Pathbuilder export reported failure".into());
    }
    let build = doc.get("build").unwrap_or(doc);
    let name = required_string(build, "name")?;
    let level = number(build.get("level"), 0);
    if level == 0 {
        return Err("missing or invalid level".into());
    }
    let mut sheet = CharacterSheet::new(name, level);
    for (field, target) in [
        ("class", &mut sheet.class),
        ("ancestry", &mut sheet.ancestry),
        ("heritage", &mut sheet.heritage),
        ("background", &mut sheet.background),
    ] {
        *target = string(build.get(field)).unwrap_or_default();
    }
    sheet.size = string(build.get("sizeName")).unwrap_or_else(|| "Medium".into());
    sheet.key_ability = Ability::parse(string(build.get("keyability")).as_deref().unwrap_or("str"))
        .unwrap_or(Ability::Str);
    if let Some(abilities) = build.get("abilities") {
        for ability in Ability::ALL {
            let score = number(abilities.get(ability.id()), 0);
            sheet.abilities[ability as usize] = if score >= 8 {
                (score - 10).div_euclid(2)
            } else {
                score
            };
        }
    }
    let attrs = build.get("attributes");
    let con = sheet.abilities[Ability::Con as usize];
    sheet.max_hp = number(attrs.and_then(|a| a.get("ancestryhp")), 0)
        + number(attrs.and_then(|a| a.get("bonushp")), 0)
        + level
            * (number(attrs.and_then(|a| a.get("classhp")), 0)
                + con
                + number(attrs.and_then(|a| a.get("bonushpPerLevel")), 0));
    sheet.current_hp = sheet.max_hp;
    sheet.speed = number(attrs.and_then(|a| a.get("speed")), 25)
        + number(attrs.and_then(|a| a.get("speedBonus")), 0);
    let ac = build.get("acTotal");
    sheet.ac_base = number(ac.and_then(|a| a.get("acTotal")), 10);
    sheet.shield_bonus = number(ac.and_then(|a| a.get("shieldBonus")), 0);
    let prof = build.get("proficiencies");
    let rank =
        |key: &str| ProfRank::from_bonus(i64::from(number(prof.and_then(|p| p.get(key)), 0)));
    sheet.perception_rank = rank("perception");
    sheet.fort_rank = rank("fortitude");
    sheet.reflex_rank = rank("reflex");
    sheet.will_rank = rank("will");
    sheet.class_dc_rank = rank("classDC");
    for skill in Skill::ALL {
        sheet.skill_ranks[skill as usize] = rank(skill.id());
    }
    if let Some(lores) = build.get("lores").and_then(JsonValue::as_array) {
        for lore in lores {
            if let Some(pair) = lore.as_array() {
                if let Some(name) = pair.first().and_then(JsonValue::as_str) {
                    sheet.lores.push(Lore {
                        name: name.into(),
                        rank: ProfRank::from_bonus(i64::from(
                            pair.get(1).map_or(0, |v| number(Some(v), 0)),
                        )),
                    });
                }
            }
        }
    }
    if let Some(mods) = build.get("mods").and_then(JsonValue::as_object) {
        for (stat, values) in mods {
            if let Some(item) = values.get("Item Bonus") {
                let key = stat.to_ascii_lowercase();
                if known_stat(&key, &sheet) {
                    sheet.item_bonuses.insert(key, number(Some(item), 0));
                }
            }
        }
    }
    if let Some(armor) = build.get("armor").and_then(JsonValue::as_array) {
        let resilient = armor
            .iter()
            .filter(|a| a.get_bool("worn") == Some(true))
            .map(|a| match a.get_str("res").unwrap_or("") {
                "majorResilient" => 3,
                "greaterResilient" => 2,
                "resilient" => 1,
                _ => 0,
            })
            .max()
            .unwrap_or(0);
        for save in ["fortitude", "reflex", "will"] {
            if resilient > sheet.item_bonuses.get(save).copied().unwrap_or(0) {
                sheet.item_bonuses.insert(save.into(), resilient);
            }
        }
    }
    if let Some(feats) = build.get("feats").and_then(JsonValue::as_array) {
        for feat in feats {
            if let Some(name) = feat
                .as_array()
                .and_then(|a| a.first())
                .and_then(JsonValue::as_str)
            {
                push_unique(&mut sheet.feats, name);
            }
        }
    }
    if let Some(specials) = build.get("specials").and_then(JsonValue::as_array) {
        for feat in specials.iter().filter_map(JsonValue::as_str) {
            push_unique(&mut sheet.feats, feat);
        }
    }
    if let Some(weapons) = build.get("weapons").and_then(JsonValue::as_array) {
        for weapon in weapons {
            sheet
                .strikes
                .push(strike_from_pathbuilder(weapon, &sheet.abilities)?);
        }
    }
    if let Some(casters) = build.get("spellCasters").and_then(JsonValue::as_array) {
        for caster in casters {
            let ability =
                Ability::parse(caster.get_str("ability").unwrap_or("int")).unwrap_or(Ability::Int);
            let spells = parse_spells(caster.get("spells"));
            sheet.spellcasting.push(SpellcastingEntry {
                name: string(caster.get("name")).unwrap_or_default(),
                tradition: string(caster.get("magicTradition")).unwrap_or_default(),
                kind: if caster.get_bool("innate") == Some(true) {
                    "innate".into()
                } else {
                    string(caster.get("spellcastingType")).unwrap_or_default()
                },
                ability,
                rank: ProfRank::from_bonus(i64::from(number(caster.get("proficiency"), 0))),
                focus_points: number(caster.get("focusPoints"), 0).clamp(0, 255) as u8,
                spells,
            });
        }
    }
    if let Some(focus) = build.get("focus").and_then(JsonValue::as_object) {
        let points = number(build.get("focusPoints"), 0).clamp(0, 255) as u8;
        for (tradition, abilities) in focus {
            if let Some(abilities) = abilities.as_object() {
                for (ability_name, entry) in abilities {
                    let ability = Ability::parse(ability_name).unwrap_or(Ability::Int);
                    let mut spells = Vec::new();
                    let cantrips = string_array(entry.get("focusCantrips"));
                    if !cantrips.is_empty() {
                        spells.push((0, cantrips));
                    }
                    let focus_spells = string_array(entry.get("focusSpells"));
                    if !focus_spells.is_empty() {
                        spells.push((((level + 1) / 2).clamp(0, 255) as u8, focus_spells));
                    }
                    sheet.spellcasting.push(SpellcastingEntry {
                        name: "Focus Spells".into(),
                        tradition: tradition.clone(),
                        kind: "focus".into(),
                        ability,
                        rank: ProfRank::from_bonus(i64::from(number(entry.get("proficiency"), 0))),
                        focus_points: points,
                        spells,
                    });
                }
            }
        }
    }
    Ok(sheet)
}

/// One entry of the export's `weapons` array, given the character's ability modifiers
/// (indexed by `Ability as usize`).
/// - `name` ← `display` (falls back to `name`); traits ← the entry's own `traits` array if
///   present, else `weapons::weapon_info(name)`, else none; traits are stored lowercase.
/// - Ranged when the table gives a range (or the entry has `"ranged": true`).
/// - `attack_bonus` ← `attack`; `attack_ability`: Dex for ranged weapons, Dex for finesse
///   melee weapons when Dex beats Str, else Str.
/// - Damage: `die` (`d8`) is the die size, one step larger (d4→d6→d8→d10→d12) when
///   `increasedDice` is true; dice count from `str`: `""` 1, `striking` 2,
///   `greaterStriking` 3, `majorStriking` 4; `damageBonus` → `flat_bonus`; `deadly dX` and
///   `fatal dX` traits → `deadly` / `fatal`.
/// - `damageType` `S`/`P`/`B` → `slashing`/`piercing`/`bludgeoning`, other text kept as is.
/// - `extraDamage` strings → `extra_damage`.
pub fn strike_from_pathbuilder(
    weapon: &JsonValue,
    abilities: &[i32; 6],
) -> Result<WeaponStrike, String> {
    let name = weapon
        .get_str("display")
        .filter(|s| !s.is_empty())
        .or_else(|| weapon.get_str("name"))
        .ok_or("weapon name is missing")?;
    let base_name = weapon.get_str("name").unwrap_or(name);
    let info = crate::weapons::weapon_info(base_name);
    let traits = weapon
        .get("traits")
        .and_then(JsonValue::as_array)
        .map(|a| {
            a.iter()
                .filter_map(JsonValue::as_str)
                .map(|s| s.to_ascii_lowercase())
                .collect()
        })
        .unwrap_or_else(|| {
            info.map_or_else(Vec::new, |i| {
                i.traits.iter().map(|s| s.trim().to_string()).collect()
            })
        });
    let is_ranged =
        weapon.get_bool("ranged") == Some(true) || info.is_some_and(|i| i.range_ft.is_some());
    let dex = abilities[Ability::Dex as usize];
    let strength = abilities[Ability::Str as usize];
    let finesse = traits.iter().any(|t| t == "finesse");
    let attack_ability = if is_ranged || (finesse && dex > strength) {
        Ability::Dex
    } else {
        Ability::Str
    };
    let raw_die = weapon.get_str("die").unwrap_or("d4");
    let mut die_sides = raw_die
        .strip_prefix('d')
        .and_then(|n| n.parse::<u32>().ok())
        .ok_or_else(|| "invalid weapon die".to_string())?;
    if weapon.get_bool("increasedDice") == Some(true) {
        die_sides = match die_sides {
            4 => 6,
            6 => 8,
            8 => 10,
            10 => 12,
            _ => return Err("cannot increase weapon die".into()),
        };
    }
    let dice_count = match weapon.get_str("str").unwrap_or("") {
        "" => 1,
        "striking" => 2,
        "greaterStriking" => 3,
        "majorStriking" => 4,
        _ => return Err("invalid striking grade".into()),
    };
    let trait_value = |prefix: &str| {
        traits.iter().find_map(|t| {
            t.strip_prefix(prefix)
                .and_then(|v| v.trim().strip_prefix('d'))
                .and_then(|v| v.parse().ok())
        })
    };
    let damage_type = match weapon.get_str("damageType").unwrap_or("") {
        "S" => "slashing".into(),
        "P" => "piercing".into(),
        "B" => "bludgeoning".into(),
        other => other.into(),
    };
    Ok(WeaponStrike {
        name: name.into(),
        attack_bonus: number(weapon.get("attack"), 0),
        attack_ability,
        is_ranged,
        damage: StrikeDamage {
            dice_count,
            die_sides,
            flat_bonus: number(weapon.get("damageBonus"), 0),
            deadly: trait_value("deadly "),
            fatal: trait_value("fatal "),
        },
        damage_type,
        extra_damage: string_array(weapon.get("extraDamage")),
        traits,
    })
}

fn string(value: Option<&JsonValue>) -> Option<String> {
    value.and_then(JsonValue::as_str).map(str::to_string)
}
fn required_string<'a>(value: &'a JsonValue, key: &str) -> Result<&'a str, String> {
    value
        .get_str(key)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("missing {key}"))
}
fn number(value: Option<&JsonValue>, default: i32) -> i32 {
    value
        .and_then(|v| {
            v.as_i64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
        .and_then(|n| i32::try_from(n).ok())
        .unwrap_or(default)
}
fn string_array(value: Option<&JsonValue>) -> Vec<String> {
    value
        .and_then(JsonValue::as_array)
        .map(|a| {
            a.iter()
                .filter_map(JsonValue::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}
fn push_unique(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.into());
    }
}
fn parse_spells(value: Option<&JsonValue>) -> Vec<(u8, Vec<String>)> {
    value
        .and_then(JsonValue::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|e| {
                    let rank = u8::try_from(number(e.get("spellLevel"), -1)).ok()?;
                    Some((rank, string_array(e.get("list"))))
                })
                .collect()
        })
        .unwrap_or_default()
}
fn known_stat(stat: &str, sheet: &CharacterSheet) -> bool {
    [
        "perception",
        "fortitude",
        "reflex",
        "will",
        "class dc",
        "classdc",
    ]
    .contains(&stat)
        || Skill::ALL.iter().any(|skill| stat == skill.id())
        || stat.ends_with(" lore")
            && sheet
                .lores
                .iter()
                .any(|lore| format!("{} lore", lore.name.to_ascii_lowercase()) == stat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_striking_grades_and_increased_dice() {
        for (grade, count) in [
            ("", 1),
            ("striking", 2),
            ("greaterStriking", 3),
            ("majorStriking", 4),
        ] {
            let value = JsonValue::parse(&format!(
                r#"{{"name":"Pick","die":"d4","str":"{grade}","increasedDice":true}}"#
            ))
            .unwrap();
            let strike = strike_from_pathbuilder(&value, &[0; 6]).unwrap();
            assert_eq!(
                (strike.damage.dice_count, strike.damage.die_sides),
                (count, 6)
            );
        }
    }

    #[test]
    fn accepts_ability_scores_and_modifiers() {
        let sheet = import_pathbuilder_json(r#"{"name":"Odd","level":1,"abilities":{"str":15,"dex":9,"con":7,"int":10,"wis":12,"cha":8}}"#).unwrap();
        assert_eq!(sheet.abilities, [2, -1, 7, 0, 1, -1]);
    }

    #[test]
    fn shield_bonus_accepts_number_string_and_null() {
        for (value, expected) in [("2", 2), (r#""2""#, 2), ("null", 0)] {
            let raw =
                format!(r#"{{"name":"Shield","level":1,"acTotal":{{"shieldBonus":{value}}}}}"#);
            assert_eq!(
                import_pathbuilder_json(&raw).unwrap().shield_bonus,
                expected
            );
        }
    }

    #[test]
    fn resilient_grades_set_save_item_bonuses() {
        for (grade, bonus) in [
            ("resilient", 1),
            ("greaterResilient", 2),
            ("majorResilient", 3),
        ] {
            let raw = format!(
                r#"{{"name":"Armor","level":1,"armor":[{{"worn":true,"res":"{grade}"}}]}}"#
            );
            let sheet = import_pathbuilder_json(&raw).unwrap();
            for save in ["fortitude", "reflex", "will"] {
                assert_eq!(sheet.item_bonuses.get(save), Some(&bonus));
            }
        }
    }
}
