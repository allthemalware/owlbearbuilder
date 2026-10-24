// SPDX-License-Identifier: GPL-3.0-or-later

use obb_core::aoe::SpellArea;
use obb_core::character::{Ability, CharacterSheet, ProfRank, SpellcastingEntry};
use obb_core::spells::*;

#[test]
fn cantrips_heighten_to_half_level() {
    assert_eq!(
        [0, 1, 2, 3, 5, 19, 20, 25].map(cantrip_rank),
        [1, 1, 1, 2, 3, 10, 10, 10]
    );
}

#[test]
fn heightens_damage() {
    let fireball = lookup_common_spell("fireball", 3).unwrap();
    assert_eq!(
        (
            fireball.name.as_str(),
            fireball.rank,
            fireball.base_rank,
            fireball.is_cantrip
        ),
        ("Fireball", 3, 3, false)
    );
    assert_eq!(fireball.damage_expr.as_deref(), Some("6d6"));
    assert_eq!(fireball.damage_type.as_deref(), Some("fire"));
    assert_eq!(fireball.area, Some(SpellArea::Burst { radius_ft: 20 }));
    assert_eq!(
        (fireball.range_ft, fireball.defense.as_deref()),
        (Some(500), Some("basic Reflex"))
    );
    assert_eq!(fireball.traditions, ["arcane", "primal"]);
    assert_eq!(
        lookup_common_spell("Fireball", 5)
            .unwrap()
            .damage_expr
            .as_deref(),
        Some("10d6")
    );
    assert_eq!(lookup_common_spell("Fireball", 1).unwrap().rank, 3);
    let breath = lookup_common_spell("Breathe Fire", 3).unwrap();
    assert_eq!(breath.damage_expr.as_deref(), Some("6d6"));
    assert_eq!(
        breath.area,
        Some(SpellArea::Cone {
            length_ft: 15,
            direction_deg: 0.0
        })
    );
    let arc = lookup_common_spell("ELECTRIC ARC", 3).unwrap();
    assert!(arc.is_cantrip);
    assert_eq!(
        (arc.rank, arc.base_rank, arc.damage_expr.as_deref()),
        (3, 0, Some("4d4"))
    );
    assert_eq!(
        lookup_common_spell("Electric Arc", 1)
            .unwrap()
            .damage_expr
            .as_deref(),
        Some("2d4")
    );
    assert!(lookup_common_spell("Ward of Whispers", 1).is_none());
    assert!(spell_data("breathe fire").is_some());
}

#[test]
fn spellbook_fills_in_what_the_export_leaves_out() {
    let mut c = CharacterSheet::new("Ezren", 5);
    c.spellcasting.push(SpellcastingEntry {
        name: "Wizard".into(),
        tradition: "arcane".into(),
        kind: "prepared".into(),
        ability: Ability::Int,
        rank: ProfRank::Trained,
        focus_points: 0,
        spells: vec![
            (0, vec!["Electric Arc".into(), "Ward of Whispers".into()]),
            (1, vec!["Breathe Fire".into()]),
            (3, vec!["Fireball".into()]),
        ],
    });
    let book = spellbook(&c);
    let names: Vec<&str> = book.iter().map(|k| k.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Electric Arc",
            "Ward of Whispers",
            "Breathe Fire",
            "Fireball"
        ]
    );
    assert_eq!(
        book[0].details.as_ref().unwrap().damage_expr.as_deref(),
        Some("4d4")
    );
    assert_eq!((book[0].entry, book[0].listed_rank), (0, 0));
    assert!(book[1].details.is_none());
    let fireball = book[3].details.as_ref().unwrap();
    assert_eq!(fireball.area, Some(SpellArea::Burst { radius_ft: 20 }));
    assert_eq!(fireball.damage_expr.as_deref(), Some("6d6"));
    let json = fireball.to_json();
    assert_eq!(json.get_str("area_label"), Some("20-foot burst"));
    assert_eq!(json.get_str("damage_expr"), Some("6d6"));
}
