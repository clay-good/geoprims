//! Geodesic buffers (add-navigation-and-geometry, geometry/computational,
//! "Geodesic buffers"): points, lines, and polygons grown or shrunk by a
//! distance on the ellipsoid, with round, mitre, or bevel joins and round,
//! flat, or square caps.
//!
//! The shape is built on an azimuthal equidistant plane centered on the input:
//! every segment contributes a rectangle, every join a disk, wedge, or mitre,
//! and every cap a disk or square, all convex. The buffer's outline is the
//! set of shape edges that no other shape covers, split where they cross and
//! stitched into rings. Vertices on round arcs and straight offsets are then
//! moved onto the true geodesic offset, and the distance from the input is
//! measured again at every vertex and edge midpoint.

use std::collections::HashMap;

use geographiclib_rs::{DirectGeodesic, Geodesic, InverseGeodesic};
use gp_base::ErrorCode;
use gp_base::error::ToolError;
use libm::{acos, atan2, ceil, cos, hypot, sin, sqrt};

type P = (f64, f64);
/// A ring of (lat, lon) degrees.
pub type Ring = Vec<(f64, f64)>;
/// A part: its outline and its holes.
pub type Part = (Ring, Vec<Ring>);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Join {
    Round,
    Mitre,
    Bevel,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cap {
    Round,
    Flat,
    Square,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shape {
    Point,
    Line,
    Polygon,
}

#[derive(Clone, Copy, Debug)]
pub struct Style {
    pub join: Join,
    pub cap: Cap,
    pub mitre_limit: f64,
}

fn sub(a: P, b: P) -> P {
    (a.0 - b.0, a.1 - b.1)
}
fn add(a: P, b: P) -> P {
    (a.0 + b.0, a.1 + b.1)
}
fn mul(a: P, k: f64) -> P {
    (a.0 * k, a.1 * k)
}
fn cross(a: P, b: P) -> f64 {
    a.0 * b.1 - a.1 * b.0
}
fn dot(a: P, b: P) -> f64 {
    a.0 * b.0 + a.1 * b.1
}
fn norm(a: P) -> f64 {
    hypot(a.0, a.1)
}

/// Twice the signed area: positive counterclockwise.
fn area2(v: &[P]) -> f64 {
    (0..v.len())
        .map(|i| cross(v[i], v[(i + 1) % v.len()]))
        .sum()
}

/// The polygon turned counterclockwise, or None when it has no area.
fn ccw(mut v: Vec<P>, eps: f64) -> Option<Vec<P>> {
    let a = area2(&v);
    if a.abs() <= eps {
        return None;
    }
    if a < 0.0 {
        v.reverse();
    }
    Some(v)
}

/// A regular polygon inscribed in the circle, counterclockwise.
fn disk(c: P, r: f64, n: usize) -> Vec<P> {
    (0..n)
        .map(|k| {
            let t = 2.0 * core::f64::consts::PI * k as f64 / n as f64;
            (c.0 + r * cos(t), c.1 + r * sin(t))
        })
        .collect()
}

/// Sides for a disk whose edges sag at most `tol` inside the circle.
pub fn disk_sides(r: f64, tol: f64) -> usize {
    if tol >= r {
        return 8;
    }
    let half = acos(1.0 - tol / r);
    (ceil(core::f64::consts::PI / half) as usize).clamp(8, 1440)
}

/// One segment's offsets: the rectangle's corners on its right and left.
struct Seg {
    a: P,
    b: P,
    u: P,
    n: P,
    ar: P,
    br: P,
    bl: P,
    al: P,
}

fn seg(a: P, b: P, r: f64) -> Seg {
    let l = norm(sub(b, a));
    let u = mul(sub(b, a), 1.0 / l);
    let n = (-u.1, u.0);
    Seg {
        a,
        b,
        u,
        n,
        ar: sub(a, mul(n, r)),
        br: sub(b, mul(n, r)),
        bl: add(b, mul(n, r)),
        al: add(a, mul(n, r)),
    }
}

/// The join between a segment ending at a vertex and the next starting there,
/// on the outside of the turn.
fn join(out: &mut Vec<Vec<P>>, s1: &Seg, s2: &Seg, r: f64, st: &Style, sides: usize, eps: f64) {
    let t = cross(s1.u, s2.u);
    if t.abs() < 1e-12 && dot(s1.u, s2.u) > 0.0 {
        return;
    }
    if st.join == Join::Round {
        out.push(disk(s1.b, r, sides));
        return;
    }
    // A left turn bulges on the right, a right turn on the left.
    let (c1, c2, n1, n2) = if t > 0.0 {
        (s1.br, s2.ar, mul(s1.n, -1.0), mul(s2.n, -1.0))
    } else {
        (s1.bl, s2.al, s1.n, s2.n)
    };
    let v = s1.b;
    let nd = dot(n1, n2);
    let ratio = if 1.0 + nd > 1e-12 {
        sqrt(2.0 / (1.0 + nd))
    } else {
        f64::INFINITY
    };
    let poly = if st.join == Join::Mitre && ratio <= st.mitre_limit {
        vec![v, c1, add(v, mul(add(n1, n2), r / (1.0 + nd))), c2]
    } else {
        vec![v, c1, c2]
    };
    out.extend(ccw(poly, eps));
}

/// The convex pieces whose union is the buffer (or, for a negative polygon
/// buffer, what is taken away).
fn pieces(
    kind: Shape,
    rings: &[Vec<P>],
    r: f64,
    st: &Style,
    sides: usize,
    eps: f64,
) -> Vec<Vec<P>> {
    let mut out = Vec::new();
    match kind {
        Shape::Point => {
            let p = rings[0][0];
            out.push(if st.cap == Cap::Square {
                vec![
                    (p.0 - r, p.1 - r),
                    (p.0 + r, p.1 - r),
                    (p.0 + r, p.1 + r),
                    (p.0 - r, p.1 + r),
                ]
            } else {
                disk(p, r, sides)
            });
        }
        Shape::Line => {
            let pts = &rings[0];
            let segs: Vec<Seg> = pts.windows(2).map(|w| seg(w[0], w[1], r)).collect();
            for s in &segs {
                out.extend(ccw(vec![s.ar, s.br, s.bl, s.al], eps));
            }
            for w in segs.windows(2) {
                join(&mut out, &w[0], &w[1], r, st, sides, eps);
            }
            let (first, last) = (&segs[0], &segs[segs.len() - 1]);
            match st.cap {
                Cap::Round => {
                    out.push(disk(first.a, r, sides));
                    out.push(disk(last.b, r, sides));
                }
                Cap::Square => {
                    let back = mul(first.u, -r);
                    out.extend(ccw(
                        vec![add(first.ar, back), first.ar, first.al, add(first.al, back)],
                        eps,
                    ));
                    let fwd = mul(last.u, r);
                    out.extend(ccw(
                        vec![last.br, add(last.br, fwd), add(last.bl, fwd), last.bl],
                        eps,
                    ));
                }
                Cap::Flat => {}
            }
        }
        Shape::Polygon => {
            for ring in rings {
                let n = ring.len();
                let segs: Vec<Seg> = (0..n).map(|i| seg(ring[i], ring[(i + 1) % n], r)).collect();
                for s in &segs {
                    out.extend(ccw(vec![s.ar, s.br, s.bl, s.al], eps));
                }
                for i in 0..n {
                    join(&mut out, &segs[i], &segs[(i + 1) % n], r, st, sides, eps);
                }
            }
        }
    }
    out
}

fn inside_convex(sh: &[P], p: P) -> bool {
    (0..sh.len()).all(|k| cross(sub(sh[(k + 1) % sh.len()], sh[k]), sub(p, sh[k])) > 0.0)
}

/// Even-odd test against every ring, so holes count as outside.
pub fn inside_rings(rings: &[Vec<P>], p: P) -> bool {
    let mut inside = false;
    for ring in rings {
        let n = ring.len();
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            if (a.1 > p.1) != (b.1 > p.1) && p.0 < a.0 + (p.1 - a.1) * (b.0 - a.0) / (b.1 - a.1) {
                inside = !inside;
            }
        }
    }
    inside
}

/// A uniform grid of boxes, for finding what lies near a point or an edge.
struct Grid {
    x0: f64,
    y0: f64,
    cell: f64,
    cells: HashMap<(i64, i64), Vec<usize>>,
}

impl Grid {
    fn new(boxes: &[(P, P)], cell: f64) -> Grid {
        let x0 = boxes.iter().map(|b| b.0.0).fold(f64::INFINITY, f64::min);
        let y0 = boxes.iter().map(|b| b.0.1).fold(f64::INFINITY, f64::min);
        let mut g = Grid {
            x0,
            y0,
            cell,
            cells: HashMap::new(),
        };
        for (i, b) in boxes.iter().enumerate() {
            let (i0, j0) = g.key(b.0);
            let (i1, j1) = g.key(b.1);
            for x in i0..=i1 {
                for y in j0..=j1 {
                    g.cells.entry((x, y)).or_default().push(i);
                }
            }
        }
        g
    }
    fn key(&self, p: P) -> (i64, i64) {
        (
            ((p.0 - self.x0) / self.cell).floor() as i64,
            ((p.1 - self.y0) / self.cell).floor() as i64,
        )
    }
    fn at(&self, p: P) -> &[usize] {
        self.cells.get(&self.key(p)).map_or(&[], |v| v.as_slice())
    }
}

fn bbox(pts: &[P]) -> (P, P) {
    let mut lo = (f64::INFINITY, f64::INFINITY);
    let mut hi = (f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in pts {
        lo = (lo.0.min(p.0), lo.1.min(p.1));
        hi = (hi.0.max(p.0), hi.1.max(p.1));
    }
    (lo, hi)
}

/// Where edge p→p2 meets q→q2: parameters on each and one shared point, so
/// both edges split at bit-identical coordinates.
fn meet(p: P, p2: P, q: P, q2: P, eps: f64, out: &mut Vec<(f64, f64, P)>) {
    let (r, s) = (sub(p2, p), sub(q2, q));
    let (lr, ls) = (norm(r), norm(s));
    if lr < eps || ls < eps {
        return;
    }
    let den = cross(r, s);
    let qp = sub(q, p);
    if den.abs() > 1e-12 * lr * ls {
        let t = cross(qp, s) / den;
        let u = cross(qp, r) / den;
        let (tt, tu) = (eps / lr, eps / ls);
        if t < -tt || t > 1.0 + tt || u < -tu || u > 1.0 + tu {
            return;
        }
        let pt = if t.abs() <= tt {
            p
        } else if (t - 1.0).abs() <= tt {
            p2
        } else if u.abs() <= tu {
            q
        } else if (u - 1.0).abs() <= tu {
            q2
        } else {
            add(p, mul(r, t))
        };
        out.push((t.clamp(0.0, 1.0), u.clamp(0.0, 1.0), pt));
    } else if cross(qp, r).abs() <= eps * lr {
        // Collinear: each edge splits at the other's endpoints inside it.
        for e in [q, q2] {
            let t = dot(sub(e, p), r) / (lr * lr);
            if t > 0.0 && t < 1.0 {
                out.push((t, f64::NAN, e));
            }
        }
        for e in [p, p2] {
            let u = dot(sub(e, q), s) / (ls * ls);
            if u > 0.0 && u < 1.0 {
                out.push((f64::NAN, u, e));
            }
        }
    }
}

/// The planar buffer's rings: outlines counterclockwise, holes clockwise.
pub fn plane(kind: Shape, rings: &[Vec<P>], d: f64, st: &Style, sides: usize) -> Vec<Vec<P>> {
    let r = d.abs();
    let scale = rings
        .iter()
        .flatten()
        .map(|p| p.0.abs().max(p.1.abs()))
        .fold(r.max(1.0), f64::max);
    let eps = 1e-9 * scale;
    let shapes = pieces(kind, rings, r, st, sides, eps * eps);
    if kind != Shape::Polygon && d < 0.0 {
        return Vec::new();
    }
    let boxes: Vec<(P, P)> = shapes.iter().map(|s| bbox(s)).collect();
    let longest = boxes
        .iter()
        .map(|b| (b.1.0 - b.0.0).max(b.1.1 - b.0.1))
        .fold(0.0, f64::max);
    let cell = r.max(longest / 8.0).max(scale * 1e-6);
    let shape_grid = Grid::new(&boxes, cell);
    // Every edge of every shape, with its owner.
    let mut edges: Vec<(P, P, usize)> = Vec::new();
    for (si, sh) in shapes.iter().enumerate() {
        for k in 0..sh.len() {
            edges.push((sh[k], sh[(k + 1) % sh.len()], si));
        }
    }
    let eboxes: Vec<(P, P)> = edges.iter().map(|e| bbox(&[e.0, e.1])).collect();
    let edge_grid = Grid::new(&eboxes, cell);
    let mut splits: Vec<Vec<(f64, P)>> = vec![Vec::new(); edges.len()];
    let mut seen = std::collections::HashSet::new();
    let mut hits = Vec::new();
    for bucket in edge_grid.cells.values() {
        for (x, &i) in bucket.iter().enumerate() {
            for &j in &bucket[x + 1..] {
                let (a, b) = (i.min(j), i.max(j));
                if edges[a].2 == edges[b].2 || !seen.insert((a, b)) {
                    continue;
                }
                let (ba, bb) = (eboxes[a], eboxes[b]);
                if ba.1.0 + eps < bb.0.0
                    || bb.1.0 + eps < ba.0.0
                    || ba.1.1 + eps < bb.0.1
                    || bb.1.1 + eps < ba.0.1
                {
                    continue;
                }
                hits.clear();
                meet(
                    edges[a].0, edges[a].1, edges[b].0, edges[b].1, eps, &mut hits,
                );
                for &(t, u, pt) in &hits {
                    if !t.is_nan() {
                        splits[a].push((t, pt));
                    }
                    if !u.is_nan() {
                        splits[b].push((u, pt));
                    }
                }
            }
        }
    }
    let nudge = 1e-6 * r.max(1e-3);
    let polygon = kind == Shape::Polygon;
    let mut kept: Vec<(P, P)> = Vec::new();
    for (i, &(a, b, _)) in edges.iter().enumerate() {
        let mut pts = splits[i].clone();
        pts.push((0.0, a));
        pts.push((1.0, b));
        pts.sort_by(|x, y| x.0.total_cmp(&y.0));
        pts.dedup_by(|x, y| x.1 == y.1);
        for w in pts.windows(2) {
            let (p, q) = (w[0].1, w[1].1);
            let dir = sub(q, p);
            let l = norm(dir);
            if l <= eps {
                continue;
            }
            // Just outside this edge's own shape (its right side).
            let o = add(mul(add(p, q), 0.5), mul((dir.1 / l, -dir.0 / l), nudge));
            let covered = shape_grid
                .at(o)
                .iter()
                .any(|&s| inside_convex(&shapes[s], o));
            if covered {
                continue;
            }
            let in_poly = polygon && inside_rings(rings, o);
            if d > 0.0 && !in_poly {
                kept.push((p, q));
            } else if d < 0.0 && in_poly {
                kept.push((q, p));
            }
        }
    }
    stitch(kept, eps, r)
}

fn bits(p: P) -> (u64, u64) {
    (p.0.to_bits(), p.1.to_bits())
}

/// Joins directed edges end to start into closed rings.
fn stitch(edges: Vec<(P, P)>, eps: f64, r: f64) -> Vec<Vec<P>> {
    let mut from: HashMap<(u64, u64), Vec<usize>> = HashMap::new();
    for (i, e) in edges.iter().enumerate() {
        from.entry(bits(e.0)).or_default().push(i);
    }
    let mut used = vec![false; edges.len()];
    let mut rings = Vec::new();
    for start in 0..edges.len() {
        if used[start] {
            continue;
        }
        used[start] = true;
        let mut ring = vec![edges[start].0];
        let mut cur = edges[start].1;
        let mut closed = false;
        for _ in 0..edges.len() {
            if norm(sub(cur, ring[0])) <= eps {
                closed = true;
                break;
            }
            ring.push(cur);
            let next = from
                .get(&bits(cur))
                .and_then(|v| v.iter().copied().find(|&k| !used[k]))
                .or_else(|| {
                    (0..edges.len()).find(|&k| !used[k] && norm(sub(edges[k].0, cur)) <= eps * 1e3)
                });
            let Some(k) = next else { break };
            used[k] = true;
            cur = edges[k].1;
        }
        if !closed {
            continue;
        }
        let ring = simplify(ring);
        if ring.len() >= 3 && area2(&ring).abs() > 1e-9 * r * r {
            rings.push(ring);
        }
    }
    rings
}

/// Drops repeated points and points on a straight run.
fn simplify(ring: Vec<P>) -> Vec<P> {
    let mut v = ring;
    loop {
        let n = v.len();
        if n < 3 {
            return v;
        }
        let keep: Vec<bool> = (0..n)
            .map(|i| {
                let (a, b, c) = (v[(i + n - 1) % n], v[i], v[(i + 1) % n]);
                let (u, w) = (sub(b, a), sub(c, b));
                let (lu, lw) = (norm(u), norm(w));
                lu > 0.0 && lw > 0.0 && !(cross(u, w).abs() <= 1e-12 * lu * lw && dot(u, w) > 0.0)
            })
            .collect();
        if keep.iter().all(|k| *k) {
            return v;
        }
        v = v
            .iter()
            .zip(&keep)
            .filter(|(_, k)| **k)
            .map(|(p, _)| *p)
            .collect();
    }
}

/// An azimuthal equidistant plane on the ellipsoid, by the geodesic problems.
pub struct Aeqd<'g> {
    pub g: &'g Geodesic,
    pub lat0: f64,
    pub lon0: f64,
}

impl Aeqd<'_> {
    pub fn fwd(&self, ll: (f64, f64)) -> P {
        let (s, az, _, _): (f64, f64, f64, f64) = self.g.inverse(self.lat0, self.lon0, ll.0, ll.1);
        let t = az.to_radians();
        (s * sin(t), s * cos(t))
    }
    pub fn rev(&self, p: P) -> (f64, f64) {
        let s = norm(p);
        if s == 0.0 {
            return (self.lat0, self.lon0);
        }
        let az = atan2(p.0, p.1).to_degrees();
        let (lat, lon, _): (f64, f64, f64) = self.g.direct(self.lat0, self.lon0, az, s);
        (lat, (lon + 540.0).rem_euclid(360.0) - 180.0)
    }
}

/// A center for the plane: the normalized mean of the points' unit vectors.
pub fn center(pts: &[(f64, f64)]) -> (f64, f64) {
    let (mut x, mut y, mut z) = (0.0, 0.0, 0.0);
    for &(la, lo) in pts {
        let (a, o) = (la.to_radians(), lo.to_radians());
        x += cos(a) * cos(o);
        y += cos(a) * sin(o);
        z += sin(a);
    }
    if hypot(hypot(x, y), z) < 1e-12 {
        return pts[0];
    }
    (atan2(z, hypot(x, y)).to_degrees(), atan2(y, x).to_degrees())
}

/// The shortest geodesic distance from `p` to the geodesic a→b, and the
/// nearest point on it, by Newton steps on the along-track offset.
pub fn seg_dist(g: &Geodesic, a: (f64, f64), b: (f64, f64), p: (f64, f64)) -> (f64, (f64, f64)) {
    let (l, az_a, _, _): (f64, f64, f64, f64) = g.inverse(a.0, a.1, b.0, b.1);
    if l < 1e-9 {
        let d: f64 = g.inverse(a.0, a.1, p.0, p.1);
        return (d, a);
    }
    let (dap, az_ap, _, _): (f64, f64, f64, f64) = g.inverse(a.0, a.1, p.0, p.1);
    let mut s = (dap * cos((az_ap - az_a).to_radians())).clamp(0.0, l);
    for _ in 0..30 {
        let (xl, xo, az_x): (f64, f64, f64) = g.direct(a.0, a.1, az_a, s);
        let (dxp, az_xp, _, _): (f64, f64, f64, f64) = g.inverse(xl, xo, p.0, p.1);
        let next = (s + dxp * cos((az_xp - az_x).to_radians())).clamp(0.0, l);
        let done = (next - s).abs() < 1e-7;
        s = next;
        if done {
            break;
        }
    }
    let (xl, xo, _): (f64, f64, f64) = g.direct(a.0, a.1, az_a, s);
    let d: f64 = g.inverse(xl, xo, p.0, p.1);
    (d, (xl, xo))
}

/// The input's edges as geodesic pieces (a point is one zero-length piece).
pub struct Edges {
    pub segs: Vec<((f64, f64), (f64, f64))>,
    planar: Vec<(P, P)>,
    grid: Grid,
}

impl Edges {
    fn new(kind: Shape, rings_ll: &[Vec<(f64, f64)>], plane: &Aeqd, reach: f64) -> Edges {
        let mut segs = Vec::new();
        for ring in rings_ll {
            let n = ring.len();
            let count = match kind {
                Shape::Point => 1,
                Shape::Line => n - 1,
                Shape::Polygon => n,
            };
            for i in 0..count {
                segs.push((ring[i], ring[(i + 1) % n]));
            }
        }
        let planar: Vec<(P, P)> = segs
            .iter()
            .map(|(a, b)| (plane.fwd(*a), plane.fwd(*b)))
            .collect();
        let boxes: Vec<(P, P)> = planar
            .iter()
            .map(|(a, b)| {
                let (lo, hi) = bbox(&[*a, *b]);
                ((lo.0 - reach, lo.1 - reach), (hi.0 + reach, hi.1 + reach))
            })
            .collect();
        let grid = Grid::new(&boxes, reach.max(1.0));
        Edges { segs, planar, grid }
    }

    /// Distance and nearest point from `ll` (at plane point `p`) to the input.
    pub fn nearest(&self, g: &Geodesic, ll: (f64, f64), p: P) -> (f64, (f64, f64)) {
        let near = self.grid.at(p);
        let candidates: Vec<usize> = if near.is_empty() {
            (0..self.segs.len()).collect()
        } else {
            // A bounded search: the nearest few by plane distance, measured exactly.
            let mut v: Vec<(f64, usize)> = near
                .iter()
                .map(|&i| (plane_seg(self.planar[i], p), i))
                .collect();
            v.sort_by(|a, b| a.0.total_cmp(&b.0));
            let best = v[0].0;
            v.into_iter()
                .take_while(|(d, _)| *d <= best * 1.05 + 1.0)
                .map(|(_, i)| i)
                .collect()
        };
        candidates
            .into_iter()
            .map(|i| seg_dist(g, self.segs[i].0, self.segs[i].1, ll))
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .expect("at least one edge")
    }
}

fn plane_seg((a, b): (P, P), p: P) -> f64 {
    let ab = sub(b, a);
    let l2 = dot(ab, ab);
    let t = if l2 > 0.0 {
        (dot(sub(p, a), ab) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    norm(sub(p, add(a, mul(ab, t))))
}

/// A finished buffer: parts as (outline, holes), in (lat, lon) degrees.
pub struct Buffered {
    pub parts: Vec<Part>,
    pub max_deviation: f64,
    pub tolerance: f64,
    pub checked: usize,
}

/// Longest geodesic piece an input edge is cut into before projecting.
const PIECE: f64 = 5_000.0;
/// How far from the plane's center the input and buffer may reach.
pub const REACH: f64 = 1_000_000.0;

/// Buffers geographic input by `d` meters (negative shrinks a polygon).
pub fn geodesic(
    g: &Geodesic,
    kind: Shape,
    rings_ll: &[Vec<(f64, f64)>],
    d: f64,
    st: &Style,
) -> Result<Buffered, ToolError> {
    let r = d.abs();
    let tolerance = (0.001 * r).max(0.5);
    // Cut long edges so each plane chord follows its geodesic closely.
    let mut dense: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut total = 0usize;
    for ring in rings_ll {
        let n = ring.len();
        let count = match kind {
            Shape::Point => 0,
            Shape::Line => n - 1,
            Shape::Polygon => n,
        };
        let mut out = vec![ring[0]];
        for i in 0..count {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            let (s, az, _, _): (f64, f64, f64, f64) = g.inverse(a.0, a.1, b.0, b.1);
            let k = (ceil(s / PIECE) as usize).max(1);
            total += k;
            if total > 20_000 {
                return Err(ToolError::new(ErrorCode::LimitExceeded, "The input is too long to buffer here: over 20,000 edge pieces of 5 km. Split it into smaller parts.").at("/vertices"));
            }
            for j in 1..=k {
                let (la, lo, _): (f64, f64, f64) = g.direct(a.0, a.1, az, s * j as f64 / k as f64);
                out.push(if j == k {
                    b
                } else {
                    (la, (lo + 540.0).rem_euclid(360.0) - 180.0)
                });
            }
        }
        if kind == Shape::Polygon {
            out.pop();
        }
        dense.push(out);
    }
    let all: Vec<(f64, f64)> = dense.iter().flatten().copied().collect();
    let (lat0, lon0) = center(&all);
    let plane_map = Aeqd { g, lat0, lon0 };
    let planar: Vec<Vec<P>> = dense
        .iter()
        .map(|ring| ring.iter().map(|&p| plane_map.fwd(p)).collect())
        .collect();
    let far = planar
        .iter()
        .flatten()
        .map(|p| norm(*p))
        .fold(0.0, f64::max);
    if far + r > REACH {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The input and buffer must fit within 1,000 km of their center; buffer large areas in parts.",
        )
        .at("/distance"));
    }
    let sides = disk_sides(r, 0.25 * tolerance);
    let rings = plane(kind, &planar, d, st, sides);
    let edges = Edges::new(kind, &dense, &plane_map, 1.2 * r + 100.0);
    // Round joins and caps leave no corners, so every point is checked.
    let round = st.join == Join::Round && (kind == Shape::Polygon || st.cap == Cap::Round);
    let on_offset = |dist: f64| round || (dist - r).abs() <= 0.01 * r + 1.0;
    let mut geo_rings = Vec::new();
    for ring in &rings {
        let mut out = Vec::with_capacity(ring.len());
        for &p in ring {
            let mut ll = plane_map.rev(p);
            // Onto the true offset: r from the nearest input point, twice over.
            for _ in 0..2 {
                let (dist, near) = edges.nearest(g, ll, plane_map.fwd(ll));
                if !on_offset(dist) || dist < 1e-9 {
                    break;
                }
                let (_, az, _, _): (f64, f64, f64, f64) = g.inverse(near.0, near.1, ll.0, ll.1);
                let (la, lo, _): (f64, f64, f64) = g.direct(near.0, near.1, az, r);
                ll = (la, (lo + 540.0).rem_euclid(360.0) - 180.0);
            }
            out.push(ll);
        }
        geo_rings.push(out);
    }
    // Measured validation at every vertex and every edge's geodesic midpoint.
    let mut max_deviation: f64 = 0.0;
    let mut checked = 0;
    for ring in &geo_rings {
        let n = ring.len();
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            let (s, az, _, _): (f64, f64, f64, f64) = g.inverse(a.0, a.1, b.0, b.1);
            let (ml, mo, _): (f64, f64, f64) = g.direct(a.0, a.1, az, s / 2.0);
            for q in [a, (ml, mo)] {
                let (dist, _) = edges.nearest(g, q, plane_map.fwd(q));
                if on_offset(dist) {
                    max_deviation = max_deviation.max((dist - r).abs());
                    checked += 1;
                }
            }
        }
    }
    // Holes go with the outline that holds them.
    let (mut outers, mut holes): (Vec<usize>, Vec<usize>) = (Vec::new(), Vec::new());
    for (i, ring) in rings.iter().enumerate() {
        if area2(ring) > 0.0 {
            outers.push(i)
        } else {
            holes.push(i)
        }
    }
    let mut parts: Vec<Part> = outers
        .iter()
        .map(|&i| (geo_rings[i].clone(), Vec::new()))
        .collect();
    for h in holes {
        if let Some(k) = outers
            .iter()
            .position(|&o| inside_rings(std::slice::from_ref(&rings[o]), rings[h][0]))
        {
            parts[k].1.push(geo_rings[h].clone());
        }
    }
    Ok(Buffered {
        parts,
        max_deviation,
        tolerance,
        checked,
    })
}

// ---- The tool ----

use gp_base::error::Warning;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};

const KARNEY: Reference = Reference {
    title: "Algorithms for geodesics",
    issuer: "Karney, C. F. F., Journal of Geodesy",
    year: 2013,
    edition: "Vol. 87, No. 1",
    locator: "pp. 43-55 (direct and inverse problems; azimuthal equidistant projection, section 8)",
    url: "https://doi.org/10.1007/s00190-012-0578-z",
};

const VERTEX: &[Field] = &[
    Field::new(
        "lat",
        "Latitude",
        "Decimal degrees, like 40.4406",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .required(),
    Field::new(
        "lon",
        "Longitude",
        "Decimal degrees, like -80.002",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .required(),
    Field::new(
        "ring",
        "Ring",
        "For a polygon: 0 for the outline, 1, 2, … for holes",
        Kind::Number {
            min: 0.0,
            max: 100.0,
        },
    ),
];
const OUT_VERTEX: &[Field] = &[
    Field::new(
        "lat",
        "Latitude",
        "Degrees",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7)),
    Field::new(
        "lon",
        "Longitude",
        "Degrees",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7)),
    Field::new(
        "part",
        "Part",
        "0, 1, … when the buffer is in pieces",
        Kind::Number { min: 0.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "ring",
        "Ring",
        "0 for a part's outline, 1, 2, … for its holes",
        Kind::Number { min: 0.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
];

pub static BUFFER: ToolDef = ToolDef {
    id: "geometry.buffer.geodesic",
    title: "Buffer a point, line, or polygon",
    summary: "The area within a distance of a point, line, or polygon on the ellipsoid, or a polygon shrunk inward, with round, mitre, or bevel corners, measured back against the input.",
    aliases: &["buffer", "geodesic buffer", "offset polygon", "setback", "zone around a line", "radius around a point"],
    keywords: &["buffer", "offset", "setback", "radius", "zone", "corridor", "geofence", "inset", "shrink", "grow"],
    inputs: &[
        Field::new("vertices", "Vertices", "A point, a line in order, or polygon corners (holes by ring), one per line, like 40.4406, -80.002", Kind::List { items: VERTEX, min: 1, max: 5_000 }).required().core(),
        Field::new("distance", "Distance", "How far out, like 500 m; negative shrinks a polygon, like -20 m", Kind::Quantity { q: QT::Length, unit: "m" }).required().core(),
        Field::new("shape", "Treat as", "point, line, or polygon; default by count: 1 point, 2 a line, 3 or more a polygon", Kind::Choice(&["polygon", "line", "point"])).core(),
        Field::new("join", "Corners", "round (default), mitre (sharp), or bevel (cut off)", Kind::Choice(&["round", "mitre", "bevel"])),
        Field::new("cap", "Line ends", "round (default), flat, or square", Kind::Choice(&["round", "flat", "square"])),
        Field::new("mitre_limit", "Mitre limit", "Longest sharp corner, in distances, like 5 (the default); longer ones are beveled", Kind::Number { min: 1.0, max: 100.0 }),
    ],
    outputs: &[
        Field::new("area", "Area", "Parts minus holes", Kind::Quantity { q: QT::Area, unit: "km2" }).precision(Precision::Significant(8)),
        Field::new("perimeter", "Perimeter", "Every outline and hole", Kind::Quantity { q: QT::Distance, unit: "km" }).precision(Precision::Decimals(3)),
        Field::new("parts", "Parts", "Separate pieces; 0 when the buffer collapsed", Kind::Number { min: 0.0, max: 1e6 }).precision(Precision::Decimals(0)),
        Field::new("vertex_count", "Vertices", "In the boundary", Kind::Number { min: 0.0, max: 1e9 }).precision(Precision::Decimals(0)),
        Field::new("max_deviation", "Largest measured error", "Geodesic distance from the input, minus the buffer distance, at every vertex and edge midpoint (sharp corners and flat ends skipped)", Kind::Quantity { q: QT::Length, unit: "m" }).precision(Precision::Decimals(3)),
        Field::new("tolerance", "Tolerance", "0.1% of the distance or 0.5 m, whichever is larger", Kind::Quantity { q: QT::Length, unit: "m" }).precision(Precision::Decimals(3)),
        Field::new("boundary", "Boundary", "Each part's outline counterclockwise, then its holes", Kind::List { items: OUT_VERTEX, min: 0, max: 1_000_000 }),
    ],
    errors: &[ErrorCode::OutOfDomain, ErrorCode::LimitExceeded],
    warnings: &["BUFFER_COLLAPSED", "BUFFER_ACCURACY", "EXPERIMENTAL_TOOL"],
    model: "On WGS 84: convex pieces (a rectangle per edge, a disk, wedge, or mitre per corner, a disk or square per line end) on an azimuthal equidistant plane at the input's center, unioned by keeping the edges no other piece covers; vertices on round and straight parts are then placed exactly at the distance from the nearest input point by the geodesic direct problem, and the distance is measured again at every vertex and edge midpoint (Karney 2013)",
    accuracy: "Round parts within 0.1% of the distance or 0.5 m, whichever is larger, as measured on every result; mitre and square corners sit at d / cos(θ/2) by construction. Input and buffer must fit within 1,000 km of their center",
    references: &[KARNEY],
    examples: &[Example {
        id: "primary",
        title: "A 500 m geofence around a field",
        input: r#"{"vertices":[{"lat":40.0,"lon":-105.0},{"lat":40.0,"lon":-104.99},{"lat":40.006,"lon":-104.99},{"lat":40.006,"lon":-105.0}],"distance":"500 m"}"#,
        source: "add-navigation-and-geometry geofence buffer scenario (500 m ± 0.5 m at every vertex and midpoint)",
    }],
    primary_example: "primary",
    visualization: &[Layer { kind: "polygon", map: &[("rings", "boundary")] }],
    related: &[
        Related { id: "geometry.area.polygon", reason: "next" },
        Related { id: "navigation.geodesic.inverse", reason: "alternative" },
    ],
    sentence: "{if parts > 0}The buffer covers {area} in {parts} {plural parts \"part\" \"parts\"}, every checked point within {max_deviation} of the distance.{/if}{if parts < 1}Nothing is left: the buffer collapsed.{/if}",
    limits: &[("batchRows", 20)],
    run: run_buffer,
    ..ToolDef::BLANK
};

fn run_buffer(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let m = units::by_symbol(QT::Length, "m").expect("m");
    let rows = ctx.rows("vertices")?;
    let mut rings: Vec<Vec<(f64, f64)>> = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let lat = ctx
            .row_quantity("vertices", i, row, "lat")?
            .expect("required")
            .to(deg);
        let lon = ctx
            .row_quantity("vertices", i, row, "lon")?
            .expect("required")
            .to(deg);
        if !(-90.0..=90.0).contains(&lat) {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "Latitude must be between -90° and 90°.",
            )
            .at(&format!("/vertices/{i}/lat")));
        }
        let ring = row
            .get("ring")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);
        if ring.fract() != 0.0 || !(0.0..=100.0).contains(&ring) {
            return Err(ToolError::invalid(
                &format!("/vertices/{i}/ring"),
                "Ring must be a whole number from 0 to 100.",
            ));
        }
        let k = ring as usize;
        if rings.len() <= k {
            rings.resize(k + 1, Vec::new());
        }
        // Repeated vertices add nothing and would make zero-length edges.
        if rings[k].last() != Some(&(lat, lon)) {
            rings[k].push((lat, lon));
        }
    }
    rings.retain(|r| !r.is_empty());
    let count: usize = rings.iter().map(Vec::len).sum();
    let kind = match ctx.choice("shape")? {
        Some("point") => Shape::Point,
        Some("line") => Shape::Line,
        Some(_) => Shape::Polygon,
        None if count == 1 => Shape::Point,
        None if count == 2 => Shape::Line,
        None => Shape::Polygon,
    };
    match kind {
        Shape::Point if count != 1 => {
            return Err(ToolError::invalid("/vertices", "A point is one vertex."));
        }
        Shape::Line if rings.len() != 1 || count < 2 => {
            return Err(ToolError::invalid(
                "/vertices",
                "A line is two or more vertices, all in ring 0.",
            ));
        }
        Shape::Polygon => {
            for (k, ring) in rings.iter_mut().enumerate() {
                if ring.len() > 1 && ring.first() == ring.last() {
                    ring.pop();
                }
                if ring.len() < 3 {
                    return Err(ToolError::invalid(
                        "/vertices",
                        format!("Ring {k} needs at least 3 distinct corners."),
                    ));
                }
            }
        }
        _ => {}
    }
    let d = ctx.req_quantity("distance")?.to(m);
    if d == 0.0 || !d.is_finite() {
        return Err(ToolError::invalid(
            "/distance",
            "The distance must be a nonzero length, like 500 m.",
        ));
    }
    let join = match ctx.choice("join")? {
        Some("mitre") => Join::Mitre,
        Some("bevel") => Join::Bevel,
        _ => Join::Round,
    };
    let cap = match ctx.choice("cap")? {
        Some("flat") => Cap::Flat,
        Some("square") => Cap::Square,
        _ => Cap::Round,
    };
    if kind == Shape::Point && cap == Cap::Flat {
        return Err(ToolError::invalid(
            "/cap",
            "A point has no direction for a flat end: use round or square.",
        ));
    }
    let st = Style {
        join,
        cap,
        mitre_limit: ctx.number("mitre_limit")?.unwrap_or(5.0),
    };
    let g = Geodesic::wgs84();
    let out = geodesic(&g, kind, &rings, d, &st)?;
    if out.parts.is_empty() {
        ctx.warnings.push(Warning::new(
            "BUFFER_COLLAPSED",
            if kind == Shape::Polygon {
                "The polygon is narrower than twice the inward distance, so nothing is left."
            } else {
                "Only a polygon can shrink: a point or line buffered inward is empty."
            },
        ));
    } else if out.max_deviation > out.tolerance {
        ctx.warnings.push(Warning::new(
            "BUFFER_ACCURACY",
            format!(
                "The boundary strays {:.3} m from the distance, over the {:.3} m tolerance.",
                out.max_deviation, out.tolerance
            ),
        ));
    }
    let (mut area, mut perimeter) = (0.0, 0.0);
    let mut boundary = Vec::new();
    let dq = |v: f64| {
        Q {
            value: v,
            unit: deg,
        }
        .to_json()
    };
    for (pi, (outline, holes)) in out.parts.iter().enumerate() {
        for (ri, ring) in core::iter::once(outline).chain(holes.iter()).enumerate() {
            let (a, p) = super::ring_area(&g, ring);
            area += if ri == 0 { a.abs() } else { -a.abs() };
            perimeter += p;
            for &(lat, lon) in ring {
                boundary.push(Json::obj([
                    ("lat", dq(lat)),
                    ("lon", dq(lon)),
                    ("part", Json::Num(pi as f64)),
                    ("ring", Json::Num(ri as f64)),
                ]));
            }
        }
    }
    let n = boundary.len();
    Ok(Json::obj([
        ("area", ctx.out("area", super::q(area, "m2", QT::Area))),
        (
            "perimeter",
            ctx.out("perimeter", super::q(perimeter, "m", QT::Distance)),
        ),
        ("parts", Json::Num(out.parts.len() as f64)),
        ("vertex_count", Json::Num(n as f64)),
        (
            "max_deviation",
            ctx.out(
                "max_deviation",
                Q {
                    value: out.max_deviation,
                    unit: m,
                },
            ),
        ),
        (
            "tolerance",
            ctx.out(
                "tolerance",
                Q {
                    value: out.tolerance,
                    unit: m,
                },
            ),
        ),
        ("boundary", Json::Arr(boundary)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROUND: Style = Style {
        join: Join::Round,
        cap: Cap::Round,
        mitre_limit: 5.0,
    };

    fn area(rings: &[Vec<P>]) -> f64 {
        rings.iter().map(|r| area2(r) / 2.0).sum()
    }

    #[test]
    fn a_square_grows_by_the_minkowski_area() {
        let sq = vec![vec![(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]];
        let sides = disk_sides(10.0, 0.001);
        let out = plane(Shape::Polygon, &sq, 10.0, &ROUND, sides);
        assert_eq!(out.len(), 1);
        // 100² + 4·100·10 + π·10² (the disk as inscribed, a hair under π).
        let expect = 10_000.0 + 4_000.0 + core::f64::consts::PI * 100.0;
        assert!((area(&out) - expect).abs() < 0.5, "{}", area(&out));
    }

    #[test]
    fn mitre_and_bevel_corners() {
        let sq = vec![vec![(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]];
        let mitre = Style {
            join: Join::Mitre,
            ..ROUND
        };
        let bevel = Style {
            join: Join::Bevel,
            ..ROUND
        };
        assert!((area(&plane(Shape::Polygon, &sq, 10.0, &mitre, 16)) - 120.0 * 120.0).abs() < 1e-6);
        assert!(
            (area(&plane(Shape::Polygon, &sq, 10.0, &bevel, 16)) - (120.0 * 120.0 - 4.0 * 50.0))
                .abs()
                < 1e-6
        );
    }

    #[test]
    fn a_negative_buffer_shrinks_then_collapses() {
        let sq = vec![vec![(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]];
        let out = plane(Shape::Polygon, &sq, -10.0, &ROUND, 64);
        assert_eq!(out.len(), 1);
        assert!((area(&out) - 80.0 * 80.0).abs() < 1e-6, "{}", area(&out));
        assert!(plane(Shape::Polygon, &sq, -60.0, &ROUND, 64).is_empty());
    }

    #[test]
    fn a_line_with_flat_caps_is_a_rectangle() {
        let line = vec![vec![(0.0, 0.0), (100.0, 0.0)]];
        let flat = Style {
            cap: Cap::Flat,
            ..ROUND
        };
        let sq = Style {
            cap: Cap::Square,
            ..ROUND
        };
        assert!((area(&plane(Shape::Line, &line, 5.0, &flat, 32)) - 1_000.0).abs() < 1e-6);
        assert!((area(&plane(Shape::Line, &line, 5.0, &sq, 32)) - 1_100.0).abs() < 1e-6);
    }

    #[test]
    fn a_ring_shaped_line_leaves_a_hole() {
        let line = vec![vec![
            (0.0, 0.0),
            (100.0, 0.0),
            (100.0, 100.0),
            (0.0, 100.0),
            (0.0, 0.0),
        ]];
        let out = plane(Shape::Line, &line, 10.0, &ROUND, 64);
        assert_eq!(out.len(), 2);
        assert_eq!(out.iter().filter(|r| area2(r) < 0.0).count(), 1);
    }

    #[test]
    fn a_concave_polygon_keeps_its_notch() {
        // A U shape: the notch is 20 wide, so a 5 buffer leaves a 10-wide gap.
        let u = vec![vec![
            (0.0, 0.0),
            (60.0, 0.0),
            (60.0, 60.0),
            (40.0, 60.0),
            (40.0, 20.0),
            (20.0, 20.0),
            (20.0, 60.0),
            (0.0, 60.0),
        ]];
        let out = plane(Shape::Polygon, &u, 5.0, &ROUND, 64);
        assert_eq!(out.len(), 1);
        assert!(!inside_rings(&out, (30.0, 40.0)));
        assert!(inside_rings(&out, (30.0, 17.0)));
        // At 12 the notch closes into the shape.
        let closed = plane(Shape::Polygon, &u, 12.0, &ROUND, 64);
        assert!(inside_rings(&closed, (30.0, 40.0)));
    }
}
