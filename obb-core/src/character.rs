// SPDX-License-Identifier: GPL-3.0-or-later
//! The canonical character sheet every importer produces, and the modifiers it yields once
//! conditions are applied. Rules references are to Pathfinder Player Core (Remaster).

use std::collections::BTreeMap;

use crate::conditions::{self, Condition};
use crate::dice::StrikeDamage;
use crate::json::JsonValue;

fn resolve_skill(c: &CharacterSheet, name: &str) -> Option<(Ability, ProfRank, String)> {
    if let Some(skill) = Skill::parse(name) {
        return Some((
            skill.ability(),
            c.skill_ranks[skill as usize],
            skill.id().into(),
        ));
    }
    let subject = name
        .trim()
        .to_ascii_lowercase()
        .strip_suffix(" lore")
        .unwrap_or(&name.trim().to_ascii_lowercase())
        .to_string();
    let lore = c
        .lores
        .iter()
        .find(|l| l.name.trim().eq_ignore_ascii_case(&subject))?;
    Some((
        Ability::Int,
        lore.rank,
        format!("{} lore", lore.name.to_lowercase()),
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Ability {
    Str,
    Dex,
    Con,
    Int,
    Wis,
    Cha,
}

impl Ability {
    pub const ALL: [Ability; 6] = [
        Ability::Str,
        Ability::Dex,
        Ability::Con,
        Ability::Int,
        Ability::Wis,
        Ability::Cha,
    ];

    /// `str`, `dex`, `con`, `int`, `wis`, `cha`.
    pub fn id(self) -> &'static str {
        ["str", "dex", "con", "int", "wis", "cha"][self as usize]
    }

    /// Accepts the id or the full name (`strength`), case-insensitive.
    pub fn parse(s: &str) -> Option<Ability> {
        match s.trim().to_ascii_lowercase().as_str() {
            "str" | "strength" => Some(Ability::Str),
            "dex" | "dexterity" => Some(Ability::Dex),
            "con" | "constitution" => Some(Ability::Con),
            "int" | "intelligence" => Some(Ability::Int),
            "wis" | "wisdom" => Some(Ability::Wis),
            "cha" | "charisma" => Some(Ability::Cha),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProfRank {
    Untrained,
    Trained,
    Expert,
    Master,
    Legendary,
}

impl ProfRank {
    /// 0, 2, 4, 6, 8.
    pub fn bonus(self) -> i32 {
        (self as i32) * 2
    }

    /// From the rank bonus builders export (0/2/4/6/8). Odd values round down to a rank;
    /// anything below 0 is Untrained and anything above 8 is Legendary.
    pub fn from_bonus(bonus: i64) -> ProfRank {
        match bonus {
            i64::MIN..=1 => ProfRank::Untrained,
            2..=3 => ProfRank::Trained,
            4..=5 => ProfRank::Expert,
            6..=7 => ProfRank::Master,
            _ => ProfRank::Legendary,
        }
    }

    /// `untrained`, `trained`, `expert`, `master`, `legendary`.
    pub fn id(self) -> &'static str {
        ["untrained", "trained", "expert", "master", "legendary"][self as usize]
    }

    /// `Untrained`, `Trained`, ...
    pub fn label(self) -> &'static str {
        ["Untrained", "Trained", "Expert", "Master", "Legendary"][self as usize]
    }

    /// `U`, `T`, `E`, `M`, `L`.
    pub fn letter(self) -> char {
        ['U', 'T', 'E', 'M', 'L'][self as usize]
    }

    /// Accepts the id or label, case-insensitive.
    pub fn parse(s: &str) -> Option<ProfRank> {
        [
            ProfRank::Untrained,
            ProfRank::Trained,
            ProfRank::Expert,
            ProfRank::Master,
            ProfRank::Legendary,
        ]
        .into_iter()
        .find(|r| s.trim().eq_ignore_ascii_case(r.id()) || s.trim().eq_ignore_ascii_case(r.label()))
    }
}

/// Proficiency bonus: the rank's bonus plus level once trained, 0 when untrained.
pub fn proficiency_bonus(rank: ProfRank, level: i32) -> i32 {
    if rank == ProfRank::Untrained {
        0
    } else {
        rank.bonus().saturating_add(level)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Skill {
    Acrobatics,
    Arcana,
    Athletics,
    Crafting,
    Deception,
    Diplomacy,
    Intimidation,
    Medicine,
    Nature,
    Occultism,
    Performance,
    Religion,
    Society,
    Stealth,
    Survival,
    Thievery,
}

impl Skill {
    pub const ALL: [Skill; 16] = [
        Skill::Acrobatics,
        Skill::Arcana,
        Skill::Athletics,
        Skill::Crafting,
        Skill::Deception,
        Skill::Diplomacy,
        Skill::Intimidation,
        Skill::Medicine,
        Skill::Nature,
        Skill::Occultism,
        Skill::Performance,
        Skill::Religion,
        Skill::Society,
        Skill::Stealth,
        Skill::Survival,
        Skill::Thievery,
    ];

    /// Lowercase name, e.g. `athletics`.
    pub fn id(self) -> &'static str {
        [
            "acrobatics",
            "arcana",
            "athletics",
            "crafting",
            "deception",
            "diplomacy",
            "intimidation",
            "medicine",
            "nature",
            "occultism",
            "performance",
            "religion",
            "society",
            "stealth",
            "survival",
            "thievery",
        ][self as usize]
    }

    /// Capitalised name, e.g. `Athletics`.
    pub fn label(self) -> &'static str {
        [
            "Acrobatics",
            "Arcana",
            "Athletics",
            "Crafting",
            "Deception",
            "Diplomacy",
            "Intimidation",
            "Medicine",
            "Nature",
            "Occultism",
            "Performance",
            "Religion",
            "Society",
            "Stealth",
            "Survival",
            "Thievery",
        ][self as usize]
    }

    /// Acrobatics, Stealth, Thievery: Dex. Athletics: Str. Arcana, Crafting, Occultism,
    /// Society: Int. Medicine, Nature, Religion, Survival: Wis. Deception, Diplomacy,
    /// Intimidation, Performance: Cha.
    pub fn ability(self) -> Ability {
        match self {
            Skill::Acrobatics | Skill::Stealth | Skill::Thievery => Ability::Dex,
            Skill::Athletics => Ability::Str,
            Skill::Arcana | Skill::Crafting | Skill::Occultism | Skill::Society => Ability::Int,
            Skill::Medicine | Skill::Nature | Skill::Religion | Skill::Survival => Ability::Wis,
            _ => Ability::Cha,
        }
    }

    /// Case-insensitive name.
    pub fn parse(s: &str) -> Option<Skill> {
        Skill::ALL
            .into_iter()
            .find(|x| s.trim().eq_ignore_ascii_case(x.id()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Save {
    Fortitude,
    Reflex,
    Will,
}

impl Save {
    pub const ALL: [Save; 3] = [Save::Fortitude, Save::Reflex, Save::Will];

    /// `fortitude`, `reflex`, `will`.
    pub fn id(self) -> &'static str {
        ["fortitude", "reflex", "will"][self as usize]
    }

    /// `Fortitude`, `Reflex`, `Will`.
    pub fn label(self) -> &'static str {
        ["Fortitude", "Reflex", "Will"][self as usize]
    }

    /// Con, Dex, Wis.
    pub fn ability(self) -> Ability {
        match self {
            Save::Fortitude => Ability::Con,
            Save::Reflex => Ability::Dex,
            Save::Will => Ability::Wis,
        }
    }

    /// Case-insensitive id, label, or short form (`fort`, `ref`).
    pub fn parse(s: &str) -> Option<Save> {
        match s.trim().to_ascii_lowercase().as_str() {
            "fortitude" | "fort" => Some(Save::Fortitude),
            "reflex" | "ref" => Some(Save::Reflex),
            "will" => Some(Save::Will),
            _ => None,
        }
    }
}

/// A Lore skill. `name` is the subject without the word "Lore", e.g. `Warfare`. Lore uses Int.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lore {
    pub name: String,
    pub rank: ProfRank,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeaponStrike {
    /// As shown at the table, e.g. `+1 Striking Longsword`.
    pub name: String,
    /// Attack modifier for a first Strike with no conditions: proficiency, ability, item bonus.
    pub attack_bonus: i32,
    /// The ability behind the attack roll; decides whether Clumsy or Enfeebled applies.
    pub attack_ability: Ability,
    pub is_ranged: bool,
    /// Weapon dice and flat damage with no conditions applied.
    pub damage: StrikeDamage,
    /// `slashing`, `piercing`, `bludgeoning`, or the builder's text.
    pub damage_type: String,
    /// Extra damage shown with the Strike, e.g. `1d6 fire`. Not rolled into totals.
    pub extra_damage: Vec<String>,
    /// Lowercase traits, e.g. `agile`, `finesse`, `deadly d8`, `thrown 20 ft`.
    pub traits: Vec<String>,
}

impl WeaponStrike {
    pub fn is_agile(&self) -> bool {
        self.traits.iter().any(|t| t.eq_ignore_ascii_case("agile"))
    }

    /// Strength is added to damage for melee Strikes and thrown ranged ones, and Enfeebled
    /// penalises Strength-based damage, so this decides whether it applies to damage.
    pub fn adds_strength_to_damage(&self) -> bool {
        !self.is_ranged || self.traits.iter().any(|t| t.starts_with("thrown"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellcastingEntry {
    /// e.g. `Wizard`, `Cleric Font`, `Focus Spells`.
    pub name: String,
    /// `arcane`, `divine`, `occult` or `primal`.
    pub tradition: String,
    /// `prepared`, `spontaneous`, `innate` or `focus`.
    pub kind: String,
    pub ability: Ability,
    pub rank: ProfRank,
    pub focus_points: u8,
    /// `(spell rank, names)`, rank 0 for cantrips, in ascending rank order.
    pub spells: Vec<(u8, Vec<String>)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterSheet {
    /// Stable within a room: `slugify(name)` unless the room had to disambiguate.
    pub id: String,
    pub name: String,
    pub player_name: String,
    pub level: i32,
    pub ancestry: String,
    pub heritage: String,
    pub background: String,
    pub class: String,
    pub size: String,
    /// Land Speed in feet, bonuses included.
    pub speed: i32,
    /// Ability modifiers (not scores), indexed by `Ability as usize`.
    pub abilities: [i32; 6],
    pub key_ability: Ability,
    /// Before Drained.
    pub max_hp: i32,
    pub current_hp: i32,
    pub temp_hp: i32,
    /// AC as built (proficiency, capped Dex, armor), before conditions and a raised shield.
    pub ac_base: i32,
    /// The shield's circumstance bonus to AC while raised; 0 without a shield.
    pub shield_bonus: i32,
    pub perception_rank: ProfRank,
    pub fort_rank: ProfRank,
    pub reflex_rank: ProfRank,
    pub will_rank: ProfRank,
    pub class_dc_rank: ProfRank,
    /// Indexed by `Skill as usize`.
    pub skill_ranks: [ProfRank; 16],
    pub lores: Vec<Lore>,
    /// Item bonuses keyed by lowercase stat: `perception`, a save id, a skill id, or
    /// `<subject> lore` (e.g. `warfare lore`).
    pub item_bonuses: BTreeMap<String, i32>,
    pub strikes: Vec<WeaponStrike>,
    pub spellcasting: Vec<SpellcastingEntry>,
    pub feats: Vec<String>,
    pub conditions: Vec<Condition>,
    pub hero_points: u8,
    pub linked_token_id: Option<String>,
}

/// Lowercase, alphanumerics kept (Unicode included), every other run of characters becomes a
/// single `-`, no leading or trailing `-`. `"Ezren the Wise!"` → `ezren-the-wise`. Empty input
/// or input with no alphanumerics gives `character`.
pub fn slugify(name: &str) -> String {
    let mut out = String::new();
    let mut sep = false;
    for ch in name.chars() {
        if ch.is_alphanumeric() {
            if sep && !out.is_empty() {
                out.push('-')
            }
            sep = false;
            out.extend(ch.to_lowercase());
        } else if !out.is_empty() {
            sep = true;
        }
    }
    if out.is_empty() {
        "character".into()
    } else {
        out
    }
}

fn string_array(values: &[String]) -> JsonValue {
    JsonValue::Array(values.iter().cloned().map(JsonValue::from).collect())
}

fn json_i32(value: &JsonValue, key: &str) -> Result<i32, String> {
    i32::try_from(
        value
            .as_i64()
            .ok_or_else(|| format!("{key} must be an integer"))?,
    )
    .map_err(|_| format!("{key} out of range"))
}

fn json_u32(value: &JsonValue, key: &str) -> Result<u32, String> {
    u32::try_from(
        value
            .as_i64()
            .ok_or_else(|| format!("{key} must be an integer"))?,
    )
    .map_err(|_| format!("{key} out of range"))
}

fn json_u8(value: &JsonValue, key: &str) -> Result<u8, String> {
    u8::try_from(
        value
            .as_i64()
            .ok_or_else(|| format!("{key} must be an integer"))?,
    )
    .map_err(|_| format!("{key} out of range"))
}

fn required_string(value: &JsonValue, key: &str) -> Result<String, String> {
    value
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| format!("{key} must be a string"))
}

fn required_object<'a>(
    value: &'a JsonValue,
    key: &str,
) -> Result<&'a [(String, JsonValue)], String> {
    value
        .as_object()
        .ok_or_else(|| format!("{key} must be an object"))
}

fn required_array<'a>(value: &'a JsonValue, key: &str) -> Result<&'a [JsonValue], String> {
    value
        .as_array()
        .ok_or_else(|| format!("{key} must be an array"))
}

fn optional_i32(value: &JsonValue, key: &str) -> Result<Option<i32>, String> {
    value.get(key).map(|v| json_i32(v, key)).transpose()
}

fn optional_string(value: &JsonValue, key: &str) -> Result<Option<String>, String> {
    value.get(key).map(|v| required_string(v, key)).transpose()
}

fn parse_rank(value: &JsonValue, key: &str) -> Result<ProfRank, String> {
    let text = required_string(value, key)?;
    ProfRank::parse(&text).ok_or_else(|| format!("{key} is not a proficiency rank"))
}

fn parse_ability(value: &JsonValue, key: &str) -> Result<Ability, String> {
    let text = required_string(value, key)?;
    Ability::parse(&text).ok_or_else(|| format!("{key} is not an ability"))
}

fn parse_string_array(value: &JsonValue, key: &str) -> Result<Vec<String>, String> {
    required_array(value, key)?
        .iter()
        .enumerate()
        .map(|(i, item)| required_string(item, &format!("{key}[{i}]")))
        .collect()
}

impl CharacterSheet {
    /// A blank sheet: `id = slugify(name)`, every ability 0, key ability Str, everything
    /// untrained, 0 HP and AC 10, size `Medium`, speed 25, 1 hero point (a session's start),
    /// empty strings and lists elsewhere.
    pub fn new(name: &str, level: i32) -> CharacterSheet {
        CharacterSheet {
            id: slugify(name),
            name: name.into(),
            player_name: String::new(),
            level,
            ancestry: String::new(),
            heritage: String::new(),
            background: String::new(),
            class: String::new(),
            size: "Medium".into(),
            speed: 25,
            abilities: [0; 6],
            key_ability: Ability::Str,
            max_hp: 0,
            current_hp: 0,
            temp_hp: 0,
            ac_base: 10,
            shield_bonus: 0,
            perception_rank: ProfRank::Untrained,
            fort_rank: ProfRank::Untrained,
            reflex_rank: ProfRank::Untrained,
            will_rank: ProfRank::Untrained,
            class_dc_rank: ProfRank::Untrained,
            skill_ranks: [ProfRank::Untrained; 16],
            lores: vec![],
            item_bonuses: BTreeMap::new(),
            strikes: vec![],
            spellcasting: vec![],
            feats: vec![],
            conditions: vec![],
            hero_points: 1,
            linked_token_id: None,
        }
    }

    pub fn ability_mod(&self, ability: Ability) -> i32 {
        self.abilities[ability as usize]
    }

    /// Case-insensitive exact feat name.
    pub fn has_feat(&self, name: &str) -> bool {
        self.feats.iter().any(|f| f.eq_ignore_ascii_case(name))
    }

    /// Drained lowers maximum HP by level (minimum 1) times its value. Never below 1.
    pub fn effective_max_hp(&self) -> i32 {
        let drained = self
            .conditions
            .iter()
            .filter_map(|c| {
                if let Condition::Drained(v) = c {
                    Some(i32::from(*v))
                } else {
                    None
                }
            })
            .max()
            .unwrap_or(0);
        self.max_hp
            .saturating_sub(self.level.max(1).saturating_mul(drained))
            .max(1)
    }

    /// `ac_base` + Dex status penalty (Frightened, Sickened, Clumsy) + circumstance modifiers
    /// (raised shield, off-guard).
    pub fn effective_ac(&self) -> i32 {
        self.ac_base
            + conditions::status_penalty(&self.conditions, Ability::Dex)
            + conditions::ac_circumstance(&self.conditions)
    }

    /// Gains `ShieldRaised(shield_bonus)`; does nothing without a shield.
    pub fn raise_shield(&mut self) {
        if self.shield_bonus > 0 {
            conditions::gain_condition(
                &mut self.conditions,
                Condition::ShieldRaised(self.shield_bonus.clamp(1, 255) as u8),
            );
        }
    }

    pub fn perception_modifier(&self) -> i32 {
        proficiency_bonus(self.perception_rank, self.level)
            + self.ability_mod(Ability::Wis)
            + self.item_bonuses.get("perception").copied().unwrap_or(0)
            + conditions::status_penalty(&self.conditions, Ability::Wis)
    }

    pub fn save_modifier(&self, save: Save) -> i32 {
        let rank = match save {
            Save::Fortitude => self.fort_rank,
            Save::Reflex => self.reflex_rank,
            Save::Will => self.will_rank,
        };
        proficiency_bonus(rank, self.level)
            + self.ability_mod(save.ability())
            + self.item_bonuses.get(save.id()).copied().unwrap_or(0)
            + conditions::status_penalty(&self.conditions, save.ability())
    }

    /// A skill by id or label, or a Lore by subject with or without the word "Lore"
    /// (`Warfare`, `warfare lore`), case-insensitive. `None` for an unknown skill or a Lore the
    /// character doesn't have.
    pub fn skill_rank(&self, name: &str) -> Option<ProfRank> {
        resolve_skill(self, name).map(|(_, r, _)| r)
    }

    /// Proficiency + ability + item bonus + status penalty for the skill's ability. Untrained
    /// checks use 0 proficiency, or with the Untrained Improvisation feat half the level
    /// (rounded down), full level from 7th.
    pub fn skill_modifier(&self, name: &str) -> Option<i32> {
        let (ability, rank, key) = resolve_skill(self, name)?;
        let prof = if rank == ProfRank::Untrained && self.has_feat("Untrained Improvisation") {
            if self.level >= 7 {
                self.level
            } else {
                self.level / 2
            }
        } else {
            proficiency_bonus(rank, self.level)
        };
        Some(
            prof + self.ability_mod(ability)
                + self.item_bonuses.get(&key).copied().unwrap_or(0)
                + conditions::status_penalty(&self.conditions, ability),
        )
    }

    /// Assurance: 10 + proficiency bonus, ignoring every other bonus, penalty and modifier
    /// (conditions included). Untrained Improvisation does count, as it sets the proficiency
    /// bonus itself.
    pub fn assurance_total(&self, name: &str) -> Option<i32> {
        let (_, rank, _) = resolve_skill(self, name)?;
        let p = if rank == ProfRank::Untrained && self.has_feat("Untrained Improvisation") {
            if self.level >= 7 {
                self.level
            } else {
                self.level / 2
            }
        } else {
            proficiency_bonus(rank, self.level)
        };
        Some(10 + p)
    }

    /// 10 + proficiency + key ability + its status penalty.
    pub fn class_dc(&self) -> i32 {
        10 + proficiency_bonus(self.class_dc_rank, self.level)
            + self.ability_mod(self.key_ability)
            + conditions::status_penalty(&self.conditions, self.key_ability)
    }

    /// The Strike's attack bonus + status penalty for its attack ability + multiple attack
    /// penalty for `map_step` (0, 1, 2+), agile-aware. `None` for an unknown index.
    pub fn strike_attack_modifier(&self, index: usize, map_step: u8) -> Option<i32> {
        let s = self.strikes.get(index)?;
        let p = if map_step == 0 {
            0
        } else if map_step == 1 {
            if s.is_agile() {
                4
            } else {
                5
            }
        } else if s.is_agile() {
            8
        } else {
            10
        };
        Some(s.attack_bonus + conditions::status_penalty(&self.conditions, s.attack_ability) - p)
    }

    /// The Strike's damage with Enfeebled's status penalty folded into `flat_bonus` when the
    /// Strike adds Strength to damage.
    pub fn strike_damage(&self, index: usize) -> Option<StrikeDamage> {
        let s = self.strikes.get(index)?;
        let mut d = s.damage.clone();
        if s.adds_strength_to_damage() {
            // Only Enfeebled reaches damage: Frightened and Sickened penalise checks and DCs,
            // and a damage roll is neither.
            let enfeebled = self
                .conditions
                .iter()
                .filter_map(|c| match c {
                    Condition::Enfeebled(v) => Some(i32::from(*v)),
                    _ => None,
                })
                .max()
                .unwrap_or(0);
            d.flat_bonus -= enfeebled;
        }
        Some(d)
    }

    /// Proficiency + spellcasting ability + its status penalty.
    pub fn spell_attack_modifier(&self, entry: usize) -> Option<i32> {
        let e = self.spellcasting.get(entry)?;
        Some(
            proficiency_bonus(e.rank, self.level)
                + self.ability_mod(e.ability)
                + conditions::status_penalty(&self.conditions, e.ability),
        )
    }

    /// 10 + spell attack modifier.
    pub fn spell_dc(&self, entry: usize) -> Option<i32> {
        self.spell_attack_modifier(entry).map(|n| 10 + n)
    }

    /// Temporary HP absorb damage first; current HP stops at 0. Negative amounts do nothing.
    pub fn apply_damage(&mut self, amount: i32) {
        if amount <= 0 {
            return;
        }
        let absorbed = amount.min(self.temp_hp);
        self.temp_hp -= absorbed;
        self.current_hp = self.current_hp.saturating_sub(amount - absorbed).max(0)
    }

    /// Up to `effective_max_hp`. Negative amounts do nothing.
    pub fn heal(&mut self, amount: i32) {
        if amount > 0 {
            self.current_hp = self
                .current_hp
                .saturating_add(amount)
                .min(self.effective_max_hp());
        }
    }

    /// Temporary HP don't stack: keep the higher of the current and new amounts.
    pub fn gain_temp_hp(&mut self, amount: i32) {
        self.temp_hp = self.temp_hp.max(amount.max(0))
    }

    /// Storage form. Top level has `"format": "owlbearbuilder-character"` and `"version": 1`,
    /// then every field under its own name. `abilities` is `{"str": 4, ...}`; ranks are rank
    /// ids; `skill_ranks` is `{"acrobatics": "trained", ...}`; `item_bonuses` is an object;
    /// each strike's `damage` is `{"dice_count", "die_sides", "flat_bonus", "deadly",
    /// "fatal"}` (absent traits as null); spellcasting `spells` is
    /// `[{"rank": 0, "names": [...]}, ...]`; conditions use `Condition::to_json`;
    /// `linked_token_id` is a string or null.
    pub fn to_json(&self) -> JsonValue {
        let mut value = JsonValue::object();
        value.insert("format", "owlbearbuilder-character");
        value.insert("version", 1);
        value.insert("id", self.id.clone());
        value.insert("name", self.name.clone());
        value.insert("player_name", self.player_name.clone());
        value.insert("level", self.level);
        value.insert("ancestry", self.ancestry.clone());
        value.insert("heritage", self.heritage.clone());
        value.insert("background", self.background.clone());
        value.insert("class", self.class.clone());
        value.insert("size", self.size.clone());
        value.insert("speed", self.speed);
        let mut abilities = JsonValue::object();
        for ability in Ability::ALL {
            abilities.insert(ability.id(), self.ability_mod(ability));
        }
        value.insert("abilities", abilities);
        value.insert("key_ability", self.key_ability.id());
        value.insert("max_hp", self.max_hp);
        value.insert("current_hp", self.current_hp);
        value.insert("temp_hp", self.temp_hp);
        value.insert("ac_base", self.ac_base);
        value.insert("shield_bonus", self.shield_bonus);
        value.insert("perception_rank", self.perception_rank.id());
        value.insert("fort_rank", self.fort_rank.id());
        value.insert("reflex_rank", self.reflex_rank.id());
        value.insert("will_rank", self.will_rank.id());
        value.insert("class_dc_rank", self.class_dc_rank.id());
        let mut skill_ranks = JsonValue::object();
        for skill in Skill::ALL {
            skill_ranks.insert(skill.id(), self.skill_ranks[skill as usize].id());
        }
        value.insert("skill_ranks", skill_ranks);
        value.insert(
            "lores",
            JsonValue::Array(
                self.lores
                    .iter()
                    .map(|lore| {
                        let mut item = JsonValue::object();
                        item.insert("name", lore.name.clone());
                        item.insert("rank", lore.rank.id());
                        item
                    })
                    .collect(),
            ),
        );
        let mut item_bonuses = JsonValue::object();
        for (key, bonus) in &self.item_bonuses {
            item_bonuses.insert(key.clone(), *bonus);
        }
        value.insert("item_bonuses", item_bonuses);
        value.insert(
            "strikes",
            JsonValue::Array(
                self.strikes
                    .iter()
                    .map(|strike| {
                        let mut item = JsonValue::object();
                        item.insert("name", strike.name.clone());
                        item.insert("attack_bonus", strike.attack_bonus);
                        item.insert("attack_ability", strike.attack_ability.id());
                        item.insert("is_ranged", strike.is_ranged);
                        let mut damage = JsonValue::object();
                        damage.insert("dice_count", strike.damage.dice_count);
                        damage.insert("die_sides", strike.damage.die_sides);
                        damage.insert("flat_bonus", strike.damage.flat_bonus);
                        damage.insert(
                            "deadly",
                            strike
                                .damage
                                .deadly
                                .map_or(JsonValue::Null, JsonValue::from),
                        );
                        damage.insert(
                            "fatal",
                            strike.damage.fatal.map_or(JsonValue::Null, JsonValue::from),
                        );
                        item.insert("damage", damage);
                        item.insert("damage_type", strike.damage_type.clone());
                        item.insert("extra_damage", string_array(&strike.extra_damage));
                        item.insert("traits", string_array(&strike.traits));
                        item
                    })
                    .collect(),
            ),
        );
        value.insert(
            "spellcasting",
            JsonValue::Array(
                self.spellcasting
                    .iter()
                    .map(|entry| {
                        let mut item = JsonValue::object();
                        item.insert("name", entry.name.clone());
                        item.insert("tradition", entry.tradition.clone());
                        item.insert("kind", entry.kind.clone());
                        item.insert("ability", entry.ability.id());
                        item.insert("rank", entry.rank.id());
                        item.insert("focus_points", u32::from(entry.focus_points));
                        item.insert(
                            "spells",
                            JsonValue::Array(
                                entry
                                    .spells
                                    .iter()
                                    .map(|(rank, names)| {
                                        let mut spell = JsonValue::object();
                                        spell.insert("rank", u32::from(*rank));
                                        spell.insert("names", string_array(names));
                                        spell
                                    })
                                    .collect(),
                            ),
                        );
                        item
                    })
                    .collect(),
            ),
        );
        value.insert("feats", string_array(&self.feats));
        value.insert(
            "conditions",
            JsonValue::Array(self.conditions.iter().map(Condition::to_json).collect()),
        );
        value.insert("hero_points", u32::from(self.hero_points));
        value.insert(
            "linked_token_id",
            self.linked_token_id
                .clone()
                .map_or(JsonValue::Null, JsonValue::from),
        );
        value
    }

    /// Reads `to_json` output. Only `name` and `level` are required; anything missing takes its
    /// value from `CharacterSheet::new`, unknown keys are ignored, and a wrongly typed value is
    /// an error naming the key.
    pub fn from_json(v: &JsonValue) -> Result<CharacterSheet, String> {
        required_object(v, "character")?;
        let name = required_string(v.get("name").ok_or("name is required")?, "name")?;
        let level = json_i32(v.get("level").ok_or("level is required")?, "level")?;
        let mut character = CharacterSheet::new(&name, level);
        if let Some(field) = v.get("id") {
            character.id = required_string(field, "id")?;
        }
        macro_rules! overlay_string {
            ($field:ident) => {
                if let Some(parsed) = optional_string(v, stringify!($field))? {
                    character.$field = parsed;
                }
            };
        }
        macro_rules! overlay_i32 {
            ($field:ident) => {
                if let Some(parsed) = optional_i32(v, stringify!($field))? {
                    character.$field = parsed;
                }
            };
        }
        overlay_string!(player_name);
        overlay_string!(ancestry);
        overlay_string!(heritage);
        overlay_string!(background);
        overlay_string!(class);
        overlay_string!(size);
        overlay_i32!(speed);
        overlay_i32!(max_hp);
        overlay_i32!(current_hp);
        overlay_i32!(temp_hp);
        overlay_i32!(ac_base);
        overlay_i32!(shield_bonus);
        if let Some(field) = v.get("key_ability") {
            character.key_ability = parse_ability(field, "key_ability")?;
        }
        for (field, rank) in [
            ("perception_rank", &mut character.perception_rank),
            ("fort_rank", &mut character.fort_rank),
            ("reflex_rank", &mut character.reflex_rank),
            ("will_rank", &mut character.will_rank),
            ("class_dc_rank", &mut character.class_dc_rank),
        ] {
            if let Some(value) = v.get(field) {
                *rank = parse_rank(value, field)?;
            }
        }
        if let Some(value) = v.get("abilities") {
            for (key, item) in required_object(value, "abilities")? {
                if let Some(ability) = Ability::parse(key) {
                    character.abilities[ability as usize] =
                        json_i32(item, &format!("abilities.{key}"))?;
                }
            }
        }
        if let Some(value) = v.get("skill_ranks") {
            for (key, item) in required_object(value, "skill_ranks")? {
                if let Some(skill) = Skill::parse(key) {
                    character.skill_ranks[skill as usize] =
                        parse_rank(item, &format!("skill_ranks.{key}"))?;
                }
            }
        }
        if let Some(value) = v.get("lores") {
            character.lores = required_array(value, "lores")?
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    let prefix = format!("lores[{i}]");
                    required_object(item, &prefix)?;
                    Ok(Lore {
                        name: required_string(
                            item.get("name")
                                .ok_or_else(|| format!("{prefix}.name is required"))?,
                            &format!("{prefix}.name"),
                        )?,
                        rank: parse_rank(
                            item.get("rank")
                                .ok_or_else(|| format!("{prefix}.rank is required"))?,
                            &format!("{prefix}.rank"),
                        )?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
        }
        if let Some(value) = v.get("item_bonuses") {
            character.item_bonuses.clear();
            for (key, item) in required_object(value, "item_bonuses")? {
                character
                    .item_bonuses
                    .insert(key.clone(), json_i32(item, &format!("item_bonuses.{key}"))?);
            }
        }
        if let Some(value) = v.get("strikes") {
            character.strikes = required_array(value, "strikes")?
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    let prefix = format!("strikes[{i}]");
                    required_object(item, &prefix)?;
                    let string_field = |field: &str| -> Result<String, String> {
                        required_string(
                            item.get(field)
                                .ok_or_else(|| format!("{prefix}.{field} is required"))?,
                            &format!("{prefix}.{field}"),
                        )
                    };
                    let int_field = |field: &str| -> Result<i32, String> {
                        json_i32(
                            item.get(field)
                                .ok_or_else(|| format!("{prefix}.{field} is required"))?,
                            &format!("{prefix}.{field}"),
                        )
                    };
                    let ranged = item
                        .get("is_ranged")
                        .and_then(JsonValue::as_bool)
                        .ok_or_else(|| format!("{prefix}.is_ranged must be a boolean"))?;
                    let attack_ability = parse_ability(
                        item.get("attack_ability")
                            .ok_or_else(|| format!("{prefix}.attack_ability is required"))?,
                        &format!("{prefix}.attack_ability"),
                    )?;
                    let damage_value = item
                        .get("damage")
                        .ok_or_else(|| format!("{prefix}.damage is required"))?;
                    let damage_prefix = format!("{prefix}.damage");
                    required_object(damage_value, &damage_prefix)?;
                    let damage_u32 = |field: &str| -> Result<u32, String> {
                        json_u32(
                            damage_value
                                .get(field)
                                .ok_or_else(|| format!("{damage_prefix}.{field} is required"))?,
                            &format!("{damage_prefix}.{field}"),
                        )
                    };
                    let damage_i32 = |field: &str| -> Result<i32, String> {
                        json_i32(
                            damage_value
                                .get(field)
                                .ok_or_else(|| format!("{damage_prefix}.{field} is required"))?,
                            &format!("{damage_prefix}.{field}"),
                        )
                    };
                    let optional_die = |field: &str| -> Result<Option<u32>, String> {
                        match damage_value.get(field) {
                            None | Some(JsonValue::Null) => Ok(None),
                            Some(value) => {
                                Ok(Some(json_u32(value, &format!("{damage_prefix}.{field}"))?))
                            }
                        }
                    };
                    Ok(WeaponStrike {
                        name: string_field("name")?,
                        attack_bonus: int_field("attack_bonus")?,
                        attack_ability,
                        is_ranged: ranged,
                        damage: StrikeDamage {
                            dice_count: damage_u32("dice_count")?,
                            die_sides: damage_u32("die_sides")?,
                            flat_bonus: damage_i32("flat_bonus")?,
                            deadly: optional_die("deadly")?,
                            fatal: optional_die("fatal")?,
                        },
                        damage_type: string_field("damage_type")?,
                        extra_damage: parse_string_array(
                            item.get("extra_damage")
                                .ok_or_else(|| format!("{prefix}.extra_damage is required"))?,
                            &format!("{prefix}.extra_damage"),
                        )?,
                        traits: parse_string_array(
                            item.get("traits")
                                .ok_or_else(|| format!("{prefix}.traits is required"))?,
                            &format!("{prefix}.traits"),
                        )?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
        }
        if let Some(value) = v.get("spellcasting") {
            character.spellcasting = required_array(value, "spellcasting")?
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    let prefix = format!("spellcasting[{i}]");
                    required_object(item, &prefix)?;
                    let string_field = |field: &str| -> Result<String, String> {
                        required_string(
                            item.get(field)
                                .ok_or_else(|| format!("{prefix}.{field} is required"))?,
                            &format!("{prefix}.{field}"),
                        )
                    };
                    let ability = parse_ability(
                        item.get("ability")
                            .ok_or_else(|| format!("{prefix}.ability is required"))?,
                        &format!("{prefix}.ability"),
                    )?;
                    let rank = parse_rank(
                        item.get("rank")
                            .ok_or_else(|| format!("{prefix}.rank is required"))?,
                        &format!("{prefix}.rank"),
                    )?;
                    let focus_points = json_u8(
                        item.get("focus_points")
                            .ok_or_else(|| format!("{prefix}.focus_points is required"))?,
                        &format!("{prefix}.focus_points"),
                    )?;
                    let spells = required_array(
                        item.get("spells")
                            .ok_or_else(|| format!("{prefix}.spells is required"))?,
                        &format!("{prefix}.spells"),
                    )?
                    .iter()
                    .enumerate()
                    .map(|(j, spell)| {
                        let spell_prefix = format!("{prefix}.spells[{j}]");
                        required_object(spell, &spell_prefix)?;
                        let rank = json_u8(
                            spell
                                .get("rank")
                                .ok_or_else(|| format!("{spell_prefix}.rank is required"))?,
                            &format!("{spell_prefix}.rank"),
                        )?;
                        let names = parse_string_array(
                            spell
                                .get("names")
                                .ok_or_else(|| format!("{spell_prefix}.names is required"))?,
                            &format!("{spell_prefix}.names"),
                        )?;
                        Ok((rank, names))
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                    Ok(SpellcastingEntry {
                        name: string_field("name")?,
                        tradition: string_field("tradition")?,
                        kind: string_field("kind")?,
                        ability,
                        rank,
                        focus_points,
                        spells,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
        }
        if let Some(value) = v.get("feats") {
            character.feats = parse_string_array(value, "feats")?;
        }
        if let Some(value) = v.get("conditions") {
            character.conditions = required_array(value, "conditions")?
                .iter()
                .enumerate()
                .map(|(i, condition)| {
                    let prefix = format!("conditions[{i}]");
                    required_object(condition, &prefix)?;
                    let name = required_string(
                        condition
                            .get("name")
                            .ok_or_else(|| format!("{prefix}.name is required"))?,
                        &format!("{prefix}.name"),
                    )?;
                    let condition_value = match condition.get("value") {
                        None => None,
                        Some(value) => Some(json_u8(value, &format!("{prefix}.value"))?),
                    };
                    Condition::parse(&name, condition_value)
                        .map_err(|error| format!("{prefix}.name: {error}"))
                })
                .collect::<Result<Vec<_>, String>>()?;
        }
        if let Some(value) = v.get("hero_points") {
            character.hero_points = json_u8(value, "hero_points")?;
        }
        if let Some(value) = v.get("linked_token_id") {
            character.linked_token_id = if value.is_null() {
                None
            } else {
                Some(required_string(value, "linked_token_id")?)
            };
        }
        Ok(character)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_abilities_match_player_core() {
        let expected = [
            Ability::Dex,
            Ability::Int,
            Ability::Str,
            Ability::Int,
            Ability::Cha,
            Ability::Cha,
            Ability::Cha,
            Ability::Wis,
            Ability::Wis,
            Ability::Int,
            Ability::Cha,
            Ability::Wis,
            Ability::Int,
            Ability::Dex,
            Ability::Wis,
            Ability::Dex,
        ];
        for (skill, ability) in Skill::ALL.into_iter().zip(expected) {
            assert_eq!(skill.ability(), ability, "{}", skill.id());
        }
    }

    #[test]
    fn rank_labels_parse_case_insensitively() {
        for rank in [
            ProfRank::Untrained,
            ProfRank::Trained,
            ProfRank::Expert,
            ProfRank::Master,
            ProfRank::Legendary,
        ] {
            assert_eq!(ProfRank::parse(rank.label()), Some(rank));
            assert_eq!(
                ProfRank::parse(&rank.label().to_ascii_uppercase()),
                Some(rank)
            );
        }
    }

    #[test]
    fn slugify_preserves_unicode_and_collapses_punctuation() {
        assert_eq!(slugify("Ezrén"), "ezrén");
        assert_eq!(slugify("  Ezrén---the___Wise!!  "), "ezrén-the-wise");
    }

    #[test]
    fn drained_hp_has_a_floor_and_level_zero_uses_level_one() {
        let mut character = CharacterSheet::new("Test", 0);
        character.max_hp = 5;
        character.conditions.push(Condition::Drained(9));
        assert_eq!(character.effective_max_hp(), 1);
        character.max_hp = 0;
        assert_eq!(character.effective_max_hp(), 1);
        character.max_hp = 10;
        character.conditions = vec![Condition::Drained(2)];
        assert_eq!(character.effective_max_hp(), 8);
    }

    #[test]
    fn nested_json_errors_include_the_field_path() {
        let mut value = CharacterSheet::new("Test", 1).to_json();
        let mut strike = JsonValue::object();
        strike.insert("name", "Weapon");
        strike.insert("attack_bonus", "strong");
        strike.insert("attack_ability", "str");
        strike.insert("is_ranged", false);
        let mut damage = JsonValue::object();
        damage.insert("dice_count", 1);
        damage.insert("die_sides", 6);
        damage.insert("flat_bonus", 0);
        damage.insert("deadly", JsonValue::Null);
        damage.insert("fatal", JsonValue::Null);
        strike.insert("damage", damage);
        strike.insert("damage_type", "slashing");
        strike.insert("extra_damage", JsonValue::Array(vec![]));
        strike.insert("traits", JsonValue::Array(vec![]));
        value.insert("strikes", JsonValue::Array(vec![strike]));
        assert!(CharacterSheet::from_json(&value)
            .unwrap_err()
            .contains("strikes[0].attack_bonus"));
    }
}
