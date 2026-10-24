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
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err("condition name is empty".into());
        }
        let key = trimmed.to_ascii_lowercase().replace(['_', ' '], "-");
        let valued = |label: &str, default: Option<u8>| -> Result<u8, String> {
            let n = value
                .or(default)
                .ok_or_else(|| format!("{label} requires a value"))?;
            if n == 0 {
                Err(format!("{label} value must be at least 1"))
            } else {
                Ok(n)
            }
        };
        Ok(match key.as_str() {
            "frightened" => Condition::Frightened(valued("frightened", None)?),
            "sickened" => Condition::Sickened(valued("sickened", None)?),
            "clumsy" => Condition::Clumsy(valued("clumsy", None)?),
            "enfeebled" => Condition::Enfeebled(valued("enfeebled", None)?),
            "stupefied" => Condition::Stupefied(valued("stupefied", None)?),
            "drained" => Condition::Drained(valued("drained", None)?),
            "off-guard" | "offguard" | "flat-footed" => Condition::OffGuard,
            "shield-raised" | "raised-shield" => {
                Condition::ShieldRaised(valued("shield-raised", Some(2))?)
            }
            _ => Condition::Other {
                name: trimmed.to_string(),
                value,
            },
        })
    }

    /// Stable id: `frightened`, ..., `off-guard`, `shield-raised`; `Other` returns its name.
    pub fn id(&self) -> &str {
        match self {
            Condition::Frightened(_) => "frightened",
            Condition::Sickened(_) => "sickened",
            Condition::Clumsy(_) => "clumsy",
            Condition::Enfeebled(_) => "enfeebled",
            Condition::Stupefied(_) => "stupefied",
            Condition::Drained(_) => "drained",
            Condition::OffGuard => "off-guard",
            Condition::ShieldRaised(_) => "shield-raised",
            Condition::Other { name, .. } => name,
        }
    }

    pub fn value(&self) -> Option<u8> {
        match self {
            Condition::Frightened(v)
            | Condition::Sickened(v)
            | Condition::Clumsy(v)
            | Condition::Enfeebled(v)
            | Condition::Stupefied(v)
            | Condition::Drained(v)
            | Condition::ShieldRaised(v) => Some(*v),
            Condition::OffGuard => None,
            Condition::Other { value, .. } => *value,
        }
    }

    /// Display text: `Frightened 2`, `Off-Guard`, `Shield Raised (+2)`, `Grabbed`, `Persistent Fire 2`.
    pub fn label(&self) -> String {
        match self {
            Condition::Frightened(v) => format!("Frightened {v}"),
            Condition::Sickened(v) => format!("Sickened {v}"),
            Condition::Clumsy(v) => format!("Clumsy {v}"),
            Condition::Enfeebled(v) => format!("Enfeebled {v}"),
            Condition::Stupefied(v) => format!("Stupefied {v}"),
            Condition::Drained(v) => format!("Drained {v}"),
            Condition::OffGuard => "Off-Guard".into(),
            Condition::ShieldRaised(v) => format!("Shield Raised (+{v})"),
            Condition::Other {
                name,
                value: Some(v),
            } => format!("{name} {v}"),
            Condition::Other { name, value: None } => name.clone(),
        }
    }

    /// `{"name": <id>, "value": <n>}`; `value` is omitted when there is none.
    pub fn to_json(&self) -> JsonValue {
        let mut o = JsonValue::object();
        o.insert("name", self.id());
        if let Some(v) = self.value() {
            o.insert("value", i64::from(v));
        }
        o
    }

    pub fn from_json(v: &JsonValue) -> Result<Condition, String> {
        let name = v
            .get_str("name")
            .ok_or_else(|| "condition name must be a string".to_string())?;
        let value = match v.get("value") {
            None => None,
            Some(x) => Some(
                u8::try_from(
                    x.as_i64()
                        .ok_or_else(|| "condition value must be an integer".to_string())?,
                )
                .map_err(|_| "condition value out of range".to_string())?,
            ),
        };
        Condition::parse(name, value)
    }
}

/// The status penalty (0 or negative) for a check or DC keyed to `ability`.
/// Player Core: status penalties don't stack, only the worst applies. Frightened and Sickened
/// apply to everything; Clumsy to Dex, Enfeebled to Str, Drained to Con, Stupefied to Int/Wis/Cha.
pub fn status_penalty(conditions: &[Condition], ability: Ability) -> i32 {
    let penalty = conditions
        .iter()
        .map(|c| {
            let n = match c {
                Condition::Frightened(v) | Condition::Sickened(v) => *v,
                Condition::Clumsy(v) if ability == Ability::Dex => *v,
                Condition::Enfeebled(v) if ability == Ability::Str => *v,
                Condition::Drained(v) if ability == Ability::Con => *v,
                Condition::Stupefied(v)
                    if matches!(ability, Ability::Int | Ability::Wis | Ability::Cha) =>
                {
                    *v
                }
                _ => 0,
            };
            i32::from(n)
        })
        .max()
        .unwrap_or(0);
    -penalty
}

/// Net circumstance modifier to AC: the raised shield's bonus, minus 2 when off-guard. A bonus
/// and a penalty of the same type both apply; only the best bonus and worst penalty count.
pub fn ac_circumstance(conditions: &[Condition]) -> i32 {
    let bonus = conditions
        .iter()
        .filter_map(|c| {
            if let Condition::ShieldRaised(v) = c {
                Some(i32::from(*v))
            } else {
                None
            }
        })
        .max()
        .unwrap_or(0);
    let penalty = if conditions.iter().any(|c| matches!(c, Condition::OffGuard)) {
        2
    } else {
        0
    };
    bonus - penalty
}

/// Gains a condition. Player Core: gaining a valued condition you already have keeps the higher
/// value. A second `OffGuard` is a no-op; `ShieldRaised` and `Other` replace an entry with the
/// same id.
pub fn gain_condition(conditions: &mut Vec<Condition>, condition: Condition) {
    let id = condition.id().to_ascii_lowercase();
    if let Some(i) = conditions
        .iter()
        .position(|c| c.id().eq_ignore_ascii_case(&id))
    {
        let keep_existing = match (&conditions[i], &condition) {
            (Condition::Frightened(a), Condition::Frightened(b)) => a >= b,
            (Condition::Sickened(a), Condition::Sickened(b)) => a >= b,
            (Condition::Clumsy(a), Condition::Clumsy(b)) => a >= b,
            (Condition::Enfeebled(a), Condition::Enfeebled(b)) => a >= b,
            (Condition::Stupefied(a), Condition::Stupefied(b)) => a >= b,
            (Condition::Drained(a), Condition::Drained(b)) => a >= b,
            (Condition::OffGuard, Condition::OffGuard) => true,
            _ => false,
        };
        if !keep_existing {
            conditions[i] = condition;
        }
    } else {
        conditions.push(condition);
    }
}

/// Sets a condition's value directly, as a GM correcting the tracker would: replaces any entry
/// with the same id; `value == Some(0)` removes it. `name` goes through `Condition::parse`.
pub fn set_condition(
    conditions: &mut Vec<Condition>,
    name: &str,
    value: Option<u8>,
) -> Result<(), String> {
    if value == Some(0) {
        // Validate names without requiring a value, since zero is the tracker’s remove command.
        Condition::parse(name, Some(1))?;
        remove_condition(conditions, name);
        return Ok(());
    }
    let condition = Condition::parse(name, value)?;
    if let Some(index) = conditions
        .iter()
        .position(|existing| existing.id().eq_ignore_ascii_case(condition.id()))
    {
        conditions[index] = condition;
    } else {
        conditions.push(condition);
    }
    Ok(())
}

/// Removes every entry with this id (case-insensitive; aliases accepted as in `parse`).
pub fn remove_condition(conditions: &mut Vec<Condition>, name: &str) {
    let normalized = name.trim().to_ascii_lowercase().replace(['_', ' '], "-");
    let id = match normalized.as_str() {
        "flat-footed" | "offguard" => "off-guard",
        "raised-shield" => "shield-raised",
        _ => normalized.as_str(),
    };
    conditions.retain(|c| !c.id().eq_ignore_ascii_case(id));
}

/// End of the creature's own turn: Frightened drops by 1 and is removed at 0.
pub fn tick_end_of_turn(conditions: &mut Vec<Condition>) {
    for c in conditions.iter_mut() {
        if let Condition::Frightened(n) = c {
            *n = n.saturating_sub(1);
        }
    }
    conditions.retain(|c| !matches!(c, Condition::Frightened(0)));
}

/// Start of the creature's own turn: a raised shield lowers.
pub fn tick_start_of_turn(conditions: &mut Vec<Condition>) {
    conditions.retain(|c| !matches!(c, Condition::ShieldRaised(_)));
}
