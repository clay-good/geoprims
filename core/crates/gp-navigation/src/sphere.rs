//! Spherical great-circle methods (add-navigation-and-geometry, geodesic
//! "Spherical great-circle methods"): inverse, direct, and intermediate point
//! on a sphere (the IUGG mean radius R1 by default), each beside the
//! ellipsoidal answer for the same inputs so the approximation error shows.

use geographiclib_rs::{DirectGeodesic, Geodesic, InverseGeodesic};
use gp_base::ErrorCode;
use gp_base::angle::{wrap_azimuth, wrap_lon};
use gp_base::display;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{
    Ctx, Example, Field, Kind, Layer, Limitation, Precision, Related, Stability, ToolDef,
};
use gp_base::units::Quantity as QT;
use gp_geo::point;
use libm::{asin, atan2, cos, sin, sqrt};

use crate::{
    AZ_P, DIST_P, IUGG_MEAN_RADIUS, JFK_LHR, KARNEY, LAT1, LAT2, LON1, LON2, R1, deg, meters,
    two_points, unit,
};

const RADIUS: Field = Field::new(
    "radius",
    "Sphere radius",
    "Default 6371008.771 m, the IUGG mean radius R1",
    Kind::Quantity {
        q: QT::Length,
        unit: "m",
    },
);

const fn km(name: &'static str, title: &'static str, help: &'static str) -> Field {
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

const fn metres(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Distance,
            unit: "m",
        },
    )
    .precision(Precision::Decimals(0))
}

const fn course(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(AZ_P)
    .angle_range("[0,360)")
}

const SPHERE_LIMITATION: Limitation = Limitation {
    simplification: "Works on a sphere, not on the ellipsoid the Earth actually is.",
    instead: "Use the geodesic tools for anything you act on: they are exact on WGS 84, and this tool reports how far the sphere puts it wrong for your inputs.",
    governs: "Karney (2013), the geodesic algorithm the geodesic tools use.",
};

fn radius(ctx: &mut Ctx) -> Result<f64, ToolError> {
    let r = ctx
        .quantity("radius")?
        .map_or(R1, |q| q.to(unit(QT::Length, "m")));
    if r.is_nan() || r <= 0.0 {
        return Err(ToolError::invalid(
            "/radius",
            "The radius must be positive.",
        ));
    }
    Ok(r)
}

/// Central angle (radians) and initial course (degrees) from 1 to 2 on a sphere.
pub fn sphere_inverse(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> (f64, f64) {
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dl = (lon2 - lon1).to_radians();
    let y = sqrt(
        (cos(p2) * sin(dl)).powi(2) + (cos(p1) * sin(p2) - sin(p1) * cos(p2) * cos(dl)).powi(2),
    );
    let x = sin(p1) * sin(p2) + cos(p1) * cos(p2) * cos(dl);
    let sigma = atan2(y, x);
    let theta = atan2(
        sin(dl) * cos(p2),
        cos(p1) * sin(p2) - sin(p1) * cos(p2) * cos(dl),
    );
    (sigma, wrap_azimuth(theta.to_degrees()))
}

/// The end of a great circle from (lat, lon) on course `c` (degrees) through
/// central angle `d` (radians): latitude, longitude.
pub fn sphere_direct(lat: f64, lon: f64, c: f64, d: f64) -> (f64, f64) {
    let (p1, th) = (lat.to_radians(), c.to_radians());
    let p2 = asin((sin(p1) * cos(d) + cos(p1) * sin(d) * cos(th)).clamp(-1.0, 1.0));
    let dl = atan2(sin(th) * sin(d) * cos(p1), cos(d) - sin(p1) * sin(p2));
    (p2.to_degrees(), wrap_lon(lon + dl.to_degrees()))
}

fn vec3(lat: f64, lon: f64) -> [f64; 3] {
    let (p, l) = (lat.to_radians(), lon.to_radians());
    [cos(p) * cos(l), cos(p) * sin(l), sin(p)]
}

// ---------------------------------------------------------------- inverse

pub static SPHERICAL_INVERSE: ToolDef = ToolDef {
    id: "navigation.geodesic.spherical-inverse",
    title: "Great-circle distance and course (spherical)",
    summary: "Distance and initial and final courses along the great circle on a sphere, with the difference from the ellipsoidal geodesic shown.",
    aliases: &[
        "great circle course",
        "great circle bearing",
        "initial bearing",
        "spherical inverse",
    ],
    keywords: &[
        "great circle",
        "sphere",
        "initial course",
        "final course",
        "bearing",
        "spherical",
    ],
    inputs: &[LAT1, LON1, LAT2, LON2, RADIUS],
    outputs: &[
        km("distance", "Great-circle distance", "On the sphere"),
        course("initial_course", "Initial course", "At the start, true"),
        course("final_course", "Final course", "At the end, true"),
        km(
            "ellipsoidal_distance",
            "Ellipsoidal distance",
            "Karney geodesic on WGS 84",
        ),
        metres(
            "difference",
            "Distance difference",
            "Ellipsoidal minus spherical",
        ),
        Field::new(
            "course_difference",
            "Initial course difference",
            "Spherical minus ellipsoidal",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(4))
        .angle_range("unbounded"),
    ],
    errors: &[ErrorCode::InvalidInput],
    stability: Stability::Stable,
    when_to_use: "Use this when you need the great-circle answer specifically: checking a figure from navigation software, a spreadsheet or a textbook, all of which use the spherical formulas. It reports the ellipsoidal distance beside its own and the gap between them, so you can see whether a discrepancy you are chasing is the sphere or a mistake.",
    limitations: "The sphere is the approximation and it is deliberate. Against the ellipsoid the distance is out by up to about half a percent, which on a transatlantic route is some fifteen kilometres, and the course by a few tenths of a degree; both are reported rather than described. For the real distance use the geodesic tool. Antipodal points have no unique great circle and coincident points no course.",
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED"],
    model: "Central angle σ = atan2(√((cos φ₂ sin Δλ)² + (cos φ₁ sin φ₂ − sin φ₁ cos φ₂ cos Δλ)²), sin φ₁ sin φ₂ + cos φ₁ cos φ₂ cos Δλ), distance R·σ; initial course atan2(sin Δλ cos φ₂, cos φ₁ sin φ₂ − sin φ₁ cos φ₂ cos Δλ), and the final course from the reverse course plus 180°. Compared with Karney (2013) on WGS 84",
    accuracy: "Exact on the sphere, at every distance (the atan2 form keeps full precision near 0 and 180°). On the Earth the sphere is off by up to about 0.5% in distance and a few tenths of a degree in course; both are reported",
    references: &[IUGG_MEAN_RADIUS, KARNEY],
    examples: &[Example {
        id: "primary",
        title: "JFK to Heathrow on the mean-radius sphere",
        input: JFK_LHR,
        source: "Worked from the spherical formulas; the distance equals the haversine scenario's 5,540,019 m",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "line-geodesic",
        map: &[("distance", "distance")],
    }],
    related: &[
        Related {
            id: "navigation.geodesic.inverse",
            reason: "alternative",
        },
        Related {
            id: "navigation.geodesic.spherical-direct",
            reason: "inverse",
        },
        Related {
            id: "navigation.geodesic.vincenty-inverse",
            reason: "alternative",
        },
    ],
    limitation: Some(SPHERE_LIMITATION),
    sentence: "The great circle is {distance} long, starting on {initial_course}; the ellipsoid differs by {abs(difference)}.",
    limits: &[("batchRows", 10_000)],
    run: run_inverse,
    ..ToolDef::BLANK
};

fn run_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let p = two_points(ctx)?;
    let r = radius(ctx)?;
    let (sigma, c1) = sphere_inverse(p.0, p.1, p.2, p.3);
    let (_, back) = sphere_inverse(p.2, p.3, p.0, p.1);
    let c2 = wrap_azimuth(back + 180.0);
    let (ell, az1, _, _): (f64, f64, f64, f64) = Geodesic::wgs84().inverse(p.0, p.1, p.2, p.3);
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Central angle",
            "σ = atan2(√(…), sin φ₁ sin φ₂ + cos φ₁ cos φ₂ cos Δλ)",
            format!(
                "from {}°, {}° to {}°, {}°",
                n(p.0, 4),
                n(p.1, 4),
                n(p.2, 4),
                n(p.3, 4)
            ),
            format!("{} rad", n(sigma, 9)),
        );
        ctx.step(
            "Distance",
            "R × σ",
            format!("{} m × {}", n(r, 3), n(sigma, 9)),
            display::quantity(r * sigma / 1000.0, "km", DIST_P, fmt),
        );
    }
    Ok(Json::obj([
        ("distance", ctx.out("distance", meters(r * sigma))),
        ("initial_course", ctx.out("initial_course", deg(c1))),
        ("final_course", ctx.out("final_course", deg(c2))),
        (
            "ellipsoidal_distance",
            ctx.out("ellipsoidal_distance", meters(ell)),
        ),
        ("difference", ctx.out("difference", meters(ell - r * sigma))),
        (
            "course_difference",
            ctx.out("course_difference", deg(wrap_lon(c1 - az1))),
        ),
    ]))
}

// ---------------------------------------------------------------- direct

pub static SPHERICAL_DIRECT: ToolDef = ToolDef {
    id: "navigation.geodesic.spherical-direct",
    title: "Great-circle destination (spherical)",
    summary: "Where a great circle from a start point on a course takes you after a distance on a sphere, and how far that is from the ellipsoidal answer.",
    aliases: &[
        "great circle destination",
        "destination point on a sphere",
        "spherical direct",
    ],
    keywords: &[
        "great circle",
        "sphere",
        "destination",
        "course",
        "distance",
        "spherical",
    ],
    inputs: &[
        LAT1,
        LON1,
        course("course", "Course", "True, like 051.4 deg")
            .required()
            .core(),
        km("distance", "Distance", "Like 5540.019 km")
            .required()
            .core(),
        RADIUS,
    ],
    outputs: &[
        point::lat_field("lat2", "Destination latitude").precision(Precision::Decimals(7)),
        point::lon_field("lon2", "Destination longitude").precision(Precision::Decimals(7)),
        course("final_course", "Final course", "At the destination, true"),
        metres(
            "offset",
            "Distance from the ellipsoidal destination",
            "Where the Karney direct on WGS 84 ends, measured on the ellipsoid",
        ),
    ],
    errors: &[ErrorCode::InvalidInput],
    stability: Stability::Stable,
    when_to_use: "Use this to reproduce where a great-circle calculation says you end up: from a point, on a course, for a distance, on a sphere. The companion to the spherical inverse, and for the same reason -- matching what other software produces rather than bettering it.",
    limitations: "The sphere, again deliberately. The `offset` says how far this destination is from the ellipsoidal one for the same course and distance, and it grows with distance rather than staying fixed. Going more than half a circumference wraps round the far side, which is correct and rarely what was meant. For the real destination use the geodesic direct tool.",
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED"],
    model: "φ₂ = asin(sin φ₁ cos δ + cos φ₁ sin δ cos θ), λ₂ = λ₁ + atan2(sin θ sin δ cos φ₁, cos δ − sin φ₁ sin φ₂), with δ = distance ÷ R; the final course from the reverse course plus 180°. The ellipsoidal destination is Karney's direct on WGS 84",
    accuracy: "Exact on the sphere; the offset shows how far the sphere lands from the ellipsoidal destination",
    references: &[IUGG_MEAN_RADIUS, KARNEY],
    examples: &[Example {
        id: "primary",
        title: "1,000 km on course 045° from 40° N, 74° W",
        input: r#"{"lat1":40,"lon1":-74,"course":"45 deg","distance":"1000 km"}"#,
        source: "Worked from the spherical formulas on R1, with the ellipsoidal end from the geodesic direct",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "navigation.geodesic.direct",
            reason: "alternative",
        },
        Related {
            id: "navigation.geodesic.spherical-inverse",
            reason: "inverse",
        },
        Related {
            id: "navigation.geodesic.vincenty-direct",
            reason: "alternative",
        },
    ],
    limitation: Some(SPHERE_LIMITATION),
    sentence: "The great circle ends at {lat2}, {lon2}, {offset} from where the ellipsoid puts it.",
    limits: &[("batchRows", 10_000)],
    run: run_direct,
    ..ToolDef::BLANK
};

fn run_direct(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (la, lo) = point::read(ctx, "lat1", "lon1")?;
    let c = wrap_azimuth(ctx.req_quantity("course")?.to(unit(QT::Angle, "deg")));
    let s = ctx.req_quantity("distance")?.base();
    let r = radius(ctx)?;
    if s.is_nan() || s < 0.0 {
        return Err(ToolError::invalid(
            "/distance",
            "The distance must be zero or more.",
        ));
    }
    let (la2, lo2) = sphere_direct(la, lo, c, s / r);
    let (_, back) = sphere_inverse(la2, lo2, la, lo);
    let g = Geodesic::wgs84();
    let (ela, elo): (f64, f64) = g.direct(la, lo, c, s);
    let offset: f64 = g.inverse(la2, lo2, ela, elo);
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Central angle",
            "δ = distance ÷ R",
            format!("{} m ÷ {} m", n(s, 3), n(r, 3)),
            format!("{} rad", n(s / r, 9)),
        );
        ctx.step(
            "Destination latitude",
            "φ₂ = asin(sin φ₁ cos δ + cos φ₁ sin δ cos θ)",
            format!("φ₁ = {}°, θ = {}°", n(la, 6), n(c, 6)),
            display::quantity(la2, "deg", Precision::Decimals(7), fmt),
        );
    }
    Ok(Json::obj([
        ("lat2", ctx.out("lat2", deg(la2))),
        ("lon2", ctx.out("lon2", deg(lo2))),
        (
            "final_course",
            ctx.out("final_course", deg(wrap_azimuth(back + 180.0))),
        ),
        ("offset", ctx.out("offset", meters(offset))),
    ]))
}

// ---------------------------------------------------------------- intermediate point

pub static INTERMEDIATE: ToolDef = ToolDef {
    id: "navigation.geodesic.intermediate-point",
    title: "Point part way along a great circle",
    summary: "The point a given fraction of the way between two points along the great circle on a sphere, beside the same fraction along the ellipsoidal geodesic.",
    aliases: &[
        "intermediate point",
        "fraction along great circle",
        "point along route",
        "spherical midpoint",
    ],
    keywords: &[
        "intermediate",
        "fraction",
        "great circle",
        "interpolate",
        "sphere",
        "midpoint",
    ],
    inputs: &[
        LAT1,
        LON1,
        LAT2,
        LON2,
        Field::new(
            "fraction",
            "Fraction of the way",
            "0 at the start, 1 at the end, like 0.25",
            Kind::Number { min: 0.0, max: 1.0 },
        )
        .required()
        .core(),
    ],
    outputs: &[
        point::lat_field("lat", "Latitude").precision(Precision::Decimals(7)),
        point::lon_field("lon", "Longitude").precision(Precision::Decimals(7)),
        point::lat_field("ellipsoidal_lat", "Ellipsoidal latitude")
            .precision(Precision::Decimals(7)),
        point::lon_field("ellipsoidal_lon", "Ellipsoidal longitude")
            .precision(Precision::Decimals(7)),
        metres(
            "offset",
            "Distance between the two",
            "The spherical point from the ellipsoidal one",
        ),
    ],
    errors: &[ErrorCode::NoSolution, ErrorCode::InvalidInput],
    stability: Stability::Stable,
    when_to_use: "Use this for a point a given fraction of the way along a route -- a waypoint every tenth, a reporting point, a place to draw a label. It returns both the spherical answer most code produces and the ellipsoidal one that is actually right, with the distance between them, so you can see what the approximation costs on this route.",
    limitations: "The two answers differ, and on a long route they differ by kilometres: the spherical one is exact on a sphere and the Earth is not one. Use the ellipsoidal pair unless you are reproducing someone else's spherical figure. Antipodal points have no unique path between them -- every great circle joins them -- so there is no intermediate point to give. And a fraction of the distance is not a fraction of the flight time, which depends on the wind.",
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED"],
    model: "Spherical linear interpolation of the unit vectors: A = sin((1 − f)δ) ÷ sin δ, B = sin(fδ) ÷ sin δ, p = A·p₁ + B·p₂, with δ the central angle; the ellipsoidal point is f × s12 along the Karney geodesic on WGS 84",
    accuracy: "Exact on the sphere; the offset shows the approximation. Undefined for antipodal points, where every great circle joins them",
    references: &[KARNEY],
    examples: &[Example {
        id: "primary",
        title: "A quarter of the way from JFK to Heathrow",
        input: r#"{"lat1":40.6413,"lon1":-73.7781,"lat2":51.47,"lon2":-0.4543,"fraction":0.25}"#,
        source: "Worked by spherical interpolation, with the ellipsoidal point from the geodesic direct at a quarter of the distance",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "navigation.geodesic.midpoint",
            reason: "alternative",
        },
        Related {
            id: "navigation.geodesic.spherical-inverse",
            reason: "parent",
        },
        Related {
            id: "navigation.geodesic.direct",
            reason: "alternative",
        },
    ],
    limitation: Some(SPHERE_LIMITATION),
    sentence: "The point is at {lat}, {lon}, {offset} from the same fraction along the ellipsoidal geodesic.",
    limits: &[("batchRows", 10_000)],
    run: run_intermediate,
    ..ToolDef::BLANK
};

fn run_intermediate(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let p = two_points(ctx)?;
    let f = ctx.number("fraction")?.expect("required");
    let (sigma, _) = sphere_inverse(p.0, p.1, p.2, p.3);
    if (core::f64::consts::PI - sigma).abs() < 1e-12 {
        return Err(ToolError::new(
            ErrorCode::NoSolution,
            "The points are antipodal, so every great circle joins them and the point part way is undefined.",
        ));
    }
    let (la, lo) = if sigma == 0.0 {
        (p.0, p.1)
    } else {
        let (a, b) = (
            sin((1.0 - f) * sigma) / sin(sigma),
            sin(f * sigma) / sin(sigma),
        );
        let (v1, v2) = (vec3(p.0, p.1), vec3(p.2, p.3));
        let v = [0, 1, 2].map(|i| a * v1[i] + b * v2[i]);
        (
            atan2(v[2], sqrt(v[0] * v[0] + v[1] * v[1])).to_degrees(),
            atan2(v[1], v[0]).to_degrees(),
        )
    };
    let g = Geodesic::wgs84();
    let (s12, az1, _, _): (f64, f64, f64, f64) = g.inverse(p.0, p.1, p.2, p.3);
    let (ela, elo): (f64, f64) = g.direct(p.0, p.1, az1, f * s12);
    let offset: f64 = g.inverse(la, lo, ela, elo);
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Central angle",
            "δ between the two points",
            format!(
                "from {}°, {}° to {}°, {}°",
                n(p.0, 4),
                n(p.1, 4),
                n(p.2, 4),
                n(p.3, 4)
            ),
            format!("{} rad", n(sigma, 9)),
        );
        ctx.step(
            "Interpolated latitude",
            "atan2(z, √(x² + y²)) of A·p₁ + B·p₂",
            format!("f = {}", n(f, 4)),
            display::quantity(la, "deg", Precision::Decimals(7), fmt),
        );
    }
    Ok(Json::obj([
        ("lat", ctx.out("lat", deg(la))),
        ("lon", ctx.out("lon", deg(lo))),
        ("ellipsoidal_lat", ctx.out("ellipsoidal_lat", deg(ela))),
        ("ellipsoidal_lon", ctx.out("ellipsoidal_lon", deg(elo))),
        ("offset", ctx.out("offset", meters(offset))),
    ]))
}
