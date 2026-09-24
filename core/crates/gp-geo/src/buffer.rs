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
    let splits = split(&edges, cell, eps, true, &mut |_, _, _, _, _| {});
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
    stitch(kept, eps, r, false)
}

fn bits(p: P) -> (u64, u64) {
    (p.0.to_bits(), p.1.to_bits())
}

/// Splits every edge where another crosses or touches it, so both split at
/// bit-identical points. `skip_same_owner` leaves pairs from one owner alone
/// (a convex shape's own edges never cross); `hit` sees each meeting as
/// (edge a, t on a, edge b, u on b, point), with NaN for an endpoint-only side.
fn split(
    edges: &[(P, P, usize)],
    cell: f64,
    eps: f64,
    skip_same_owner: bool,
    hit: &mut dyn FnMut(usize, f64, usize, f64, P),
) -> Vec<Vec<(f64, P)>> {
    let eboxes: Vec<(P, P)> = edges.iter().map(|e| bbox(&[e.0, e.1])).collect();
    let edge_grid = Grid::new(&eboxes, cell);
    let mut splits: Vec<Vec<(f64, P)>> = vec![Vec::new(); edges.len()];
    let mut seen = std::collections::HashSet::new();
    let mut hits = Vec::new();
    for bucket in edge_grid.cells.values() {
        for (x, &i) in bucket.iter().enumerate() {
            for &j in &bucket[x + 1..] {
                let (a, b) = (i.min(j), i.max(j));
                if (skip_same_owner && edges[a].2 == edges[b].2) || !seen.insert((a, b)) {
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
                    hit(a, t, b, u, pt);
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
    splits
}

/// Every edge of closed rings, owned by its ring.
fn ring_edges(rings: &[Vec<P>]) -> Vec<(P, P, usize)> {
    let mut edges = Vec::new();
    for (ri, r) in rings.iter().enumerate() {
        for k in 0..r.len() {
            edges.push((r[k], r[(k + 1) % r.len()], ri));
        }
    }
    edges
}

fn ring_scale(rings: &[Vec<P>]) -> (f64, f64, f64) {
    let all: Vec<P> = rings.iter().flatten().copied().collect();
    let (lo, hi) = bbox(&all);
    let size = (hi.0 - lo.0).max(hi.1 - lo.1).max(1e-9);
    let scale = all
        .iter()
        .map(|p| p.0.abs().max(p.1.abs()))
        .fold(size, f64::max);
    let longest = ring_edges(rings)
        .iter()
        .map(|e| norm(sub(e.1, e.0)))
        .fold(0.0, f64::max);
    (
        size,
        1e-9 * scale,
        (size / 64.0).max(longest / 8.0).max(scale * 1e-9),
    )
}

/// A place where rings cross or touch other than at a shared corner:
/// (ring, edge, other ring, other edge, point). Edge k runs from corner k to k + 1.
pub type Crossing = (usize, usize, usize, usize, P);

/// Every self-intersection and touch between the rings' edges, adjacent
/// edges meeting at their shared corner excepted.
pub fn crossings(rings: &[Vec<P>]) -> Vec<Crossing> {
    let edges = ring_edges(rings);
    let (_, eps, cell) = ring_scale(rings);
    // Each edge's index within its ring, and its ring's length.
    let mut local = Vec::with_capacity(edges.len());
    for r in rings {
        for k in 0..r.len() {
            local.push((k, r.len()));
        }
    }
    let mut out = Vec::new();
    let inner = |t: f64| !t.is_nan() && t > 1e-9 && t < 1.0 - 1e-9;
    split(&edges, cell, eps, false, &mut |a, t, b, u, pt| {
        let (ra, rb) = (edges[a].2, edges[b].2);
        let ((ka, n), (kb, _)) = (local[a], local[b]);
        let adjacent = ra == rb && ((ka + 1) % n == kb || (kb + 1) % n == ka);
        if (inner(t) || inner(u)) || (!adjacent && (!t.is_nan() || !u.is_nan())) {
            out.push((ra, ka, rb, kb, pt));
        }
    });
    out.sort_by_key(|x| (x.0, x.1, x.2, x.3));
    out.dedup_by(|x, y| (x.0, x.1, x.2, x.3) == (y.0, y.1, y.2, y.3) && norm(sub(x.4, y.4)) < 1e-9);
    out
}

/// The region the rings enclose by the even-odd rule, as valid rings:
/// outlines counterclockwise, holes clockwise. Crossing rings are split where
/// they cross, so a bow-tie becomes two triangles.
pub fn even_odd(rings: &[Vec<P>]) -> Vec<Vec<P>> {
    let (size, eps, cell) = ring_scale(rings);
    // As in `boolean`: near-touches made exact, and the side test's nudge a
    // tenth of the snapping distance.
    let snap_tol = (1e-6 * size).min(1e-3);
    let snapped = snap(rings, snap_tol, cell);
    let rings = &snapped[..];
    let edges = ring_edges(rings);
    let mut splits = split(&edges, cell, eps, false, &mut |_, _, _, _, _| {});
    unify(rings, &mut splits, snap_tol);
    let nudge = snap_tol / 10.0;
    let mut kept = Vec::new();
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
            let (m, n) = (mul(add(p, q), 0.5), (dir.1 / l, -dir.0 / l));
            let right = inside_rings(rings, add(m, mul(n, nudge)));
            let left = inside_rings(rings, sub(m, mul(n, nudge)));
            match (left, right) {
                (true, false) => kept.push((p, q)),
                (false, true) => kept.push((q, p)),
                _ => {}
            }
        }
    }
    // Two copies of one piece (rings sharing an edge) would pair up; keep one.
    kept.sort_by(|x, y| bits(x.0).cmp(&bits(y.0)).then(bits(x.1).cmp(&bits(y.1))));
    kept.dedup();
    stitch(kept, eps, size, true)
}

/// A set operation on two regions.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Op {
    Intersection,
    Union,
    Difference,
    SymmetricDifference,
}

/// The region `op(A, B)`, each input read by the even-odd rule, as valid
/// rings (outlines counterclockwise, holes clockwise). Every piece of either
/// boundary is kept exactly when the result's inside differs on its two sides.
pub fn boolean(a: &[Vec<P>], b: &[Vec<P>], op: Op) -> Vec<Vec<P>> {
    let both: Vec<Vec<P>> = a.iter().chain(b).cloned().collect();
    let (size, eps, cell) = ring_scale(&both);
    // Corners that nearly touch the other boundary are put on it, so the two
    // boundaries either meet exactly or stay apart by more than ten times the
    // side test's nudge. Without this a corner a hair off an edge (as a corner
    // on an edge becomes once projected) could be nudged across it, and the
    // piece beside it read on the wrong side. The tolerance is a millionth of
    // the shapes' size, and never more than a millimetre.
    let snap_tol = (1e-6 * size).min(1e-3);
    let snapped = snap(&both, snap_tol, cell);
    let (a, b) = snapped.split_at(a.len());
    let both: Vec<Vec<P>> = snapped.clone();
    let edges = ring_edges(&both);
    let mut splits = split(&edges, cell, eps, false, &mut |_, _, _, _, _| {});
    unify(&both, &mut splits, snap_tol);
    let nudge = snap_tol / 10.0;
    let inside = |p: P| {
        let (ia, ib) = (inside_rings(a, p), inside_rings(b, p));
        match op {
            Op::Intersection => ia && ib,
            Op::Union => ia || ib,
            Op::Difference => ia && !ib,
            Op::SymmetricDifference => ia != ib,
        }
    };
    let mut kept = Vec::new();
    for (i, &(p0, p1, _)) in edges.iter().enumerate() {
        let mut pts = splits[i].clone();
        pts.push((0.0, p0));
        pts.push((1.0, p1));
        pts.sort_by(|x, y| x.0.total_cmp(&y.0));
        pts.dedup_by(|x, y| x.1 == y.1);
        for w in pts.windows(2) {
            let (p, q) = (w[0].1, w[1].1);
            let dir = sub(q, p);
            let l = norm(dir);
            if l <= eps {
                continue;
            }
            let (m, n) = (mul(add(p, q), 0.5), (dir.1 / l, -dir.0 / l));
            match (inside(sub(m, mul(n, nudge))), inside(add(m, mul(n, nudge)))) {
                (true, false) => kept.push((p, q)),
                (false, true) => kept.push((q, p)),
                _ => {}
            }
        }
    }
    kept.sort_by(|x, y| bits(x.0).cmp(&bits(y.0)).then(bits(x.1).cmp(&bits(y.1))));
    kept.dedup();
    stitch(kept, eps, size, true)
}

/// Every corner within `tol` of another edge moved onto it: onto that edge's
/// corner when one is that close, or else onto the edge, which gains the moved
/// corner as a corner of its own, so the two then share it exactly. Any edge
/// counts except the two that meet at the corner itself: another polygon's,
/// another ring of the same polygon (a hole touching its outline), or a far
/// part of the same ring (a ring touching itself). So that two corners near
/// each other do not each move to where the other was, corners and edges are
/// numbered in order and a corner moves only onto an earlier edge; toward a
/// later one it stays put, and is added to it when it is near its middle.
/// Passes repeat until nothing moves, at most four times.
fn snap(rings: &[Vec<P>], tol: f64, cell: f64) -> Vec<Vec<P>> {
    let mut rings = rings.to_vec();
    for _ in 0..4 {
        let edges = ring_edges(&rings);
        let boxes: Vec<(P, P)> = edges
            .iter()
            .map(|e| {
                let (lo, hi) = bbox(&[e.0, e.1]);
                ((lo.0 - tol, lo.1 - tol), (hi.0 + tol, hi.1 + tol))
            })
            .collect();
        let grid = Grid::new(&boxes, cell);
        let mut inserts: Vec<Vec<(f64, P)>> = vec![Vec::new(); edges.len()];
        let mut moved = false;
        let mut next = rings.clone();
        let mut g = 0;
        for ring in next.iter_mut() {
            let (first, n) = (g, ring.len());
            for (vi, v) in ring.iter_mut().enumerate() {
                // Edge k runs from corner k to k + 1: the two that meet here.
                let own = [first + vi, first + (vi + n - 1) % n];
                let me = first + vi;
                g += 1;
                // Onto the nearest earlier edge first.
                let mut best: Option<(f64, usize, f64, P)> = None;
                for &k in grid.at(*v) {
                    let (p, q, _) = edges[k];
                    let d = sub(q, p);
                    let l2 = dot(d, d);
                    if k >= me || own.contains(&k) || l2 == 0.0 {
                        continue;
                    }
                    let t = (dot(sub(*v, p), d) / l2).clamp(0.0, 1.0);
                    let foot = add(p, mul(d, t));
                    let dist = norm(sub(*v, foot));
                    if dist <= tol && best.is_none_or(|b| dist < b.0) {
                        best = Some((dist, k, t, foot));
                    }
                }
                if let Some((_, k, t, foot)) = best {
                    let (p, q, _) = edges[k];
                    let target = if norm(sub(*v, p)) <= tol {
                        p
                    } else if norm(sub(*v, q)) <= tol {
                        q
                    } else {
                        inserts[k].push((t, foot));
                        foot
                    };
                    if *v != target {
                        *v = target;
                        moved = true;
                    }
                }
                // Then into the middle of any later edge it is near.
                for &k in grid.at(*v) {
                    let (p, q, _) = edges[k];
                    let d = sub(q, p);
                    let l2 = dot(d, d);
                    if k < me
                        || own.contains(&k)
                        || l2 == 0.0
                        || norm(sub(*v, p)) <= tol
                        || norm(sub(*v, q)) <= tol
                    {
                        continue;
                    }
                    let t = dot(sub(*v, p), d) / l2;
                    if (0.0..=1.0).contains(&t) && norm(sub(*v, add(p, mul(d, t)))) <= tol {
                        inserts[k].push((t, *v));
                    }
                }
            }
        }
        let mut k = 0;
        for ring in &mut next {
            let mut r = Vec::with_capacity(ring.len());
            for &corner in ring.iter() {
                if r.last() != Some(&corner) {
                    r.push(corner);
                }
                let mut here = std::mem::take(&mut inserts[k]);
                here.sort_by(|x, y| x.0.total_cmp(&y.0));
                for (_, pt) in here {
                    if r.last() != Some(&pt) {
                        r.push(pt);
                        moved = true;
                    }
                }
                k += 1;
            }
            while r.len() > 1 && r.first() == r.last() {
                r.pop();
            }
            *ring = r;
        }
        rings = next;
        if !moved {
            break;
        }
    }
    rings
}

/// Crossing points within `tol` of a corner or of each other made one point,
/// so that where three edges cross at one place (computed pair by pair, the
/// crossings differ in the last digits) the pieces still share a corner.
fn unify(rings: &[Vec<P>], splits: &mut [Vec<(f64, P)>], tol: f64) {
    let key = |p: P| ((p.0 / tol).floor() as i64, (p.1 / tol).floor() as i64);
    let mut reps: HashMap<(i64, i64), Vec<P>> = HashMap::new();
    for &c in rings.iter().flatten() {
        reps.entry(key(c)).or_default().push(c);
    }
    for (_, p) in splits.iter_mut().flatten() {
        let (i, j) = key(*p);
        let near = (i - 1..=i + 1)
            .flat_map(|x| (j - 1..=j + 1).map(move |y| (x, y)))
            .filter_map(|k| reps.get(&k))
            .flatten()
            .copied()
            .find(|r| norm(sub(*r, *p)) <= tol);
        match near {
            Some(r) => *p = r,
            None => reps.entry((i, j)).or_default().push(*p),
        }
    }
}

/// A ring that passes through one corner twice split there into two rings,
/// repeatedly, so shapes that touch at a point come out as separate rings,
/// as valid polygons must (a ring may not touch itself).
fn unpinch(ring: Vec<P>) -> Vec<Vec<P>> {
    let mut out = Vec::new();
    let mut stack = vec![ring];
    while let Some(r) = stack.pop() {
        let mut seen: HashMap<(u64, u64), usize> = HashMap::new();
        let mut cut = None;
        for (j, p) in r.iter().enumerate() {
            if let Some(&i) = seen.get(&bits(*p)) {
                cut = Some((i, j));
                break;
            }
            seen.insert(bits(*p), j);
        }
        match cut {
            None => out.push(r),
            Some((i, j)) => {
                let inner: Vec<P> = r[i..j].to_vec();
                let mut outer: Vec<P> = r[..i].to_vec();
                outer.extend_from_slice(&r[j..]);
                stack.push(inner);
                stack.push(outer);
            }
        }
    }
    out
}

/// Joins directed edges end to start into closed rings. With `pinch`, a ring
/// that passes through one corner twice is split there first (see `unpinch`).
fn stitch(edges: Vec<(P, P)>, eps: f64, r: f64, pinch: bool) -> Vec<Vec<P>> {
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
            let incoming = sub(cur, ring[ring.len() - 2]);
            // Where rings meet at a corner, the sharpest left turn keeps each
            // ring around its own face (the result's inside is on the left),
            // so pieces that touch at a point come out as separate rings.
            let turn = |k: usize| {
                let out = sub(edges[k].1, edges[k].0);
                atan2(cross(incoming, out), dot(incoming, out))
            };
            let next = from
                .get(&bits(cur))
                .and_then(|v| {
                    let free = v.iter().copied().filter(|&k| !used[k]);
                    if pinch {
                        free.max_by(|&x, &y| turn(x).total_cmp(&turn(y)))
                    } else {
                        free.into_iter().next()
                    }
                })
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
        let parts = if pinch { unpinch(ring) } else { vec![ring] };
        for ring in parts {
            let ring = simplify(ring);
            if ring.len() >= 3 && area2(&ring).abs() > 1e-9 * r * r {
                rings.push(ring);
            }
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

/// The input with long edges cut into geodesic pieces, so each plane chord
/// follows its geodesic closely.
pub fn densify(
    g: &Geodesic,
    kind: Shape,
    rings_ll: &[Vec<(f64, f64)>],
) -> Result<Vec<Ring>, ToolError> {
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
    Ok(dense)
}

/// The geodesic distance from each point to the input (0 inside a polygon).
pub fn distances(
    g: &Geodesic,
    kind: Shape,
    rings_ll: &[Vec<(f64, f64)>],
    pts: &[(f64, f64)],
    reach: f64,
) -> Result<Vec<f64>, ToolError> {
    let dense = densify(g, kind, rings_ll)?;
    let all: Vec<(f64, f64)> = dense.iter().flatten().copied().collect();
    let (lat0, lon0) = center(&all);
    let plane_map = Aeqd { g, lat0, lon0 };
    let planar: Vec<Vec<P>> = dense
        .iter()
        .map(|ring| ring.iter().map(|&p| plane_map.fwd(p)).collect())
        .collect();
    let edges = Edges::new(kind, &dense, &plane_map, reach);
    Ok(pts
        .iter()
        .map(|&ll| {
            let p = plane_map.fwd(ll);
            if kind == Shape::Polygon && inside_rings(&planar, p) {
                0.0
            } else {
                edges.nearest(g, ll, p).0
            }
        })
        .collect())
}

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
    let dense = densify(g, kind, rings_ll)?;
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
    fn a_bow_tie_crosses_once_and_repairs_into_two_triangles() {
        let bow = vec![vec![(0.0, 0.0), (10.0, 10.0), (10.0, 0.0), (0.0, 10.0)]];
        let x = crossings(&bow);
        assert_eq!(x.len(), 1, "{x:?}");
        assert!((x[0].4.0 - 5.0).abs() < 1e-9 && (x[0].4.1 - 5.0).abs() < 1e-9);
        let fixed = even_odd(&bow);
        assert_eq!(fixed.len(), 2);
        assert!(
            fixed.iter().all(|r| area2(r) > 0.0),
            "outlines run counterclockwise"
        );
        assert!((area(&fixed) - 50.0).abs() < 1e-9, "{}", area(&fixed));
    }

    #[test]
    fn valid_rings_have_no_crossings_and_repair_to_themselves() {
        let sq = vec![
            vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
            vec![(3.0, 3.0), (3.0, 6.0), (6.0, 6.0), (6.0, 3.0)],
        ];
        assert!(crossings(&sq).is_empty());
        let fixed = even_odd(&sq);
        assert_eq!(fixed.len(), 2);
        assert!((area(&fixed) - 91.0).abs() < 1e-9);
        // A hole touching the outline at one corner is a touch, reported.
        let touch = vec![sq[0].clone(), vec![(0.0, 0.0), (3.0, 6.0), (6.0, 3.0)]];
        assert!(!crossings(&touch).is_empty());
    }

    #[test]
    fn set_operations_on_two_overlapping_squares() {
        let a = vec![vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]];
        let b = vec![vec![(5.0, 5.0), (15.0, 5.0), (15.0, 15.0), (5.0, 15.0)]];
        let got = |op| area(&boolean(&a, &b, op));
        assert!((got(Op::Intersection) - 25.0).abs() < 1e-9);
        assert!((got(Op::Union) - 175.0).abs() < 1e-9);
        assert!((got(Op::Difference) - 75.0).abs() < 1e-9);
        assert!((got(Op::SymmetricDifference) - 150.0).abs() < 1e-9);
        // Sharing an edge exactly: the union is one rectangle, the intersection empty.
        let c = vec![vec![(10.0, 0.0), (20.0, 0.0), (20.0, 10.0), (10.0, 10.0)]];
        let u = boolean(&a, &c, Op::Union);
        assert_eq!(u.len(), 1, "{u:?}");
        assert!((area(&u) - 200.0).abs() < 1e-9);
        assert!(boolean(&a, &c, Op::Intersection).is_empty());
        // One inside the other: the difference has a hole.
        let inner = vec![vec![(2.0, 2.0), (4.0, 2.0), (4.0, 4.0), (2.0, 4.0)]];
        let d = boolean(&a, &inner, Op::Difference);
        assert_eq!(d.len(), 2);
        assert!((area(&d) - 96.0).abs() < 1e-9);
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
