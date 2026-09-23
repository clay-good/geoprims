//! Navigation: geodesics on the ellipsoid (add-navigation-and-geometry). Karney
//! (2013) is the default; Vincenty and haversine are comparison tools that
//! always report their difference from Karney.

pub mod intersect;
pub mod los;
pub mod rhumb;
pub mod rings;
pub mod route;
pub mod sphere;
pub mod vector;
pub mod vincenty;
pub mod waypoints;

use geographiclib_rs::{DirectGeodesic, Geodesic, InverseGeodesic};
use gp_base::ErrorCode;
use gp_base::angle::wrap_azimuth;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{
    Ctx, Example, Field, Kind, Layer, Limitation, Precision, Q, Reference, Registry, Related,
    Stability, ToolDef,
};
use gp_base::units::{self, Quantity as QT, Unit};
use gp_geo::ellipsoid::{self, Ellipsoid};
use gp_geo::point;

const KARNEY: Reference = Reference {
    title: "Algorithms for geodesics",
    issuer: "Karney, C. F. F., Journal of Geodesy",
    year: 2013,
    edition: "Vol. 87, No. 1",
    locator: "pp. 43-55",
    url: "https://doi.org/10.1007/s00190-012-0578-z",
};
const GEOGRAPHICLIB: Reference = Reference {
    title: "GeographicLib geodesic test data (GeodTest.dat) and GeodSolve",
    issuer: "Karney, C. F. F., GeographicLib",
    year: 2011,
    edition: "GeographicLib 2.x",
    locator: "Geodesic test data, 500,000 geodesics on WGS 84",
    url: "https://geographiclib.sourceforge.io/C++/doc/geodesic.html#testgeod",
};
const VINCENTY: Reference = Reference {
    title: "Direct and inverse solutions of geodesics on the ellipsoid with application of nested equations",
    issuer: "Vincenty, T., Survey Review",
    year: 1975,
    edition: "Vol. 23, No. 176",
    locator: "pp. 88-93",
    url: "https://doi.org/10.1179/sre.1975.23.176.88",
};
const IUGG_MEAN_RADIUS: Reference = Reference {
    title: "Geodetic Reference System 1980 (mean radius R1 = (2a + b)/3)",
    issuer: "Moritz, H., International Association of Geodesy",
    year: 2000,
    edition: "Journal of Geodesy 74(1)",
    locator: "pp. 128-133",
    url: "https://doi.org/10.1007/s001900050278",
};

/// The IUGG mean Earth radius R1 = (2a + b)/3 for WGS 84, meters.
pub const R1: f64 = 6_371_008.771;

fn unit(q: QT, s: &str) -> &'static Unit {
    units::by_symbol(q, s).expect("registered unit")
}

fn meters(v: f64) -> Q {
    Q {
        value: v,
        unit: unit(QT::Distance, "m"),
    }
}

fn deg(v: f64) -> Q {
    Q {
        value: v,
        unit: unit(QT::Angle, "deg"),
    }
}

const LAT1: Field = point::lat_field("lat1", "Start latitude");
const LON1: Field = point::lon_field("lon1", "Start longitude");
const LAT2: Field = point::lat_field("lat2", "End latitude");
const LON2: Field = point::lon_field("lon2", "End longitude");
const E: [Field; 3] = ellipsoid::FIELDS;
const LINE: &[Layer] = &[Layer {
    kind: "line-geodesic",
    map: &[("distance", "distance")],
}];
const WARNINGS: &[&str] = &[
    "INPUT_NORMALIZED",
    "AZIMUTH_UNDEFINED",
    "AZIMUTH_NOT_UNIQUE",
    "UNIT_ASSUMED",
    "EXPERIMENTAL_TOOL",
];
/// The same list for a tool that is past the stable bar.
const STABLE_WARNINGS: &[&str] = &[
    "INPUT_NORMALIZED",
    "AZIMUTH_UNDEFINED",
    "AZIMUTH_NOT_UNIQUE",
    "UNIT_ASSUMED",
];
const JFK_LHR: &str = r#"{"lat1":40.6413,"lon1":-73.7781,"lat2":51.47,"lon2":-0.4543}"#;
const DIST_P: Precision = Precision::Decimals(3);
const AZ_P: Precision = Precision::Decimals(7);

fn setup(ctx: &mut Ctx) -> Result<(Ellipsoid, Geodesic), ToolError> {
    let e = Ellipsoid::from_ctx(ctx)?;
    let g = e.geodesic()?;
    Ok((e, g))
}

fn two_points(ctx: &mut Ctx) -> Result<(f64, f64, f64, f64), ToolError> {
    let (la1, lo1) = point::read(ctx, "lat1", "lon1")?;
    let (la2, lo2) = point::read(ctx, "lat2", "lon2")?;
    Ok((la1, lo1, la2, lo2))
}

/// Karney distance, azimuths, and arc, with the uniqueness warnings.
fn karney(
    ctx: &mut Ctx,
    g: &Geodesic,
    p: (f64, f64, f64, f64),
) -> (f64, f64, f64, f64, f64, f64, f64, f64) {
    let r: (f64, f64, f64, f64, f64, f64, f64, f64) = g.inverse(p.0, p.1, p.2, p.3);
    if r.0 == 0.0 {
        ctx.warnings.push(Warning::new("AZIMUTH_UNDEFINED", "The points coincide, so the direction between them is undefined; the azimuths shown are a convention."));
    } else if (r.7 - 180.0).abs() < 1e-12 {
        ctx.warnings.push(Warning::new(
            "AZIMUTH_NOT_UNIQUE",
            "The points are antipodal: infinitely many geodesics of this length connect them, so the azimuths shown are one of many.",
        ));
    }
    r
}

pub static INVERSE: ToolDef = ToolDef {
    id: "navigation.geodesic.inverse",
    stability: gp_base::tool::Stability::Stable,
    title: "Distance between two points (geodesic)",
    summary: "The shortest distance and the start and end courses between two points on the WGS 84 ellipsoid (or any ellipsoid), exact to nanometers with Karney's algorithm.",
    aliases: &[
        "distance between two points",
        "geodesic distance",
        "great circle distance calculator",
        "inverse geodesic",
        "how far",
        "how far apart",
    ],
    keywords: &[
        "distance", "far", "apart", "bearing", "azimuth", "geodesic", "inverse", "Karney", "WGS 84",
    ],
    inputs: &[LAT1, LON1, LAT2, LON2, E[0], E[1], E[2]],
    outputs: &[
        Field::new(
            "distance",
            "Distance",
            "Geodesic distance s12",
            Kind::Quantity {
                q: QT::Distance,
                unit: "km",
            },
        )
        .precision(DIST_P),
        Field::new(
            "azimuth1",
            "Initial course",
            "Azimuth at the start, clockwise from true north",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(AZ_P)
        .angle_range("[0,360)"),
        Field::new(
            "azimuth2",
            "Final course",
            "Azimuth at the end, clockwise from true north",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(AZ_P)
        .angle_range("[0,360)"),
        Field::new(
            "arc",
            "Arc length",
            "Spherical arc a12 on the auxiliary sphere",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(9))
        .angle_range("unbounded"),
        Field::new(
            "reduced_length",
            "Reduced length m12",
            "Sensitivity of the end point to the start azimuth",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Significant(10)),
        Field::new(
            "scale12",
            "Geodesic scale M12",
            "Dimensionless",
            Kind::Number {
                min: -1e3,
                max: 1e3,
            },
        )
        .precision(Precision::Significant(10)),
        Field::new(
            "scale21",
            "Geodesic scale M21",
            "Dimensionless",
            Kind::Number {
                min: -1e3,
                max: 1e3,
            },
        )
        .precision(Precision::Significant(10)),
        Field::new(
            "area",
            "Area under the geodesic S12",
            "Between the geodesic and the equator",
            Kind::Quantity {
                q: QT::Area,
                unit: "m2",
            },
        )
        .precision(Precision::Significant(10)),
    ],
    errors: &[ErrorCode::Unsupported],
    warnings: WARNINGS,
    model: "Karney (2013) geodesic on WGS 84",
    accuracy: "About 15 nanometers on WGS 84; converges for every pair, including antipodal and polar points",
    when_to_use: "Use this when you need the distance between two coordinates and the courses to fly or walk between them: the shortest path on the ellipsoid, with the azimuth at each end. This is the tool behind leg distances, ranges, and any comparison of one route against another, and it holds for pairs that defeat simpler formulas, including nearly antipodal ones.",
    limitations: "The distance is along the surface of the ellipsoid: it is not a road or track distance, it takes no account of terrain or height, and it is not the length of a constant-heading course, which is a rhumb line. The azimuths are true and they differ at the two ends, because a geodesic changes direction as it goes.",
    references: &[KARNEY, GEOGRAPHICLIB],
    examples: &[Example {
        id: "primary",
        title: "New York JFK to London Heathrow",
        input: JFK_LHR,
        source: "add-navigation-and-geometry geodesic scenario: 5,554,908.791 m, 51.3816479°, 107.9828291°",
    }],
    primary_example: "primary",
    visualization: LINE,
    related: &[
        Related {
            id: "navigation.geodesic.direct",
            reason: "inverse",
        },
        Related {
            id: "navigation.geodesic.haversine",
            reason: "alternative",
        },
        Related {
            id: "navigation.geodesic.vincenty-inverse",
            reason: "alternative",
        },
        Related {
            id: "navigation.geodesic.midpoint",
            reason: "next",
        },
    ],
    sentence: "The shortest distance is {distance}, leaving on a course of {azimuth1} and arriving on {azimuth2}.",
    limits: &[("batchRows", 10_000)],
    run: run_inverse,
    ..ToolDef::BLANK
};

/// The exact method for a strongly flattened ellipsoid (0.02 < |f| ≤ 0.5).
fn exact_for(e: &Ellipsoid) -> Option<gp_geo::exact::Exact> {
    (e.f.abs() > ellipsoid::SERIES_LIMIT && e.f.abs() <= gp_geo::exact::MAX_F)
        .then(|| gp_geo::exact::Exact::new(e.a, e.f))
}

fn exact_model(e: &Ellipsoid) -> String {
    format!(
        "Exact geodesic (the integrals behind GeographicLib's GeodesicExact, by Gauss-Legendre quadrature) on {}",
        e.describe()
    )
}

fn run_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let p = two_points(ctx)?;
    let e = Ellipsoid::from_ctx(ctx)?;
    let ((s12, az1, az2, m12, big_m12, big_m21, s_12, a12), model) = match exact_for(&e) {
        Some(x) => {
            let r = x.inverse(p.0, p.1, p.2, p.3).ok_or_else(|| {
                ToolError::new(
                    ErrorCode::Unsupported,
                    "These points are nearly antipodal on a strongly flattened ellipsoid, where the exact method here does not settle which geodesic is shortest.",
                )
                .at("/lat2")
            })?;
            (
                (
                    r.s12, r.azi1, r.azi2, r.m12, r.big_m12, r.big_m21, r.area12, r.a12,
                ),
                exact_model(&e),
            )
        }
        None => {
            let g = e.geodesic()?;
            (
                karney(ctx, &g, p),
                format!("Karney (2013) geodesic on {}", e.describe()),
            )
        }
    };
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        // Karney's method solves the auxiliary sphere iteratively; the honest
        // work is the geometry it settles on, not its iterations.
        ctx.step(
            "Arc between the points",
            "σ12, the angle the geodesic subtends on the auxiliary sphere",
            format!(
                "{}°, {}° to {}°, {}°",
                n(p.0, 6),
                n(p.1, 6),
                n(p.2, 6),
                n(p.3, 6)
            ),
            format!("{}°", n(a12, 6)),
        );
        ctx.step(
            "Courses at each end",
            "α1 leaving and α2 arriving, which differ because the meridians converge",
            format!("{}° and {}°", n(az1, 4), n(az2, 4)),
            format!("{}°", n(az1, 4)),
        );
        ctx.step(
            "Distance",
            "s12 along the ellipsoid, from the arc and the ellipsoid's shape",
            format!("{}° of arc on {}", n(a12, 6), e.name),
            format!("{} km", n(s12 / 1000.0, 3)),
        );
    }
    ctx.model = Some(model);
    Ok(Json::obj([
        ("distance", ctx.out("distance", meters(s12))),
        ("azimuth1", ctx.out("azimuth1", deg(wrap_azimuth(az1)))),
        ("azimuth2", ctx.out("azimuth2", deg(wrap_azimuth(az2)))),
        ("arc", ctx.out("arc", deg(a12))),
        (
            "reduced_length",
            ctx.out(
                "reduced_length",
                Q {
                    value: m12,
                    unit: unit(QT::Length, "m"),
                },
            ),
        ),
        ("scale12", Json::Num(big_m12)),
        ("scale21", Json::Num(big_m21)),
        (
            "area",
            ctx.out(
                "area",
                Q {
                    value: s_12,
                    unit: unit(QT::Area, "m2"),
                },
            ),
        ),
    ]))
}

const DEST_OUTPUTS: &[Field] = &[
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
    Field::new(
        "azimuth2",
        "Final course",
        "Azimuth at the destination",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(AZ_P)
    .angle_range("[0,360)"),
];

pub static DIRECT: ToolDef = ToolDef {
    id: "navigation.geodesic.direct",
    stability: gp_base::tool::Stability::Stable,
    title: "Destination from a start, course, and distance (geodesic)",
    summary: "Where you end up after traveling a distance on an initial course along the ellipsoid, any length, with Karney's algorithm.",
    aliases: &[
        "destination point",
        "direct geodesic",
        "point at distance and bearing",
    ],
    keywords: &[
        "destination",
        "bearing",
        "distance",
        "geodesic",
        "direct",
        "project point",
    ],
    inputs: &[
        LAT1,
        LON1,
        Field::new(
            "azimuth",
            "Initial course",
            "Degrees clockwise from true north, like 51",
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
            "Like 1000 km; negative goes backward",
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
    outputs: DEST_OUTPUTS,
    errors: &[ErrorCode::Unsupported],
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Karney (2013) geodesic on WGS 84",
    accuracy: "About 15 nanometers on WGS 84, for any distance",
    when_to_use: "Use this when you have a starting point, a bearing, and a distance and need the point you arrive at: laying out a leg, projecting a position from a fix, placing a point at a known range and bearing, or building a circle of points around a center. It follows the shortest path on the ellipsoid, at any distance.",
    limitations: "It answers geometry on an ellipsoid, not travel: the azimuth is the initial one and it changes along the path, so a course held constant on a compass is a rhumb line and belongs in that tool. Heights are not part of it, a geodesic is not a route around terrain or airspace, and the azimuth here is true rather than magnetic.",
    references: &[KARNEY, GEOGRAPHICLIB],
    examples: &[Example {
        id: "primary",
        title: "1,000 km from JFK on a course of 051°",
        input: r#"{"lat1":40.6413,"lon1":-73.7781,"azimuth":51,"distance":"1000 km"}"#,
        source: "add-navigation-and-geometry geodesic scenario: (45.8920808°, -63.7549563°), final course 57.886373°",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "point",
        map: &[("lat", "lat2"), ("lon", "lon2")],
    }],
    related: &[
        Related {
            id: "navigation.geodesic.inverse",
            reason: "inverse",
        },
        Related {
            id: "navigation.rhumb.direct",
            reason: "alternative",
        },
        Related {
            id: "navigation.geodesic.waypoints",
            reason: "next",
        },
    ],
    sentence: "The destination is {lat2}, {lon2}, arriving on a course of {azimuth2}.",
    limits: &[("batchRows", 10_000)],
    run: run_direct,
    ..ToolDef::BLANK
};

fn run_direct(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (la1, lo1) = point::read(ctx, "lat1", "lon1")?;
    let az = ctx.req_quantity("azimuth")?.to(unit(QT::Angle, "deg"));
    let s = ctx.req_quantity("distance")?.to(unit(QT::Distance, "m"));
    let e = Ellipsoid::from_ctx(ctx)?;
    let (la2, lo2, az2) = match exact_for(&e) {
        Some(x) => {
            let r = x.direct(la1, lo1, az, s);
            ctx.model = Some(exact_model(&e));
            (r.lat2, r.lon2, r.azi2)
        }
        None => {
            let g = e.geodesic()?;
            ctx.model = Some(format!("Karney (2013) geodesic on {}", e.describe()));
            g.direct(la1, lo1, az, s)
        }
    };
    Ok(Json::obj([
        ("lat2", ctx.out("lat2", deg(la2))),
        ("lon2", ctx.out("lon2", deg(gp_base::angle::wrap_lon(lo2)))),
        ("azimuth2", ctx.out("azimuth2", deg(wrap_azimuth(az2)))),
    ]))
}

pub static HAVERSINE: ToolDef = ToolDef {
    id: "navigation.geodesic.haversine",
    stability: gp_base::tool::Stability::Stable,
    title: "Haversine distance (spherical)",
    summary: "The great-circle distance on a sphere by the haversine formula, with its error against the ellipsoidal geodesic shown.",
    aliases: &["haversine calculator", "great circle distance on a sphere"],
    keywords: &["haversine", "great circle", "sphere", "spherical distance"],
    inputs: &[
        LAT1,
        LON1,
        LAT2,
        LON2,
        Field::new(
            "radius",
            "Sphere radius",
            "Default 6371008.771 m, the IUGG mean radius R1",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        ),
    ],
    outputs: &[
        Field::new(
            "distance",
            "Haversine distance",
            "Great-circle distance on the sphere",
            Kind::Quantity {
                q: QT::Distance,
                unit: "km",
            },
        )
        .precision(DIST_P),
        Field::new(
            "ellipsoidal_distance",
            "Ellipsoidal distance",
            "Karney geodesic on WGS 84",
            Kind::Quantity {
                q: QT::Distance,
                unit: "km",
            },
        )
        .precision(DIST_P),
        Field::new(
            "difference",
            "Difference",
            "Ellipsoidal minus haversine",
            Kind::Quantity {
                q: QT::Distance,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "difference_percent",
            "Difference (percent)",
            "Relative to the ellipsoidal distance",
            Kind::Number {
                min: -100.0,
                max: 100.0,
            },
        )
        .precision(Precision::Decimals(2)),
    ],
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Haversine great circle on a sphere of radius R1; compared with Karney (2013) on WGS 84",
    accuracy: "Exact on the sphere. On the Earth the sphere itself is off by up to about 0.5%; the difference is reported",
    when_to_use: "Use this when you want the classic spherical distance, either because a specification calls for the haversine formula, because you are checking another system that uses it, or because you want to see how far the sphere is from the ellipsoid for your pair of points. The difference against the exact geodesic is reported beside the answer.",
    limitations: "A sphere is not the Earth: this can differ from the ellipsoidal distance by up to about half a percent, which is kilometers on a long leg, and the error depends on latitude and direction. Use the geodesic tool when the distance is the answer rather than the method. Course angles from a sphere carry the same approximation.",
    references: &[IUGG_MEAN_RADIUS, KARNEY],
    examples: &[Example {
        id: "primary",
        title: "JFK to Heathrow on the mean-radius sphere",
        input: JFK_LHR,
        source: "add-navigation-and-geometry scenario: about 5,540,019 m, 14,890 m (0.27%) shorter than the geodesic",
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
            id: "navigation.geodesic.direct",
            reason: "next",
        },
        Related {
            id: "navigation.route.time-speed-distance",
            reason: "next",
        },
    ],
    limitation: Some(Limitation {
        simplification: "Measures on a sphere, not on the ellipsoid the Earth actually is.",
        instead: "Use the geodesic distance for anything you act on: it is exact on WGS 84, and this tool reports how far the sphere puts it wrong for your own two points.",
        governs: "Karney (2013), the geodesic algorithm the geodesic tools use.",
    }),
    sentence: "The haversine distance is {distance}, {abs(difference)} ({abs(difference_percent)}%) {if difference > 0}shorter{else}longer{/if} than the ellipsoidal geodesic.",
    limits: &[("batchRows", 10_000)],
    run: run_haversine,
    ..ToolDef::BLANK
};

fn run_haversine(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let p = two_points(ctx)?;
    let r = ctx
        .quantity("radius")?
        .map_or(R1, |q| q.to(unit(QT::Length, "m")));
    if r <= 0.0 {
        return Err(ToolError::invalid(
            "/radius",
            "The radius must be positive.",
        ));
    }
    let (p1, p2) = (p.0.to_radians(), p.2.to_radians());
    let dphi = p2 - p1;
    let dl = (p.3 - p.1).to_radians();
    let h =
        libm::sin(dphi / 2.0).powi(2) + libm::cos(p1) * libm::cos(p2) * libm::sin(dl / 2.0).powi(2);
    let d = 2.0 * r * libm::asin(libm::sqrt(h.min(1.0)));
    let ell: f64 = Geodesic::wgs84().inverse(p.0, p.1, p.2, p.3);
    let diff = ell - d;
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        let km = |x: f64| format!("{} km", n(x / 1000.0, 3));
        ctx.step(
            "Half-chord squared",
            "h = sin²(Δφ/2) + cos φ₁ × cos φ₂ × sin²(Δλ/2)",
            format!(
                "Δφ = {}°, Δλ = {}°, φ₁ = {}°, φ₂ = {}°",
                n(p.2 - p.0, 6),
                n(p.3 - p.1, 6),
                n(p.0, 6),
                n(p.2, 6)
            ),
            n(h, 9),
        );
        ctx.step(
            "Great-circle distance",
            "d = 2 × R × asin(√h)",
            format!("2 × {} × asin(√{})", km(r), n(h, 9)),
            km(d),
        );
    }
    Ok(Json::obj([
        ("distance", ctx.out("distance", meters(d))),
        (
            "ellipsoidal_distance",
            ctx.out("ellipsoidal_distance", meters(ell)),
        ),
        ("difference", ctx.out("difference", meters(diff))),
        (
            "difference_percent",
            Json::Num(if ell == 0.0 { 0.0 } else { diff / ell * 100.0 }),
        ),
    ]))
}

pub static VINCENTY_INVERSE: ToolDef = ToolDef {
    id: "navigation.geodesic.vincenty-inverse",
    title: "Vincenty inverse (legacy)",
    summary: "Distance and courses by Vincenty's 1975 method, for checking legacy software; reports its difference from Karney and fails honestly near antipodes.",
    aliases: &["vincenty distance", "vincenty formula"],
    keywords: &["Vincenty", "legacy", "inverse", "distance"],
    inputs: &[LAT1, LON1, LAT2, LON2, E[0], E[1], E[2]],
    outputs: &[
        Field::new(
            "distance",
            "Distance",
            "Vincenty inverse distance",
            Kind::Quantity {
                q: QT::Distance,
                unit: "km",
            },
        )
        .precision(Precision::Decimals(6)),
        Field::new(
            "azimuth1",
            "Initial course",
            "Vincenty azimuth at the start",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(AZ_P)
        .angle_range("[0,360)"),
        Field::new(
            "azimuth2",
            "Final course",
            "Vincenty azimuth at the end",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(AZ_P)
        .angle_range("[0,360)"),
        Field::new(
            "karney_difference",
            "Difference from Karney",
            "Vincenty minus Karney distance",
            Kind::Quantity {
                q: QT::Length,
                unit: "mm",
            },
        )
        .precision(Precision::Significant(3)),
    ],
    errors: &[ErrorCode::DidNotConverge, ErrorCode::Unsupported],
    stability: Stability::Stable,
    when_to_use: "Use this when the answer has to be Vincenty's: checking surveying software, a GIS library or a published figure from the last forty years, all of which use his 1975 method. The difference from the exact modern answer is reported alongside, so you can tell whether a discrepancy is the algorithm or an error.",
    limitations: "Vincenty's inverse does not converge for nearly antipodal points -- a known property of the method, not of this implementation -- and the tool says so rather than returning a plausible number from a truncated loop. Its series is truncated, so it carries about half a millimetre against the exact answer. For new work use the geodesic inverse, which converges everywhere and is exact to fifteen nanometres; this one is for agreeing with the past.",
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED"],
    model: "Vincenty (1975) inverse, tolerance 1e-12 rad, at most 200 iterations",
    accuracy: "About 0.5 mm where it converges; fails to converge for nearly antipodal points",
    references: &[VINCENTY, KARNEY],
    examples: &[Example {
        id: "primary",
        title: "JFK to Heathrow",
        input: JFK_LHR,
        source: "add-navigation-and-geometry scenario: sub-millimeter from Karney",
    }],
    primary_example: "primary",
    visualization: LINE,
    related: &[
        Related {
            id: "navigation.geodesic.inverse",
            reason: "alternative",
        },
        Related {
            id: "navigation.geodesic.vincenty-direct",
            reason: "inverse",
        },
        Related {
            id: "navigation.geodesic.spherical-inverse",
            reason: "alternative",
        },
    ],
    sentence: "Vincenty gives {distance}, {abs(karney_difference)} from the Karney geodesic.",
    limits: &[("batchRows", 10_000)],
    run: run_vincenty_inverse,
    ..ToolDef::BLANK
};

fn run_vincenty_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let p = two_points(ctx)?;
    let (e, g) = setup(ctx)?;
    let Some((s, az1, az2)) = vincenty::inverse(e.a, e.f, p.0, p.1, p.2, p.3) else {
        return Err(ToolError::new(
            ErrorCode::DidNotConverge,
            "Vincenty's method did not converge in 200 iterations; this happens for nearly antipodal points.",
        )
        .hint("Use navigation.geodesic.inverse (Karney), which converges for every pair."));
    };
    let k: f64 = g.inverse(p.0, p.1, p.2, p.3);
    ctx.model = Some(format!(
        "Vincenty (1975) inverse on {}, tolerance 1e-12 rad, at most 200 iterations",
        e.describe()
    ));
    Ok(Json::obj([
        ("distance", ctx.out("distance", meters(s))),
        ("azimuth1", ctx.out("azimuth1", deg(wrap_azimuth(az1)))),
        ("azimuth2", ctx.out("azimuth2", deg(wrap_azimuth(az2)))),
        (
            "karney_difference",
            ctx.out(
                "karney_difference",
                Q {
                    value: s - k,
                    unit: unit(QT::Length, "m"),
                },
            ),
        ),
    ]))
}

pub static VINCENTY_DIRECT: ToolDef = ToolDef {
    id: "navigation.geodesic.vincenty-direct",
    title: "Vincenty direct (legacy)",
    summary: "The destination by Vincenty's 1975 method, for checking legacy software; reports how far it lands from the Karney destination.",
    aliases: &["vincenty destination"],
    keywords: &["Vincenty", "legacy", "direct", "destination"],
    inputs: &[
        LAT1,
        LON1,
        Field::new(
            "azimuth",
            "Initial course",
            "Degrees clockwise from true north, like 51",
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
        DEST_OUTPUTS[0],
        DEST_OUTPUTS[1],
        DEST_OUTPUTS[2],
        Field::new(
            "karney_difference",
            "Distance from the Karney destination",
            "Geodesic distance between the two destinations",
            Kind::Quantity {
                q: QT::Length,
                unit: "mm",
            },
        )
        .precision(Precision::Significant(3)),
    ],
    errors: &[ErrorCode::DidNotConverge, ErrorCode::Unsupported],
    stability: Stability::Stable,
    when_to_use: "Use this to reproduce a destination computed by Vincenty's 1975 direct method -- from a point, an azimuth and a distance -- when matching older surveying software or a published traverse. The gap from the exact answer is reported with it.",
    limitations: "The same truncated series as the inverse, about half a millimetre on a long line, which is reported beside the answer rather than left to be assumed. Unlike the inverse there is no convergence problem: nothing about going a given way for a given distance is antipodal, so the iteration on sigma always settles, usually in three or four passes. For new work use the geodesic direct tool, which is exact to about fifteen nanometres and has no series to truncate; this one exists to agree with what came before, and agreeing with it is the only reason to reach for it.",
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED"],
    model: "Vincenty (1975) direct, tolerance 1e-12 rad, at most 200 iterations",
    accuracy: "About 0.5 mm",
    references: &[VINCENTY, KARNEY],
    examples: &[Example {
        id: "primary",
        title: "1,000 km from JFK on a course of 051°",
        input: r#"{"lat1":40.6413,"lon1":-73.7781,"azimuth":51,"distance":"1000 km"}"#,
        source: "Compared with the Karney direct solution for the same inputs",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "point",
        map: &[("lat", "lat2"), ("lon", "lon2")],
    }],
    related: &[
        Related {
            id: "navigation.geodesic.direct",
            reason: "alternative",
        },
        Related {
            id: "navigation.geodesic.vincenty-inverse",
            reason: "inverse",
        },
        Related {
            id: "navigation.geodesic.spherical-direct",
            reason: "alternative",
        },
    ],
    sentence: "Vincenty lands at {lat2}, {lon2}, {karney_difference} from the Karney destination.",
    limits: &[("batchRows", 10_000)],
    run: run_vincenty_direct,
    ..ToolDef::BLANK
};

fn run_vincenty_direct(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (la1, lo1) = point::read(ctx, "lat1", "lon1")?;
    let az = ctx.req_quantity("azimuth")?.to(unit(QT::Angle, "deg"));
    let s = ctx.req_quantity("distance")?.to(unit(QT::Distance, "m"));
    let (e, g) = setup(ctx)?;
    let Some((la2, lo2, az2)) = vincenty::direct(e.a, e.f, la1, lo1, az, s) else {
        return Err(ToolError::new(
            ErrorCode::DidNotConverge,
            "Vincenty's direct method did not converge in 200 iterations.",
        )
        .hint("Use navigation.geodesic.direct (Karney)."));
    };
    let (kla, klo): (f64, f64) = g.direct(la1, lo1, az, s);
    let miss: f64 = g.inverse(la2, lo2, kla, klo);
    ctx.model = Some(format!(
        "Vincenty (1975) direct on {}, tolerance 1e-12 rad, at most 200 iterations",
        e.describe()
    ));
    Ok(Json::obj([
        ("lat2", ctx.out("lat2", deg(la2))),
        ("lon2", ctx.out("lon2", deg(gp_base::angle::wrap_lon(lo2)))),
        ("azimuth2", ctx.out("azimuth2", deg(wrap_azimuth(az2)))),
        (
            "karney_difference",
            ctx.out(
                "karney_difference",
                Q {
                    value: miss,
                    unit: unit(QT::Length, "m"),
                },
            ),
        ),
    ]))
}

pub static MIDPOINT: ToolDef = ToolDef {
    id: "navigation.geodesic.midpoint",
    title: "Midpoint on the geodesic",
    summary: "The point halfway along the shortest path between two points on the ellipsoid, and the course there.",
    aliases: &["midpoint calculator", "halfway point"],
    keywords: &["midpoint", "halfway", "geodesic", "center"],
    inputs: &[LAT1, LON1, LAT2, LON2, E[0], E[1], E[2]],
    outputs: &[
        Field::new(
            "lat",
            "Midpoint latitude",
            "Degrees, north positive",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(AZ_P)
        .angle_range("[-90,90]"),
        Field::new(
            "lon",
            "Midpoint longitude",
            "Degrees, east positive",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(AZ_P)
        .angle_range("[-180,180)"),
        Field::new(
            "azimuth",
            "Course at the midpoint",
            "Azimuth of the geodesic there",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(AZ_P)
        .angle_range("[0,360)"),
        Field::new(
            "half_distance",
            "Distance to each end",
            "Half the geodesic distance",
            Kind::Quantity {
                q: QT::Distance,
                unit: "km",
            },
        )
        .precision(DIST_P),
    ],
    errors: &[ErrorCode::Unsupported],
    stability: Stability::Stable,
    when_to_use: "Use this for the point halfway along the shortest path between two places -- a staging point, the centre of a search area, a label position for a long route. It is halfway by distance travelled, which is not where averaging the coordinates puts it.",
    limitations: "Averaging latitude and longitude is not this, and is not close: for New York to London the two are more than 700 km apart, and across the antimeridian the average lands on the wrong side of the planet. Halfway by distance is also not halfway in time or in fuel, which depend on wind and on speed. For nearly antipodal points the geodesic itself is barely determined -- many paths are almost equally short -- so the midpoint moves a long way for a small change in either end.",
    warnings: STABLE_WARNINGS,
    model: "Karney (2013) geodesic on WGS 84 (inverse, then direct to half the distance)",
    accuracy: "About 15 nanometers on WGS 84",
    references: &[KARNEY],
    examples: &[Example {
        id: "primary",
        title: "Midpoint of JFK to Heathrow",
        input: JFK_LHR,
        source: "Karney direct at half the inverse distance",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "point",
        map: &[("lat", "lat"), ("lon", "lon")],
    }],
    related: &[
        Related {
            id: "navigation.geodesic.intermediate-point",
            reason: "alternative",
        },
        Related {
            id: "navigation.geodesic.vertex",
            reason: "next",
        },
        Related {
            id: "navigation.geodesic.inverse",
            reason: "parent",
        },
    ],
    sentence: "The midpoint is {lat}, {lon}, {half_distance} from each end.",
    limits: &[("batchRows", 10_000)],
    run: run_midpoint,
    ..ToolDef::BLANK
};

fn run_midpoint(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let p = two_points(ctx)?;
    let (e, g) = setup(ctx)?;
    let (s12, az1, ..) = karney(ctx, &g, p);
    let (la, lo, az): (f64, f64, f64) = g.direct(p.0, p.1, az1, s12 / 2.0);
    ctx.model = Some(format!(
        "Karney (2013) geodesic on {} (inverse, then direct to half the distance)",
        e.describe()
    ));
    Ok(Json::obj([
        ("lat", ctx.out("lat", deg(la))),
        ("lon", ctx.out("lon", deg(gp_base::angle::wrap_lon(lo)))),
        ("azimuth", ctx.out("azimuth", deg(wrap_azimuth(az)))),
        ("half_distance", ctx.out("half_distance", meters(s12 / 2.0))),
    ]))
}

pub static TOOLS: &[&ToolDef] = &[
    &INVERSE,
    &DIRECT,
    &HAVERSINE,
    &VINCENTY_INVERSE,
    &VINCENTY_DIRECT,
    &MIDPOINT,
    &rhumb::RHUMB_INVERSE,
    &rhumb::RHUMB_DIRECT,
    &route::CROSS_TRACK,
    &route::FLY_BY,
    &route::TSD,
    &route::CPA,
    &route::LEGS,
    &route::CLOSEST_POINT,
    &rings::RANGE_RINGS,
    &waypoints::WAYPOINTS,
    &los::HORIZON,
    &los::VISIBILITY,
    &los::DIP,
    &los::FRESNEL,
    &intersect::COURSE_INTERSECTION,
    &intersect::INTERCEPT,
    &intersect::SEGMENT_INTERSECTION,
    &intersect::VERTEX,
    &sphere::SPHERICAL_INVERSE,
    &sphere::SPHERICAL_DIRECT,
    &sphere::INTERMEDIATE,
    &vector::DISTANCE_3D,
    &vector::LOOK_ANGLES,
    &vector::POLAR_CARTESIAN,
    &vector::OPERATIONS,
];

pub static REGISTRY: Registry = Registry {
    module: "navigation",
    tools: TOOLS,
};

gp_base::export_module!("navigation", REGISTRY);
