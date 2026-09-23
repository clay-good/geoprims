//! Delaunay triangulation and Voronoi diagrams (add-navigation-and-geometry,
//! geometry/computational, "Densification, triangulation, Voronoi, and
//! distances"), planar and spherical, from one 3D convex hull:
//!
//! - spherical: the hull of the points' directions on the unit sphere is their
//!   Delaunay triangulation on the sphere;
//! - planar: on an azimuthal equidistant plane centered on the points, the
//!   lower hull of the points lifted to the paraboloid z = x² + y² is their
//!   planar Delaunay triangulation.
//!
//! Ties such as four points on one circle have more than one valid answer; a
//! fixed jitter far below a millimeter picks one the same way every time.
//! Voronoi cells come from the Delaunay neighbors: on the plane, the box
//! around the points cut by the perpendicular bisector with each neighbor; on
//! the sphere, the circumcenters of the triangles around the point.

use std::collections::HashMap;

use geographiclib_rs::Geodesic;
use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::buffer::{Aeqd, REACH, center};
use libm::{atan2, cos, hypot, sin, sqrt};

type V3 = [f64; 3];

const DELAUNAY_REF: Reference = Reference {
    title: "Computational Geometry: Algorithms and Applications",
    issuer: "de Berg, M., Cheong, O., van Kreveld, M., and Overmars, M., Springer",
    year: 2008,
    edition: "3rd edition",
    locator: "Chapter 9 (Delaunay triangulations) and 11.5 (the lifting map and convex hulls)",
    url: "https://doi.org/10.1007/978-3-540-77974-2",
};

const POINT: &[Field] = &[
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

const MAX_POINTS: usize = 5_000;

const POINTS: Field = Field::new(
    "points",
    "Points",
    "At least 3, like 40.0, -105.0 on each row",
    Kind::List {
        items: POINT,
        min: 3,
        max: MAX_POINTS,
    },
)
.required()
.core();
const SURFACE: Field = Field::new(
    "surface",
    "Surface",
    "auto (default: planar within 1,000 km of the points' center, else spherical), planar, or spherical",
    Kind::Choice(&["auto", "planar", "spherical"]),
)
.core();

fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(a: V3) -> f64 {
    sqrt(dot(a, a))
}

/// A fixed, input-order jitter in [-1, 1), from a hash of the index.
fn jitter(i: usize, k: u64) -> f64 {
    let mut x =
        (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ k.wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    x ^= x >> 31;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 29;
    (x >> 11) as f64 / (1u64 << 52) as f64 - 1.0
}

/// Whether every point lies on one plane (to 1e-12 of the spread), so there
/// is no hull; checked before any jitter.
fn flat(p: &[V3]) -> bool {
    let n = p.len();
    let c = p
        .iter()
        .fold([0.0; 3], |a, v| [a[0] + v[0], a[1] + v[1], a[2] + v[2]]);
    let c = [c[0] / n as f64, c[1] / n as f64, c[2] / n as f64];
    let q: Vec<V3> = p.iter().map(|&v| sub(v, c)).collect();
    let scale = q.iter().map(|&v| norm(v)).fold(0.0, f64::max);
    if scale == 0.0 {
        return true;
    }
    let far = |from: &dyn Fn(usize) -> f64| {
        (0..n)
            .max_by(|&a, &b| from(a).total_cmp(&from(b)))
            .unwrap_or(0)
    };
    let i1 = far(&|k| norm(sub(q[k], q[0])));
    let i2 = far(&|k| norm(cross(sub(q[k], q[0]), sub(q[i1], q[0]))));
    let nrm = cross(sub(q[i1], q[0]), sub(q[i2], q[0]));
    let l = norm(nrm);
    l <= 1e-12 * scale * scale
        || q.iter()
            .all(|&v| (dot(nrm, sub(v, q[0])) / l).abs() <= 1e-12 * scale)
}

/// Triangles (counterclockwise seen from outside) of the convex hull of `p`,
/// by incremental insertion; None when every point is on one plane.
fn hull(p: &[V3]) -> Option<Vec<[usize; 3]>> {
    let n = p.len();
    let vol = |a: usize, b: usize, c: usize, d: usize| {
        dot(cross(sub(p[b], p[a]), sub(p[c], p[a])), sub(p[d], p[a]))
    };
    // A starting tetrahedron: the farthest pair, the point farthest from their
    // line, and the point farthest from their plane.
    let i0 = 0;
    let i1 = (1..n).max_by(|&a, &b| norm(sub(p[a], p[i0])).total_cmp(&norm(sub(p[b], p[i0]))))?;
    let i2 = (0..n).max_by(|&a, &b| {
        let d = |k: usize| norm(cross(sub(p[k], p[i0]), sub(p[i1], p[i0])));
        d(a).total_cmp(&d(b))
    })?;
    let i3 = (0..n).max_by(|&a, &b| {
        vol(i0, i1, i2, a)
            .abs()
            .total_cmp(&vol(i0, i1, i2, b).abs())
    })?;
    let scale = (0..n).map(|k| norm(p[k])).fold(0.0, f64::max).max(1e-300);
    if vol(i0, i1, i2, i3).abs() <= 1e-18 * scale * scale * scale {
        return None;
    }
    let mut faces: Vec<[usize; 3]> = Vec::new();
    let mut alive: Vec<bool> = Vec::new();
    let mut edge: HashMap<(usize, usize), usize> = HashMap::new();
    let add = |faces: &mut Vec<[usize; 3]>,
               alive: &mut Vec<bool>,
               edge: &mut HashMap<(usize, usize), usize>,
               f: [usize; 3]| {
        let id = faces.len();
        for k in 0..3 {
            edge.insert((f[k], f[(k + 1) % 3]), id);
        }
        faces.push(f);
        alive.push(true);
    };
    let base = if vol(i0, i1, i2, i3) < 0.0 {
        [i0, i1, i2]
    } else {
        [i0, i2, i1]
    };
    add(&mut faces, &mut alive, &mut edge, base);
    add(&mut faces, &mut alive, &mut edge, [base[0], base[2], i3]);
    add(&mut faces, &mut alive, &mut edge, [base[2], base[1], i3]);
    add(&mut faces, &mut alive, &mut edge, [base[1], base[0], i3]);
    let eps = 1e-14 * scale * scale * scale;
    for q in 0..n {
        if [i0, i1, i2, i3].contains(&q) {
            continue;
        }
        let visible: Vec<usize> = (0..faces.len())
            .filter(|&f| alive[f] && vol(faces[f][0], faces[f][1], faces[f][2], q) > eps)
            .collect();
        if visible.is_empty() {
            continue;
        }
        // The horizon: edges of visible faces whose twin face is not visible.
        let mut horizon = Vec::new();
        for &f in &visible {
            for k in 0..3 {
                let (a, b) = (faces[f][k], faces[f][(k + 1) % 3]);
                let twin = edge.get(&(b, a)).copied();
                if twin.is_none_or(|t| !visible.contains(&t)) {
                    horizon.push((a, b));
                }
            }
        }
        for &f in &visible {
            alive[f] = false;
        }
        for (a, b) in horizon {
            add(&mut faces, &mut alive, &mut edge, [a, b, q]);
        }
    }
    Some(
        faces
            .into_iter()
            .zip(alive)
            .filter_map(|(f, a)| a.then_some(f))
            .collect(),
    )
}

struct Mesh {
    ll: Vec<(f64, f64)>,
    spherical: bool,
    /// Plane coordinates (planar) or unit vectors (spherical).
    xy: Vec<(f64, f64)>,
    unit: Vec<V3>,
    tris: Vec<[usize; 3]>,
    map: Option<(f64, f64)>,
}

fn build(ctx: &mut Ctx) -> Result<Mesh, ToolError> {
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
        if let Some(j) = ll.iter().position(|&p| p == (lat, lon)) {
            return Err(ToolError::invalid(
                &format!("/points/{i}"),
                format!(
                    "Point {} repeats point {}; give each point once.",
                    i + 1,
                    j + 1
                ),
            ));
        }
        ll.push((lat, lon));
    }
    let g = Geodesic::wgs84();
    let (lat0, lon0) = center(&ll);
    let map = Aeqd { g: &g, lat0, lon0 };
    let xy: Vec<(f64, f64)> = ll.iter().map(|&p| map.fwd(p)).collect();
    let far = xy.iter().map(|p| hypot(p.0, p.1)).fold(0.0, f64::max);
    let spherical = match ctx.choice("surface")?.unwrap_or("auto") {
        "planar" => {
            if far > REACH {
                return Err(ToolError::new(
                    ErrorCode::OutOfDomain,
                    "Planar triangulation works within 1,000 km of the points' center; use spherical for wider sets.",
                )
                .at("/surface"));
            }
            false
        }
        "spherical" => true,
        _ => far > REACH,
    };
    let unit: Vec<V3> = ll
        .iter()
        .map(|&(la, lo)| {
            let (p, l) = (la.to_radians(), lo.to_radians());
            [cos(p) * cos(l), cos(p) * sin(l), sin(p)]
        })
        .collect();
    let degenerate = || {
        ToolError::new(
            ErrorCode::DegenerateGeometry,
            "The points all lie on one line (or one great circle), so they make no triangles.",
        )
        .at("/points")
    };
    let tris = if spherical {
        if flat(&unit) {
            return Err(degenerate());
        }
        let pts: Vec<V3> = unit
            .iter()
            .enumerate()
            .map(|(i, u)| {
                [
                    u[0] + 1e-12 * jitter(i, 1),
                    u[1] + 1e-12 * jitter(i, 2),
                    u[2] + 1e-12 * jitter(i, 3),
                ]
            })
            .collect();
        hull(&pts)
    } else {
        // Lift to the paraboloid in units of the spread, and keep the lower hull.
        let s = far.max(1.0);
        let lifted: Vec<V3> = xy
            .iter()
            .map(|&(x, y)| [x / s, y / s, (x * x + y * y) / (s * s)])
            .collect();
        if flat(&lifted) {
            return Err(degenerate());
        }
        let pts: Vec<V3> = xy
            .iter()
            .enumerate()
            .map(|(i, &(x, y))| {
                let (u, v) = (x / s + 1e-12 * jitter(i, 1), y / s + 1e-12 * jitter(i, 2));
                [u, v, u * u + v * v]
            })
            .collect();
        hull(&pts).map(|t| {
            t.into_iter()
                .filter(|f| {
                    let nz = cross(sub(pts[f[1]], pts[f[0]]), sub(pts[f[2]], pts[f[0]]))[2];
                    nz < 0.0
                })
                .map(|f| [f[0], f[2], f[1]])
                .collect()
        })
    }
    .ok_or_else(degenerate)?;
    Ok(Mesh {
        ll,
        spherical,
        xy,
        unit,
        tris,
        map: Some((lat0, lon0)),
    })
}

// ---------------------------------------------------------------- Delaunay

pub static DELAUNAY: ToolDef = ToolDef {
    stability: gp_base::tool::Stability::Stable,
    id: "geometry.mesh.delaunay",
    title: "Delaunay triangulation of points",
    summary: "Triangles joining a set of points so no point sits inside any triangle's circumcircle: planar on a local map for a regional set, or on the sphere for a global one.",
    aliases: &[
        "Delaunay triangulation",
        "triangulate points",
        "TIN from points",
        "triangulated irregular network",
    ],
    keywords: &[
        "Delaunay",
        "triangulation",
        "TIN",
        "mesh",
        "triangles",
        "points",
    ],
    inputs: &[POINTS, SURFACE],
    outputs: &[
        Field::new(
            "triangle_count",
            "Triangles",
            "How many",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "surface_used",
            "Surface",
            "planar or spherical",
            Kind::Text { max_len: 12 },
        ),
        Field::new(
            "triangles",
            "Triangles",
            "Each as three point numbers, counting the first point as 1, counterclockwise from above",
            Kind::List {
                items: &[
                    Field::new(
                        "a",
                        "First",
                        "Point number",
                        Kind::Number { min: 1.0, max: 1e6 },
                    )
                    .precision(Precision::Decimals(0)),
                    Field::new(
                        "b",
                        "Second",
                        "Point number",
                        Kind::Number { min: 1.0, max: 1e6 },
                    )
                    .precision(Precision::Decimals(0)),
                    Field::new(
                        "c",
                        "Third",
                        "Point number",
                        Kind::Number { min: 1.0, max: 1e6 },
                    )
                    .precision(Precision::Decimals(0)),
                ],
                min: 0,
                max: 2 * MAX_POINTS,
            },
        ),
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::DegenerateGeometry,
    ],
    warnings: &["UNIT_ASSUMED"],
    model: "Planar: the lower convex hull of the points lifted to z = x² + y² on an azimuthal equidistant plane at their center. Spherical: the convex hull of their directions on the unit sphere. Both by incremental insertion, with a fixed jitter far below a millimeter to settle ties",
    accuracy: "Exact triangulation of the points on the chosen surface; where four points share a circle, either diagonal is valid and one is chosen consistently",
    when_to_use: "Use this to turn scattered points into a surface. A triangulation is what interpolation between measurements runs on — a terrain model from spot heights, a field from soundings or samples, contours from a survey — and it is also the skeleton the Voronoi cells are built from, so the two answer opposite halves of the same question. Take the spherical surface when the points spread far enough that a plane would distort them; planar is right for a site, a field, or a survey block.",
    limitations: "It triangulates the convex hull of the points and nothing beyond it, so a concave boundary is filled in: an L-shaped site comes back with triangles spanning the notch, and they have to be clipped afterwards. Where four or more points share a circle the triangulation is genuinely not unique and either diagonal is correct; one is chosen consistently, but two implementations can differ there and neither is wrong. The result is a mesh over the points given and interpolating on it is only as good as they are — it says nothing about what happens between them beyond a straight line.",
    references: &[DELAUNAY_REF],
    examples: &[Example {
        id: "primary",
        title: "Five survey points",
        input: r#"{"points":[{"lat":40.0,"lon":-105.0},{"lat":40.01,"lon":-104.99},{"lat":40.0,"lon":-104.98},{"lat":39.99,"lon":-104.992},{"lat":40.004,"lon":-104.995}]}"#,
        source: "de Berg et al. (2008), the lifting map. Checked against GEOS 3.11.4's own delaunay_triangles over fifteen point sets — scatters, jittered grids, rings, two clusters, and sets at the equator, at 70 north and across the antimeridian — agreeing triangle for triangle on every one, which a unique triangulation permits demanding",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geometry.mesh.voronoi",
            reason: "next",
        },
        Related {
            id: "geometry.shape.enclosing",
            reason: "alternative",
        },
        Related {
            id: "geometry.area.polygon",
            reason: "next",
        },
    ],
    sentence: "The points make {triangle_count} triangles on the {surface_used} surface.",
    limits: &[("batchRows", 100)],
    run: run_delaunay,
    ..ToolDef::BLANK
};

fn run_delaunay(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let m = build(ctx)?;
    if ctx.explaining() {
        let fmt = ctx.options.format;
        ctx.step(
            "Surface",
            "planar within 1,000 km of the center, else the sphere",
            format!("{} points", m.ll.len()),
            if m.spherical { "spherical" } else { "planar" }.to_owned(),
        );
        ctx.step(
            "Triangles",
            "faces of the convex hull (the lower hull of the lifted points on the plane)",
            format!("{} points", m.ll.len()),
            display::number(m.tris.len() as f64, Precision::Decimals(0), fmt),
        );
    }
    let rows: Vec<Json> = m
        .tris
        .iter()
        .map(|t| {
            Json::obj([
                ("a", Json::Num((t[0] + 1) as f64)),
                ("b", Json::Num((t[1] + 1) as f64)),
                ("c", Json::Num((t[2] + 1) as f64)),
            ])
        })
        .collect();
    Ok(Json::obj(vec![
        ("triangle_count", Json::Num(m.tris.len() as f64)),
        (
            "surface_used",
            Json::str(if m.spherical { "spherical" } else { "planar" }),
        ),
        ("triangles", Json::Arr(rows)),
    ]))
}

// ---------------------------------------------------------------- Voronoi

pub static VORONOI: ToolDef = ToolDef {
    stability: gp_base::tool::Stability::Stable,
    id: "geometry.mesh.voronoi",
    title: "Voronoi cells around points",
    summary: "The region nearest each point (its Voronoi cell), planar on a local map within a box around the points, or on the whole sphere for a global set.",
    aliases: &[
        "Voronoi diagram",
        "Thiessen polygons",
        "nearest point regions",
        "service areas from points",
    ],
    keywords: &[
        "Voronoi", "Thiessen", "cells", "nearest", "regions", "points",
    ],
    inputs: &[POINTS, SURFACE],
    outputs: &[
        Field::new(
            "cell_count",
            "Cells",
            "One per point",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "surface_used",
            "Surface",
            "planar or spherical",
            Kind::Text { max_len: 12 },
        ),
        Field::new(
            "cells",
            "Cell corners",
            "Each cell's corners in order, tagged with its point number (the first point is 1)",
            Kind::List {
                items: &[
                    Field::new(
                        "point",
                        "Point",
                        "Whose cell",
                        Kind::Number { min: 1.0, max: 1e6 },
                    )
                    .precision(Precision::Decimals(0)),
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
                ],
                min: 0,
                max: 20 * MAX_POINTS,
            },
        ),
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::DegenerateGeometry,
    ],
    warnings: &["UNIT_ASSUMED"],
    model: "From the Delaunay triangulation. Planar: each cell is the box around the points (padded by a tenth of their spread) cut by the perpendicular bisector with each Delaunay neighbor, on an azimuthal equidistant plane. Spherical: the circumcenters of the triangles around the point, in order",
    accuracy: "Exact on the chosen surface; planar cells at the edge of the set end at the box",
    when_to_use: "Use this to divide ground between points: the catchment of each depot, station, transmitter or sensor, the area each is nearest to, the territory a set of sites carves up between them. Every place inside a cell is closer to that cell's point than to any other, which is what makes it the right answer when the question is which one serves a given place. It is the Delaunay triangulation seen the other way round, so the two come from the same construction.",
    limitations: "The outer cells are unbounded — nothing stops the territory of an edge point running to the horizon — so on the planar surface they are cut at a box around the points, padded by a tenth of their spread. That box is a presentation choice and not geometry: the outer cells' shapes and areas depend on it, and only the interior cells are determined by the points alone. On the spherical surface no box is needed, since every cell closes. Cells are computed on the chosen surface and a point exactly equidistant from three or more generators sits on a shared corner, so a cell boundary belongs to no single cell.",
    references: &[DELAUNAY_REF],
    examples: &[Example {
        id: "primary",
        title: "Five survey points",
        input: r#"{"points":[{"lat":40.0,"lon":-105.0},{"lat":40.01,"lon":-104.99},{"lat":40.0,"lon":-104.98},{"lat":39.99,"lon":-104.992},{"lat":40.004,"lon":-104.995}]}"#,
        source: "The Delaunay dual; checked against Qhull via SciPy, and against GEOS 3.11.4's voronoi_polygons over fifteen point sets in the same clip box, agreeing on every cell's corner count and on its area to 4.4e-7 relative",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geometry.mesh.delaunay",
            reason: "parent",
        },
        Related {
            id: "geometry.area.polygon",
            reason: "next",
        },
        Related {
            id: "geometry.predicate.point-in-polygon",
            reason: "next",
        },
    ],
    sentence: "Each of the {cell_count} points has a cell on the {surface_used} surface.",
    limits: &[("batchRows", 100)],
    run: run_voronoi,
    ..ToolDef::BLANK
};

/// Keeps the part of `poly` on p's side of the bisector of p and q.
fn clip(poly: &[(f64, f64)], p: (f64, f64), q: (f64, f64)) -> Vec<(f64, f64)> {
    let (nx, ny) = (q.0 - p.0, q.1 - p.1);
    let c = (q.0 * q.0 + q.1 * q.1 - p.0 * p.0 - p.1 * p.1) / 2.0;
    let side = |v: (f64, f64)| nx * v.0 + ny * v.1 - c; // ≤ 0 on p's side
    let mut out = Vec::new();
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        let (sa, sb) = (side(a), side(b));
        if sa <= 0.0 {
            out.push(a);
        }
        if (sa < 0.0 && sb > 0.0) || (sa > 0.0 && sb < 0.0) {
            let t = sa / (sa - sb);
            out.push((a.0 + t * (b.0 - a.0), a.1 + t * (b.1 - a.1)));
        }
    }
    out
}

fn run_voronoi(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let m = build(ctx)?;
    let n = m.ll.len();
    let mut around: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (t, f) in m.tris.iter().enumerate() {
        for &v in f {
            around[v].push(t);
        }
    }
    let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let dq = |v: f64| {
        Q {
            value: v,
            unit: deg,
        }
        .to_json()
    };
    let mut rows = Vec::new();
    if m.spherical {
        let circ: Vec<V3> = m
            .tris
            .iter()
            .map(|f| {
                let c = cross(
                    sub(m.unit[f[1]], m.unit[f[0]]),
                    sub(m.unit[f[2]], m.unit[f[0]]),
                );
                let l = norm(c);
                [c[0] / l, c[1] / l, c[2] / l]
            })
            .collect();
        for (v, ts) in around.iter().enumerate() {
            // Order the circumcenters by angle around the point.
            let u = m.unit[v];
            let e = {
                let t = if u[2].abs() < 0.9 {
                    [0.0, 0.0, 1.0]
                } else {
                    [1.0, 0.0, 0.0]
                };
                let c = cross(t, u);
                let l = norm(c);
                [c[0] / l, c[1] / l, c[2] / l]
            };
            let nrt = cross(u, e);
            let mut cs: Vec<(f64, V3)> = ts
                .iter()
                .map(|&t| (atan2(dot(circ[t], nrt), dot(circ[t], e)), circ[t]))
                .collect();
            cs.sort_by(|a, b| a.0.total_cmp(&b.0));
            for (_, c) in cs {
                rows.push(Json::obj([
                    ("point", Json::Num((v + 1) as f64)),
                    ("lat", dq(atan2(c[2], hypot(c[0], c[1])).to_degrees())),
                    ("lon", dq(atan2(c[1], c[0]).to_degrees())),
                ]));
            }
        }
    } else {
        let (lat0, lon0) = m.map.expect("planar");
        let g = Geodesic::wgs84();
        let map = Aeqd { g: &g, lat0, lon0 };
        let (mut lo, mut hi) = (
            (f64::INFINITY, f64::INFINITY),
            (f64::NEG_INFINITY, f64::NEG_INFINITY),
        );
        for p in &m.xy {
            lo = (lo.0.min(p.0), lo.1.min(p.1));
            hi = (hi.0.max(p.0), hi.1.max(p.1));
        }
        let pad = 0.1 * (hi.0 - lo.0).max(hi.1 - lo.1).max(1.0);
        let bx = vec![
            (lo.0 - pad, lo.1 - pad),
            (hi.0 + pad, lo.1 - pad),
            (hi.0 + pad, hi.1 + pad),
            (lo.0 - pad, hi.1 + pad),
        ];
        for (v, ts) in around.iter().enumerate() {
            let mut cell = bx.clone();
            let mut nb: Vec<usize> = ts
                .iter()
                .flat_map(|&t| m.tris[t])
                .filter(|&w| w != v)
                .collect();
            nb.sort_unstable();
            nb.dedup();
            for w in nb {
                cell = clip(&cell, m.xy[v], m.xy[w]);
            }
            for c in cell {
                let (la, lo) = map.rev(c);
                rows.push(Json::obj([
                    ("point", Json::Num((v + 1) as f64)),
                    ("lat", dq(la)),
                    ("lon", dq(lo)),
                ]));
            }
        }
    }
    if ctx.explaining() {
        let fmt = ctx.options.format;
        ctx.step(
            "Delaunay triangles",
            "the convex hull on the chosen surface",
            format!("{} points", n),
            display::number(m.tris.len() as f64, Precision::Decimals(0), fmt),
        );
        ctx.step(
            "Cells",
            "one per point, from its Delaunay neighbors",
            format!(
                "{} {} surface",
                n,
                if m.spherical {
                    "points on the spherical"
                } else {
                    "points on the planar"
                }
            ),
            display::number(n as f64, Precision::Decimals(0), fmt),
        );
    }
    Ok(Json::obj(vec![
        ("cell_count", Json::Num(n as f64)),
        (
            "surface_used",
            Json::str(if m.spherical { "spherical" } else { "planar" }),
        ),
        ("cells", Json::Arr(rows)),
    ]))
}
