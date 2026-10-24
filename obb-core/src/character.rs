// SPDX-License-Identifier: GPL-3.0-or-later
//! The canonical character sheet every importer produces, and the modifiers it yields once
//! conditions are applied. Rules references are to Pathfinder Player Core (Remaster).

use std::collections::BTreeMap;

use crate::conditions::{self, Condition};
use crate::dice::{map_penalty, StrikeDamage};
use crate::json::JsonValue;

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
        todo!()
    }

    /// Accepts the id or the full name (`strength`), case-insensitive.
    pub fn parse(s: &str) -> Option<Ability> {
        todo!()
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
        todo!()
    }

    /// From the rank bonus builders export (0/2/4/6/8). Odd values round down to a rank;
    /// anything below 0 is Untrained and anything above 8 is Legendary.
    pub fn from_bonus(bonus: i64) -> ProfRank {
        todo!()
    }

    /// `untrained`, `trained`, `expert`, `master`, `legendary`.
    pub fn id(self) -> &'static str {
        todo!()
    }

    /// `Untrained`, `Trained`, ...
    pub fn label(self) -> &'static str {
        todo!()
    }

    /// `U`, `T`, `E`, `M`, `L`.
    pub fn letter(self) -> char {
        todo!()
    }

    /// Accepts the id or label, case-insensitive.
    pub fn parse(s: &str) -> Option<ProfRank> {
        todo!()
    }
}

/// Proficiency bonus: the rank's bonus plus level once trained, 0 when untrained.
pub fn proficiency_bonus(rank: ProfRank, level: i32) -> i32 {
    todo!()
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
        todo!()
    }

    /// Capitalised name, e.g. `Athletics`.
    pub fn label(self) -> &'static str {
        todo!()
    }

    /// Acrobatics, Stealth, Thievery: Dex. Athletics: Str. Arcana, Crafting, Occultism,
    /// Society: Int. Medicine, Nature, Religion, Survival: Wis. Deception, Diplomacy,
    /// Intimidation, Performance: Cha.
    pub fn ability(self) -> Ability {
        todo!()
    }

    /// Case-insensitive name.
    pub fn parse(s: &str) -> Option<Skill> {
        todo!()
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
        todo!()
    }

    /// `Fortitude`, `Reflex`, `Will`.
    pub fn label(self) -> &'static str {
        todo!()
    }

    /// Con, Dex, Wis.
    pub fn ability(self) -> Ability {
        todo!()
    }

    /// Case-insensitive id, label, or short form (`fort`, `ref`).
    pub fn parse(s: &str) -> Option<Save> {
        todo!()
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
        todo!()
    }

    /// Strength is added to damage for melee Strikes and thrown ranged ones, and Enfeebled
    /// penalises Strength-based damage, so this decides whether it applies to damage.
    pub fn adds_strength_to_damage(&self) -> bool {
        todo!()
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
    todo!()
}

impl CharacterSheet {
    /// A blank sheet: `id = slugify(name)`, every ability 0, key ability Str, everything
    /// untrained, 0 HP and AC 10, size `Medium`, speed 25, 1 hero point (a session's start),
    /// empty strings and lists elsewhere.
    pub fn new(name: &str, level: i32) -> CharacterSheet {
        todo!()
    }

    pub fn ability_mod(&self, ability: Ability) -> i32 {
        self.abilities[ability as usize]
    }

    /// Case-insensitive exact feat name.
    pub fn has_feat(&self, name: &str) -> bool {
        todo!()
    }

    /// Drained lowers maximum HP by level (minimum 1) times its value. Never below 1.
    pub fn effective_max_hp(&self) -> i32 {
        todo!()
    }

    /// `ac_base` + Dex status penalty (Frightened, Sickened, Clumsy) + circumstance modifiers
    /// (raised shield, off-guard).
    pub fn effective_ac(&self) -> i32 {
        todo!()
    }

    /// Gains `ShieldRaised(shield_bonus)`; does nothing without a shield.
    pub fn raise_shield(&mut self) {
        todo!()
    }

    pub fn perception_modifier(&self) -> i32 {
        todo!()
    }

    pub fn save_modifier(&self, save: Save) -> i32 {
        todo!()
    }

    /// A skill by id or label, or a Lore by subject with or without the word "Lore"
    /// (`Warfare`, `warfare lore`), case-insensitive. `None` for an unknown skill or a Lore the
    /// character doesn't have.
    pub fn skill_rank(&self, name: &str) -> Option<ProfRank> {
        todo!()
    }

    /// Proficiency + ability + item bonus + status penalty for the skill's ability. Untrained
    /// checks use 0 proficiency, or with the Untrained Improvisation feat half the level
    /// (rounded down), full level from 7th.
    pub fn skill_modifier(&self, name: &str) -> Option<i32> {
        todo!()
    }

    /// Assurance: 10 + proficiency bonus, ignoring every other bonus, penalty and modifier
    /// (conditions included). Untrained Improvisation does count, as it sets the proficiency
    /// bonus itself.
    pub fn assurance_total(&self, name: &str) -> Option<i32> {
        todo!()
    }

    /// 10 + proficiency + key ability + its status penalty.
    pub fn class_dc(&self) -> i32 {
        todo!()
    }

    /// The Strike's attack bonus + status penalty for its attack ability + multiple attack
    /// penalty for `map_step` (0, 1, 2+), agile-aware. `None` for an unknown index.
    pub fn strike_attack_modifier(&self, index: usize, map_step: u8) -> Option<i32> {
        todo!()
    }

    /// The Strike's damage with Enfeebled's status penalty folded into `flat_bonus` when the
    /// Strike adds Strength to damage.
    pub fn strike_damage(&self, index: usize) -> Option<StrikeDamage> {
        todo!()
    }

    /// Proficiency + spellcasting ability + its status penalty.
    pub fn spell_attack_modifier(&self, entry: usize) -> Option<i32> {
        todo!()
    }

    /// 10 + spell attack modifier.
    pub fn spell_dc(&self, entry: usize) -> Option<i32> {
        todo!()
    }

    /// Temporary HP absorb damage first; current HP stops at 0. Negative amounts do nothing.
    pub fn apply_damage(&mut self, amount: i32) {
        todo!()
    }

    /// Up to `effective_max_hp`. Negative amounts do nothing.
    pub fn heal(&mut self, amount: i32) {
        todo!()
    }

    /// Temporary HP don't stack: keep the higher of the current and new amounts.
    pub fn gain_temp_hp(&mut self, amount: i32) {
        todo!()
    }

    /// Storage form. Top level has `"format": "owlbearbuilder-character"` and `"version": 1`,
    /// then every field under its own name. `abilities` is `{"str": 4, ...}`; ranks are rank
    /// ids; `skill_ranks` is `{"acrobatics": "trained", ...}`; `item_bonuses` is an object;
    /// each strike's `damage` is `{"dice_count", "die_sides", "flat_bonus", "deadly",
    /// "fatal"}` (absent traits as null); spellcasting `spells` is
    /// `[{"rank": 0, "names": [...]}, ...]`; conditions use `Condition::to_json`;
    /// `linked_token_id` is a string or null.
    pub fn to_json(&self) -> JsonValue {
        todo!()
    }

    /// Reads `to_json` output. Only `name` and `level` are required; anything missing takes its
    /// value from `CharacterSheet::new`, unknown keys are ignored, and a wrongly typed value is
    /// an error naming the key.
    pub fn from_json(v: &JsonValue) -> Result<CharacterSheet, String> {
        todo!()
    }
}
