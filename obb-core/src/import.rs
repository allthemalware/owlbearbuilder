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
    let doc = JsonValue::parse(raw_json).map_err(|error| {
        format!(
            "invalid JSON; accepted formats are OwlbearBuilder character and Pathbuilder: {error}"
        )
    })?;
    import_character_value(&doc)
}

/// Same as `import_character_auto` on a parsed document.
pub fn import_character_value(doc: &JsonValue) -> Result<CharacterSheet, String> {
    if doc.get_str("format") == Some("owlbearbuilder-character") {
        return CharacterSheet::from_json(doc);
    }
    if doc.get("build").is_some()
        || (doc.get("name").is_some()
            && doc.get("abilities").is_some()
            && doc.get("proficiencies").is_some())
    {
        return crate::import_pb::import_pathbuilder(doc);
    }
    Err("unsupported character format; accepted formats are OwlbearBuilder character and Pathbuilder".into())
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
    use crate::character::{Save, Skill};

    let mut before = existing.clone();
    let mut after = updated.clone();
    before.conditions.clear();
    after.conditions.clear();
    let mut changes = Vec::new();
    if before.level != after.level {
        changes.push(format!("Level: {} -> {}", before.level, after.level));
    }
    if before.class != after.class {
        changes.push(format!("Class: {} -> {}", before.class, after.class));
    }
    if before.max_hp != after.max_hp {
        changes.push(format!("Max HP: {} -> {}", before.max_hp, after.max_hp));
    }
    if before.effective_ac() != after.effective_ac() {
        changes.push(format!(
            "AC: {} -> {}",
            before.effective_ac(),
            after.effective_ac()
        ));
    }
    push_modifier_change(
        &mut changes,
        "Perception",
        before.perception_modifier(),
        after.perception_modifier(),
    );
    for save in Save::ALL {
        push_modifier_change(
            &mut changes,
            save.label(),
            before.save_modifier(save),
            after.save_modifier(save),
        );
    }
    if before.class_dc() != after.class_dc() {
        changes.push(format!(
            "Class DC: {} -> {}",
            before.class_dc(),
            after.class_dc()
        ));
    }
    for skill in Skill::ALL {
        let old = before.skill_modifier(skill.id()).unwrap_or(0);
        let new = after.skill_modifier(skill.id()).unwrap_or(0);
        if old != new {
            let rank = if before.skill_ranks[skill as usize] != after.skill_ranks[skill as usize] {
                format!(" ({})", after.skill_ranks[skill as usize].label())
            } else {
                String::new()
            };
            changes.push(format!(
                "{}: {} -> {}{}",
                skill.label(),
                signed(old),
                signed(new),
                rank
            ));
        }
    }
    for old in &before.lores {
        if let Some(new) = after
            .lores
            .iter()
            .find(|l| l.name.eq_ignore_ascii_case(&old.name))
        {
            let old_mod = before.skill_modifier(&old.name).unwrap_or(0);
            let new_mod = after.skill_modifier(&new.name).unwrap_or(0);
            if old_mod != new_mod {
                changes.push(format!(
                    "{} Lore: {} -> {}",
                    old.name,
                    signed(old_mod),
                    signed(new_mod)
                ));
            }
        }
    }
    for lore in &after.lores {
        if !before
            .lores
            .iter()
            .any(|l| l.name.eq_ignore_ascii_case(&lore.name))
        {
            changes.push(format!(
                "New Lore: {} Lore {} ({})",
                lore.name,
                signed(after.skill_modifier(&lore.name).unwrap_or(0)),
                lore.rank.label()
            ));
        }
    }
    if before.speed != after.speed {
        changes.push(format!("Speed: {} -> {}", before.speed, after.speed));
    }
    for strike in &after.strikes {
        if !before.strikes.iter().any(|s| s.name == strike.name) {
            changes.push(format!("New Strike: {}", strike.name));
        }
    }
    for strike in &before.strikes {
        if !after.strikes.iter().any(|s| s.name == strike.name) {
            changes.push(format!("Removed Strike: {}", strike.name));
        }
    }
    for feat in &after.feats {
        if !before.feats.iter().any(|f| f.eq_ignore_ascii_case(feat)) {
            changes.push(format!("New feat: {feat}"));
        }
    }
    let old_spells = spell_names(&before);
    let mut reported = Vec::new();
    for spell in spell_names(&after) {
        if !old_spells.iter().any(|s| s.eq_ignore_ascii_case(&spell))
            && !reported
                .iter()
                .any(|s: &String| s.eq_ignore_ascii_case(&spell))
        {
            changes.push(format!("New spell: {spell}"));
            reported.push(spell);
        }
    }

    let id = existing.id.clone();
    let linked_token_id = existing.linked_token_id.clone();
    let conditions = existing.conditions.clone();
    let hero_points = existing.hero_points;
    let temp_hp = existing.temp_hp;
    let player_name = if updated.player_name.is_empty() {
        existing.player_name.clone()
    } else {
        updated.player_name.clone()
    };
    let hp_gain = updated.max_hp.saturating_sub(existing.max_hp).max(0);
    let current_hp = existing
        .current_hp
        .saturating_add(hp_gain)
        .min(updated.max_hp);
    *existing = updated;
    existing.id = id;
    existing.linked_token_id = linked_token_id;
    existing.conditions = conditions;
    existing.hero_points = hero_points;
    existing.temp_hp = temp_hp;
    existing.player_name = player_name;
    existing.current_hp = current_hp;
    changes
}

fn signed(value: i32) -> String {
    format!("{value:+}")
}

fn push_modifier_change(changes: &mut Vec<String>, label: &str, old: i32, new: i32) {
    if old != new {
        changes.push(format!("{label}: {} -> {}", signed(old), signed(new)));
    }
}

fn spell_names(sheet: &CharacterSheet) -> Vec<String> {
    sheet
        .spellcasting
        .iter()
        .flat_map(|entry| entry.spells.iter())
        .flat_map(|(_, spells)| spells.iter().cloned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::{Ability, ProfRank, Skill, SpellcastingEntry, WeaponStrike};
    use crate::dice::StrikeDamage;

    fn sheet() -> CharacterSheet {
        CharacterSheet::new("Test", 1)
    }

    fn strike(name: &str) -> WeaponStrike {
        WeaponStrike {
            name: name.into(),
            attack_bonus: 0,
            attack_ability: Ability::Str,
            is_ranged: false,
            damage: StrikeDamage {
                dice_count: 1,
                die_sides: 8,
                flat_bonus: 0,
                deadly: None,
                fatal: None,
            },
            damage_type: "slashing".into(),
            extra_damage: Vec::new(),
            traits: Vec::new(),
        }
    }

    #[test]
    fn reports_removed_strikes() {
        let mut old = sheet();
        old.strikes.push(strike("Longsword"));
        assert_eq!(
            merge_update(&mut old, sheet()),
            ["Removed Strike: Longsword"]
        );
    }

    #[test]
    fn reports_class_changes() {
        let mut old = sheet();
        old.class = "Fighter".into();
        let mut new = sheet();
        new.class = "Ranger".into();
        assert_eq!(merge_update(&mut old, new), ["Class: Fighter -> Ranger"]);
    }

    #[test]
    fn formats_negative_modifiers_with_sign() {
        let mut old = sheet();
        old.abilities[Ability::Str as usize] = -5;
        old.skill_ranks[Skill::Athletics as usize] = ProfRank::Trained;
        let mut new = old.clone();
        new.abilities[Ability::Str as usize] = -4;
        assert!(merge_update(&mut old, new).contains(&"Athletics: -2 -> -1".into()));
    }

    #[test]
    fn reports_new_spell_once_across_entries() {
        let mut old = sheet();
        let mut new = sheet();
        for name in ["Wizard", "Witch"] {
            new.spellcasting.push(SpellcastingEntry {
                name: name.into(),
                tradition: "arcane".into(),
                kind: "prepared".into(),
                ability: Ability::Int,
                rank: ProfRank::Trained,
                focus_points: 0,
                spells: vec![(1, vec!["Fear".into()])],
            });
        }
        let changes = merge_update(&mut old, new);
        assert_eq!(
            changes
                .iter()
                .filter(|line| *line == "New spell: Fear")
                .count(),
            1
        );
    }

    #[test]
    fn updated_player_name_replaces_existing_name() {
        let mut old = sheet();
        old.player_name = "Old".into();
        let mut new = sheet();
        new.player_name = "New".into();
        merge_update(&mut old, new);
        assert_eq!(old.player_name, "New");
    }
}
