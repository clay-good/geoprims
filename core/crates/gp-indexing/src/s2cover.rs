//! S2 region covering (indexing/hierarchical-cells, "Region covering"): the
//! cells that cover a rectangle, a cap, or a polygon with great-circle edges,
//! within a level range and a cell budget.
//!
//! The covering starts from the six faces and refines, always splitting the
//! candidate that covers the most ground outside the region, until the budget
//! is reached or nothing is left to refine. A covering always contains the
//! region: a cell is kept whole rather than dropped when it only partly
//! overlaps, so the answer errs outward, which is what a covering is for.

use std::collections::VecDeque;

use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Related, Stability, ToolDef};
use gp_base::units::{self, Quantity as QT};
use libm::{cos, sqrt};

use crate::s2::CellId;

const EARTH_RADIUS_M: f64 = 6_371_008.8;

/// What is being covered.
enum Region {
    /// South, north, west, east in degrees; west may exceed east across the
    /// antimeridian.
    Rect {
        south: f64,
        north: f64,
        west: f64,
        east: f64,
    },
    /// A circle on the sphere: center and angular radius in radians.
    Cap { lat: f64, lon: f64, radius: f64 },
    /// A polygon with great-circle edges, as S2 draws them: its corners as
    /// unit vectors, holes as further rings.
    Polygon { rings: Vec<Vec<V>> },
}

type V = [f64; 3];

fn unit(lat: f64, lon: f64) -> V {
    let (a, o) = (lat.to_radians(), lon.to_radians());
    [cos(a) * cos(o), cos(a) * libm::sin(o), libm::sin(a)]
}

fn dot3(a: V, b: V) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross3(a: V, b: V) -> V {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Which side of the great circle a → b the point c is on (the sign of the
/// triple product), 0 within rounding.
fn side(a: V, b: V, c: V) -> i8 {
    let t = dot3(cross3(a, b), c);
    if t.abs() < 1e-15 {
        0
    } else if t > 0.0 {
        1
    } else {
        -1
    }
}

/// Whether the great-circle arcs a → b and c → d (each under 180°) may cross;
/// a touch within rounding counts, since a covering errs outward.
fn arcs_meet(a: V, b: V, c: V, d: V) -> bool {
    let (s1, s2) = (side(a, b, c), side(a, b, d));
    let (s3, s4) = (side(c, d, a), side(c, d, b));
    s1 * s2 <= 0 && s3 * s4 <= 0 && dot3(a, c) + dot3(a, d) + dot3(b, c) + dot3(b, d) > 0.0
}

/// Inside a polygon by winding: the turn of its edges seen from the point, in
/// the plane tangent there, adds to a full circle around each ring that holds
/// it; holes wind the other way. Exact on the sphere for rings smaller than a
/// hemisphere around the point.
fn in_polygon(rings: &[Vec<V>], p: V) -> bool {
    let mut turns = 0.0;
    for (k, ring) in rings.iter().enumerate() {
        let n = ring.len();
        let mut sum = 0.0;
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            let ta = [
                a[0] - dot3(a, p) * p[0],
                a[1] - dot3(a, p) * p[1],
                a[2] - dot3(a, p) * p[2],
            ];
            let tb = [
                b[0] - dot3(b, p) * p[0],
                b[1] - dot3(b, p) * p[1],
                b[2] - dot3(b, p) * p[2],
            ];
            sum += libm::atan2(dot3(p, cross3(ta, tb)), dot3(ta, tb));
        }
        let w = (sum / core::f64::consts::TAU).round().abs();
        turns += if k == 0 { w } else { -w };
    }
    turns > 0.5
}

fn lon_in(west: f64, east: f64, lon: f64) -> bool {
    if west <= east {
        lon >= west && lon <= east
    } else {
        lon >= west || lon <= east
    }
}

impl Region {
    fn contains(&self, lat: f64, lon: f64) -> bool {
        match *self {
            Region::Rect {
                south,
                north,
                west,
                east,
            } => lat >= south && lat <= north && lon_in(west, east, lon),
            Region::Cap {
                lat: cl,
                lon: co,
                radius,
            } => angle(cl, co, lat, lon) <= radius,
            Region::Polygon { ref rings } => in_polygon(rings, unit(lat, lon)),
        }
    }

    /// For a polygon: whether any of its edges crosses or touches an edge of
    /// the cell, or any of its corners lies in the cell.
    fn edges_meet(rings: &[Vec<V>], cell: CellId) -> bool {
        let v = cell.vertices().map(|(la, lo)| unit(la, lo));
        for ring in rings {
            let n = ring.len();
            for i in 0..n {
                let (a, b) = (ring[i], ring[(i + 1) % n]);
                if (0..4).any(|k| arcs_meet(a, b, v[k], v[(k + 1) % 4])) {
                    return true;
                }
                // A corner inside the cell, by S2's own rule: the cell at this
                // level that holds the corner is this one.
                let (la, lo) = (
                    libm::asin(a[2].clamp(-1.0, 1.0)).to_degrees(),
                    libm::atan2(a[1], a[0]).to_degrees(),
                );
                if CellId::from_lat_lon(la, lo, cell.level()) == cell {
                    return true;
                }
            }
        }
        false
    }

    /// The region's center and the angular radius of a circle around it that
    /// holds the whole region.
    fn bounding_circle(&self) -> (f64, f64, f64) {
        match *self {
            Region::Rect {
                south,
                north,
                west,
                east,
            } => {
                let clat = (south + north) / 2.0;
                let span = if west <= east {
                    east - west
                } else {
                    east + 360.0 - west
                };
                let clon = (west + span / 2.0 + 540.0) % 360.0 - 180.0;
                let r = [
                    angle(clat, clon, south, west),
                    angle(clat, clon, south, east),
                    angle(clat, clon, north, west),
                    angle(clat, clon, north, east),
                ]
                .into_iter()
                .fold(0.0, f64::max);
                (clat, clon, r)
            }
            Region::Cap { lat, lon, radius } => (lat, lon, radius),
            Region::Polygon { ref rings } => {
                let s = rings[0]
                    .iter()
                    .fold([0.0; 3], |m, v| [m[0] + v[0], m[1] + v[1], m[2] + v[2]]);
                let l = dot3(s, s).sqrt();
                let c = [s[0] / l, s[1] / l, s[2] / l];
                let r = rings[0]
                    .iter()
                    .map(|v| libm::acos(dot3(*v, c).clamp(-1.0, 1.0)))
                    .fold(0.0, f64::max);
                (
                    libm::asin(c[2].clamp(-1.0, 1.0)).to_degrees(),
                    libm::atan2(c[1], c[0]).to_degrees(),
                    r,
                )
            }
        }
    }

    /// Whether a cell may intersect the region. It errs toward yes: a cell
    /// wrongly kept only makes the covering coarser, while a cell wrongly
    /// dropped would leave a hole, and a covering with a hole is not one.
    ///
    /// The test is two circles — one around the cell, one around the region —
    /// because a latitude and longitude box does not describe an S2 cell near
    /// a pole or a face edge, and a box test that looks reasonable is exactly
    /// how a covering ends up with holes along its region's edge.
    fn may_intersect(&self, cell: CellId) -> bool {
        if let Region::Polygon { rings } = self {
            // Its bounding circle first, then the exact tests: a corner of
            // the cell inside it, or its boundary meeting the cell.
            let (rlat, rlon, rrad) = self.bounding_circle();
            let (clat, clon) = cell.center();
            let cr = cell
                .vertices()
                .iter()
                .map(|&(la, lo)| angle(clat, clon, la, lo))
                .fold(0.0, f64::max);
            if angle(rlat, rlon, clat, clon) > rrad + cr {
                return false;
            }
            return self.contains(clat, clon)
                || cell
                    .vertices()
                    .iter()
                    .any(|&(la, lo)| self.contains(la, lo))
                || Region::edges_meet(rings, cell);
        }
        let v = cell.vertices();
        let center = cell.center();
        if self.contains(center.0, center.1) || v.iter().any(|&(la, lo)| self.contains(la, lo)) {
            return true;
        }
        let (rlat, rlon, rrad) = self.bounding_circle();
        if CellId::from_lat_lon(rlat, rlon, cell.level()) == cell {
            return true;
        }
        let cell_radius = v
            .iter()
            .map(|&(la, lo)| angle(center.0, center.1, la, lo))
            .fold(0.0, f64::max);
        angle(rlat, rlon, center.0, center.1) <= rrad + cell_radius
    }

    /// Whether the region holds the whole cell, which means it need not be
    /// refined any further.
    fn contains_cell(&self, cell: CellId) -> bool {
        if let Region::Polygon { rings } = self {
            // Every corner inside and no part of the boundary in the cell.
            return cell
                .vertices()
                .iter()
                .all(|&(la, lo)| self.contains(la, lo))
                && !Region::edges_meet(rings, cell);
        }
        cell.vertices()
            .iter()
            .all(|&(la, lo)| self.contains(la, lo))
            && self.contains(cell.center().0, cell.center().1)
    }
}

/// The angle between two positions, in radians.
fn angle(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let (a, b) = (lat1.to_radians(), lat2.to_radians());
    let d = (lon2 - lon1).to_radians();
    let h = ((b - a) / 2.0).sin().powi(2) + cos(a) * cos(b) * (d / 2.0).sin().powi(2);
    2.0 * sqrt(h.min(1.0)).asin()
}

/// How many cells the refinement may look at. A wide region with a fine level
/// range can refine for a very long time, and a covering that takes minutes is
/// not one anyone waits for; stopping early keeps every candidate, so the
/// answer is still a covering, only coarser.
const MAX_STEPS: usize = 20_000;

/// The covering: cells that together contain the region, within the level
/// range, and at most `max_cells` of them where that is possible.
fn cover(region: &Region, min_level: u8, max_level: u8, max_cells: usize) -> (Vec<CellId>, bool) {
    // Start from the faces that may be involved.
    // A queue rather than a sorted list: every child is exactly one level
    // deeper than its parent, so taking from the front and adding to the back
    // visits levels in order without sorting or searching for the coarsest.
    let mut candidates: VecDeque<CellId> = (0..6)
        .map(|f| CellId::from_face_ij(f, 1 << 29, 1 << 29).parent(0))
        .filter(|c| region.may_intersect(*c))
        .collect();
    let mut done: Vec<CellId> = Vec::new();
    let mut steps = 0;
    let mut stopped_early = false;
    // Refine the candidate that is largest, so the budget is spent where it
    // buys the most: a large cell is what makes a covering coarse. The list
    // is kept in level order rather than sorted each time round.
    while !candidates.is_empty() {
        steps += 1;
        if steps > MAX_STEPS {
            stopped_early = true;
            done.extend(candidates.iter().copied());
            candidates.clear();
            break;
        }
        let cell = candidates.pop_front().expect("not empty");
        let level = cell.level();
        let finished = (level >= min_level && region.contains_cell(cell)) || level >= max_level;
        if finished {
            done.push(cell);
        } else if done.len() + candidates.len() + 4 > max_cells && level >= min_level {
            // Splitting would break the budget, so keep this cell whole.
            done.push(cell);
        } else {
            match cell.children() {
                None => done.push(cell),
                Some(children) => {
                    for child in children {
                        if region.may_intersect(child) {
                            candidates.push_back(child);
                        }
                    }
                }
            }
        }
        if done.len() >= max_cells {
            done.extend(candidates.iter().copied());
            candidates.clear();
            break;
        }
    }
    done.sort_by_key(|c| c.0);
    done.dedup();
    (done, stopped_early)
}

fn run_covering(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let min_level = ctx.number("min_level")?.unwrap_or(0.0) as u8;
    let max_level = ctx.number("max_level")?.unwrap_or(16.0) as u8;
    let max_cells = ctx.number("max_cells")?.unwrap_or(8.0) as usize;
    if min_level > max_level {
        return Err(ToolError::invalid(
            "/min_level",
            "The lowest level cannot be finer than the highest.",
        ));
    }
    if max_level > 30 {
        return Err(ToolError::invalid("/max_level", "S2 levels stop at 30."));
    }
    if max_cells == 0 {
        return Err(ToolError::invalid(
            "/max_cells",
            "A covering of no cells covers nothing.",
        ));
    }
    let radius = ctx.quantity("radius")?.map(|r| r.base());
    let polygon = read_polygon(ctx)?;
    let region = if let Some(rings) = polygon {
        if ["south", "north", "west", "east", "lat", "lon", "radius"]
            .iter()
            .any(|k| ctx.is_set(k))
        {
            return Err(ToolError::invalid(
                "/polygon",
                "Give a polygon, a rectangle, or a circle, not more than one.",
            ));
        }
        Region::Polygon { rings }
    } else {
        match (
            ctx.quantity("south")?,
            ctx.quantity("north")?,
            ctx.quantity("west")?,
            ctx.quantity("east")?,
            radius,
        ) {
            (Some(s), Some(n), Some(w), Some(e), None) => {
                let dg = units::by_symbol(QT::Angle, "deg").expect("deg");
                let (south, north, west, east) = (s.to(dg), n.to(dg), w.to(dg), e.to(dg));
                if south >= north {
                    return Err(ToolError::invalid(
                        "/south",
                        "The south edge must be below the north edge.",
                    ));
                }
                Region::Rect {
                    south,
                    north,
                    west,
                    east,
                }
            }
            (None, None, None, None, Some(r)) => {
                let dg = units::by_symbol(QT::Angle, "deg").expect("deg");
                let (Some(lat), Some(lon)) = (ctx.quantity("lat")?, ctx.quantity("lon")?) else {
                    return Err(ToolError::invalid(
                        "/lat",
                        "A circle needs its center: give lat and lon with the radius.",
                    ));
                };
                let (lat, lon) = (lat.to(dg), lon.to(dg));
                if r <= 0.0 {
                    return Err(ToolError::invalid(
                        "/radius",
                        "The radius must be positive.",
                    ));
                }
                Region::Cap {
                    lat,
                    lon,
                    radius: r / EARTH_RADIUS_M,
                }
            }
            _ => {
                return Err(ToolError::invalid(
                    "/radius",
                    "Give a rectangle (south, north, west, east), a center with a radius, or a polygon.",
                ));
            }
        }
    };
    // Rectangles and caps go through S2's own coverer, so the cells are the
    // ones S2 itself gives; polygons, which S2 covers through S2Polygon, keep
    // the refinement here.
    let exact = match region {
        Region::Rect {
            south,
            north,
            west,
            east,
        } => Some((
            crate::s2exact::Region::Rect {
                south,
                north,
                west,
                east,
            },
            (south.to_radians().sin() - north.to_radians().sin()).abs()
                * if west <= east {
                    east - west
                } else {
                    east + 360.0 - west
                }
                .to_radians(),
        )),
        Region::Cap { lat, lon, radius } => Some((
            crate::s2exact::Region::Cap { lat, lon, radius },
            2.0 * core::f64::consts::PI * (1.0 - radius.min(core::f64::consts::PI).cos()),
        )),
        Region::Polygon { .. } => None,
    };
    let (cells, stopped_early) = match exact {
        Some((r, area)) => {
            // S2 splits every cell coarser than the lowest level, so a wide
            // region with a fine lowest level would make millions of them.
            // Cells at a level vary about twofold around the average, so an
            // estimate past eight times the budget means at least four times
            // it, which is refused below anyway; refuse before building them.
            let estimate = area / crate::s2::average_area_steradians(min_level);
            if estimate > 8.0 * max_cells as f64 {
                return Err(ToolError::new(
                    ErrorCode::OutOfDomain,
                    format!(
                        "Covering this region with cells no coarser than level {min_level} takes about {estimate:.0} of them, far past the {max_cells} asked for. Raise the budget, or lower the lowest level."
                    ),
                )
                .at("/min_level"));
            }
            (
                crate::s2exact::covering(&r, min_level, max_level, max_cells),
                false,
            )
        }
        None => cover(&region, min_level, max_level, max_cells),
    };
    if stopped_early {
        ctx.warnings.push(Warning::new(
            "COVERING_OVER_BUDGET",
            "The region is wide for this level range, so the refinement stopped early; the cells still cover it, but coarsely. Raise the lowest level or narrow the region for a tighter cover.",
        ));
    }
    // A lowest level finer than the budget allows cannot be honored: returning
    // tens of thousands of cells to a caller who asked for a few is not a
    // better answer than saying so.
    if cells.len() > max_cells.saturating_mul(4) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            format!(
                "Covering this region with cells no coarser than level {min_level} takes about {} of them, far past the {max_cells} asked for. Raise the budget, or lower the lowest level.",
                cells.len()
            ),
        )
        .at("/min_level"));
    }
    if cells.len() > max_cells {
        ctx.warnings.push(Warning::new(
            "COVERING_OVER_BUDGET",
            format!(
                "The region needs {} cells at level {min_level} or coarser, more than the {max_cells} asked for; a smaller covering would leave part of it out.",
                cells.len()
            ),
        ));
    }
    let area: f64 = cells.iter().map(|c| c.area_steradians()).sum();
    let levels: Vec<u8> = cells.iter().map(|c| c.level()).collect();
    Ok(Json::obj([
        ("count", Json::Num(cells.len() as f64)),
        (
            "cells",
            Json::Arr(
                cells
                    .iter()
                    .map(|c| {
                        let (lat, lon) = c.center();
                        Json::obj([
                            ("cell", Json::str(c.token())),
                            ("level", Json::Num(f64::from(c.level()))),
                            (
                                "lat",
                                Q {
                                    value: lat,
                                    unit: units::by_symbol(QT::Angle, "deg").expect("deg"),
                                }
                                .to_json(),
                            ),
                            (
                                "lon",
                                Q {
                                    value: lon,
                                    unit: units::by_symbol(QT::Angle, "deg").expect("deg"),
                                }
                                .to_json(),
                            ),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "covered_area",
            ctx.out(
                "covered_area",
                Q {
                    value: area * EARTH_RADIUS_M * EARTH_RADIUS_M,
                    unit: units::by_symbol(QT::Area, "m2").expect("m2"),
                },
            ),
        ),
        (
            "finest_level",
            Json::Num(f64::from(levels.iter().copied().max().unwrap_or(0))),
        ),
        (
            "coarsest_level",
            Json::Num(f64::from(levels.iter().copied().min().unwrap_or(0))),
        ),
    ]))
}

/// The polygon input, if given: rings of corners as unit vectors, the
/// outline first.
fn read_polygon(ctx: &mut Ctx) -> Result<Option<Vec<Vec<V>>>, ToolError> {
    if !ctx.is_set("polygon") {
        return Ok(None);
    }
    let dg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let rows = ctx.rows("polygon")?;
    let mut rings: Vec<Vec<V>> = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        let lat = ctx
            .row_quantity("polygon", i, r, "lat")?
            .expect("required")
            .to(dg);
        let lon = ctx
            .row_quantity("polygon", i, r, "lon")?
            .expect("required")
            .to(dg);
        if !(-90.0..=90.0).contains(&lat) {
            return Err(ToolError::invalid(
                &format!("/polygon/{i}/lat"),
                "Latitude must be between -90° and 90°.",
            ));
        }
        let ring = r
            .get("ring")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);
        if ring.fract() != 0.0 || !(0.0..=100.0).contains(&ring) {
            return Err(ToolError::invalid(
                &format!("/polygon/{i}/ring"),
                "Ring must be a whole number from 0 (the outline) to 100.",
            ));
        }
        let k = ring as usize;
        if rings.len() <= k {
            rings.resize(k + 1, Vec::new());
        }
        let v = unit(lat, lon);
        if rings[k].last() != Some(&v) {
            rings[k].push(v);
        }
    }
    rings.retain(|r| !r.is_empty());
    for (k, ring) in rings.iter_mut().enumerate() {
        if ring.len() > 1 && ring.first() == ring.last() {
            ring.pop();
        }
        if ring.len() < 3 {
            return Err(ToolError::invalid(
                "/polygon",
                format!("Ring {k} needs at least 3 distinct corners."),
            ));
        }
    }
    Ok(Some(rings))
}

const POLY_ROW: &[Field] = &[
    Field::new(
        "lat",
        "Latitude",
        "Decimal degrees, like 40.44",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .required(),
    Field::new(
        "lon",
        "Longitude",
        "Decimal degrees, like -79.99",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .required(),
    Field::new(
        "ring",
        "Ring",
        "0 for the outline, 1, 2, … for holes, like 1",
        Kind::Number {
            min: 0.0,
            max: 100.0,
        },
    ),
];

const COVER_ROW: &[Field] = &[
    Field::new("cell", "Cell", "Its token", Kind::Text { max_len: 24 }),
    Field::new(
        "level",
        "Level",
        "0 to 30",
        Kind::Number {
            min: 0.0,
            max: 30.0,
        },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "lat",
        "Latitude",
        "Of its center, in degrees",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7)),
    Field::new(
        "lon",
        "Longitude",
        "Of its center, in degrees",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7)),
];

pub static COVERING: ToolDef = ToolDef {
    id: "indexing.s2.covering",
    version: "2.0.0",
    title: "S2 cells covering a region",
    summary: "The S2 cells that cover a latitude and longitude rectangle, a circle around a point, or a polygon with holes, within a level range and a cell budget.",
    aliases: &[
        "S2 region coverer",
        "S2 covering",
        "cover a box with S2 cells",
        "cover a polygon with S2 cells",
    ],
    keywords: &[
        "S2",
        "covering",
        "coverer",
        "region",
        "cells",
        "bounding box",
        "radius",
        "index",
    ],
    inputs: &[
        Field::new(
            "south",
            "South edge",
            "Of the rectangle, like 40.4 deg",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .core(),
        Field::new(
            "north",
            "North edge",
            "Of the rectangle, like 40.5 deg",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .core(),
        Field::new(
            "west",
            "West edge",
            "Of the rectangle, like -80.1 deg",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .core(),
        Field::new(
            "east",
            "East edge",
            "Of the rectangle, like -79.9 deg",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .core(),
        Field::new(
            "lat",
            "Center latitude",
            "For a circle, like 40.44 deg",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .core(),
        Field::new(
            "lon",
            "Center longitude",
            "For a circle, like -79.99 deg",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .core(),
        Field::new(
            "radius",
            "Radius",
            "For a circle instead of a rectangle, like 5 km",
            Kind::Quantity {
                q: QT::Length,
                unit: "km",
            },
        ),
        Field::new(
            "polygon",
            "Polygon",
            "Instead of a rectangle or circle: corners in order, like 40.44, -79.99; holes as ring 1, 2, …",
            Kind::List {
                items: POLY_ROW,
                min: 3,
                max: 2_000,
            },
        ),
        Field::new(
            "min_level",
            "Lowest level",
            "Coarsest cell allowed, 0 to 30; default 0",
            Kind::Number {
                min: 0.0,
                max: 30.0,
            },
        ),
        Field::new(
            "max_level",
            "Highest level",
            "Finest cell allowed, 0 to 30; default 16",
            Kind::Number {
                min: 0.0,
                max: 30.0,
            },
        ),
        Field::new(
            "max_cells",
            "Cell budget",
            "How many cells at most, like 8",
            Kind::Number {
                min: 1.0,
                max: 1000.0,
            },
        ),
    ],
    outputs: &[
        Field::new(
            "count",
            "Cells",
            "How many the covering has",
            Kind::Number {
                min: 0.0,
                max: 1000.0,
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "cells",
            "Covering",
            "The cells, in id order",
            Kind::List {
                items: COVER_ROW,
                min: 0,
                max: 1000,
            },
        ),
        Field::new(
            "covered_area",
            "Covered area",
            "The area of the covering, which is larger than the region",
            Kind::Quantity {
                q: QT::Area,
                unit: "km2",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "finest_level",
            "Finest level",
            "The deepest cell used",
            Kind::Number {
                min: 0.0,
                max: 30.0,
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "coarsest_level",
            "Coarsest level",
            "The shallowest cell used",
            Kind::Number {
                min: 0.0,
                max: 30.0,
            },
        )
        .precision(Precision::Decimals(0)),
    ],
    errors: &[ErrorCode::InvalidInput],
    stability: Stability::Stable,
    warnings: &["COVERING_OVER_BUDGET", "UNIT_ASSUMED"],
    model: "Refinement from the six faces: the coarsest candidate that meets the region is split into its four children, keeping any cell the region contains whole, until the budget is reached; a cell that only partly overlaps is kept, so the covering contains the region. A polygon's edges are great circles, as in S2; a cell meets it when its center or a corner is inside (by the winding of the polygon's edges seen from there), an edge of the polygon crosses or touches one of the cell's, or a corner of the polygon lies in the cell, and it is whole inside when its four corners are inside and none of that boundary reaches it",
    accuracy: "The covering always contains the region. It is not the smallest such set: S2's own coverer uses a priority order that can find a tighter cover for the same budget, so treat the cells as a superset rather than a canonical answer.",
    when_to_use: "Use this to turn an area into index keys: the cells of a covering are what you query a cell-indexed store with, then filter the results by the true geometry. The budget and the level range are the two knobs — more cells or finer levels mean a tighter cover and a larger key set.",
    limitations: "A covering is a superset: every cell overlaps the region but the cells together cover more ground than it, which is why a covering query still needs a second filter. A polygon's edges are great circles, as S2 draws them, not lines of constant bearing and not geodesics on the ellipsoid; for edges of more than a few hundred kilometers, add corners if the boundary should follow something else. A polygon must be smaller than a hemisphere. The cells are a valid covering rather than S2's own choice, so a set from another library may differ while covering the same ground. The cell budget is not a hard cap: the lowest level wins over it, so a region that needs more cells than the budget just to reach that level gets them, with a warning saying so -- a smaller set would leave part of the region out. Past four times the budget the request is refused instead.",
    references: &[crate::s2tools::S2_DOCS],
    examples: &[Example {
        id: "primary",
        title: "A small box near Pittsburgh in at most 8 cells",
        input: r#"{"south":"40.43 deg","north":"40.46 deg","west":"-80.01 deg","east":"-79.96 deg","min_level":10,"max_level":16,"max_cells":8}"#,
        source: "add-spatial-indexing-and-raster hierarchical-cells scenario: at most 8 cells, all between levels 10 and 16, together covering the region",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "indexing.s2.lat-lng-to-cell",
            reason: "parent",
        },
        Related {
            id: "indexing.s2.cell-info",
            reason: "next",
        },
        Related {
            id: "indexing.h3.polygon-to-cells",
            reason: "alternative",
        },
    ],
    sentence: "That region takes {count} S2 {plural count \"cell\" \"cells\"}, from level {coarsest_level} to {finest_level}.",
    limits: &[("batchRows", 1_000)],
    run: run_covering,
    ..ToolDef::BLANK
};
