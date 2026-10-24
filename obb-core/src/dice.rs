// SPDX-License-Identifier: GPL-3.0-or-later
//! Dice expressions, the random source behind them, and Pathfinder's degrees of success.

use std::fmt;

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
        todo!()
    }

    pub fn next_u64(&mut self) -> u64 {
        todo!()
    }

    /// Uniform in `1..=sides`, without modulo bias (rejection sampling). Panics if `sides == 0`.
    pub fn roll_die(&mut self, sides: u32) -> u32 {
        todo!()
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
        todo!()
    }

    /// Rolls dice terms in source order, each die in turn, so a seed always gives the same rolls.
    pub fn roll(&self, rng: &mut Rng) -> RollResult {
        todo!()
    }
}

/// Canonical form: dice terms in source order, then the summed constant if non-zero; no spaces;
/// `1dM` keeps its `1`. `" d20 + 4 + 3 "` → `1d20+7`, `"-2+1d6"` → `1d6-2`, `"5"` → `5`.
impl fmt::Display for DiceExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
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
        todo!()
    }

    /// One step worse; `CriticalFailure` stays put.
    pub fn step_down(self) -> DegreeOfSuccess {
        todo!()
    }

    /// `"Critical Success"`, `"Success"`, `"Failure"`, `"Critical Failure"`.
    pub fn label(self) -> &'static str {
        todo!()
    }

    /// Stable id for JSON: `"critical_success"`, `"success"`, `"failure"`, `"critical_failure"`.
    pub fn id(self) -> &'static str {
        todo!()
    }
}

/// Player Core, Degrees of Success: beat the DC by 10 or more for a critical success, miss it by
/// 10 or more for a critical failure. A natural 20 then improves the result one step and a
/// natural 1 worsens it one step, so the natural die adjusts the degree, not the total.
pub fn evaluate_check(total: i32, dc: i32, natural_d20: Option<u8>) -> DegreeOfSuccess {
    todo!()
}

/// Player Core, Multiple Attack Penalty. `strike_index` counts attacks this turn from 0.
/// 0 → 0; 1 → -5 (agile -4); 2 or more → -10 (agile -8).
pub fn map_penalty(strike_index: u8, is_agile: bool) -> i32 {
    todo!()
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
        todo!()
    }
}

/// Deadly dice on a critical hit for a weapon with `dice_count` weapon damage dice.
pub fn deadly_dice_count(dice_count: u32) -> u32 {
    todo!()
}
