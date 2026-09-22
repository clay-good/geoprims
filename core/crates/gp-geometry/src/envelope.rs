//! Envelopes (add-navigation-and-geometry, geometry/computational,
//! "Envelopes"): the bounding box of points, a line, or a polygon, aware of
//! the antimeridian (west greater than east when the box crosses it) and of
//! geodesic edges that bow poleward between their corners.

use geographiclib_rs::{DirectGeodesic, Geodesic, InverseGeodesic};
use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use libm::cos;

const KARNEY: Reference = Reference {
    title: "Algorithms for geodesics",
    issuer: "Karney, C. F. F., Journal of Geodesy",
    year: 2013,
    edition: "Vol. 87, No. 1",
    locator: "pp. 43-55 (direct and inverse problems; the vertex of a geodesic, where its azimuth is 90°)",
    url: "https://doi.org/10.1007/s00190-012-0578-z",
};
const RFC7946: Reference = Reference {
    title: "The GeoJSON Format",
    issuer: "IETF RFC 7946",
    year: 2016,
    edition: "RFC 7946",
    locator: "Section 5.2 (the antimeridian: a bounding box with west greater than east)",
    url: "https://www.rfc-editor.org/rfc/rfc7946",
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

const fn deg_out(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7))
}

pub static BBOX: ToolDef = ToolDef {
    id: "geometry.shape.bbox",
    title: "Bounding box, antimeridian-aware",
    summary: "The smallest latitude and longitude box around points, a line, or a polygon, written west-south-east-north, crossing the antimeridian when that is shorter and reaching a pole a polygon circles.",
    aliases: &[
        "bounding box",
        "bbox",
        "extent",
        "envelope",
        "min max lat lon",
    ],
    keywords: &[
        "bbox",
        "bounding box",
        "extent",
        "envelope",
        "antimeridian",
        "dateline",
        "west",
        "east",
    ],
    inputs: &[
        Field::new(
            "points",
            "Points",
            "One per line, like 40.4406, -80.002",
            Kind::List {
                items: VERTEX,
                min: 1,
                max: 20_000,
            },
        )
        .required()
        .core(),
        Field::new(
            "shape",
            "Treat as",
            "points (the default), line, or polygon; a line or polygon adds its geodesic edges",
            Kind::Choice(&["points", "line", "polygon"]),
        )
        .core(),
    ],
    outputs: &[
        deg_out("west", "West", "Longitude of the west edge"),
        deg_out("south", "South", "Latitude of the south edge"),
        deg_out(
            "east",
            "East",
            "Longitude of the east edge; less than west when the box crosses the antimeridian",
        ),
        deg_out("north", "North", "Latitude of the north edge"),
        Field::new(
            "lon_span",
            "Longitude span",
            "East of west, going east",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(7)),
        Field::new(
            "crosses_antimeridian",
            "Crosses the antimeridian",
            "yes or no",
            Kind::Text { max_len: 3 },
        ),
        Field::new(
            "pole",
            "Pole enclosed",
            "none, north, or south",
            Kind::Text { max_len: 5 },
        ),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &["CROSSES_ANTIMERIDIAN", "POLE_ENCLOSED", "EXPERIMENTAL_TOOL"],
    model: "Longitudes: every point, and for a line or polygon each geodesic edge's shorter arc of longitude, marked on the circle; the box is the complement of the largest gap (RFC 7946 §5.2 writes a crossing box with west > east). Latitudes: the points, plus each edge's vertex where its azimuth passes 90° or 270°, found by bisection on the geodesic direct problem (Karney 2013). A polygon whose outline winds around a pole spans every longitude and reaches that pole",
    accuracy: "Edge extremes to 1e-9° or better; the box is the smallest in longitude span",
    references: &[KARNEY, RFC7946],
    examples: &[Example {
        id: "primary",
        title: "Two points either side of the antimeridian",
        input: r#"{"points":[{"lat":-17.0,"lon":170.0},{"lat":-15.0,"lon":-170.0}]}"#,
        source: "add-navigation-and-geometry antimeridian bbox scenario (west 170, east -170, a 20° span)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "bbox",
        map: &[
            ("south", "south"),
            ("west", "west"),
            ("north", "north"),
            ("east", "east"),
        ],
    }],
    related: &[
        Related {
            id: "geometry.shape.centroid",
            reason: "alternative",
        },
        Related {
            id: "geometry.area.polygon",
            reason: "next",
        },
    ],
    sentence: "The box runs from {west} to {east} in longitude, a {lon_span} span, and from {south} to {north} in latitude.",
    limits: &[("batchRows", 1_000)],
    run: run_bbox,
    ..ToolDef::BLANK
};

fn norm180(x: f64) -> f64 {
    let v = (x + 180.0).rem_euclid(360.0) - 180.0;
    if v == -180.0 && x > 0.0 { 180.0 } else { v }
}

/// The latitude where edge a→b turns (its azimuth passes 90° or 270°), if inside the edge.
fn edge_vertex(g: &Geodesic, a: (f64, f64), b: (f64, f64)) -> Option<f64> {
    let (s12, azi1, azi2, _): (f64, f64, f64, f64) = g.inverse(a.0, a.1, b.0, b.1);
    let (c1, c2) = (cos(azi1.to_radians()), cos(azi2.to_radians()));
    if s12 <= 0.0 || c1 * c2 >= 0.0 {
        return None;
    }
    // Northbound turning south (or the reverse): bisect on the sign of cos(azimuth).
    let (mut lo, mut hi) = (0.0, s12);
    for _ in 0..80 {
        let mid = 0.5 * (lo + hi);
        let (_, _, az): (f64, f64, f64) = g.direct(a.0, a.1, azi1, mid);
        if cos(az.to_radians()) * c1 > 0.0 {
            lo = mid
        } else {
            hi = mid
        }
    }
    let (lat, _, _): (f64, f64, f64) = g.direct(a.0, a.1, azi1, 0.5 * (lo + hi));
    Some(lat)
}

fn run_bbox(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let rows = ctx.rows("points")?;
    let mut rings: Vec<Vec<(f64, f64)>> = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        let lat = ctx
            .row_quantity("points", i, r, "lat")?
            .expect("required")
            .to(deg);
        let lon = ctx
            .row_quantity("points", i, r, "lon")?
            .expect("required")
            .to(deg);
        if !(-90.0..=90.0).contains(&lat) {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "Latitude must be between -90° and 90°.",
            )
            .at(&format!("/points/{i}/lat")));
        }
        let ring = r
            .get("ring")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);
        if ring.fract() != 0.0 || !(0.0..=100.0).contains(&ring) {
            return Err(ToolError::invalid(
                &format!("/points/{i}/ring"),
                "Ring must be a whole number from 0 to 100.",
            ));
        }
        let k = ring as usize;
        if rings.len() <= k {
            rings.resize(k + 1, Vec::new());
        }
        rings[k].push((lat, norm180(lon)));
    }
    rings.retain(|r| !r.is_empty());
    let shape = ctx.choice("shape")?.unwrap_or("points");
    let g = Geodesic::wgs84();
    let (mut south, mut north) = (f64::INFINITY, f64::NEG_INFINITY);
    // Arcs of longitude covered: (start, eastward length).
    let mut arcs: Vec<(f64, f64)> = Vec::new();
    for ring in &rings {
        for &(lat, lon) in ring {
            south = south.min(lat);
            north = north.max(lat);
            arcs.push((lon, 0.0));
        }
        let n = ring.len();
        let edges = match shape {
            "line" => n.saturating_sub(1),
            "polygon" if n >= 3 => n,
            _ => 0,
        };
        for i in 0..edges {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            if let Some(lat) = edge_vertex(&g, a, b) {
                south = south.min(lat);
                north = north.max(lat);
            }
            let d = norm180(b.1 - a.1);
            arcs.push(if d >= 0.0 { (a.1, d) } else { (b.1, -d) });
        }
    }
    // A polygon outline that winds once around a pole covers every longitude up to it.
    let mut pole = "none";
    if shape == "polygon" && rings[0].len() >= 3 {
        let r = &rings[0];
        let turn: f64 = (0..r.len())
            .map(|i| norm180(r[(i + 1) % r.len()].1 - r[i].1))
            .sum();
        if turn.abs() > 180.0 {
            pole = if r.iter().map(|p| p.0).sum::<f64>() >= 0.0 {
                "north"
            } else {
                "south"
            };
        }
    }
    let (west, east, span) = if pole != "none" {
        if pole == "north" {
            north = 90.0
        } else {
            south = -90.0
        }
        (-180.0, 180.0, 360.0)
    } else {
        // The largest gap between covered arcs, going east around the circle.
        arcs.sort_by(|x, y| x.0.total_cmp(&y.0));
        let (mut best_gap, mut best_start) = (-1.0, 0.0);
        let mut reach = arcs[0].0 + arcs[0].1;
        for &(s, l) in &arcs[1..] {
            if s > reach && s - reach > best_gap {
                best_gap = s - reach;
                best_start = s;
            }
            reach = reach.max(s + l);
        }
        // The wrap-around gap, from the farthest reach back to the first start.
        let wrap = arcs[0].0 + 360.0 - reach;
        if wrap > best_gap {
            best_gap = wrap;
            best_start = arcs[0].0;
        }
        if best_gap <= 0.0 {
            (-180.0, 180.0, 360.0)
        } else {
            let span = 360.0 - best_gap;
            (best_start, norm180(best_start + span), span)
        }
    };
    let crosses = pole == "none" && span < 360.0 && east < west;
    if crosses {
        ctx.warnings.push(Warning::new(
            "CROSSES_ANTIMERIDIAN",
            format!("The box crosses the antimeridian, so west ({west}°) is greater than east ({east}°), as RFC 7946 writes it."),
        ));
    }
    if pole != "none" {
        ctx.warnings.push(Warning::new("POLE_ENCLOSED", format!("The outline circles the {pole} pole, so the box spans every longitude and reaches it.")));
    }
    let dq = |v: f64| Q {
        value: v,
        unit: deg,
    };
    Ok(Json::obj([
        ("west", ctx.out("west", dq(west))),
        ("south", ctx.out("south", dq(south))),
        ("east", ctx.out("east", dq(east))),
        ("north", ctx.out("north", dq(north))),
        ("lon_span", ctx.out("lon_span", dq(span))),
        (
            "crosses_antimeridian",
            Json::str(if crosses { "yes" } else { "no" }),
        ),
        ("pole", Json::str(pole)),
    ]))
}
