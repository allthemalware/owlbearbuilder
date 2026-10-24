// SPDX-License-Identifier: GPL-3.0-or-later
//! Initiative order, turns and rounds, plus the party-wide views a GM wants at a glance.

use crate::character::{Ability, CharacterSheet, Save, Skill};
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
        if self
            .combatants
            .iter()
            .any(|existing| existing.combatant_id == combatant.combatant_id)
        {
            return Err(format!(
                "combatant '{}' already exists",
                combatant.combatant_id
            ));
        }
        if !self.active {
            self.combatants.push(combatant);
            return Ok(());
        }
        let active_id = self.active_combatant().map(|c| c.combatant_id.clone());
        let index = self
            .combatants
            .iter()
            .position(
                |existing| match (combatant.initiative, existing.initiative) {
                    (Some(new), Some(old)) => old < new,
                    (Some(_), None) => true,
                    _ => false,
                },
            )
            .unwrap_or(self.combatants.len());
        self.combatants.insert(index, combatant);
        if let Some(id) = active_id {
            self.active_index = self.index_of(&id).unwrap_or(0);
        }
        Ok(())
    }

    /// Removes a combatant. If it was acting, the turn passes to whoever now holds its place
    /// (no condition ticks: a removed creature's turn didn't end normally). `None` if unknown.
    pub fn remove_combatant(&mut self, combatant_id: &str) -> Option<Combatant> {
        let index = self.index_of(combatant_id)?;
        let removed = self.combatants.remove(index);
        if self.combatants.is_empty() {
            self.active_index = 0;
            self.active = false;
            return Some(removed);
        }
        if self.active {
            if index < self.active_index {
                self.active_index -= 1;
            } else if index == self.active_index {
                // Removing an actor does not end its turn normally, so no conditions tick.
                if self.active_index >= self.combatants.len() {
                    self.active_index = 0;
                    self.round = self.round.saturating_add(1);
                }
            }
        } else {
            self.active_index = 0;
        }
        Some(removed)
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
        let index = self
            .index_of(combatant_id)
            .ok_or_else(|| format!("unknown combatant '{combatant_id}'"))?;
        let combatant = &mut self.combatants[index];
        combatant.initiative = Some(total);
        combatant.initiative_mod = modifier;
        combatant.initiative_source = source.to_string();
        if !self.active {
            self.sort_initiative();
        }
        Ok(())
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
        let (modifier, source) = initiative_modifier(sheet, check)
            .ok_or_else(|| format!("unknown initiative check '{check}'"))?;
        if self.index_of(combatant_id).is_none() {
            return Err(format!("unknown combatant '{combatant_id}'"));
        }
        let roll = crate::dice::roll_expr(&format!("1d20{modifier:+}"), rng)?;
        let total = roll.total;
        let breakdown = format!(
            "Rolled {} + {} ({}) = {}",
            roll.natural_d20.unwrap_or(0),
            modifier,
            source.trim_end_matches(&format!(" ({modifier:+})")),
            total
        );
        self.set_initiative(combatant_id, total, modifier, &source)?;
        Ok((total, roll, breakdown))
    }

    /// Descending by initiative; combatants without initiative go last. Ties: the GM's
    /// creatures act before player characters (GM Core), then higher `initiative_mod`, then
    /// name, then id, so the order is always deterministic.
    pub fn sort_initiative(&mut self) {
        let active_id = self
            .active_combatant()
            .map(|combatant| combatant.combatant_id.clone());
        self.combatants.sort_by(|a, b| {
            b.initiative
                .cmp(&a.initiative)
                // GM Core: on a tie between a player character and a GM-controlled creature,
                // the creature acts first.
                .then_with(|| a.is_player.cmp(&b.is_player))
                .then_with(|| b.initiative_mod.cmp(&a.initiative_mod))
                .then_with(|| a.name.cmp(&b.name))
                .then_with(|| a.combatant_id.cmp(&b.combatant_id))
        });
        if let Some(id) = active_id {
            self.active_index = self.index_of(&id).unwrap_or(0);
        }
    }

    /// Sorts, sets round 1, gives the first combatant the turn, and reports it as
    /// `{ended: None, started: <first>, round: 1, new_round: true}`. Errors when there are no
    /// combatants.
    pub fn start(&mut self) -> Result<TurnTransition, String> {
        if self.combatants.is_empty() {
            return Err("cannot start an encounter without combatants".into());
        }
        self.sort_initiative();
        self.round = 1;
        self.active_index = 0;
        self.active = true;
        Ok(TurnTransition {
            ended: None,
            started: Some(self.combatants[0].combatant_id.clone()),
            round: self.round,
            new_round: true,
        })
    }

    /// Ends the active combatant's turn and starts the next one, wrapping to a new round.
    /// The ending actor's conditions get `tick_end_of_turn`; the starting actor's get
    /// `tick_start_of_turn`. Actors are matched by `actor_id`; combatants with no matching
    /// actor are skipped silently. When inactive it changes nothing and returns
    /// `{ended: None, started: None, round, new_round: false}`.
    pub fn next_turn(&mut self, actors: &mut [&mut dyn Actor]) -> TurnTransition {
        if !self.active {
            return TurnTransition {
                ended: None,
                started: None,
                round: self.round,
                new_round: false,
            };
        }
        let ended_index = self.active_index;
        let ended_combatant = &self.combatants[ended_index];
        let ended_id = ended_combatant.combatant_id.clone();
        tick_actor(actors, &ended_combatant.actor_id, false);
        self.active_index += 1;
        let new_round = self.active_index >= self.combatants.len();
        if new_round {
            self.active_index = 0;
            self.round = self.round.saturating_add(1);
        }
        let started_combatant = &self.combatants[self.active_index];
        let started_id = started_combatant.combatant_id.clone();
        tick_actor(actors, &started_combatant.actor_id, true);
        TurnTransition {
            ended: Some(ended_id),
            started: Some(started_id),
            round: self.round,
            new_round,
        }
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
        let old_index = self
            .index_of(combatant_id)
            .ok_or_else(|| format!("unknown combatant '{combatant_id}'"))?;
        let was_active = self.active && old_index == self.active_index;
        let next_id = if was_active && self.combatants.len() > 1 {
            // Delaying moves a turn in the order; it does not end that creature's turn.
            Some(
                self.combatants[(old_index + 1) % self.combatants.len()]
                    .combatant_id
                    .clone(),
            )
        } else {
            None
        };
        let active_id = (!was_active)
            .then(|| self.active_combatant().map(|c| c.combatant_id.clone()))
            .flatten();
        let mut combatant = self.combatants.remove(old_index);
        combatant.initiative = Some(new_initiative);
        let insert_at = self
            .combatants
            .iter()
            .position(|other| {
                other
                    .initiative
                    .is_none_or(|initiative| initiative <= new_initiative)
            })
            .unwrap_or(self.combatants.len());
        self.combatants.insert(insert_at, combatant);
        if !self.active {
            self.active_index = 0;
        } else if was_active {
            if self.combatants.len() == 1 {
                self.active_index = 0;
                self.round = self.round.saturating_add(1);
            } else {
                self.active_index = self.index_of(next_id.as_deref().unwrap()).unwrap_or(0);
                if old_index == self.combatants.len() - 1 {
                    self.round = self.round.saturating_add(1);
                }
            }
        } else {
            self.active_index = active_id.and_then(|id| self.index_of(&id)).unwrap_or(0);
        }
        Ok(())
    }

    /// Stops the encounter: inactive, round 0, initiatives kept.
    pub fn end(&mut self) {
        self.active = false;
        self.round = 0;
        self.active_index = 0;
    }

    pub fn active_combatant(&self) -> Option<&Combatant> {
        self.active
            .then(|| self.combatants.get(self.active_index))
            .flatten()
    }

    /// `{"round", "active_index", "active", "combatants": [{every Combatant field}]}`.
    pub fn to_json(&self) -> JsonValue {
        let mut value = JsonValue::object();
        value.insert("round", i64::from(self.round));
        value.insert("active_index", self.active_index as i64);
        value.insert("active", self.active);
        value.insert(
            "combatants",
            JsonValue::Array(self.combatants.iter().map(combatant_to_json).collect()),
        );
        value
    }

    pub fn from_json(v: &JsonValue) -> Result<EncounterTracker, String> {
        let round = json_u32(v, "round")?;
        let active_index = json_usize(v, "active_index")?;
        let active = v
            .get_bool("active")
            .ok_or_else(|| "active must be a boolean".to_string())?;
        let values = v
            .get_array("combatants")
            .ok_or_else(|| "combatants must be an array".to_string())?;
        let combatants = values
            .iter()
            .map(combatant_from_json)
            .collect::<Result<Vec<_>, _>>()?;
        if active && active_index >= combatants.len() {
            return Err("active_index is out of range".into());
        }
        Ok(EncounterTracker {
            round,
            active_index,
            active,
            combatants,
        })
    }

    fn index_of(&self, id: &str) -> Option<usize> {
        self.combatants.iter().position(|c| c.combatant_id == id)
    }
}

/// The modifier and label for an initiative check: `perception` (any case) or any skill or
/// Lore name `CharacterSheet::skill_modifier` accepts, conditions included. The label is the
/// check's display name with the signed modifier, e.g. `Perception (+11)`, `Stealth (+13)`,
/// `Warfare Lore (+7)`.
pub fn initiative_modifier(sheet: &CharacterSheet, check: &str) -> Option<(i32, String)> {
    let modifier_and_label = if check.trim().eq_ignore_ascii_case("perception") {
        Some((sheet.perception_modifier(), "Perception".to_string()))
    } else {
        sheet.skill_modifier(check).map(|modifier| {
            let label = Skill::parse(check).map_or_else(
                || {
                    sheet
                        .lores
                        .iter()
                        .find(|l| {
                            check.trim().eq_ignore_ascii_case(&l.name)
                                || check
                                    .trim()
                                    .eq_ignore_ascii_case(&format!("{} lore", l.name))
                        })
                        .map(|l| format!("{} Lore", l.name))
                        .unwrap_or_else(|| check.trim().to_string())
                },
                |skill| skill.label().to_string(),
            );
            (modifier, label)
        })
    }?;
    Some((
        modifier_and_label.0,
        format!("{} ({:+})", modifier_and_label.1, modifier_and_label.0),
    ))
}

fn tick_actor(actors: &mut [&mut dyn Actor], actor_id: &str, start: bool) {
    if let Some(actor) = actors.iter_mut().find(|actor| actor.actor_id() == actor_id) {
        if start {
            conditions::tick_start_of_turn(actor.conditions_mut());
        } else {
            conditions::tick_end_of_turn(actor.conditions_mut());
        }
    }
}

fn combatant_to_json(combatant: &Combatant) -> JsonValue {
    let mut value = JsonValue::object();
    value.insert("combatant_id", combatant.combatant_id.as_str());
    value.insert("actor_id", combatant.actor_id.as_str());
    value.insert("name", combatant.name.as_str());
    value.insert("is_player", combatant.is_player);
    value.insert(
        "initiative",
        combatant
            .initiative
            .map_or(JsonValue::Null, JsonValue::from),
    );
    value.insert("initiative_mod", combatant.initiative_mod);
    value.insert("initiative_source", combatant.initiative_source.as_str());
    value.insert(
        "token_id",
        combatant
            .token_id
            .clone()
            .map_or(JsonValue::Null, JsonValue::from),
    );
    value
}

fn combatant_from_json(value: &JsonValue) -> Result<Combatant, String> {
    let combatant_id = json_string(value, "combatant_id")?;
    let actor_id = json_string(value, "actor_id")?;
    let name = json_string(value, "name")?;
    let is_player = value
        .get_bool("is_player")
        .ok_or_else(|| "is_player must be a boolean".to_string())?;
    let initiative = match value.get("initiative") {
        Some(JsonValue::Null) => None,
        Some(value) => Some(json_i32_value(value, "initiative")?),
        None => return Err("missing field 'initiative'".into()),
    };
    let initiative_mod = json_i32(value, "initiative_mod")?;
    let initiative_source = json_string(value, "initiative_source")?;
    let token_id = match value.get("token_id") {
        Some(JsonValue::Null) => None,
        Some(value) => Some(
            value
                .as_str()
                .ok_or_else(|| "token_id must be a string or null".to_string())?
                .to_string(),
        ),
        None => return Err("missing field 'token_id'".into()),
    };
    Ok(Combatant {
        combatant_id,
        actor_id,
        name,
        is_player,
        initiative,
        initiative_mod,
        initiative_source,
        token_id,
    })
}

fn json_string(value: &JsonValue, key: &str) -> Result<String, String> {
    value
        .get_str(key)
        .map(str::to_string)
        .ok_or_else(|| format!("{key} must be a string"))
}

fn json_i32(value: &JsonValue, key: &str) -> Result<i32, String> {
    value
        .get(key)
        .ok_or_else(|| format!("missing field '{key}'"))
        .and_then(|v| json_i32_value(v, key))
}

fn json_i32_value(value: &JsonValue, key: &str) -> Result<i32, String> {
    value
        .as_i64()
        .and_then(|number| i32::try_from(number).ok())
        .ok_or_else(|| format!("{key} must be a 32-bit integer"))
}

fn json_u32(value: &JsonValue, key: &str) -> Result<u32, String> {
    value
        .get(key)
        .ok_or_else(|| format!("missing field '{key}'"))
        .and_then(|v| {
            v.as_i64()
                .and_then(|n| u32::try_from(n).ok())
                .ok_or_else(|| format!("{key} must be a non-negative integer"))
        })
}

fn json_usize(value: &JsonValue, key: &str) -> Result<usize, String> {
    value
        .get(key)
        .ok_or_else(|| format!("missing field '{key}'"))
        .and_then(|v| {
            v.as_i64()
                .and_then(|n| usize::try_from(n).ok())
                .ok_or_else(|| format!("{key} must be a non-negative integer"))
        })
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
        let mut value = JsonValue::object();
        value.insert("id", self.id.as_str());
        value.insert("name", self.name.as_str());
        value.insert("class", self.class.as_str());
        value.insert("level", self.level);
        value.insert("current_hp", self.current_hp);
        value.insert("max_hp", self.max_hp);
        value.insert("temp_hp", self.temp_hp);
        value.insert("ac", self.ac);
        value.insert("fort", self.fort);
        value.insert("reflex", self.reflex);
        value.insert("will", self.will);
        value.insert("perception", self.perception);
        value.insert("perception_dc", self.perception_dc);
        value.insert("stealth_dc", self.stealth_dc);
        value.insert("class_dc", self.class_dc);
        value.insert("speed", self.speed);
        value.insert("hero_points", i64::from(self.hero_points));
        value.insert(
            "conditions",
            JsonValue::Array(
                self.conditions
                    .iter()
                    .cloned()
                    .map(JsonValue::from)
                    .collect(),
            ),
        );
        value.insert(
            "linked_token_id",
            self.linked_token_id
                .clone()
                .map_or(JsonValue::Null, JsonValue::from),
        );
        value
    }
}

pub fn build_party_overview(sheets: &[CharacterSheet]) -> Vec<PartyOverviewRow> {
    sheets
        .iter()
        .map(|sheet| {
            let perception = sheet.perception_modifier();
            let stealth = sheet.skill_modifier("stealth").unwrap_or(0);
            PartyOverviewRow {
                id: sheet.id.clone(),
                name: sheet.name.clone(),
                class: sheet.class.clone(),
                level: sheet.level,
                current_hp: sheet.current_hp,
                max_hp: sheet.effective_max_hp(),
                temp_hp: sheet.temp_hp,
                ac: sheet.effective_ac(),
                fort: sheet.save_modifier(Save::Fortitude),
                reflex: sheet.save_modifier(Save::Reflex),
                will: sheet.save_modifier(Save::Will),
                perception,
                perception_dc: 10 + perception,
                stealth_dc: 10 + stealth,
                class_dc: sheet.class_dc(),
                speed: sheet.speed,
                hero_points: sheet.hero_points,
                conditions: sheet.conditions.iter().map(Condition::label).collect(),
                linked_token_id: sheet.linked_token_id.clone(),
            }
        })
        .collect()
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
    let normalized = check.trim();
    let lower = normalized.to_ascii_lowercase();
    if sheets.is_empty()
        && lower != "perception"
        && Save::parse(normalized).is_none()
        && Skill::parse(normalized).is_none()
        && !lower.ends_with(" lore")
    {
        return Err(format!("unknown secret check '{check}'"));
    }
    let known = sheets.iter().all(|sheet| {
        lower == "perception"
            || Save::parse(normalized).is_some()
            || sheet.skill_modifier(normalized).is_some()
            || lower.ends_with(" lore")
    });
    if !known {
        return Err(format!("unknown secret check '{check}'"));
    }
    let mut outcomes = Vec::with_capacity(sheets.len());
    for sheet in sheets {
        let modifier = if lower == "perception" {
            sheet.perception_modifier()
        } else if let Some(save) = Save::parse(normalized) {
            sheet.save_modifier(save)
        } else if let Some(modifier) = sheet.skill_modifier(normalized) {
            modifier
        } else if lower.ends_with(" lore") {
            sheet.abilities[Ability::Int as usize]
                + conditions::status_penalty(&sheet.conditions, Ability::Int)
        } else {
            return Err(format!("unknown secret check '{check}'"));
        };
        let roll = crate::dice::roll_expr(&format!("1d20{modifier:+}"), rng)?;
        let total = roll.total;
        let degree = dc.map(|dc| crate::dice::evaluate_check(total, dc, roll.natural_d20));
        outcomes.push(SecretCheckOutcome {
            id: sheet.id.clone(),
            name: sheet.name.clone(),
            modifier,
            roll,
            total,
            degree,
        });
    }
    Ok(outcomes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, initiative: i32) -> Combatant {
        let mut combatant = Combatant::new(id, id, id, true);
        combatant.initiative = Some(initiative);
        combatant
    }

    #[test]
    fn removal_keeps_the_active_combatant_and_wraps_the_round() {
        let mut tracker = EncounterTracker::new();
        tracker.combatants = vec![entry("first", 30), entry("active", 20), entry("last", 10)];
        tracker.active = true;
        tracker.active_index = 1;
        tracker.remove_combatant("first").unwrap();
        assert_eq!(tracker.active_combatant().unwrap().combatant_id, "active");

        let mut wrapping = EncounterTracker::new();
        wrapping.combatants = vec![entry("first", 20), entry("last", 10)];
        wrapping.active = true;
        wrapping.active_index = 1;
        wrapping.round = 2;
        wrapping.remove_combatant("last").unwrap();
        assert_eq!(wrapping.active_combatant().unwrap().combatant_id, "first");
        assert_eq!(wrapping.round, 3);

        wrapping.remove_combatant("first").unwrap();
        assert!(!wrapping.active);
        assert_eq!(wrapping.active_index, 0);
        assert!(wrapping.active_combatant().is_none());
    }

    #[test]
    fn identical_initiative_entries_fall_back_to_id() {
        let mut tracker = EncounterTracker::new();
        let mut b = entry("b", 12);
        b.name = "Twin".into();
        let mut a = entry("a", 12);
        a.name = "Twin".into();
        tracker.combatants = vec![b, a];
        tracker.sort_initiative();
        assert_eq!(tracker.combatants[0].combatant_id, "a");
        assert_eq!(tracker.combatants[1].combatant_id, "b");
    }

    #[test]
    fn initiative_label_keeps_a_positive_sign_for_zero() {
        let sheet = CharacterSheet::new("Hero", 1);
        assert_eq!(
            initiative_modifier(&sheet, "stealth"),
            Some((0, "Stealth (+0)".into()))
        );
    }
}
