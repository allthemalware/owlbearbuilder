// SPDX-License-Identifier: GPL-3.0-or-later

use obb_core::character::*;
use obb_core::conditions::Condition;
use obb_core::dice::{evaluate_check, Rng};
use obb_core::encounter::*;
use obb_core::json::JsonValue;

/// Level 5: Str +4, Dex +2, Con +2, Int +0, Wis +1, Cha +0; Perception expert (+10).
fn fighter() -> CharacterSheet {
    let mut c = CharacterSheet::new("Amiri", 5);
    c.abilities = [4, 2, 2, 0, 1, 0];
    c.max_hp = 76;
    c.current_hp = 60;
    c.temp_hp = 3;
    c.ac_base = 22;
    c.class = "Fighter".into();
    c.perception_rank = ProfRank::Expert;
    c.fort_rank = ProfRank::Expert;
    c.reflex_rank = ProfRank::Expert;
    c.will_rank = ProfRank::Trained;
    c.class_dc_rank = ProfRank::Trained;
    c.lores.push(Lore {
        name: "Warfare".into(),
        rank: ProfRank::Trained,
    });
    c
}

/// Level 3: Int +4, Wis +1, Dex +2; Perception trained (+6), Arcana expert (+11).
fn wizard() -> CharacterSheet {
    let mut c = CharacterSheet::new("Ezren", 3);
    c.abilities = [0, 2, 1, 4, 1, 0];
    c.perception_rank = ProfRank::Trained;
    c.skill_ranks[Skill::Arcana as usize] = ProfRank::Expert;
    c
}

fn with_init(id: &str, name: &str, player: bool, init: Option<i32>, modifier: i32) -> Combatant {
    let mut c = Combatant::new(id, id, name, player);
    c.initiative = init;
    c.initiative_mod = modifier;
    c
}

fn order(t: &EncounterTracker) -> Vec<&str> {
    t.combatants
        .iter()
        .map(|c| c.combatant_id.as_str())
        .collect()
}

fn table() -> EncounterTracker {
    let mut t = EncounterTracker::new();
    for c in [
        with_init("amiri", "Amiri", true, Some(20), 7),
        with_init("gob1", "Goblin Warrior", false, Some(20), 2),
        with_init("ezren", "Ezren", true, Some(15), 5),
        with_init("cmdo", "Goblin Commando", false, Some(15), 5),
        with_init("kyra", "Kyra", true, Some(15), 8),
        with_init("zombie", "Zombie", false, None, 0),
    ] {
        t.add_combatant(c).unwrap();
    }
    t
}

#[test]
fn initiative_modifiers() {
    let mut c = fighter();
    assert_eq!(
        initiative_modifier(&c, "perception"),
        Some((10, "Perception (+10)".to_string()))
    );
    assert_eq!(
        initiative_modifier(&c, "Stealth"),
        Some((2, "Stealth (+2)".to_string()))
    );
    assert_eq!(
        initiative_modifier(&c, "warfare"),
        Some((7, "Warfare Lore (+7)".to_string()))
    );
    assert_eq!(initiative_modifier(&c, "basket weaving"), None);
    c.conditions = vec![Condition::Frightened(1), Condition::Stupefied(2)];
    assert_eq!(
        initiative_modifier(&c, "Perception"),
        Some((8, "Perception (+8)".to_string()))
    );
    c.abilities[1] = -4;
    c.conditions.clear();
    assert_eq!(
        initiative_modifier(&c, "stealth"),
        Some((-4, "Stealth (-4)".to_string()))
    );
}

#[test]
fn rolls_initiative_for_a_character() {
    let mut t = EncounterTracker::new();
    t.add_combatant(Combatant::new("amiri", "amiri", "Amiri", true))
        .unwrap();
    let sheet = fighter();
    let (total, roll, text) = t
        .roll_initiative_for_character("amiri", &sheet, "perception", &mut Rng::from_seed(4))
        .unwrap();
    let nat = roll.natural_d20.unwrap() as i32;
    assert_eq!(total, nat + 10);
    assert_eq!(roll.total, total);
    assert_eq!(text, format!("Rolled {nat} + 10 (Perception) = {total}"));
    let c = &t.combatants[0];
    assert_eq!(
        (c.initiative, c.initiative_mod, c.initiative_source.as_str()),
        (Some(total), 10, "Perception (+10)")
    );
    assert!(t
        .roll_initiative_for_character("nobody", &sheet, "perception", &mut Rng::from_seed(4))
        .is_err());
    assert!(t
        .roll_initiative_for_character("amiri", &sheet, "juggling", &mut Rng::from_seed(4))
        .is_err());
}

#[test]
fn sorting_breaks_ties_for_the_gm_then_by_modifier() {
    let mut t = table();
    t.sort_initiative();
    assert_eq!(
        order(&t),
        ["gob1", "amiri", "cmdo", "kyra", "ezren", "zombie"]
    );
    let mut twins = EncounterTracker::new();
    twins
        .add_combatant(with_init("b", "Seoni", true, Some(12), 3))
        .unwrap();
    twins
        .add_combatant(with_init("a", "Merisiel", true, Some(12), 3))
        .unwrap();
    twins.sort_initiative();
    assert_eq!(order(&twins), ["a", "b"]);
    assert!(twins
        .add_combatant(with_init("a", "Again", true, None, 0))
        .is_err());
}

#[test]
fn turns_tick_conditions_and_wrap_rounds() {
    let mut t = EncounterTracker::new();
    t.add_combatant(with_init("goblin-1", "Goblin", false, Some(12), 2))
        .unwrap();
    t.add_combatant(with_init("amiri", "Amiri", true, Some(18), 10))
        .unwrap();
    assert!(EncounterTracker::new().start().is_err());
    let first = t.start().unwrap();
    assert_eq!(
        (
            first.ended,
            first.started.as_deref(),
            first.round,
            first.new_round
        ),
        (None, Some("amiri"), 1, true)
    );
    assert_eq!(t.active_combatant().unwrap().combatant_id, "amiri");

    let mut amiri = fighter();
    amiri.id = "amiri".into();
    amiri.conditions = vec![Condition::Frightened(2), Condition::ShieldRaised(2)];

    let step = t.next_turn(&mut [&mut amiri]);
    assert_eq!(
        (
            step.ended.as_deref(),
            step.started.as_deref(),
            step.round,
            step.new_round
        ),
        (Some("amiri"), Some("goblin-1"), 1, false)
    );
    assert_eq!(
        amiri.conditions,
        vec![Condition::Frightened(1), Condition::ShieldRaised(2)]
    );

    let step = t.next_turn(&mut [&mut amiri]);
    assert_eq!(
        (step.started.as_deref(), step.round, step.new_round),
        (Some("amiri"), 2, true)
    );
    assert_eq!(amiri.conditions, vec![Condition::Frightened(1)]);

    t.next_turn(&mut [&mut amiri]);
    assert!(amiri.conditions.is_empty());

    t.end();
    assert!(!t.active);
    assert_eq!(t.round, 0);
    assert_eq!(t.combatants[0].initiative, Some(18));
    let idle = t.next_turn(&mut [&mut amiri]);
    assert_eq!((idle.ended, idle.started), (None, None));
}

#[test]
fn delay_keeps_the_current_turn() {
    let mut t = table();
    t.start().unwrap();
    assert_eq!(t.active_combatant().unwrap().combatant_id, "gob1");
    t.delay_combatant("ezren", 25).unwrap();
    assert_eq!(
        order(&t),
        ["ezren", "gob1", "amiri", "cmdo", "kyra", "zombie"]
    );
    assert_eq!(t.active_combatant().unwrap().combatant_id, "gob1");
    assert_eq!(t.combatants[0].initiative, Some(25));

    // The acting combatant delaying hands the turn on.
    t.delay_combatant("gob1", 10).unwrap();
    assert_eq!(
        order(&t),
        ["ezren", "amiri", "cmdo", "kyra", "gob1", "zombie"]
    );
    assert_eq!(t.active_combatant().unwrap().combatant_id, "amiri");
    assert!(t.delay_combatant("nobody", 3).is_err());
}

#[test]
fn joining_and_leaving_mid_encounter() {
    let mut t = table();
    t.start().unwrap();
    t.next_turn(&mut []);
    assert_eq!(t.active_combatant().unwrap().combatant_id, "amiri");
    t.add_combatant(with_init("valeros", "Valeros", true, Some(22), 9))
        .unwrap();
    assert_eq!(order(&t)[0], "valeros");
    assert_eq!(t.active_combatant().unwrap().combatant_id, "amiri");
    t.add_combatant(with_init("seelah", "Seelah", true, Some(15), 1))
        .unwrap();
    assert_eq!(
        order(&t),
        ["valeros", "gob1", "amiri", "cmdo", "kyra", "ezren", "seelah", "zombie"]
    );

    let gone = t.remove_combatant("amiri").unwrap();
    assert_eq!(gone.name, "Amiri");
    assert_eq!(t.active_combatant().unwrap().combatant_id, "cmdo");
    assert!(t.remove_combatant("amiri").is_none());
}

#[test]
fn tracker_json_round_trip() {
    let mut t = table();
    t.combatants[0].token_id = Some("tok-9".into());
    t.combatants[0].initiative_source = "Stealth (+7)".into();
    t.start().unwrap();
    t.next_turn(&mut []);
    let json = JsonValue::parse(&t.to_json().to_json_string()).unwrap();
    assert_eq!(json.get_i64("round"), Some(1));
    assert_eq!(EncounterTracker::from_json(&json).unwrap(), t);
    assert!(EncounterTracker::from_json(&JsonValue::parse("[]").unwrap()).is_err());
}

#[test]
fn party_overview_applies_conditions() {
    let mut a = fighter();
    a.conditions = vec![Condition::Frightened(1), Condition::Drained(1)];
    a.linked_token_id = Some("tok-1".into());
    let rows = build_party_overview(&[a, wizard()]);
    assert_eq!(rows.len(), 2);
    let r = &rows[0];
    assert_eq!(
        (r.id.as_str(), r.name.as_str(), r.class.as_str(), r.level),
        ("amiri", "Amiri", "Fighter", 5)
    );
    assert_eq!((r.current_hp, r.max_hp, r.temp_hp), (60, 71, 3));
    assert_eq!((r.ac, r.fort, r.reflex, r.will), (21, 10, 10, 7));
    assert_eq!((r.perception, r.perception_dc, r.stealth_dc), (9, 19, 11));
    assert_eq!(r.class_dc, 20);
    assert_eq!((r.speed, r.hero_points), (25, 1));
    assert_eq!(r.conditions, ["Frightened 1", "Drained 1"]);
    assert_eq!(r.linked_token_id.as_deref(), Some("tok-1"));
    assert_eq!(rows[1].perception, 6);
    let json = r.to_json();
    assert_eq!(json.get_i64("perception_dc"), Some(19));
    assert_eq!(json.get_str("linked_token_id"), Some("tok-1"));
}

#[test]
fn party_secret_checks() {
    let party = [fighter(), wizard()];
    let out =
        roll_party_secret_check(&party, "Perception", Some(18), &mut Rng::from_seed(8)).unwrap();
    assert_eq!(out.len(), 2);
    assert_eq!(
        (out[0].id.as_str(), out[0].modifier, out[1].modifier),
        ("amiri", 10, 6)
    );
    for o in &out {
        assert_eq!(o.total, o.roll.total);
        assert_eq!(o.total, o.roll.natural_d20.unwrap() as i32 + o.modifier);
        assert_eq!(
            o.degree,
            Some(evaluate_check(o.total, 18, o.roll.natural_d20))
        );
    }
    let arcana = roll_party_secret_check(&party, "arcana", None, &mut Rng::from_seed(8)).unwrap();
    assert_eq!((arcana[0].modifier, arcana[1].modifier), (0, 11));
    assert!(arcana.iter().all(|o| o.degree.is_none()));
    let will = roll_party_secret_check(&party, "will", None, &mut Rng::from_seed(1)).unwrap();
    assert_eq!(will[0].modifier, 8);
    let lore =
        roll_party_secret_check(&party, "Warfare Lore", None, &mut Rng::from_seed(1)).unwrap();
    assert_eq!((lore[0].modifier, lore[1].modifier), (7, 4));
    assert!(roll_party_secret_check(&party, "juggling", None, &mut Rng::from_seed(1)).is_err());
    let same =
        roll_party_secret_check(&party, "Perception", Some(18), &mut Rng::from_seed(8)).unwrap();
    assert_eq!(same, out);
}
