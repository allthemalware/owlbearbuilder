// SPDX-License-Identifier: GPL-3.0-or-later
//! Spells as cast: base data from `spell_data`, heightened to the rank they're cast at, and
//! the list of spells a character can cast with those details filled in.

use crate::aoe::SpellArea;
use crate::character::CharacterSheet;
use crate::json::JsonValue;
use crate::spell_data::{SpellData, COMMON_SPELLS};

#[derive(Debug, Clone, PartialEq)]
pub struct SpellEntry {
    pub name: String,
    /// The rank it is cast at (cantrips: their heightened rank).
    pub rank: u8,
    pub base_rank: u8,
    pub is_cantrip: bool,
    pub traditions: Vec<String>,
    pub actions: String,
    pub range_ft: Option<u32>,
    pub area: Option<SpellArea>,
    pub defense: Option<String>,
    /// Heightened damage in canonical dice form, e.g. `8d6` for Fireball at rank 4.
    pub damage_expr: Option<String>,
    pub damage_type: Option<String>,
}

impl SpellEntry {
    /// Every field under its own name; `area` via `SpellArea::to_json` (null when absent),
    /// `area_label` added as `SpellArea::label` or null.
    pub fn to_json(&self) -> JsonValue {
        todo!()
    }
}

/// Cantrips are cast at half your level rounded up (Player Core), at least rank 1, at most 10.
pub fn cantrip_rank(level: i32) -> u8 {
    todo!()
}

/// Case-insensitive name lookup in `COMMON_SPELLS`.
pub fn spell_data(name: &str) -> Option<&'static SpellData> {
    todo!()
}

/// The spell as cast at `cast_rank` (raised to its base rank if lower; cantrips are rank 0 in
/// the data and take `cast_rank` as their heightened rank). Damage adds `heighten_damage` once
/// per full `heighten_every` ranks above the base (cantrips count from rank 1): dice of the
/// same size merge and constants add (`6d6` + 2×`2d6` → `10d6`; `1d4+1` + `1d4+1` →
/// `2d4+2`). `None` for spells not in the table.
pub fn lookup_common_spell(name: &str, cast_rank: u8) -> Option<SpellEntry> {
    todo!()
}

/// One spell a character can cast.
#[derive(Debug, Clone, PartialEq)]
pub struct KnownSpell {
    /// Index into `CharacterSheet::spellcasting`.
    pub entry: usize,
    pub name: String,
    /// The rank it is listed at on the sheet (0 for cantrips).
    pub listed_rank: u8,
    /// Details heightened to the listed rank (cantrips to `cantrip_rank(level)`; focus spells
    /// to the listed rank). `None` when the spell isn't in the table.
    pub details: Option<SpellEntry>,
}

/// Every spell on the sheet, in entry order then listed order, with details where known.
pub fn spellbook(sheet: &CharacterSheet) -> Vec<KnownSpell> {
    todo!()
}
