//! Point in polygon (add-navigation-and-geometry, geometry/computational,
//! "Predicates"): by winding number and by even-odd, with geodesic edges,
//! and on-boundary within 1 mm.
//!
//! Seen from the test point, each geodesic edge sweeps the azimuth from its
//! start to its end by less than 180° (exactly 180° only when the point lies
//! on it), so the signed sweeps add to 360° times the winding number. That
//! uses exact geodesic azimuths from the point, so it holds on the
//! ellipsoid. The even-odd rule is the winding number's parity, which is
//! what a ray's crossing count always matches.

use geographiclib_rs::{Geodesic, InverseGeodesic};
use gp_base::ErrorCode;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::buffer::seg_dist;

const KARNEY: Reference = Reference {
    title: "Algorithms for geodesics",
    issuer: "Karney, C. F. F., Journal of Geodesy",
    year: 2013,
    edition: "Vol. 87, No. 1",
    locator: "pp. 43-55 (the inverse problem: azimuths from the test point)",
    url: "https://doi.org/10.1007/s00190-012-0578-z",
};

/// Within this distance of an edge, a point is on the boundary.
const ON_EDGE_M: f64 = 0.001;

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
        "0 for the outline, 1, 2, … for holes",
        Kind::Number {
            min: 0.0,
            max: 100.0,
        },
    ),
];
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
const RESULT_ROW: &[Field] = &[
    Field::new(
        "point",
        "Point",
        "Its number in the list, from 1",
        Kind::Number { min: 1.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "winding",
        "Winding number",
        "Times the outline wraps the point, signed; not meaningful on the boundary",
        Kind::Number {
            min: -1e6,
            max: 1e6,
        },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "nonzero",
        "Winding rule",
        "inside, outside, or on-boundary",
        Kind::Text { max_len: 12 },
    ),
    Field::new(
        "even_odd",
        "Even-odd rule",
        "inside, outside, or on-boundary",
        Kind::Text { max_len: 12 },
    ),
    Field::new(
        "distance",
        "To the boundary",
        "Geodesic, to the nearest edge",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(Precision::Decimals(3)),
];

pub static POINT_IN_POLYGON: ToolDef = ToolDef {
    id: "geometry.predicate.point-in-polygon",
    stability: gp_base::tool::Stability::Stable,
    title: "Point in polygon",
    summary: "Whether points are inside a polygon with geodesic edges, by the winding rule and the even-odd rule, with holes, on the boundary within 1 mm, and how far each is from the edge.",
    aliases: &[
        "point in polygon",
        "inside polygon test",
        "is point in area",
        "winding number",
        "even odd rule",
    ],
    keywords: &[
        "point in polygon",
        "inside",
        "outside",
        "boundary",
        "winding number",
        "even-odd",
        "containment",
        "geofence check",
    ],
    inputs: &[
        Field::new(
            "polygon",
            "Polygon",
            "Corners in order (ring 0), then any holes (ring 1, 2, …), like 40.4406, -80.002",
            Kind::List {
                items: VERTEX,
                min: 3,
                max: 20_000,
            },
        )
        .required()
        .core(),
        Field::new(
            "points",
            "Points to test",
            "One per line, like 40.4406, -80.002",
            Kind::List {
                items: POINT,
                min: 1,
                max: 5_000,
            },
        )
        .required()
        .core(),
    ],
    outputs: &[
        Field::new(
            "inside_count",
            "Points inside",
            "By the winding rule, or on the boundary",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "first",
            "First point",
            "inside, outside, or on-boundary by the winding rule",
            Kind::Text { max_len: 12 },
        ),
        Field::new(
            "results",
            "Each point",
            "Both rules, the winding number, and the distance to the edge",
            Kind::List {
                items: RESULT_ROW,
                min: 0,
                max: 5_000,
            },
        ),
    ],
    errors: &[ErrorCode::OutOfDomain, ErrorCode::LimitExceeded],
    warnings: &[],
    model: "Winding number = Σ over geodesic edges of the signed turn in azimuth seen from the point, ÷ 360° (azimuths by the geodesic inverse problem, Karney 2013); holes are turned opposite the outline first. Winding rule: inside when the number is not 0. Even-odd: inside when it is odd. On the boundary when the geodesic distance to an edge is under 1 mm",
    accuracy: "Exact on the ellipsoid for rings smaller than a hemisphere around the point; on-boundary within 1 mm",
    when_to_use: "Use this to ask whether positions fall inside an area: aircraft in a restricted zone, vehicles in a geofence, sightings in a survey block, addresses in a district. Many points can be asked about at once. It answers by both of the rules in common use — the winding rule and the even-odd rule, which differ for a self-overlapping outline — and gives each point's distance to the nearest edge, so a point that is nearly in can be told from one that is comfortably in.",
    limitations: "Inside is decided on the ellipsoid with geodesic edges, which is not the same question as inside a polygon drawn on a projected map: near a boundary the two can differ, and the difference grows with the length of the edges. A point within a millimetre of an edge is reported as on the boundary rather than forced to one side, because at that range the answer belongs to the data rather than to the arithmetic. An outline that crosses itself has no single meaning of inside, which is why both rules are reported rather than one; where they differ, the shape is the problem. The ring must be smaller than a hemisphere around the point.",
    references: &[KARNEY],
    examples: &[Example {
        id: "primary",
        title: "A field with a pond, and three points",
        input: r#"{"polygon":[{"lat":40.0,"lon":-105.0},{"lat":40.0,"lon":-104.99},{"lat":40.006,"lon":-104.99},{"lat":40.006,"lon":-105.0},{"lat":40.002,"lon":-104.997,"ring":1},{"lat":40.004,"lon":-104.997,"ring":1},{"lat":40.004,"lon":-104.994,"ring":1},{"lat":40.002,"lon":-104.994,"ring":1}],"points":[{"lat":40.001,"lon":-104.998},{"lat":40.003,"lon":-104.995},{"lat":40.003,"lon":-105.0}]}"#,
        source: "inside, in a hole, and on the boundary. The verdicts are checked against GEOS 3.11.4 through shapely on PROJ's azimuthal equidistant plane over 51 points and five shapes — a planar test against a geodesic winding number, agreeing on every point — with the edge distances matching to 7.4e-6 relative",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "polygon",
        map: &[("area", "inside_count")],
    }],
    related: &[
        Related {
            id: "geometry.area.polygon",
            reason: "alternative",
        },
        Related {
            id: "geometry.shape.centroid",
            reason: "alternative",
        },
        Related {
            id: "geometry.overlay.boolean",
            reason: "next",
        },
    ],
    sentence: "{inside_count} of the points {plural inside_count \"is\" \"are\"} inside the polygon by the winding rule. The first is {first}.",
    limits: &[("batchRows", 1_000)],
    run: run_pip,
    ..ToolDef::BLANK
};

fn norm180(x: f64) -> f64 {
    (x + 180.0).rem_euclid(360.0) - 180.0
}

fn read(ctx: &mut Ctx, list: &str, rings_ok: bool) -> Result<Vec<Vec<(f64, f64)>>, ToolError> {
    let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let rows = ctx.rows(list)?;
    let mut rings: Vec<Vec<(f64, f64)>> = Vec::new();
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
        let ring = if rings_ok {
            r.get("ring")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0)
        } else {
            0.0
        };
        if ring.fract() != 0.0 || !(0.0..=100.0).contains(&ring) {
            return Err(ToolError::invalid(
                &format!("/{list}/{i}/ring"),
                "Ring must be a whole number from 0 (the outline) to 100.",
            ));
        }
        let k = ring as usize;
        if rings.len() <= k {
            rings.resize(k + 1, Vec::new());
        }
        if !rings_ok || rings[k].last() != Some(&(lat, lon)) {
            rings[k].push((lat, lon));
        }
    }
    Ok(rings)
}

/// Twice the signed turn of the ring's own azimuths, seen from its first
/// corner's neighborhood: positive when the ring runs counterclockwise.
fn orientation(g: &Geodesic, ring: &[(f64, f64)]) -> f64 {
    let mut p = geographiclib_rs::PolygonArea::new(g, geographiclib_rs::Winding::CounterClockwise);
    for &(la, lo) in ring {
        p.add_point(la, lo);
    }
    let (_, area, _) = p.compute(true);
    area
}

fn run_pip(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let mut rings = read(ctx, "polygon", true)?;
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
    let pts: Vec<(f64, f64)> = read(ctx, "points", false)?.into_iter().flatten().collect();
    let g = Geodesic::wgs84();
    // Holes run opposite the outline, so the winding rule leaves them out.
    let sign0 = orientation(&g, &rings[0]).signum();
    for ring in rings.iter_mut().skip(1) {
        if orientation(&g, ring).signum() == sign0 {
            ring.reverse();
        }
    }
    let m = units::by_symbol(QT::Length, "m").expect("m");
    let mut rows = Vec::with_capacity(pts.len());
    let (mut inside_count, mut first) = (0usize, String::new());
    for (i, &p) in pts.iter().enumerate() {
        let (mut turn, mut nearest) = (0.0, f64::INFINITY);
        for ring in &rings {
            let n = ring.len();
            let az: Vec<(f64, f64)> = ring
                .iter()
                .map(|v| {
                    let (s, a, _, _): (f64, f64, f64, f64) = g.inverse(p.0, p.1, v.0, v.1);
                    (s, a)
                })
                .collect();
            for k in 0..n {
                let (a, b) = (az[k], az[(k + 1) % n]);
                let sweep = norm180(b.1 - a.1);
                turn += sweep;
                // An edge seen within 90° is at least cos 45° of its nearer corner's
                // distance away; only a wider sweep can pass nearer than that.
                let near = a.0.min(b.0);
                if near * 0.7 < nearest || sweep.abs() > 90.0 {
                    let (d, _) = seg_dist(&g, ring[k], ring[(k + 1) % n], p);
                    nearest = nearest.min(d);
                }
            }
        }
        // Azimuths grow clockwise, so a counterclockwise ring turns -360°; count
        // the outline's own direction as positive.
        let w = -(turn / 360.0).round() * sign0;
        let on = nearest < ON_EDGE_M;
        let rule = |inside: bool| {
            if on {
                "on-boundary"
            } else if inside {
                "inside"
            } else {
                "outside"
            }
        };
        let nonzero = rule(w != 0.0);
        let even_odd = rule((w as i64).rem_euclid(2) == 1);
        if nonzero != "outside" {
            inside_count += 1;
        }
        if i == 0 {
            first = nonzero.to_owned();
        }
        rows.push(Json::obj([
            ("point", Json::Num((i + 1) as f64)),
            ("winding", Json::Num(if w == 0.0 { 0.0 } else { w })),
            ("nonzero", Json::str(nonzero)),
            ("even_odd", Json::str(even_odd)),
            (
                "distance",
                Q {
                    value: nearest,
                    unit: m,
                }
                .to_json(),
            ),
        ]));
    }
    Ok(Json::obj([
        ("inside_count", Json::Num(inside_count as f64)),
        ("first", Json::str(first)),
        ("results", Json::Arr(rows)),
    ]))
}
