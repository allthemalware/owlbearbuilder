// SPDX-License-Identifier: GPL-3.0-or-later
//! The rules API the GM's extension popover calls.
//!
//! Only the GM's popover (and the CLI) talk to this server. Players' popovers relay their
//! actions to the GM through Owlbear Rodeo broadcasts, so the server never needs to be
//! reachable from other machines. All bodies are JSON; errors are `{"error": "..."}` with 400
//! (bad input), 404 (unknown route or id) or 405 (wrong method). Every request that changes
//! the room bumps `revision`, saves the state file (if any) and broadcasts the full room view
//! as an `event: state` to `/api/events` listeners.
//!
//! Assets: `GET /manifest.json`, `/icon.svg`, `/` and `/index.html`, `/obr_bridge.js`.
//!
//! Room:
//! - `GET /api/state` → the room view: `{"revision", "characters": [{"sheet", "row",
//!   "spellbook", "token"}], "monsters": [{"instance", "stats", "token"}], "encounter",
//!   "roll_log", "target_dc", "aoe_templates", "bestiary": [{"id", "name", "level"}]}` where
//!   `sheet` is `CharacterSheet::to_json`, `row` `PartyOverviewRow::to_json`, `spellbook` the
//!   `spells::spellbook` entries (`{"entry", "name", "listed_rank", "details"}`), `token` the
//!   payload for `com.owlbearbuilder/actor` metadata (characters: `{"kind": "character",
//!   "id", "name", "hp", "max_hp", "temp_hp", "ac", "conditions"}`; monsters:
//!   `MonsterInstance::to_obr_token_metadata`), `stats` the adjusted stat block.
//! - `GET /api/events` → event stream; the first event is the room view.
//!
//! Characters:
//! - `POST /api/import/character` with an export as the body (anything
//!   `import::import_character_value` reads) → `{"id", "merged": bool, "changes": [...]}`;
//!   a sheet whose id is already in the room is merged with `import::merge_update`.
//! - `POST /api/character/update` `{"id", "hp"?, "damage"?, "heal"?, "temp_hp"?,
//!   "hero_points"?, "condition"?: {"name", "value"?}, "remove_condition"?: name,
//!   "raise_shield"?: true, "linked_token_id"?: string|null}` → `{"character": {...}}` (the
//!   room-view entry). `hp` sets current HP (clamped to 0..=max), `temp_hp` gains temporary
//!   HP, `condition` uses `set_condition` (value 0 removes). Applied in that order.
//! - `POST /api/character/remove` `{"id"}` → `{"removed": id}`; also leaves the encounter.
//!
//! Monsters:
//! - `POST /api/monster/import` with exactly one of `{"text": stat block}`, `{"monster":
//!   MonsterStatBlock JSON}`, `{"builtin": id}`, plus optional `"variant"` (default
//!   normal), `"count"` (1–20, default 1), `"label"` → `{"instances": [{...}]}`. Instance ids
//!   are `<stat block id>-<n>` with `n` counting up per stat block across the room's life;
//!   labels are `<label or name> <n>`.
//! - `POST /api/monster/update` `{"id", "variant"?, "hp"?, "damage"?, "heal"?, "temp_hp"?,
//!   "condition"?, "remove_condition"?, "linked_token_id"?}` → `{"monster": {...}}`.
//! - `POST /api/monster/remove` `{"id"}` → `{"removed": id}`; also leaves the encounter.
//!
//! Rolls (`POST /api/roll`) → the new roll-log entry `{"id", "actor", "actor_name", "label",
//! "expr", "rolls", "total", "natural", "dc", "degree", "secret"}` (`degree` a
//! `DegreeOfSuccess::id` or null). Body: `{"actor": id, "kind", ...}`:
//! - `perception`; `save` + `"name"`; `skill` + `"name"` (skills and Lores; monsters: listed
//!   skills); `assurance` + `"name"` (characters only; no dice, `rolls` empty, `natural`
//!   null); `strike` + `"index"`, `"map"` (0–2); `damage` + `"index"`, `"critical"`;
//!   `spell_attack` + `"index"` (spellcasting entry); `expr` + `"expr"` (any dice).
//! - d20 checks are `1d20+<modifier>` and compare against `"dc"`, else the room's
//!   `target_dc` (for `strike` against a target: pass `"dc"` = its AC); damage never has a DC.
//! - `"secret": true` marks the entry for the GM only.
//! - `kind: "party"` + `"name"` (+ `"dc"`): `encounter::roll_party_secret_check` for every
//!   character → `{"results": [entries]}`, all secret.
//! Labels read like `Perception`, `Fortitude save`, `Athletics`, `Assurance (Athletics)`,
//! `+1 Striking Longsword (2nd attack)`, `+1 Striking Longsword damage (critical)`,
//! `Wizard spell attack`.
//!
//! Initiative (`POST /api/initiative`) → `{"encounter", "transition"?}`. `"action"`:
//! `add` (`"actor"`, optional `"check"` and `"roll": true`; combatant id = actor id; monsters
//! roll Perception), `roll` (`"combatant"`, `"check"`?), `set` (`"combatant"`, `"total"`),
//! `start`, `next` (ticks conditions of every character and monster), `delay`
//! (`"combatant"`, `"initiative"`), `remove` (`"combatant"`), `end`, `clear` (empties it).
//!
//! Table: `POST /api/target` `{"dc": n|null}` → `{"target_dc"}`.
//!
//! Spell areas: `POST /api/aoe/compute` with one of `{"area": SpellArea JSON}`, `{"text":
//! "20-foot burst"}`, `{"spell": name, "rank"?}`, plus `"origin": {"x", "y"}` (required),
//! `"direction_deg"`, `"grid_dpi"` (default 150), `"token_size"` (default 1), `"color"`
//! (default `#e8590c`), `"label"` (default the spell name or area label), `"keep"`
//! (store in `aoe_templates`) → the `aoe::compute_obr_aoe_item` object (plus `"spell"` details
//! when a spell was named). `POST /api/aoe/clear` → `{"cleared": n}`.
//!
//! Token binding: `POST /api/token` `{"actor": id}` → that actor's token payload.
use crate::{
    assets,
    http::{self, Handler, Reply, Request, Response, RunningServer},
    state::RoomState,
};
use obb_core::{
    aoe::{self, SpellArea},
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
                | "/api/roll"
                | "/api/initiative"
                | "/api/aoe/compute"
                | "/api/aoe/clear"
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
                "/api/roll" => {
                    let mut rng = self.rng.lock().unwrap_or_else(PoisonError::into_inner);
                    let (value, entries) = roll_request(&mut s, &body, &mut rng)?;
                    for entry in entries {
                        s.push_roll(entry);
                    }
                    changed = true;
                    value
                }
                "/api/initiative" => {
                    let mut rng = self.rng.lock().unwrap_or_else(PoisonError::into_inner);
                    let value = initiative_request(&mut s, &body, &mut rng)?;
                    changed = true;
                    value
                }
                "/api/aoe/compute" => {
                    let (value, keep) = aoe_request(&body)?;
                    if keep {
                        s.aoe_templates.push(value.clone());
                        changed = true;
                    }
                    value
                }
                "/api/aoe/clear" => {
                    let count = s.aoe_templates.len();
                    s.aoe_templates.clear();
                    changed = count > 0;
                    let mut value = JsonValue::object();
                    value.insert("cleared", count as i64);
                    value
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

fn roll_request(
    state: &mut RoomState,
    body: &JsonValue,
    rng: &mut obb_core::dice::Rng,
) -> Result<(JsonValue, Vec<JsonValue>), (u16, String)> {
    let kind = str_field(body, "kind")?;
    if kind == "party" {
        let name = str_field(body, "name")?;
        let dc = num(body, "dc").or(state.target_dc);
        let outcomes = encounter::roll_party_secret_check(&state.characters, name, dc, rng)
            .map_err(|e| (400, e))?;
        let entries: Vec<JsonValue> = outcomes
            .iter()
            .map(|outcome| {
                roll_json(
                    RollFields {
                        actor: &outcome.id,
                        actor_name: &outcome.name,
                        label: name,
                        expr: &outcome.roll.expr,
                        total: outcome.total,
                        modifier: Some(outcome.modifier),
                        dc,
                        degree: outcome.degree.map(|degree| degree.id()),
                        secret: true,
                    },
                    &outcome.roll,
                )
            })
            .collect();
        let mut value = JsonValue::object();
        value.insert("results", JsonValue::Array(entries.clone()));
        return Ok((value, entries));
    }

    let actor_id = str_field(body, "actor")?;
    let character = state.characters.iter().find(|c| c.id == actor_id);
    let monster = state.monsters.iter().find(|m| m.instance_id == actor_id);
    if character.is_none() && monster.is_none() {
        return Err((404, "unknown actor".into()));
    }
    let actor_name = character.map_or_else(|| monster.unwrap().label.clone(), |c| c.name.clone());
    let secret = body.get_bool("secret").unwrap_or(false);
    let dc = num(body, "dc").or(state.target_dc);
    let index = body.get_i64("index").unwrap_or(0);
    if index < 0 {
        return Err((400, "index must be non-negative".into()));
    }
    let map = body.get_i64("map").unwrap_or(0).clamp(0, 2) as u8;
    let critical = body.get_bool("critical").unwrap_or(false);
    let (label, expr, result, total, modifier, natural) = match kind {
        "expr" => {
            let expr = str_field(body, "expr")?;
            let result = obb_core::dice::roll_expr(expr, rng).map_err(|e| (400, e))?;
            (
                expr.to_string(),
                result.expr.clone(),
                Some(result.clone()),
                result.total,
                None,
                result.natural_d20,
            )
        }
        "perception" | "save" | "skill" | "assurance" | "spell" => {
            let name = body.get_str("name").unwrap_or("");
            let (label, modifier, assurance) = if let Some(c) = character {
                match kind {
                    "perception" => ("Perception".to_string(), c.perception_modifier(), false),
                    "save" => {
                        let save = obb_core::character::Save::parse(name)
                            .ok_or((400, "unknown save".into()))?;
                        (
                            format!("{} save", save.label()),
                            c.save_modifier(save),
                            false,
                        )
                    }
                    "skill" => {
                        let modifier = c
                            .skill_modifier(name)
                            .ok_or((400, "unknown skill".into()))?;
                        let label = obb_core::character::Skill::parse(name).map_or_else(
                            || {
                                c.lores
                                    .iter()
                                    .find(|l| {
                                        l.name.eq_ignore_ascii_case(name)
                                            || format!("{} lore", l.name).eq_ignore_ascii_case(name)
                                    })
                                    .map(|l| format!("{} Lore", l.name))
                                    .unwrap_or_else(|| name.to_string())
                            },
                            |skill| skill.label().to_string(),
                        );
                        (label, modifier, false)
                    }
                    "assurance" => {
                        let total = c
                            .assurance_total(name)
                            .ok_or((400, "unknown skill".into()))?;
                        let skill_label = obb_core::character::Skill::parse(name).map_or_else(
                            || {
                                c.lores
                                    .iter()
                                    .find(|l| {
                                        l.name.eq_ignore_ascii_case(name)
                                            || format!("{} lore", l.name).eq_ignore_ascii_case(name)
                                    })
                                    .map(|l| format!("{} Lore", l.name))
                                    .unwrap_or_else(|| name.to_string())
                            },
                            |skill| skill.label().to_string(),
                        );
                        (assurance_label(&skill_label), total, true)
                    }
                    _ => {
                        let slot = index as usize;
                        let spell = c
                            .spellcasting
                            .get(slot)
                            .ok_or((400, "unknown spellcasting entry".into()))?;
                        let modifier = c
                            .spell_attack_modifier(slot)
                            .ok_or((400, "unknown spellcasting entry".into()))?;
                        (format!("{} spell attack", spell.name), modifier, false)
                    }
                }
            } else {
                let m = monster.unwrap();
                if kind == "assurance" || kind == "spell" {
                    return Err((400, "unsupported roll for monster".into()));
                }
                match kind {
                    "perception" => ("Perception".to_string(), m.perception_modifier(), false),
                    "save" => {
                        let save = obb_core::character::Save::parse(name)
                            .ok_or((400, "unknown save".into()))?;
                        (
                            format!("{} save", save.label()),
                            m.save_modifier(save),
                            false,
                        )
                    }
                    _ => (
                        name.to_string(),
                        m.skill_modifier(name)
                            .ok_or((400, "unknown skill".into()))?,
                        false,
                    ),
                }
            };
            if assurance {
                (label, String::new(), None, modifier, Some(modifier), None)
            } else {
                let expr = format!("1d20{modifier:+}");
                let result = obb_core::dice::roll_expr(&expr, rng).map_err(|e| (400, e))?;
                (
                    label,
                    result.expr.clone(),
                    Some(result.clone()),
                    result.total,
                    Some(modifier),
                    result.natural_d20,
                )
            }
        }
        "strike" | "damage" => {
            let strike_index = index as usize;
            let strike_name;
            if let Some(c) = character {
                let strike = c
                    .strikes
                    .get(strike_index)
                    .ok_or((400, "unknown strike".into()))?;
                strike_name = strike.name.clone();
                if kind == "strike" {
                    let modifier = c
                        .strike_attack_modifier(strike_index, map)
                        .ok_or((400, "unknown strike".into()))?;
                    let label = attack_label(&strike.name, map);
                    let expr = format!("1d20{modifier:+}");
                    let result = obb_core::dice::roll_expr(&expr, rng).map_err(|e| (400, e))?;
                    (
                        label,
                        result.expr.clone(),
                        Some(result.clone()),
                        result.total,
                        Some(modifier),
                        result.natural_d20,
                    )
                } else {
                    let damage = c
                        .strike_damage(strike_index)
                        .ok_or((400, "unknown strike".into()))?;
                    let expr = format!(
                        "{}d{}{:+}",
                        damage.dice_count, damage.die_sides, damage.flat_bonus
                    );
                    let result = damage.roll(critical, rng);
                    let label = damage_label(&strike_name, critical);
                    (label, expr, None, result.total, None, None)
                }
            } else {
                let m = monster.unwrap();
                let stats = m.stats();
                let strike = stats
                    .strikes
                    .get(strike_index)
                    .ok_or((400, "unknown strike".into()))?;
                strike_name = strike.name.clone();
                if kind == "strike" {
                    let modifier = m
                        .strike_attack_modifier(strike_index, map)
                        .ok_or((400, "unknown strike".into()))?;
                    let label = attack_label(&strike.name, map);
                    let expr = format!("1d20{modifier:+}");
                    let result = obb_core::dice::roll_expr(&expr, rng).map_err(|e| (400, e))?;
                    (
                        label,
                        result.expr.clone(),
                        Some(result.clone()),
                        result.total,
                        Some(modifier),
                        result.natural_d20,
                    )
                } else {
                    let expr = m
                        .strike_damage_expr(strike_index)
                        .ok_or((400, "unknown strike".into()))?;
                    let result = obb_core::dice::roll_expr(&expr, rng).map_err(|e| (400, e))?;
                    let total = if critical {
                        result.total.saturating_mul(2).max(1)
                    } else {
                        result.total.max(1)
                    };
                    let label = damage_label(&strike_name, critical);
                    (label, expr, Some(result), total, None, None)
                }
            }
        }
        _ => return Err((400, "unknown roll kind".into())),
    };
    let degree = dc
        .zip(natural)
        .map(|(dc, natural)| obb_core::dice::evaluate_check(total, dc, Some(natural)).id());
    let entry = if let Some(result) = result {
        roll_json(
            RollFields {
                actor: actor_id,
                actor_name: &actor_name,
                label: &label,
                expr: &expr,
                total,
                modifier,
                dc,
                degree,
                secret,
            },
            &result,
        )
    } else {
        let mut entry = JsonValue::object();
        entry.insert("actor", actor_id);
        entry.insert("actor_name", actor_name.as_str());
        entry.insert("label", label.as_str());
        entry.insert("expr", expr.as_str());
        entry.insert("total", total);
        entry.insert(
            "modifier",
            modifier.map_or(JsonValue::Null, JsonValue::from),
        );
        entry.insert("rolls", JsonValue::Array(Vec::new()));
        entry.insert("natural", JsonValue::Null);
        entry.insert("dc", dc.map_or(JsonValue::Null, JsonValue::from));
        entry.insert("degree", degree.map_or(JsonValue::Null, JsonValue::from));
        entry.insert("secret", secret);
        entry
    };
    Ok((entry.clone(), vec![entry]))
}

fn attack_label(name: &str, map: u8) -> String {
    match map {
        0 => name.to_string(),
        1 => format!("{name} (2nd attack)"),
        _ => format!("{name} (3rd attack)"),
    }
}

fn damage_label(name: &str, critical: bool) -> String {
    format!("{name} damage{}", if critical { " (critical)" } else { "" })
}

fn assurance_label(skill: &str) -> String {
    format!("Assurance ({skill})")
}

struct RollFields<'a> {
    actor: &'a str,
    actor_name: &'a str,
    label: &'a str,
    expr: &'a str,
    total: i32,
    modifier: Option<i32>,
    dc: Option<i32>,
    degree: Option<&'a str>,
    secret: bool,
}

fn roll_json(fields: RollFields<'_>, result: &obb_core::dice::RollResult) -> JsonValue {
    let mut entry = JsonValue::object();
    entry.insert("actor", fields.actor);
    entry.insert("actor_name", fields.actor_name);
    entry.insert("label", fields.label);
    entry.insert("expr", fields.expr);
    entry.insert("total", fields.total);
    entry.insert(
        "modifier",
        fields.modifier.map_or(JsonValue::Null, JsonValue::from),
    );
    entry.insert(
        "rolls",
        JsonValue::Array(
            result
                .terms
                .iter()
                .flat_map(|term| term.rolls.iter().copied().map(JsonValue::from))
                .collect(),
        ),
    );
    entry.insert(
        "natural",
        result
            .natural_d20
            .map_or(JsonValue::Null, |n| JsonValue::from(u32::from(n))),
    );
    entry.insert("dc", fields.dc.map_or(JsonValue::Null, JsonValue::from));
    entry.insert(
        "degree",
        fields.degree.map_or(JsonValue::Null, JsonValue::from),
    );
    entry.insert("secret", fields.secret);
    entry
}

fn initiative_request(
    state: &mut RoomState,
    body: &JsonValue,
    rng: &mut obb_core::dice::Rng,
) -> Result<JsonValue, (u16, String)> {
    let action = str_field(body, "action")?;
    let mut transition = JsonValue::Null;
    match action {
        "add" => {
            let actor_id = str_field(body, "actor")?;
            let (name, is_player, token_id) =
                if let Some(c) = state.characters.iter().find(|c| c.id == actor_id) {
                    (c.name.clone(), true, c.linked_token_id.clone())
                } else if let Some(m) = state.monsters.iter().find(|m| m.instance_id == actor_id) {
                    (m.label.clone(), false, m.linked_token_id.clone())
                } else {
                    return Err((404, "unknown actor".into()));
                };
            let mut combatant = encounter::Combatant::new(actor_id, actor_id, &name, is_player);
            combatant.token_id = token_id;
            state
                .encounter
                .add_combatant(combatant)
                .map_err(|e| (400, e))?;
            if body.get_bool("roll") == Some(true) {
                roll_initiative(
                    state,
                    actor_id,
                    body.get_str("check").unwrap_or("perception"),
                    rng,
                )?;
            }
        }
        "roll" => {
            let id = str_field(body, "combatant")?;
            roll_initiative(
                state,
                id,
                body.get_str("check").unwrap_or("perception"),
                rng,
            )?;
        }
        "set" => {
            let id = str_field(body, "combatant")?;
            let total = num(body, "total").ok_or((400, "total must be an integer".into()))?;
            let source = body.get_str("source").unwrap_or("GM set");
            state
                .encounter
                .set_initiative(id, total, 0, source)
                .map_err(|e| (404, e))?;
        }
        "delay" => {
            let id = str_field(body, "combatant")?;
            let total =
                num(body, "initiative").ok_or((400, "initiative must be an integer".into()))?;
            state
                .encounter
                .delay_combatant(id, total)
                .map_err(|e| (404, e))?;
        }
        "start" => {
            let t = state.encounter.start().map_err(|e| (400, e))?;
            transition = transition_json(&t);
        }
        "next" => {
            let mut actors: Vec<&mut dyn encounter::Actor> = Vec::new();
            actors.extend(
                state
                    .characters
                    .iter_mut()
                    .map(|c| c as &mut dyn encounter::Actor),
            );
            actors.extend(
                state
                    .monsters
                    .iter_mut()
                    .map(|m| m as &mut dyn encounter::Actor),
            );
            let t = state.encounter.next_turn(&mut actors);
            transition = transition_json(&t);
        }
        "end" => state.encounter.end(),
        "clear" => {
            state.encounter = encounter::EncounterTracker::new();
        }
        _ => return Err((400, "unknown initiative action".into())),
    }
    let mut value = JsonValue::object();
    value.insert("encounter", state.encounter.to_json());
    value.insert("transition", transition);
    Ok(value)
}

fn roll_initiative(
    state: &mut RoomState,
    combatant_id: &str,
    check: &str,
    rng: &mut obb_core::dice::Rng,
) -> Result<(), (u16, String)> {
    let combatant = state
        .encounter
        .combatants
        .iter()
        .find(|c| c.combatant_id == combatant_id)
        .ok_or((404, "unknown combatant".into()))?;
    let actor_id = combatant.actor_id.clone();
    if let Some(sheet) = state.characters.iter().find(|c| c.id == actor_id) {
        state
            .encounter
            .roll_initiative_for_character(combatant_id, sheet, check, rng)
            .map(|_| ())
            .map_err(|e| (400, e))
    } else if let Some(monster) = state.monsters.iter().find(|m| m.instance_id == actor_id) {
        let modifier = if check.eq_ignore_ascii_case("perception") {
            Some(monster.perception_modifier())
        } else {
            monster.skill_modifier(check)
        }
        .ok_or((400, "unknown initiative check".into()))?;
        let source = format!(
            "{} ({modifier:+})",
            if check.eq_ignore_ascii_case("perception") {
                "Perception"
            } else {
                check
            }
        );
        let result =
            obb_core::dice::roll_expr(&format!("1d20{modifier:+}"), rng).map_err(|e| (400, e))?;
        state
            .encounter
            .set_initiative(combatant_id, result.total, modifier, &source)
            .map_err(|e| (404, e))
    } else {
        Err((404, "unknown actor".into()))
    }
}

fn transition_json(t: &encounter::TurnTransition) -> JsonValue {
    let mut value = JsonValue::object();
    value.insert(
        "ended",
        t.ended.clone().map_or(JsonValue::Null, JsonValue::from),
    );
    value.insert(
        "started",
        t.started.clone().map_or(JsonValue::Null, JsonValue::from),
    );
    value.insert("round", i64::from(t.round));
    value.insert("new_round", t.new_round);
    value
}

fn aoe_request(body: &JsonValue) -> Result<(JsonValue, bool), (u16, String)> {
    let origin = body
        .get("origin")
        .ok_or((400, "origin is required".into()))?;
    let x = origin
        .get("x")
        .and_then(JsonValue::as_f64)
        .ok_or((400, "origin.x must be a number".into()))?;
    let y = origin
        .get("y")
        .and_then(JsonValue::as_f64)
        .ok_or((400, "origin.y must be a number".into()))?;
    let mut spell_value = JsonValue::Null;
    let area = if let Some(name) = body.get_str("spell") {
        let rank = body.get_i64("rank").unwrap_or(0);
        if !(0..=10).contains(&rank) {
            return Err((400, "rank must be 0 through 10".into()));
        }
        let spell =
            spells::lookup_common_spell(name, rank as u8).ok_or((404, "unknown spell".into()))?;
        let area = spell
            .area
            .clone()
            .ok_or((400, "spell has no area".into()))?;
        spell_value = spell.to_json();
        area
    } else if let Some(text) = body.get_str("text") {
        aoe::parse_area_description(text).ok_or((400, "invalid area description".into()))?
    } else if let Some(value) = body.get("area") {
        SpellArea::from_json(value).map_err(|e| (400, e))?
    } else {
        return Err((400, "spell, text, or area is required".into()));
    };
    let label = body
        .get_str("label")
        .map(str::to_owned)
        .or_else(|| spell_value.get_str("name").map(str::to_owned))
        .unwrap_or_else(|| area.label());
    let color = body.get_str("color").unwrap_or("#ff0000");
    let grid_dpi = body
        .get("grid_dpi")
        .and_then(JsonValue::as_f64)
        .unwrap_or(100.0);
    let size = body.get_i64("token_size_squares").unwrap_or(1).max(1) as u32;
    let mut result = aoe::compute_obr_aoe_item(&area, x, y, grid_dpi, size, color, &label);
    result.insert("label", label.as_str());
    result.insert("fill_color", color);
    result.insert("spell", spell_value);
    let keep = body.get_bool("keep").unwrap_or(false);
    Ok((result, keep))
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

#[cfg(test)]
mod tests {
    use super::{assurance_label, attack_label, damage_label};

    #[test]
    fn roll_labels_show_attack_order_and_critical_damage() {
        assert_eq!(attack_label("Strike", 1), "Strike (2nd attack)");
        assert_eq!(attack_label("Strike", 2), "Strike (3rd attack)");
        assert_eq!(damage_label("Strike", true), "Strike damage (critical)");
    }

    #[test]
    fn assurance_label_names_the_skill() {
        assert_eq!(assurance_label("Athletics"), "Assurance (Athletics)");
    }
}
