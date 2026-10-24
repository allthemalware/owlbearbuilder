// SPDX-License-Identifier: GPL-3.0-or-later

mod common;

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

use common::*;
use obb_core::json::JsonValue;

#[test]
fn serves_assets_and_errors() {
    let server = obb_server::start("127.0.0.1:0", None, Some(1)).unwrap();
    let addr = server.addr;
    let (status, head, body) = raw(addr, "GET", "/manifest.json", None);
    assert_eq!(status, 200);
    assert!(head.contains("application/json"), "{head}");
    assert_eq!(
        JsonValue::parse(&body).unwrap().get_str("name"),
        Some("OwlbearBuilder")
    );
    let (status, head, _) = raw(addr, "GET", "/icon.svg", None);
    assert!(status == 200 && head.contains("image/svg+xml"), "{head}");
    for path in ["/", "/index.html"] {
        let (status, head, _) = raw(addr, "GET", path, None);
        assert!(
            status == 200 && head.contains("text/html"),
            "{path}: {head}"
        );
    }
    let (status, head, _) = raw(addr, "GET", "/obr_bridge.js", None);
    assert!(status == 200 && head.contains("javascript"), "{head}");
    assert_eq!(get(addr, "/api/nope").0, 404);
    assert_eq!(get(addr, "/api/import/character").0, 405);
    assert_eq!(post(addr, "/api/character/update", "not json").0, 400);
    server.shutdown();
}

#[test]
fn characters_import_merge_update_and_remove() {
    let server = obb_server::start("127.0.0.1:0", None, Some(1)).unwrap();
    let addr = server.addr;
    let first = ok(addr, "/api/import/character", AMIRI);
    assert_eq!(first.get_str("id"), Some("amiri"));
    assert_eq!(first.get_bool("merged"), Some(false));

    let updated = ok(
        addr,
        "/api/character/update",
        r#"{"id": "amiri", "damage": 10, "condition": {"name": "frightened", "value": 2}, "linked_token_id": "tok-1"}"#,
    );
    let entry = updated.get("character").unwrap();
    let max = entry.get("sheet").unwrap().get_i64("max_hp").unwrap();
    assert_eq!(
        entry.get("sheet").unwrap().get_i64("current_hp"),
        Some(max - 10)
    );
    assert_eq!(
        entry.get("row").unwrap().get_array("conditions").unwrap()[0].as_str(),
        Some("Frightened 2")
    );
    assert_eq!(entry.get("row").unwrap().get_i64("ac"), Some(20));
    let token = entry.get("token").unwrap();
    assert_eq!(
        (token.get_str("kind"), token.get_i64("hp")),
        (Some("character"), Some(max - 10))
    );

    let level6 = AMIRI.replace(r#""level":5"#, r#""level":6"#);
    let merged = ok(addr, "/api/import/character", &level6);
    assert_eq!(merged.get_bool("merged"), Some(true));
    let changes: Vec<&str> = merged
        .get_array("changes")
        .unwrap()
        .iter()
        .filter_map(|c| c.as_str())
        .collect();
    assert!(changes.contains(&"Level: 5 -> 6"), "{changes:?}");

    let (_, state) = get(addr, "/api/state");
    let chars = state.get_array("characters").unwrap();
    assert_eq!(chars.len(), 1);
    let sheet = chars[0].get("sheet").unwrap();
    assert_eq!(sheet.get_i64("level"), Some(6));
    assert_eq!(
        sheet.get_str("linked_token_id"),
        Some("tok-1"),
        "re-import keeps the token link"
    );
    assert!(chars[0].get_array("spellbook").is_some());

    assert_eq!(
        post(
            addr,
            "/api/character/update",
            r#"{"id": "nobody", "damage": 1}"#
        )
        .0,
        404
    );
    assert_eq!(
        post(addr, "/api/import/character", r#"{"hello": 1}"#).0,
        400
    );
    assert_eq!(
        ok(addr, "/api/character/remove", r#"{"id": "amiri"}"#).get_str("removed"),
        Some("amiri")
    );
    assert!(get(addr, "/api/state")
        .1
        .get_array("characters")
        .unwrap()
        .is_empty());
    server.shutdown();
}

#[test]
fn monsters_import_adjust_and_bind() {
    let server = obb_server::start("127.0.0.1:0", None, Some(1)).unwrap();
    let addr = server.addr;
    let (_, state) = get(addr, "/api/state");
    let bestiary = state.get_array("bestiary").unwrap();
    assert_eq!(bestiary.len(), 6);
    assert_eq!(bestiary[0].get_str("id"), Some("goblin-warrior"));

    let gobs = ok(
        addr,
        "/api/monster/import",
        r#"{"builtin": "goblin-warrior", "count": 2, "variant": "elite"}"#,
    );
    let list = gobs.get_array("instances").unwrap();
    assert_eq!(list.len(), 2);
    let ids: Vec<&str> = list
        .iter()
        .map(|m| m.get("instance").unwrap().get_str("instance_id").unwrap())
        .collect();
    assert_eq!(ids, ["goblin-warrior-1", "goblin-warrior-2"]);
    assert_eq!(
        list[1].get("instance").unwrap().get_str("label"),
        Some("Goblin Warrior 2")
    );
    assert_eq!(list[0].get("stats").unwrap().get_i64("level"), Some(1));

    let orc = ok(
        addr,
        "/api/monster/import",
        &format!("{{\"text\": {}}}", JsonValue::from(ORC).to_json_string()),
    );
    assert_eq!(
        orc.get_array("instances").unwrap()[0]
            .get("instance")
            .unwrap()
            .get_str("instance_id"),
        Some("orc-brute-1")
    );

    let weak = ok(
        addr,
        "/api/monster/update",
        r#"{"id": "goblin-warrior-1", "variant": "weak", "damage": 1}"#,
    );
    assert_eq!(
        weak.get("monster")
            .unwrap()
            .get("instance")
            .unwrap()
            .get_str("variant"),
        Some("weak")
    );
    ok(addr, "/api/monster/remove", r#"{"id": "goblin-warrior-2"}"#);
    let again = ok(
        addr,
        "/api/monster/import",
        r#"{"builtin": "goblin-warrior"}"#,
    );
    assert_eq!(
        again.get_array("instances").unwrap()[0]
            .get("instance")
            .unwrap()
            .get_str("instance_id"),
        Some("goblin-warrior-3")
    );

    assert_eq!(
        post(
            addr,
            "/api/monster/import",
            r#"{"builtin": "goblin-warrior", "text": "x"}"#
        )
        .0,
        400
    );
    assert_eq!(
        post(
            addr,
            "/api/monster/import",
            r#"{"builtin": "goblin-warrior", "count": 0}"#
        )
        .0,
        400
    );
    assert_eq!(
        post(addr, "/api/monster/import", r#"{"builtin": "tarrasque"}"#).0,
        404
    );
    assert_eq!(
        post(
            addr,
            "/api/monster/import",
            r#"{"text": "not a stat block"}"#
        )
        .0,
        400
    );

    let token = ok(addr, "/api/token", r#"{"actor": "orc-brute-1"}"#);
    assert_eq!(
        (token.get_str("kind"), token.get_i64("max_hp")),
        (Some("monster"), Some(15))
    );
    assert_eq!(post(addr, "/api/token", r#"{"actor": "nobody"}"#).0, 404);
    assert_eq!(
        ok(addr, "/api/target", r#"{"dc": 18}"#).get_i64("target_dc"),
        Some(18)
    );
    assert!(ok(addr, "/api/target", r#"{"dc": null}"#)
        .get("target_dc")
        .unwrap()
        .is_null());
    server.shutdown();
}

#[test]
fn changes_are_broadcast_and_persisted() {
    let file = temp_state_file("persist");
    let server = obb_server::start("127.0.0.1:0", Some(file.clone()), Some(1)).unwrap();
    let addr = server.addr;
    ok(addr, "/api/import/character", AMIRI);

    let mut stream = TcpStream::connect(addr).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    stream
        .write_all(b"GET /api/events HTTP/1.1\r\nHost: test\r\n\r\n")
        .unwrap();
    let mut reader = BufReader::new(stream);
    let mut next_state = || -> JsonValue {
        let mut event = String::new();
        loop {
            let mut line = String::new();
            assert!(reader.read_line(&mut line).unwrap() > 0, "stream closed");
            if line.starts_with("event: state") {
                event.clear();
            } else if let Some(data) = line.strip_prefix("data: ") {
                event.push_str(data.trim_end_matches('\n'));
            } else if line == "\n" && !event.is_empty() {
                return JsonValue::parse(&event).unwrap();
            }
        }
    };
    let first = next_state();
    let rev = first.get_i64("revision").unwrap();
    assert_eq!(first.get_array("characters").unwrap().len(), 1);
    ok(addr, "/api/target", r#"{"dc": 20}"#);
    let second = next_state();
    assert_eq!(second.get_i64("target_dc"), Some(20));
    assert!(second.get_i64("revision").unwrap() > rev);
    server.shutdown();

    let saved = JsonValue::parse(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(saved.get_str("format"), Some("owlbearbuilder-room"));
    let reopened = obb_server::start("127.0.0.1:0", Some(file.clone()), Some(1)).unwrap();
    let (_, state) = get(reopened.addr, "/api/state");
    assert_eq!(state.get_array("characters").unwrap().len(), 1);
    assert_eq!(state.get_i64("target_dc"), Some(20));
    reopened.shutdown();

    std::fs::write(&file, "{ this is not a room").unwrap();
    assert!(obb_server::start("127.0.0.1:0", Some(file), Some(1)).is_err());
}
