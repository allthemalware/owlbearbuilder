// SPDX-License-Identifier: GPL-3.0-or-later
//! Creature stat blocks: parsed from text copied out of Archives of Nethys or a rulebook PDF,
//! or read from this crate's own JSON form.

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
    todo!()
}

impl MonsterStatBlock {
    /// Every field under its own name; `abilities` as `{"str": 3, ...}`, `skills` as
    /// `[{"name", "mod"}]`, strikes and abilities as objects of their fields (`actions`
    /// null when absent). Top level also carries `"format": "owlbearbuilder-monster"`,
    /// `"version": 1`.
    pub fn to_json(&self) -> JsonValue {
        todo!()
    }

    /// Reads `to_json` output. Required: `name`, `level`, `ac`, `max_hp`; everything else
    /// defaults to empty / 0 / `Medium`, and `id` to `slugify(name)`. Errors name the bad key.
    pub fn from_json(v: &JsonValue) -> Result<MonsterStatBlock, String> {
        todo!()
    }
}
