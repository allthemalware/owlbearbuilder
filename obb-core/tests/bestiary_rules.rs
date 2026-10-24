// SPDX-License-Identifier: GPL-3.0-or-later

use obb_core::bestiary::*;
use obb_core::character::Save;
use obb_core::conditions::Condition;
use obb_core::encounter::{Combatant, EncounterTracker};
use obb_core::json::JsonValue;
use obb_core::monster::{parse_aon_statblock, MonsterStatBlock};

fn orc() -> MonsterStatBlock {
    parse_aon_statblock(
        "ORC BRUTE CREATURE 0
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
",
    )
    .unwrap()
}

fn caster() -> MonsterStatBlock {
    parse_aon_statblock(
        "Cult Adept Creature 3
Medium Human Humanoid
Perception +9
AC 18; Fort +7, Ref +8, Will +11
HP 40
Speed 25 feet
Melee [one-action] dagger +8 (agile, finesse), Damage 1d4+1 piercing
Divine Prepared Spells DC 20, attack +12; 2nd harm; Cantrips (2nd) void warp
Unholy Glare [two-actions] (divine) One creature must succeed at a DC 20 Will save.
",
    )
    .unwrap()
}

fn at_level(level: i32, hp: i32) -> MonsterStatBlock {
    let mut m = orc();
    m.level = level;
    m.max_hp = hp;
    m
}

#[test]
fn hp_brackets() {
    assert_eq!(
        [-1, 0, 1, 2, 4, 5, 19, 20, 24].map(elite_hp_bonus),
        [10, 10, 10, 15, 15, 20, 20, 30, 30]
    );
    assert_eq!(
        [-1, 0, 1, 2, 3, 5, 6, 20, 21].map(weak_hp_penalty),
        [10, 10, 10, 10, 15, 15, 20, 20, 30]
    );
}

#[test]
fn level_shifts_across_the_table() {
    let cases = [
        (-1, 2, 1, -8),
        (1, 50, 2, 40),
        (3, 50, 4, 35),
        (7, 100, 8, 80),
        (21, 400, 22, 370),
    ];
    for (base, hp, elite_level, weak_hp) in cases {
        let m = at_level(base, hp);
        let e = apply_variant(&m, CreatureVariant::Elite);
        let w = apply_variant(&m, CreatureVariant::Weak);
        assert_eq!(e.level, elite_level, "elite level from {base}");
        assert_eq!(e.max_hp, hp + elite_hp_bonus(base), "elite hp at {base}");
        assert_eq!(w.max_hp, weak_hp.max(1), "weak hp at {base}");
    }
    assert_eq!(
        apply_variant(&at_level(0, 15), CreatureVariant::Elite).level,
        2
    );
    assert_eq!(
        apply_variant(&at_level(1, 15), CreatureVariant::Weak).level,
        -1
    );
    assert_eq!(
        apply_variant(&at_level(0, 15), CreatureVariant::Weak).level,
        -1
    );
    assert_eq!(
        apply_variant(&at_level(3, 15), CreatureVariant::Weak).level,
        2
    );
    assert_eq!(
        apply_variant(&at_level(1, 6), CreatureVariant::Weak).max_hp,
        1
    );
    assert_eq!(apply_variant(&orc(), CreatureVariant::Normal), orc());
}

#[test]
fn elite_numbers() {
    let e = apply_variant(&orc(), CreatureVariant::Elite);
    assert_eq!((e.name.as_str(), e.id.as_str()), ("Orc Brute", "orc-brute"));
    assert_eq!(
        (
            e.level,
            e.ac,
            e.fort,
            e.reflex,
            e.will,
            e.perception,
            e.max_hp
        ),
        (2, 17, 10, 7, 6, 7, 25)
    );
    assert_eq!(
        e.skills,
        [
            ("Athletics".to_string(), 8),
            ("Intimidation".to_string(), 6)
        ]
    );
    let attacks: Vec<i32> = e.strikes.iter().map(|s| s.attack_mod).collect();
    assert_eq!(attacks, [9, 9, 7]);
    let damage: Vec<&str> = e.strikes.iter().map(|s| s.damage_expr.as_str()).collect();
    assert_eq!(damage, ["1d4+5", "1d10+5", "1d6+5"]);
}

#[test]
fn weak_numbers_and_ability_text() {
    let w = apply_variant(&orc(), CreatureVariant::Weak);
    assert_eq!((w.level, w.ac, w.perception, w.max_hp), (-1, 13, 3, 5));
    assert_eq!(w.strikes[0].damage_expr, "1d4+1");
    let c = apply_variant(&caster(), CreatureVariant::Weak);
    assert_eq!(c.strikes[0].damage_expr, "1d4-1");
    assert!(c.spells_and_abilities[0]
        .text
        .starts_with("DC 18, attack +10; 2nd harm"));
    assert!(c.spells_and_abilities[1].text.contains("DC 18 Will save"));
    let e = apply_variant(&caster(), CreatureVariant::Elite);
    assert!(e.spells_and_abilities[0]
        .text
        .starts_with("DC 22, attack +14;"));
    assert_eq!(e.strikes[0].damage_expr, "1d4+3");
}

#[test]
fn variants_parse() {
    assert_eq!(
        CreatureVariant::parse("ELITE"),
        Some(CreatureVariant::Elite)
    );
    assert_eq!(CreatureVariant::parse("nope"), None);
    assert_eq!(CreatureVariant::Weak.label(), "Weak");
    assert_eq!(CreatureVariant::Normal.id(), "normal");
}

#[test]
fn instances_keep_damage_across_variant_changes() {
    let mut m = MonsterInstance::new(orc(), CreatureVariant::Normal, "orc-brute-1", "Orc Brute 1");
    assert_eq!((m.current_hp, m.temp_hp), (15, 0));
    m.apply_damage(5);
    m.set_variant(CreatureVariant::Elite);
    assert_eq!((m.current_hp, m.effective_max_hp()), (20, 25));
    m.set_variant(CreatureVariant::Weak);
    assert_eq!(m.current_hp, 0);
    m.heal(100);
    assert_eq!(m.current_hp, 5);
    m.temp_hp = 3;
    m.apply_damage(4);
    assert_eq!((m.temp_hp, m.current_hp), (0, 4));
    m.apply_damage(-2);
    assert_eq!(m.current_hp, 4);
}

#[test]
fn instance_modifiers_apply_conditions() {
    let mut m = MonsterInstance::new(orc(), CreatureVariant::Normal, "orc-brute-1", "Orc Brute 1");
    m.conditions = vec![Condition::Frightened(1), Condition::OffGuard];
    assert_eq!(m.effective_ac(), 12);
    assert_eq!(m.skill_modifier("athletics"), Some(5));
    assert_eq!(m.skill_modifier("Stealth"), None);
    assert_eq!(m.perception_modifier(), 4);
    m.conditions = vec![Condition::Drained(2)];
    assert_eq!(m.save_modifier(Save::Fortitude), 6);
    assert_eq!(m.save_modifier(Save::Reflex), 5);
    assert_eq!(m.effective_max_hp(), 13);
    m.conditions = vec![Condition::Enfeebled(2)];
    assert_eq!(m.strike_attack_modifier(1, 1), Some(0));
    assert_eq!(m.strike_attack_modifier(0, 2), Some(-3));
    assert_eq!(m.strike_damage_expr(1).as_deref(), Some("1d10+1"));
    assert_eq!(m.strike_damage_expr(2).as_deref(), Some("1d6+1"));
    assert_eq!(m.strike_attack_modifier(2, 0), Some(5));
    m.conditions = vec![Condition::Clumsy(1), Condition::Frightened(1)];
    assert_eq!(m.strike_attack_modifier(2, 0), Some(4));
    assert_eq!(m.strike_damage_expr(1).as_deref(), Some("1d10+3"));
    assert_eq!(m.strike_attack_modifier(7, 0), None);
    m.variant = CreatureVariant::Elite;
    m.conditions = vec![Condition::Enfeebled(1)];
    assert_eq!(m.strike_damage_expr(1).as_deref(), Some("1d10+4"));
    let adept = MonsterInstance::new(caster(), CreatureVariant::Normal, "adept", "Adept");
    let mut clumsy = adept.clone();
    clumsy.conditions = vec![Condition::Clumsy(2)];
    assert_eq!(clumsy.strike_attack_modifier(0, 0), Some(6));
    assert_eq!(
        strike_ability(&adept.base.strikes[0]),
        obb_core::character::Ability::Dex
    );
}

#[test]
fn token_metadata_and_json() {
    let mut m = MonsterInstance::new(orc(), CreatureVariant::Elite, "orc-brute-1", "Orc Brute 1");
    m.conditions = vec![Condition::Frightened(2)];
    m.apply_damage(6);
    assert_eq!(
        m.to_obr_token_metadata().to_json_string(),
        r#"{"kind":"monster","id":"orc-brute-1","name":"Orc Brute 1","hp":19,"max_hp":25,"temp_hp":0,"ac":15,"level":2,"variant":"elite","conditions":["Frightened 2"]}"#
    );
    m.linked_token_id = Some("tok-4".into());
    let back =
        MonsterInstance::from_json(&JsonValue::parse(&m.to_json().to_json_pretty()).unwrap())
            .unwrap();
    assert_eq!(back, m);
    assert!(MonsterInstance::from_json(&JsonValue::parse("{}").unwrap()).is_err());
}

#[test]
fn monsters_tick_in_the_initiative_order() {
    let mut orc1 = MonsterInstance::new(orc(), CreatureVariant::Normal, "orc-1", "Orc 1");
    orc1.conditions = vec![Condition::Frightened(2)];
    let mut t = EncounterTracker::new();
    let mut c = Combatant::new("c-orc-1", "orc-1", "Orc 1", false);
    c.initiative = Some(14);
    t.add_combatant(c).unwrap();
    t.start().unwrap();
    t.next_turn(&mut [&mut orc1]);
    assert_eq!(orc1.conditions, vec![Condition::Frightened(1)]);
}
