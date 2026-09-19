//! H3 polygonToCells (hexagonal-grids spec, "Polyfill with explicit
//! containment and limits") on h3o's core API. The containment tests follow
//! H3 C: ray casting in latitude and longitude (radians) with H3's
//! tie-breaking and antimeridian handling, per loop bounding box, holes
//! subtracted. Cells are found by a breadth-first fill seeded from cells
//! sampled densely along every edge.

use std::collections::{BTreeSet, VecDeque};

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
        if !self.bbox_contains(lat0, lng0) {
            return false;
        }
        let mut lat = lat0;
        let mut lng = self.norm(lng0);
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
            let (al, bl) = (self.norm(a.1), self.norm(b.1));
            if al == lng || bl == lng {
                lng -= EPS;
            }
            let ratio = (lat - a.0) / (b.0 - a.0);
            let test = self.norm(al + (bl - al) * ratio);
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

/// H3 C cellBoundaryCrossesGeoLoop; `cell` is the boundary as a loop.
fn crosses_loop(l: &Loop, cell: &Loop) -> bool {
    if !bbox_overlaps(l, cell) {
        return false;
    }
    let (nl, nc) = normalization(l, cell);
    let verts: Vec<(f64, f64)> = cell
        .pts
        .iter()
        .map(|&(la, lo)| (la, normalize(lo, nc)))
        .collect();
    let (east, west) = (normalize(cell.east, nc), normalize(cell.west, nc));
    for i in 0..l.pts.len() {
        let a = (l.pts[i].0, normalize(l.pts[i].1, nl));
        let b = l.pts[(i + 1) % l.pts.len()];
        let b = (b.0, normalize(b.1, nl));
        if (a.0 >= cell.north && b.0 >= cell.north)
            || (a.0 <= cell.south && b.0 <= cell.south)
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

/// H3 C cellBoundaryInsidePolygon.
fn boundary_inside(poly: &Polygon, b: &Loop) -> bool {
    let (la, lo) = b.pts[0];
    poly.contains(la, lo)
        && !crosses_loop(&poly.outer, b)
        && !poly.holes.iter().any(|h| {
            !h.pts.is_empty() && (b.contains(h.pts[0].0, h.pts[0].1) || crosses_loop(h, b))
        })
}

fn keep(poly: &Polygon, cell: CellIndex, res: Resolution, mode: Mode) -> bool {
    let c = LatLng::from(cell);
    match mode {
        Mode::Center => poly.contains(c.lat_radians(), c.lng_radians()),
        Mode::Full => boundary_inside(poly, &boundary(cell)),
        Mode::Overlap => {
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
            poly.loops().any(|l| crosses_loop(l, &b))
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
    // Seed cells sampled along every edge, at a quarter of the cell edge.
    let step = res.edge_length_km() / 4.0 / 6_371.007_180_918_475;
    let mut seeds = BTreeSet::new();
    for l in poly.loops() {
        for ((la, lo), (lb, lob)) in l.edges() {
            let dist = (lb - la).hypot((lob - lo) * libm::cos((la + lb) / 2.0));
            let n = (dist / step).ceil().max(1.0) as usize;
            for k in 0..=n {
                let t = k as f64 / n as f64;
                let lat = la + (lb - la) * t;
                let mut lng = lo + (lob - lo) * t;
                if lng > PI {
                    lng -= 2.0 * PI;
                }
                if let Ok(ll) = LatLng::from_radians(lat, lng) {
                    seeds.insert(ll.to_cell(res));
                }
            }
        }
    }
    let mut seen: BTreeSet<CellIndex> = BTreeSet::new();
    let mut queue: VecDeque<CellIndex> = VecDeque::new();
    for s in &seeds {
        for c in s.grid_disk::<Vec<_>>(1) {
            if seen.insert(c) {
                queue.push_back(c);
            }
        }
    }
    let mut out = Vec::new();
    while let Some(c) = queue.pop_front() {
        let near_edge = seeds.contains(&c);
        let hit = keep(poly, c, res, mode);
        if hit {
            out.push(c);
            if out.len() > limit {
                return Err(out.len());
            }
        }
        // Grow through kept cells, and through any cell whose center is
        // inside (full mode keeps fewer cells than lie inside).
        let grow = hit || near_edge || (mode == Mode::Full && keep(poly, c, res, Mode::Center));
        if grow {
            for n in c.grid_disk::<Vec<_>>(1) {
                if seen.insert(n) {
                    queue.push_back(n);
                }
            }
        }
    }
    out.sort_unstable();
    Ok(out)
}
