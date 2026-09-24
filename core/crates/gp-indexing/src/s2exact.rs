//! S2's own RegionCoverer for latitude-longitude rectangles and caps, ported
//! from s2sphere (the Python port of the C++ library): the exact cell tests
//! of S2Cap and S2LatLngRect, the cell's latitude-longitude and cap bounds,
//! the priority queue that decides which candidate to split, and the cell
//! union's normalization. A covering from here is the one S2 itself gives,
//! so it can be exchanged with any system built on S2.
//!
//! Only what the coverer needs is here; the cell ids themselves are `s2`.

use crate::s2::{self, CellId, MAX_LEVEL};
use core::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};
use libm::{asin, atan2, cos, frexp, remainder, sin, sqrt};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

type P = [f64; 3];

fn dot(a: P, b: P) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: P, b: P) -> P {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn neg(a: P) -> P {
    [-a[0], -a[1], -a[2]]
}

fn norm2(a: P) -> f64 {
    dot(a, a)
}

fn unit(a: P) -> P {
    let n = sqrt(norm2(a));
    if n == 0.0 {
        a
    } else {
        [a[0] / n, a[1] / n, a[2] / n]
    }
}

/// Latitude and longitude (radians) to a unit vector, as s2sphere does it.
fn to_point(lat: f64, lng: f64) -> P {
    let cp = cos(lat);
    [cos(lng) * cp, sin(lng) * cp, sin(lat)]
}

fn latitude(p: P) -> f64 {
    atan2(p[2], sqrt(p[0] * p[0] + p[1] * p[1]))
}

fn longitude(p: P) -> f64 {
    atan2(p[1], p[0])
}

// ---------------------------------------------------------------- intervals

#[derive(Clone, Copy, Debug)]
struct Line {
    lo: f64,
    hi: f64,
}

impl Line {
    fn from_pair(a: f64, b: f64) -> Line {
        if a <= b {
            Line { lo: a, hi: b }
        } else {
            Line { lo: b, hi: a }
        }
    }
    fn is_empty(self) -> bool {
        self.lo > self.hi
    }
    fn contains(self, o: Line) -> bool {
        o.is_empty() || (o.lo >= self.lo && o.hi <= self.hi)
    }
    fn intersects(self, o: Line) -> bool {
        if self.lo <= o.lo {
            o.lo <= self.hi && o.lo <= o.hi
        } else {
            self.lo <= o.hi && self.lo <= self.hi
        }
    }
    fn expanded(self, r: f64) -> Line {
        if self.is_empty() {
            self
        } else {
            Line {
                lo: self.lo - r,
                hi: self.hi + r,
            }
        }
    }
    fn intersection(self, o: Line) -> Line {
        Line {
            lo: self.lo.max(o.lo),
            hi: self.hi.min(o.hi),
        }
    }
    fn center(self) -> f64 {
        0.5 * (self.lo + self.hi)
    }
    fn bound(self, i: usize) -> f64 {
        if i == 0 { self.lo } else { self.hi }
    }
}

/// An interval of longitude on the circle, which may wrap past ±π.
#[derive(Clone, Copy, Debug)]
struct Sphere {
    lo: f64,
    hi: f64,
}

impl Sphere {
    fn new(lo: f64, hi: f64) -> Sphere {
        let (mut l, mut h) = (lo, hi);
        if lo == -PI && hi != PI {
            l = PI;
        }
        if hi == -PI && lo != PI {
            h = PI;
        }
        Sphere { lo: l, hi: h }
    }
    fn full() -> Sphere {
        Sphere { lo: -PI, hi: PI }
    }
    fn positive_distance(a: f64, b: f64) -> f64 {
        let d = b - a;
        if d >= 0.0 { d } else { (b + PI) - (a - PI) }
    }
    fn from_pair(a: f64, b: f64) -> Sphere {
        let a = if a == -PI { PI } else { a };
        let b = if b == -PI { PI } else { b };
        if Sphere::positive_distance(a, b) <= PI {
            Sphere { lo: a, hi: b }
        } else {
            Sphere { lo: b, hi: a }
        }
    }
    fn is_full(self) -> bool {
        self.hi - self.lo == 2.0 * PI
    }
    fn is_inverted(self) -> bool {
        self.lo > self.hi
    }
    fn is_empty(self) -> bool {
        self.lo - self.hi == 2.0 * PI
    }
    fn center(self) -> f64 {
        let c = 0.5 * (self.lo + self.hi);
        if !self.is_inverted() {
            c
        } else if c <= 0.0 {
            c + PI
        } else {
            c - PI
        }
    }
    fn length(self) -> f64 {
        let mut l = self.hi - self.lo;
        if l >= 0.0 {
            return l;
        }
        l += 2.0 * PI;
        if l > 0.0 { l } else { -1.0 }
    }
    fn contains(self, o: Sphere) -> bool {
        if self.is_inverted() {
            if o.is_inverted() {
                return o.lo >= self.lo && o.hi <= self.hi;
            }
            (o.lo >= self.lo || o.hi <= self.hi) && !self.is_empty()
        } else {
            if o.is_inverted() {
                return self.is_full() || o.is_empty();
            }
            o.lo >= self.lo && o.hi <= self.hi
        }
    }
    fn intersects(self, o: Sphere) -> bool {
        if self.is_empty() || o.is_empty() {
            return false;
        }
        if self.is_inverted() {
            o.is_inverted() || o.lo <= self.hi || o.hi >= self.lo
        } else if o.is_inverted() {
            o.lo <= self.hi || o.hi >= self.lo
        } else {
            o.lo <= self.hi && o.hi >= self.lo
        }
    }
    fn expanded(self, r: f64) -> Sphere {
        if self.is_empty() {
            return self;
        }
        if self.length() + 2.0 * r >= 2.0 * PI - 1e-15 {
            return Sphere::full();
        }
        let mut lo = remainder(self.lo - r, 2.0 * PI);
        let hi = remainder(self.hi + r, 2.0 * PI);
        if lo <= -PI {
            lo = PI;
        }
        Sphere::new(lo, hi)
    }
    fn bound(self, i: usize) -> f64 {
        if i == 0 { self.lo } else { self.hi }
    }
}

// ---------------------------------------------------------------- regions

#[derive(Clone, Copy, Debug)]
struct Rect {
    lat: Line,
    lng: Sphere,
}

impl Rect {
    fn is_empty(self) -> bool {
        self.lat.is_empty()
    }
    fn contains(self, o: Rect) -> bool {
        self.lat.contains(o.lat) && self.lng.contains(o.lng)
    }
    fn intersects(self, o: Rect) -> bool {
        self.lat.intersects(o.lat) && self.lng.intersects(o.lng)
    }
    fn vertex(self, k: usize) -> P {
        to_point(self.lat.bound(k >> 1), self.lng.bound((k >> 1) ^ (k & 1)))
    }
    fn cap_bound(self) -> Cap {
        if self.is_empty() {
            return Cap::empty();
        }
        let (pole_z, pole_angle) = if self.lat.lo + self.lat.hi < 0.0 {
            (-1.0, FRAC_PI_2 + self.lat.hi)
        } else {
            (1.0, FRAC_PI_2 - self.lat.lo)
        };
        let pole_cap = Cap::from_axis_angle([0.0, 0.0, pole_z], pole_angle);
        let lng_span = self.lng.hi - self.lng.lo;
        if remainder(lng_span, 2.0 * PI) >= 0.0 && lng_span < 2.0 * PI {
            let mut mid = Cap::from_axis_angle(to_point(self.lat.center(), self.lng.center()), 0.0);
            for k in 0..4 {
                mid.add_point(self.vertex(k));
            }
            if mid.height < pole_cap.height {
                return mid;
            }
        }
        pole_cap
    }
}

#[derive(Clone, Copy, Debug)]
struct Cap {
    axis: P,
    height: f64,
}

impl Cap {
    fn empty() -> Cap {
        Cap {
            axis: [1.0, 0.0, 0.0],
            height: -1.0,
        }
    }
    fn from_axis_angle(axis: P, radians: f64) -> Cap {
        let height = if radians >= PI {
            2.0
        } else {
            let d = sin(0.5 * radians);
            2.0 * d * d
        };
        Cap { axis, height }
    }
    fn is_empty(self) -> bool {
        self.height < 0.0
    }
    fn is_full(self) -> bool {
        self.height >= 2.0
    }
    fn angle(self) -> f64 {
        if self.is_empty() {
            -1.0
        } else {
            2.0 * asin(sqrt(0.5 * self.height))
        }
    }
    fn add_point(&mut self, p: P) {
        if self.is_empty() {
            self.axis = p;
            self.height = 0.0;
        } else {
            let d2 = norm2(sub(self.axis, p));
            self.height = self
                .height
                .max((1.0 + 1.0 / (1u64 << 52) as f64) * 0.5 * d2);
        }
    }
    fn complement(self) -> Cap {
        let height = if self.is_full() {
            -1.0
        } else {
            2.0 - self.height.max(0.0)
        };
        Cap {
            axis: neg(self.axis),
            height,
        }
    }
    fn contains_point(self, p: P) -> bool {
        norm2(sub(self.axis, p)) <= 2.0 * self.height
    }
    fn contains_cell(self, cell: &Cell) -> bool {
        let mut v = [[0.0; 3]; 4];
        for (k, slot) in v.iter_mut().enumerate() {
            *slot = cell.vertex(k);
            if !self.contains_point(*slot) {
                return false;
            }
        }
        !self.complement().intersects_cell(cell, &v)
    }
    fn intersects_cell(self, cell: &Cell, v: &[P; 4]) -> bool {
        if self.height >= 1.0 || self.is_empty() {
            return false;
        }
        if cell.contains_point(self.axis) {
            return true;
        }
        let sin2 = self.height * (2.0 - self.height);
        for k in 0..4 {
            let edge = cell.edge_raw(k);
            let d = dot(self.axis, edge);
            if d > 0.0 {
                continue;
            }
            if d * d > sin2 * norm2(edge) {
                return false;
            }
            let dir = cross(edge, self.axis);
            if dot(dir, v[k]) < 0.0 && dot(dir, v[(k + 1) & 3]) > 0.0 {
                return true;
            }
        }
        false
    }
    fn may_intersect_cell(self, cell: &Cell) -> bool {
        let mut v = [[0.0; 3]; 4];
        for (k, slot) in v.iter_mut().enumerate() {
            *slot = cell.vertex(k);
            if self.contains_point(*slot) {
                return true;
            }
        }
        self.intersects_cell(cell, &v)
    }
}

/// The two regions S2's coverer takes here.
pub enum Region {
    /// Degrees: south, north, west, east (east below west crosses 180).
    Rect {
        south: f64,
        north: f64,
        west: f64,
        east: f64,
    },
    /// Center in degrees and angular radius in radians.
    Cap { lat: f64, lon: f64, radius: f64 },
}

enum Shape {
    Rect(Rect),
    Cap(Cap),
}

impl Shape {
    fn new(r: &Region) -> Shape {
        match *r {
            Region::Rect {
                south,
                north,
                west,
                east,
            } => Shape::Rect(Rect {
                lat: Line {
                    lo: south.to_radians(),
                    hi: north.to_radians(),
                },
                lng: Sphere::new(west.to_radians(), east.to_radians()),
            }),
            Region::Cap { lat, lon, radius } => Shape::Cap(Cap::from_axis_angle(
                to_point(lat.to_radians(), lon.to_radians()),
                radius,
            )),
        }
    }
    fn may_intersect(&self, c: &Cell) -> bool {
        match self {
            Shape::Rect(r) => r.intersects(c.rect_bound()),
            Shape::Cap(cap) => cap.may_intersect_cell(c),
        }
    }
    fn contains(&self, c: &Cell) -> bool {
        match self {
            Shape::Rect(r) => r.contains(c.rect_bound()),
            Shape::Cap(cap) => cap.contains_cell(c),
        }
    }
    fn cap_bound(&self) -> Cap {
        match self {
            Shape::Rect(r) => r.cap_bound(),
            Shape::Cap(c) => *c,
        }
    }
}

// ---------------------------------------------------------------- cells

fn face_uv_to_xyz(face: u8, u: f64, v: f64) -> P {
    match face {
        0 => [1.0, u, v],
        1 => [-u, 1.0, v],
        2 => [-u, -v, 1.0],
        3 => [-1.0, -v, -u],
        4 => [v, -1.0, -u],
        _ => [v, u, -1.0],
    }
}

fn valid_face_xyz_to_uv(face: u8, p: P) -> (f64, f64) {
    match face {
        0 => (p[1] / p[0], p[2] / p[0]),
        1 => (-p[0] / p[1], p[2] / p[1]),
        2 => (-p[0] / p[2], -p[1] / p[2]),
        3 => (p[2] / p[0], p[1] / p[0]),
        4 => (p[2] / p[1], -p[0] / p[1]),
        _ => (-p[1] / p[2], -p[0] / p[2]),
    }
}

/// The face a direction falls on, with s2sphere's ties (the later axis wins).
fn xyz_to_face_uv(p: P) -> (u8, f64, f64) {
    let a = [p[0].abs(), p[1].abs(), p[2].abs()];
    let mut face = if a[0] > a[1] {
        if a[0] > a[2] { 0 } else { 2 }
    } else if a[1] > a[2] {
        1
    } else {
        2
    };
    if p[face as usize] < 0.0 {
        face += 3;
    }
    let (u, v) = valid_face_xyz_to_uv(face, p);
    (face, u, v)
}

fn u_norm(face: u8, u: f64) -> P {
    match face {
        0 => [u, -1.0, 0.0],
        1 => [1.0, u, 0.0],
        2 => [1.0, 0.0, u],
        3 => [-u, 0.0, 1.0],
        4 => [0.0, -u, 1.0],
        _ => [0.0, -1.0, -u],
    }
}

fn v_norm(face: u8, v: f64) -> P {
    match face {
        0 => [-v, 0.0, 1.0],
        1 => [0.0, -v, 1.0],
        2 => [0.0, -1.0, -v],
        3 => [v, -1.0, 0.0],
        4 => [1.0, v, 0.0],
        _ => [1.0, 0.0, v],
    }
}

/// The z component of a face's u and v axes, which decides which corner of a
/// cell is its highest and lowest latitude.
fn u_axis_z(face: u8) -> f64 {
    if matches!(face, 3 | 4) { -1.0 } else { 0.0 }
}

fn v_axis_z(face: u8) -> f64 {
    if matches!(face, 0 | 1) { 1.0 } else { 0.0 }
}

const MAX_SIZE: i64 = 1 << 30;

struct Cell {
    id: CellId,
    face: u8,
    level: u8,
    /// uv[0] is the u range, uv[1] the v range.
    uv: [[f64; 2]; 2],
}

impl Cell {
    fn new(id: CellId) -> Cell {
        let (face, i, j) = id.to_face_ij();
        let size = 1i64 << (30 - id.level());
        let uv_of = |v: i64| s2::st_to_uv((1.0 / MAX_SIZE as f64) * v as f64);
        let (i, j) = (i64::from(i), i64::from(j));
        Cell {
            id,
            face,
            level: id.level(),
            uv: [[uv_of(i), uv_of(i + size)], [uv_of(j), uv_of(j + size)]],
        }
    }
    fn vertex_raw(&self, k: usize) -> P {
        face_uv_to_xyz(
            self.face,
            self.uv[0][(k >> 1) ^ (k & 1)],
            self.uv[1][k >> 1],
        )
    }
    fn vertex(&self, k: usize) -> P {
        unit(self.vertex_raw(k))
    }
    fn edge_raw(&self, k: usize) -> P {
        match k {
            0 => v_norm(self.face, self.uv[1][0]),
            1 => u_norm(self.face, self.uv[0][1]),
            2 => neg(v_norm(self.face, self.uv[1][1])),
            _ => neg(u_norm(self.face, self.uv[0][0])),
        }
    }
    fn contains_point(&self, p: P) -> bool {
        let f = self.face as usize;
        let valid = if f < 3 { p[f] > 0.0 } else { p[f - 3] < 0.0 };
        if !valid {
            return false;
        }
        let (u, v) = valid_face_xyz_to_uv(self.face, p);
        u >= self.uv[0][0] && u <= self.uv[0][1] && v >= self.uv[1][0] && v <= self.uv[1][1]
    }
    fn corner_lat(&self, i: usize, j: usize) -> f64 {
        latitude(face_uv_to_xyz(self.face, self.uv[0][i], self.uv[1][j]))
    }
    fn corner_lng(&self, i: usize, j: usize) -> f64 {
        longitude(face_uv_to_xyz(self.face, self.uv[0][i], self.uv[1][j]))
    }
    fn rect_bound(&self) -> Rect {
        if self.level > 0 {
            let u = self.uv[0][0] + self.uv[0][1];
            let v = self.uv[1][0] + self.uv[1][1];
            let i = if u_axis_z(self.face) == 0.0 {
                usize::from(u < 0.0)
            } else {
                usize::from(u > 0.0)
            };
            let j = if v_axis_z(self.face) == 0.0 {
                usize::from(v < 0.0)
            } else {
                usize::from(v > 0.0)
            };
            let max_error = 1.0 / (1u64 << 51) as f64;
            let lat = Line::from_pair(self.corner_lat(i, j), self.corner_lat(1 - i, 1 - j))
                .expanded(max_error)
                .intersection(Line {
                    lo: -FRAC_PI_2,
                    hi: FRAC_PI_2,
                });
            if lat.lo == -FRAC_PI_2 || lat.hi == FRAC_PI_2 {
                return Rect {
                    lat,
                    lng: Sphere::full(),
                };
            }
            let lng = Sphere::from_pair(self.corner_lng(i, 1 - j), self.corner_lng(1 - i, j));
            return Rect {
                lat,
                lng: lng.expanded(max_error),
            };
        }
        let pole_min_lat = asin(sqrt(1.0 / 3.0));
        let band = Line {
            lo: -FRAC_PI_4,
            hi: FRAC_PI_4,
        };
        match self.face {
            0 => Rect {
                lat: band,
                lng: Sphere::new(-FRAC_PI_4, FRAC_PI_4),
            },
            1 => Rect {
                lat: band,
                lng: Sphere::new(FRAC_PI_4, 3.0 * FRAC_PI_4),
            },
            2 => Rect {
                lat: Line {
                    lo: pole_min_lat,
                    hi: FRAC_PI_2,
                },
                lng: Sphere::full(),
            },
            3 => Rect {
                lat: band,
                lng: Sphere::new(3.0 * FRAC_PI_4, -3.0 * FRAC_PI_4),
            },
            4 => Rect {
                lat: band,
                lng: Sphere::new(-3.0 * FRAC_PI_4, -FRAC_PI_4),
            },
            _ => Rect {
                lat: Line {
                    lo: -FRAC_PI_2,
                    hi: -pole_min_lat,
                },
                lng: Sphere::full(),
            },
        }
    }
}

fn st_to_ij(s: f64) -> i64 {
    ((MAX_SIZE as f64 * s).floor() as i64).clamp(0, MAX_SIZE - 1)
}

fn from_point(p: P) -> CellId {
    let (face, u, v) = xyz_to_face_uv(p);
    CellId::from_face_ij(
        face,
        st_to_ij(s2::uv_to_st(u)) as u32,
        st_to_ij(s2::uv_to_st(v)) as u32,
    )
}

/// s2sphere's FromFaceIJWrap: the cell just across a face edge, by the
/// linear projection.
fn from_face_ij_wrap(face: u8, i: i64, j: i64) -> CellId {
    let i = i.clamp(-1, MAX_SIZE);
    let j = j.clamp(-1, MAX_SIZE);
    let scale = 1.0 / MAX_SIZE as f64;
    let u = scale * ((i << 1) + 1 - MAX_SIZE) as f64;
    let v = scale * ((j << 1) + 1 - MAX_SIZE) as f64;
    let (f, u, v) = xyz_to_face_uv(face_uv_to_xyz(face, u, v));
    CellId::from_face_ij(
        f,
        st_to_ij(0.5 * (u + 1.0)) as u32,
        st_to_ij(0.5 * (v + 1.0)) as u32,
    )
}

fn from_face_ij_same(face: u8, i: i64, j: i64, same: bool) -> CellId {
    if same {
        CellId::from_face_ij(face, i as u32, j as u32)
    } else {
        from_face_ij_wrap(face, i, j)
    }
}

/// The three or four cells at `level` around the vertex nearest a leaf cell.
fn vertex_neighbors(leaf: CellId, level: u8) -> Vec<CellId> {
    let (face, i, j) = leaf.to_face_ij();
    let (i, j) = (i64::from(i), i64::from(j));
    let halfsize = 1i64 << (30 - (level + 1));
    let size = halfsize << 1;
    let (ioff, isame) = if i & halfsize != 0 {
        (size, i + size < MAX_SIZE)
    } else {
        (-size, i - size >= 0)
    };
    let (joff, jsame) = if j & halfsize != 0 {
        (size, j + size < MAX_SIZE)
    } else {
        (-size, j - size >= 0)
    };
    let mut out = vec![
        leaf.parent(level),
        from_face_ij_same(face, i + ioff, j, isame).parent(level),
        from_face_ij_same(face, i, j + joff, jsame).parent(level),
    ];
    if isame || jsame {
        out.push(from_face_ij_same(face, i + ioff, j + joff, isame && jsame).parent(level));
    }
    out
}

/// The deepest level whose cells are all at least `value` radians wide.
fn min_width_max_level(value: f64) -> u8 {
    if value <= 0.0 {
        return MAX_LEVEL;
    }
    let (_, x) = frexp((2.0 * 2f64.sqrt() / 3.0) / value);
    (x - 1).clamp(0, i32::from(MAX_LEVEL)) as u8
}

// ---------------------------------------------------------------- cell unions

fn lsb(id: u64) -> u64 {
    id.isolate_lowest_one()
}

fn contains_id(a: u64, b: u64) -> bool {
    let l = lsb(a);
    b >= a - (l - 1) && b <= a + (l - 1)
}

/// S2CellUnion::Normalize: sorted, nothing inside anything else, and any four
/// siblings replaced by their parent.
fn normalize(mut ids: Vec<u64>) -> Vec<u64> {
    ids.sort_unstable();
    let mut out: Vec<u64> = Vec::new();
    for mut id in ids {
        if out.last().is_some_and(|&l| contains_id(l, id)) {
            continue;
        }
        while out.last().is_some_and(|&l| contains_id(id, l)) {
            out.pop();
        }
        while out.len() >= 3 {
            let n = out.len();
            let (a, b, c) = (out[n - 3], out[n - 2], out[n - 1]);
            if a ^ b ^ c != id {
                break;
            }
            let mask = lsb(id) << 1;
            let mask = !(mask + (mask << 1));
            let m = id & mask;
            let is_face = CellId(id).level() == 0;
            if a & mask != m || b & mask != m || c & mask != m || is_face {
                break;
            }
            out.truncate(n - 3);
            id = CellId(id).parent(CellId(id).level() - 1).0;
        }
        out.push(id);
    }
    out
}

/// Cells coarser than `min_level` replaced by their descendants at it.
fn denormalize(ids: &[u64], min_level: u8) -> Vec<CellId> {
    let mut out = Vec::new();
    for &id in ids {
        let c = CellId(id);
        if c.level() >= min_level {
            out.push(c);
            continue;
        }
        let mut level_cells = vec![c];
        while level_cells[0].level() < min_level {
            level_cells = level_cells
                .iter()
                .flat_map(|x| x.children().expect("below level 30"))
                .collect();
        }
        out.extend(level_cells);
    }
    out
}

// ---------------------------------------------------------------- the coverer

struct Candidate {
    cell: CellId,
    terminal: bool,
    children: Vec<usize>,
}

struct Coverer<'a> {
    shape: &'a Shape,
    min_level: u8,
    max_level: u8,
    max_cells: usize,
    arena: Vec<Candidate>,
    pq: BinaryHeap<Reverse<(u64, u64, usize)>>,
    result: Vec<u64>,
}

impl Coverer<'_> {
    fn new_candidate(&mut self, cell: &Cell) -> Option<usize> {
        if !self.shape.may_intersect(cell) {
            return None;
        }
        let terminal = cell.level >= self.min_level
            && (cell.level + 1 > self.max_level || self.shape.contains(cell));
        self.arena.push(Candidate {
            cell: cell.id,
            terminal,
            children: Vec::new(),
        });
        Some(self.arena.len() - 1)
    }

    fn expand_children(&mut self, parent: usize, cell: CellId, levels: u8) -> usize {
        let levels = levels - 1;
        let mut terminals = 0;
        for child in cell.children().expect("coarser than level 30") {
            let cc = Cell::new(child);
            if levels > 0 {
                if self.shape.may_intersect(&cc) {
                    terminals += self.expand_children(parent, child, levels);
                }
                continue;
            }
            if let Some(k) = self.new_candidate(&cc) {
                self.arena[parent].children.push(k);
                if self.arena[k].terminal {
                    terminals += 1;
                }
            }
        }
        terminals
    }

    fn add_candidate(&mut self, c: Option<usize>) {
        let Some(c) = c else { return };
        if self.arena[c].terminal {
            self.result.push(self.arena[c].cell.0);
            return;
        }
        let cell = self.arena[c].cell;
        let terminals = self.expand_children(c, cell, 1);
        let n = self.arena[c].children.len();
        if n == 0 {
            return;
        }
        if terminals == 4 && cell.level() >= self.min_level {
            self.arena[c].terminal = true;
            self.add_candidate(Some(c));
        } else {
            // S2's priority: coarser cells first, then those with fewer
            // children, then fewer terminal children; ties by cell id, as
            // s2sphere breaks them.
            let priority = (((u64::from(cell.level()) << 2) + n as u64) << 2) + terminals as u64;
            self.pq.push(Reverse((priority, cell.0, c)));
        }
    }

    fn run(&mut self) {
        // Initial candidates: the cells around the region's bounding cap.
        let mut started = false;
        if self.max_cells >= 4 {
            let cap = self.shape.cap_bound();
            let level =
                min_width_max_level(2.0 * cap.angle()).min(self.max_level.min(MAX_LEVEL - 1));
            if level > 0 {
                for n in vertex_neighbors(from_point(cap.axis), level) {
                    let k = self.new_candidate(&Cell::new(n));
                    self.add_candidate(k);
                }
                started = true;
            }
        }
        if !started {
            for face in 0..6u8 {
                let k = self.new_candidate(&Cell::new(CellId((u64::from(face) << 61) | (1 << 60))));
                self.add_candidate(k);
            }
        }
        while let Some(Reverse((_, _, c))) = self.pq.pop() {
            let level = self.arena[c].cell.level();
            let n = self.arena[c].children.len();
            if level < self.min_level
                || n == 1
                || self.result.len() + self.pq.len() + n <= self.max_cells
            {
                let children = std::mem::take(&mut self.arena[c].children);
                for k in children {
                    self.add_candidate(Some(k));
                }
            } else {
                self.arena[c].terminal = true;
                self.arena[c].children.clear();
                self.add_candidate(Some(c));
            }
        }
    }
}

/// S2's covering of a rectangle or a cap: at most `max_cells` cells where
/// the level range allows, none coarser than `min_level` or finer than
/// `max_level`, sorted by id.
pub fn covering(region: &Region, min_level: u8, max_level: u8, max_cells: usize) -> Vec<CellId> {
    let shape = Shape::new(region);
    let mut c = Coverer {
        shape: &shape,
        min_level,
        max_level,
        max_cells,
        arena: Vec::new(),
        pq: BinaryHeap::new(),
        result: Vec::new(),
    };
    c.run();
    denormalize(&normalize(c.result), min_level)
}
