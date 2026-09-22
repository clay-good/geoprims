//! Line and polygon simplification (add-navigation-and-geometry,
//! geometry/computational, "Simplification"): Ramer-Douglas-Peucker with a
//! distance tolerance and Visvalingam-Whyatt with an area threshold or a
//! target vertex count. Vertices are chosen on an azimuthal equidistant plane
//! centered on the shape; RDP's tolerance is then checked on the ellipsoid, so
//! every removed vertex lies within it of its simplified geodesic edge. The
//! topology option puts vertices back until no two edges cross.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use geographiclib_rs::Geodesic;
use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::buffer::{Aeqd, REACH, center, seg_dist};

type P = (f64, f64);

const DOUGLAS_PEUCKER: Reference = Reference {
    title: "Algorithms for the reduction of the number of points required to represent a digitized line or its caricature",
    issuer: "Douglas, D. H., and Peucker, T. K., Cartographica",
    year: 1973,
    edition: "Vol. 10, No. 2",
    locator: "pp. 112-122",
    url: "https://doi.org/10.3138/FM57-6770-U75U-7727",
};
const VISVALINGAM: Reference = Reference {
    title: "Line generalisation by repeated elimination of points",
    issuer: "Visvalingam, M., and Whyatt, J. D., The Cartographic Journal",
    year: 1993,
    edition: "Vol. 30, No. 1",
    locator: "pp. 46-51 (effective area)",
    url: "https://doi.org/10.1179/000870493786962263",
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
];

const MAX_VERTICES: usize = 10_000;

const POINTS: Field = Field::new(
    "points",
    "Vertices",
    "In order, like 40.0, -105.0 on each row; a repeated closing corner is dropped",
    Kind::List {
        items: VERTEX,
        min: 2,
        max: MAX_VERTICES,
    },
)
.required()
.core();
const SHAPE: Field = Field::new(
    "shape",
    "Shape",
    "line (default) or polygon (a closed ring)",
    Kind::Choice(&["line", "polygon"]),
)
.core();
const TOPOLOGY: Field = Field::new(
    "preserve_topology",
    "Keep edges from crossing",
    "yes (default) puts vertices back so no two edges cross; no is plain simplification",
    Kind::Choice(&["yes", "no"]),
)
.core();

const OUTPUTS: &[Field] = &[
    Field::new(
        "vertices_out",
        "Vertices kept",
        "After simplification",
        Kind::Number { min: 0.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "vertices_in",
        "Vertices given",
        "Before simplification",
        Kind::Number { min: 0.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "max_deviation",
        "Largest deviation",
        "From a removed vertex to its simplified geodesic edge",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(Precision::Decimals(3)),
    Field::new(
        "restored",
        "Vertices put back",
        "To keep edges from crossing",
        Kind::Number { min: 0.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "simplified",
        "Simplified vertices",
        "In order; a polygon is not closed with a repeated corner",
        Kind::List {
            items: OUT_VERTEX,
            min: 0,
            max: MAX_VERTICES,
        },
    ),
];

struct Input {
    ll: Vec<(f64, f64)>,
    xy: Vec<P>,
    ring: bool,
    topology: bool,
}

fn read(ctx: &mut Ctx, g: &Geodesic) -> Result<Input, ToolError> {
    let rows = ctx.rows("points")?;
    let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let mut ll = Vec::with_capacity(rows.len());
    for (i, r) in rows.iter().enumerate() {
        let lat = ctx
            .row_quantity("points", i, r, "lat")?
            .expect("required")
            .to(deg);
        let lon = ctx
            .row_quantity("points", i, r, "lon")?
            .expect("required")
            .to(deg);
        if !(-90.0..=90.0).contains(&lat) || !lon.is_finite() {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "Latitude must be between -90° and 90°.",
            )
            .at(&format!("/points/{i}/lat")));
        }
        // Consecutive repeats add nothing and would make zero-length edges.
        if ll.last() != Some(&(lat, lon)) {
            ll.push((lat, lon));
        }
    }
    let ring = ctx.choice("shape")? == Some("polygon");
    if ring && ll.len() > 1 && ll.first() == ll.last() {
        ll.pop();
    }
    let need = if ring { 3 } else { 2 };
    if ll.len() < need {
        return Err(ToolError::invalid(
            "/points",
            if ring {
                "A polygon needs at least 3 different corners."
            } else {
                "A line needs at least 2 different points."
            },
        ));
    }
    let (lat0, lon0) = center(&ll);
    let map = Aeqd { g, lat0, lon0 };
    let xy: Vec<P> = ll.iter().map(|&p| map.fwd(p)).collect();
    let far = xy.iter().map(|p| p.0.hypot(p.1)).fold(0.0, f64::max);
    if far > REACH * 5.0 {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "Simplification works on shapes within 5,000 km of their center; split a larger one.",
        )
        .at("/points"));
    }
    let topology = ctx.choice("preserve_topology")? != Some("no");
    let input = Input {
        ll,
        xy,
        ring,
        topology,
    };
    if topology && !crossings(&input.xy, &all(input.xy.len()), ring).is_empty() {
        return Err(ToolError::invalid(
            "/points",
            "The input already crosses itself, so there is no topology to keep. Set preserve_topology to no, or fix the shape first.",
        ));
    }
    Ok(input)
}

fn all(n: usize) -> Vec<usize> {
    (0..n).collect()
}

/// Planar distance from `p` to the segment a→b.
fn seg_plane(a: P, b: P, p: P) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l2 = dx * dx + dy * dy;
    let t = if l2 == 0.0 {
        0.0
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2).clamp(0.0, 1.0)
    };
    (p.0 - a.0 - t * dx).hypot(p.1 - a.1 - t * dy)
}

/// Vertex indices strictly inside the span from `a` to `b` (cyclic for a ring).
fn inner(a: usize, b: usize, n: usize) -> impl Iterator<Item = usize> {
    let len = if b > a { b - a } else { b + n - a };
    (1..len).map(move |k| (a + k) % n)
}

/// The inner vertex farthest (on the plane) from the chord a→b.
fn farthest(xy: &[P], a: usize, b: usize) -> Option<(usize, f64)> {
    inner(a, b, xy.len())
        .map(|k| (k, seg_plane(xy[a], xy[b], xy[k])))
        .max_by(|x, y| x.1.total_cmp(&y.1))
}

/// Edges of the kept path, as index pairs into the vertices.
fn edges(kept: &[usize], ring: bool) -> Vec<(usize, usize)> {
    let mut e: Vec<(usize, usize)> = kept.windows(2).map(|w| (w[0], w[1])).collect();
    if ring && kept.len() > 2 {
        e.push((kept[kept.len() - 1], kept[0]));
    }
    e
}

fn orient(a: P, b: P, c: P) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

fn on_segment(a: P, b: P, p: P) -> bool {
    p.0 >= a.0.min(b.0) && p.0 <= a.0.max(b.0) && p.1 >= a.1.min(b.1) && p.1 <= a.1.max(b.1)
}

/// Whether segments p1→p2 and q1→q2 cross or touch.
fn cross(p1: P, p2: P, q1: P, q2: P) -> bool {
    let (d1, d2) = (orient(q1, q2, p1), orient(q1, q2, p2));
    let (d3, d4) = (orient(p1, p2, q1), orient(p1, p2, q2));
    if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
    {
        return true;
    }
    (d1 == 0.0 && on_segment(q1, q2, p1))
        || (d2 == 0.0 && on_segment(q1, q2, p2))
        || (d3 == 0.0 && on_segment(p1, p2, q1))
        || (d4 == 0.0 && on_segment(p1, p2, q2))
}

/// Pairs of kept edges (by position in `edges`) that cross or touch, other
/// than neighbors meeting at their shared vertex. A sweep over x.
fn crossings(xy: &[P], kept: &[usize], ring: bool) -> Vec<(usize, usize)> {
    let es = edges(kept, ring);
    let m = es.len();
    let mut order: Vec<usize> = (0..m).collect();
    let lo = |e: (usize, usize)| xy[e.0].0.min(xy[e.1].0);
    let hi = |e: (usize, usize)| xy[e.0].0.max(xy[e.1].0);
    order.sort_by(|&i, &j| lo(es[i]).total_cmp(&lo(es[j])));
    let mut out = Vec::new();
    for (k, &i) in order.iter().enumerate() {
        let end = hi(es[i]);
        for &j in &order[k + 1..] {
            if lo(es[j]) > end {
                break;
            }
            let (a, b) = (i.min(j), i.max(j));
            let neighbors = b == a + 1 || (ring && a == 0 && b == m - 1);
            let (e, f) = (es[a], es[b]);
            if neighbors {
                // Neighbors share one vertex; they only conflict if they fold back.
                let shared = if e.1 == f.0 { e.1 } else { e.0 };
                let (u, v) = (
                    if e.0 == shared { e.1 } else { e.0 },
                    if f.0 == shared { f.1 } else { f.0 },
                );
                let (s, pu, pv) = (xy[shared], xy[u], xy[v]);
                if orient(s, pu, pv) == 0.0
                    && (pu.0 - s.0) * (pv.0 - s.0) + (pu.1 - s.1) * (pv.1 - s.1) > 0.0
                {
                    out.push((a, b));
                }
                continue;
            }
            if cross(xy[e.0], xy[e.1], xy[f.0], xy[f.1]) {
                out.push((a, b));
            }
        }
    }
    out
}

/// Puts back, for every pair of crossing edges, each edge's farthest inner
/// vertex, until nothing crosses. Returns how many came back.
fn untangle(xy: &[P], keep: &mut [bool], ring: bool) -> usize {
    let mut restored = 0;
    loop {
        let kept: Vec<usize> = (0..xy.len()).filter(|&i| keep[i]).collect();
        let found = crossings(xy, &kept, ring);
        if found.is_empty() {
            return restored;
        }
        let es = edges(&kept, ring);
        let mut added = false;
        for (a, b) in found {
            for e in [es[a], es[b]] {
                if let Some((k, _)) = farthest(xy, e.0, e.1)
                    && !keep[k]
                {
                    keep[k] = true;
                    restored += 1;
                    added = true;
                }
            }
        }
        if !added {
            // Only original edges cross, which the input check rules out.
            return restored;
        }
    }
}

/// Largest geodesic distance from a removed vertex to its kept edge, and the
/// vertex, over every kept edge.
fn geodesic_worst(g: &Geodesic, inp: &Input, kept: &[usize]) -> Vec<(usize, usize, f64)> {
    edges(kept, inp.ring)
        .into_iter()
        .filter_map(|(a, b)| {
            inner(a, b, inp.ll.len())
                .map(|k| (k, seg_dist(g, inp.ll[a], inp.ll[b], inp.ll[k]).0))
                .max_by(|x, y| x.1.total_cmp(&y.1))
                .map(|(k, d)| (a, k, d))
        })
        .collect()
}

fn kept_of(keep: &[bool]) -> Vec<usize> {
    (0..keep.len()).filter(|&i| keep[i]).collect()
}

fn finish(
    ctx: &mut Ctx,
    g: &Geodesic,
    inp: &Input,
    keep: &[bool],
    restored: usize,
) -> Result<Json, ToolError> {
    let kept = kept_of(keep);
    let dev = geodesic_worst(g, inp, &kept)
        .into_iter()
        .map(|w| w.2)
        .fold(0.0, f64::max);
    if ctx.explaining() {
        let fmt = ctx.options.format;
        ctx.step(
            "Largest deviation",
            "max over removed vertices of the geodesic distance to their simplified edge",
            format!("{} removed", inp.ll.len() - kept.len()),
            display::quantity(dev, "m", Precision::Decimals(3), fmt),
        );
        ctx.step(
            "Vertices kept",
            "given − removed",
            format!("{} − {}", inp.ll.len(), inp.ll.len() - kept.len()),
            display::number(kept.len() as f64, Precision::Decimals(0), fmt),
        );
    }
    let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let m = units::by_symbol(QT::Length, "m").expect("m");
    let dq = |v: f64| {
        Q {
            value: v,
            unit: deg,
        }
        .to_json()
    };
    let rows: Vec<Json> = kept
        .iter()
        .map(|&i| Json::obj([("lat", dq(inp.ll[i].0)), ("lon", dq(inp.ll[i].1))]))
        .collect();
    Ok(Json::obj(vec![
        ("vertices_out", Json::Num(kept.len() as f64)),
        ("vertices_in", Json::Num(inp.ll.len() as f64)),
        (
            "max_deviation",
            ctx.out(
                "max_deviation",
                Q {
                    value: dev,
                    unit: m,
                },
            ),
        ),
        ("restored", Json::Num(restored as f64)),
        ("simplified", Json::Arr(rows)),
    ]))
}

// ---------------------------------------------------------------- RDP

pub static RDP: ToolDef = ToolDef {
    id: "geometry.simplify.rdp",
    title: "Simplify a line or polygon (Douglas-Peucker)",
    summary: "Removes vertices from a line or polygon while keeping every removed vertex within a distance tolerance of the simplified geodesic edges, optionally without letting edges cross, and reports the largest deviation.",
    aliases: &[
        "simplify polygon",
        "simplify line",
        "Douglas-Peucker",
        "Ramer-Douglas-Peucker",
        "reduce vertices",
    ],
    keywords: &[
        "simplify",
        "generalize",
        "Douglas-Peucker",
        "RDP",
        "vertices",
        "tolerance",
        "topology",
    ],
    inputs: &[
        POINTS,
        Field::new(
            "tolerance",
            "Tolerance",
            "How far a removed vertex may be from the new edge, like 25 m",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .required()
        .core(),
        SHAPE,
        TOPOLOGY,
    ],
    outputs: OUTPUTS,
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Ramer-Douglas-Peucker on an azimuthal equidistant plane centered on the shape, then checked on the ellipsoid: any edge with a removed vertex farther than the tolerance (by geodesic distance) is split again. A polygon is split at the vertex farthest from its first. With topology kept, crossing edges get their farthest vertex back until none cross",
    accuracy: "Every removed vertex is within the tolerance of its simplified geodesic edge, measured on the ellipsoid; the crossing check is on the plane, exact for shapes a few hundred kilometers across",
    references: &[DOUGLAS_PEUCKER],
    examples: &[Example {
        id: "primary",
        title: "A wavy 1 km track at a 25 m tolerance",
        input: r#"{"points":[{"lat":40.0,"lon":-105.0},{"lat":40.0002,"lon":-104.998},{"lat":39.9998,"lon":-104.996},{"lat":40.0003,"lon":-104.994},{"lat":40.0,"lon":-104.992},{"lat":40.0015,"lon":-104.990},{"lat":40.0,"lon":-104.988}],"tolerance":"25 m"}"#,
        source: "Douglas and Peucker (1973) with geodesic deviations (Karney 2013)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geometry.simplify.visvalingam",
            reason: "alternative",
        },
        Related {
            id: "geometry.validity.make-valid",
            reason: "parent",
        },
        Related {
            id: "geometry.area.polygon",
            reason: "next",
        },
    ],
    sentence: "Kept {vertices_out} of {vertices_in} vertices. The largest shift is {max_deviation}.",
    limits: &[("batchRows", 100)],
    run: run_rdp,
    ..ToolDef::BLANK
};

/// Plane RDP between kept vertices `a` and `b`.
fn rdp(xy: &[P], a: usize, b: usize, tol: f64, keep: &mut [bool]) {
    let mut stack = vec![(a, b)];
    while let Some((a, b)) = stack.pop() {
        if let Some((k, d)) = farthest(xy, a, b)
            && d > tol
        {
            keep[k] = true;
            stack.push((a, k));
            stack.push((k, b));
        }
    }
}

fn run_rdp(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let g = Geodesic::wgs84();
    let inp = read(ctx, &g)?;
    let tol = ctx.req_quantity("tolerance")?.base();
    if tol.is_nan() || tol <= 0.0 {
        return Err(ToolError::invalid(
            "/tolerance",
            "The tolerance must be greater than 0.",
        ));
    }
    let n = inp.xy.len();
    let mut keep = vec![false; n];
    keep[0] = true;
    if inp.ring {
        // Split the ring at the vertex farthest from its first.
        let far = (1..n)
            .max_by(|&i, &j| {
                let d = |k: usize| (inp.xy[k].0 - inp.xy[0].0).hypot(inp.xy[k].1 - inp.xy[0].1);
                d(i).total_cmp(&d(j))
            })
            .expect("three corners");
        keep[far] = true;
        rdp(&inp.xy, 0, far, tol, &mut keep);
        rdp(&inp.xy, far, 0, tol, &mut keep);
        // A ring keeps at least three corners.
        if kept_of(&keep).len() < 3
            && let Some((k, _)) = farthest(&inp.xy, 0, far)
                .into_iter()
                .chain(farthest(&inp.xy, far, 0))
                .max_by(|x, y| x.1.total_cmp(&y.1))
        {
            keep[k] = true;
        }
    } else {
        keep[n - 1] = true;
        rdp(&inp.xy, 0, n - 1, tol, &mut keep);
    }
    // The plane chose; the ellipsoid checks. Split any edge whose worst
    // removed vertex is beyond the tolerance by geodesic distance.
    let mut restored = 0;
    loop {
        let mut changed = false;
        if inp.topology {
            let r = untangle(&inp.xy, &mut keep, inp.ring);
            restored += r;
            changed |= r > 0;
        }
        for (_, k, d) in geodesic_worst(&g, &inp, &kept_of(&keep)) {
            if d > tol {
                keep[k] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    finish(ctx, &g, &inp, &keep, restored)
}

// ---------------------------------------------------------------- Visvalingam-Whyatt

pub static VISVALINGAM_WHYATT: ToolDef = ToolDef {
    id: "geometry.simplify.visvalingam",
    title: "Simplify a line or polygon (Visvalingam)",
    summary: "Removes the vertices that make the smallest triangles with their neighbors, down to an area threshold or a target number of vertices, optionally without letting edges cross, and reports the largest deviation.",
    aliases: &[
        "Visvalingam-Whyatt",
        "Visvalingam simplification",
        "effective area simplification",
        "reduce vertices to a count",
    ],
    keywords: &[
        "simplify",
        "generalize",
        "Visvalingam",
        "effective area",
        "vertices",
        "topology",
    ],
    inputs: &[
        POINTS,
        Field::new(
            "area",
            "Area threshold",
            "Remove vertices whose triangle is smaller, like 500 m2",
            Kind::Quantity {
                q: QT::Area,
                unit: "m2",
            },
        )
        .core(),
        Field::new(
            "target_vertices",
            "Target vertex count",
            "Instead of an area, like 50",
            Kind::Number {
                min: 2.0,
                max: 10_000.0,
            },
        )
        .core(),
        SHAPE,
        TOPOLOGY,
    ],
    outputs: OUTPUTS,
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Visvalingam-Whyatt on an azimuthal equidistant plane centered on the shape: repeatedly remove the vertex whose triangle with its neighbors has the smallest area (never less than the last removed), until every remaining triangle reaches the threshold or the target count is met. With topology kept, a removal that would make edges cross is skipped. The largest deviation is measured on the ellipsoid",
    accuracy: "Triangle areas are on the plane, exact to about 0.1% for shapes a few hundred kilometers across; the reported deviation is geodesic",
    references: &[VISVALINGAM],
    examples: &[Example {
        id: "primary",
        title: "The wavy track down to 4 vertices",
        input: r#"{"points":[{"lat":40.0,"lon":-105.0},{"lat":40.0002,"lon":-104.998},{"lat":39.9998,"lon":-104.996},{"lat":40.0003,"lon":-104.994},{"lat":40.0,"lon":-104.992},{"lat":40.0015,"lon":-104.990},{"lat":40.0,"lon":-104.988}],"target_vertices":4}"#,
        source: "Visvalingam and Whyatt (1993) with geodesic deviations (Karney 2013)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geometry.simplify.rdp",
            reason: "alternative",
        },
        Related {
            id: "geometry.validity.make-valid",
            reason: "parent",
        },
    ],
    sentence: "Kept {vertices_out} of {vertices_in} vertices. The largest shift is {max_deviation}.",
    limits: &[("batchRows", 100)],
    run: run_vw,
    ..ToolDef::BLANK
};

#[derive(PartialEq)]
struct Entry {
    area: f64,
    i: usize,
    stamp: u32,
}
impl Eq for Entry {}
impl Ord for Entry {
    // A min-heap on area, then index, so ties break the same way every time.
    fn cmp(&self, o: &Self) -> Ordering {
        o.area.total_cmp(&self.area).then(o.i.cmp(&self.i))
    }
}
impl PartialOrd for Entry {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

/// A uniform grid of edges by the cells their bounding boxes cover. Edges
/// that a removal replaces stay filed and are skipped when read.
struct Grid {
    lo: P,
    size: f64,
    side: usize,
    edges: Vec<Vec<(usize, usize)>>,
}

impl Grid {
    fn new(xy: &[P]) -> Grid {
        let (mut lo, mut hi) = (
            (f64::INFINITY, f64::INFINITY),
            (f64::NEG_INFINITY, f64::NEG_INFINITY),
        );
        for p in xy {
            lo = (lo.0.min(p.0), lo.1.min(p.1));
            hi = (hi.0.max(p.0), hi.1.max(p.1));
        }
        let side = (libm::sqrt(xy.len() as f64) as usize).clamp(1, 256);
        let size = ((hi.0 - lo.0).max(hi.1 - lo.1) / side as f64).max(1e-9);
        Grid {
            lo,
            size,
            side,
            edges: vec![Vec::new(); side * side],
        }
    }

    fn cells(&self, a: P, b: P) -> impl Iterator<Item = usize> + '_ {
        let cell = |v: f64, o: f64| (((v - o) / self.size) as usize).min(self.side - 1);
        let (x0, x1) = (cell(a.0.min(b.0), self.lo.0), cell(a.0.max(b.0), self.lo.0));
        let (y0, y1) = (cell(a.1.min(b.1), self.lo.1), cell(a.1.max(b.1), self.lo.1));
        (y0..=y1).flat_map(move |y| (x0..=x1).map(move |x| y * self.side + x))
    }

    fn file(&mut self, xy: &[P], j: usize, k: usize) {
        let cells: Vec<usize> = self.cells(xy[j], xy[k]).collect();
        for c in cells {
            self.edges[c].push((j, k));
        }
    }
}

fn tri(a: P, b: P, c: P) -> f64 {
    orient(a, b, c).abs() / 2.0
}

fn run_vw(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let g = Geodesic::wgs84();
    let inp = read(ctx, &g)?;
    let area = ctx.quantity("area")?.map(|q| q.base());
    let target = ctx.number("target_vertices")?;
    let n = inp.xy.len();
    let floor = if inp.ring { 3 } else { 2 };
    let (limit_area, target) = match (area, target) {
        (Some(a), None) if a > 0.0 => (a, floor),
        (Some(_), None) => {
            return Err(ToolError::invalid(
                "/area",
                "The area threshold must be greater than 0.",
            ));
        }
        (None, Some(t)) if t.fract() == 0.0 => (f64::INFINITY, (t as usize).max(floor)),
        (None, Some(_)) => {
            return Err(ToolError::invalid(
                "/target_vertices",
                "The target must be a whole number.",
            ));
        }
        (Some(_), Some(_)) => {
            return Err(ToolError::invalid(
                "/target_vertices",
                "Give an area threshold or a target count, not both.",
            ));
        }
        (None, None) => {
            return Err(ToolError::invalid(
                "/area",
                "Give an area threshold or a target vertex count.",
            )
            .hint("Example: 500 m2, or 50 vertices"));
        }
    };
    let xy = &inp.xy;
    let mut prev: Vec<usize> = (0..n).map(|i| (i + n - 1) % n).collect();
    let mut next: Vec<usize> = (0..n).map(|i| (i + 1) % n).collect();
    let mut alive = vec![true; n];
    let mut stamp = vec![0u32; n];
    let removable = |i: usize| inp.ring || (i != 0 && i != n - 1);
    let mut heap = BinaryHeap::new();
    for i in (0..n).filter(|&i| removable(i)) {
        heap.push(Entry {
            area: tri(xy[prev[i]], xy[i], xy[next[i]]),
            i,
            stamp: 0,
        });
    }
    let mut count = n;
    let mut last = 0.0f64;
    let mut blocked = vec![false; n];
    let mut grid = Grid::new(xy);
    if inp.topology {
        let edges = if inp.ring { n } else { n - 1 };
        for (j, &k) in next.iter().enumerate().take(edges) {
            grid.file(xy, j, k);
        }
    }
    while count > target {
        let Some(Entry {
            area: a,
            i,
            stamp: s,
        }) = heap.pop()
        else {
            break;
        };
        if !alive[i] || s != stamp[i] || blocked[i] {
            continue;
        }
        let effective = a.max(last);
        if effective >= limit_area {
            break;
        }
        let (p, q) = (prev[i], next[i]);
        if inp.topology {
            // The new edge p→q must not cross any other remaining edge; the
            // edges that meet p, i, or q are its neighbors, not rivals. Only
            // edges filed in the cells p→q passes over can meet it.
            let hits = grid.cells(xy[p], xy[q]).any(|c| {
                grid.edges[c].iter().any(|&(j, k)| {
                    let live = alive[j] && alive[k] && next[j] == k;
                    let near = [p, i, q].contains(&j) || [p, i, q].contains(&k);
                    live && !near && cross(xy[p], xy[q], xy[j], xy[k])
                })
            });
            if hits {
                blocked[i] = true;
                continue;
            }
        }
        last = effective;
        alive[i] = false;
        count -= 1;
        next[p] = q;
        prev[q] = p;
        if inp.topology {
            grid.file(xy, p, q);
        }
        for v in [p, q] {
            if removable(v) && alive[v] {
                stamp[v] += 1;
                blocked[v] = false;
                heap.push(Entry {
                    area: tri(xy[prev[v]], xy[v], xy[next[v]]),
                    i: v,
                    stamp: stamp[v],
                });
            }
        }
    }
    finish(ctx, &g, &inp, &alive, 0)
}
