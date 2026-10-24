// SPDX-License-Identifier: GPL-3.0-or-later
//! Game data for common spells (Player Core, Remaster names), used to fill in what builder
//! exports leave out: an export lists spell names and ranks, not areas, saves or damage.

pub struct SpellData {
    pub name: &'static str,
    /// Base rank: 0 for cantrips.
    pub rank: u8,
    /// Lowercase: `arcane`, `divine`, `occult`, `primal`.
    pub traditions: &'static [&'static str],
    /// `1`, `2`, `3`, `1 to 3`, `reaction`, `free`.
    pub actions: &'static str,
    pub range_ft: Option<u32>,
    /// Area text understood by `aoe::parse_area_description`, e.g. `20-foot burst`.
    pub area: Option<&'static str>,
    /// `AC`, `Fortitude`, `Reflex`, `Will`, or `basic Fortitude` / `basic Reflex` / `basic Will`.
    pub defense: Option<&'static str>,
    /// Damage (or healing) at base rank, e.g. `6d6`.
    pub damage: Option<&'static str>,
    pub damage_type: Option<&'static str>,
    /// Heightening: every `heighten_every` ranks above base add `heighten_damage`. 0 = none.
    pub heighten_every: u8,
    pub heighten_damage: Option<&'static str>,
}

pub static COMMON_SPELLS: &[SpellData] = &[
    SpellData {
        name: "Electric Arc",
        rank: 0,
        traditions: &["arcane", "primal"],
        actions: "2",
        range_ft: Some(30),
        area: None,
        defense: Some("basic Reflex"),
        damage: Some("2d4"),
        damage_type: Some("electricity"),
        heighten_every: 1,
        heighten_damage: Some("1d4"),
    },
    SpellData {
        name: "Breathe Fire",
        rank: 1,
        traditions: &["arcane", "primal"],
        actions: "2",
        range_ft: None,
        area: Some("15-foot cone"),
        defense: Some("basic Reflex"),
        damage: Some("2d6"),
        damage_type: Some("fire"),
        heighten_every: 1,
        heighten_damage: Some("2d6"),
    },
    SpellData {
        name: "Fireball",
        rank: 3,
        traditions: &["arcane", "primal"],
        actions: "2",
        range_ft: Some(500),
        area: Some("20-foot burst"),
        defense: Some("basic Reflex"),
        damage: Some("6d6"),
        damage_type: Some("fire"),
        heighten_every: 1,
        heighten_damage: Some("2d6"),
    },
];
