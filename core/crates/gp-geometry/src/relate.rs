//! How two geometries relate (add-navigation-and-geometry,
//! geometry/computational, "Predicates"): the DE-9IM matrix of a point set,
//! line, or polygon with holes against another, and the named predicates the
//! matrix defines (OGC 06-103r4, Simple feature access, part 1).
//!
//! Both geometries are noded against each other: every edge is cut wherever
//! the other geometry crosses it, touches it, or runs along it. The matrix is
//! then read off three kinds of place, each located in both geometries:
//!
//! - the nodes (every vertex and every crossing), giving dimension 0;
//! - the edge pieces between nodes, located by their middle, giving 1;
//! - the two sides of every polygon edge piece, giving 2. Every region of the
//!   plane where the two interiors and exteriors meet is bounded by some
//!   polygon edge, so reading both sides of each finds all of them.
//!
//! Planar edges are straight in longitude and latitude, and every side-of-line
//! test is exact (Shewchuk's orientation predicate on floating-point
//! expansions), so a vertex on an edge is on it however the digits fall.
//! Geodesic edges are cut into pieces of at most 5 km on the ellipsoidal
//! gnomonic plane at the shapes' center, where geodesics are nearly straight
//! (within 0.05 mm for pieces this short within 1,000 km of the center), and
//! places within 1 mm of an edge are on it.

use geographiclib_rs::{DirectGeodesic, Geodesic, InverseGeodesic};
use gp_base::ErrorCode;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::buffer;

use crate::robust::orient_exact;

type P = (f64, f64);

const OGC: Reference = Reference {
    title: "OpenGIS Implementation Specification for Geographic information - Simple feature access - Part 1: Common architecture",
    issuer: "Open Geospatial Consortium",
    year: 2011,
    edition: "OGC 06-103r4, version 1.2.1",
    locator: "The Dimensionally Extended Nine-Intersection Model (DE-9IM) and the named spatial relationship predicates based on it",
    url: "https://www.ogc.org/standards/sfa",
};
const SHEWCHUK: Reference = Reference {
    title: "Adaptive precision floating-point arithmetic and fast robust geometric predicates",
    issuer: "Shewchuk, J. R., Discrete & Computational Geometry",
    year: 1997,
    edition: "Vol. 18, No. 3",
    locator: "pp. 305-363 (expansion arithmetic and the orientation test)",
    url: "https://people.eecs.berkeley.edu/~jrs/papers/robustr.pdf",
};
const KARNEY: Reference = Reference {
    title: "Algorithms for geodesics",
    issuer: "Karney, C. F. F., Journal of Geodesy",
    year: 2013,
    edition: "Vol. 87, No. 1",
    locator: "pp. 43-55, section 8 (the ellipsoidal gnomonic projection)",
    url: "https://doi.org/10.1007/s00190-012-0578-z",
};

/// Within this distance of an edge, geodesic mode counts a place as on it.
const ON_EDGE_M: f64 = 0.001;
/// Longest geodesic piece an edge is cut into on the gnomonic plane.
const PIECE_M: f64 = 5_000.0;
/// How far from the shapes' center geodesic mode reaches.
const REACH_M: f64 = 1_000_000.0;
/// Edge pieces allowed per geometry after cutting.
const MAX_PIECES: usize = 4_000;

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
        "For a polygon: 0 for the outline, 1, 2, … for holes, like 1",
        Kind::Number {
            min: 0.0,
            max: 100.0,
        },
    ),
];
const TEST_ROW: &[Field] = &[
    Field::new(
        "test",
        "Test",
        "The named relationship, like contains",
        Kind::Text { max_len: 12 },
    ),
    Field::new("answer", "Answer", "yes or no", Kind::Text { max_len: 3 }),
];
const KINDS: &[&str] = &["polygon", "line", "points"];

pub static RELATE: ToolDef = ToolDef {
    id: "geometry.predicate.relate",
    stability: gp_base::tool::Stability::Stable,
    title: "How two shapes relate",
    summary: "Whether two shapes (points, a line, or a polygon with holes) intersect, touch, cross, overlap, or lie one inside the other, with the DE-9IM matrix behind every answer, on geodesic or straight edges.",
    aliases: &[
        "spatial relationship",
        "does it intersect",
        "de-9im",
        "relate",
        "contains or within",
        "does the route cross the zone",
    ],
    keywords: &[
        "intersects",
        "contains",
        "within",
        "touches",
        "crosses",
        "overlaps",
        "disjoint",
        "equals",
        "topology",
        "geofence",
    ],
    inputs: &[
        Field::new(
            "geometry_a",
            "First shape",
            "Corners or points in order, like 40.4406, -80.002; a polygon's holes as ring 1, 2, …",
            Kind::List {
                items: VERTEX,
                min: 1,
                max: 2_000,
            },
        )
        .required()
        .core(),
        Field::new(
            "kind_a",
            "First shape is",
            "polygon (the default), line, or points",
            Kind::Choice(KINDS),
        )
        .core(),
        Field::new(
            "geometry_b",
            "Second shape",
            "Corners or points in order, like 40.4406, -80.002; a polygon's holes as ring 1, 2, …",
            Kind::List {
                items: VERTEX,
                min: 1,
                max: 2_000,
            },
        )
        .required()
        .core(),
        Field::new(
            "kind_b",
            "Second shape is",
            "polygon (the default), line, or points",
            Kind::Choice(KINDS),
        )
        .core(),
        Field::new(
            "edges",
            "Edges",
            "geodesic (the default), or planar: straight in longitude and latitude, as most mapping software draws them",
            Kind::Choice(&["geodesic", "planar"]),
        ),
    ],
    outputs: &[
        Field::new(
            "relation",
            "Relation",
            "The first shape against the second, like lies inside",
            Kind::Text { max_len: 24 },
        ),
        Field::new(
            "matrix",
            "DE-9IM matrix",
            "Nine characters: interior, boundary, exterior of the first against each of the second's, like 212101212",
            Kind::Text { max_len: 9 },
        ),
        Field::new(
            "tests",
            "Each test",
            "The named relationships, each yes or no",
            Kind::List {
                items: TEST_ROW,
                min: 8,
                max: 8,
            },
        ),
    ],
    errors: &[ErrorCode::OutOfDomain, ErrorCode::LimitExceeded],
    warnings: &[],
    model: "Both shapes noded against each other; the DE-9IM matrix read from every node (dimension 0), every edge piece between nodes (1), and both sides of every polygon edge piece (2), each located in both shapes; the named tests from the matrix patterns of OGC 06-103r4 (Simple feature access). Planar: straight edges in longitude and latitude, with Shewchuk's exact orientation test. Geodesic: edges cut into 5 km geodesic pieces on the ellipsoidal gnomonic plane at the shapes' center (Karney 2013), on an edge within 1 mm",
    accuracy: "Planar: exact for the coordinates as given. Geodesic: on an edge within 1 mm; geodesic pieces follow their chords to 0.05 mm within 1,000 km of the shapes' center",
    when_to_use: "Use this to ask how two shapes sit against each other rather than how big their overlap is: whether a planned flight line crosses a restricted zone or only grazes its edge, whether a parcel lies wholly inside a district, whether two survey blocks share a boundary without overlapping, whether any of a set of points falls in an area. Every answer comes with the DE-9IM matrix, the standard nine-cell record of which parts meet, so the result can be checked against any GIS that follows the same standard.",
    limitations: "Inside here is strict, as in the standard: a point on a polygon's boundary is not within it, and a line along a polygon's edge touches it rather than lying inside. The shapes must be valid, with a polygon's holes inside its outline and no ring crossing itself or another; repair one first with the make-valid tool. A line is one unbranched path, and one that crosses itself is read as if it did not. Geodesic mode needs both shapes within 1,000 km of their shared center and treats places within a millimetre of an edge as on it; planar mode draws straight lines in longitude and latitude, which is what most GIS software does with unprojected data and which differs from the ground for long edges, most of all near the poles and for east-west edges.",
    references: &[OGC, SHEWCHUK, KARNEY],
    examples: &[Example {
        id: "primary",
        title: "Does a flight line cross a restricted zone?",
        input: r#"{"geometry_a":[{"lat":40.001,"lon":-105.004},{"lat":40.009,"lon":-104.991}],"kind_a":"line","geometry_b":[{"lat":40.0,"lon":-105.0},{"lat":40.0,"lon":-104.99},{"lat":40.006,"lon":-104.99},{"lat":40.006,"lon":-105.0}],"kind_b":"polygon"}"#,
        source: "GEOS 3.11.4 through shapely (the reference implementation of the DE-9IM) on the same gnomonic plane gives the matrix 101FF0212; the line enters the zone's west edge and leaves by its north edge",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geometry.predicate.point-in-polygon",
            reason: "alternative",
        },
        Related {
            id: "geometry.overlay.boolean",
            reason: "next",
        },
        Related {
            id: "geometry.buffer.geodesic",
            reason: "next",
        },
    ],
    sentence: "The first shape {relation} the second.",
    limits: &[("batchRows", 2_000)],
    run: run_relate,
    ..ToolDef::BLANK
};

// ---------------------------------------------------------------------------
// Plane geometry with a tolerance (0 = exact).

fn sub(a: P, b: P) -> P {
    (a.0 - b.0, a.1 - b.1)
}
fn dot(a: P, b: P) -> f64 {
    a.0 * b.0 + a.1 * b.1
}
fn cross(a: P, b: P) -> f64 {
    a.0 * b.1 - a.1 * b.0
}

struct Plane {
    tol: f64,
}

impl Plane {
    fn orient(&self, a: P, b: P, c: P) -> i8 {
        if self.tol == 0.0 {
            return orient_exact(a, b, c);
        }
        let d = sub(b, a);
        let det = cross(d, sub(c, a));
        if det.abs() <= self.tol * d.0.hypot(d.1) {
            0
        } else {
            det.signum() as i8
        }
    }
    fn same(&self, p: P, q: P) -> bool {
        if self.tol == 0.0 {
            p == q
        } else {
            (p.0 - q.0).hypot(p.1 - q.1) <= self.tol
        }
    }
    /// Where p falls along a → b: 0 at a, 1 at b.
    fn param(a: P, b: P, p: P) -> f64 {
        let d = sub(b, a);
        dot(sub(p, a), d) / dot(d, d)
    }
    fn on_seg(&self, a: P, b: P, p: P) -> bool {
        if self.orient(a, b, p) != 0 {
            return false;
        }
        if self.tol == 0.0 {
            return p.0 >= a.0.min(b.0)
                && p.0 <= a.0.max(b.0)
                && p.1 >= a.1.min(b.1)
                && p.1 <= a.1.max(b.1);
        }
        let d = sub(b, a);
        let slack = self.tol / d.0.hypot(d.1);
        let t = Self::param(a, b, p);
        (-slack..=1.0 + slack).contains(&t)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Dim {
    Points,
    Line,
    Area,
}

const IN: usize = 0;
const BD: usize = 1;
const EX: usize = 2;

struct Seg {
    a: P,
    b: P,
    /// For a polygon: whether its interior lies left of a → b.
    left_in: bool,
}

struct Shape {
    dim: Dim,
    /// Points: the points. Line: its vertices. Area: the rings, outline first.
    parts: Vec<Vec<P>>,
    segs: Vec<Seg>,
}

fn signed_area(r: &[P]) -> f64 {
    (0..r.len())
        .map(|i| cross(r[i], r[(i + 1) % r.len()]))
        .sum()
}

impl Shape {
    fn new(dim: Dim, parts: Vec<Vec<P>>) -> Shape {
        let mut segs = Vec::new();
        match dim {
            Dim::Points => {}
            Dim::Line => {
                for w in parts[0].windows(2) {
                    if w[0] != w[1] {
                        segs.push(Seg {
                            a: w[0],
                            b: w[1],
                            left_in: false,
                        });
                    }
                }
            }
            Dim::Area => {
                for (k, r) in parts.iter().enumerate() {
                    // An outline wound counterclockwise, or a hole clockwise,
                    // has the polygon's interior on its left.
                    let left_in = (signed_area(r) > 0.0) == (k == 0);
                    for i in 0..r.len() {
                        let (a, b) = (r[i], r[(i + 1) % r.len()]);
                        if a != b {
                            segs.push(Seg { a, b, left_in });
                        }
                    }
                }
            }
        }
        Shape { dim, parts, segs }
    }

    /// The location of an edge piece in its own shape.
    fn own(&self) -> usize {
        if self.dim == Dim::Line { IN } else { BD }
    }

    fn vertices(&self) -> impl Iterator<Item = P> + '_ {
        self.parts.iter().flatten().copied()
    }

    /// Inside by the even-odd rule, for a place known not to be on an edge.
    fn inside(&self, pl: &Plane, p: P) -> bool {
        let mut inside = false;
        for s in &self.segs {
            let (a, b) = (s.a, s.b);
            if (a.1 > p.1) != (b.1 > p.1) {
                // The ray to +x meets an upward edge when p is left of it,
                // and a downward edge when p is right of it.
                let o = if pl.tol == 0.0 {
                    orient_exact(a, b, p)
                } else {
                    cross(sub(b, a), sub(p, a)).signum() as i8
                };
                if (a.1 < b.1 && o > 0) || (a.1 > b.1 && o < 0) {
                    inside = !inside;
                }
            }
        }
        inside
    }

    fn locate(&self, pl: &Plane, p: P) -> usize {
        match self.dim {
            Dim::Points => {
                if self.parts[0].iter().any(|&q| pl.same(p, q)) {
                    IN
                } else {
                    EX
                }
            }
            Dim::Line => {
                let path = &self.parts[0];
                let (first, last) = (path[0], path[path.len() - 1]);
                if first != last && (pl.same(p, first) || pl.same(p, last)) {
                    BD
                } else if self.segs.iter().any(|s| pl.on_seg(s.a, s.b, p)) {
                    IN
                } else {
                    EX
                }
            }
            Dim::Area => {
                if self.segs.iter().any(|s| pl.on_seg(s.a, s.b, p)) {
                    BD
                } else if self.inside(pl, p) {
                    IN
                } else {
                    EX
                }
            }
        }
    }
}

/// Cut points and shared stretches found on one shape's edges.
#[derive(Default, Clone)]
struct Cuts {
    at: Vec<P>,
    /// Stretches (from, to, other edge) running along an edge of the other shape.
    shared: Vec<(f64, f64, usize)>,
}

fn bbox(s: &Seg, tol: f64) -> (f64, f64, f64, f64) {
    (
        s.a.0.min(s.b.0) - tol,
        s.a.1.min(s.b.1) - tol,
        s.a.0.max(s.b.0) + tol,
        s.a.1.max(s.b.1) + tol,
    )
}

/// The DE-9IM matrix of `a` against `b`: entry [i][j] is the dimension of
/// where a's part i (interior, boundary, exterior) meets b's part j, or -1.
fn matrix(pl: &Plane, a: &Shape, b: &Shape) -> [[i8; 3]; 3] {
    let mut m = [[-1i8; 3]; 3];
    m[EX][EX] = 2;
    let mut put = |la: usize, lb: usize, d: i8| {
        if m[la][lb] < d {
            m[la][lb] = d;
        }
    };
    let mut cuts_a = vec![Cuts::default(); a.segs.len()];
    let mut cuts_b = vec![Cuts::default(); b.segs.len()];

    // Edge against edge, pruned by boxes swept along x.
    let mut order: Vec<usize> = (0..b.segs.len()).collect();
    let boxes_b: Vec<_> = b.segs.iter().map(|s| bbox(s, pl.tol)).collect();
    order.sort_by(|&i, &j| boxes_b[i].0.total_cmp(&boxes_b[j].0));
    for (ia, sa) in a.segs.iter().enumerate() {
        let ba = bbox(sa, pl.tol);
        let end = order.partition_point(|&j| boxes_b[j].0 <= ba.2);
        for &ib in &order[..end] {
            let bb = boxes_b[ib];
            if bb.2 < ba.0 || bb.1 > ba.3 || bb.3 < ba.1 {
                continue;
            }
            let sb = &b.segs[ib];
            let (p, q, r, s) = (sa.a, sa.b, sb.a, sb.b);
            let (o1, o2) = (pl.orient(p, q, r), pl.orient(p, q, s));
            let (o3, o4) = (pl.orient(r, s, p), pl.orient(r, s, q));
            if o1 == 0 && o2 == 0 {
                // Collinear: cut each at the other's ends that fall on it, and
                // record the stretch they share.
                for v in [r, s] {
                    if pl.on_seg(p, q, v) {
                        cuts_a[ia].at.push(v);
                    }
                }
                for v in [p, q] {
                    if pl.on_seg(r, s, v) {
                        cuts_b[ib].at.push(v);
                    }
                }
                let (tr, ts) = (Plane::param(p, q, r), Plane::param(p, q, s));
                let (lo, hi) = (tr.min(ts).max(0.0), tr.max(ts).min(1.0));
                let (tp, tq) = (Plane::param(r, s, p), Plane::param(r, s, q));
                let (lo2, hi2) = (tp.min(tq).max(0.0), tp.max(tq).min(1.0));
                if hi > lo && hi2 > lo2 {
                    cuts_a[ia].shared.push((lo, hi, ib));
                    cuts_b[ib].shared.push((lo2, hi2, ia));
                }
                continue;
            }
            if o1 == 0 && pl.on_seg(p, q, r) {
                cuts_a[ia].at.push(r);
            }
            if o2 == 0 && pl.on_seg(p, q, s) {
                cuts_a[ia].at.push(s);
            }
            if o3 == 0 && pl.on_seg(r, s, p) {
                cuts_b[ib].at.push(p);
            }
            if o4 == 0 && pl.on_seg(r, s, q) {
                cuts_b[ib].at.push(q);
            }
            if o1 * o2 < 0 && o3 * o4 < 0 {
                let (d1, d2) = (sub(q, p), sub(s, r));
                let t = cross(sub(r, p), d2) / cross(d1, d2);
                let x = (p.0 + t * d1.0, p.1 + t * d1.1);
                cuts_a[ia].at.push(x);
                cuts_b[ib].at.push(x);
                // A proper crossing is inside both edges, so it is where a's
                // edge meets b's edge, whatever rounding did to x.
                put(a.own(), b.own(), 0);
            }
        }
    }
    // Isolated points cut the edges they lie on.
    for (pts, other, cuts) in [(a, b, &mut cuts_b), (b, a, &mut cuts_a)] {
        if pts.dim == Dim::Points {
            for &v in &pts.parts[0] {
                for (k, s) in other.segs.iter().enumerate() {
                    if pl.on_seg(s.a, s.b, v) {
                        cuts[k].at.push(v);
                    }
                }
            }
        }
    }

    // Nodes: every vertex, located in both shapes.
    for v in a.vertices() {
        put(a.locate(pl, v), b.locate(pl, v), 0);
    }
    for v in b.vertices() {
        put(a.locate(pl, v), b.locate(pl, v), 0);
    }

    // Edge pieces, and the two sides of each polygon piece.
    for (x, y, cuts, x_is_a) in [(a, b, &cuts_a, true), (b, a, &cuts_b, false)] {
        for (k, s) in x.segs.iter().enumerate() {
            let mut pts: Vec<(f64, P)> = cuts[k]
                .at
                .iter()
                .filter(|&&v| !pl.same(v, s.a) && !pl.same(v, s.b))
                .map(|&v| (Plane::param(s.a, s.b, v), v))
                .collect();
            pts.push((0.0, s.a));
            pts.push((1.0, s.b));
            pts.sort_by(|u, v| u.0.total_cmp(&v.0));
            pts.dedup_by(|u, v| pl.same(u.1, v.1));
            for w in pts.windows(2) {
                let ((t0, p0), (t1, p1)) = (w[0], w[1]);
                let (tm, mid) = ((t0 + t1) / 2.0, ((p0.0 + p1.0) / 2.0, (p0.1 + p1.1) / 2.0));
                let along = cuts[k]
                    .shared
                    .iter()
                    .find(|&&(lo, hi, _)| lo <= tm && tm <= hi)
                    .map(|&(_, _, o)| o);
                let there = if along.is_some() {
                    y.own()
                } else {
                    y.locate(pl, mid)
                };
                let mut record = |lx: usize, ly: usize, d: i8| {
                    if x_is_a {
                        put(lx, ly, d)
                    } else {
                        put(ly, lx, d)
                    }
                };
                record(x.own(), there, 1);
                if x.dim != Dim::Area {
                    continue;
                }
                let (left, right) = if s.left_in { (IN, EX) } else { (EX, IN) };
                let (y_left, y_right) = match (y.dim, along) {
                    (Dim::Area, Some(o)) => {
                        let so = &y.segs[o];
                        let same_way = dot(sub(p1, p0), sub(so.b, so.a)) > 0.0;
                        if so.left_in == same_way {
                            (IN, EX)
                        } else {
                            (EX, IN)
                        }
                    }
                    (Dim::Area, None) => {
                        let l = if y.inside(pl, mid) { IN } else { EX };
                        (l, l)
                    }
                    _ => (EX, EX),
                };
                record(left, y_left, 2);
                record(right, y_right, 2);
            }
        }
    }
    m
}

fn text(m: &[[i8; 3]; 3]) -> String {
    m.iter()
        .flatten()
        .map(|&d| if d < 0 { 'F' } else { (b'0' + d as u8) as char })
        .collect()
}

/// Whether a matrix string matches an OGC pattern (T, F, *, 0, 1, 2).
fn matches(m: &str, pat: &str) -> bool {
    m.chars().zip(pat.chars()).all(|(c, p)| match p {
        '*' => true,
        'T' => c != 'F',
        _ => c == p,
    })
}

fn rank(d: Dim) -> u8 {
    match d {
        Dim::Points => 0,
        Dim::Line => 1,
        Dim::Area => 2,
    }
}

/// The eight named tests, in a fixed order.
fn tests(m: &str, da: Dim, db: Dim) -> [(&'static str, bool); 8] {
    let disjoint = matches(m, "FF*FF****");
    let equals = matches(m, "T*F**FFF*");
    let within = matches(m, "T*F**F***");
    let contains = matches(m, "T*****FF*");
    let touches = !(da == Dim::Points && db == Dim::Points)
        && (matches(m, "FT*******") || matches(m, "F**T*****") || matches(m, "F***T****"));
    let crosses = match rank(da).cmp(&rank(db)) {
        core::cmp::Ordering::Less => matches(m, "T*T******"),
        core::cmp::Ordering::Greater => matches(m, "T*****T**"),
        core::cmp::Ordering::Equal => da == Dim::Line && matches(m, "0********"),
    };
    let overlaps = match (da, db) {
        (Dim::Points, Dim::Points) | (Dim::Area, Dim::Area) => matches(m, "T*T***T**"),
        (Dim::Line, Dim::Line) => matches(m, "1*T***T**"),
        _ => false,
    };
    [
        ("intersects", !disjoint),
        ("disjoint", disjoint),
        ("equals", equals),
        ("within", within),
        ("contains", contains),
        ("touches", touches),
        ("crosses", crosses),
        ("overlaps", overlaps),
    ]
}

fn relation(t: &[(&str, bool); 8]) -> &'static str {
    let has = |name: &str| t.iter().any(|&(n, v)| n == name && v);
    if has("equals") {
        "is the same shape as"
    } else if has("within") {
        "lies inside"
    } else if has("contains") {
        "contains"
    } else if has("overlaps") {
        "overlaps"
    } else if has("crosses") {
        "crosses"
    } else if has("touches") {
        "touches"
    } else if has("disjoint") {
        "does not meet"
    } else {
        "meets"
    }
}

// ---------------------------------------------------------------------------
// Reading the input.

fn read(ctx: &mut Ctx, list: &str, dim: Dim) -> Result<Vec<Vec<(f64, f64)>>, ToolError> {
    let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let rows = ctx.rows(list)?;
    let mut parts: Vec<Vec<(f64, f64)>> = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        let lat = ctx
            .row_quantity(list, i, r, "lat")?
            .expect("required")
            .to(deg);
        let lon = ctx
            .row_quantity(list, i, r, "lon")?
            .expect("required")
            .to(deg);
        if !(-90.0..=90.0).contains(&lat) {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "Latitude must be between -90° and 90°.",
            )
            .at(&format!("/{list}/{i}/lat")));
        }
        let ring = r
            .get("ring")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);
        if ring.fract() != 0.0 || !(0.0..=100.0).contains(&ring) {
            return Err(ToolError::invalid(
                &format!("/{list}/{i}/ring"),
                "Ring must be a whole number from 0 (the outline) to 100.",
            ));
        }
        if ring != 0.0 && dim != Dim::Area {
            return Err(ToolError::invalid(
                &format!("/{list}/{i}/ring"),
                "Only a polygon has rings; leave ring out for a line or points.",
            ));
        }
        let k = ring as usize;
        if parts.len() <= k {
            parts.resize(k + 1, Vec::new());
        }
        if dim == Dim::Points || parts[k].last() != Some(&(lat, lon)) {
            parts[k].push((lat, lon));
        }
    }
    parts.retain(|r| !r.is_empty());
    match dim {
        Dim::Points => {}
        Dim::Line => {
            if parts[0].len() < 2 {
                return Err(ToolError::invalid(
                    &format!("/{list}"),
                    "A line needs at least 2 distinct points.",
                ));
            }
        }
        Dim::Area => {
            for (k, ring) in parts.iter_mut().enumerate() {
                if ring.len() > 1 && ring.first() == ring.last() {
                    ring.pop();
                }
                if ring.len() < 3 {
                    return Err(ToolError::invalid(
                        &format!("/{list}"),
                        format!("Ring {k} needs at least 3 distinct corners."),
                    ));
                }
                gp_geo::point::refuse_repeated_corner(ring, &format!("/{list}"))?;
            }
        }
    }
    Ok(parts)
}

fn dim_of(ctx: &Ctx, name: &str) -> Result<Dim, ToolError> {
    Ok(match ctx.choice(name)? {
        Some("line") => Dim::Line,
        Some("points") => Dim::Points,
        _ => Dim::Area,
    })
}

/// The ellipsoidal gnomonic projection centered at `c` (Karney 2013, section 8).
fn gnomonic(g: &Geodesic, c: (f64, f64), p: (f64, f64)) -> Option<(P, f64)> {
    let (s, az, _, m12, big_m12, _, _, _): (f64, f64, f64, f64, f64, f64, f64, f64) =
        g.inverse(c.0, c.1, p.0, p.1);
    if big_m12 <= 0.0 {
        return None;
    }
    let rho = m12 / big_m12;
    let t = az.to_radians();
    Some(((rho * libm::sin(t), rho * libm::cos(t)), s))
}

/// Each part on the gnomonic plane, with edges cut into short geodesic pieces.
fn project(
    g: &Geodesic,
    c: (f64, f64),
    dim: Dim,
    parts: &[Vec<(f64, f64)>],
    field: &str,
) -> Result<Vec<Vec<P>>, ToolError> {
    let far = || {
        ToolError::new(
            ErrorCode::OutOfDomain,
            "Geodesic mode needs both shapes within 1,000 km of their shared center. Use planar edges, or compare smaller pieces.",
        )
        .at(&format!("/{field}"))
    };
    let mut out = Vec::with_capacity(parts.len());
    let mut total = 0usize;
    for part in parts {
        let n = part.len();
        let edges = match dim {
            Dim::Points => 0,
            Dim::Line => n - 1,
            Dim::Area => n,
        };
        let mut pts = Vec::new();
        if edges == 0 {
            for &v in part {
                let (xy, s) = gnomonic(g, c, v).ok_or_else(far)?;
                if s > REACH_M {
                    return Err(far());
                }
                pts.push(xy);
            }
        }
        for i in 0..edges {
            let (a, b) = (part[i], part[(i + 1) % n]);
            let (s, az, _, _): (f64, f64, f64, f64) = g.inverse(a.0, a.1, b.0, b.1);
            let k = (libm::ceil(s / PIECE_M) as usize).max(1);
            total += k;
            if total > MAX_PIECES {
                return Err(ToolError::new(
                    ErrorCode::LimitExceeded,
                    "A shape is too long to compare here: over 4,000 edge pieces of 5 km. Use planar edges, or fewer and shorter edges.",
                )
                .at(&format!("/{field}")));
            }
            for j in 0..k {
                let v = if j == 0 {
                    a
                } else {
                    let (la, lo, _): (f64, f64, f64) =
                        g.direct(a.0, a.1, az, s * j as f64 / k as f64);
                    (la, lo)
                };
                let (xy, d) = gnomonic(g, c, v).ok_or_else(far)?;
                if d > REACH_M {
                    return Err(far());
                }
                pts.push(xy);
            }
        }
        if dim == Dim::Line {
            let (xy, d) = gnomonic(g, c, part[n - 1]).ok_or_else(far)?;
            if d > REACH_M {
                return Err(far());
            }
            pts.push(xy);
        }
        out.push(pts);
    }
    Ok(out)
}

fn run_relate(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (da, db) = (dim_of(ctx, "kind_a")?, dim_of(ctx, "kind_b")?);
    let a = read(ctx, "geometry_a", da)?;
    let b = read(ctx, "geometry_b", db)?;
    let planar = ctx.choice("edges")? == Some("planar");
    let (pl, sa, sb) = if planar {
        let flip = |parts: &[Vec<(f64, f64)>]| -> Vec<Vec<P>> {
            parts
                .iter()
                .map(|r| r.iter().map(|&(la, lo)| (lo, la)).collect())
                .collect()
        };
        (Plane { tol: 0.0 }, flip(&a), flip(&b))
    } else {
        let g = Geodesic::wgs84();
        let all: Vec<(f64, f64)> = a.iter().chain(b.iter()).flatten().copied().collect();
        let c = buffer::center(&all);
        let pa = project(&g, c, da, &a, "geometry_a")?;
        let pb = project(&g, c, db, &b, "geometry_b")?;
        (Plane { tol: ON_EDGE_M }, pa, pb)
    };
    let (ga, gb) = (Shape::new(da, sa), Shape::new(db, sb));
    let m = text(&matrix(&pl, &ga, &gb));
    let t = tests(&m, da, db);
    let rows = t
        .iter()
        .map(|&(name, v)| {
            Json::obj([
                ("test", Json::str(name)),
                ("answer", Json::str(if v { "yes" } else { "no" })),
            ])
        })
        .collect();
    Ok(Json::obj([
        ("relation", Json::str(relation(&t))),
        ("matrix", Json::str(&m)),
        ("tests", Json::Arr(rows)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_tests_follow_their_patterns() {
        let t = tests("212101212", Dim::Area, Dim::Area);
        assert!(t.contains(&("overlaps", true)) && t.contains(&("intersects", true)));
        let t = tests("FF2FF1212", Dim::Area, Dim::Area);
        assert!(t.contains(&("disjoint", true)) && !t.contains(&("touches", true)));
        let t = tests("FF2F11212", Dim::Area, Dim::Area);
        assert!(t.contains(&("touches", true)));
        assert_eq!(
            relation(&tests("2FFF1FFF2", Dim::Area, Dim::Area)),
            "is the same shape as"
        );
    }
}
