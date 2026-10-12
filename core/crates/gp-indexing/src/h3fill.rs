//! H3 polygonToCells (hexagonal-grids spec, "Polyfill with explicit
//! containment and limits") on h3o's core API. The containment tests follow
//! H3 C: ray casting in latitude and longitude (radians) with H3's
//! tie-breaking and antimeridian handling, per loop bounding box, holes
//! subtracted. Cells are found by H3 C's hierarchical traversal from the base
//! cells, including the experimental overlapping-bbox mode.

use std::collections::VecDeque;

use core::f64::consts::PI;
use h3o::{CellIndex, LatLng, Resolution};

const EPS: f64 = f64::EPSILON;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    /// Cell centers inside the polygon (H3 C polygonToCells).
    Center,
    /// Whole cell boundaries inside the polygon.
    Full,
    /// Any overlap between the cell and the polygon.
    Overlap,
    /// Any overlap with the buffered cell bounding box used by H3's
    /// experimental hierarchical fill.
    OverlapBbox,
}

#[derive(Clone, Debug)]
struct Loop {
    pts: Vec<(f64, f64)>,
    south: f64,
    north: f64,
    west: f64,
    east: f64,
    transmeridian: bool,
}

impl Loop {
    /// `pts` are (lat, lng) in radians, open (no repeated closing vertex).
    fn new(pts: Vec<(f64, f64)>) -> Loop {
        let (mut s, mut n, mut w, mut e) = (f64::MAX, -f64::MAX, f64::MAX, -f64::MAX);
        let (mut min_pos, mut max_neg) = (f64::MAX, -f64::MAX);
        let mut tm = false;
        for i in 0..pts.len() {
            let (lat, lng) = pts[i];
            let next = pts[(i + 1) % pts.len()];
            s = s.min(lat);
            n = n.max(lat);
            w = w.min(lng);
            e = e.max(lng);
            if lng > 0.0 && lng < min_pos {
                min_pos = lng;
            }
            if lng < 0.0 && lng > max_neg {
                max_neg = lng;
            }
            if (lng - next.1).abs() > PI {
                tm = true;
            }
        }
        if tm {
            e = max_neg;
            w = min_pos;
        }
        Loop {
            pts,
            south: s,
            north: n,
            west: w,
            east: e,
            transmeridian: tm,
        }
    }

    fn norm(&self, lng: f64) -> f64 {
        if self.transmeridian && lng < 0.0 {
            lng + 2.0 * PI
        } else {
            lng
        }
    }

    fn bbox(north: f64, south: f64, east: f64, west: f64) -> Loop {
        Loop {
            pts: vec![(north, east), (north, west), (south, west), (south, east)],
            south,
            north,
            west,
            east,
            transmeridian: east < west,
        }
    }

    fn bbox_contains(&self, lat: f64, lng: f64) -> bool {
        let in_lat = lat >= self.south && lat <= self.north;
        let in_lng = if self.transmeridian {
            lng >= self.west || lng <= self.east
        } else {
            lng >= self.west && lng <= self.east
        };
        in_lat && in_lng
    }

    /// H3 C pointInsideGeoLoop.
    fn contains(&self, lat0: f64, lng0: f64) -> bool {
        self.contains_with_bbox(self, lat0, lng0)
    }

    /// H3 C pointInsideGeoLoop when the caller supplies a conservative bbox
    /// for this boundary, as polygonToCells does for cells near a pole.
    fn contains_with_bbox(&self, bbox: &Loop, lat0: f64, lng0: f64) -> bool {
        if !bbox.bbox_contains(lat0, lng0) {
            return false;
        }
        let mut lat = lat0;
        let mut lng = bbox.norm(lng0);
        let mut inside = false;
        for i in 0..self.pts.len() {
            let (mut a, mut b) = (self.pts[i], self.pts[(i + 1) % self.pts.len()]);
            if a.0 > b.0 {
                core::mem::swap(&mut a, &mut b);
            }
            if lat == a.0 || lat == b.0 {
                lat += EPS;
            }
            if lat < a.0 || lat > b.0 {
                continue;
            }
            let (al, bl) = (bbox.norm(a.1), bbox.norm(b.1));
            if al == lng || bl == lng {
                lng -= EPS;
            }
            let ratio = (lat - a.0) / (b.0 - a.0);
            let test = bbox.norm(al + (bl - al) * ratio);
            if test > lng {
                inside = !inside;
            }
        }
        inside
    }

    fn edges(&self) -> impl Iterator<Item = ((f64, f64), (f64, f64))> + '_ {
        (0..self.pts.len()).map(|i| {
            let (a, b) = (self.pts[i], self.pts[(i + 1) % self.pts.len()]);
            ((a.0, self.norm(a.1)), (b.0, self.norm(b.1)))
        })
    }
}

#[derive(Clone, Debug)]
pub struct Polygon {
    outer: Loop,
    holes: Vec<Loop>,
}

impl Polygon {
    /// Rings of (lat, lng) degrees; the first is the outer ring, the rest holes.
    pub fn new(rings: &[Vec<(f64, f64)>]) -> Polygon {
        let mut loops = rings.iter().map(|r| {
            let mut pts: Vec<(f64, f64)> = r
                .iter()
                .map(|&(la, lo)| (la.to_radians(), lo.to_radians()))
                .collect();
            if pts.len() > 1 && pts.first() == pts.last() {
                pts.pop();
            }
            Loop::new(pts)
        });
        let outer = loops.next().expect("an outer ring");
        Polygon {
            outer,
            holes: loops.collect(),
        }
    }

    /// H3 C pointInsidePolygon (radians).
    fn contains(&self, lat: f64, lng: f64) -> bool {
        self.outer.contains(lat, lng) && !self.holes.iter().any(|h| h.contains(lat, lng))
    }

    fn loops(&self) -> impl Iterator<Item = &Loop> {
        core::iter::once(&self.outer).chain(&self.holes)
    }
}

/// H3 C lineCrossesLine: Cartesian in (lat, lng).
fn line_crosses_line(a1: (f64, f64), a2: (f64, f64), b1: (f64, f64), b2: (f64, f64)) -> bool {
    let denom = (b2.1 - b1.1) * (a2.0 - a1.0) - (b2.0 - b1.0) * (a2.1 - a1.1);
    if denom == 0.0 {
        return false;
    }
    let t = ((b2.0 - b1.0) * (a1.1 - b1.1) - (b2.1 - b1.1) * (a1.0 - b1.0)) / denom;
    if !(0.0..=1.0).contains(&t) {
        return false;
    }
    let u = ((a2.0 - a1.0) * (a1.1 - b1.1) - (a2.1 - a1.1) * (a1.0 - b1.0)) / denom;
    (0.0..=1.0).contains(&u)
}

#[derive(Clone, Copy, PartialEq)]
enum Norm {
    None,
    East,
    West,
}

fn normalize(lng: f64, n: Norm) -> f64 {
    match n {
        Norm::East if lng < 0.0 => lng + 2.0 * PI,
        Norm::West if lng > 0.0 => lng - 2.0 * PI,
        _ => lng,
    }
}

/// H3 C bboxNormalization.
fn normalization(a: &Loop, b: &Loop) -> (Norm, Norm) {
    let (at, bt) = (a.east < a.west, b.east < b.west);
    let east = a.west - b.east < b.west - a.east;
    let na = if !at {
        Norm::None
    } else if bt || east {
        Norm::East
    } else {
        Norm::West
    };
    let nb = if !bt {
        Norm::None
    } else if at {
        Norm::East
    } else if east {
        Norm::West
    } else {
        Norm::East
    };
    (na, nb)
}

/// H3 C bboxOverlapsBBox.
fn bbox_overlaps(a: &Loop, b: &Loop) -> bool {
    if a.north < b.south || a.south > b.north {
        return false;
    }
    let (na, nb) = normalization(a, b);
    !(normalize(a.east, na) < normalize(b.west, nb)
        || normalize(a.west, na) > normalize(b.east, nb))
}

/// H3 C bboxContainsBBox.
fn bbox_contains_bbox(a: &Loop, b: &Loop) -> bool {
    if a.north < b.north || a.south > b.south {
        return false;
    }
    let (na, nb) = normalization(a, b);
    normalize(a.west, na) <= normalize(b.west, nb) && normalize(a.east, na) >= normalize(b.east, nb)
}

/// H3 C cellBoundaryCrossesGeoLoop; `cell` is the boundary as a loop.
fn crosses_loop(l: &Loop, boundary: &Loop, boundary_bbox: &Loop) -> bool {
    if !bbox_overlaps(l, boundary_bbox) {
        return false;
    }
    let (nl, nc) = normalization(l, boundary_bbox);
    let verts: Vec<(f64, f64)> = boundary
        .pts
        .iter()
        .map(|&(la, lo)| (la, normalize(lo, nc)))
        .collect();
    let (east, west) = (
        normalize(boundary_bbox.east, nc),
        normalize(boundary_bbox.west, nc),
    );
    for i in 0..l.pts.len() {
        let a = (l.pts[i].0, normalize(l.pts[i].1, nl));
        let b = l.pts[(i + 1) % l.pts.len()];
        let b = (b.0, normalize(b.1, nl));
        if (a.0 >= boundary_bbox.north && b.0 >= boundary_bbox.north)
            || (a.0 <= boundary_bbox.south && b.0 <= boundary_bbox.south)
            || (a.1 <= west && b.1 <= west)
            || (a.1 >= east && b.1 >= east)
        {
            continue;
        }
        if (0..verts.len()).any(|j| line_crosses_line(a, b, verts[j], verts[(j + 1) % verts.len()]))
        {
            return true;
        }
    }
    false
}

fn boundary(cell: CellIndex) -> Loop {
    Loop::new(
        cell.boundary()
            .iter()
            .map(|ll| (ll.lat_radians(), ll.lng_radians()))
            .collect(),
    )
}

const MAX_EDGE_LENGTH_RADS: [f64; 16] = [
    0.21577206265130,
    0.08308767068495,
    0.03148970436439,
    0.01190662871439,
    0.00450053330908,
    0.00170105523619,
    0.00064293917678,
    0.00024300820659,
    0.00009184847087,
    0.00003471545901,
    0.00001312121017,
    0.00000495935129,
    0.00000187445860,
    0.00000070847876,
    0.00000026777980,
    0.00000010121125,
];

const NORTH_POLE_CELLS: [u64; 16] = [
    0x8001fffffffffff,
    0x81033ffffffffff,
    0x820327fffffffff,
    0x830326fffffffff,
    0x8403263ffffffff,
    0x85032623fffffff,
    0x860326237ffffff,
    0x870326233ffffff,
    0x880326233bfffff,
    0x890326233abffff,
    0x8a0326233ab7fff,
    0x8b0326233ab0fff,
    0x8c0326233ab03ff,
    0x8d0326233ab03bf,
    0x8e0326233ab039f,
    0x8f0326233ab0399,
];

const SOUTH_POLE_CELLS: [u64; 16] = [
    0x80f3fffffffffff,
    0x81f2bffffffffff,
    0x82f297fffffffff,
    0x83f293fffffffff,
    0x84f2939ffffffff,
    0x85f29383fffffff,
    0x86f29380fffffff,
    0x87f29380effffff,
    0x88f29380e1fffff,
    0x89f29380e0fffff,
    0x8af29380e0d7fff,
    0x8bf29380e0d0fff,
    0x8cf29380e0d0dff,
    0x8df29380e0d0cff,
    0x8ef29380e0d0cc7,
    0x8ff29380e0d0cc4,
];

/// H3 C cellToBBox. Resolution 0 only reaches this function when resolution
/// 0 itself is requested; coarser traversal descends all base cells so it does
/// not need H3's 122-entry precomputed optimization table.
fn cell_bbox(cell: CellIndex, cover_children: bool) -> Loop {
    let res = usize::from(cell.resolution());
    let (mut north, mut south, mut east, mut west) = if res == 0 {
        let b = boundary(cell);
        (b.north, b.south, b.east, b.west)
    } else {
        let center = LatLng::from(cell);
        let edge = MAX_EDGE_LENGTH_RADS[res];
        let lng_edge = edge / libm::cos(center.lat_radians());
        (
            center.lat_radians() + edge,
            center.lat_radians() - edge,
            center.lng_radians() + lng_edge,
            center.lng_radians() - lng_edge,
        )
    };
    let width = if east < west {
        east - west + 2.0 * PI
    } else {
        east - west
    };
    let factor = if cover_children { 1.4 } else { 1.1 };
    let width_buffer = (width * factor - width) * 0.5;
    let height = north - south;
    let height_buffer = (height * factor - height) * 0.5;
    north = (north + height_buffer).min(PI / 2.0);
    south = (south - height_buffer).max(-PI / 2.0);
    east += width_buffer;
    if east > PI {
        east -= 2.0 * PI;
    }
    if east < -PI {
        east += 2.0 * PI;
    }
    west -= width_buffer;
    if west > PI {
        west -= 2.0 * PI;
    }
    if west < -PI {
        west += 2.0 * PI;
    }
    let raw = u64::from(cell);
    if raw == NORTH_POLE_CELLS[res] {
        north = PI / 2.0;
    }
    if raw == SOUTH_POLE_CELLS[res] {
        south = -PI / 2.0;
    }
    if north == PI / 2.0 || south == -PI / 2.0 {
        east = PI;
        west = -PI;
    }
    Loop::bbox(north, south, east, west)
}

/// H3 C cellBoundaryInsidePolygon.
fn boundary_inside(poly: &Polygon, b: &Loop, bbox: &Loop) -> bool {
    let (la, lo) = b.pts[0];
    poly.contains(la, lo)
        && !crosses_loop(&poly.outer, b, bbox)
        && !poly.holes.iter().any(|h| {
            !h.pts.is_empty()
                && (b.contains_with_bbox(bbox, h.pts[0].0, h.pts[0].1) || crosses_loop(h, b, bbox))
        })
}

fn keep(poly: &Polygon, cell: CellIndex, res: Resolution, mode: Mode) -> bool {
    let c = LatLng::from(cell);
    match mode {
        Mode::Center => poly.contains(c.lat_radians(), c.lng_radians()),
        Mode::Full => {
            let b = boundary(cell);
            boundary_inside(poly, &b, &cell_bbox(cell, false))
        }
        Mode::Overlap | Mode::OverlapBbox => {
            if poly.contains(c.lat_radians(), c.lng_radians()) {
                return true;
            }
            // The cell holding the polygon's first vertex (H3 C's check for a
            // polygon wholly inside one cell).
            let (la, lo) = poly.outer.pts[0];
            if LatLng::from_radians(la, lo).is_ok_and(|v| v.to_cell(res) == cell) {
                return true;
            }
            let b = boundary(cell);
            let bbox = cell_bbox(cell, false);
            if mode == Mode::OverlapBbox && boundary_inside(poly, &b, &bbox) {
                return true;
            }
            if poly.loops().any(|l| crosses_loop(l, &b, &bbox)) {
                return true;
            }
            if mode == Mode::Overlap {
                return false;
            }
            let bbox = cell_bbox(cell, true);
            bbox_overlaps(&poly.outer, &bbox)
                && (bbox_contains_bbox(&bbox, &poly.outer)
                    || poly.contains(bbox.pts[0].0, bbox.pts[0].1)
                    || poly.loops().any(|l| crosses_loop(l, &bbox, &bbox)))
        }
    }
}

/// Rough number of cells before filling: bounding-box area over the average
/// cell area, doubled for shape and edge effects.
pub fn estimate(poly: &Polygon, res: Resolution) -> f64 {
    let o = &poly.outer;
    let width = if o.transmeridian {
        o.east + 2.0 * PI - o.west
    } else {
        o.east - o.west
    };
    let r = 6_371.007_180_918_475_f64;
    let area = r * r * width.max(0.0) * (libm::sin(o.north) - libm::sin(o.south)).abs();
    2.0 * area / res.area_km2() + 12.0
}

/// The number of edge samples the fill will seed from: its cost grows with the
/// perimeter at this resolution, which the area estimate cannot see.
pub fn edge_samples(poly: &Polygon, res: Resolution) -> f64 {
    let step = res.edge_length_km() / 4.0 / 6_371.007_180_918_475;
    poly.loops()
        .flat_map(|l| l.edges())
        .map(|((la, lo), (lb, lob))| {
            ((lb - la).hypot((lob - lo) * libm::cos((la + lb) / 2.0)) / step).ceil() + 1.0
        })
        .sum()
}

/// Fills a polygon; `Err(n)` when more than `limit` cells would result.
pub fn fill(
    poly: &Polygon,
    res: Resolution,
    mode: Mode,
    limit: usize,
) -> Result<Vec<CellIndex>, usize> {
    let mut queue: VecDeque<CellIndex> = CellIndex::base_cells().collect();
    let mut out = Vec::new();
    while let Some(c) = queue.pop_front() {
        let cell_res = c.resolution();
        if cell_res == res {
            if keep(poly, c, res, mode) {
                out.push(c);
                if out.len() > limit {
                    return Err(out.len());
                }
            }
            continue;
        }

        // H3 uses a precomputed bounding box for each base cell. Descending
        // their immediate children instead is equivalent and avoids copying a
        // large optimization table into the Wasm core.
        let next = cell_res.succ().expect("coarser than the target");
        if cell_res == Resolution::Zero {
            queue.extend(c.children(next));
            continue;
        }

        let bbox = cell_bbox(c, true);
        if !bbox_overlaps(&poly.outer, &bbox) {
            continue;
        }
        if bbox_contains_bbox(&poly.outer, &bbox) && boundary_inside(poly, &bbox, &bbox) {
            let count = c.children_count(res) as usize;
            if out.len().saturating_add(count) > limit {
                return Err(out.len().saturating_add(count));
            }
            out.extend(c.children(res));
        } else {
            queue.extend(c.children(next));
        }
    }
    out.sort_unstable();
    Ok(out)
}
