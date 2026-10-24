// SPDX-License-Identifier: GPL-3.0-or-later
//! Elite and weak adjustments (GM Core), creatures in play, and a small built-in bestiary.

use crate::character::{Ability, Save};
use crate::conditions::Condition;
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
        todo!()
    }

    /// `Normal`, `Elite`, `Weak`.
    pub fn label(self) -> &'static str {
        todo!()
    }

    /// Case-insensitive id.
    pub fn parse(s: &str) -> Option<CreatureVariant> {
        todo!()
    }
}

/// HP gained by an elite creature of this (base) level: 10 at level 1 or lower, 15 at 2–4,
/// 20 at 5–19, 30 at 20 and up.
pub fn elite_hp_bonus(level: i32) -> i32 {
    todo!()
}

/// HP lost by a weak creature of this (base) level: 10 at level 2 or lower, 15 at 3–5, 20 at
/// 6–20, 30 at 21 and up. GM Core's table starts at level 1; lower levels use its first row.
pub fn weak_hp_penalty(level: i32) -> i32 {
    todo!()
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
pub fn apply_variant(base: &MonsterStatBlock, variant: CreatureVariant) -> MonsterStatBlock {
    todo!()
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
        todo!()
    }

    pub fn stats(&self) -> MonsterStatBlock {
        apply_variant(&self.base, self.variant)
    }

    /// Switches variant keeping the damage taken: `current_hp` moves by the change in max HP,
    /// clamped to `0..=new max`.
    pub fn set_variant(&mut self, variant: CreatureVariant) {
        todo!()
    }

    /// Adjusted max HP, lowered by Drained (level, minimum 1, times its value), never below 1.
    pub fn effective_max_hp(&self) -> i32 {
        todo!()
    }

    /// Adjusted AC + Dex status penalty + circumstance (off-guard, raised shield).
    pub fn effective_ac(&self) -> i32 {
        todo!()
    }

    /// Adjusted save + status penalty for the save's ability.
    pub fn save_modifier(&self, save: Save) -> i32 {
        todo!()
    }

    /// Adjusted Perception + Wis status penalty.
    pub fn perception_modifier(&self) -> i32 {
        todo!()
    }

    /// A listed skill (case-insensitive name) plus the status penalty for its ability (Lores
    /// use Int); `None` if the creature doesn't list it.
    pub fn skill_modifier(&self, name: &str) -> Option<i32> {
        todo!()
    }

    /// Adjusted attack + status penalty (Dex for ranged or finesse Strikes, else Str) +
    /// multiple attack penalty for `map_step`, agile-aware.
    pub fn strike_attack_modifier(&self, index: usize, map_step: u8) -> Option<i32> {
        todo!()
    }

    /// Adjusted damage expression with Enfeebled subtracted for melee and thrown Strikes
    /// (only Enfeebled affects damage), canonical dice form. `None` for an unknown index;
    /// empty string for a Strike without damage.
    pub fn strike_damage_expr(&self, index: usize) -> Option<String> {
        todo!()
    }

    /// Temporary HP absorb first; current HP stops at 0. Negative amounts do nothing.
    pub fn apply_damage(&mut self, amount: i32) {
        todo!()
    }

    /// Up to `effective_max_hp`. Negative amounts do nothing.
    pub fn heal(&mut self, amount: i32) {
        todo!()
    }

    /// The compact payload stored on the Owlbear Rodeo token under
    /// `com.owlbearbuilder/actor`: `{"kind": "monster", "id": instance_id, "name": label,
    /// "hp", "max_hp" (effective), "temp_hp", "ac" (effective), "level" (adjusted),
    /// "variant" (id), "conditions": [labels]}`.
    pub fn to_obr_token_metadata(&self) -> JsonValue {
        todo!()
    }

    /// Every field under its own name; `base` via `MonsterStatBlock::to_json`, `variant` as
    /// its id, conditions via `Condition::to_json`.
    pub fn to_json(&self) -> JsonValue {
        todo!()
    }

    pub fn from_json(v: &JsonValue) -> Result<MonsterInstance, String> {
        todo!()
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
