// SPDX-License-Identifier: GPL-3.0-or-later

use obb_core::dice::DegreeOfSuccess::{CriticalFailure, CriticalSuccess, Failure, Success};
use obb_core::dice::*;

#[test]
fn degree_boundaries_without_natural_die() {
    let dc = 20;
    for (total, want) in [
        (30, CriticalSuccess),
        (29, Success),
        (20, Success),
        (19, Failure),
        (11, Failure),
        (10, CriticalFailure),
        (-5, CriticalFailure),
    ] {
        assert_eq!(evaluate_check(total, dc, None), want, "total {total}");
    }
}

#[test]
fn natural_die_shifts_after_the_comparison() {
    // A natural 20 that still misses by 10 is only a failure, not a success.
    assert_eq!(evaluate_check(10, 20, Some(20)), Failure);
    assert_eq!(evaluate_check(19, 20, Some(20)), Success);
    assert_eq!(evaluate_check(25, 20, Some(20)), CriticalSuccess);
    assert_eq!(evaluate_check(35, 20, Some(20)), CriticalSuccess);
    // A natural 1 that still beats the DC by 10 is only a success.
    assert_eq!(evaluate_check(30, 20, Some(1)), Success);
    assert_eq!(evaluate_check(20, 20, Some(1)), Failure);
    assert_eq!(evaluate_check(5, 20, Some(1)), CriticalFailure);
    // Other faces change nothing.
    assert_eq!(evaluate_check(20, 20, Some(19)), Success);
}

#[test]
fn degree_helpers() {
    assert_eq!(CriticalSuccess.step_up(), CriticalSuccess);
    assert_eq!(CriticalFailure.step_down(), CriticalFailure);
    assert_eq!(Failure.step_up(), Success);
    assert_eq!(Success.step_down(), Failure);
    assert!(CriticalFailure < Failure && Failure < Success && Success < CriticalSuccess);
    assert_eq!(CriticalSuccess.label(), "Critical Success");
    assert_eq!(CriticalFailure.id(), "critical_failure");
}

#[test]
fn multiple_attack_penalty() {
    assert_eq!(
        [0, 1, 2, 3].map(|i| map_penalty(i, false)),
        [0, -5, -10, -10]
    );
    assert_eq!([0, 1, 2, 3].map(|i| map_penalty(i, true)), [0, -4, -8, -8]);
}

#[test]
fn parses_and_canonicalises_expressions() {
    for (text, canon) in [
        ("1d20+14", "1d20+14"),
        (" d20 + 4 + 3 ", "1d20+7"),
        ("-2+1d6", "1d6-2"),
        ("3d10+1d6+7", "3d10+1d6+7"),
        ("1D20-2", "1d20-2"),
        ("5", "5"),
        ("1d20-1d4", "1d20-1d4"),
    ] {
        assert_eq!(DiceExpr::parse(text).unwrap().to_string(), canon, "{text}");
    }
    for bad in [
        "",
        "d",
        "1d",
        "0d6",
        "1d0",
        "2x6",
        "1d20+",
        "1d20++2",
        "101d6",
        "1d1001",
        "99999999999",
    ] {
        assert!(DiceExpr::parse(bad).is_err(), "accepted {bad:?}");
    }
}

#[test]
fn rolls_are_seeded_and_in_range() {
    let a = roll_expr("3d10+1d6+7", &mut Rng::from_seed(42)).unwrap();
    let b = roll_expr("3d10+1d6+7", &mut Rng::from_seed(42)).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.terms.len(), 2);
    assert_eq!(a.terms[0].rolls.len(), 3);
    assert!(a
        .terms
        .iter()
        .flat_map(|t| t.rolls.iter().map(move |r| (*r, t.sides)))
        .all(|(r, s)| r >= 1 && r <= s));
    assert_eq!(a.flat_modifier, 7);
    assert_eq!(a.total, a.terms.iter().map(|t| t.subtotal).sum::<i32>() + 7);
    assert_eq!(a.natural_d20, None);

    let mut rng = Rng::from_seed(7);
    let mut seen = [false; 21];
    for _ in 0..2000 {
        let r = roll_expr("1d20-2", &mut rng).unwrap();
        let nat = r.natural_d20.unwrap();
        assert_eq!(r.total, nat as i32 - 2);
        seen[nat as usize] = true;
    }
    assert!(
        seen[1..].iter().all(|s| *s),
        "every face of a d20 should come up in 2000 rolls"
    );
}

#[test]
fn subtracted_dice_count_negative() {
    let r = roll_expr("1d20-1d4", &mut Rng::from_seed(3)).unwrap();
    assert_eq!(r.terms[1].sign, -1);
    assert_eq!(r.terms[1].subtotal, -(r.terms[1].rolls[0] as i32));
    assert_eq!(r.total, r.terms[0].subtotal + r.terms[1].subtotal);
}

#[test]
fn natural_d20_only_for_single_positive_d20() {
    let mut rng = Rng::from_seed(1);
    assert!(roll_expr("d20", &mut rng).unwrap().natural_d20.is_some());
    assert!(roll_expr("1d20+1d4", &mut rng)
        .unwrap()
        .natural_d20
        .is_none());
    assert!(roll_expr("2d20", &mut rng).unwrap().natural_d20.is_none());
    assert!(roll_expr("-1d20", &mut rng).unwrap().natural_d20.is_none());
}

#[test]
fn os_rng_produces_different_streams() {
    let a: Vec<u64> = (0..4)
        .map({
            let mut r = Rng::from_os();
            move |_| r.next_u64()
        })
        .collect();
    let b: Vec<u64> = (0..4)
        .map({
            let mut r = Rng::from_os();
            move |_| r.next_u64()
        })
        .collect();
    assert_ne!(a, b);
}

fn longsword(dice_count: u32) -> StrikeDamage {
    StrikeDamage {
        dice_count,
        die_sides: 8,
        flat_bonus: 4,
        deadly: None,
        fatal: None,
    }
}

#[test]
fn normal_hit_and_plain_crit() {
    let mut rng = Rng::from_seed(11);
    let hit = longsword(2).roll(false, &mut rng);
    assert_eq!(hit.weapon_rolls.len(), 2);
    assert!(hit.extra_rolls.is_empty());
    assert_eq!(hit.total, hit.weapon_rolls.iter().sum::<u32>() as i32 + 4);
    assert!(!hit.critical);

    let crit = longsword(2).roll(true, &mut rng);
    assert_eq!(
        crit.total,
        2 * (crit.weapon_rolls.iter().sum::<u32>() as i32 + 4)
    );
    assert!(crit.critical);
}

#[test]
fn deadly_adds_undoubled_dice_by_striking_grade() {
    assert_eq!([1, 2, 3, 4, 5].map(deadly_dice_count), [1, 1, 2, 3, 3]);
    let rapier = StrikeDamage {
        dice_count: 3,
        die_sides: 6,
        flat_bonus: 2,
        deadly: Some(8),
        fatal: None,
    };
    let mut rng = Rng::from_seed(5);
    let hit = rapier.roll(false, &mut rng);
    assert!(hit.extra_rolls.is_empty());
    let crit = rapier.roll(true, &mut rng);
    assert_eq!(crit.extra_rolls.len(), 2);
    assert!(crit.extra_rolls.iter().all(|r| (1..=8).contains(r)));
    let weapon: i32 = crit.weapon_rolls.iter().sum::<u32>() as i32;
    let extra: i32 = crit.extra_rolls.iter().sum::<u32>() as i32;
    assert_eq!(crit.total, 2 * (weapon + 2) + extra);
}

#[test]
fn fatal_upsizes_weapon_dice_and_adds_one() {
    let pick = StrikeDamage {
        dice_count: 2,
        die_sides: 6,
        flat_bonus: 4,
        deadly: None,
        fatal: Some(10),
    };
    let mut rng = Rng::from_seed(9);
    let hit = pick.roll(false, &mut rng);
    assert!(hit.weapon_rolls.iter().all(|r| (1..=6).contains(r)));
    for _ in 0..50 {
        let crit = pick.roll(true, &mut rng);
        assert_eq!(crit.weapon_rolls.len(), 2);
        assert_eq!(crit.extra_rolls.len(), 1);
        assert!(crit
            .weapon_rolls
            .iter()
            .chain(&crit.extra_rolls)
            .all(|r| (1..=10).contains(r)));
        let weapon: i32 = crit.weapon_rolls.iter().sum::<u32>() as i32;
        assert_eq!(crit.total, 2 * (weapon + 4) + crit.extra_rolls[0] as i32);
    }
}

#[test]
fn damage_is_at_least_one() {
    let weak = StrikeDamage {
        dice_count: 1,
        die_sides: 4,
        flat_bonus: -10,
        deadly: None,
        fatal: None,
    };
    let mut rng = Rng::from_seed(2);
    assert_eq!(weak.roll(false, &mut rng).total, 1);
    assert_eq!(weak.roll(true, &mut rng).total, 1);
}
