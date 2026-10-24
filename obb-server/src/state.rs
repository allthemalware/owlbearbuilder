// SPDX-License-Identifier: GPL-3.0-or-later
//! The room: characters, creatures, the encounter, the roll log and spell templates, and how
//! they are saved to disk.

use obb_core::bestiary::MonsterInstance;
use obb_core::character::CharacterSheet;
use obb_core::encounter::EncounterTracker;
use obb_core::json::JsonValue;
use std::collections::BTreeMap;

pub const ROLL_LOG_CAP: usize = 50;

#[derive(Clone)]
pub struct RoomState {
    pub revision: u64,
    pub characters: Vec<CharacterSheet>,
    pub monsters: Vec<MonsterInstance>,
    pub encounter: EncounterTracker,
    pub roll_log: Vec<JsonValue>,
    pub next_roll_id: u64,
    pub target_dc: Option<i32>,
    pub aoe_templates: Vec<JsonValue>,
    pub monster_counters: BTreeMap<String, u64>,
}

impl Default for RoomState {
    fn default() -> Self {
        Self {
            revision: 0,
            characters: Vec::new(),
            monsters: Vec::new(),
            encounter: EncounterTracker::new(),
            roll_log: Vec::new(),
            next_roll_id: 1,
            target_dc: None,
            aoe_templates: Vec::new(),
            monster_counters: BTreeMap::new(),
        }
    }
}

impl RoomState {
    pub fn to_json(&self) -> JsonValue {
        let mut value = JsonValue::object();
        value.insert("format", "owlbearbuilder-room");
        value.insert("version", 1);
        value.insert("revision", self.revision as i64);
        value.insert(
            "characters",
            JsonValue::Array(
                self.characters
                    .iter()
                    .map(CharacterSheet::to_json)
                    .collect(),
            ),
        );
        value.insert(
            "monsters",
            JsonValue::Array(self.monsters.iter().map(MonsterInstance::to_json).collect()),
        );
        value.insert("encounter", self.encounter.to_json());
        value.insert("roll_log", JsonValue::Array(self.roll_log.clone()));
        value.insert("next_roll_id", self.next_roll_id as i64);
        value.insert(
            "target_dc",
            self.target_dc.map_or(JsonValue::Null, JsonValue::from),
        );
        value.insert(
            "aoe_templates",
            JsonValue::Array(self.aoe_templates.clone()),
        );
        let mut counters = JsonValue::object();
        for (id, count) in &self.monster_counters {
            counters.insert(id, *count as i64);
        }
        value.insert("monster_counters", counters);
        value
    }

    pub fn from_json(value: &JsonValue) -> Result<Self, String> {
        if value.get_str("format") != Some("owlbearbuilder-room")
            || value.get_i64("version") != Some(1)
        {
            return Err("unsupported room format".into());
        }
        let array = |name: &str| {
            value
                .get_array(name)
                .ok_or_else(|| format!("{name} must be an array"))
        };
        let characters = array("characters")?
            .iter()
            .map(CharacterSheet::from_json)
            .collect::<Result<_, _>>()?;
        let monsters = array("monsters")?
            .iter()
            .map(MonsterInstance::from_json)
            .collect::<Result<_, _>>()?;
        let encounter = EncounterTracker::from_json(
            value
                .get("encounter")
                .ok_or_else(|| "encounter is required".to_string())?,
        )?;
        let roll_log = array("roll_log")?.to_vec();
        let aoe_templates = array("aoe_templates")?.to_vec();
        let mut monster_counters = BTreeMap::new();
        for (key, count) in value
            .get("monster_counters")
            .and_then(JsonValue::as_object)
            .ok_or("monster_counters must be an object")?
        {
            let count = count
                .as_i64()
                .filter(|n| *n >= 0)
                .ok_or("invalid monster counter")?;
            monster_counters.insert(key.clone(), count as u64);
        }
        Ok(Self {
            revision: value
                .get_i64("revision")
                .filter(|n| *n >= 0)
                .ok_or("revision is required")? as u64,
            characters,
            monsters,
            encounter,
            roll_log,
            next_roll_id: value.get_i64("next_roll_id").unwrap_or(1).max(1) as u64,
            target_dc: match value.get("target_dc") {
                None | Some(JsonValue::Null) => None,
                Some(v) => Some(v.as_i64().ok_or("target_dc must be an integer or null")? as i32),
            },
            aoe_templates,
            monster_counters,
        })
    }

    pub fn push_roll(&mut self, mut entry: JsonValue) {
        entry.insert("id", self.next_roll_id as i64);
        self.next_roll_id += 1;
        self.roll_log.push(entry);
        if self.roll_log.len() > ROLL_LOG_CAP {
            self.roll_log.remove(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_json_round_trip() {
        let state = RoomState::default();
        let decoded = RoomState::from_json(&state.to_json()).unwrap();
        assert_eq!(decoded.revision, state.revision);
        assert_eq!(decoded.encounter, state.encounter);
    }

    #[test]
    fn monster_id_counters_survive_storage() {
        let mut state = RoomState::default();
        state.monster_counters.insert("goblin-warrior".into(), 8);
        let decoded = RoomState::from_json(&state.to_json()).unwrap();
        assert_eq!(decoded.monster_counters.get("goblin-warrior"), Some(&8));
    }

    #[test]
    fn roll_log_is_capped_without_reusing_ids() {
        let mut state = RoomState::default();
        for _ in 0..52 {
            state.push_roll(JsonValue::object());
        }
        assert_eq!(state.roll_log.len(), ROLL_LOG_CAP);
        assert_eq!(state.roll_log[0].get_i64("id"), Some(3));
        assert_eq!(state.roll_log[49].get_i64("id"), Some(52));
        state.push_roll(JsonValue::object());
        assert_eq!(state.roll_log[49].get_i64("id"), Some(53));
    }
}
