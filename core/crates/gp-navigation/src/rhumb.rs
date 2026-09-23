//! Rhumb lines (loxodromes): constant-course routes on the ellipsoid, the
//! inverse reporting how much longer they are than the geodesic.

use geographiclib_rs::InverseGeodesic;
use gp_base::ErrorCode;
use gp_base::angle::{wrap_azimuth, wrap_lon};
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use gp_geo::ellipsoid::Ellipsoid;
use gp_geo::point;
use gp_geo::rhumb::Rhumb;

use crate::{
    AZ_P, DIST_P, E, JFK_LHR, KARNEY, LAT1, LAT2, LON1, LON2, deg, meters, two_points, unit,
};

const KARNEY_RHUMB: Reference = Reference {
    title: "GeographicLib Rhumb class and RhumbSolve",
    issuer: "Karney, C. F. F., GeographicLib",
    year: 2014,
    edition: "GeographicLib 2.x",
    locator: "Rhumb lines on the ellipsoid (Rhumb, RhumbSolve)",
    url: "https://geographiclib.sourceforge.io/C++/doc/classGeographicLib_1_1Rhumb.html",
};

fn engine(ctx: &mut Ctx) -> Result<(Ellipsoid, Rhumb), ToolError> {
    let e = Ellipsoid::from_ctx(ctx)?;
    e.geodesic()?; // the same series limit (|f| ≤ 0.02) applies
    let r = Rhumb::new(e.a, e.f);
    Ok((e, r))
}

const fn km_field(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Distance,
            unit: "km",
        },
    )
    .precision(DIST_P)
}

pub static RHUMB_INVERSE: ToolDef = ToolDef {
    id: "navigation.rhumb.inverse",
    version: "1.0.1",
    stability: gp_base::tool::Stability::Stable,
    title: "Rhumb line distance and course",
    summary: "The constant course and distance of the rhumb line (loxodrome) between two points on the ellipsoid, and how much longer it is than the shortest route.",
    aliases: &[
        "rhumb line",
        "loxodrome",
        "constant course distance",
        "mercator course",
    ],
    keywords: &[
        "rhumb",
        "loxodrome",
        "constant heading",
        "course",
        "distance",
        "mercator",
    ],
    inputs: &[LAT1, LON1, LAT2, LON2, E[0], E[1], E[2]],
    outputs: &[
        km_field("distance", "Rhumb distance", "Along the constant course"),
        Field::new(
            "course",
            "Constant course",
            "Degrees clockwise from true north",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(AZ_P)
        .angle_range("[0,360)"),
        km_field(
            "geodesic_distance",
            "Geodesic distance",
            "The shortest route, for comparison",
        ),
        // A difference, not a distance: kept to the millimeter.
        km_field("extra_distance", "Extra distance", "Rhumb minus geodesic")
            .precision(Precision::Decimals(3)),
        Field::new(
            "extra_percent",
            "Extra over the geodesic (%)",
            "Rhumb minus geodesic, as a share of the geodesic",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Decimals(2))
        .optional(),
    ],
    errors: &[ErrorCode::Unsupported],
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Rhumb line on WGS 84 (conformal latitude and Krüger series, as GeographicLib's Rhumb)",
    accuracy: "Within 1 µm of GeographicLib's RhumbSolve (47 nm typical); the course within 1e-10°",
    when_to_use: "Use this when you want the single course that connects two points and the distance along it: the heading to steer without changing it, a line as drawn on a Mercator chart, or a comparison against the shortest route. The extra distance over the geodesic is reported, which is what tells you whether holding one course is worth it.",
    limitations: "The rhumb line is not the shortest path, and near the poles the difference grows quickly. The course is true. As with any two-point geometry here, it takes no account of terrain, airspace, traffic, or current, and a course held on a compass also needs the magnetic variation applied.",
    references: &[KARNEY_RHUMB, KARNEY],
    examples: &[Example {
        id: "primary",
        title: "New York JFK to London Heathrow by rhumb line",
        input: JFK_LHR,
        source: "add-navigation-and-geometry rhumb scenario: course 77.968°, 5,774,190 m, 219.3 km (3.9%) longer than the geodesic",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "line-rhumb",
        map: &[("distance", "distance")],
    }],
    related: &[
        Related {
            id: "navigation.rhumb.direct",
            reason: "inverse",
        },
        Related {
            id: "navigation.geodesic.inverse",
            reason: "alternative",
        },
        Related {
            id: "navigation.route.legs",
            reason: "next",
        },
    ],
    sentence: "The rhumb line runs {distance} on a constant course of {course}, {extra_distance} longer than the shortest route.",
    limits: &[("batchRows", 10_000)],
    run: run_inverse,
    ..ToolDef::BLANK
};

fn run_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let p = two_points(ctx)?;
    let (e, r) = engine(ctx)?;
    let (s, course) = r.inverse(p.0, p.1, p.2, p.3);
    let g: f64 = e.geodesic()?.inverse(p.0, p.1, p.2, p.3);
    ctx.model = Some(format!(
        "Rhumb line on {} (conformal latitude and Krüger series)",
        e.describe()
    ));
    let mut out = vec![
        ("distance", ctx.out("distance", meters(s))),
        ("course", ctx.out("course", deg(wrap_azimuth(course)))),
        ("geodesic_distance", ctx.out("geodesic_distance", meters(g))),
        // Never negative: the geodesic is the shortest route.
        (
            "extra_distance",
            ctx.out("extra_distance", meters((s - g).max(0.0))),
        ),
    ];
    if g > 0.0 {
        out.push(("extra_percent", Json::Num((s - g).max(0.0) / g * 100.0)));
    }
    Ok(Json::obj(out))
}

pub static RHUMB_DIRECT: ToolDef = ToolDef {
    id: "navigation.rhumb.direct",
    version: "1.0.1",
    stability: gp_base::tool::Stability::Stable,
    title: "Destination on a constant course (rhumb line)",
    summary: "Where a constant course (rhumb line, loxodrome) for a given distance ends on the ellipsoid; a rhumb that would cross a pole stops there.",
    aliases: &[
        "rhumb line destination",
        "constant course destination",
        "dead reckoning constant heading",
    ],
    keywords: &[
        "rhumb",
        "loxodrome",
        "constant course",
        "destination",
        "dead reckoning",
    ],
    inputs: &[
        LAT1,
        LON1,
        Field::new(
            "course",
            "Constant course",
            "Degrees clockwise from true north, like 78",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core()
        .angle_range("unbounded"),
        Field::new(
            "distance",
            "Distance",
            "Like 1000 km",
            Kind::Quantity {
                q: QT::Distance,
                unit: "km",
            },
        )
        .required()
        .core(),
        E[0],
        E[1],
        E[2],
    ],
    outputs: &[
        Field::new(
            "lat2",
            "Destination latitude",
            "Degrees, north positive",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(AZ_P)
        .angle_range("[-90,90]"),
        Field::new(
            "lon2",
            "Destination longitude",
            "Degrees, east positive",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(AZ_P)
        .angle_range("[-180,180)"),
        km_field(
            "beyond_pole",
            "Distance left at the pole",
            "What the rhumb could not travel past the pole",
        )
        .optional(),
    ],
    errors: &[ErrorCode::Unsupported],
    warnings: &[
        "RHUMB_REACHES_POLE",
        "INPUT_NORMALIZED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Rhumb line on WGS 84 (conformal latitude and Krüger series, as GeographicLib's Rhumb)",
    accuracy: "Within 1 µm of GeographicLib's RhumbSolve away from the poles; starts within 0.01° of a pole within 20 µm",
    when_to_use: "Use this when a course is held constant rather than flown as the shortest path: marine navigation on a single heading, a leg drawn on a Mercator chart, or any layout where the bearing must not change along the line. Given a start, a constant course, and a distance, it gives where you arrive on the ellipsoid.",
    limitations: "A rhumb line is longer than the geodesic, and much longer on high-latitude east-west runs; the inverse tool reports that difference. A rhumb course that would carry you over a pole stops at the pole instead, because the line spirals there. The course is true rather than magnetic, and this is geometry, not a route clear of terrain or traffic.",
    references: &[KARNEY_RHUMB],
    examples: &[Example {
        id: "primary",
        title: "1,000 km from JFK on a constant course of 078°",
        input: r#"{"lat1":40.6413,"lon1":-73.7781,"course":78,"distance":"1000 km"}"#,
        source: "GeographicLib RhumbSolve on WGS 84",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "point",
        map: &[("lat", "lat2"), ("lon", "lon2")],
    }],
    related: &[
        Related {
            id: "navigation.rhumb.inverse",
            reason: "inverse",
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
    sentence: "The destination is {lat2}, {lon2}.{warn RHUMB_REACHES_POLE} The rhumb line stops at the pole with {beyond_pole} left.{/warn}",
    limits: &[("batchRows", 10_000)],
    run: run_direct,
    ..ToolDef::BLANK
};

fn run_direct(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (la1, lo1) = point::read(ctx, "lat1", "lon1")?;
    let course = ctx.req_quantity("course")?.to(unit(QT::Angle, "deg"));
    let s = ctx.req_quantity("distance")?.to(unit(QT::Distance, "m"));
    let (e, r) = engine(ctx)?;
    let end = r.direct(la1, lo1, course, s);
    ctx.model = Some(format!(
        "Rhumb line on {} (conformal latitude and Krüger series)",
        e.describe()
    ));
    let mut out = vec![
        ("lat2", ctx.out("lat2", deg(end.lat))),
        ("lon2", ctx.out("lon2", deg(wrap_lon(end.lon)))),
    ];
    if end.beyond_pole > 0.0 {
        ctx.warnings.push(
            Warning::new(
                "RHUMB_REACHES_POLE",
                "A rhumb line spirals into the pole and cannot cross it, so it stops there; longitude at the pole is undefined, so the start's is shown.",
            )
            .at("/distance"),
        );
        out.push((
            "beyond_pole",
            ctx.out("beyond_pole", meters(end.beyond_pole)),
        ));
    }
    Ok(Json::obj(out))
}
