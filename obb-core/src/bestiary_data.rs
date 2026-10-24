// SPDX-License-Identifier: GPL-3.0-or-later
//! A few creatures from Pathfinder Monster Core, used under the ORC License, so a table can try
//! token binding and Elite/Weak adjustments before pasting its own stat blocks.

use crate::monster::{parse_aon_statblock, MonsterStatBlock};

const GOBLIN_WARRIOR: &str = r#"Goblin Warrior Creature -1
Small Goblin Humanoid
Perception +2; darkvision
Languages Common, Goblin
Skills Acrobatics +5, Athletics +2, Nature +1, Stealth +5 (+7 in undergrowth)
Str +0, Dex +3, Con +1, Int +0, Wis -1, Cha +1
Items dogslicer, leather armor, shortbow (10 arrows)
AC 16; Fort +5, Ref +7, Will +3
HP 6
Goblin Scuttle [reaction] Trigger A goblin ally ends a move action adjacent to the warrior; Effect The Goblin Warrior Steps.
Speed 25 feet
Melee [one-action] dogslicer +7 (agile, backstabber, finesse), Damage 1d6 slashing
Ranged [one-action] shortbow +7 (deadly d10, range increment 60 feet), Damage 1d6 piercing
"#;

const ORC_BRUTE: &str = r#"Orc Brute Creature 0
Medium Humanoid Orc
Perception +5; darkvision
Languages Common, Orcish
Skills Athletics +6, Intimidation +4
Str +3, Dex +2, Con +3, Int -1, Wis +1, Cha +0
Items falchion, javelins (4), shoddy breastplate
AC 15; Fort +8, Ref +5, Will +4
HP 15
Ferocity [reaction] Trigger The monster is reduced to 0 HP; Effect The monster avoids being knocked out and remains at 1 HP, increasing its wounded value by 1.
Speed 25 feet
Melee [one-action] falchion +7 (forceful, sweep), Damage 1d10+3 slashing
Melee [one-action] fist +7 (agile, nonlethal), Damage 1d4+3 bludgeoning
Ranged [one-action] javelin +5 (thrown 30 feet), Damage 1d6+3 piercing
"#;

const SKELETON_GUARD: &str = r#"Skeleton Guard Creature -1
Medium Mindless Skeleton Undead Unholy
Perception +2; darkvision
Skills Acrobatics +6, Athletics +3
Str +2, Dex +4, Con +0, Int -5, Wis +0, Cha +0
Items scimitar, shortbow (20 arrows)
AC 16; Fort +2, Ref +8, Will +2
HP 4 (void healing); Immunities bleed, death effects, disease, mental, paralyzed, poison, unconscious; Resistances cold 5, electricity 5, fire 5, piercing 5, slashing 5
Speed 25 feet
Melee [one-action] scimitar +6 (forceful, sweep), Damage 1d6+2 slashing
Melee [one-action] claw +6 (agile, finesse), Damage 1d4+2 slashing
Ranged [one-action] shortbow +6 (deadly d10, range increment 60 feet), Damage 1d6 piercing
"#;

const KOBOLD_WARRIOR: &str = r#"Kobold Warrior Creature -1
Small Humanoid Kobold
Perception +3; darkvision
Languages Common, Sakvroth
Skills Acrobatics +5, Crafting +2 (+4 traps), Stealth +5
Str +1, Dex +3, Con -1, Int +0, Wis +1, Cha +1
Items leather armor, sling (20 bullets), spear
AC 16; Fort +3, Ref +7, Will +3
HP 7
Scamper [one-action] Requirements The kobold warrior is adjacent to at least one enemy; Effect The kobold warrior Strides up to their Speed plus 5 feet, gaining a +2 circumstance bonus to AC against reactions triggered by this movement, and must end in a space not adjacent to any enemy.
Speed 25 feet
Melee [one-action] spear +3, Damage 1d6+1 piercing
Ranged [one-action] sling +5 (propulsive, range increment 50 feet, reload 1), Damage 1d4 bludgeoning
Ranged [one-action] spear +5 (thrown 20 feet), Damage 1d6+1 piercing
Sneak Attack. The kobold warrior deals an extra 1d4 precision damage to off-guard creatures.
"#;

const GIANT_RAT: &str = r#"Giant Rat Creature -1
Small Animal
Perception +5; low-light vision, scent (imprecise) 30 feet
Skills Acrobatics +5, Athletics +2 (+4 to Climb or Swim), Stealth +5
Str +1, Dex +3, Con +2, Int -4, Wis +1, Cha -3
AC 15; Fort +6, Ref +7, Will +3
HP 8
Speed 30 feet, climb 10 feet
Melee [one-action] jaws +7 (agile, finesse), Damage 1d6+1 piercing plus filth fever
Filth Fever. (disease) Saving Throw DC 14 Fortitude; Stage 1 carrier with no ill effect (1d4 hours); Stage 2 sickened 1 (1 day); Stage 3 sickened 1 and slowed 1 (1 day).
"#;

const OGRE_WARRIOR: &str = r#"Ogre Warrior Creature 3
Large Giant Humanoid
Perception +5; darkvision
Languages Jotun
Skills Athletics +12, Intimidation +9
Str +5, Dex -1, Con +4, Int -2, Wis +0, Cha -2
Items hide armor, javelins (6), ogre hook
AC 17; Fort +11, Ref +6, Will +5
HP 50
Speed 25 feet
Melee [one-action] ogre hook +12 (deadly d10, reach 10 feet, trip), Damage 1d10+7 piercing
Melee [one-action] fist +12 (agile, reach 10 feet), Damage 1d4+7 bludgeoning
Ranged [one-action] javelin +6 (thrown 30 feet), Damage 1d6+7 piercing
"#;

/// Goblin Warrior, Orc Brute, Skeleton Guard, Kobold Warrior, Giant Rat and Ogre Warrior, in
/// that order, parsed from stat block text embedded in this module.
pub fn builtin_bestiary() -> Vec<MonsterStatBlock> {
    vec![
        parse_aon_statblock(GOBLIN_WARRIOR).expect("goblin"),
        parse_aon_statblock(ORC_BRUTE).expect("orc"),
        parse_aon_statblock(SKELETON_GUARD).expect("skeleton"),
        parse_aon_statblock(KOBOLD_WARRIOR).expect("kobold"),
        parse_aon_statblock(GIANT_RAT).expect("rat"),
        parse_aon_statblock(OGRE_WARRIOR).expect("ogre"),
    ]
}
