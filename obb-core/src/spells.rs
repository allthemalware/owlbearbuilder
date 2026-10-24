// SPDX-License-Identifier: GPL-3.0-or-later
//! Spells as cast: base data from `spell_data`, heightened to the rank they're cast at, and
//! the list of spells a character can cast with those details filled in.

use crate::aoe::SpellArea;
use crate::character::CharacterSheet;
use crate::dice::DiceExpr;
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
        let mut value = JsonValue::object();
        value.insert("name", self.name.clone());
        value.insert("rank", u32::from(self.rank));
        value.insert("base_rank", u32::from(self.base_rank));
        value.insert("is_cantrip", self.is_cantrip);
        value.insert(
            "traditions",
            JsonValue::Array(
                self.traditions
                    .iter()
                    .cloned()
                    .map(JsonValue::from)
                    .collect(),
            ),
        );
        value.insert("actions", self.actions.clone());
        value.insert(
            "range_ft",
            self.range_ft.map_or(JsonValue::Null, JsonValue::from),
        );
        value.insert(
            "area",
            self.area
                .as_ref()
                .map_or(JsonValue::Null, SpellArea::to_json),
        );
        value.insert(
            "area_label",
            self.area
                .as_ref()
                .map_or(JsonValue::Null, |area| JsonValue::from(area.label())),
        );
        value.insert(
            "defense",
            self.defense
                .as_ref()
                .map_or(JsonValue::Null, |text| JsonValue::from(text.clone())),
        );
        value.insert(
            "damage_expr",
            self.damage_expr
                .as_ref()
                .map_or(JsonValue::Null, |text| JsonValue::from(text.clone())),
        );
        value.insert(
            "damage_type",
            self.damage_type
                .as_ref()
                .map_or(JsonValue::Null, |text| JsonValue::from(text.clone())),
        );
        value
    }
}

/// Cantrips are cast at half your level rounded up (Player Core), at least rank 1, at most 10.
pub fn cantrip_rank(level: i32) -> u8 {
    (level.clamp(0, 20) as u8).div_ceil(2).clamp(1, 10)
}

/// Case-insensitive name lookup in `COMMON_SPELLS`.
pub fn spell_data(name: &str) -> Option<&'static SpellData> {
    COMMON_SPELLS
        .iter()
        .find(|spell| spell.name.eq_ignore_ascii_case(name))
}

/// The spell as cast at `cast_rank` (raised to its base rank if lower; cantrips are rank 0 in
/// the data and take `cast_rank` as their heightened rank). Damage adds `heighten_damage` once
/// per full `heighten_every` ranks above the base (cantrips count from rank 1): dice of the
/// same size merge and constants add (`6d6` + 2×`2d6` → `10d6`; `1d4+1` + `1d4+1` →
/// `2d4+2`). `None` for spells not in the table.
pub fn lookup_common_spell(name: &str, cast_rank: u8) -> Option<SpellEntry> {
    let data = spell_data(name)?;
    Some(spell_at_rank(data, cast_rank))
}

fn spell_at_rank(data: &SpellData, cast_rank: u8) -> SpellEntry {
    let rank = cast_rank.max(data.rank);
    let is_cantrip = data.rank == 0;
    let steps = rank
        .saturating_sub(if is_cantrip { 1 } else { data.rank })
        .checked_div(data.heighten_every)
        .unwrap_or(0);
    let damage_expr = data.damage.map(|base| {
        let mut expression =
            DiceExpr::parse(base).expect("spell damage must be a valid dice expression");
        if steps > 0 {
            if let Some(heighten) = data.heighten_damage {
                let addition = DiceExpr::parse(heighten)
                    .expect("spell heightening damage must be a valid dice expression");
                for &(sign, count, sides) in &addition.dice {
                    let total = count * u32::from(steps);
                    if let Some((_, existing, _)) = expression
                        .dice
                        .iter_mut()
                        .find(|term| term.0 == sign && term.2 == sides)
                    {
                        *existing += total;
                    } else {
                        expression.dice.push((sign, total, sides));
                    }
                }
                expression.flat += addition.flat * i32::from(steps);
            }
        }
        expression.to_string()
    });
    SpellEntry {
        name: data.name.to_string(),
        rank,
        base_rank: data.rank,
        is_cantrip,
        traditions: data
            .traditions
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        actions: data.actions.to_string(),
        range_ft: data.range_ft,
        area: data.area.and_then(crate::aoe::parse_area_description),
        defense: data.defense.map(str::to_string),
        damage_expr,
        damage_type: data.damage_type.map(str::to_string),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heightening_merges_repeated_damage_terms_and_constants() {
        let data = SpellData {
            name: "Practice Spell",
            rank: 1,
            traditions: &[],
            actions: "1",
            range_ft: None,
            area: None,
            defense: None,
            damage: Some("1d4+1"),
            damage_type: None,
            heighten_every: 1,
            heighten_damage: Some("1d4+1"),
        };
        assert_eq!(
            spell_at_rank(&data, 4).damage_expr.as_deref(),
            Some("4d4+4")
        );
    }

    #[test]
    fn heightening_respects_multiple_rank_steps() {
        let data = SpellData {
            name: "Slowly Heightened",
            rank: 1,
            traditions: &[],
            actions: "1",
            range_ft: None,
            area: None,
            defense: None,
            damage: Some("1d6"),
            damage_type: None,
            heighten_every: 2,
            heighten_damage: Some("1d6"),
        };
        assert_eq!(spell_at_rank(&data, 5).damage_expr.as_deref(), Some("3d6"));
    }

    #[test]
    fn spell_without_damage_remains_damage_free() {
        let data = SpellData {
            name: "Utility",
            rank: 1,
            traditions: &[],
            actions: "1",
            range_ft: None,
            area: None,
            defense: None,
            damage: None,
            damage_type: None,
            heighten_every: 1,
            heighten_damage: Some("1d6"),
        };
        assert_eq!(spell_at_rank(&data, 5).damage_expr, None);
    }

    #[test]
    fn focus_cantrips_heighten_with_character_level() {
        let mut sheet = CharacterSheet::new("Caster", 7);
        sheet
            .spellcasting
            .push(crate::character::SpellcastingEntry {
                name: "Focus".into(),
                tradition: "arcane".into(),
                kind: "focus".into(),
                ability: crate::character::Ability::Int,
                rank: crate::character::ProfRank::Trained,
                focus_points: 1,
                spells: vec![(0, vec!["Electric Arc".into()])],
            });
        let book = spellbook(&sheet);
        assert_eq!(book[0].details.as_ref().unwrap().rank, 4);
    }
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
    sheet
        .spellcasting
        .iter()
        .enumerate()
        .flat_map(|(entry_index, entry)| {
            entry.spells.iter().flat_map(move |(listed_rank, names)| {
                names.iter().map(move |name| {
                    let rank = if *listed_rank == 0 {
                        cantrip_rank(sheet.level)
                    } else {
                        *listed_rank
                    };
                    KnownSpell {
                        entry: entry_index,
                        name: name.clone(),
                        listed_rank: *listed_rank,
                        details: lookup_common_spell(name, rank),
                    }
                })
            })
        })
        .collect()
}
