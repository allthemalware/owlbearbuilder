// SPDX-License-Identifier: GPL-3.0-or-later
//! Spell areas on the grid (Player Core, Areas) and their Owlbear Rodeo shapes.
//!
//! Grid coordinates are cells `(x, y)` with y growing downwards, as on screen. Directions are
//! degrees clockwise from east: 0 east, 90 south, 180 west, 270 north. Distances follow the
//! movement rule: every second diagonal costs 10 feet instead of 5.

use crate::json::JsonValue;

#[derive(Debug, Clone, PartialEq)]
pub enum SpellArea {
    /// Spreads from a grid intersection.
    Burst { radius_ft: u32 },
    /// Spreads from the edges of the caster's space.
    Emanation { radius_ft: u32 },
    /// A quarter circle starting at the caster's space.
    Cone { length_ft: u32, direction_deg: f64 },
    /// Starts at the caster's space; `width_ft` is 5 unless the spell says otherwise.
    Line {
        length_ft: u32,
        width_ft: u32,
        direction_deg: f64,
    },
}

impl SpellArea {
    /// `20-foot burst`, `30-foot emanation`, `15-foot cone`, `60-foot line`, and for wide lines
    /// `60-foot line, 10 feet wide`.
    pub fn label(&self) -> String {
        match self {
            Self::Burst { radius_ft } => format!("{radius_ft}-foot burst"),
            Self::Emanation { radius_ft } => format!("{radius_ft}-foot emanation"),
            Self::Cone { length_ft, .. } => format!("{length_ft}-foot cone"),
            Self::Line {
                length_ft,
                width_ft,
                ..
            } if *width_ft == 5 => {
                format!("{length_ft}-foot line")
            }
            Self::Line {
                length_ft,
                width_ft,
                ..
            } => {
                format!("{length_ft}-foot line, {width_ft} feet wide")
            }
        }
    }

    /// `{"kind": "burst"|"emanation"|"cone"|"line", "radius_ft" | "length_ft", "width_ft",
    /// "direction_deg"}` with only the fields the kind has.
    pub fn to_json(&self) -> JsonValue {
        let mut value = JsonValue::object();
        match self {
            Self::Burst { radius_ft } => {
                value.insert("kind", "burst");
                value.insert("radius_ft", *radius_ft as f64);
            }
            Self::Emanation { radius_ft } => {
                value.insert("kind", "emanation");
                value.insert("radius_ft", *radius_ft as f64);
            }
            Self::Cone {
                length_ft,
                direction_deg,
            } => {
                value.insert("kind", "cone");
                value.insert("length_ft", *length_ft as f64);
                value.insert("direction_deg", *direction_deg);
            }
            Self::Line {
                length_ft,
                width_ft,
                direction_deg,
            } => {
                value.insert("kind", "line");
                value.insert("length_ft", *length_ft as f64);
                value.insert("width_ft", *width_ft as f64);
                value.insert("direction_deg", *direction_deg);
            }
        }
        value
    }

    pub fn from_json(v: &JsonValue) -> Result<SpellArea, String> {
        let field = |name: &str| {
            v.get(name)
                .and_then(JsonValue::as_i64)
                .and_then(|n| u32::try_from(n).ok())
                .ok_or_else(|| format!("invalid or missing {name}"))
        };
        let direction = || {
            v.get("direction_deg")
                .and_then(JsonValue::as_f64)
                .filter(|n| n.is_finite())
                .ok_or_else(|| "invalid or missing direction_deg".to_string())
        };
        match v.get_str("kind") {
            Some("burst") => Ok(Self::Burst {
                radius_ft: field("radius_ft")?,
            }),
            Some("emanation") => Ok(Self::Emanation {
                radius_ft: field("radius_ft")?,
            }),
            Some("cone") => Ok(Self::Cone {
                length_ft: field("length_ft")?,
                direction_deg: direction()?,
            }),
            Some("line") => Ok(Self::Line {
                length_ft: field("length_ft")?,
                width_ft: field("width_ft")?,
                direction_deg: direction()?,
            }),
            Some(kind) => Err(format!("unknown area kind: {kind}")),
            None => Err("missing area kind".into()),
        }
    }
}

/// Finds an area in spell text, case-insensitive, anywhere in the string: `20-foot burst`,
/// `10-foot radius burst`, `20 ft. burst`, `20-ft burst`, `30-foot emanation`,
/// `15-foot cone`, `60-foot line`, `120-foot line, 10 feet wide` (also `10-foot-wide`).
/// The first match wins; directions start at 0; lines default to 5 feet wide. `None` when
/// no area is mentioned or the size is 0.
pub fn parse_area_description(desc: &str) -> Option<SpellArea> {
    let lower = desc.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    for start in 0..bytes.len() {
        if !bytes[start].is_ascii_digit() || (start > 0 && bytes[start - 1].is_ascii_alphanumeric())
        {
            continue;
        }
        let mut end = start;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        let size = lower[start..end].parse::<u32>().ok()?;
        let mut cursor = end;
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        let suffix = &lower[cursor..];
        let consumed = if suffix.starts_with("-foot") {
            5
        } else if suffix.starts_with("-ft") || suffix.starts_with("ft.") {
            3
        } else if suffix.starts_with("feet") || suffix.starts_with("foot") {
            4
        } else {
            continue;
        };
        cursor += consumed;
        if cursor < bytes.len() && bytes[cursor] == b'-' {
            cursor += 1;
        }
        while cursor < bytes.len() && (bytes[cursor].is_ascii_whitespace() || bytes[cursor] == b',')
        {
            cursor += 1;
        }
        if lower[cursor..].starts_with("radius") {
            cursor += 6;
            while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
                cursor += 1;
            }
        }
        let word_end = lower[cursor..]
            .find(|c: char| !c.is_ascii_alphabetic())
            .map_or(lower.len(), |n| cursor + n);
        let kind = &lower[cursor..word_end];
        if size == 0 {
            continue;
        }
        let area = match kind {
            "burst" => SelfArea::Burst(size),
            "emanation" => SelfArea::Emanation(size),
            "cone" => SelfArea::Cone(size),
            "line" => SelfArea::Line(size),
            _ => continue,
        };
        return Some(match area {
            SelfArea::Burst(radius_ft) => SpellArea::Burst { radius_ft },
            SelfArea::Emanation(radius_ft) => SpellArea::Emanation { radius_ft },
            SelfArea::Cone(length_ft) => SpellArea::Cone {
                length_ft,
                direction_deg: 0.0,
            },
            SelfArea::Line(length_ft) => {
                let tail = &lower[word_end..];
                let width_ft = parse_line_width(tail).unwrap_or(5);
                SpellArea::Line {
                    length_ft,
                    width_ft,
                    direction_deg: 0.0,
                }
            }
        });
    }
    None
}

enum SelfArea {
    Burst(u32),
    Emanation(u32),
    Cone(u32),
    Line(u32),
}

fn parse_line_width(tail: &str) -> Option<u32> {
    let bytes = tail.as_bytes();
    for start in 0..bytes.len() {
        if !bytes[start].is_ascii_digit() || (start > 0 && bytes[start - 1].is_ascii_alphanumeric())
        {
            continue;
        }
        let mut end = start;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        let size = tail[start..end].parse::<u32>().ok()?;
        let rest = tail[end..].trim_start_matches(|c: char| c.is_ascii_whitespace() || c == ',');
        if rest.starts_with("-foot-wide") || rest.starts_with("-ft-wide") {
            return (size > 0).then_some(size);
        }
        let rest = rest
            .strip_prefix("-foot")
            .or_else(|| rest.strip_prefix("-ft"))
            .or_else(|| rest.strip_prefix("feet"))
            .or_else(|| rest.strip_prefix("foot"))
            .or_else(|| rest.strip_prefix("ft."))
            .unwrap_or("")
            .trim_start();
        if rest.starts_with("wide") {
            return (size > 0).then_some(size);
        }
    }
    None
}

/// Feet to move `dx` by `dy` squares: the longer leg's squares cost 5 each, and every second
/// diagonal adds another 5 (`5 * max + 5 * (min / 2)`), so 1 diagonal is 5 ft, 2 are 15 ft,
/// 3 are 20 ft. Signs are ignored.
pub fn pf2e_distance_ft(dx: i32, dy: i32) -> u32 {
    let (x, y) = (dx.unsigned_abs(), dy.unsigned_abs());
    let (long, short) = (x.max(y), x.min(y));
    long.saturating_mul(5)
        .saturating_add((short / 2).saturating_mul(5))
}

/// The nearest multiple of 45 in `0..360`, for any input angle (negative or above 360 too).
/// Exact halves round up (22.5 → 45).
pub fn snap_direction(direction_deg: f64) -> u32 {
    if !direction_deg.is_finite() {
        return 0;
    }
    (direction_deg.rem_euclid(360.0) / 45.0 + 0.5).floor() as u32 * 45 % 360
}

/// Cells the area covers, sorted by `(y, x)`, without duplicates.
///
/// - Burst: the origin is the intersection at the top-left corner of cell `(0, 0)`. A cell
///   `dx` columns and `dy` rows away from the origin within its quadrant (the four cells
///   touching the origin are 0, 0) is covered when `pf2e_distance_ft(dx, dy) + 5 <= radius`.
///   A 5-foot burst is 2×2, 10-foot 4×4, 15-foot 6×6 less one cell per corner, 20-foot 52
///   cells.
/// - The other kinds start from the caster's space: cells `0..size` on both axes
///   (`token_size_squares`, at least 1).
/// - Emanation: the caster's own cells plus every cell whose distance to the space
///   (`pf2e_distance_ft` of the squares between, counting the cell itself) is within the
///   radius: a 10-foot emanation from a 1-cell creature is 5×5 less the corners.
/// - Cone, direction snapped to 45°. Orthogonal: column `c` (0 = touching the space) out
///   from the middle of that side covers lateral offsets `-c..=c` around the middle row(s),
///   kept when `pf2e_distance_ft(c, offset) + 5 <= length`; for spaces wider than 1 the
///   middle is the space's whole side, widened by `c` cells either way. Diagonal: the
///   quadrant beyond the space's corner in that direction, cells kept by the burst rule
///   measured from that corner.
/// - Line, direction snapped to 45°. Orthogonal: `length / 5` cells straight out from the
///   middle of that side (the space's whole side for wider spaces), `width / 5` rows wide,
///   extra rows added on alternate sides starting with the positive side. Diagonal: cells
///   along the diagonal from the corner while `pf2e_distance_ft(n, n) <= length`; width is
///   ignored on diagonals.
pub fn affected_squares(area: &SpellArea, token_size_squares: u32) -> Vec<(i32, i32)> {
    let size = token_size_squares.max(1) as i32;
    let mut cells = Vec::new();
    match area {
        SpellArea::Burst { radius_ft } => {
            let reach = (*radius_ft / 5 + 2) as i32;
            for y in -reach..=reach {
                for x in -reach..=reach {
                    let dx = if x < 0 { -x - 1 } else { x };
                    let dy = if y < 0 { -y - 1 } else { y };
                    if pf2e_distance_ft(dx, dy).saturating_add(5) <= *radius_ft {
                        cells.push((x, y));
                    }
                }
            }
        }
        SpellArea::Emanation { radius_ft } => {
            let reach = (*radius_ft / 5 + 2) as i32;
            for y in -reach..size + reach {
                for x in -reach..size + reach {
                    let dx = if x < 0 {
                        -x
                    } else if x >= size {
                        x - size + 1
                    } else {
                        0
                    };
                    let dy = if y < 0 {
                        -y
                    } else if y >= size {
                        y - size + 1
                    } else {
                        0
                    };
                    if dx == 0 && dy == 0 || pf2e_distance_ft(dx, dy) <= *radius_ft {
                        cells.push((x, y));
                    }
                }
            }
        }
        SpellArea::Cone {
            length_ft,
            direction_deg,
        } => {
            let dir = snap_direction(*direction_deg);
            if matches!(dir, 0 | 90 | 180 | 270) {
                let reach = (*length_ft / 5 + 1) as i32;
                for c in 0..=reach {
                    for offset in -c..=size - 1 + c {
                        if pf2e_distance_ft(c, lateral_offset(offset, size)).saturating_add(5)
                            > *length_ft
                        {
                            continue;
                        }
                        for base in 0..size {
                            let point = match dir {
                                0 => (size + c, offset),
                                90 => (offset, size + c),
                                180 => (-1 - c, offset),
                                _ => (offset, -1 - c),
                            };
                            let _ = base;
                            cells.push(point);
                        }
                    }
                }
            } else {
                let (sx, sy) = diagonal_signs(dir);
                let reach = (*length_ft / 5 + 2) as i32;
                for y in 0..reach {
                    for x in 0..reach {
                        if pf2e_distance_ft(x, y).saturating_add(5) <= *length_ft {
                            cells.push((
                                if sx > 0 { size + x } else { -1 - x },
                                if sy > 0 { size + y } else { -1 - y },
                            ));
                        }
                    }
                }
            }
        }
        SpellArea::Line {
            length_ft,
            width_ft,
            direction_deg,
        } => {
            let dir = snap_direction(*direction_deg);
            if matches!(dir, 0 | 90 | 180 | 270) {
                let count = (*length_ft / 5) as i32;
                let rows = (*width_ft / 5).max(1) as i32;
                let low = -((rows - 1) / 2);
                let high = rows / 2;
                for n in 0..count {
                    for offset in low..=high {
                        for side in 0..size {
                            let lateral = offset + side;
                            cells.push(match dir {
                                0 => (size + n, lateral),
                                90 => (lateral, size + n),
                                180 => (-1 - n, lateral),
                                _ => (lateral, -1 - n),
                            });
                        }
                    }
                }
            } else {
                let (sx, sy) = diagonal_signs(dir);
                let mut n = 1;
                while pf2e_distance_ft(n, n) <= *length_ft {
                    cells.push((
                        if sx > 0 { size - 1 + n } else { -n },
                        if sy > 0 { size - 1 + n } else { -n },
                    ));
                    n += 1;
                }
            }
        }
    }
    cells.sort_by_key(|&(x, y)| (y, x));
    cells.dedup();
    cells
}

fn lateral_offset(offset: i32, size: i32) -> i32 {
    if offset < 0 {
        offset
    } else if offset >= size {
        offset - size + 1
    } else {
        0
    }
}

fn diagonal_signs(direction: u32) -> (i32, i32) {
    match direction {
        45 => (1, 1),
        135 => (-1, 1),
        225 => (-1, -1),
        _ => (1, -1),
    }
}

/// A drawable shape for Owlbear Rodeo plus the covered cells.
///
/// `origin_x/y` is in scene pixels: the grid intersection for a burst, the caster's centre
/// otherwise. `grid_dpi` is pixels per 5-foot square, so `pixels_per_foot = grid_dpi / 5`.
/// Output object:
/// - `shape_type`: `CIRCLE` for burst (centred on the origin, radius = radius) and emanation
///   (centred on the caster, radius = radius + half the space); `POLYGON` for cone (apex where
///   the direction leaves the caster's space, then 9 points along the 90° arc at `length`,
///   using the exact unsnapped direction) and line (4 corners of the rectangle starting where
///   the direction leaves the space).
/// - `position {x, y}`: the circle's centre, or the polygon's anchor (the apex or line start).
/// - `width`, `height`: the circle's diameter, or the polygon's bounding box.
/// - `points`: `[{x, y}]` relative to `position` (empty for circles).
/// - `fill_color` and `stroke_color` = `color_hex`, `fill_opacity` 0.28, `label`,
///   `area` (`SpellArea::to_json`), `squares`: covered cells as `[x, y]` pairs offset to
///   absolute cell indices (burst: relative to the intersection's top-left cell; others:
///   relative to the caster's top-left cell at `origin - size * grid_dpi / 2`, divided by
///   `grid_dpi` and floored), and `metadata: {"com.owlbearbuilder/aoe": true}`.
pub fn compute_obr_aoe_item(
    area: &SpellArea,
    origin_x: f64,
    origin_y: f64,
    grid_dpi: f64,
    token_size_squares: u32,
    color_hex: &str,
    label: &str,
) -> JsonValue {
    let size = token_size_squares.max(1);
    let ppf = grid_dpi / 5.0;
    let mut item = JsonValue::object();
    let mut points: Vec<(f64, f64)> = Vec::new();
    let (shape, anchor_x, anchor_y, width, height) = match area {
        SpellArea::Burst { radius_ft } => {
            let radius = *radius_ft as f64 * ppf;
            ("CIRCLE", origin_x, origin_y, radius * 2.0, radius * 2.0)
        }
        SpellArea::Emanation { radius_ft } => {
            let radius = *radius_ft as f64 * ppf + size as f64 * grid_dpi / 2.0;
            ("CIRCLE", origin_x, origin_y, radius * 2.0, radius * 2.0)
        }
        SpellArea::Cone {
            length_ft,
            direction_deg,
        } => {
            let (ax, ay) = space_edge(origin_x, origin_y, grid_dpi, size, *direction_deg);
            let radius = *length_ft as f64 * ppf;
            points.push((0.0, 0.0));
            for i in 0..=8 {
                let angle = (direction_deg - 45.0 + i as f64 * 90.0 / 8.0).to_radians();
                points.push((radius * angle.cos(), radius * angle.sin()));
            }
            (
                "POLYGON",
                ax,
                ay,
                bounds(&points, true),
                bounds(&points, false),
            )
        }
        SpellArea::Line {
            length_ft,
            width_ft,
            direction_deg,
        } => {
            let (ax, ay) = space_edge(origin_x, origin_y, grid_dpi, size, *direction_deg);
            let angle = direction_deg.to_radians();
            let (ux, uy) = clean_direction(angle.cos(), angle.sin());
            let (px, py) = (
                -uy * *width_ft as f64 * ppf / 2.0,
                ux * *width_ft as f64 * ppf / 2.0,
            );
            let (length, _) = (*length_ft as f64 * ppf, *width_ft as f64 * ppf);
            points.extend([
                (-px, -py),
                (ux * length - px, uy * length - py),
                (ux * length + px, uy * length + py),
                (px, py),
            ]);
            for point in &mut points {
                if point.0.abs() < 1e-10 {
                    point.0 = 0.0;
                }
                if point.1.abs() < 1e-10 {
                    point.1 = 0.0;
                }
            }
            (
                "POLYGON",
                ax,
                ay,
                bounds(&points, true),
                bounds(&points, false),
            )
        }
    };
    item.insert("shape_type", shape);
    item.insert("position", point_object(anchor_x, anchor_y));
    item.insert("width", width);
    item.insert("height", height);
    item.insert(
        "points",
        JsonValue::Array(points.iter().map(|&(x, y)| point_object(x, y)).collect()),
    );
    item.insert("fill_color", color_hex);
    item.insert("stroke_color", color_hex);
    item.insert("fill_opacity", 0.28);
    item.insert("label", label);
    item.insert("area", area.to_json());
    let base_x = if matches!(area, SpellArea::Burst { .. }) {
        (origin_x / grid_dpi).floor() as i32
    } else {
        ((origin_x - size as f64 * grid_dpi / 2.0) / grid_dpi).floor() as i32
    };
    let base_y = if matches!(area, SpellArea::Burst { .. }) {
        (origin_y / grid_dpi).floor() as i32
    } else {
        ((origin_y - size as f64 * grid_dpi / 2.0) / grid_dpi).floor() as i32
    };
    let squares = affected_squares(area, size)
        .into_iter()
        .map(|(x, y)| {
            JsonValue::Array(vec![
                JsonValue::Number((x + base_x) as f64),
                JsonValue::Number((y + base_y) as f64),
            ])
        })
        .collect();
    item.insert("squares", JsonValue::Array(squares));
    let mut metadata = JsonValue::object();
    metadata.insert("com.owlbearbuilder/aoe", true);
    item.insert("metadata", metadata);
    item
}

fn point_object(x: f64, y: f64) -> JsonValue {
    let mut point = JsonValue::object();
    point.insert("x", x);
    point.insert("y", y);
    point
}

fn bounds(points: &[(f64, f64)], x_axis: bool) -> f64 {
    let (min, max) = points
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), p| {
            let v = if x_axis { p.0 } else { p.1 };
            (min.min(v), max.max(v))
        });
    max - min
}

fn space_edge(x: f64, y: f64, grid_dpi: f64, size: u32, direction_deg: f64) -> (f64, f64) {
    let angle = direction_deg.to_radians();
    let (dx, dy) = clean_direction(angle.cos(), angle.sin());
    let distance = size as f64 * grid_dpi / 2.0 / dx.abs().max(dy.abs());
    (x + dx * distance, y + dy * distance)
}

fn clean_direction(x: f64, y: f64) -> (f64, f64) {
    let clean = |value: f64| if value.abs() < 1e-10 { 0.0 } else { value };
    (clean(x), clean(y))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn burst_quadrants_are_mirrors() {
        let squares = affected_squares(&SpellArea::Burst { radius_ft: 15 }, 1);
        let reflect_x = |(x, y): &(i32, i32)| (-*x - 1, *y);
        let reflect_y = |(x, y): &(i32, i32)| (*x, -*y - 1);
        let set = |cells: Vec<(i32, i32)>| {
            let mut cells = cells;
            cells.sort_by_key(|&(x, y)| (y, x));
            cells
        };
        assert_eq!(set(squares.iter().map(reflect_x).collect()), squares);
        assert_eq!(set(squares.iter().map(reflect_y).collect()), squares);
    }

    #[test]
    fn cones_have_directional_count_symmetry() {
        let counts: Vec<_> = (0..8)
            .map(|i| {
                affected_squares(
                    &SpellArea::Cone {
                        length_ft: 30,
                        direction_deg: i as f64 * 45.0,
                    },
                    1,
                )
                .len()
            })
            .collect();
        assert_eq!(counts[0], counts[2]);
        assert_eq!(counts[2], counts[4]);
        assert_eq!(counts[4], counts[6]);
        assert_eq!(counts[1], counts[3]);
        assert_eq!(counts[3], counts[5]);
        assert_eq!(counts[5], counts[7]);
        assert_eq!(counts[0], 26);
    }

    #[test]
    fn diagonal_south_west_line_uses_corner_path() {
        let cells = affected_squares(
            &SpellArea::Line {
                length_ft: 30,
                width_ft: 10,
                direction_deg: 135.0,
            },
            1,
        );
        assert_eq!(cells, [(-1, 1), (-2, 2), (-3, 3), (-4, 4)]);
    }

    #[test]
    fn large_cone_apex_is_on_diagonal_space_corner() {
        let item = compute_obr_aoe_item(
            &SpellArea::Cone {
                length_ft: 15,
                direction_deg: 45.0,
            },
            150.0,
            150.0,
            150.0,
            2,
            "#fff",
            "",
        );
        let position = item.get("position").unwrap();
        assert_eq!(position.get_f64("x"), Some(300.0));
        assert_eq!(position.get_f64("y"), Some(300.0));
    }
}
