// SPDX-License-Identifier: GPL-3.0-or-later

use obb_core::aoe::*;

fn burst(r: u32) -> SpellArea {
    SpellArea::Burst { radius_ft: r }
}

#[test]
fn diagonal_distance_alternates() {
    assert_eq!(pf2e_distance_ft(0, 0), 0);
    assert_eq!(pf2e_distance_ft(1, 1), 5);
    assert_eq!(pf2e_distance_ft(2, 2), 15);
    assert_eq!(pf2e_distance_ft(3, 3), 20);
    assert_eq!(pf2e_distance_ft(4, 4), 30);
    assert_eq!(pf2e_distance_ft(-3, 1), 15);
    assert_eq!(pf2e_distance_ft(6, 0), 30);
    assert_eq!(pf2e_distance_ft(2, 5), 30);
}

#[test]
fn parses_area_text() {
    assert_eq!(
        parse_area_description("Area 20-foot burst"),
        Some(burst(20))
    );
    assert_eq!(
        parse_area_description("10-foot radius burst centered on you"),
        Some(burst(10))
    );
    assert_eq!(parse_area_description("a 20 ft. burst"), Some(burst(20)));
    assert_eq!(parse_area_description("20-ft burst"), Some(burst(20)));
    assert_eq!(
        parse_area_description("30-foot Emanation"),
        Some(SpellArea::Emanation { radius_ft: 30 })
    );
    assert_eq!(
        parse_area_description("Area 15-foot cone"),
        Some(SpellArea::Cone {
            length_ft: 15,
            direction_deg: 0.0
        })
    );
    assert_eq!(
        parse_area_description("60-foot line"),
        Some(SpellArea::Line {
            length_ft: 60,
            width_ft: 5,
            direction_deg: 0.0
        })
    );
    assert_eq!(
        parse_area_description("120-foot line, 10 feet wide"),
        Some(SpellArea::Line {
            length_ft: 120,
            width_ft: 10,
            direction_deg: 0.0
        })
    );
    assert_eq!(
        parse_area_description("120-foot line, 10-foot-wide"),
        Some(SpellArea::Line {
            length_ft: 120,
            width_ft: 10,
            direction_deg: 0.0
        })
    );
    assert_eq!(
        parse_area_description("Range 30 feet; Targets 1 creature"),
        None
    );
    assert_eq!(parse_area_description("0-foot burst"), None);
}

#[test]
fn labels_and_json() {
    assert_eq!(burst(20).label(), "20-foot burst");
    let wide = SpellArea::Line {
        length_ft: 60,
        width_ft: 10,
        direction_deg: 90.0,
    };
    assert_eq!(wide.label(), "60-foot line, 10 feet wide");
    assert_eq!(
        SpellArea::Cone {
            length_ft: 15,
            direction_deg: 45.0
        }
        .label(),
        "15-foot cone"
    );
    for a in [
        burst(5),
        SpellArea::Emanation { radius_ft: 10 },
        SpellArea::Cone {
            length_ft: 30,
            direction_deg: 135.0,
        },
        wide,
    ] {
        assert_eq!(SpellArea::from_json(&a.to_json()), Ok(a));
    }
}

#[test]
fn snaps_directions() {
    assert_eq!(
        [0.0, 22.4, 22.5, 44.0, 100.0, 359.0, -45.0, 405.0, 337.6].map(snap_direction),
        [0, 0, 45, 45, 90, 0, 315, 45, 0]
    );
}

#[test]
fn burst_sizes() {
    let counts: Vec<usize> = [5, 10, 15, 20, 30]
        .iter()
        .map(|r| affected_squares(&burst(*r), 1).len())
        .collect();
    assert_eq!(counts[..4], [4, 16, 32, 52]);
    let five = affected_squares(&burst(5), 1);
    assert_eq!(five, [(-1, -1), (0, -1), (-1, 0), (0, 0)]);
    let fifteen = affected_squares(&burst(15), 1);
    // 6×6 from (-3,-3) to (2,2) less its four corners.
    for (x, y) in [(-3, -3), (2, -3), (-3, 2), (2, 2)] {
        assert!(!fifteen.contains(&(x, y)), "corner {x},{y}");
    }
    assert!(fifteen.contains(&(2, 1)) && fifteen.contains(&(-3, -2)));
    let twenty = affected_squares(&burst(20), 1);
    assert!(twenty.contains(&(3, 1)) && twenty.contains(&(-3, 2)) && !twenty.contains(&(3, 2)));
    let mut sorted = twenty.clone();
    sorted.sort_by_key(|&(x, y)| (y, x));
    sorted.dedup();
    assert_eq!(sorted, twenty);
}

#[test]
fn emanation_sizes() {
    let em = |r, size| affected_squares(&SpellArea::Emanation { radius_ft: r }, size);
    assert_eq!(em(5, 1).len(), 9);
    let ten = em(10, 1);
    assert_eq!(ten.len(), 21);
    assert!(
        ten.contains(&(0, 0))
            && ten.contains(&(2, 1))
            && !ten.contains(&(2, 2))
            && !ten.contains(&(-2, -2))
    );
    assert_eq!(em(5, 2).len(), 16);
    assert!(em(5, 2).contains(&(2, -1)));
    assert_eq!(em(10, 0), ten);
}

#[test]
fn cone_shapes() {
    let east = affected_squares(
        &SpellArea::Cone {
            length_ft: 15,
            direction_deg: 0.0,
        },
        1,
    );
    assert_eq!(
        east,
        [(2, -1), (3, -1), (1, 0), (2, 0), (3, 0), (2, 1), (3, 1)]
    );
    let north = affected_squares(
        &SpellArea::Cone {
            length_ft: 15,
            direction_deg: 268.0,
        },
        1,
    );
    assert_eq!(north.len(), 7);
    assert!(north.contains(&(0, -1)) && north.contains(&(-1, -3)));
    let north_east = affected_squares(
        &SpellArea::Cone {
            length_ft: 15,
            direction_deg: 315.0,
        },
        1,
    );
    let mut want: Vec<(i32, i32)> = [
        (0, 0),
        (1, 0),
        (2, 0),
        (0, 1),
        (1, 1),
        (2, 1),
        (0, 2),
        (1, 2),
    ]
    .iter()
    .map(|&(dx, dy)| (1 + dx, -1 - dy))
    .collect();
    want.sort_by_key(|&(x, y)| (y, x));
    assert_eq!(north_east, want);
    let large_east = affected_squares(
        &SpellArea::Cone {
            length_ft: 5,
            direction_deg: 0.0,
        },
        2,
    );
    assert_eq!(large_east, [(2, 0), (2, 1)]);
}

#[test]
fn line_shapes() {
    let east = affected_squares(
        &SpellArea::Line {
            length_ft: 30,
            width_ft: 5,
            direction_deg: 0.0,
        },
        1,
    );
    assert_eq!(east, (1..=6).map(|x| (x, 0)).collect::<Vec<_>>());
    let wide = affected_squares(
        &SpellArea::Line {
            length_ft: 10,
            width_ft: 15,
            direction_deg: 180.0,
        },
        1,
    );
    assert_eq!(
        wide,
        [(-2, -1), (-1, -1), (-2, 0), (-1, 0), (-2, 1), (-1, 1)]
    );
    let south_east = affected_squares(
        &SpellArea::Line {
            length_ft: 30,
            width_ft: 5,
            direction_deg: 45.0,
        },
        1,
    );
    assert_eq!(south_east, [(1, 1), (2, 2), (3, 3), (4, 4)]);
    let large_south = affected_squares(
        &SpellArea::Line {
            length_ft: 10,
            width_ft: 5,
            direction_deg: 90.0,
        },
        2,
    );
    assert_eq!(large_south, [(0, 2), (1, 2), (0, 3), (1, 3)]);
}

#[test]
fn owlbear_shapes() {
    let item = compute_obr_aoe_item(&burst(20), 300.0, 450.0, 150.0, 1, "#ff6600", "Fireball");
    assert_eq!(item.get_str("shape_type"), Some("CIRCLE"));
    assert_eq!(item.get("position").unwrap().get_f64("x"), Some(300.0));
    assert_eq!(item.get_f64("width"), Some(1200.0));
    assert_eq!(item.get_f64("height"), Some(1200.0));
    assert_eq!(item.get_f64("fill_opacity"), Some(0.28));
    assert_eq!(item.get_str("fill_color"), Some("#ff6600"));
    assert_eq!(item.get_str("label"), Some("Fireball"));
    assert_eq!(
        item.get("metadata")
            .unwrap()
            .get_bool("com.owlbearbuilder/aoe"),
        Some(true)
    );
    assert_eq!(item.get_array("squares").unwrap().len(), 52);
    // Origin (300, 450) at 150 dpi is intersection (2, 3): the 5-foot burst's cells sit around it.
    let small = compute_obr_aoe_item(&burst(5), 300.0, 450.0, 150.0, 1, "#fff", "");
    let cells: Vec<(i64, i64)> = small
        .get_array("squares")
        .unwrap()
        .iter()
        .map(|c| {
            let c = c.as_array().unwrap();
            (c[0].as_i64().unwrap(), c[1].as_i64().unwrap())
        })
        .collect();
    assert_eq!(cells, [(1, 2), (2, 2), (1, 3), (2, 3)]);

    let aura = compute_obr_aoe_item(
        &SpellArea::Emanation { radius_ft: 10 },
        75.0,
        75.0,
        150.0,
        1,
        "#00f",
        "Aura",
    );
    assert_eq!(aura.get_str("shape_type"), Some("CIRCLE"));
    assert_eq!(aura.get_f64("width"), Some(2.0 * (300.0 + 75.0)));

    let cone = compute_obr_aoe_item(
        &SpellArea::Cone {
            length_ft: 15,
            direction_deg: 0.0,
        },
        75.0,
        75.0,
        150.0,
        1,
        "#f00",
        "Breathe Fire",
    );
    assert_eq!(cone.get_str("shape_type"), Some("POLYGON"));
    let pos = cone.get("position").unwrap();
    assert_eq!(
        (pos.get_f64("x"), pos.get_f64("y")),
        (Some(150.0), Some(75.0))
    );
    let points = cone.get_array("points").unwrap();
    assert_eq!(points.len(), 10);
    assert_eq!(
        (points[0].get_f64("x"), points[0].get_f64("y")),
        (Some(0.0), Some(0.0))
    );
    let far = points[5].get_f64("x").unwrap();
    assert!(
        (far - 450.0).abs() < 1e-6,
        "arc midpoint at the cone's length, got {far}"
    );
    assert_eq!(cone.get_array("squares").unwrap().len(), 7);

    let line = compute_obr_aoe_item(
        &SpellArea::Line {
            length_ft: 60,
            width_ft: 5,
            direction_deg: 90.0,
        },
        75.0,
        75.0,
        150.0,
        1,
        "#0f0",
        "Lightning Bolt",
    );
    assert_eq!(line.get_str("shape_type"), Some("POLYGON"));
    assert_eq!(line.get_array("points").unwrap().len(), 4);
    assert_eq!(
        (line.get_f64("width"), line.get_f64("height")),
        (Some(150.0), Some(1800.0))
    );
}
