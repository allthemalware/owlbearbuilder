// SPDX-License-Identifier: GPL-3.0-or-later
//! Creature stat blocks: parsed from text copied out of Archives of Nethys or a rulebook PDF,
//! or read from this crate's own JSON form.

use crate::character::slugify;
use crate::json::JsonValue;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonsterStrike {
    /// As printed, e.g. `fist`, `shortbow`.
    pub name: String,
    pub is_ranged: bool,
    /// 1 unless the line says otherwise.
    pub actions: u8,
    /// First-Strike attack modifier; AoN's bracketed MAP values (`+7 [+3/-1]`) are ignored.
    pub attack_mod: i32,
    /// Lowercase, as printed: `agile`, `deadly d10`, `range increment 60 feet`, `reach 10 feet`.
    pub traits: Vec<String>,
    /// The first damage expression, e.g. `1d10+3`; empty when the Strike deals none.
    pub damage_expr: String,
    /// The first damage type, e.g. `slashing`.
    pub damage_type: String,
    /// Everything after the first damage, e.g. `plus Grab`, `plus 1d6 fire`; empty if none.
    pub damage_extra: String,
}

impl MonsterStrike {
    pub fn is_agile(&self) -> bool {
        self.traits.iter().any(|t| t == "agile")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonsterAbility {
    /// e.g. `Attack of Opportunity`, `Arcane Innate Spells`, `Sneak Attack`.
    pub name: String,
    /// `1`, `2`, `3`, `reaction` or `free`; `None` for passive abilities and spell lists.
    pub actions: Option<String>,
    /// Lowercase traits from a parenthesised list right after the name or action marker.
    pub traits: Vec<String>,
    /// The rest of the entry, whitespace-normalised.
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonsterStatBlock {
    pub id: String,
    pub name: String,
    pub level: i32,
    /// `Tiny`, `Small`, `Medium`, `Large`, `Huge` or `Gargantuan`.
    pub size: String,
    /// Every trait from the trait line except the size, lowercase, in order: rarity,
    /// alignment abbreviations from older books, creature types, `humanoid`, ...
    pub traits: Vec<String>,
    pub perception: i32,
    /// Text after the Perception modifier, e.g. `darkvision, scent (imprecise) 30 feet`.
    pub senses: String,
    pub languages: String,
    /// `(name, modifier)` in printed order, e.g. `("Athletics", 6)`. Notes such as
    /// `(+9 in forests)` are dropped.
    pub skills: Vec<(String, i32)>,
    /// Modifiers indexed like `Ability`: Str, Dex, Con, Int, Wis, Cha.
    pub abilities: [i32; 6],
    pub items: String,
    pub ac: i32,
    pub fort: i32,
    pub reflex: i32,
    pub will: i32,
    /// Anything after the saves, e.g. `+1 status to all saves vs. magic`.
    pub save_notes: String,
    pub max_hp: i32,
    /// Anything attached to HP, e.g. `void healing` or `regeneration 10 (deactivated by fire)`.
    pub hp_notes: String,
    /// e.g. `["disease", "poison"]`.
    pub immunities: Vec<String>,
    /// e.g. `["fire 5", "vitality 5"]`.
    pub weaknesses: Vec<String>,
    /// e.g. `["physical 3 (except silver)"]`.
    pub resistances: Vec<String>,
    /// Text after `Speed`, e.g. `25 feet, climb 15 feet`.
    pub speed: String,
    pub strikes: Vec<MonsterStrike>,
    pub spells_and_abilities: Vec<MonsterAbility>,
}

/// Parses a creature stat block as copied from Archives of Nethys or a PDF.
///
/// The header is the first line ending in `CREATURE <level>` (any case, level may be
/// negative); lines before it are ignored. The name is what precedes `CREATURE`, converted to
/// Title Case when it is all capitals.
/// The next non-empty line is the trait line. Source lines, `Recall Knowledge` lines and blank
/// lines are skipped anywhere. Recognised lines, by leading keyword: `Perception`,
/// `Languages`, `Skills`, `Str`, `Items`, `AC`, `HP`, `Speed`, `Melee`, `Ranged`.
/// - `AC 15; Fort +8, Ref +5, Will +4; <notes>` (also `Reflex`).
/// - `HP 15; Immunities a, b; Weaknesses c 5, d 5; Resistances e 3` (any of the three, each
///   comma-separated outside parentheses); text after the number up to the first `;`, or a
///   parenthesised note after the number, goes to `hp_notes`.
/// - Strikes: `Melee [one-action] fist +7 (agile, nonlethal), Damage 1d4+3 bludgeoning`.
///   The traits list is optional; `Effect ...` in place of `Damage ...` leaves the damage
///   fields empty and puts the effect text in `damage_extra`.
/// - Action markers accepted wherever actions appear: `[one-action]`, `[two-actions]`,
///   `[three-actions]`, `[reaction]`, `[free-action]` (AoN), `Single Action`, `Two Actions`,
///   `Three Actions`, `Reaction`, `Free Action` (AoN's text copy), and `◆`, `◆◆`, `◆◆◆`,
///   `⬲`, `◇`.
/// - Every other line after the trait line is an ability. A line containing an action marker
///   starts one named by the text before the marker. A line starting with
///   `<words> Spells` or `<words> Rituals` (e.g. `Arcane Innate Spells DC 17, attack +9; ...`)
///   starts one named up to and including that word. Any other line continues the previous
///   ability's text, or, when there is none yet, starts an ability whose name is the text up
///   to its first period and whose text is the rest.
///
/// Errors: no header line, or a missing or malformed `AC`, `HP` or `Perception` line.
/// `id` is `slugify(name)`.
pub fn parse_aon_statblock(text: &str) -> Result<MonsterStatBlock, String> {
    let lines: Vec<String> = text
        .lines()
        .map(normalize)
        .filter(|line| !line.is_empty())
        .collect();
    let header_index = lines
        .iter()
        .position(|line| {
            let upper = line.to_ascii_uppercase();
            upper.split_whitespace().any(|word| word == "CREATURE")
        })
        .ok_or_else(|| "missing creature header".to_string())?;
    let header = &lines[header_index];
    let upper = header.to_ascii_uppercase();
    let marker_start = upper
        .find("CREATURE ")
        .ok_or_else(|| "malformed creature header".to_string())?;
    let name = normalize(&header[..marker_start]);
    let level_text = header[marker_start + 9..]
        .split_whitespace()
        .next()
        .unwrap_or("");
    let level = signed_int(level_text).ok_or_else(|| "malformed creature level".to_string())?;
    let name = if name.chars().any(|c| c.is_alphabetic())
        && name
            .chars()
            .filter(|c| c.is_alphabetic())
            .all(char::is_uppercase)
    {
        title_case(&name)
    } else {
        name
    };
    let mut m = MonsterStatBlock {
        id: slugify(&name),
        name,
        level,
        size: "Medium".into(),
        traits: Vec::new(),
        perception: 0,
        senses: String::new(),
        languages: String::new(),
        skills: Vec::new(),
        abilities: [0; 6],
        items: String::new(),
        ac: 0,
        fort: 0,
        reflex: 0,
        will: 0,
        save_notes: String::new(),
        max_hp: 0,
        hp_notes: String::new(),
        immunities: Vec::new(),
        weaknesses: Vec::new(),
        resistances: Vec::new(),
        speed: String::new(),
        strikes: Vec::new(),
        spells_and_abilities: Vec::new(),
    };
    let mut got_perception = false;
    let mut got_ac = false;
    let mut got_hp = false;
    let mut previous_ability: Option<usize> = None;
    for (index, line) in lines.iter().enumerate().skip(header_index + 1) {
        if line.to_ascii_lowercase().starts_with("source ")
            || line.to_ascii_lowercase().starts_with("recall knowledge")
        {
            continue;
        }
        if index == header_index + 1 {
            parse_traits(line, &mut m);
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("perception ") {
            let rest = &line[11..];
            let (n, tail) =
                parse_signed_prefix(rest).ok_or_else(|| "malformed Perception line".to_string())?;
            m.perception = n;
            m.senses = trim_punctuation(tail);
            got_perception = true;
            previous_ability = None;
        } else if lower.starts_with("languages ") {
            m.languages = normalize(&line[10..]);
            previous_ability = None;
        } else if lower.starts_with("skills ") {
            m.skills = parse_skills(&line[7..]);
            previous_ability = None;
        } else if lower.starts_with("str ") || lower.starts_with("str,") {
            m.abilities = parse_abilities(line);
            previous_ability = None;
        } else if lower.starts_with("items ") {
            m.items = normalize(&line[6..]);
            previous_ability = None;
        } else if lower.starts_with("ac ") {
            parse_ac(line, &mut m)?;
            got_ac = true;
            previous_ability = None;
        } else if lower.starts_with("hp ") {
            parse_hp(line, &mut m)?;
            got_hp = true;
            previous_ability = None;
        } else if lower.starts_with("speed ") {
            m.speed = normalize(&line[6..]);
            previous_ability = None;
        } else if lower.starts_with("melee ") || lower.starts_with("ranged ") {
            m.strikes.push(parse_strike(line)?);
            previous_ability = None;
        } else if let Some((name, action, traits, rest)) = parse_action_ability(line) {
            m.spells_and_abilities.push(MonsterAbility {
                name,
                actions: Some(action),
                traits,
                text: rest,
            });
            previous_ability = Some(m.spells_and_abilities.len() - 1);
        } else if let Some(name) = spell_header(line) {
            let rest = normalize(&line[name.len()..]);
            m.spells_and_abilities.push(MonsterAbility {
                name,
                actions: None,
                traits: Vec::new(),
                text: rest,
            });
            previous_ability = Some(m.spells_and_abilities.len() - 1);
        } else if let Some(last) = previous_ability {
            // AoN wraps a single ability across lines; a markerless continuation is not a new entry.
            let old = &mut m.spells_and_abilities[last].text;
            if !old.is_empty() {
                old.push(' ');
            }
            old.push_str(line);
        } else {
            let (name, rest) = line
                .split_once('.')
                .map(|(a, b)| (normalize(a), normalize(b)))
                .unwrap_or_else(|| (line.clone(), String::new()));
            m.spells_and_abilities.push(MonsterAbility {
                name,
                actions: None,
                traits: Vec::new(),
                text: rest,
            });
            previous_ability = Some(m.spells_and_abilities.len() - 1);
        }
    }
    if !got_perception {
        return Err("missing or malformed Perception line".into());
    }
    if !got_ac {
        return Err("missing or malformed AC line".into());
    }
    if !got_hp {
        return Err("missing or malformed HP line".into());
    }
    Ok(m)
}

impl MonsterStatBlock {
    /// Every field under its own name; `abilities` as `{"str": 3, ...}`, `skills` as
    /// `[{"name", "mod"}]`, strikes and abilities as objects of their fields (`actions`
    /// null when absent). Top level also carries `"format": "owlbearbuilder-monster"`,
    /// `"version": 1`.
    pub fn to_json(&self) -> JsonValue {
        let mut root = JsonValue::object();
        root.insert("format", "owlbearbuilder-monster");
        root.insert("version", 1);
        for (key, value) in [
            ("id", self.id.as_str()),
            ("name", self.name.as_str()),
            ("size", self.size.as_str()),
            ("senses", self.senses.as_str()),
            ("languages", self.languages.as_str()),
            ("items", self.items.as_str()),
            ("save_notes", self.save_notes.as_str()),
            ("hp_notes", self.hp_notes.as_str()),
            ("speed", self.speed.as_str()),
        ] {
            root.insert(key, value);
        }
        for (key, value) in [
            ("level", self.level),
            ("perception", self.perception),
            ("ac", self.ac),
            ("fort", self.fort),
            ("reflex", self.reflex),
            ("will", self.will),
            ("max_hp", self.max_hp),
        ] {
            root.insert(key, value);
        }
        root.insert("traits", strings_json(&self.traits));
        root.insert("immunities", strings_json(&self.immunities));
        root.insert("weaknesses", strings_json(&self.weaknesses));
        root.insert("resistances", strings_json(&self.resistances));
        let mut abilities = JsonValue::object();
        for (key, value) in ["str", "dex", "con", "int", "wis", "cha"]
            .into_iter()
            .zip(self.abilities)
        {
            abilities.insert(key, value);
        }
        root.insert("abilities", abilities);
        root.insert(
            "skills",
            JsonValue::Array(
                self.skills
                    .iter()
                    .map(|(name, value)| {
                        object(&[
                            ("name", JsonValue::from(name.as_str())),
                            ("mod", JsonValue::from(*value)),
                        ])
                    })
                    .collect(),
            ),
        );
        root.insert(
            "strikes",
            JsonValue::Array(self.strikes.iter().map(strike_json).collect()),
        );
        root.insert(
            "spells_and_abilities",
            JsonValue::Array(self.spells_and_abilities.iter().map(ability_json).collect()),
        );
        root
    }

    /// Reads `to_json` output. Required: `name`, `level`, `ac`, `max_hp`; everything else
    /// defaults to empty / 0 / `Medium`, and `id` to `slugify(name)`. Errors name the bad key.
    pub fn from_json(v: &JsonValue) -> Result<MonsterStatBlock, String> {
        let name = required_str(v, "name")?;
        let mut m = MonsterStatBlock {
            id: opt_str(v, "id")?.unwrap_or_else(|| slugify(&name)),
            name,
            level: required_i32(v, "level")?,
            size: opt_str(v, "size")?.unwrap_or_else(|| "Medium".into()),
            traits: opt_strings(v, "traits")?,
            perception: opt_i32(v, "perception")?,
            senses: opt_str(v, "senses")?.unwrap_or_default(),
            languages: opt_str(v, "languages")?.unwrap_or_default(),
            skills: parse_skill_json(v)?,
            abilities: parse_ability_json(v)?,
            items: opt_str(v, "items")?.unwrap_or_default(),
            ac: required_i32(v, "ac")?,
            fort: opt_i32(v, "fort")?,
            reflex: opt_i32(v, "reflex")?,
            will: opt_i32(v, "will")?,
            save_notes: opt_str(v, "save_notes")?.unwrap_or_default(),
            max_hp: required_i32(v, "max_hp")?,
            hp_notes: opt_str(v, "hp_notes")?.unwrap_or_default(),
            immunities: opt_strings(v, "immunities")?,
            weaknesses: opt_strings(v, "weaknesses")?,
            resistances: opt_strings(v, "resistances")?,
            speed: opt_str(v, "speed")?.unwrap_or_default(),
            strikes: parse_strikes_json(v)?,
            spells_and_abilities: parse_abilities_json(v)?,
        };
        if m.id.is_empty() {
            m.id = slugify(&m.name);
        }
        Ok(m)
    }
}

fn normalize(s: &str) -> String {
    s.replace('\u{a0}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
fn signed_int(s: &str) -> Option<i32> {
    let s = s.trim();
    let digits = s.strip_prefix(['+', '-']).unwrap_or(s);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        None
    } else {
        s.parse().ok()
    }
}
fn parse_signed_prefix(s: &str) -> Option<(i32, &str)> {
    let s = s.trim_start();
    let end = s
        .char_indices()
        .skip_while(|(_, c)| *c == '+' || *c == '-')
        .find(|(_, c)| !c.is_ascii_digit())
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    let n = signed_int(&s[..end])?;
    Some((n, &s[end..]))
}
fn trim_punctuation(s: &str) -> String {
    normalize(s.trim().trim_start_matches(';'))
}
fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|first| {
                    first.to_uppercase().collect::<String>() + &c.as_str().to_ascii_lowercase()
                })
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}
fn parse_traits(line: &str, m: &mut MonsterStatBlock) {
    let sizes = ["tiny", "small", "medium", "large", "huge", "gargantuan"];
    let words: Vec<_> = line.split_whitespace().collect();
    if let Some(size) = words
        .iter()
        .find(|word| sizes.iter().any(|size| size.eq_ignore_ascii_case(word)))
    {
        m.size = title_case(&size.to_ascii_lowercase());
    }
    m.traits = words
        .into_iter()
        .filter(|word| !sizes.iter().any(|size| size.eq_ignore_ascii_case(word)))
        .map(str::to_ascii_lowercase)
        .collect();
}
fn split_commas(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                out.push(normalize(&s[start..i]));
                start = i + 1
            }
            _ => (),
        }
    }
    out.push(normalize(&s[start..]));
    out.into_iter().filter(|x| !x.is_empty()).collect()
}
fn split_first_outside_parens(s: &str, delimiter: char) -> Option<(&str, &str)> {
    let mut depth = 0;
    for (index, character) in s.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth -= 1,
            value if value == delimiter && depth == 0 => {
                return Some((&s[..index], &s[index + character.len_utf8()..]));
            }
            _ => {}
        }
    }
    None
}
fn parse_skills(s: &str) -> Vec<(String, i32)> {
    let mut out = Vec::new();
    for item in split_commas(s) {
        let item = item.split('(').next().unwrap_or(&item).trim();
        if let Some((name, tail)) = parse_signed_suffix(item) {
            out.push((normalize(name), tail));
        }
    }
    out
}
fn parse_signed_suffix(s: &str) -> Option<(&str, i32)> {
    let b = s.as_bytes();
    let mut i = b.len();
    while i > 0 && b[i - 1].is_ascii_digit() {
        i -= 1
    }
    if i == b.len() {
        return None;
    }
    let end = b.len();
    if i > 0 && matches!(b[i - 1], b'+' | b'-') {
        i -= 1
    }
    let value = signed_int(&s[i..end])?;
    Some((s[..i].trim(), value))
}
fn parse_abilities(line: &str) -> [i32; 6] {
    let mut out = [0; 6];
    for (i, key) in ["str", "dex", "con", "int", "wis", "cha"]
        .iter()
        .enumerate()
    {
        let lower = line.to_ascii_lowercase();
        if let Some(p) = lower.find(key) {
            if let Some((n, _)) = parse_signed_prefix(&line[p + key.len()..]) {
                out[i] = n;
            }
        }
    }
    out
}
fn parse_ac(line: &str, m: &mut MonsterStatBlock) -> Result<(), String> {
    let parts: Vec<_> = line[3..].split(';').map(normalize).collect();
    let (ac, _) = parse_signed_prefix(parts.first().map(String::as_str).unwrap_or(""))
        .ok_or("malformed AC line")?;
    m.ac = ac;
    let saves = parts.get(1).ok_or("malformed AC saves")?;
    for save in split_commas(saves) {
        let lower = save.to_ascii_lowercase();
        let slot = if lower.starts_with("fort") {
            Some(0)
        } else if lower.starts_with("reflex")
            || lower.starts_with("ref ")
            || lower.starts_with("ref+")
            || lower.starts_with("ref-")
        {
            Some(1)
        } else if lower.starts_with("will") {
            Some(2)
        } else {
            None
        };
        if let Some(slot) = slot {
            let label_end = match slot {
                0 | 2 => 4,
                1 if lower.starts_with("reflex") => 6,
                _ => 3,
            };
            if let Some((n, _)) = parse_signed_prefix(&save[label_end..]) {
                match slot {
                    0 => m.fort = n,
                    1 => m.reflex = n,
                    _ => m.will = n,
                }
            } else {
                return Err("malformed AC saves".into());
            }
        }
    }
    if parts.len() > 2 {
        m.save_notes = parts[2..].join("; ");
    }
    Ok(())
}
fn parse_hp(line: &str, m: &mut MonsterStatBlock) -> Result<(), String> {
    let rest = normalize(&line[3..]);
    let (hp, tail) = parse_signed_prefix(&rest).ok_or("malformed HP line")?;
    m.max_hp = hp;
    let tail = tail.trim();
    if tail.starts_with('(') {
        if let Some(end) = tail.find(')') {
            m.hp_notes = normalize(&tail[1..end]);
            if let Some((_, extra)) = tail[end + 1..].split_once(';') {
                parse_defense_notes(extra, m);
            }
        }
    } else if let Some((notes, extra)) = tail.split_once(';') {
        m.hp_notes = normalize(notes);
        parse_defense_notes(extra, m);
    } else {
        m.hp_notes = normalize(tail);
    }
    Ok(())
}
fn parse_defense_notes(s: &str, m: &mut MonsterStatBlock) {
    for part in s.split(';') {
        let lower = part.to_ascii_lowercase();
        for (key, target) in [
            ("immunities", &mut m.immunities),
            ("weaknesses", &mut m.weaknesses),
            ("resistances", &mut m.resistances),
        ] {
            if let Some(p) = lower.find(key) {
                *target = split_commas(&part[p + key.len()..]);
            }
        }
    }
}
fn parse_strike(line: &str) -> Result<MonsterStrike, String> {
    let ranged = line.to_ascii_lowercase().starts_with("ranged ");
    let body = normalize(&line[6..]);
    let (actions, body) = take_action(&body).ok_or("strike missing action marker")?;
    let (head, damage) = split_first_outside_parens(&body, ',').unwrap_or((&body, ""));
    let (before, traits) = if let Some(open) = head.find('(') {
        (
            head[..open].trim(),
            head[open + 1..].trim_end_matches(')').to_string(),
        )
    } else {
        (head.trim(), String::new())
    };
    // MAP values in brackets describe later attacks, so the printed first modifier is the only one parsed.
    let before = before.split('[').next().unwrap_or(before).trim();
    let (name, modtail) = parse_signed_suffix(before)
        .ok_or_else(|| format!("strike missing attack modifier in {before:?}"))?;
    let attack_mod = modtail;
    let mut damage_expr = String::new();
    let mut damage_type = String::new();
    let mut damage_extra = String::new();
    let damage = damage.trim();
    if let Some(value) = damage.strip_prefix("Damage ") {
        let mut words = value.split_whitespace();
        damage_expr = words.next().unwrap_or("").to_string();
        damage_type = words.next().unwrap_or("").to_string();
        let rest = words.collect::<Vec<_>>().join(" ");
        if !rest.is_empty() {
            damage_extra = if rest.to_ascii_lowercase().starts_with("plus ") {
                rest
            } else {
                format!("plus {rest}")
            };
        }
    } else if let Some(value) = damage.strip_prefix("Effect ") {
        damage_extra = normalize(value);
    }
    Ok(MonsterStrike {
        name: normalize(name),
        is_ranged: ranged,
        actions,
        attack_mod,
        traits: split_commas(&traits)
            .into_iter()
            .map(|x| x.to_ascii_lowercase())
            .collect(),
        damage_expr,
        damage_type,
        damage_extra,
    })
}
fn take_action(s: &str) -> Option<(u8, String)> {
    let (idx, len, actions) = find_action(s)?;
    Some((actions, normalize(&s[idx + len..])))
}
fn find_action(s: &str) -> Option<(usize, usize, u8)> {
    let forms = [
        ("[one-action]", 1),
        ("[two-actions]", 2),
        ("[three-actions]", 3),
        ("[reaction]", 0),
        ("[free-action]", 0),
        ("Single Action", 1),
        ("Two Actions", 2),
        ("Three Actions", 3),
        ("Reaction", 0),
        ("Free Action", 0),
        ("◆◆◆", 3),
        ("◆◆", 2),
        ("◆", 1),
        ("⬲", 0),
        ("◇", 0),
    ];
    forms
        .iter()
        .filter_map(|(form, n)| s.find(form).map(|i| (i, form.len(), *n)))
        .min_by_key(|(i, _, _)| *i)
}
fn parse_action_ability(line: &str) -> Option<(String, String, Vec<String>, String)> {
    let (idx, len, count) = find_action(line)?;
    let prefix = normalize(&line[..idx]);
    let action = match count {
        1 => "1",
        2 => "2",
        3 => "3",
        _ => {
            if line[idx..].starts_with("◇")
                || line[idx..].to_ascii_lowercase().starts_with("free")
                || line[idx..].to_ascii_lowercase().starts_with("[free")
            {
                "free"
            } else {
                "reaction"
            }
        }
    }
    .to_string();
    let name = prefix;
    let mut rest = normalize(&line[idx + len..]);
    let traits = if rest.starts_with('(') {
        if let Some(end) = rest.find(')') {
            let traits = split_commas(&rest[1..end])
                .into_iter()
                .map(|x| x.to_ascii_lowercase())
                .collect();
            rest = normalize(&rest[end + 1..]);
            traits
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    Some((name, action, traits, rest))
}
fn spell_header(line: &str) -> Option<String> {
    let words: Vec<_> = line.split_whitespace().collect();
    for (i, w) in words.iter().enumerate() {
        if w.eq_ignore_ascii_case("spells") || w.eq_ignore_ascii_case("rituals") {
            return Some(words[..=i].join(" "));
        }
    }
    None
}
fn strings_json(values: &[String]) -> JsonValue {
    JsonValue::Array(values.iter().cloned().map(JsonValue::from).collect())
}
fn object(values: &[(&str, JsonValue)]) -> JsonValue {
    let mut o = JsonValue::object();
    for (k, v) in values {
        o.insert(*k, v.clone());
    }
    o
}
fn strike_json(s: &MonsterStrike) -> JsonValue {
    object(&[
        ("name", s.name.clone().into()),
        ("is_ranged", s.is_ranged.into()),
        ("actions", i32::from(s.actions).into()),
        ("attack_mod", s.attack_mod.into()),
        ("traits", strings_json(&s.traits)),
        ("damage_expr", s.damage_expr.clone().into()),
        ("damage_type", s.damage_type.clone().into()),
        ("damage_extra", s.damage_extra.clone().into()),
    ])
}
fn ability_json(a: &MonsterAbility) -> JsonValue {
    object(&[
        ("name", a.name.clone().into()),
        (
            "actions",
            a.actions
                .clone()
                .map(JsonValue::from)
                .unwrap_or(JsonValue::Null),
        ),
        ("traits", strings_json(&a.traits)),
        ("text", a.text.clone().into()),
    ])
}
fn required_str(v: &JsonValue, key: &str) -> Result<String, String> {
    v.get_str(key)
        .map(str::to_string)
        .ok_or_else(|| format!("invalid or missing {key}"))
}
fn opt_str(v: &JsonValue, key: &str) -> Result<Option<String>, String> {
    match v.get(key) {
        None => Ok(None),
        Some(JsonValue::Str(s)) => Ok(Some(s.clone())),
        _ => Err(format!("invalid {key}")),
    }
}
fn read_i32(value: Option<&JsonValue>, key: &str, required: bool) -> Result<i32, String> {
    match value {
        None if !required => Ok(0),
        Some(v) => v
            .as_i64()
            .and_then(|n| i32::try_from(n).ok())
            .ok_or_else(|| format!("invalid {key}")),
        None => Err(format!("missing {key}")),
    }
}
fn required_i32(v: &JsonValue, key: &str) -> Result<i32, String> {
    read_i32(v.get(key), key, true)
}
fn opt_i32(v: &JsonValue, key: &str) -> Result<i32, String> {
    read_i32(v.get(key), key, false)
}
fn opt_strings(v: &JsonValue, key: &str) -> Result<Vec<String>, String> {
    match v.get(key) {
        None => Ok(Vec::new()),
        Some(JsonValue::Array(a)) => a
            .iter()
            .map(|x| {
                x.as_str()
                    .map(str::to_string)
                    .ok_or_else(|| format!("invalid {key}"))
            })
            .collect(),
        _ => Err(format!("invalid {key}")),
    }
}
fn parse_skill_json(v: &JsonValue) -> Result<Vec<(String, i32)>, String> {
    match v.get("skills") {
        None => Ok(Vec::new()),
        Some(JsonValue::Array(a)) => a
            .iter()
            .map(|x| Ok((required_str(x, "name")?, required_i32(x, "mod")?)))
            .collect(),
        _ => Err("invalid skills".into()),
    }
}
fn parse_ability_json(v: &JsonValue) -> Result<[i32; 6], String> {
    let mut out = [0; 6];
    if let Some(value) = v.get("abilities") {
        for (i, key) in ["str", "dex", "con", "int", "wis", "cha"]
            .iter()
            .enumerate()
        {
            out[i] = read_i32(value.get(key), key, false)?;
        }
    }
    Ok(out)
}
fn parse_strikes_json(v: &JsonValue) -> Result<Vec<MonsterStrike>, String> {
    match v.get("strikes") {
        None => Ok(Vec::new()),
        Some(JsonValue::Array(a)) => a
            .iter()
            .map(|x| {
                Ok(MonsterStrike {
                    name: required_str(x, "name")?,
                    is_ranged: x
                        .get("is_ranged")
                        .and_then(JsonValue::as_bool)
                        .ok_or("invalid is_ranged")?,
                    actions: u8::try_from(required_i32(x, "actions")?)
                        .map_err(|_| "invalid actions")?,
                    attack_mod: required_i32(x, "attack_mod")?,
                    traits: opt_strings(x, "traits")?,
                    damage_expr: opt_str(x, "damage_expr")?.unwrap_or_default(),
                    damage_type: opt_str(x, "damage_type")?.unwrap_or_default(),
                    damage_extra: opt_str(x, "damage_extra")?.unwrap_or_default(),
                })
            })
            .collect(),
        _ => Err("invalid strikes".into()),
    }
}
fn parse_abilities_json(v: &JsonValue) -> Result<Vec<MonsterAbility>, String> {
    match v.get("spells_and_abilities") {
        None => Ok(Vec::new()),
        Some(JsonValue::Array(a)) => a
            .iter()
            .map(|x| {
                let actions = match x.get("actions") {
                    None | Some(JsonValue::Null) => None,
                    Some(JsonValue::Str(s)) => Some(s.clone()),
                    _ => return Err("invalid actions".into()),
                };
                Ok(MonsterAbility {
                    name: required_str(x, "name")?,
                    actions,
                    traits: opt_strings(x, "traits")?,
                    text: opt_str(x, "text")?.unwrap_or_default(),
                })
            })
            .collect(),
        _ => Err("invalid spells_and_abilities".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(extra: &str) -> String {
        format!(
            "Test Creature 1\nMedium Humanoid\nPerception +3; vision\nSkills Warfare Lore +9\nStr +0, Dex +0, Con +0, Int +0, Wis +0, Cha +0\nAC 15; Fort +5, Ref +4, Will +3\nHP 10\n{extra}\n"
        )
    }

    #[test]
    fn accepts_crlf_nonbreaking_spaces_lore_and_ref_abbreviation() {
        let input = block("Speed 25\u{a0}feet").replace('\n', "\r\n");
        let monster = parse_aon_statblock(&input).unwrap();
        assert_eq!(monster.speed, "25 feet");
        assert_eq!(monster.skills, [("Warfare Lore".into(), 9)]);
        assert_eq!((monster.reflex, monster.will), (4, 3));
        let abbreviated = block("Speed 25").replace("Ref +4", "Ref +8");
        assert_eq!(parse_aon_statblock(&abbreviated).unwrap().reflex, 8);
    }

    #[test]
    fn action_markers_map_for_strikes_and_abilities() {
        let markers = [
            ("[one-action]", 1, "1"),
            ("[two-actions]", 2, "2"),
            ("[three-actions]", 3, "3"),
            ("[reaction]", 0, "reaction"),
            ("[free-action]", 0, "free"),
            ("Single Action", 1, "1"),
            ("Two Actions", 2, "2"),
            ("Three Actions", 3, "3"),
            ("Reaction", 0, "reaction"),
            ("Free Action", 0, "free"),
            ("◆", 1, "1"),
            ("◆◆", 2, "2"),
            ("◆◆◆", 3, "3"),
            ("⬲", 0, "reaction"),
            ("◇", 0, "free"),
        ];
        for (marker, strike_actions, ability_actions) in markers {
            let input = block(&format!(
                "Melee {marker} fist +7, Damage 1d4 bludgeoning\nQuick Step {marker} (agile) Move quickly."
            ));
            let monster = parse_aon_statblock(&input).unwrap();
            assert_eq!(monster.strikes[0].actions, strike_actions, "{marker}");
            assert_eq!(
                monster.spells_and_abilities[0].actions.as_deref(),
                Some(ability_actions),
                "{marker}"
            );
        }
    }

    #[test]
    fn parses_negative_strike_modifier() {
        let monster =
            parse_aon_statblock(&block("Melee [one-action] claw -2, Damage 1d4 slashing")).unwrap();
        assert_eq!(monster.strikes[0].attack_mod, -2);
    }
}
