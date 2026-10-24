// SPDX-License-Identifier: GPL-3.0-or-later

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::time::Duration;

use obb_core::json::JsonValue;

const BIN: &str = env!("CARGO_BIN_EXE_owlbearbuilder");

fn run(args: &[&str]) -> Output {
    Command::new(BIN).args(args).output().unwrap()
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn fixture(name: &str) -> String {
    format!(
        "{}/../obb-core/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("obb-cli-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn help_version_and_usage_errors() {
    for args in [&[][..], &["--help"], &["help"]] {
        let o = run(args);
        assert!(o.status.success(), "{args:?}");
        let text = stdout(&o);
        for cmd in ["serve", "inspect", "roll", "parse-monster", "export-static"] {
            assert!(text.contains(cmd), "usage lists {cmd}: {text}");
        }
    }
    let v = run(&["--version"]);
    assert_eq!(
        stdout(&v).trim(),
        format!("owlbearbuilder {}", env!("CARGO_PKG_VERSION"))
    );
    let bad = run(&["frobnicate"]);
    assert_eq!(bad.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&bad.stderr).contains("serve"));
    assert_eq!(run(&["roll"]).status.code(), Some(2));
}

#[test]
fn inspects_characters() {
    let o = run(&["inspect", &fixture("pb_fighter_lvl5.json")]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let text = stdout(&o);
    for want in [
        "Fighter",
        "Level 5",
        "HP 68",
        "AC 19",
        "Perception +",
        "Fortitude +",
        "Longsword",
        "+16 / +11 / +6",
        "Athletics",
    ] {
        assert!(text.contains(want), "missing {want:?} in:\n{text}");
    }
    let wiz = stdout(&run(&["inspect", &fixture("pb_wizard_lvl5.json")]));
    assert!(
        wiz.contains("Fireball") && wiz.contains("20-foot burst"),
        "{wiz}"
    );
    // Level 5, Int +4: trained class DC 10 + 7 + 4; Arcana expert 9 + 4; Society trained 7 + 4.
    for want in [
        "Class DC 21",
        "Arcana +13 (Expert)",
        "Society +11 (Trained)",
        "Wizard (arcane prepared, DC 21, attack +11)",
    ] {
        assert!(wiz.contains(want), "missing {want:?} in:\n{wiz}");
    }
    assert!(!wiz.contains("DC +"), "a DC is not a modifier:\n{wiz}");
    let missing = run(&["inspect", "/nonexistent/sheet.json"]);
    assert_eq!(missing.status.code(), Some(1));
    assert!(!missing.stderr.is_empty());
}

#[test]
fn parses_monsters_with_variants() {
    let dir = temp_dir("monster");
    let path = dir.join("orc.txt");
    std::fs::write(&path, "ORC BRUTE CREATURE 0\nMEDIUM ORC HUMANOID\nPerception +5; darkvision\nAC 15; Fort +8, Ref +5, Will +4\nHP 15\nSpeed 25 feet\nMelee [one-action] falchion +7 (forceful, sweep), Damage 1d10+3 slashing\n").unwrap();
    let p = path.to_str().unwrap();
    let normal = stdout(&run(&["parse-monster", p]));
    assert!(
        normal.contains("Orc Brute") && normal.contains("AC 15") && normal.contains("falchion"),
        "{normal}"
    );
    let elite = stdout(&run(&["parse-monster", p, "--elite"]));
    assert!(
        elite.contains("Elite")
            && elite.contains("Level 2")
            && elite.contains("AC 17")
            && elite.contains("1d10+5"),
        "{elite}"
    );
    let weak = run(&["parse-monster", p, "--weak", "--json"]);
    let json = JsonValue::parse(&stdout(&weak)).unwrap();
    assert_eq!(
        (json.get_i64("level"), json.get_i64("ac")),
        (Some(-1), Some(13))
    );
    assert_eq!(
        run(&["parse-monster", p, "--elite", "--weak"])
            .status
            .code(),
        Some(2)
    );
    std::fs::write(&path, "nothing useful").unwrap();
    assert_eq!(run(&["parse-monster", p]).status.code(), Some(1));
}

#[test]
fn rolls_seeded_dice() {
    let a = run(&["roll", "1d20+14", "--dc", "25", "--seed", "42"]);
    assert!(a.status.success());
    let text = stdout(&a);
    assert_eq!(
        text,
        stdout(&run(&["roll", "1d20+14", "--dc", "25", "--seed", "42"])),
        "seeded rolls repeat"
    );
    assert!(text.contains("1d20+14") && text.contains("DC 25"), "{text}");
    // `<expr>: <each die> + <flat> = <total>`, then `DC <n>: <degree>`.
    let first = text.lines().next().unwrap();
    let (expr, rest) = first.split_once(": ").unwrap();
    assert_eq!(expr, "1d20+14", "{text}");
    let (sum, total) = rest.rsplit_once(" = ").unwrap();
    let parts: Vec<i64> = sum
        .split(" + ")
        .map(|p| p.trim().parse().unwrap())
        .collect();
    assert_eq!((parts.len(), parts[1]), (2, 14), "{text}");
    assert_eq!(
        total
            .split_whitespace()
            .next()
            .unwrap()
            .parse::<i64>()
            .unwrap(),
        parts[0] + 14
    );
    assert!(
        text.lines().nth(1).unwrap().starts_with("DC 25: "),
        "{text}"
    );
    assert!(
        ["Critical Success", "Success", "Failure", "Critical Failure"]
            .iter()
            .any(|d| text.contains(d)),
        "{text}"
    );
    let dmg = stdout(&run(&["roll", "2d6+3", "--seed", "7"]));
    assert!(dmg.contains("2d6+3") && !dmg.contains("DC"), "{dmg}");
    assert_eq!(run(&["roll", "2x6"]).status.code(), Some(1));
    assert_eq!(run(&["roll", "1d20", "--dc", "abc"]).status.code(), Some(2));
}

#[test]
fn exports_static_files() {
    let dir = temp_dir("static").join("site");
    let o = run(&["export-static", dir.to_str().unwrap()]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    for (name, _, contents) in obb_server::assets::ALL {
        assert_eq!(
            std::fs::read_to_string(dir.join(name)).unwrap(),
            contents,
            "{name}"
        );
    }
}

#[test]
fn serves_until_killed() {
    let dir = temp_dir("serve");
    let state = dir.join("room.json");
    let mut child = Command::new(BIN)
        .args([
            "serve",
            "--bind",
            "127.0.0.1:0",
            "--state-file",
            state.to_str().unwrap(),
            "--seed",
            "1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let url = line
        .split_whitespace()
        .find(|w| w.starts_with("http://") && w.ends_with("/manifest.json"))
        .unwrap_or_else(|| panic!("no install URL in {line:?}"));
    let addr = url
        .trim_start_matches("http://")
        .trim_end_matches("/manifest.json");
    let mut s = TcpStream::connect(addr).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    s.write_all(b"GET /manifest.json HTTP/1.1\r\nHost: x\r\n\r\n")
        .unwrap();
    let mut resp = String::new();
    s.read_to_string(&mut resp).unwrap();
    assert!(
        resp.starts_with("HTTP/1.1 200") && resp.contains("OwlbearBuilder"),
        "{resp}"
    );
    child.kill().unwrap();
    child.wait().unwrap();
    let bad = run(&["serve", "--bind", "not-an-address"]);
    assert_eq!(bad.status.code(), Some(1));
}
