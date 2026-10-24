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

use std::path::PathBuf;

use crate::http::RunningServer;

/// See `crate::start`.
pub fn start(
    bind: &str,
    state_file: Option<PathBuf>,
    seed: Option<u64>,
) -> Result<RunningServer, String> {
    todo!()
}
