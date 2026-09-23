//! Points along a geodesic or rhumb line: at N equal intervals, at a fixed
//! spacing, or at given fractions, with cumulative distance and azimuth, and
//! the same points as a GPX route and GeoJSON.

use geographiclib_rs::{DirectGeodesic, InverseGeodesic};
use gp_base::ErrorCode;
use gp_base::angle::wrap_lon;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Related, ToolDef};
use gp_base::units::Quantity as QT;
use gp_geo::rhumb::Rhumb;

use crate::{E, KARNEY, LAT1, LAT2, LON1, LON2, setup, two_points};

/// At most this many points, so the GPX and GeoJSON stay a sensible size.
pub const MAX_POINTS: usize = 10_000;

const POINT_ROW: &[Field] = &[
    Field::new(
        "n",
        "Point",
        "1 is the start",
        Kind::Number { min: 1.0, max: 1e5 },
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
    Field::new(
        "distance",
        "Distance from the start",
        "Along the line",
        Kind::Quantity {
            q: QT::Distance,
            unit: "km",
        },
    )
    .precision(Precision::Decimals(3)),
    Field::new(
        "azimuth",
        "Course here",
        "Degrees true, like 090",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(3))
    .angle_range("[0,360)"),
];

pub static WAYPOINTS: ToolDef = ToolDef {
    id: "navigation.geodesic.waypoints",
    stability: gp_base::tool::Stability::Stable,
    title: "Waypoints along a route line",
    summary: "Points along the geodesic (or rhumb line) from A to B at N equal intervals, a fixed spacing, or given fractions, with distance and course at each, as a table, a GPX route, and GeoJSON.",
    aliases: &[
        "intermediate points",
        "densify geodesic",
        "great circle waypoints",
        "points along a line",
    ],
    keywords: &[
        "waypoints",
        "intermediate point",
        "densify",
        "great circle",
        "GPX",
        "GeoJSON",
        "spacing",
    ],
    inputs: &[
        LAT1,
        LON1,
        LAT2,
        LON2,
        Field::new(
            "intervals",
            "Equal intervals",
            "Like 10 (gives 11 points)",
            Kind::Number {
                min: 1.0,
                max: 9_999.0,
            },
        )
        .core(),
        Field::new(
            "spacing",
            "Spacing",
            "Like 100 km; the last interval may be shorter",
            Kind::Quantity {
                q: QT::Distance,
                unit: "km",
            },
        ),
        Field::new(
            "fractions",
            "Fractions",
            "Like 0.25, 0.5, 0.75 (0 is A, 1 is B)",
            Kind::Text { max_len: 2000 },
        ),
        Field::new(
            "path",
            "Line",
            "geodesic (default) or rhumb",
            Kind::Choice(&["geodesic", "rhumb"]),
        ),
        E[0],
        E[1],
        E[2],
    ],
    outputs: &[
        Field::new(
            "count",
            "Points",
            "Including both ends for intervals and spacing",
            Kind::Number { min: 1.0, max: 1e5 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "length",
            "Line length",
            "A to B",
            Kind::Quantity {
                q: QT::Distance,
                unit: "km",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "points",
            "Points",
            "In order from A",
            Kind::List {
                items: POINT_ROW,
                min: 1,
                max: MAX_POINTS,
            },
        ),
        Field::new(
            "gpx",
            "GPX route",
            "Paste into a GPX file",
            Kind::Text { max_len: 2_000_000 },
        ),
        Field::new(
            "geojson",
            "GeoJSON",
            "A LineString feature",
            Kind::Text { max_len: 2_000_000 },
        ),
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::LimitExceeded,
        ErrorCode::Unsupported,
    ],
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED"],
    when_to_use: "Use this to lay points along a route: evenly spaced marks for a flight log, a track to draw on a map, a line to sample terrain or weather along, or a GPX or GeoJSON file to load somewhere else. The points sit on the geodesic, which is the line the route actually follows.",
    limitations: "The points are on the shortest path, so drawn on a Mercator chart they curve; a line that holds one heading is the rhumb tool instead. Spacing is even in distance along the line, not in latitude or longitude, and not in time unless the speed is constant. Between two nearly antipodal points the shortest path is poorly determined, so a small change in either end can swing the whole line to the other side of the globe.",
    model: "Karney (2013) geodesic on WGS 84",
    accuracy: "Points on the geodesic to nanometers",
    references: &[KARNEY],
    examples: &[Example {
        id: "primary",
        title: "JFK to London Heathrow in 10 equal intervals",
        input: r#"{"lat1":40.6413,"lon1":-73.7781,"lat2":51.47,"lon2":-0.4543,"intervals":10}"#,
        source: "navigation geodesic scenario: 11 points, each 555,490.879 m apart along the geodesic",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "line-geodesic",
        map: &[("distance", "length")],
    }],
    related: &[
        Related {
            id: "navigation.geodesic.inverse",
            reason: "parent",
        },
        Related {
            id: "navigation.geodesic.direct",
            reason: "alternative",
        },
        Related {
            id: "navigation.route.legs",
            reason: "next",
        },
    ],
    sentence: "There are {count} points along the {length} line.",
    limits: &[("maxPoints", MAX_POINTS as u64)],
    run: run_waypoints,
    ..ToolDef::BLANK
};

fn run_waypoints(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (la1, lo1, la2, lo2) = two_points(ctx)?;
    let rhumb = ctx.choice("path")? == Some("rhumb");
    let (e, g) = setup(ctx)?;
    let rh = Rhumb::new(e.a, e.f);
    let (len, azi) = if rhumb {
        rh.inverse(la1, lo1, la2, lo2)
    } else {
        let (s, a1, _, _): (f64, f64, f64, f64) = g.inverse(la1, lo1, la2, lo2);
        (s, a1)
    };
    let n_int = ctx.number("intervals")?;
    let spacing = ctx.quantity("spacing")?.map(|q| q.base());
    let fractions = ctx.text("fractions")?;
    let given = [n_int.is_some(), spacing.is_some(), fractions.is_some()]
        .iter()
        .filter(|x| **x)
        .count();
    if given != 1 {
        return Err(ToolError::invalid(
            "/intervals",
            "Give exactly one of intervals, spacing, or fractions.",
        ));
    }
    let too_many = || {
        ToolError::new(
            ErrorCode::LimitExceeded,
            format!(
                "That is more than {MAX_POINTS} points; use fewer intervals or a wider spacing."
            ),
        )
        .at("/intervals")
    };
    let dists: Vec<f64> = if let Some(n) = n_int {
        if n.fract() != 0.0 {
            return Err(ToolError::invalid(
                "/intervals",
                "Intervals must be a whole number.",
            ));
        }
        let n = n as usize;
        if n + 1 > MAX_POINTS {
            return Err(too_many());
        }
        (0..=n).map(|i| len * i as f64 / n as f64).collect()
    } else if let Some(sp) = spacing {
        if sp <= 0.0 {
            return Err(
                ToolError::new(ErrorCode::OutOfDomain, "Spacing must be more than zero.")
                    .at("/spacing"),
            );
        }
        if len / sp + 2.0 > MAX_POINTS as f64 {
            return Err(too_many().at("/spacing"));
        }
        let mut v: Vec<f64> = (0..)
            .map(|i| i as f64 * sp)
            .take_while(|s| *s < len)
            .collect();
        v.push(len);
        v
    } else {
        let text = fractions.unwrap_or_default();
        let mut v = Vec::new();
        for part in text.split([',', ';', ' ']).filter(|p| !p.trim().is_empty()) {
            let f: f64 = part.trim().parse().map_err(|_| {
                ToolError::invalid("/fractions", format!("{part} is not a number."))
            })?;
            if !(-1e3..=1e3).contains(&f) {
                return Err(ToolError::new(
                    ErrorCode::OutOfDomain,
                    "Fractions must be between -1,000 and 1,000.",
                )
                .at("/fractions"));
            }
            v.push(f * len);
        }
        if v.is_empty() {
            return Err(ToolError::invalid(
                "/fractions",
                "Give at least one fraction, like 0.5.",
            ));
        }
        if v.len() > MAX_POINTS {
            return Err(too_many().at("/fractions"));
        }
        v
    };
    let dunit = ctx.output_unit("length");
    let deg = |v: f64| Json::obj([("value", Json::Num(v)), ("unit", Json::str("deg"))]);
    let mut rows = Vec::with_capacity(dists.len());
    let mut coords = Vec::with_capacity(dists.len());
    for (i, s) in dists.iter().enumerate() {
        let (lat, lon, az) = if rhumb {
            let end = rh.direct(la1, lo1, azi, *s);
            (end.lat, end.lon, azi)
        } else {
            let (lat, lon, az2): (f64, f64, f64) = g.direct(la1, lo1, azi, *s);
            (lat, lon, az2)
        };
        let lon = wrap_lon(lon);
        coords.push((lat, lon));
        rows.push(Json::obj([
            ("n", Json::Num((i + 1) as f64)),
            ("lat", deg(lat)),
            ("lon", deg(lon)),
            ("distance", ctx.emit("length", crate::meters(*s), dunit)),
            ("azimuth", deg(az.rem_euclid(360.0))),
        ]));
    }
    let gpx = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<gpx version=\"1.1\" creator=\"geoprims\" xmlns=\"http://www.topografix.com/GPX/1/1\">\n<rte>\n{}</rte>\n</gpx>\n",
        coords
            .iter()
            .enumerate()
            .map(|(i, (la, lo))| format!(
                "<rtept lat=\"{la}\" lon=\"{lo}\"><name>WP{}</name></rtept>\n",
                i + 1
            ))
            .collect::<String>()
    );
    let geojson = format!(
        "{{\"type\":\"Feature\",\"properties\":{{}},\"geometry\":{{\"type\":\"LineString\",\"coordinates\":[{}]}}}}",
        coords
            .iter()
            .map(|(la, lo)| format!("[{lo},{la}]"))
            .collect::<Vec<_>>()
            .join(",")
    );
    ctx.model = Some(format!(
        "{} on {}",
        if rhumb {
            "Rhumb line"
        } else {
            "Karney (2013) geodesic"
        },
        e.describe()
    ));
    Ok(Json::obj([
        ("count", Json::Num(rows.len() as f64)),
        ("length", ctx.out("length", crate::meters(len))),
        ("points", Json::Arr(rows)),
        ("gpx", Json::str(gpx)),
        ("geojson", Json::str(geojson)),
    ]))
}
