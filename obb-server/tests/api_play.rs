// SPDX-License-Identifier: GPL-3.0-or-later

mod common;

use common::*;
use obb_core::dice::evaluate_check;
use obb_core::json::JsonValue;

fn table() -> obb_server::RunningServer {
    let server = obb_server::start("127.0.0.1:0", None, Some(42)).unwrap();
    ok(server.addr, "/api/import/character", AMIRI);
    let orc = format!("{{\"text\": {}}}", JsonValue::from(ORC).to_json_string());
    ok(server.addr, "/api/monster/import", &orc);
    server
}

fn natural(entry: &JsonValue) -> i64 {
    entry.get_i64("natural").unwrap()
}

fn degree_for(entry: &JsonValue, dc: i32) -> String {
    let total = entry.get_i64("total").unwrap() as i32;
    evaluate_check(total, dc, Some(natural(entry) as u8))
        .id()
        .to_string()
}

#[test]
fn character_checks() {
    let server = table();
    let addr = server.addr;
    let p = ok(
        addr,
        "/api/roll",
        r#"{"actor": "amiri", "kind": "perception"}"#,
    );
    assert_eq!(
        (p.get_str("label"), p.get_str("expr")),
        (Some("Perception"), Some("1d20+10"))
    );
    assert_eq!(p.get_i64("total"), Some(natural(&p) + 10));
    assert!(p.get("dc").unwrap().is_null() && p.get("degree").unwrap().is_null());
    assert_eq!(
        (
            p.get_str("actor"),
            p.get_str("actor_name"),
            p.get_bool("secret")
        ),
        (Some("amiri"), Some("Amiri"), Some(false))
    );

    ok(addr, "/api/target", r#"{"dc": 15}"#);
    let fort = ok(
        addr,
        "/api/roll",
        r#"{"actor": "amiri", "kind": "save", "name": "fortitude"}"#,
    );
    assert_eq!(
        (
            fort.get_str("label"),
            fort.get_str("expr"),
            fort.get_i64("dc")
        ),
        (Some("Fortitude save"), Some("1d20+11"), Some(15))
    );
    assert_eq!(fort.get_str("degree"), Some(degree_for(&fort, 15).as_str()));

    let ath = ok(
        addr,
        "/api/roll",
        r#"{"actor": "amiri", "kind": "skill", "name": "Athletics", "secret": true}"#,
    );
    assert_eq!(
        (
            ath.get_str("label"),
            ath.get_str("expr"),
            ath.get_bool("secret")
        ),
        (Some("Athletics"), Some("1d20+13"), Some(true))
    );
    let assurance = ok(
        addr,
        "/api/roll",
        r#"{"actor": "amiri", "kind": "assurance", "name": "athletics"}"#,
    );
    assert_eq!(
        (assurance.get_str("label"), assurance.get_i64("total")),
        (Some("Assurance (Athletics)"), Some(19))
    );
    assert!(
        assurance.get_array("rolls").unwrap().is_empty()
            && assurance.get("natural").unwrap().is_null()
    );
    let lore = ok(
        addr,
        "/api/roll",
        r#"{"actor": "amiri", "kind": "skill", "name": "Warfare Lore"}"#,
    );
    assert_eq!(lore.get_str("expr"), Some("1d20+7"));
    server.shutdown();
}

#[test]
fn strikes_and_damage() {
    let server = table();
    let addr = server.addr;
    let second = ok(
        addr,
        "/api/roll",
        r#"{"actor": "amiri", "kind": "strike", "index": 0, "map": 1, "dc": 20}"#,
    );
    assert_eq!(
        second.get_str("label"),
        Some("+1 Striking Longsword (2nd attack)")
    );
    assert_eq!(
        (second.get_str("expr"), second.get_i64("dc")),
        (Some("1d20+11"), Some(20))
    );
    assert_eq!(
        second.get_str("degree"),
        Some(degree_for(&second, 20).as_str())
    );
    let hit = ok(
        addr,
        "/api/roll",
        r#"{"actor": "amiri", "kind": "damage", "index": 0}"#,
    );
    assert_eq!(
        (hit.get_str("label"), hit.get_str("expr")),
        (Some("+1 Striking Longsword damage"), Some("2d8+4"))
    );
    let crit = ok(
        addr,
        "/api/roll",
        r#"{"actor": "amiri", "kind": "damage", "index": 0, "critical": true}"#,
    );
    assert_eq!(
        crit.get_str("label"),
        Some("+1 Striking Longsword damage (critical)")
    );
    assert!(crit.get("dc").unwrap().is_null() && crit.get("degree").unwrap().is_null());
    assert!((12..=40).contains(&crit.get_i64("total").unwrap()));
    let any = ok(
        addr,
        "/api/roll",
        r#"{"actor": "amiri", "kind": "expr", "expr": "2d6+3"}"#,
    );
    assert!((5..=15).contains(&any.get_i64("total").unwrap()));
    assert_eq!(
        post(
            addr,
            "/api/roll",
            r#"{"actor": "amiri", "kind": "strike", "index": 9}"#
        )
        .0,
        400
    );
    assert_eq!(
        post(
            addr,
            "/api/roll",
            r#"{"actor": "amiri", "kind": "expr", "expr": "2x6"}"#
        )
        .0,
        400
    );
    server.shutdown();
}

#[test]
fn monster_rolls_party_checks_and_errors() {
    let server = table();
    let addr = server.addr;
    let falchion = ok(
        addr,
        "/api/roll",
        r#"{"actor": "orc-brute-1", "kind": "strike", "index": 1, "map": 0}"#,
    );
    assert_eq!(
        (falchion.get_str("expr"), falchion.get_str("actor_name")),
        (Some("1d20+7"), Some("Orc Brute 1"))
    );
    let will = ok(
        addr,
        "/api/roll",
        r#"{"actor": "orc-brute-1", "kind": "save", "name": "will"}"#,
    );
    assert_eq!(will.get_str("expr"), Some("1d20+4"));
    let ath = ok(
        addr,
        "/api/roll",
        r#"{"actor": "orc-brute-1", "kind": "skill", "name": "athletics"}"#,
    );
    assert_eq!(ath.get_str("expr"), Some("1d20+6"));
    let dmg = ok(
        addr,
        "/api/roll",
        r#"{"actor": "orc-brute-1", "kind": "damage", "index": 1}"#,
    );
    assert_eq!(dmg.get_str("expr"), Some("1d10+3"));

    let party = ok(
        addr,
        "/api/roll",
        r#"{"kind": "party", "name": "perception", "dc": 15}"#,
    );
    let results = party.get_array("results").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].get_bool("secret"), Some(true));
    assert_eq!(
        results[0].get_str("degree"),
        Some(degree_for(&results[0], 15).as_str())
    );

    assert_eq!(
        post(addr, "/api/roll", r#"{"actor": "amiri", "kind": "juggle"}"#).0,
        400
    );
    assert_eq!(
        post(
            addr,
            "/api/roll",
            r#"{"actor": "nobody", "kind": "perception"}"#
        )
        .0,
        404
    );
    assert_eq!(
        post(
            addr,
            "/api/roll",
            r#"{"actor": "orc-brute-1", "kind": "assurance", "name": "athletics"}"#
        )
        .0,
        400
    );

    for _ in 0..55 {
        ok(
            addr,
            "/api/roll",
            r#"{"actor": "amiri", "kind": "expr", "expr": "1d4"}"#,
        );
    }
    let (_, state) = get(addr, "/api/state");
    let log = state.get_array("roll_log").unwrap();
    assert_eq!(log.len(), 50);
    assert!(log[49].get_i64("id").unwrap() > log[0].get_i64("id").unwrap());
    server.shutdown();
}

#[test]
fn initiative_flow() {
    let server = table();
    let addr = server.addr;
    let init = |body: &str| ok(addr, "/api/initiative", body);
    init(r#"{"action": "add", "actor": "amiri", "roll": true}"#);
    let after = init(r#"{"action": "add", "actor": "orc-brute-1", "roll": true}"#);
    let combatants = after
        .get("encounter")
        .unwrap()
        .get_array("combatants")
        .unwrap();
    assert_eq!(combatants.len(), 2);
    assert!(combatants.iter().all(|c| c.get_i64("initiative").is_some()));
    assert_eq!(
        post(
            addr,
            "/api/initiative",
            r#"{"action": "add", "actor": "amiri"}"#
        )
        .0,
        400
    );

    ok(
        addr,
        "/api/monster/update",
        r#"{"id": "orc-brute-1", "condition": {"name": "frightened", "value": 2}}"#,
    );
    let started = init(r#"{"action": "start"}"#);
    assert_eq!(
        started.get("encounter").unwrap().get_bool("active"),
        Some(true)
    );
    assert!(started
        .get("transition")
        .unwrap()
        .get_str("started")
        .is_some());
    init(r#"{"action": "next"}"#);
    let wrapped = init(r#"{"action": "next"}"#);
    assert_eq!(wrapped.get("encounter").unwrap().get_i64("round"), Some(2));
    let (_, state) = get(addr, "/api/state");
    let orc = &state.get_array("monsters").unwrap()[0];
    assert_eq!(
        orc.get("instance")
            .unwrap()
            .get_array("conditions")
            .unwrap()[0]
            .get_i64("value"),
        Some(1)
    );

    init(r#"{"action": "delay", "combatant": "amiri", "initiative": 1}"#);
    init(r#"{"action": "set", "combatant": "orc-brute-1", "total": 30}"#);
    assert_eq!(
        post(
            addr,
            "/api/initiative",
            r#"{"action": "roll", "combatant": "nobody"}"#
        )
        .0,
        404
    );
    assert_eq!(
        post(addr, "/api/initiative", r#"{"action": "dance"}"#).0,
        400
    );
    let ended = init(r#"{"action": "end"}"#);
    assert_eq!(
        ended.get("encounter").unwrap().get_bool("active"),
        Some(false)
    );
    let cleared = init(r#"{"action": "clear"}"#);
    assert!(cleared
        .get("encounter")
        .unwrap()
        .get_array("combatants")
        .unwrap()
        .is_empty());
    server.shutdown();
}

#[test]
fn spell_areas() {
    let server = table();
    let addr = server.addr;
    let fireball = ok(
        addr,
        "/api/aoe/compute",
        r#"{"spell": "Fireball", "origin": {"x": 300, "y": 450}}"#,
    );
    assert_eq!(
        (fireball.get_str("shape_type"), fireball.get_str("label")),
        (Some("CIRCLE"), Some("Fireball"))
    );
    assert_eq!(fireball.get_array("squares").unwrap().len(), 52);
    assert_eq!(
        fireball.get("spell").unwrap().get_str("damage_expr"),
        Some("6d6")
    );
    let big = ok(
        addr,
        "/api/aoe/compute",
        r#"{"spell": "fireball", "rank": 5, "origin": {"x": 0, "y": 0}}"#,
    );
    assert_eq!(
        big.get("spell").unwrap().get_str("damage_expr"),
        Some("10d6")
    );
    let cone = ok(
        addr,
        "/api/aoe/compute",
        r##"{"text": "15-foot cone", "direction_deg": 90, "origin": {"x": 75, "y": 75}, "color": "#123456", "keep": true}"##,
    );
    assert_eq!(
        (
            cone.get_str("shape_type"),
            cone.get_str("label"),
            cone.get_str("fill_color")
        ),
        (Some("POLYGON"), Some("15-foot cone"), Some("#123456"))
    );
    let small = ok(
        addr,
        "/api/aoe/compute",
        r#"{"area": {"kind": "burst", "radius_ft": 10}, "origin": {"x": 0, "y": 0}}"#,
    );
    assert_eq!(small.get_array("squares").unwrap().len(), 16);
    assert_eq!(
        get(addr, "/api/state")
            .1
            .get_array("aoe_templates")
            .unwrap()
            .len(),
        1
    );

    assert_eq!(
        post(addr, "/api/aoe/compute", r#"{"spell": "Fireball"}"#).0,
        400
    );
    assert_eq!(
        post(
            addr,
            "/api/aoe/compute",
            r#"{"spell": "Ward of Whispers", "origin": {"x": 0, "y": 0}}"#
        )
        .0,
        404
    );
    assert_eq!(
        post(
            addr,
            "/api/aoe/compute",
            r#"{"text": "a lovely day", "origin": {"x": 0, "y": 0}}"#
        )
        .0,
        400
    );
    assert_eq!(ok(addr, "/api/aoe/clear", "{}").get_i64("cleared"), Some(1));
    assert!(get(addr, "/api/state")
        .1
        .get_array("aoe_templates")
        .unwrap()
        .is_empty());
    server.shutdown();
}
