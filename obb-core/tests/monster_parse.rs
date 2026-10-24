// SPDX-License-Identifier: GPL-3.0-or-later

use obb_core::json::JsonValue;
use obb_core::monster::*;

const ORC_BRUTE: &str = "ORC BRUTE CREATURE 0
MEDIUM ORC HUMANOID
Perception +5; darkvision
Skills Athletics +6, Intimidation +4
Str +3, Dex +2, Con +3, Int -1, Wis +1, Cha +0
AC 15; Fort +8, Ref +5, Will +4
HP 15
Speed 25 feet
Melee [one-action] fist +7 (agile, nonlethal), Damage 1d4+3 bludgeoning
Melee [one-action] falchion +7 (forceful, sweep), Damage 1d10+3 slashing
Ranged [one-action] javelin +5 (thrown 30 feet), Damage 1d6+3 piercing
";

const GOBLIN: &str = "Goblin Warrior
Source Monster Core pg. 172
Goblin Warrior Creature -1
Small Goblin Humanoid
Perception +2; darkvision
Languages Common, Goblin
Skills Acrobatics +5, Athletics +2, Nature +1, Stealth +5 (+7 in undergrowth)
Str +0, Dex +3, Con +1, Int +0, Wis -1, Cha +1
Items dogslicer, leather armor, shortbow (10 arrows)
AC 16; Fort +5, Ref +7, Will +3
HP 6
Speed 25 feet
Melee Single Action dogslicer +7 [+3/-1] (agile, backstabber, finesse), Damage 1d6 slashing
Ranged Single Action shortbow +7 [+2/-3] (deadly d10, range increment 60 feet), Damage 1d6 piercing
Recall Knowledge - Humanoid (Society): DC 13
";

const EMBER_WITCH: &str = "EMBER WITCH CREATURE 4
UNCOMMON CE MEDIUM HUMAN HUMANOID
Perception +10; low-light vision
Languages Common, Infernal
Skills Arcana +12, Deception +11, Occultism +10
Str -1, Dex +2, Con +1, Int +4, Wis +2, Cha +3
Items staff, spellbook
AC 20; Fort +9, Ref +11, Will +13; +1 status to all saves vs. magic
HP 50 (void healing); Immunities fire, sleep; Weaknesses cold 5; Resistances physical 3 (except silver), poison 5
Ember Ward [reaction] (fire, occult) Trigger A creature within 15 feet hits the witch with a melee Strike; Effect The attacker takes 2d6 fire damage.
Speed 25 feet, fly 20 feet
Melee [one-action] staff +8 (two-hand d8), Damage 1d4+2 bludgeoning plus 1d6 fire
Ranged [two-actions] cinder bolt +12 (fire, range 60 feet), Damage 2d8 fire
Occult Spontaneous Spells DC 21, attack +13; 2nd (3 slots) blazing bolt, see the unseen; 1st (4 slots) breathe fire, fear; Cantrips (2nd) ignition, shield
Hex Fire ◆◆ (concentrate, fire) The witch hurls cursed flame.
The target takes 3d6 fire damage (DC 21 basic Reflex save).
";

const BEAST: &str = "Test Beast Creature 2
Large Animal
Perception +8; scent (imprecise) 30 feet
AC 18; Fort +10, Ref +7, Will +5
HP 30
Speed 40 feet
Melee ◆ jaws +11, Damage 1d10+5 piercing plus Grab
Melee ◆ tail +11 (agile, reach 10 feet), Effect Push 5 feet
";

#[test]
fn parses_the_classic_block() {
    let m = parse_aon_statblock(ORC_BRUTE).unwrap();
    assert_eq!(
        (m.id.as_str(), m.name.as_str(), m.level),
        ("orc-brute", "Orc Brute", 0)
    );
    assert_eq!(m.size, "Medium");
    assert_eq!(m.traits, ["orc", "humanoid"]);
    assert_eq!((m.perception, m.senses.as_str()), (5, "darkvision"));
    assert_eq!(
        m.skills,
        [
            ("Athletics".to_string(), 6),
            ("Intimidation".to_string(), 4)
        ]
    );
    assert_eq!(m.abilities, [3, 2, 3, -1, 1, 0]);
    assert_eq!(
        (m.ac, m.fort, m.reflex, m.will, m.max_hp),
        (15, 8, 5, 4, 15)
    );
    assert_eq!(m.speed, "25 feet");
    assert_eq!(m.strikes.len(), 3);
    let fist = &m.strikes[0];
    assert_eq!(
        (
            fist.name.as_str(),
            fist.is_ranged,
            fist.actions,
            fist.attack_mod
        ),
        ("fist", false, 1, 7)
    );
    assert_eq!(fist.traits, ["agile", "nonlethal"]);
    assert!(fist.is_agile());
    assert_eq!(
        (
            fist.damage_expr.as_str(),
            fist.damage_type.as_str(),
            fist.damage_extra.as_str()
        ),
        ("1d4+3", "bludgeoning", "")
    );
    let javelin = &m.strikes[2];
    assert!(javelin.is_ranged);
    assert_eq!(
        (javelin.attack_mod, javelin.traits.clone()),
        (5, vec!["thrown 30 feet".to_string()])
    );
    assert!(m.spells_and_abilities.is_empty());
}

#[test]
fn parses_an_aon_text_copy_with_negative_level() {
    let m = parse_aon_statblock(GOBLIN).unwrap();
    assert_eq!(
        (m.name.as_str(), m.level, m.size.as_str()),
        ("Goblin Warrior", -1, "Small")
    );
    assert_eq!(m.traits, ["goblin", "humanoid"]);
    assert_eq!(m.languages, "Common, Goblin");
    assert_eq!(m.skills.last().unwrap(), &("Stealth".to_string(), 5));
    assert_eq!(m.items, "dogslicer, leather armor, shortbow (10 arrows)");
    assert_eq!((m.ac, m.reflex, m.max_hp), (16, 7, 6));
    let dogslicer = &m.strikes[0];
    assert_eq!((dogslicer.attack_mod, dogslicer.actions), (7, 1));
    assert_eq!(dogslicer.traits, ["agile", "backstabber", "finesse"]);
    let bow = &m.strikes[1];
    assert!(bow.is_ranged);
    assert_eq!(bow.traits, ["deadly d10", "range increment 60 feet"]);
    assert_eq!(bow.damage_expr, "1d6");
    assert!(m.spells_and_abilities.is_empty());
}

#[test]
fn parses_defenses_spells_and_abilities() {
    let m = parse_aon_statblock(EMBER_WITCH).unwrap();
    assert_eq!((m.name.as_str(), m.level), ("Ember Witch", 4));
    assert_eq!(m.traits, ["uncommon", "ce", "human", "humanoid"]);
    assert_eq!(m.save_notes, "+1 status to all saves vs. magic");
    assert_eq!((m.max_hp, m.hp_notes.as_str()), (50, "void healing"));
    assert_eq!(m.immunities, ["fire", "sleep"]);
    assert_eq!(m.weaknesses, ["cold 5"]);
    assert_eq!(m.resistances, ["physical 3 (except silver)", "poison 5"]);
    assert_eq!(m.speed, "25 feet, fly 20 feet");

    let staff = &m.strikes[0];
    assert_eq!(staff.traits, ["two-hand d8"]);
    assert_eq!(
        (
            staff.damage_expr.as_str(),
            staff.damage_type.as_str(),
            staff.damage_extra.as_str()
        ),
        ("1d4+2", "bludgeoning", "plus 1d6 fire")
    );
    let bolt = &m.strikes[1];
    assert_eq!(
        (bolt.is_ranged, bolt.actions, bolt.attack_mod),
        (true, 2, 12)
    );
    assert_eq!(
        (bolt.damage_expr.as_str(), bolt.damage_type.as_str()),
        ("2d8", "fire")
    );

    let a = &m.spells_and_abilities;
    assert_eq!(a.len(), 3);
    assert_eq!(
        (a[0].name.as_str(), a[0].actions.as_deref()),
        ("Ember Ward", Some("reaction"))
    );
    assert_eq!(a[0].traits, ["fire", "occult"]);
    assert!(a[0].text.starts_with("Trigger A creature within 15 feet"));
    assert_eq!(
        (a[1].name.as_str(), a[1].actions.as_deref()),
        ("Occult Spontaneous Spells", None)
    );
    assert!(a[1].text.starts_with("DC 21, attack +13; 2nd (3 slots)"));
    assert_eq!(
        (a[2].name.as_str(), a[2].actions.as_deref()),
        ("Hex Fire", Some("2"))
    );
    assert_eq!(a[2].traits, ["concentrate", "fire"]);
    assert_eq!(
        a[2].text,
        "The witch hurls cursed flame. The target takes 3d6 fire damage (DC 21 basic Reflex save)."
    );
}

#[test]
fn parses_glyph_actions_and_effects() {
    let m = parse_aon_statblock(BEAST).unwrap();
    assert_eq!(
        (m.size.as_str(), m.traits.clone()),
        ("Large", vec!["animal".to_string()])
    );
    assert_eq!(m.senses, "scent (imprecise) 30 feet");
    assert_eq!(m.abilities, [0; 6]);
    assert!(m.skills.is_empty());
    let jaws = &m.strikes[0];
    assert!(jaws.traits.is_empty());
    assert_eq!(
        (jaws.damage_expr.as_str(), jaws.damage_extra.as_str()),
        ("1d10+5", "plus Grab")
    );
    let tail = &m.strikes[1];
    assert_eq!(tail.traits, ["agile", "reach 10 feet"]);
    assert_eq!(
        (
            tail.damage_expr.as_str(),
            tail.damage_type.as_str(),
            tail.damage_extra.as_str()
        ),
        ("", "", "Push 5 feet")
    );
}

#[test]
fn rejects_incomplete_blocks() {
    assert!(parse_aon_statblock("").is_err());
    assert!(parse_aon_statblock("Just some text\nAC 10\nHP 5").is_err());
    assert!(
        parse_aon_statblock(&ORC_BRUTE.replace("AC 15; Fort +8, Ref +5, Will +4\n", "")).is_err()
    );
    assert!(parse_aon_statblock(&ORC_BRUTE.replace("HP 15\n", "")).is_err());
    assert!(parse_aon_statblock(&ORC_BRUTE.replace("Perception +5; darkvision\n", "")).is_err());
}

#[test]
fn json_round_trip() {
    for text in [ORC_BRUTE, GOBLIN, EMBER_WITCH, BEAST] {
        let m = parse_aon_statblock(text).unwrap();
        let json = JsonValue::parse(&m.to_json().to_json_pretty()).unwrap();
        assert_eq!(json.get_str("format"), Some("owlbearbuilder-monster"));
        assert_eq!(MonsterStatBlock::from_json(&json).unwrap(), m);
    }
    let minimal =
        JsonValue::parse(r#"{"name": "Wolf", "level": 1, "ac": 15, "max_hp": 24}"#).unwrap();
    let wolf = MonsterStatBlock::from_json(&minimal).unwrap();
    assert_eq!((wolf.id.as_str(), wolf.size.as_str()), ("wolf", "Medium"));
    assert!(MonsterStatBlock::from_json(
        &JsonValue::parse(r#"{"name": "Wolf", "level": 1, "ac": 15}"#).unwrap()
    )
    .is_err());
}
