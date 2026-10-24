// SPDX-License-Identifier: GPL-3.0-or-later
//! Dice expressions, the random source behind them, and Pathfinder's degrees of success.

use std::fmt;
use std::fs::File;
use std::io::Read;
use std::time::{SystemTime, UNIX_EPOCH};

/// Expressions with more dice or bigger dice than this are rejected: anything in the game fits
/// comfortably, and the limit stops a typo like `1000000d6` from stalling the server.
pub const MAX_DICE_PER_TERM: u32 = 100;
pub const MAX_DIE_SIDES: u32 = 1000;

/// SplitMix64. Seeded for tests and replays; `from_os` for live play.
#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn from_seed(seed: u64) -> Rng {
        Rng { state: seed }
    }

    /// Seeds from 8 bytes of `/dev/urandom`; where that is unavailable, from the system clock's
    /// nanoseconds mixed with the process id.
    pub fn from_os() -> Rng {
        let mut bytes = [0u8; 8];
        if File::open("/dev/urandom")
            .and_then(|mut file| file.read_exact(&mut bytes))
            .is_ok()
        {
            return Rng::from_seed(u64::from_ne_bytes(bytes));
        }
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as u64);
        Rng::from_seed(nanos ^ (std::process::id() as u64).rotate_left(32))
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    /// Uniform in `1..=sides`, without modulo bias (rejection sampling). Panics if `sides == 0`.
    pub fn roll_die(&mut self, sides: u32) -> u32 {
        assert!(sides > 0, "sides must be positive");
        let bound = u64::from(sides);
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let value = self.next_u64();
            if value >= threshold {
                return (value % bound) as u32 + 1;
            }
        }
    }
}

/// One `NdM` term of an expression, with its rolls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DieTermResult {
    pub count: u32,
    pub sides: u32,
    /// `-1` when the term is subtracted, as in `1d20-1d4`.
    pub sign: i32,
    pub rolls: Vec<u32>,
    /// `sign * sum(rolls)`.
    pub subtotal: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollResult {
    /// Canonical form of the expression, see `DiceExpr`'s `Display`.
    pub expr: String,
    pub terms: Vec<DieTermResult>,
    /// Sum of all constant terms.
    pub flat_modifier: i32,
    pub total: i32,
    /// The d20's face when the expression is a check: exactly one die term, and it is a
    /// positive `1d20`. `None` otherwise (damage, `2d20`, `1d20+1d4`, ...).
    pub natural_d20: Option<u8>,
}

/// A parsed expression such as `1d20+14`, `3d10+1d6+7`, `1d20-2`, `d6` or `2d8 + 4`.
///
/// Grammar: terms separated by `+` or `-` (the first may carry a sign); a term is `NdM`, `dM`
/// (meaning `1dM`) or an integer constant. Whitespace is ignored and `D` is accepted for `d`.
/// Errors: empty input, `0` dice or `d0`, counts above `MAX_DICE_PER_TERM`, sides above
/// `MAX_DIE_SIDES`, constants that overflow i32, and any other character.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiceExpr {
    /// `(sign, count, sides)` in source order.
    pub dice: Vec<(i32, u32, u32)>,
    pub flat: i32,
}

impl DiceExpr {
    pub fn parse(expr: &str) -> Result<DiceExpr, String> {
        let compact: String = expr.chars().filter(|c| !c.is_whitespace()).collect();
        if compact.is_empty() {
            return Err("empty expression".into());
        }
        if !compact.is_ascii() {
            return Err("unexpected character".into());
        }
        let chars: Vec<char> = compact.chars().collect();
        let mut at = 0;
        let mut dice = Vec::new();
        let mut flat = 0i32;
        let mut first = true;
        while at < chars.len() {
            let sign = if chars[at] == '+' || chars[at] == '-' {
                let s = if chars[at] == '-' { -1 } else { 1 };
                if !first && at > 0 && (chars[at - 1] == '+' || chars[at - 1] == '-') {
                    return Err("expected term".into());
                }
                at += 1;
                s
            } else {
                1
            };
            first = false;
            let start = at;
            while at < chars.len() && chars[at].is_ascii_digit() {
                at += 1;
            }
            let number: Option<u32> = if at > start {
                Some(
                    compact[start..at]
                        .parse()
                        .map_err(|_| "number out of range".to_string())?,
                )
            } else {
                None
            };
            if at < chars.len() && (chars[at] == 'd' || chars[at] == 'D') {
                at += 1;
                let sides_start = at;
                while at < chars.len() && chars[at].is_ascii_digit() {
                    at += 1;
                }
                if sides_start == at {
                    return Err("missing die sides".into());
                }
                let sides: u32 = compact[sides_start..at]
                    .parse()
                    .map_err(|_| "die sides out of range".to_string())?;
                let count = number.unwrap_or(1);
                if count == 0 {
                    return Err("dice count must be positive".into());
                }
                if count > MAX_DICE_PER_TERM {
                    return Err("too many dice".into());
                }
                if sides == 0 {
                    return Err("die sides must be positive".into());
                }
                if sides > MAX_DIE_SIDES {
                    return Err("die sides too large".into());
                }
                dice.push((sign, count, sides));
            } else if let Some(value) = number {
                let signed = if sign < 0 {
                    -(value as i64)
                } else {
                    value as i64
                };
                flat = i32::try_from(i64::from(flat) + signed)
                    .map_err(|_| "constant overflow".to_string())?;
            } else {
                return Err("expected term".into());
            }
            if at < chars.len() && chars[at] != '+' && chars[at] != '-' {
                return Err("unexpected character".into());
            }
            if at == chars.len() {
                break;
            }
            at += 1;
            if at == chars.len() || chars[at] == '+' || chars[at] == '-' {
                return Err("expected term".into());
            }
            // The next iteration consumes this operator as the term sign.
            at -= 1;
        }
        Ok(DiceExpr { dice, flat })
    }

    /// Rolls dice terms in source order, each die in turn, so a seed always gives the same rolls.
    pub fn roll(&self, rng: &mut Rng) -> RollResult {
        let mut terms = Vec::with_capacity(self.dice.len());
        let mut total = self.flat;
        for &(sign, count, sides) in &self.dice {
            let rolls: Vec<u32> = (0..count).map(|_| rng.roll_die(sides)).collect();
            let sum: i32 = rolls.iter().fold(0i32, |a, &b| a.saturating_add(b as i32));
            let subtotal = if sign < 0 { sum.saturating_neg() } else { sum };
            total = total.saturating_add(subtotal);
            terms.push(DieTermResult {
                count,
                sides,
                sign,
                rolls,
                subtotal,
            });
        }
        let natural_d20 = if self.dice.len() == 1 && self.dice[0] == (1, 1, 20) {
            terms[0].rolls.first().copied().map(|n| n as u8)
        } else {
            None
        };
        RollResult {
            expr: self.to_string(),
            terms,
            flat_modifier: self.flat,
            total,
            natural_d20,
        }
    }
}

/// Canonical form: dice terms in source order, then the summed constant if non-zero; no spaces;
/// `1dM` keeps its `1`. `" d20 + 4 + 3 "` → `1d20+7`, `"-2+1d6"` → `1d6-2`, `"5"` → `5`.
impl fmt::Display for DiceExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for &(sign, count, sides) in &self.dice {
            if first {
                if sign < 0 {
                    f.write_str("-")?;
                }
            } else {
                f.write_str(if sign < 0 { "-" } else { "+" })?;
            }
            write!(f, "{}d{}", count, sides)?;
            first = false;
        }
        if self.flat != 0 || first {
            if !first {
                f.write_str(if self.flat < 0 { "-" } else { "+" })?;
            }
            write!(f, "{}", self.flat.unsigned_abs())?;
        }
        Ok(())
    }
}

pub fn roll_expr(expr: &str, rng: &mut Rng) -> Result<RollResult, String> {
    Ok(DiceExpr::parse(expr)?.roll(rng))
}

/// Ordered worst to best, so `max`/`min` and comparisons read naturally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DegreeOfSuccess {
    CriticalFailure,
    Failure,
    Success,
    CriticalSuccess,
}

impl DegreeOfSuccess {
    /// One step better; `CriticalSuccess` stays put.
    pub fn step_up(self) -> DegreeOfSuccess {
        match self {
            Self::CriticalFailure => Self::Failure,
            Self::Failure => Self::Success,
            Self::Success => Self::CriticalSuccess,
            Self::CriticalSuccess => Self::CriticalSuccess,
        }
    }

    /// One step worse; `CriticalFailure` stays put.
    pub fn step_down(self) -> DegreeOfSuccess {
        match self {
            Self::CriticalFailure => Self::CriticalFailure,
            Self::Failure => Self::CriticalFailure,
            Self::Success => Self::Failure,
            Self::CriticalSuccess => Self::Success,
        }
    }

    /// `"Critical Success"`, `"Success"`, `"Failure"`, `"Critical Failure"`.
    pub fn label(self) -> &'static str {
        match self {
            Self::CriticalSuccess => "Critical Success",
            Self::Success => "Success",
            Self::Failure => "Failure",
            Self::CriticalFailure => "Critical Failure",
        }
    }

    /// Stable id for JSON: `"critical_success"`, `"success"`, `"failure"`, `"critical_failure"`.
    pub fn id(self) -> &'static str {
        match self {
            Self::CriticalSuccess => "critical_success",
            Self::Success => "success",
            Self::Failure => "failure",
            Self::CriticalFailure => "critical_failure",
        }
    }
}

/// Player Core, Degrees of Success: beat the DC by 10 or more for a critical success, miss it by
/// 10 or more for a critical failure. A natural 20 then improves the result one step and a
/// natural 1 worsens it one step, so the natural die adjusts the degree, not the total.
pub fn evaluate_check(total: i32, dc: i32, natural_d20: Option<u8>) -> DegreeOfSuccess {
    let mut degree = if total >= dc.saturating_add(10) {
        DegreeOfSuccess::CriticalSuccess
    } else if total <= dc.saturating_sub(10) {
        DegreeOfSuccess::CriticalFailure
    } else if total >= dc {
        DegreeOfSuccess::Success
    } else {
        DegreeOfSuccess::Failure
    };
    // Player Core applies the natural die after comparing the total, so it changes degree without changing the check total.
    degree = match natural_d20 {
        Some(20) => degree.step_up(),
        Some(1) => degree.step_down(),
        _ => degree,
    };
    degree
}

/// Player Core, Multiple Attack Penalty. `strike_index` counts attacks this turn from 0.
/// 0 → 0; 1 → -5 (agile -4); 2 or more → -10 (agile -8).
pub fn map_penalty(strike_index: u8, is_agile: bool) -> i32 {
    match strike_index {
        0 => 0,
        1 => {
            if is_agile {
                -4
            } else {
                -5
            }
        }
        _ => {
            if is_agile {
                -8
            } else {
                -10
            }
        }
    }
}

/// Weapon damage for one Strike.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrikeDamage {
    /// Weapon damage dice: 1, plus one per striking rune grade for PCs; as printed for creatures.
    pub dice_count: u32,
    pub die_sides: u32,
    /// Everything flat: ability modifier, weapon specialization, other bonuses and penalties.
    pub flat_bonus: i32,
    /// Die size of the weapon's deadly trait, if any.
    pub deadly: Option<u32>,
    /// Die size of the weapon's fatal trait, if any.
    pub fatal: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DamageResult {
    /// Human-readable breakdown, e.g. `2×(2d10+4) + 1d10` for a fatal d10 crit.
    pub expr: String,
    /// The weapon damage dice as rolled (on a fatal crit, already the fatal size).
    pub weapon_rolls: Vec<u32>,
    /// Dice added on a critical hit by deadly or fatal; never doubled.
    pub extra_rolls: Vec<u32>,
    pub total: i32,
    pub critical: bool,
}

impl StrikeDamage {
    /// Normal hit: `sum(weapon_rolls) + flat_bonus`.
    ///
    /// Critical hit (Player Core, Critical Hits; Weapon Traits):
    /// - fatal dX: every weapon die becomes a dX, the result is doubled, then one extra dX is
    ///   added (not doubled);
    /// - deadly dX: the result is doubled, then deadly dice are added (not doubled): 1 die,
    ///   2 with 3 weapon dice (greater striking), 3 with 4 or more (major striking);
    /// - otherwise `2 × (sum(weapon_rolls) + flat_bonus)`.
    /// A weapon with both traits applies fatal's die size, then both extra dice.
    ///
    /// Total is at least 1: Player Core says damage reduced below 1 still deals 1.
    /// Rolls in this order: weapon dice, then fatal's die, then deadly dice.
    pub fn roll(&self, critical: bool, rng: &mut Rng) -> DamageResult {
        let weapon_sides = if critical {
            self.fatal.unwrap_or(self.die_sides)
        } else {
            self.die_sides
        };
        let weapon_rolls: Vec<u32> = (0..self.dice_count)
            .map(|_| rng.roll_die(weapon_sides))
            .collect();
        let mut extra_rolls = Vec::new();
        if critical {
            if let Some(fatal) = self.fatal {
                extra_rolls.push(rng.roll_die(fatal));
            }
            // Player Core's deadly trait adds these dice after doubling the Strike's damage.
            if let Some(deadly) = self.deadly {
                for _ in 0..deadly_dice_count(self.dice_count) {
                    extra_rolls.push(rng.roll_die(deadly));
                }
            }
        }
        let weapon: i32 = weapon_rolls
            .iter()
            .fold(0, |a, &b| a.saturating_add(b as i32));
        let base = weapon.saturating_add(self.flat_bonus);
        let doubled = if critical {
            base.saturating_mul(2)
        } else {
            base
        };
        let extra: i32 = extra_rolls
            .iter()
            .fold(0, |a, &b| a.saturating_add(b as i32));
        let total = doubled.saturating_add(extra).max(1);
        let die_expr = format!("{}d{}", self.dice_count, weapon_sides);
        let base_expr = if self.flat_bonus == 0 {
            die_expr.clone()
        } else {
            format!(
                "{}{}{}",
                die_expr,
                if self.flat_bonus < 0 { "" } else { "+" },
                self.flat_bonus
            )
        };
        let expr = if !critical {
            base_expr
        } else {
            let mut parts = vec![format!("2×({})", base_expr)];
            if let Some(fatal) = self.fatal {
                parts.push(format!("1d{}", fatal));
            }
            if let Some(deadly) = self.deadly {
                let count = deadly_dice_count(self.dice_count);
                if count > 0 {
                    parts.push(format!("{}d{}", count, deadly));
                }
            }
            parts.join(" + ")
        };
        DamageResult {
            expr,
            weapon_rolls,
            extra_rolls,
            total,
            critical,
        }
    }
}

/// Deadly dice on a critical hit for a weapon with `dice_count` weapon damage dice.
pub fn deadly_dice_count(dice_count: u32) -> u32 {
    if dice_count >= 4 {
        3
    } else if dice_count == 3 {
        2
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn splitmix_seed_zero() {
        assert_eq!(Rng::from_seed(0).next_u64(), 0xE220A8397B1DCDAF);
    }
    #[test]
    fn one_sided_die() {
        let mut r = Rng::from_seed(0);
        assert_eq!(
            (0..100).map(|_| r.roll_die(1)).collect::<Vec<_>>(),
            vec![1; 100]
        );
    }
    #[test]
    fn constants_overflow_is_an_error() {
        assert!(DiceExpr::parse("2147483647+1")
            .unwrap_err()
            .contains("overflow"));
    }
    #[test]
    fn damage_expressions_are_readable() {
        let normal = StrikeDamage {
            dice_count: 2,
            die_sides: 8,
            flat_bonus: 4,
            deadly: None,
            fatal: None,
        };
        assert_eq!(normal.roll(false, &mut Rng::from_seed(1)).expr, "2d8+4");
        assert_eq!(normal.roll(true, &mut Rng::from_seed(1)).expr, "2×(2d8+4)");
        let deadly = StrikeDamage {
            deadly: Some(10),
            ..normal.clone()
        };
        assert_eq!(
            deadly.roll(true, &mut Rng::from_seed(1)).expr,
            "2×(2d8+4) + 1d10"
        );
        let fatal = StrikeDamage {
            fatal: Some(10),
            ..normal
        };
        assert_eq!(
            fatal.roll(true, &mut Rng::from_seed(1)).expr,
            "2×(2d10+4) + 1d10"
        );
    }
}
