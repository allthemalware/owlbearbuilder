// SPDX-License-Identifier: GPL-3.0-or-later
//! Initiative order, turns and rounds, plus the party-wide views a GM wants at a glance.

use crate::character::CharacterSheet;
use crate::conditions::{self, Condition};
use crate::dice::{DegreeOfSuccess, Rng, RollResult};
use crate::json::JsonValue;

/// Anything whose conditions tick as turns pass: player characters now, monsters later.
pub trait Actor {
    /// The id a `Combatant::actor_id` points at.
    fn actor_id(&self) -> &str;
    fn conditions_mut(&mut self) -> &mut Vec<Condition>;
}

impl Actor for CharacterSheet {
    fn actor_id(&self) -> &str {
        &self.id
    }

    fn conditions_mut(&mut self) -> &mut Vec<Condition> {
        &mut self.conditions
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Combatant {
    /// Unique within the encounter (several goblins share an actor template but not this).
    pub combatant_id: String,
    /// The `CharacterSheet::id` or monster instance id whose conditions tick on this turn.
    pub actor_id: String,
    pub name: String,
    pub is_player: bool,
    /// `None` until rolled or set.
    pub initiative: Option<i32>,
    /// The modifier the roll used; breaks ties within a side.
    pub initiative_mod: i32,
    /// e.g. `Perception (+11)` or `Stealth (+13)`.
    pub initiative_source: String,
    pub token_id: Option<String>,
}

impl Combatant {
    /// A combatant with no initiative yet, `initiative_mod` 0, empty source and no token.
    pub fn new(combatant_id: &str, actor_id: &str, name: &str, is_player: bool) -> Combatant {
        Combatant {
            combatant_id: combatant_id.to_string(),
            actor_id: actor_id.to_string(),
            name: name.to_string(),
            is_player,
            initiative: None,
            initiative_mod: 0,
            initiative_source: String::new(),
            token_id: None,
        }
    }
}

/// What changed when the turn passed, so callers can announce it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnTransition {
    /// Combatant whose turn ended, if any.
    pub ended: Option<String>,
    /// Combatant whose turn starts, if any (none when the order is empty).
    pub started: Option<String>,
    pub round: u32,
    pub new_round: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncounterTracker {
    /// 0 before the encounter starts, then 1, 2, ...
    pub round: u32,
    /// Index into `combatants` of whose turn it is; meaningful only while `active`.
    pub active_index: usize,
    pub active: bool,
    pub combatants: Vec<Combatant>,
}

impl Default for EncounterTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl EncounterTracker {
    pub fn new() -> EncounterTracker {
        EncounterTracker {
            round: 0,
            active_index: 0,
            active: false,
            combatants: Vec::new(),
        }
    }

    /// Errors if the id is already present. Before the encounter starts the combatant is just
    /// appended (`start` sorts). Joining mid-encounter is allowed: the newcomer goes before the
    /// first combatant with a lower initiative (so after equal totals; no initiative counts as
    /// lowest), without changing whose turn it is.
    pub fn add_combatant(&mut self, combatant: Combatant) -> Result<(), String> {
        todo!()
    }

    /// Removes a combatant. If it was acting, the turn passes to whoever now holds its place
    /// (no condition ticks: a removed creature's turn didn't end normally). `None` if unknown.
    pub fn remove_combatant(&mut self, combatant_id: &str) -> Option<Combatant> {
        todo!()
    }

    /// Records a rolled or GM-set initiative. Re-sorts only while the encounter isn't active;
    /// during an encounter use `delay_combatant`. Errors for an unknown id.
    pub fn set_initiative(
        &mut self,
        combatant_id: &str,
        total: i32,
        modifier: i32,
        source: &str,
    ) -> Result<(), String> {
        todo!()
    }

    /// Rolls `1d20 + modifier` for a character's combatant using `initiative_modifier`, stores
    /// the result via `set_initiative`, and returns the total, the roll, and a breakdown such
    /// as `Rolled 14 + 11 (Perception) = 25`. Errors for an unknown id or check name.
    pub fn roll_initiative_for_character(
        &mut self,
        combatant_id: &str,
        sheet: &CharacterSheet,
        check: &str,
        rng: &mut Rng,
    ) -> Result<(i32, RollResult, String), String> {
        todo!()
    }

    /// Descending by initiative; combatants without initiative go last. Ties: the GM's
    /// creatures act before player characters (GM Core), then higher `initiative_mod`, then
    /// name, then id, so the order is always deterministic.
    pub fn sort_initiative(&mut self) {
        todo!()
    }

    /// Sorts, sets round 1, gives the first combatant the turn, and reports it as
    /// `{ended: None, started: <first>, round: 1, new_round: true}`. Errors when there are no
    /// combatants.
    pub fn start(&mut self) -> Result<TurnTransition, String> {
        todo!()
    }

    /// Ends the active combatant's turn and starts the next one, wrapping to a new round.
    /// The ending actor's conditions get `tick_end_of_turn`; the starting actor's get
    /// `tick_start_of_turn`. Actors are matched by `actor_id`; combatants with no matching
    /// actor are skipped silently. When inactive it changes nothing and returns
    /// `{ended: None, started: None, round, new_round: false}`.
    pub fn next_turn(&mut self, actors: &mut [&mut dyn Actor]) -> TurnTransition {
        todo!()
    }

    /// Delay (Player Core): the combatant leaves its place and rejoins with `new_initiative`,
    /// before the first combatant whose initiative is equal or lower (no initiative counts as
    /// lowest). Whose turn it is doesn't change, unless the delaying combatant is the one
    /// acting: then the turn passes to whoever followed it, with no condition ticks.
    /// Errors for an unknown id.
    pub fn delay_combatant(
        &mut self,
        combatant_id: &str,
        new_initiative: i32,
    ) -> Result<(), String> {
        todo!()
    }

    /// Stops the encounter: inactive, round 0, initiatives kept.
    pub fn end(&mut self) {
        todo!()
    }

    pub fn active_combatant(&self) -> Option<&Combatant> {
        todo!()
    }

    /// `{"round", "active_index", "active", "combatants": [{every Combatant field}]}`.
    pub fn to_json(&self) -> JsonValue {
        todo!()
    }

    pub fn from_json(v: &JsonValue) -> Result<EncounterTracker, String> {
        todo!()
    }
}

/// The modifier and label for an initiative check: `perception` (any case) or any skill or
/// Lore name `CharacterSheet::skill_modifier` accepts, conditions included. The label is the
/// check's display name with the signed modifier, e.g. `Perception (+11)`, `Stealth (+13)`,
/// `Warfare Lore (+7)`.
pub fn initiative_modifier(sheet: &CharacterSheet, check: &str) -> Option<(i32, String)> {
    todo!()
}

/// One row of the GM's party overview, conditions applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyOverviewRow {
    pub id: String,
    pub name: String,
    pub class: String,
    pub level: i32,
    pub current_hp: i32,
    /// `CharacterSheet::effective_max_hp`.
    pub max_hp: i32,
    pub temp_hp: i32,
    pub ac: i32,
    pub fort: i32,
    pub reflex: i32,
    pub will: i32,
    pub perception: i32,
    /// 10 + Perception, the DC to sneak past or lie to them.
    pub perception_dc: i32,
    /// 10 + Stealth, the DC to notice them while they Avoid Notice.
    pub stealth_dc: i32,
    pub class_dc: i32,
    pub speed: i32,
    pub hero_points: u8,
    /// `Condition::label` of each condition, in order.
    pub conditions: Vec<String>,
    pub linked_token_id: Option<String>,
}

impl PartyOverviewRow {
    /// Every field under its own name.
    pub fn to_json(&self) -> JsonValue {
        todo!()
    }
}

pub fn build_party_overview(sheets: &[CharacterSheet]) -> Vec<PartyOverviewRow> {
    todo!()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretCheckOutcome {
    pub id: String,
    pub name: String,
    pub modifier: i32,
    pub roll: RollResult,
    pub total: i32,
    /// Present when a DC was given.
    pub degree: Option<DegreeOfSuccess>,
}

/// Rolls `1d20 + modifier` once per character, in sheet order, for a GM's secret check
/// (Perception to notice, Recall Knowledge, ...). `check` resolves, case-insensitively, as:
/// `perception`; a save (`Save::parse`); a skill or Lore the sheet knows (`skill_modifier`);
/// or a name ending in ` lore` the character lacks, rolled untrained as Int modifier plus
/// Int's status penalty. Anything else is an error, checked before any dice are rolled.
/// With a DC, each outcome carries `evaluate_check(total, dc, natural_d20)`.
pub fn roll_party_secret_check(
    sheets: &[CharacterSheet],
    check: &str,
    dc: Option<i32>,
    rng: &mut Rng,
) -> Result<Vec<SecretCheckOutcome>, String> {
    todo!()
}
