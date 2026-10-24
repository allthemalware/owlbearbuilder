// SPDX-License-Identifier: GPL-3.0-or-later
//! Traits of common weapons. Builder exports name the weapon and its dice but usually not its
//! traits, and the traits decide the multiple attack penalty (agile), which ability attacks
//! (finesse, ranged), and critical damage (deadly, fatal).

pub struct WeaponInfo {
    /// Base weapon name as printed in Player Core, e.g. `Composite Longbow`.
    pub name: &'static str,
    /// Lowercase traits with their values, e.g. `agile`, `deadly d10`, `thrown 20 ft`,
    /// `volley 30 ft`, `versatile p`, `two-hand d12`.
    pub traits: &'static [&'static str],
    /// Range increment in feet for ranged weapons. Melee weapons that can be thrown keep `None`
    /// here and carry a `thrown N ft` trait instead.
    pub range_ft: Option<u32>,
}

/// Looks a weapon up by name, case-insensitive, ignoring surrounding whitespace and a trailing
/// parenthetical (`Longsword (cold iron)` → `Longsword`). `None` for unknown weapons.
pub fn weapon_info(name: &str) -> Option<&'static WeaponInfo> {
    todo!()
}
