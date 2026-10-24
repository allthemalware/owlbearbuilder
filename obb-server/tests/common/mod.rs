// SPDX-License-Identifier: GPL-3.0-or-later
#![allow(dead_code)]

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::time::Duration;

use obb_core::json::JsonValue;

pub const AMIRI: &str = r#"{"success":true,"build":{"name":"Amiri","class":"Fighter","level":5,
  "ancestry":"Human","heritage":"Versatile Human","background":"Warrior","sizeName":"Medium","keyability":"str",
  "attributes":{"ancestryhp":8,"classhp":10,"bonushp":0,"bonushpPerLevel":0,"speed":25,"speedBonus":0},
  "abilities":{"str":18,"dex":14,"con":14,"int":10,"wis":12,"cha":10},
  "proficiencies":{"classDC":2,"perception":4,"fortitude":4,"reflex":4,"will":2,"athletics":4,"intimidation":2},
  "lores":[["Warfare",2]],"feats":[],"specials":[],
  "weapons":[{"name":"Longsword","display":"+1 Striking Longsword","die":"d8","pot":1,"str":"striking","attack":16,"damageBonus":4,"damageType":"S","extraDamage":[]}],
  "armor":[],"spellCasters":[],"focusPoints":0,"focus":{},
  "acTotal":{"acTotal":22,"shieldBonus":"2"}}}"#;

pub const ORC: &str = "ORC BRUTE CREATURE 0
MEDIUM ORC HUMANOID
Perception +5; darkvision
Skills Athletics +6, Intimidation +4
Str +3, Dex +2, Con +3, Int -1, Wis +1, Cha +0
AC 15; Fort +8, Ref +5, Will +4
HP 15
Speed 25 feet
Melee [one-action] fist +7 (agile, nonlethal), Damage 1d4+3 bludgeoning
Melee [one-action] falchion +7 (forceful, sweep), Damage 1d10+3 slashing
";

pub fn temp_state_file(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("obb-test-{}-{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("room.json")
}

/// Raw exchange: returns (status, headers text, body text).
pub fn raw(
    addr: SocketAddr,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> (u16, String, String) {
    let mut s = TcpStream::connect(addr).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    let body = body.unwrap_or("");
    let req = if method == "POST" {
        format!("{method} {path} HTTP/1.1\r\nHost: test\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", body.len())
    } else {
        format!("{method} {path} HTTP/1.1\r\nHost: test\r\n\r\n")
    };
    s.write_all(req.as_bytes()).unwrap();
    let mut out = String::new();
    s.read_to_string(&mut out).unwrap();
    let (head, body) = out.split_once("\r\n\r\n").unwrap();
    let status = head[9..12].parse().unwrap();
    (status, head.to_string(), body.to_string())
}

pub fn get(addr: SocketAddr, path: &str) -> (u16, JsonValue) {
    let (status, _, body) = raw(addr, "GET", path, None);
    (status, JsonValue::parse(&body).unwrap_or(JsonValue::Null))
}

pub fn post(addr: SocketAddr, path: &str, body: &str) -> (u16, JsonValue) {
    let (status, _, text) = raw(addr, "POST", path, Some(body));
    (
        status,
        JsonValue::parse(&text).unwrap_or_else(|e| panic!("{path}: {e}: {text}")),
    )
}

/// POST that must succeed; returns the JSON body.
pub fn ok(addr: SocketAddr, path: &str, body: &str) -> JsonValue {
    let (status, v) = post(addr, path, body);
    assert_eq!(status, 200, "{path} {body} -> {}", v.to_json_string());
    v
}
