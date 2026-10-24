// SPDX-License-Identifier: GPL-3.0-or-later
//! The rules API the GM's extension popover calls.
//!
//! Room changes bump `revision`, save the state file when configured, and broadcast the room
//! view to `/api/events` listeners. Invalid input, unknown routes or ids, and wrong methods
//! return JSON errors with status 400, 404, and 405 respectively.
//!
//! Assets: `GET /manifest.json`, `/icon.svg`, `/`, `/index.html`, and `/obr_bridge.js`.
//!
//! Room: `GET /api/state` returns the room view and `GET /api/events` opens an event stream
//! whose first event is that view. The view includes character sheets, party rows, spellbooks
//! and token metadata; monsters include instances, adjusted statistics and token metadata.
//!
//! Characters: `POST /api/import/character` imports or merges a character. `POST
//! /api/character/update` accepts `id`, `hp`, `damage`, `heal`, `temp_hp`, `hero_points`,
//! `condition`, `remove_condition`, `raise_shield`, and `linked_token_id`. `POST
//! /api/character/remove` removes the character from the room and encounter.
//!
//! Monsters: `POST /api/monster/import` accepts exactly one of `text`, `monster` or `builtin`,
//! plus optional `variant`, `count` (1–20), and `label`. Instance IDs count up per stat block
//! across the room's life. `POST /api/monster/update` accepts `id`, `variant`, `hp`, `damage`,
//! `heal`, `temp_hp`, `condition`, `remove_condition`, and `linked_token_id`. `POST
//! /api/monster/remove` removes the monster from the room and encounter.
//!
//! Table: `POST /api/target` accepts `{"dc": integer|null}`. Token binding: `POST /api/token`
//! accepts `{"actor": id}` and returns that actor's token metadata.
//!
//! Rolls (`POST /api/roll`), initiative (`POST /api/initiative`), and spell areas (`POST
//! /api/aoe/compute` and `/api/aoe/clear`) are reserved for later routes and currently return
//! 404.
use crate::{
    assets,
    http::{self, Handler, Reply, Request, Response, RunningServer},
    state::RoomState,
};
use obb_core::{
    bestiary::{CreatureVariant, MonsterInstance},
    bestiary_data, conditions, encounter, fs, import,
    json::JsonValue,
    monster::MonsterStatBlock,
    spells,
};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, PoisonError},
};

struct App {
    state: Mutex<RoomState>,
    hub: http::SseHub,
    file: Option<PathBuf>,
    rng: Mutex<obb_core::dice::Rng>,
}

impl App {
    fn room(&self, s: &RoomState) -> JsonValue {
        let mut out = JsonValue::object();
        out.insert("revision", s.revision as i64);
        let rows = encounter::build_party_overview(&s.characters);
        out.insert(
            "characters",
            JsonValue::Array(
                s.characters
                    .iter()
                    .map(|c| {
                        let mut e = JsonValue::object();
                        e.insert("sheet", c.to_json());
                        e.insert(
                            "row",
                            rows.iter()
                                .find(|r| r.id == c.id)
                                .map_or(JsonValue::Null, |r| r.to_json()),
                        );
                        e.insert(
                            "spellbook",
                            JsonValue::Array(
                                spells::spellbook(c)
                                    .iter()
                                    .map(|x| {
                                        let mut v = JsonValue::object();
                                        v.insert("entry", x.entry as u32);
                                        v.insert("name", x.name.as_str());
                                        v.insert("listed_rank", u32::from(x.listed_rank));
                                        v.insert(
                                            "details",
                                            x.details
                                                .as_ref()
                                                .map_or(JsonValue::Null, |d| d.to_json()),
                                        );
                                        v
                                    })
                                    .collect(),
                            ),
                        );
                        let mut token = JsonValue::object();
                        token.insert("kind", "character");
                        token.insert("id", c.id.as_str());
                        token.insert("name", c.name.as_str());
                        token.insert("hp", c.current_hp);
                        token.insert("max_hp", c.effective_max_hp());
                        token.insert("temp_hp", c.temp_hp);
                        token.insert("ac", c.effective_ac());
                        token.insert(
                            "conditions",
                            JsonValue::Array(
                                c.conditions
                                    .iter()
                                    .map(|x| JsonValue::from(x.label()))
                                    .collect(),
                            ),
                        );
                        e.insert("token", token);
                        e
                    })
                    .collect(),
            ),
        );
        out.insert(
            "monsters",
            JsonValue::Array(
                s.monsters
                    .iter()
                    .map(|m| {
                        let mut e = JsonValue::object();
                        e.insert("instance", m.to_json());
                        e.insert("stats", m.stats().to_json());
                        e.insert("token", m.to_obr_token_metadata());
                        e
                    })
                    .collect(),
            ),
        );
        out.insert("encounter", s.encounter.to_json());
        out.insert("roll_log", JsonValue::Array(s.roll_log.clone()));
        out.insert(
            "target_dc",
            s.target_dc.map_or(JsonValue::Null, JsonValue::from),
        );
        out.insert("aoe_templates", JsonValue::Array(s.aoe_templates.clone()));
        out.insert(
            "bestiary",
            JsonValue::Array(
                bestiary_data::builtin_bestiary()
                    .iter()
                    .map(|m| {
                        let mut v = JsonValue::object();
                        v.insert("id", m.id.as_str());
                        v.insert("name", m.name.as_str());
                        v.insert("level", m.level);
                        v
                    })
                    .collect(),
            ),
        );
        out
    }
    fn changed(&self, s: &mut RoomState) -> JsonValue {
        s.revision += 1;
        let view = self.room(s);
        if let Some(p) = &self.file {
            if let Err(e) = fs::atomic_write(p, s.to_json().to_json_pretty().as_bytes()) {
                eprintln!("failed to save room: {e}");
            }
        }
        self.hub.broadcast("state", &view.to_json_string());
        view
    }
}

impl Handler for App {
    fn handle(&self, r: &Request) -> Reply {
        if r.method == "GET" && r.path == "/api/events" {
            return Reply::EventStream;
        }
        Reply::Response(self.route(r))
    }
    fn hub(&self) -> &http::SseHub {
        &self.hub
    }
    fn initial_event(&self) -> Option<(String, String)> {
        Some((
            "state".into(),
            self.room(&self.state.lock().unwrap_or_else(PoisonError::into_inner))
                .to_json_string(),
        ))
    }
}

impl App {
    fn route(&self, r: &Request) -> Response {
        let get_route = assets::ALL
            .iter()
            .any(|(name, _, _)| match r.path.as_str() {
                "/manifest.json" => *name == "manifest.json",
                "/icon.svg" => *name == "icon.svg",
                "/" | "/index.html" => *name == "index.html",
                "/obr_bridge.js" => *name == "obr_bridge.js",
                _ => false,
            })
            || matches!(r.path.as_str(), "/api/events" | "/api/state");
        let post_route = matches!(
            r.path.as_str(),
            "/api/import/character"
                | "/api/character/update"
                | "/api/character/remove"
                | "/api/monster/import"
                | "/api/monster/update"
                | "/api/monster/remove"
                | "/api/target"
                | "/api/token"
        );

        if (r.method == "GET" && post_route) || (r.method == "POST" && get_route) {
            return Response::error(405, "wrong method");
        }
        if r.method == "GET" {
            if let Some((_, ty, data)) =
                assets::ALL
                    .iter()
                    .find(|(name, _, _)| match r.path.as_str() {
                        "/manifest.json" => *name == "manifest.json",
                        "/icon.svg" => *name == "icon.svg",
                        "/" | "/index.html" => *name == "index.html",
                        "/obr_bridge.js" => *name == "obr_bridge.js",
                        _ => false,
                    })
            {
                return Response::text(200, ty, data);
            }
            if r.path == "/api/state" {
                return Response::json(
                    200,
                    &self.room(&self.state.lock().unwrap_or_else(PoisonError::into_inner)),
                );
            }
            return Response::error(404, "unknown route");
        }
        if r.method != "POST" {
            return Response::error(405, "wrong method");
        }
        let body = match r.json() {
            Ok(v) => v,
            Err(e) => return Response::error(400, &e),
        };
        // A panic in an earlier request poisons the lock; the room data is still usable, and
        // refusing it would turn one bad request into a dead server for the rest of the session.
        let mut s = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let mut changed = false;
        let result: Result<JsonValue, (u16, String)> = (|| {
            Ok(match r.path.as_str() {
                "/api/import/character" => {
                    let c = import::import_character_value(&body).map_err(|e| (400, e))?;
                    let imported_id = c.id.clone();
                    let mut merged = false;
                    let mut changes = Vec::new();
                    if let Some(old) = s.characters.iter_mut().find(|x| x.id == c.id) {
                        changes = import::merge_update(old, c);
                        merged = true;
                    } else {
                        s.characters.push(c.clone());
                    }
                    changed = true;
                    let mut o = JsonValue::object();
                    o.insert(
                        "id",
                        if merged {
                            s.characters
                                .iter()
                                .find(|x| x.id == imported_id)
                                .unwrap()
                                .id
                                .as_str()
                        } else {
                            imported_id.as_str()
                        },
                    );
                    o.insert("merged", merged);
                    o.insert(
                        "changes",
                        JsonValue::Array(changes.into_iter().map(JsonValue::from).collect()),
                    );
                    o
                }
                "/api/character/update" => {
                    let id = str_field(&body, "id")?;
                    let c = s
                        .characters
                        .iter_mut()
                        .find(|c| c.id == id)
                        .ok_or((404, "unknown character".into()))?;
                    adjust_character(c, &body)?;
                    changed = true;
                    let mut o = JsonValue::object();
                    o.insert("character", JsonValue::Null);
                    o
                }
                "/api/character/remove" => {
                    let id = str_field(&body, "id")?;
                    let n = s.characters.len();
                    s.characters.retain(|c| c.id != id);
                    if n == s.characters.len() {
                        return Err((404, "unknown character".into()));
                    }
                    s.encounter.remove_combatant(id);
                    changed = true;
                    let mut o = JsonValue::object();
                    o.insert("removed", id);
                    o
                }
                "/api/monster/import" => {
                    let source = ["text", "monster", "builtin"]
                        .iter()
                        .filter(|k| body.get(k).is_some())
                        .count();
                    if source != 1 {
                        return Err((400, "provide exactly one monster source".into()));
                    }
                    let base = if let Some(t) = body.get_str("text") {
                        obb_core::monster::parse_aon_statblock(t).map_err(|e| (400, e))?
                    } else if let Some(v) = body.get("monster") {
                        MonsterStatBlock::from_json(v).map_err(|e| (400, e))?
                    } else {
                        let id = body
                            .get_str("builtin")
                            .ok_or((400, "builtin must be a string".into()))?;
                        bestiary_data::builtin_bestiary()
                            .into_iter()
                            .find(|m| m.id == id)
                            .ok_or((404, "unknown creature".into()))?
                    };
                    let variant = body
                        .get_str("variant")
                        .and_then(CreatureVariant::parse)
                        .unwrap_or(CreatureVariant::Normal);
                    let count = body.get_i64("count").unwrap_or(1);
                    if !(1..=20).contains(&count) {
                        return Err((400, "count must be 1 through 20".into()));
                    }
                    let base_id = base.id.clone();
                    let mut list = Vec::new();
                    for _ in 0..count {
                        let n = {
                            let counter = s.monster_counters.entry(base_id.clone()).or_insert(0);
                            *counter += 1;
                            *counter
                        };
                        let label = body.get_str("label").unwrap_or(&base.name);
                        let m = MonsterInstance::new(
                            base.clone(),
                            variant,
                            &format!("{base_id}-{n}"),
                            &format!("{label} {n}"),
                        );
                        list.push(m.to_json());
                        s.monsters.push(m);
                    }
                    changed = true;
                    let mut o = JsonValue::object();
                    o.insert("instances", JsonValue::Array(list));
                    o
                }
                "/api/monster/update" => {
                    let id = str_field(&body, "id")?;
                    let m = s
                        .monsters
                        .iter_mut()
                        .find(|m| m.instance_id == id)
                        .ok_or((404, "unknown monster".into()))?;
                    adjust_monster(m, &body)?;
                    changed = true;
                    let mut o = JsonValue::object();
                    o.insert("monster", JsonValue::Null);
                    o
                }
                "/api/monster/remove" => {
                    let id = str_field(&body, "id")?;
                    let n = s.monsters.len();
                    s.monsters.retain(|m| m.instance_id != id);
                    if n == s.monsters.len() {
                        return Err((404, "unknown monster".into()));
                    }
                    s.encounter.remove_combatant(id);
                    changed = true;
                    let mut o = JsonValue::object();
                    o.insert("removed", id);
                    o
                }
                "/api/target" => {
                    let dc = body.get("dc").ok_or((400, "dc is required".into()))?;
                    s.target_dc = if dc.is_null() {
                        None
                    } else {
                        Some(
                            dc.as_i64()
                                .ok_or((400, "dc must be an integer or null".into()))?
                                as i32,
                        )
                    };
                    changed = true;
                    let mut o = JsonValue::object();
                    o.insert(
                        "target_dc",
                        s.target_dc.map_or(JsonValue::Null, JsonValue::from),
                    );
                    o
                }
                "/api/token" => {
                    let id = str_field(&body, "actor")?;
                    if let Some(c) = s.characters.iter().find(|c| c.id == id) {
                        character_token(c)
                    } else if let Some(m) = s.monsters.iter().find(|m| m.instance_id == id) {
                        m.to_obr_token_metadata()
                    } else {
                        return Err((404, "unknown actor".into()));
                    }
                }
                _ => return Err((404, "unknown route".into())),
            })
        })();
        match result {
            Ok(mut value) => {
                if changed {
                    let view = self.changed(&mut s);
                    match r.path.as_str() {
                        "/api/monster/import" => {
                            if let Some(ids) = value.get_array("instances") {
                                let ids: Vec<String> = ids
                                    .iter()
                                    .filter_map(|v| v.get_str("instance_id").map(str::to_owned))
                                    .collect();
                                let entries = view
                                    .get_array("monsters")
                                    .map(|all| {
                                        all.iter()
                                            .filter(|entry| {
                                                entry
                                                    .get("instance")
                                                    .and_then(|m| m.get_str("instance_id"))
                                                    .is_some_and(|id| {
                                                        ids.iter().any(|wanted| wanted == id)
                                                    })
                                            })
                                            .cloned()
                                            .collect()
                                    })
                                    .unwrap_or_default();
                                value.insert("instances", JsonValue::Array(entries));
                            }
                        }
                        "/api/character/update" => {
                            if let Some(e) = view.get_array("characters").and_then(|a| {
                                a.iter().find(|e| {
                                    e.get("sheet").and_then(|v| v.get_str("id"))
                                        == body.get_str("id")
                                })
                            }) {
                                value.insert("character", e.clone());
                            }
                        }
                        "/api/monster/update" => {
                            if let Some(e) = view.get_array("monsters").and_then(|a| {
                                a.iter().find(|e| {
                                    e.get("instance").and_then(|v| v.get_str("instance_id"))
                                        == body.get_str("id")
                                })
                            }) {
                                value.insert("monster", e.clone());
                            }
                        }
                        _ => {}
                    }
                }
                Response::json(200, &value)
            }
            Err((code, e)) => Response::error(code, &e),
        }
    }
}

fn str_field<'a>(v: &'a JsonValue, k: &str) -> Result<&'a str, (u16, String)> {
    v.get_str(k).ok_or((400, format!("{k} must be a string")))
}
fn num(v: &JsonValue, k: &str) -> Option<i32> {
    v.get_i64(k)
        .map(|n| n.clamp(i32::MIN as i64, i32::MAX as i64) as i32)
}
fn adjust_character(
    c: &mut obb_core::character::CharacterSheet,
    v: &JsonValue,
) -> Result<(), (u16, String)> {
    if let Some(n) = num(v, "hp") {
        c.current_hp = n.clamp(0, c.effective_max_hp())
    }
    if let Some(n) = num(v, "damage") {
        c.apply_damage(n)
    }
    if let Some(n) = num(v, "heal") {
        c.heal(n)
    }
    if let Some(n) = num(v, "temp_hp") {
        c.gain_temp_hp(n)
    }
    if let Some(n) = num(v, "hero_points") {
        c.hero_points = n.clamp(0, 3) as u8
    }
    if let Some(x) = v.get("condition") {
        let name = x
            .get_str("name")
            .ok_or((400, "condition.name required".into()))?;
        let val = x.get_i64("value").map(|n| n.clamp(0, 255) as u8);
        conditions::set_condition(&mut c.conditions, name, val).map_err(|e| (400, e))?;
    }
    if let Some(name) = v.get_str("remove_condition") {
        conditions::remove_condition(&mut c.conditions, name)
    }
    if v.get_bool("raise_shield") == Some(true) {
        c.raise_shield()
    }
    if let Some(t) = v.get("linked_token_id") {
        c.linked_token_id = if t.is_null() {
            None
        } else {
            Some(
                t.as_str()
                    .ok_or((400, "linked_token_id must be string or null".into()))?
                    .to_string(),
            )
        }
    }
    Ok(())
}
fn adjust_monster(m: &mut MonsterInstance, v: &JsonValue) -> Result<(), (u16, String)> {
    if let Some(x) = v.get_str("variant") {
        m.set_variant(CreatureVariant::parse(x).ok_or((400, "invalid variant".into()))?)
    }
    if let Some(n) = num(v, "hp") {
        m.current_hp = n.clamp(0, m.effective_max_hp())
    }
    if let Some(n) = num(v, "damage") {
        m.apply_damage(n)
    }
    if let Some(n) = num(v, "heal") {
        m.heal(n)
    }
    if let Some(n) = num(v, "temp_hp") {
        m.temp_hp = m.temp_hp.max(n)
    }
    if let Some(x) = v.get("condition") {
        let name = x
            .get_str("name")
            .ok_or((400, "condition.name required".into()))?;
        conditions::set_condition(
            &mut m.conditions,
            name,
            x.get_i64("value").map(|n| n.clamp(0, 255) as u8),
        )
        .map_err(|e| (400, e))?;
    }
    if let Some(x) = v.get_str("remove_condition") {
        conditions::remove_condition(&mut m.conditions, x)
    }
    if let Some(t) = v.get("linked_token_id") {
        m.linked_token_id = if t.is_null() {
            None
        } else {
            Some(
                t.as_str()
                    .ok_or((400, "linked_token_id must be string or null".into()))?
                    .into(),
            )
        }
    }
    Ok(())
}
fn character_token(c: &obb_core::character::CharacterSheet) -> JsonValue {
    let mut t = JsonValue::object();
    t.insert("kind", "character");
    t.insert("id", c.id.as_str());
    t.insert("name", c.name.as_str());
    t.insert("hp", c.current_hp);
    t.insert("max_hp", c.effective_max_hp());
    t.insert("temp_hp", c.temp_hp);
    t.insert("ac", c.effective_ac());
    t.insert(
        "conditions",
        JsonValue::Array(
            c.conditions
                .iter()
                .map(|x| JsonValue::from(x.label()))
                .collect(),
        ),
    );
    t
}

pub fn start(
    bind: &str,
    state_file: Option<PathBuf>,
    seed: Option<u64>,
) -> Result<RunningServer, String> {
    let state = match &state_file {
        Some(p) if p.exists() => {
            let text = std::fs::read_to_string(p).map_err(|e| e.to_string())?;
            let json = JsonValue::parse(&text).map_err(|e| e.to_string())?;
            RoomState::from_json(&json)?
        }
        _ => RoomState::default(),
    };
    let app = Arc::new(App {
        state: Mutex::new(state),
        hub: http::SseHub::new(),
        file: state_file,
        rng: Mutex::new(
            seed.map_or_else(obb_core::dice::Rng::from_os, obb_core::dice::Rng::from_seed),
        ),
    });
    http::serve(bind, app).map_err(|e| e.to_string())
}
