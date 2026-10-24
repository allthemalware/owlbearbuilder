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

use crate::character::{CharacterSheet, WeaponStrike};
use crate::json::JsonValue;

/// Parses an export. Accepts `{"success": true, "build": {...}}` or the bare build object.
/// Errors: invalid JSON, `"success": false`, no build object, missing `name` or `level`.
pub fn import_pathbuilder_json(raw_json: &str) -> Result<CharacterSheet, String> {
    todo!()
}

/// Same as `import_pathbuilder_json` on an already parsed document.
pub fn import_pathbuilder(doc: &JsonValue) -> Result<CharacterSheet, String> {
    todo!()
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
    todo!()
}
