// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;

const USAGE: &str = "Usage: owlbearbuilder <command>\nCommands:\n  serve [--bind ADDR] [--state-file FILE] [--seed N]\n  inspect <file>\n  parse-monster <file> [--elite|--weak] [--json]\n  roll <expr> [--dc N] [--seed N]\n  export-static <dir>";

#[derive(Debug, PartialEq, Eq)]
enum Parsed {
    Serve {
        bind: String,
        state: Option<PathBuf>,
        seed: Option<u64>,
    },
    Inspect(PathBuf),
    Monster {
        path: PathBuf,
        variant: obb_core::bestiary::CreatureVariant,
        json: bool,
    },
    Roll {
        expr: String,
        dc: Option<i32>,
        seed: Option<u64>,
    },
    Export(PathBuf),
    Help,
    Version,
}

fn number<T: std::str::FromStr>(s: &str) -> Result<T, String> {
    s.parse().map_err(|_| format!("invalid argument: {s}"))
}

type OptionValues = Vec<(String, String)>;

fn options(
    args: &[String],
    valued: &[&str],
    flags: &[&str],
) -> Result<(Vec<String>, OptionValues), String> {
    let mut positional = Vec::new();
    let mut values = Vec::new();
    let mut seen = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if let Some((key, value)) = arg.split_once('=') {
            if !valued.contains(&key) || seen.iter().any(|s| s == key) || value.is_empty() {
                return Err(format!("invalid or repeated option: {key}"));
            }
            seen.push(key.to_string());
            values.push((key.to_string(), value.to_string()));
        } else if valued.contains(&arg.as_str()) {
            if seen.iter().any(|s| s == arg) || i + 1 >= args.len() || args[i + 1].starts_with("--")
            {
                return Err(format!("missing or repeated value for {arg}"));
            }
            seen.push(arg.clone());
            i += 1;
            values.push((arg.clone(), args[i].clone()));
        } else if flags.contains(&arg.as_str()) {
            if seen.iter().any(|s| s == arg) {
                return Err(format!("repeated option: {arg}"));
            }
            seen.push(arg.clone());
            values.push((arg.clone(), String::new()));
        } else if arg.starts_with('-') {
            return Err(format!("unknown option: {arg}"));
        } else {
            positional.push(arg.clone());
        }
        i += 1;
    }
    Ok((positional, values))
}

fn parse(args: &[String]) -> Result<Parsed, String> {
    let Some((cmd, rest)) = args.split_first() else {
        return Ok(Parsed::Help);
    };
    match cmd.as_str() {
        "help" | "--help" => Ok(Parsed::Help),
        "--version" => Ok(Parsed::Version),
        "serve" => {
            let (p, v) = options(rest, &["--bind", "--state-file", "--seed"], &[])?;
            if !p.is_empty() {
                return Err("unexpected serve argument".into());
            }
            let get = |key: &str| v.iter().find(|(k, _)| k == key).map(|(_, x)| x.clone());
            Ok(Parsed::Serve {
                bind: get("--bind").unwrap_or_else(|| "127.0.0.1:8080".into()),
                state: get("--state-file").map(PathBuf::from),
                seed: get("--seed").map(|s| number(&s)).transpose()?,
            })
        }
        "inspect" | "export-static" => {
            if rest.len() != 1 {
                return Err(format!("{cmd} requires one path"));
            }
            let path = PathBuf::from(&rest[0]);
            if cmd == "inspect" {
                Ok(Parsed::Inspect(path))
            } else {
                Ok(Parsed::Export(path))
            }
        }
        "parse-monster" => {
            let (p, v) = options(rest, &[], &["--elite", "--weak", "--json"])?;
            if p.len() != 1 {
                return Err("parse-monster requires one file".into());
            }
            let elite = v.iter().any(|(k, _)| k == "--elite");
            let weak = v.iter().any(|(k, _)| k == "--weak");
            if elite && weak {
                return Err("--elite and --weak cannot be combined".into());
            }
            let variant = if elite {
                obb_core::bestiary::CreatureVariant::Elite
            } else if weak {
                obb_core::bestiary::CreatureVariant::Weak
            } else {
                obb_core::bestiary::CreatureVariant::Normal
            };
            Ok(Parsed::Monster {
                path: PathBuf::from(&p[0]),
                variant,
                json: v.iter().any(|(k, _)| k == "--json"),
            })
        }
        "roll" => {
            let (p, v) = options(rest, &["--dc", "--seed"], &[])?;
            if p.len() != 1 {
                return Err("roll requires one expression".into());
            }
            let dc = v
                .iter()
                .find(|(k, _)| k == "--dc")
                .map(|(_, x)| number(x))
                .transpose()?;
            let seed = v
                .iter()
                .find(|(k, _)| k == "--seed")
                .map(|(_, x)| number(x))
                .transpose()?;
            Ok(Parsed::Roll {
                expr: p[0].clone(),
                dc,
                seed,
            })
        }
        _ => Err(format!("unknown command: {cmd}")),
    }
}

fn failure(message: impl std::fmt::Display, code: i32) -> i32 {
    eprintln!("{message}");
    if code == 2 {
        eprintln!("{USAGE}");
    }
    code
}

pub fn run(args: Vec<String>) -> i32 {
    let command = match parse(&args) {
        Ok(x) => x,
        Err(e) => return failure(e, 2),
    };
    let result: Result<(), String> = match command {
        Parsed::Help => {
            println!("{USAGE}");
            return 0;
        }
        Parsed::Version => {
            println!("owlbearbuilder {}", env!("CARGO_PKG_VERSION"));
            return 0;
        }
        Parsed::Serve { bind, state, seed } => match obb_server::start(&bind, state, seed) {
            Ok(server) => {
                println!("Install URL: http://{}/manifest.json", server.addr);
                println!("Players don't need access: only the GM's browser talks to this address.");
                use std::io::Write;
                let _ = std::io::stdout().flush();
                loop {
                    std::thread::park();
                }
            }
            Err(e) => Err(e),
        },
        Parsed::Inspect(path) => inspect(&path),
        Parsed::Monster {
            path,
            variant,
            json,
        } => monster(&path, variant, json),
        Parsed::Roll { expr, dc, seed } => roll(&expr, dc, seed),
        Parsed::Export(dir) => export(&dir),
    };
    match result {
        Ok(()) => 0,
        Err(e) => failure(e, 1),
    }
}

fn inspect(path: &std::path::Path) -> Result<(), String> {
    use obb_core::{character::Save, import::import_character_auto, spells::spellbook};
    let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let c = import_character_auto(&raw)?;
    println!(
        "{} — {}, {}\nLevel {}\nHP {}\nAC {}\nPerception {:+}",
        c.name,
        c.ancestry,
        c.class,
        c.level,
        c.max_hp,
        c.effective_ac(),
        c.perception_modifier()
    );
    for save in Save::ALL {
        println!("{} {:+}", save.label(), c.save_modifier(save));
    }
    println!("Class DC {}", c.class_dc());
    for (i, s) in c.strikes.iter().enumerate() {
        let a = c.strike_attack_modifier(i, 0).unwrap_or(s.attack_bonus);
        let b = c.strike_attack_modifier(i, 1).unwrap_or(a);
        let d = c.strike_attack_modifier(i, 2).unwrap_or(a);
        println!(
            "{} {a:+} / {b:+} / {d:+} — {}d{}{:+} {}",
            s.name, s.damage.dice_count, s.damage.die_sides, s.damage.flat_bonus, s.damage_type
        );
    }
    for skill in obb_core::character::Skill::ALL {
        if let Some(rank) = c.skill_rank(skill.label()) {
            if rank != obb_core::character::ProfRank::Untrained {
                println!(
                    "{} {:+} ({})",
                    skill.label(),
                    c.skill_modifier(skill.id()).unwrap_or(0),
                    rank.label()
                );
            }
        }
    }
    for lore in &c.lores {
        println!(
            "{} Lore {:+} ({})",
            lore.name,
            c.skill_modifier(&lore.name).unwrap_or(0),
            lore.rank.label()
        );
    }
    let spells = spellbook(&c);
    for (index, entry) in c.spellcasting.iter().enumerate() {
        println!(
            "{} ({} {}, DC {}, attack {:+})",
            entry.name,
            entry.tradition,
            entry.kind,
            c.spell_dc(index).unwrap_or(10),
            c.spell_attack_modifier(index).unwrap_or(0)
        );
        for known in spells.iter().filter(|known| known.entry == index) {
            if let Some(detail) = &known.details {
                let area = detail
                    .area
                    .as_ref()
                    .map(|a| format!(" — {}", a.label()))
                    .unwrap_or_default();
                let damage = detail
                    .damage_expr
                    .as_ref()
                    .map(|d| {
                        format!(
                            " — {d} {}",
                            detail.damage_type.as_deref().unwrap_or_default()
                        )
                    })
                    .unwrap_or_default();
                println!("  {}{}{} (rank {})", known.name, area, damage, detail.rank);
            } else {
                println!("  {}", known.name);
            }
        }
    }
    Ok(())
}

fn monster(
    path: &std::path::Path,
    variant: obb_core::bestiary::CreatureVariant,
    json: bool,
) -> Result<(), String> {
    let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let base = obb_core::monster::parse_aon_statblock(&raw)?;
    let mut m = obb_core::bestiary::apply_variant(&base, variant);
    if json {
        println!("{}", m.to_json().to_json_pretty());
        return Ok(());
    }
    if variant != obb_core::bestiary::CreatureVariant::Normal {
        m.name = format!("{} {}", variant.label(), m.name);
    }
    println!(
        "{}\nLevel {}\nAC {}\nFort {:+}, Ref {:+}, Will {:+}\nHP {}",
        m.name, m.level, m.ac, m.fort, m.reflex, m.will, m.max_hp
    );
    for s in m.strikes {
        println!(
            "{} {:+} — {} {}",
            s.name, s.attack_mod, s.damage_expr, s.damage_type
        );
    }
    Ok(())
}

fn roll(expr: &str, dc: Option<i32>, seed: Option<u64>) -> Result<(), String> {
    let mut rng = seed.map_or_else(obb_core::dice::Rng::from_os, obb_core::dice::Rng::from_seed);
    let r = obb_core::dice::roll_expr(expr, &mut rng)?;
    let mut pieces = Vec::new();
    for t in &r.terms {
        for die in &t.rolls {
            pieces.push((i32::try_from(*die).unwrap_or(i32::MAX) * t.sign).to_string());
        }
    }
    if r.flat_modifier != 0 || pieces.is_empty() {
        pieces.push(r.flat_modifier.to_string());
    }
    let mut line = format!("{}: {} = {}", r.expr, pieces.join(" + "), r.total);
    if let Some(natural) = r.natural_d20 {
        if natural == 20 {
            line.push_str(" (natural 20)");
        }
        if natural == 1 {
            line.push_str(" (natural 1)");
        }
    }
    println!("{line}");
    if let Some(dc) = dc {
        println!(
            "DC {dc}: {}",
            obb_core::dice::evaluate_check(r.total, dc, r.natural_d20).label()
        );
    }
    Ok(())
}

fn export(dir: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    for (name, _, contents) in obb_server::assets::ALL {
        let path = dir.join(name);
        std::fs::write(&path, contents).map_err(|e| e.to_string())?;
        println!("{}", path.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn a(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| x.to_string()).collect()
    }
    #[test]
    fn accepts_flags_in_any_order_and_equals() {
        assert!(parse(&a(&["serve", "--seed=4", "--bind", "127.0.0.1:9"])).is_ok());
        assert!(parse(&a(&["roll", "1d20", "--seed", "4", "--dc=12"])).is_ok());
        assert!(parse(&a(&["parse-monster", "x", "--json", "--elite"])).is_ok());
    }
    #[test]
    fn rejects_repeated_flags() {
        assert!(parse(&a(&["serve", "--seed", "1", "--seed=2"])).is_err());
        assert!(parse(&a(&["parse-monster", "x", "--json", "--json"])).is_err());
    }
}
