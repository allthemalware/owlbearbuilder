// SPDX-License-Identifier: GPL-3.0-or-later

use obb_core::json::{JsonValue, MAX_DEPTH};

#[test]
fn parses_a_pathbuilder_shaped_document() {
    let doc = r#"{"success": true, "build": {"name": "Valeros", "level": 5,
        "abilities": {"str": 18, "dex": 14}, "lores": [["Warfare", 2]],
        "money": {"gp": 12.5}, "familiar": null}}"#;
    let v = JsonValue::parse(doc).unwrap();
    assert_eq!(v.get_bool("success"), Some(true));
    let build = v.get("build").unwrap();
    assert_eq!(build.get_str("name"), Some("Valeros"));
    assert_eq!(build.get_i64("level"), Some(5));
    assert_eq!(build.get("abilities").unwrap().get_i64("dex"), Some(14));
    let lore = &build.get_array("lores").unwrap()[0];
    assert_eq!(lore.as_array().unwrap()[0].as_str(), Some("Warfare"));
    assert_eq!(build.get("money").unwrap().get_f64("gp"), Some(12.5));
    assert_eq!(build.get("money").unwrap().get_i64("gp"), None);
    assert!(build.get("familiar").unwrap().is_null());
    assert_eq!(build.get("missing"), None);
    assert_eq!(build.get_str("level"), None);
}

#[test]
fn decodes_every_escape() {
    let v = JsonValue::parse(r#""q\" b\\ s\/ \b\f\n\r\t \u00e9 \uD83C\uDFB2 \uD800x""#).unwrap();
    assert_eq!(v.as_str(), Some("q\" b\\ s/ \u{8}\u{c}\n\r\t é 🎲 \u{FFFD}x"));
}

#[test]
fn parses_numbers() {
    for (text, want) in [("0", 0.0), ("-1", -1.0), ("3.25", 3.25), ("-0.5e1", -5.0), ("1E+2", 100.0), ("2e-2", 0.02)] {
        assert_eq!(JsonValue::parse(text).unwrap().as_f64(), Some(want), "{text}");
    }
    assert_eq!(JsonValue::parse("-7").unwrap().as_i64(), Some(-7));
    assert_eq!(JsonValue::parse("2.0").unwrap().as_i64(), Some(2));
}

#[test]
fn rejects_malformed_input_with_position() {
    for bad in ["", "{", "[1,]", "{\"a\":1,}", "01", "1.", ".5", "+1", "tru", "\"abc", "\"\\x\"", "\"a\nb\"", "{a:1}", "[1 2]", "1 2", "'a'", "NaN"] {
        assert!(JsonValue::parse(bad).is_err(), "accepted {bad:?}");
    }
    let err = JsonValue::parse("{\n  \"a\": tru\n}").unwrap_err();
    assert_eq!(err.line, 2);
    assert_eq!(err.column, 8);
    assert!(err.to_string().contains("line 2"));
}

#[test]
fn accepts_bom_and_surrounding_whitespace() {
    let v = JsonValue::parse("\u{FEFF} \n\t{\"a\": [ ]}\r\n ").unwrap();
    assert_eq!(v.get_array("a").map(|a| a.len()), Some(0));
}

#[test]
fn limits_nesting_depth() {
    let ok = "[".repeat(MAX_DEPTH) + &"]".repeat(MAX_DEPTH);
    assert!(JsonValue::parse(&ok).is_ok());
    let deep = "[".repeat(MAX_DEPTH + 1) + &"]".repeat(MAX_DEPTH + 1);
    assert!(JsonValue::parse(&deep).is_err());
    let hostile = "[".repeat(1_000_000);
    assert!(JsonValue::parse(&hostile).is_err());
}

#[test]
fn duplicate_keys_keep_first_position_and_last_value() {
    let v = JsonValue::parse(r#"{"a": 1, "b": 2, "a": 3}"#).unwrap();
    assert_eq!(v.to_json_string(), r#"{"a":3,"b":2}"#);
}

#[test]
fn writes_compact_and_deterministic() {
    let mut v = JsonValue::object();
    v.insert("name", "Ezren \"the\" Wizard\n");
    v.insert("level", 3);
    v.insert("ratio", 0.1);
    v.insert("big", 1e21);
    v.insert("nan", f64::NAN);
    v.insert("ok", true);
    v.insert("none", Option::<i32>::None);
    v.insert("list", vec![JsonValue::from(1), JsonValue::from("é/\u{1}")]);
    v.insert("level", 4);
    assert_eq!(
        v.to_json_string(),
        r#"{"name":"Ezren \"the\" Wizard\n","level":4,"ratio":0.1,"big":1000000000000000000000,"nan":null,"ok":true,"none":null,"list":[1,"é/\u0001"]}"#
    );
    assert_eq!(v.to_string(), v.to_json_string());
}

#[test]
fn writes_pretty() {
    let v = JsonValue::parse(r#"{"a":[1,{"b":null}],"c":{},"d":[]}"#).unwrap();
    assert_eq!(
        v.to_json_pretty(),
        "{\n  \"a\": [\n    1,\n    {\n      \"b\": null\n    }\n  ],\n  \"c\": {},\n  \"d\": []\n}"
    );
}

#[test]
fn round_trips() {
    let doc = r#"{"s":"tab\there \\ \"q\" 🎲","n":[-0.25,7,1.5e300],"o":{"x":false,"y":null}}"#;
    let v = JsonValue::parse(doc).unwrap();
    assert_eq!(JsonValue::parse(&v.to_json_string()).unwrap(), v);
    assert_eq!(JsonValue::parse(&v.to_json_pretty()).unwrap(), v);
}

#[test]
fn get_mut_edits_in_place() {
    let mut v = JsonValue::parse(r#"{"hp": {"current": 10}}"#).unwrap();
    v.get_mut("hp").unwrap().insert("current", 7);
    assert_eq!(v.get("hp").unwrap().get_i64("current"), Some(7));
}
