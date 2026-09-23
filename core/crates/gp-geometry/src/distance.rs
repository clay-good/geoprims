//! Distances between tracks (add-navigation-and-geometry,
//! geometry/computational, "Densification, triangulation, Voronoi, and
//! distances"): the discrete Fréchet distance with the index pair where it
//! happens, the Hausdorff distance, and the closest approach, all geodesic.

use geographiclib_rs::{Geodesic, InverseGeodesic};
use gp_base::ErrorCode;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::buffer::seg_dist;

use crate::validity::to_plane;

const EITER: Reference = Reference {
    title: "Computing Discrete Fréchet Distance",
    issuer: "Eiter, T., and Mannila, H., Technische Universität Wien",
    year: 1994,
    edition: "Technical Report CD-TR 94/64",
    locator: "The recurrence c(i, j) = max(min(c(i-1, j), c(i-1, j-1), c(i, j-1)), d(a_i, b_j))",
    url: "https://www.kr.tuwien.ac.at/staff/eiter/et-archive/files/cdtr9464.pdf",
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
const fn meters(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(Precision::Decimals(3))
}
const fn index(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(name, title, help, Kind::Number { min: 1.0, max: 1e6 })
        .precision(Precision::Decimals(0))
}

/// The most point pairs one comparison looks at.
const MAX_PAIRS: usize = 250_000;

pub static TRACKS: ToolDef = ToolDef {
    stability: gp_base::tool::Stability::Stable,
    id: "geometry.distance.tracks",
    title: "Compare two tracks",
    summary: "How far apart two GPS tracks or routes are: the discrete Fréchet distance (the leash two walkers need, with where it is tightest), the Hausdorff distance, and their closest approach.",
    aliases: &[
        "Fréchet distance",
        "Hausdorff distance",
        "track similarity",
        "compare GPS tracks",
        "route deviation",
    ],
    keywords: &[
        "Fréchet",
        "Frechet",
        "Hausdorff",
        "track",
        "similarity",
        "deviation",
        "GPS",
        "compare",
        "distance between lines",
    ],
    inputs: &[
        Field::new(
            "track_a",
            "First track",
            "Points in order, one per line, like 40.4406, -80.002",
            Kind::List {
                items: POINT,
                min: 2,
                max: 5_000,
            },
        )
        .required()
        .core(),
        Field::new(
            "track_b",
            "Second track",
            "Points in order, one per line, like 40.4407, -80.0019",
            Kind::List {
                items: POINT,
                min: 2,
                max: 5_000,
            },
        )
        .required()
        .core(),
    ],
    outputs: &[
        meters(
            "frechet",
            "Fréchet distance",
            "The shortest leash that lets two walkers cover both tracks in order",
        ),
        index("frechet_a", "Tightest at first-track point", "From 1"),
        index("frechet_b", "and second-track point", "From 1"),
        meters(
            "hausdorff",
            "Hausdorff distance",
            "The farthest any point of one track is from the other track",
        ),
        meters("closest", "Closest approach", "0 where the tracks cross"),
    ],
    errors: &[ErrorCode::LimitExceeded, ErrorCode::OutOfDomain],
    warnings: &[],
    model: "Discrete Fréchet by the Eiter-Mannila recurrence over geodesic point distances (Karney's inverse), with the tightest pair found by walking the best coupling back; Hausdorff as the larger of the two directed maxima of each point's geodesic distance to the other track's segments; closest approach as the least point-to-segment distance, or 0 where segments cross on an azimuthal equidistant plane",
    accuracy: "Distances exact to the geodesic (under 1 µm); Fréchet is the discrete measure over the given points, so sample both tracks alike",
    when_to_use: "Use this to ask how alike two paths are: a flown track against the route that was filed, a survey line against the one that was planned, two GPS traces of the same journey, a vehicle's path against a corridor. Three numbers come back because they answer different questions. The Fréchet distance is the leash two walkers need if neither may go backwards, so it notices when two paths cover the same ground in a different order or at a different pace. The Hausdorff distance ignores order and asks only how far any point of one is from the other path. The closest approach is how near they ever come, which is zero if they cross.",
    limitations: "The Fréchet distance here is the discrete one, taken over the points as given, so it depends on how the two tracks are sampled: a coarse track compared against a fine one will show a Fréchet distance of about the coarse track's own step, which is a fact about the sampling and not about the paths. Sample both alike, or densify first. The Hausdorff distance says nothing about direction or order — two tracks over the same ground in opposite directions have a Hausdorff distance of zero and a Fréchet distance of the whole track length, which is correct and is the clearest illustration of what each measure is for. Neither is a similarity score: they are worst-case distances, so one stray point moves them both.",
    references: &[EITER],
    examples: &[Example {
        id: "primary",
        title: "A planned line and the flown track",
        input: r#"{"track_a":[{"lat":40.0,"lon":-105.0},{"lat":40.001,"lon":-104.999},{"lat":40.002,"lon":-104.998},{"lat":40.003,"lon":-104.997}],"track_b":[{"lat":40.0,"lon":-104.9999},{"lat":40.00105,"lon":-104.9989},{"lat":40.0021,"lon":-104.998},{"lat":40.0029,"lon":-104.9971}]}"#,
        source: "checked against GEOS 3.11.4 through shapely on PROJ's azimuthal equidistant plane — GEOS's own Fréchet implementation, and its point-to-segment distance for the Hausdorff — over fourteen pairs of tracks, agreeing to 6.3e-9 relative at worst",
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
            id: "geometry.buffer.geodesic",
            reason: "alternative",
        },
        Related {
            id: "geometry.shape.densify",
            reason: "parent",
        },
    ],
    sentence: "The tracks are {frechet} apart by Fréchet distance, tightest at points {frechet_a} and {frechet_b}, and {hausdorff} by Hausdorff distance.",
    limits: &[("batchRows", 100)],
    run: run_tracks,
    ..ToolDef::BLANK
};

fn read(ctx: &mut Ctx, list: &str) -> Result<Vec<(f64, f64)>, ToolError> {
    let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let rows = ctx.rows(list)?;
    let mut out = Vec::with_capacity(rows.len());
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
        out.push((lat, (lon + 540.0).rem_euclid(360.0) - 180.0));
    }
    Ok(out)
}

/// The largest distance from a point of `from` to the polyline `to`.
fn directed(g: &Geodesic, from: &[(f64, f64)], to: &[(f64, f64)]) -> f64 {
    from.iter()
        .map(|&p| {
            to.windows(2)
                .map(|w| seg_dist(g, w[0], w[1], p).0)
                .fold(f64::INFINITY, f64::min)
        })
        .fold(0.0, f64::max)
}

fn run_tracks(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let a = read(ctx, "track_a")?;
    let b = read(ctx, "track_b")?;
    let (n, m) = (a.len(), b.len());
    if n * m > MAX_PAIRS {
        return Err(ToolError::new(
            ErrorCode::LimitExceeded,
            format!("That is {} point pairs, over the {MAX_PAIRS} limit: thin the tracks (every second or fifth point) and compare again.", n * m),
        )
        .at("/track_b"));
    }
    let g = Geodesic::wgs84();
    let d: Vec<f64> = (0..n * m)
        .map(|k| {
            let (p, q) = (a[k / m], b[k % m]);
            let s: f64 = g.inverse(p.0, p.1, q.0, q.1);
            s
        })
        .collect();
    // Eiter-Mannila: c(i, j) = max(min of the three predecessors, d(i, j)).
    let mut c = vec![0.0f64; n * m];
    for i in 0..n {
        for j in 0..m {
            let here = d[i * m + j];
            let prev = match (i, j) {
                (0, 0) => 0.0,
                (0, _) => c[j - 1],
                (_, 0) => c[(i - 1) * m],
                _ => c[(i - 1) * m + j]
                    .min(c[(i - 1) * m + j - 1])
                    .min(c[i * m + j - 1]),
            };
            c[i * m + j] = prev.max(here);
        }
    }
    let frechet = c[n * m - 1];
    // Walk the best coupling back to the pair that sets the distance.
    let (mut i, mut j) = (n - 1, m - 1);
    let mut tight = (i, j);
    loop {
        if d[i * m + j] == frechet {
            tight = (i, j);
            break;
        }
        if i == 0 && j == 0 {
            break;
        }
        let steps = [
            (i.wrapping_sub(1), j.wrapping_sub(1)),
            (i.wrapping_sub(1), j),
            (i, j.wrapping_sub(1)),
        ];
        let (bi, bj) = steps
            .iter()
            .filter(|&&(x, y)| x < n && y < m)
            .min_by(|&&(x1, y1), &&(x2, y2)| c[x1 * m + y1].total_cmp(&c[x2 * m + y2]))
            .copied()
            .expect("a predecessor");
        (i, j) = (bi, bj);
    }
    let hausdorff = directed(&g, &a, &b).max(directed(&g, &b, &a));
    // Closest approach: 0 where the tracks cross, else the nearest point to a segment.
    let (ra, rb) = (a.clone(), b.clone());
    let pl = to_plane(&g, &[&ra, &rb])?;
    let cross = {
        let (pa, pb) = (&pl.plane[0], &pl.plane[1]);
        let mut hit = false;
        'outer: for wa in pa.windows(2) {
            for wb in pb.windows(2) {
                let (p, r) = (wa[0], (wa[1].0 - wa[0].0, wa[1].1 - wa[0].1));
                let (q, s) = (wb[0], (wb[1].0 - wb[0].0, wb[1].1 - wb[0].1));
                let den = r.0 * s.1 - r.1 * s.0;
                if den.abs() < 1e-18 {
                    continue;
                }
                let qp = (q.0 - p.0, q.1 - p.1);
                let t = (qp.0 * s.1 - qp.1 * s.0) / den;
                let u = (qp.0 * r.1 - qp.1 * r.0) / den;
                if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
                    hit = true;
                    break 'outer;
                }
            }
        }
        hit
    };
    let closest = if cross {
        0.0
    } else {
        let near = |from: &[(f64, f64)], to: &[(f64, f64)]| {
            from.iter()
                .map(|&p| {
                    to.windows(2)
                        .map(|w| seg_dist(&g, w[0], w[1], p).0)
                        .fold(f64::INFINITY, f64::min)
                })
                .fold(f64::INFINITY, f64::min)
        };
        near(&a, &b).min(near(&b, &a))
    };
    let m_ = units::by_symbol(QT::Length, "m").expect("m");
    let q = |v: f64| Q { value: v, unit: m_ };
    Ok(Json::obj([
        ("frechet", ctx.out("frechet", q(frechet))),
        ("frechet_a", Json::Num((tight.0 + 1) as f64)),
        ("frechet_b", Json::Num((tight.1 + 1) as f64)),
        ("hausdorff", ctx.out("hausdorff", q(hausdorff))),
        ("closest", ctx.out("closest", q(closest))),
    ]))
}
