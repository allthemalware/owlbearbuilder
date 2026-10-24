// SPDX-License-Identifier: GPL-3.0-or-later
//! One entry point for every character format, and non-destructive re-imports: a player who
//! levels up sends a fresh export, and the table keeps their HP, conditions and token.

use crate::character::CharacterSheet;
use crate::json::JsonValue;

/// Detects the format from the JSON root and imports it:
/// - `"format": "owlbearbuilder-character"` → `CharacterSheet::from_json`;
/// - a `build` object, or a root with `name`, `abilities` and `proficiencies` → Pathbuilder;
/// - anything else (including invalid JSON) → an error saying which formats are accepted.
pub fn import_character_auto(raw_json: &str) -> Result<CharacterSheet, String> {
    todo!()
}

/// Same as `import_character_auto` on a parsed document.
pub fn import_character_value(doc: &JsonValue) -> Result<CharacterSheet, String> {
    todo!()
}

/// Replaces `existing`'s build with `updated`'s while keeping what belongs to the table:
/// `id`, `linked_token_id`, `conditions`, `hero_points`, `temp_hp` and `player_name` (unless
/// `updated` has one). `current_hp` rises by any increase in `max_hp` and is capped at the new
/// `max_hp`.
///
/// Returns what changed, in this order, comparing both sheets with conditions cleared so the
/// list reflects the build rather than the moment:
/// `Level: 4 -> 5`, `Class: ...`, `Max HP: 52 -> 64`, `AC: 21 -> 22`, `Perception: +9 -> +11`,
/// `Fortitude`, `Reflex`, `Will`, `Class DC: 20 -> 22`, then each skill in `Skill::ALL` order
/// (`Athletics: +11 -> +14 (Master)`, the rank label only when the rank changed), then Lores
/// (`Warfare Lore: +7 -> +9`, `New Lore: Sailing Lore +5 (Trained)`), `Speed: 25 -> 30`,
/// `New Strike: <name>` / `Removed Strike: <name>`, `New feat: <name>`, `New spell: <name>`.
/// Modifiers are signed (`+0`, `-1`). Unchanged values produce no line.
pub fn merge_update(existing: &mut CharacterSheet, updated: CharacterSheet) -> Vec<String> {
    todo!()
}
