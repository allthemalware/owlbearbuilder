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
        todo!()
    }

    /// `{"kind": "burst"|"emanation"|"cone"|"line", "radius_ft" | "length_ft", "width_ft",
    /// "direction_deg"}` with only the fields the kind has.
    pub fn to_json(&self) -> JsonValue {
        todo!()
    }

    pub fn from_json(v: &JsonValue) -> Result<SpellArea, String> {
        todo!()
    }
}

/// Finds an area in spell text, case-insensitive, anywhere in the string: `20-foot burst`,
/// `10-foot radius burst`, `20 ft. burst`, `20-ft burst`, `30-foot emanation`,
/// `15-foot cone`, `60-foot line`, `120-foot line, 10 feet wide` (also `10-foot-wide`).
/// The first match wins; directions start at 0; lines default to 5 feet wide. `None` when
/// no area is mentioned or the size is 0.
pub fn parse_area_description(desc: &str) -> Option<SpellArea> {
    todo!()
}

/// Feet to move `dx` by `dy` squares: the longer leg's squares cost 5 each, and every second
/// diagonal adds another 5 (`5 * max + 5 * (min / 2)`), so 1 diagonal is 5 ft, 2 are 15 ft,
/// 3 are 20 ft. Signs are ignored.
pub fn pf2e_distance_ft(dx: i32, dy: i32) -> u32 {
    todo!()
}

/// The nearest multiple of 45 in `0..360`, for any input angle (negative or above 360 too).
/// Exact halves round up (22.5 → 45).
pub fn snap_direction(direction_deg: f64) -> u32 {
    todo!()
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
    todo!()
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
    todo!()
}
