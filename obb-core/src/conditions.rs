// SPDX-License-Identifier: GPL-3.0-or-later
//! Conditions that change numbers on a sheet, plus a free-form entry for everything else a GM
//! wants to track (grabbed, slowed, persistent fire...).

use crate::character::Ability;
use crate::json::JsonValue;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    /// Status penalty to every check and DC; drops by 1 at the end of each of your turns.
    Frightened(u8),
    /// Status penalty to every check and DC.
    Sickened(u8),
    /// Status penalty to Dexterity-based checks and DCs, including AC and Reflex.
    Clumsy(u8),
    /// Status penalty to Strength-based checks, DCs and damage.
    Enfeebled(u8),
    /// Status penalty to Intelligence-, Wisdom- and Charisma-based checks and DCs, including
    /// Perception, Will, spell attacks and spell DCs.
    Stupefied(u8),
    /// Status penalty to Constitution-based checks (Fortitude); also lowers maximum HP.
    Drained(u8),
    /// -2 circumstance penalty to AC (called flat-footed before the Remaster).
    OffGuard,
    /// Circumstance bonus to AC from a raised shield; lasts until the start of your next turn.
    ShieldRaised(u8),
    /// Tracked for the table but changes no numbers here.
    Other { name: String, value: Option<u8> },
}

impl Condition {
    /// Builds a condition from a UI or JSON name, case-insensitive: `frightened`, `sickened`,
    /// `clumsy`, `enfeebled`, `stupefied`, `drained` (value required, 1 or more), `off-guard`
    /// (also `off_guard`, `offguard`, `flat-footed`), `shield-raised` (also `shield_raised`,
    /// `raised shield`; value is the shield's bonus, default 2). Any other non-empty name
    /// becomes `Other` with the name as given (trimmed). Errors: empty name, value 0 or
    /// missing for a valued condition.
    pub fn parse(name: &str, value: Option<u8>) -> Result<Condition, String> {
        todo!()
    }

    /// Stable id: `frightened`, ..., `off-guard`, `shield-raised`; `Other` returns its name.
    pub fn id(&self) -> &str {
        todo!()
    }

    pub fn value(&self) -> Option<u8> {
        todo!()
    }

    /// Display text: `Frightened 2`, `Off-Guard`, `Shield Raised (+2)`, `Grabbed`, `Persistent Fire 2`.
    pub fn label(&self) -> String {
        todo!()
    }

    /// `{"name": <id>, "value": <n>}`; `value` is omitted when there is none.
    pub fn to_json(&self) -> JsonValue {
        todo!()
    }

    pub fn from_json(v: &JsonValue) -> Result<Condition, String> {
        todo!()
    }
}

/// The status penalty (0 or negative) for a check or DC keyed to `ability`.
/// Player Core: status penalties don't stack, only the worst applies. Frightened and Sickened
/// apply to everything; Clumsy to Dex, Enfeebled to Str, Drained to Con, Stupefied to Int/Wis/Cha.
pub fn status_penalty(conditions: &[Condition], ability: Ability) -> i32 {
    todo!()
}

/// Net circumstance modifier to AC: the raised shield's bonus, minus 2 when off-guard. A bonus
/// and a penalty of the same type both apply; only the best bonus and worst penalty count.
pub fn ac_circumstance(conditions: &[Condition]) -> i32 {
    todo!()
}

/// Gains a condition. Player Core: gaining a valued condition you already have keeps the higher
/// value. A second `OffGuard` is a no-op; `ShieldRaised` and `Other` replace an entry with the
/// same id.
pub fn gain_condition(conditions: &mut Vec<Condition>, condition: Condition) {
    todo!()
}

/// Sets a condition's value directly, as a GM correcting the tracker would: replaces any entry
/// with the same id; `value == Some(0)` removes it. `name` goes through `Condition::parse`.
pub fn set_condition(
    conditions: &mut Vec<Condition>,
    name: &str,
    value: Option<u8>,
) -> Result<(), String> {
    todo!()
}

/// Removes every entry with this id (case-insensitive; aliases accepted as in `parse`).
pub fn remove_condition(conditions: &mut Vec<Condition>, name: &str) {
    todo!()
}

/// End of the creature's own turn: Frightened drops by 1 and is removed at 0.
pub fn tick_end_of_turn(conditions: &mut Vec<Condition>) {
    todo!()
}

/// Start of the creature's own turn: a raised shield lowers.
pub fn tick_start_of_turn(conditions: &mut Vec<Condition>) {
    todo!()
}
