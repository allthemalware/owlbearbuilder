// SPDX-License-Identifier: GPL-3.0-or-later
//! Elite and weak adjustments (GM Core), creatures in play, and a small built-in bestiary.

use crate::character::{Ability, Save, Skill};
use crate::conditions::{self, Condition};
use crate::dice::{map_penalty, DiceExpr};
use crate::encounter::Actor;
use crate::json::JsonValue;
use crate::monster::MonsterStatBlock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CreatureVariant {
    Normal,
    Elite,
    Weak,
}

impl CreatureVariant {
    /// `normal`, `elite`, `weak`.
    pub fn id(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Elite => "elite",
            Self::Weak => "weak",
        }
    }

    /// `Normal`, `Elite`, `Weak`.
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Elite => "Elite",
            Self::Weak => "Weak",
        }
    }

    /// Case-insensitive id.
    pub fn parse(s: &str) -> Option<CreatureVariant> {
        match s.trim().to_ascii_lowercase().as_str() {
            "normal" => Some(Self::Normal),
            "elite" => Some(Self::Elite),
            "weak" => Some(Self::Weak),
            _ => None,
        }
    }
}

/// HP gained by an elite creature of this (base) level: 10 at level 1 or lower, 15 at 2–4,
/// 20 at 5–19, 30 at 20 and up.
pub fn elite_hp_bonus(level: i32) -> i32 {
    match level {
        i32::MIN..=1 => 10,
        2..=4 => 15,
        5..=19 => 20,
        _ => 30,
    }
}

/// HP lost by a weak creature of this (base) level: 10 at level 2 or lower, 15 at 3–5, 20 at
/// 6–20, 30 at 21 and up. GM Core's table starts at level 1; lower levels use its first row.
pub fn weak_hp_penalty(level: i32) -> i32 {
    match level {
        i32::MIN..=2 => 10,
        3..=5 => 15,
        6..=20 => 20,
        _ => 30,
    }
}

/// A copy of `base` with the GM Core adjustment applied (Normal returns it unchanged):
/// - Elite: level +1, or +2 when the base level is 0 or lower; +2 to AC, Fortitude, Reflex,
///   Will, Perception, every skill and every Strike's attack modifier; +2 to every Strike's
///   damage (the flat part of `damage_expr`, kept in canonical dice form, e.g. `1d6` →
///   `1d6+2`); every `DC <n>` and `attack +<n>` in ability text +2; max HP + `elite_hp_bonus`.
/// - Weak: level -1, or -2 when the base level is 1; the same numbers -2 (damage
///   `1d4+1` → `1d4-1`; the minimum of 1 damage is applied when rolling, not here); max HP -
///   `weak_hp_penalty`, never below 1.
/// The name and id stay the base creature's.
#[allow(clippy::doc_lazy_continuation)]
pub fn apply_variant(base: &MonsterStatBlock, variant: CreatureVariant) -> MonsterStatBlock {
    if variant == CreatureVariant::Normal {
        return base.clone();
    }
    let mut adjusted = base.clone();
    let delta = if variant == CreatureVariant::Elite {
        2
    } else {
        -2
    };
    adjusted.level = match variant {
        CreatureVariant::Elite if base.level <= 0 => base.level.saturating_add(2),
        CreatureVariant::Elite => base.level.saturating_add(1),
        CreatureVariant::Weak if base.level == 1 => base.level.saturating_sub(2),
        CreatureVariant::Weak => base.level.saturating_sub(1),
        CreatureVariant::Normal => base.level,
    };
    adjusted.ac = adjusted.ac.saturating_add(delta);
    adjusted.fort = adjusted.fort.saturating_add(delta);
    adjusted.reflex = adjusted.reflex.saturating_add(delta);
    adjusted.will = adjusted.will.saturating_add(delta);
    adjusted.perception = adjusted.perception.saturating_add(delta);
    for (_, modifier) in &mut adjusted.skills {
        *modifier = modifier.saturating_add(delta);
    }
    for strike in &mut adjusted.strikes {
        strike.attack_mod = strike.attack_mod.saturating_add(delta);
        if !strike.damage_expr.is_empty() {
            if let Ok(mut expression) = DiceExpr::parse(&strike.damage_expr) {
                expression.flat = expression.flat.saturating_add(delta);
                strike.damage_expr = expression.to_string();
            }
        }
    }
    for ability in &mut adjusted.spells_and_abilities {
        ability.text = adjust_ability_text(&ability.text, delta);
    }
    // GM Core applies the damage floor when damage is rolled, so weak adjustments retain negative flat damage here.
    adjusted.max_hp = match variant {
        CreatureVariant::Elite => base.max_hp.saturating_add(elite_hp_bonus(base.level)),
        CreatureVariant::Weak => base
            .max_hp
            .saturating_sub(weak_hp_penalty(base.level))
            .max(1),
        CreatureVariant::Normal => base.max_hp,
    };
    adjusted
}

/// One creature on the map: its base stat block, adjustment and live state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonsterInstance {
    /// Unique within a room, e.g. `goblin-warrior-2`.
    pub instance_id: String,
    /// Unadjusted; `stats()` applies the variant.
    pub base: MonsterStatBlock,
    pub variant: CreatureVariant,
    /// What the table calls it, e.g. `Goblin Warrior 2`.
    pub label: String,
    pub current_hp: i32,
    pub temp_hp: i32,
    pub conditions: Vec<Condition>,
    pub linked_token_id: Option<String>,
}

impl MonsterInstance {
    /// Full HP for the variant, no conditions, no token.
    pub fn new(
        base: MonsterStatBlock,
        variant: CreatureVariant,
        instance_id: &str,
        label: &str,
    ) -> MonsterInstance {
        let current_hp = apply_variant(&base, variant).max_hp.max(1);
        MonsterInstance {
            instance_id: instance_id.to_string(),
            base,
            variant,
            label: label.to_string(),
            current_hp,
            temp_hp: 0,
            conditions: Vec::new(),
            linked_token_id: None,
        }
    }

    pub fn stats(&self) -> MonsterStatBlock {
        apply_variant(&self.base, self.variant)
    }

    /// Switches variant keeping the damage taken: `current_hp` moves by the change in max HP,
    /// clamped to `0..=new max`.
    pub fn set_variant(&mut self, variant: CreatureVariant) {
        if self.variant == variant {
            return;
        }
        let old_max = apply_variant(&self.base, self.variant).max_hp.max(1);
        let new_max = apply_variant(&self.base, variant).max_hp.max(1);
        self.current_hp = self
            .current_hp
            .saturating_add(new_max.saturating_sub(old_max))
            .clamp(0, new_max);
        self.variant = variant;
    }

    /// Adjusted max HP, lowered by Drained (level, minimum 1, times its value), never below 1.
    pub fn effective_max_hp(&self) -> i32 {
        let stats = self.stats();
        let drained = self
            .conditions
            .iter()
            .filter_map(|condition| match condition {
                Condition::Drained(value) => Some(i32::from(*value)),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        stats
            .max_hp
            .saturating_sub(stats.level.max(1).saturating_mul(drained))
            .max(1)
    }

    /// Adjusted AC + Dex status penalty + circumstance (off-guard, raised shield).
    pub fn effective_ac(&self) -> i32 {
        let stats = self.stats();
        stats.ac
            + conditions::status_penalty(&self.conditions, Ability::Dex)
            + conditions::ac_circumstance(&self.conditions)
    }

    /// Adjusted save + status penalty for the save's ability.
    pub fn save_modifier(&self, save: Save) -> i32 {
        let stats = self.stats();
        let (modifier, ability) = match save {
            Save::Fortitude => (stats.fort, Ability::Con),
            Save::Reflex => (stats.reflex, Ability::Dex),
            Save::Will => (stats.will, Ability::Wis),
        };
        modifier + conditions::status_penalty(&self.conditions, ability)
    }

    /// Adjusted Perception + Wis status penalty.
    pub fn perception_modifier(&self) -> i32 {
        self.stats().perception + conditions::status_penalty(&self.conditions, Ability::Wis)
    }

    /// A listed skill (case-insensitive name) plus the status penalty for its ability (Lores
    /// use Int); `None` if the creature doesn't list it.
    pub fn skill_modifier(&self, name: &str) -> Option<i32> {
        let stats = self.stats();
        let (listed_name, modifier) = stats
            .skills
            .iter()
            .find(|(listed, _)| listed.eq_ignore_ascii_case(name))?;
        let ability = if listed_name.to_ascii_lowercase().contains("lore") {
            Ability::Int
        } else if let Some(skill) = Skill::parse(listed_name) {
            skill.ability()
        } else {
            return Some(modifier + universal_status_penalty(&self.conditions));
        };
        Some(*modifier + conditions::status_penalty(&self.conditions, ability))
    }

    /// Adjusted attack + status penalty (Dex for ranged or finesse Strikes, else Str) +
    /// multiple attack penalty for `map_step`, agile-aware.
    pub fn strike_attack_modifier(&self, index: usize, map_step: u8) -> Option<i32> {
        let stats = self.stats();
        let strike = stats.strikes.get(index)?;
        let ability = strike_ability(strike);
        Some(
            strike.attack_mod
                + conditions::status_penalty(&self.conditions, ability)
                + map_penalty(map_step, strike.is_agile()),
        )
    }

    /// Adjusted damage expression with Enfeebled subtracted for melee and thrown Strikes
    /// (only Enfeebled affects damage), canonical dice form. `None` for an unknown index;
    /// empty string for a Strike without damage.
    pub fn strike_damage_expr(&self, index: usize) -> Option<String> {
        let strike = self.stats().strikes.get(index)?.clone();
        if strike.damage_expr.is_empty() {
            return Some(String::new());
        }
        let damage_penalty = if !strike.is_ranged
            || strike
                .traits
                .iter()
                .any(|trait_name| trait_name.starts_with("thrown"))
        {
            -self
                .conditions
                .iter()
                .filter_map(|condition| match condition {
                    Condition::Enfeebled(value) => Some(i32::from(*value)),
                    _ => None,
                })
                .max()
                .unwrap_or(0)
        } else {
            0
        };
        Some(adjust_damage(&strike.damage_expr, damage_penalty))
    }

    /// Temporary HP absorb first; current HP stops at 0. Negative amounts do nothing.
    pub fn apply_damage(&mut self, amount: i32) {
        if amount <= 0 {
            return;
        }
        let absorbed = amount.min(self.temp_hp);
        self.temp_hp -= absorbed;
        self.current_hp = self.current_hp.saturating_sub(amount - absorbed).max(0);
    }

    /// Up to `effective_max_hp`. Negative amounts do nothing.
    pub fn heal(&mut self, amount: i32) {
        if amount > 0 {
            self.current_hp = self
                .current_hp
                .saturating_add(amount)
                .min(self.effective_max_hp());
        }
    }

    /// The compact payload stored on the Owlbear Rodeo token under
    /// `com.owlbearbuilder/actor`: `{"kind": "monster", "id": instance_id, "name": label,
    /// "hp", "max_hp" (effective), "temp_hp", "ac" (effective), "level" (adjusted),
    /// "variant" (id), "conditions": [labels]}`.
    pub fn to_obr_token_metadata(&self) -> JsonValue {
        let stats = self.stats();
        let mut value = JsonValue::object();
        value.insert("kind", "monster");
        value.insert("id", self.instance_id.as_str());
        value.insert("name", self.label.as_str());
        value.insert("hp", self.current_hp);
        value.insert("max_hp", self.effective_max_hp());
        value.insert("temp_hp", self.temp_hp);
        value.insert("ac", self.effective_ac());
        value.insert("level", stats.level);
        value.insert("variant", self.variant.id());
        value.insert(
            "conditions",
            JsonValue::Array(
                self.conditions
                    .iter()
                    .map(|condition| JsonValue::from(condition.label()))
                    .collect(),
            ),
        );
        value
    }

    /// Every field under its own name; `base` via `MonsterStatBlock::to_json`, `variant` as
    /// its id, conditions via `Condition::to_json`.
    pub fn to_json(&self) -> JsonValue {
        let mut value = JsonValue::object();
        value.insert("instance_id", self.instance_id.as_str());
        value.insert("base", self.base.to_json());
        value.insert("variant", self.variant.id());
        value.insert("label", self.label.as_str());
        value.insert("current_hp", self.current_hp);
        value.insert("temp_hp", self.temp_hp);
        value.insert(
            "conditions",
            JsonValue::Array(self.conditions.iter().map(Condition::to_json).collect()),
        );
        value.insert(
            "linked_token_id",
            self.linked_token_id
                .as_ref()
                .map_or(JsonValue::Null, |token_id| {
                    JsonValue::from(token_id.as_str())
                }),
        );
        value
    }

    pub fn from_json(v: &JsonValue) -> Result<MonsterInstance, String> {
        let instance_id = required_string(v, "instance_id")?;
        let base = v
            .get("base")
            .ok_or_else(|| "base is required".to_string())
            .and_then(MonsterStatBlock::from_json)
            .map_err(|error| format!("base: {error}"))?;
        let variant = CreatureVariant::parse(&required_string(v, "variant")?)
            .ok_or_else(|| "variant is invalid".to_string())?;
        let label = required_string(v, "label")?;
        let current_hp = required_i32(v, "current_hp")?;
        let temp_hp = required_i32(v, "temp_hp")?;
        let conditions = v
            .get_array("conditions")
            .ok_or_else(|| "conditions must be an array".to_string())?
            .iter()
            .map(Condition::from_json)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("conditions: {error}"))?;
        let linked_token_id = match v.get("linked_token_id") {
            None | Some(JsonValue::Null) => None,
            Some(JsonValue::Str(token_id)) => Some(token_id.clone()),
            Some(_) => return Err("linked_token_id must be a string or null".into()),
        };
        Ok(MonsterInstance {
            instance_id,
            base,
            variant,
            label,
            current_hp,
            temp_hp,
            conditions,
            linked_token_id,
        })
    }
}

impl Actor for MonsterInstance {
    fn actor_id(&self) -> &str {
        &self.instance_id
    }

    fn conditions_mut(&mut self) -> &mut Vec<Condition> {
        &mut self.conditions
    }
}

/// Which ability a monster Strike attacks with, which decides Clumsy vs Enfeebled.
pub fn strike_ability(strike: &crate::monster::MonsterStrike) -> Ability {
    if strike.is_ranged || strike.traits.iter().any(|t| t == "finesse") {
        Ability::Dex
    } else {
        Ability::Str
    }
}

fn adjust_damage(damage: &str, delta: i32) -> String {
    let Ok(mut expression) = DiceExpr::parse(damage) else {
        return damage.to_string();
    };
    expression.flat = expression.flat.saturating_add(delta);
    expression.to_string()
}

fn adjust_ability_text(text: &str, delta: i32) -> String {
    let bytes = text.as_bytes();
    let mut output = String::with_capacity(text.len());
    let mut copied = 0;
    let mut index = 0;
    while index < bytes.len() {
        let keyword = if bytes[index..].starts_with(b"DC") {
            Some((b"DC".as_slice(), false))
        } else if bytes[index..].starts_with(b"attack") {
            Some((b"attack".as_slice(), true))
        } else {
            None
        };
        let Some((word, signed)) = keyword else {
            index += 1;
            continue;
        };
        if index > 0 && is_word_byte(bytes[index - 1]) {
            index += word.len();
            continue;
        }
        let mut number_start = index + word.len();
        if number_start >= bytes.len() || !bytes[number_start].is_ascii_whitespace() {
            index += word.len();
            continue;
        }
        while number_start < bytes.len() && bytes[number_start].is_ascii_whitespace() {
            number_start += 1;
        }
        let sign_start = number_start;
        if signed && number_start < bytes.len() && matches!(bytes[number_start], b'+' | b'-') {
            number_start += 1;
        } else if signed {
            index += word.len();
            continue;
        }
        let digit_start = number_start;
        while number_start < bytes.len() && bytes[number_start].is_ascii_digit() {
            number_start += 1;
        }
        if digit_start == number_start {
            index += word.len();
            continue;
        }
        let Some(value) = text[sign_start..number_start].parse::<i32>().ok() else {
            index += word.len();
            continue;
        };
        output.push_str(&text[copied..sign_start]);
        let adjusted = value.saturating_add(delta);
        if signed {
            output.push_str(&format!("{adjusted:+}"));
        } else {
            output.push_str(&adjusted.to_string());
        }
        copied = number_start;
        index = number_start;
    }
    output.push_str(&text[copied..]);
    output
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn universal_status_penalty(conditions: &[Condition]) -> i32 {
    -i32::from(
        conditions
            .iter()
            .filter_map(|condition| match condition {
                Condition::Frightened(value) | Condition::Sickened(value) => Some(*value),
                _ => None,
            })
            .max()
            .unwrap_or(0),
    )
}

fn required_string(value: &JsonValue, key: &str) -> Result<String, String> {
    value
        .get_str(key)
        .map(str::to_string)
        .ok_or_else(|| format!("{key} must be a string"))
}

fn required_i32(value: &JsonValue, key: &str) -> Result<i32, String> {
    value
        .get_i64(key)
        .and_then(|number| i32::try_from(number).ok())
        .ok_or_else(|| format!("{key} must be a 32-bit integer"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ability_text_adjustment_matches_only_supported_tokens() {
        let text = "DCs are listed; DC-less text; wordsDC 12; DC 10, attack +4";
        assert_eq!(
            adjust_ability_text(text, 2),
            "DCs are listed; DC-less text; wordsDC 12; DC 12, attack +6"
        );
    }

    #[test]
    fn ability_text_adjustment_normalizes_negative_attack_modifier() {
        assert_eq!(
            adjust_ability_text("Special attack -2", 2),
            "Special attack +0"
        );
    }

    #[test]
    fn malformed_damage_is_unchanged() {
        assert_eq!(adjust_damage("damage?", 2), "damage?");
    }

    #[test]
    fn setting_current_variant_preserves_instance_exactly() {
        let mut instance = MonsterInstance::new(
            crate::monster::parse_aon_statblock(
                "Rat Creature 0\nSmall Animal\nPerception +3\nAC 14; Fort +4, Ref +6, Will +2\nHP 8\n",
            )
            .unwrap(),
            CreatureVariant::Normal,
            "rat-1",
            "Rat 1",
        );
        instance.apply_damage(3);
        let original = instance.clone();
        instance.set_variant(CreatureVariant::Normal);
        assert_eq!(instance, original);
    }
}
