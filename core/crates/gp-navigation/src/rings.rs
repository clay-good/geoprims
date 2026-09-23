//! Range rings (add-navigation-and-geometry, navigation/route-geometry,
//! "Range rings and circles"): geodesic circles at one or more radii around
//! a center, densified to the canvas accuracy rule, and written as GeoJSON
//! cut at the antimeridian and around a pole per RFC 7946.

use geographiclib_rs::{DirectGeodesic, Geodesic, PolygonArea, Winding};
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{
    Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, Stability, ToolDef,
};
use gp_base::units::{self, Quantity as QT};
use gp_geo::buffer::disk_sides;
use gp_geo::point;

const KARNEY: Reference = Reference {
    title: "Algorithms for geodesics",
    issuer: "Karney, C. F. F., Journal of Geodesy",
    year: 2013,
    edition: "Vol. 87, No. 1",
    locator: "pp. 43-55 (the direct problem; area of geodesic polygons, section 6)",
    url: "https://doi.org/10.1007/s00190-012-0578-z",
};
const RFC7946: Reference = Reference {
    title: "The GeoJSON Format",
    issuer: "IETF RFC 7946",
    year: 2016,
    edition: "RFC 7946",
    locator: "Sections 3.1.6 (polygon rings counterclockwise) and 3.1.9 (cutting at the antimeridian)",
    url: "https://www.rfc-editor.org/rfc/rfc7946",
};

const RADIUS_ROW: &[Field] = &[Field::new(
    "radius",
    "Radius",
    "Like 25 NM or 10 km",
    Kind::Quantity {
        q: QT::Distance,
        unit: "km",
    },
)
.required()];

const RING_ROW: &[Field] = &[
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
        "Ring",
        "0 for the first radius, 1 for the next, …",
        Kind::Number {
            min: 0.0,
            max: 100.0,
        },
    )
    .precision(Precision::Decimals(0)),
];
const SUMMARY_ROW: &[Field] = &[
    Field::new(
        "radius",
        "Radius",
        "As given",
        Kind::Quantity {
            q: QT::Distance,
            unit: "km",
        },
    )
    .precision(crate::DIST_P),
    Field::new(
        "area",
        "Area inside",
        "Of the drawn ring",
        Kind::Quantity {
            q: QT::Area,
            unit: "km2",
        },
    )
    .precision(Precision::Significant(8)),
    Field::new(
        "pole",
        "Pole inside",
        "none, north, or south",
        Kind::Text { max_len: 5 },
    ),
    Field::new(
        "crosses_antimeridian",
        "Crosses the antimeridian",
        "yes or no",
        Kind::Text { max_len: 3 },
    ),
];

pub static RANGE_RINGS: ToolDef = ToolDef {
    id: "navigation.route.range-rings",
    version: "1.0.1",
    title: "Range rings",
    summary: "Geodesic circles at one or more distances around a point, drawn true on the ellipsoid, with their areas, and a GeoJSON file cut correctly at the antimeridian and around a pole.",
    aliases: &[
        "range rings",
        "radius rings",
        "distance circles",
        "geodesic circle",
        "radius around a point",
    ],
    keywords: &[
        "range ring",
        "circle",
        "radius",
        "distance",
        "geodesic circle",
        "coverage",
        "GeoJSON",
        "antimeridian",
        "pole",
    ],
    inputs: &[
        point::lat_field("lat", "Center latitude"),
        point::lon_field("lon", "Center longitude"),
        Field::new(
            "radii",
            "Radii",
            "One per line, like 10 NM, 25 NM, 50 NM",
            Kind::List {
                items: RADIUS_ROW,
                min: 1,
                max: 20,
            },
        )
        .required()
        .core(),
        Field::new(
            "points",
            "Points per ring",
            "Like 144; by default enough that each chord sags under 0.025% of the radius or 0.125 m",
            Kind::Number {
                min: 8.0,
                max: 3_600.0,
            },
        ),
    ],
    outputs: &[
        Field::new(
            "ring_count",
            "Rings",
            "One per radius",
            Kind::Number {
                min: 0.0,
                max: 100.0,
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "summary",
            "Each ring",
            "Radius, area, and where it reaches",
            Kind::List {
                items: SUMMARY_ROW,
                min: 0,
                max: 20,
            },
        ),
        Field::new(
            "filename",
            "File name",
            "Suggested",
            Kind::Text { max_len: 60 },
        ),
        Field::new(
            "media_type",
            "Media type",
            "For the download",
            Kind::Text { max_len: 40 },
        ),
        Field::new(
            "file",
            "GeoJSON",
            "The rings as RFC 7946 features",
            Kind::Text {
                max_len: 10_000_000,
            },
        ),
        Field::new(
            "rings",
            "Ring points",
            "Every ring's points, for drawing",
            Kind::List {
                items: RING_ROW,
                min: 0,
                max: 100_000,
            },
        ),
    ],
    errors: &[gp_base::ErrorCode::OutOfDomain],
    stability: Stability::Stable,
    when_to_use: "Use this to draw circles of true ground distance around a point -- a fuel radius, a radio horizon, a search area, a buffer zone -- as GeoJSON you can put straight on a map. Every vertex is exactly the distance you asked for, at any latitude.",
    limitations: "A ring of constant ground distance is not a circle on a map, and drawing one as a map circle is the error this replaces: at high latitude the two are wildly different. The ring is a polygon, so the boundary between vertices is a chord rather than an arc, sagging 0.024% of the radius below the true circle at the default point count and giving an area slightly under the true one; more points reduce it as one over n squared. A ring that encloses a pole or crosses the antimeridian says so in a warning, because both need care in whatever draws them next.",
    warnings: &[
        "POLE_ENCLOSED",
        "CROSSES_ANTIMERIDIAN",
        "UNIT_ASSUMED",
        "INPUT_NORMALIZED",
    ],
    model: "Each ring's points by the geodesic direct problem at equal azimuth steps from the center (Karney 2013), counterclockwise; area by Karney's geodesic polygon area. The GeoJSON splits a ring that crosses the antimeridian into a MultiPolygon, and turns a ring around a pole into a polygon that runs along ±180° to the pole, as RFC 7946 §3.1.9 asks",
    accuracy: "Points exact to the geodesic; with the default count each chord sags under 0.025% of the radius (or 0.125 m), inside the 0.1% canvas rule",
    references: &[KARNEY, RFC7946],
    examples: &[Example {
        id: "primary",
        title: "10, 25, and 50 NM around Denver International",
        input: r#"{"lat":39.8617,"lon":-104.6731,"radii":[{"radius":"10 NM"},{"radius":"25 NM"},{"radius":"50 NM"}]}"#,
        source: "add-navigation-and-geometry range rings",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "polygon",
        map: &[("rings", "rings")],
    }],
    related: &[
        Related {
            id: "navigation.geodesic.direct",
            reason: "parent",
        },
        Related {
            id: "navigation.route.legs",
            reason: "next",
        },
        Related {
            id: "navigation.geodesic.inverse",
            reason: "alternative",
        },
    ],
    sentence: "Drew {ring_count} {plural ring_count \"ring\" \"rings\"} around the point, each true on the ellipsoid.",
    limits: &[("batchRows", 100)],
    run: run_rings,
    ..ToolDef::BLANK
};

fn norm180(x: f64) -> f64 {
    (x + 180.0).rem_euclid(360.0) - 180.0
}

fn round7(x: f64) -> f64 {
    let v = (x * 1e7).round() / 1e7;
    if v == 0.0 { 0.0 } else { v }
}

/// Clips a ring (x = unwrapped longitude, y = latitude) to one side of x = at.
fn clip(ring: &[(f64, f64)], at: f64, keep_left: bool) -> Vec<(f64, f64)> {
    let inside = |p: (f64, f64)| if keep_left { p.0 <= at } else { p.0 >= at };
    let mut out = Vec::new();
    for i in 0..ring.len() {
        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
        if inside(a) {
            out.push(a);
        }
        if inside(a) != inside(b) {
            let t = (at - a.0) / (b.0 - a.0);
            out.push((at, a.1 + t * (b.1 - a.1)));
        }
    }
    out
}

/// A closed GeoJSON ring of [lon, lat], rounded.
fn closed(ring: &[(f64, f64)]) -> Json {
    let mut v: Vec<Json> = ring
        .iter()
        .map(|&(x, y)| Json::Arr(vec![Json::Num(round7(x)), Json::Num(round7(y))]))
        .collect();
    v.push(v[0].clone());
    Json::Arr(v)
}

/// The RFC 7946 geometry for one counterclockwise ring of (lat, lon).
fn geometry(pts: &[(f64, f64)], pole: &str) -> (Json, bool) {
    let n = pts.len();
    if pole != "none" {
        // Around a pole: every longitude once, in order, closed along ±180° and the pole.
        let mut s: Vec<(f64, f64)> = pts.iter().map(|&(la, lo)| (norm180(lo), la)).collect();
        s.sort_by(|a, b| a.0.total_cmp(&b.0));
        let (first, last) = (s[0], s[s.len() - 1]);
        let t = (180.0 - last.0) / (first.0 + 360.0 - last.0);
        let edge = last.1 + t * (first.1 - last.1);
        let mut ring = Vec::with_capacity(s.len() + 4);
        if pole == "north" {
            ring.push((-180.0, edge));
            ring.extend(s);
            ring.extend([(180.0, edge), (180.0, 90.0), (-180.0, 90.0)]);
        } else {
            ring.extend([(-180.0, -90.0), (180.0, -90.0), (180.0, edge)]);
            ring.extend(s.into_iter().rev());
            ring.push((-180.0, edge));
        }
        return (
            Json::obj([
                ("type", Json::str("Polygon")),
                ("coordinates", Json::Arr(vec![closed(&ring)])),
            ]),
            true,
        );
    }
    // Unwrap longitudes so the ring is continuous, then cut where it passes ±180°.
    let mut u = Vec::with_capacity(n);
    u.push((pts[0].1, pts[0].0));
    for i in 1..n {
        let prev = u[i - 1].0;
        u.push((prev + norm180(pts[i].1 - pts[i - 1].1), pts[i].0));
    }
    let (lo, hi) = u
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| {
            (a.min(p.0), b.max(p.0))
        });
    let shift = if lo < -180.0 {
        360.0
    } else if hi > 180.0 {
        0.0
    } else {
        f64::NAN
    };
    if shift.is_nan() {
        return (
            Json::obj([
                ("type", Json::str("Polygon")),
                ("coordinates", Json::Arr(vec![closed(&u)])),
            ]),
            false,
        );
    }
    // Put the crossing at +180° (unwrapped), then fold the east part back by 360°.
    let u: Vec<(f64, f64)> = u.iter().map(|&(x, y)| (x + shift, y)).collect();
    let west = clip(&u, 180.0, true);
    let east: Vec<(f64, f64)> = clip(&u, 180.0, false)
        .into_iter()
        .map(|(x, y)| (x - 360.0, y))
        .collect();
    let parts: Vec<Json> = [west, east]
        .iter()
        .filter(|p| p.len() >= 3)
        .map(|p| Json::Arr(vec![closed(p)]))
        .collect();
    (
        Json::obj([
            ("type", Json::str("MultiPolygon")),
            ("coordinates", Json::Arr(parts)),
        ]),
        true,
    )
}

fn run_rings(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let m = units::by_symbol(QT::Distance, "m").expect("m");
    let rows = ctx.rows("radii")?;
    let mut radii = Vec::with_capacity(rows.len());
    for (i, row) in rows.iter().enumerate() {
        let q = ctx
            .row_quantity("radii", i, row, "radius")?
            .expect("required");
        let r = q.to(m);
        if !(r > 0.0 && r <= 10_000_000.0) {
            return Err(ToolError::new(gp_base::ErrorCode::OutOfDomain, "A radius must be above 0 and at most 10,000 km (a quarter of the way around the Earth).").at(&format!("/radii/{i}/radius")));
        }
        radii.push((q, r));
    }
    let fixed = ctx.number("points")?.map(|n| n.round() as usize);
    let g = Geodesic::wgs84();
    let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let (mut features, mut summary, mut draw) = (Vec::new(), Vec::new(), Vec::new());
    let (mut any_pole, mut any_cross) = (Vec::new(), false);
    for (k, &(q, r)) in radii.iter().enumerate() {
        let n = fixed.unwrap_or_else(|| disk_sides(r, 0.25 * (0.001 * r).max(0.5)).max(72));
        // Decreasing azimuth runs counterclockwise seen from above, as RFC 7946 wants.
        let pts: Vec<(f64, f64)> = (0..n)
            .map(|i| {
                let az = 360.0 - 360.0 * i as f64 / n as f64;
                let (la, lo, _): (f64, f64, f64) = g.direct(lat, lon, az, r);
                (la, norm180(lo))
            })
            .collect();
        let turn: f64 = (0..n).map(|i| norm180(pts[(i + 1) % n].1 - pts[i].1)).sum();
        let pole = if turn.abs() > 180.0 {
            if lat >= 0.0 { "north" } else { "south" }
        } else {
            "none"
        };
        let mut pa = PolygonArea::new(&g, Winding::CounterClockwise);
        for &(la, lo) in &pts {
            pa.add_point(la, lo);
        }
        let (_, area, _) = pa.compute(true);
        let (geom, cut) = geometry(&pts, pole);
        let crosses = cut && pole == "none";
        any_cross |= crosses;
        if pole != "none" {
            any_pole.push(format!("{} ring encloses the {pole} pole", ordinal(k)));
        }
        features.push(Json::obj([
            ("type", Json::str("Feature")),
            (
                "properties",
                Json::obj([
                    ("radius_m", Json::Num((r * 1000.0).round() / 1000.0)),
                    (
                        "center",
                        Json::Arr(vec![Json::Num(round7(lon)), Json::Num(round7(lat))]),
                    ),
                ]),
            ),
            ("geometry", geom),
        ]));
        summary.push(Json::obj([
            ("radius", ctx.emit("radius", q, q.unit)),
            (
                "area",
                Q {
                    value: area.abs() / 1e6,
                    unit: units::by_symbol(QT::Area, "km2").expect("km2"),
                }
                .to_json(),
            ),
            ("pole", Json::str(pole)),
            (
                "crosses_antimeridian",
                Json::str(if crosses { "yes" } else { "no" }),
            ),
        ]));
        for &(la, lo) in &pts {
            draw.push(Json::obj([
                (
                    "lat",
                    Q {
                        value: la,
                        unit: deg,
                    }
                    .to_json(),
                ),
                (
                    "lon",
                    Q {
                        value: lo,
                        unit: deg,
                    }
                    .to_json(),
                ),
                ("part", Json::Num(k as f64)),
            ]));
        }
    }
    if !any_pole.is_empty() {
        ctx.warnings.push(Warning::new(
            "POLE_ENCLOSED",
            format!(
                "The {}; its GeoJSON runs along ±180° to the pole.",
                any_pole.join(", and the ")
            ),
        ));
    }
    if any_cross {
        ctx.warnings.push(Warning::new("CROSSES_ANTIMERIDIAN", "A ring crosses the antimeridian, so its GeoJSON is split into two polygons there (RFC 7946 §3.1.9)."));
    }
    let doc = Json::obj([
        ("type", Json::str("FeatureCollection")),
        ("features", Json::Arr(features)),
    ]);
    Ok(Json::obj([
        ("ring_count", Json::Num(radii.len() as f64)),
        ("summary", Json::Arr(summary)),
        ("filename", Json::str("range-rings.geojson")),
        ("media_type", Json::str("application/geo+json")),
        ("file", Json::str(doc.to_string()? + "\n")),
        ("rings", Json::Arr(draw)),
    ]))
}

fn ordinal(k: usize) -> String {
    match k {
        0 => "first".into(),
        1 => "second".into(),
        2 => "third".into(),
        _ => format!("ring {}", k + 1),
    }
}
