//! Ellipsoids and frames (geodesy/reference-frames spec): ellipsoid
//! parameters, radii of curvature and meridian arcs, auxiliary latitudes,
//! geodetic ↔ ECEF, and local ENU/NED/AER frames. The math is in
//! gp_geo::frames.

use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::ellipsoid::{self, Ellipsoid};
use gp_geo::frames::{self as fr, Aux};
use gp_geo::point;

const GEOGRAPHICLIB_GEOCENTRIC: Reference = Reference {
    title: "GeographicLib Geocentric and LocalCartesian classes",
    issuer: "Karney, C. F. F., GeographicLib",
    year: 2022,
    edition: "GeographicLib 2.x",
    locator: "Geocentric.cpp: Vermeille's closed-form inverse; LocalCartesian.cpp: ENU rotation",
    url: "https://geographiclib.sourceforge.io/C++/doc/classGeographicLib_1_1Geocentric.html",
};
const NGA_WGS84: Reference = Reference {
    title: "Department of Defense World Geodetic System 1984, NGA.STND.0036",
    issuer: "National Geospatial-Intelligence Agency",
    year: 2014,
    edition: "NGA.STND.0036_1.0.0_WGS84",
    locator: "Table 3.1 (defining parameters) and Table 3.3 (derived geometric constants)",
    url: "https://earth-info.nga.mil/php/download.php?file=coord-wgs84",
};
const GRS80_REF: Reference = Reference {
    title: "Geodetic Reference System 1980",
    issuer: "Moritz, H., Journal of Geodesy",
    year: 2000,
    edition: "Journal of Geodesy 74(1)",
    locator: "Derived geometric constants and mean radii R1, R2, R3",
    url: "https://doi.org/10.1007/s001900050278",
};
const SNYDER: Reference = Reference {
    title: "Map Projections: A Working Manual, USGS Professional Paper 1395",
    issuer: "Snyder, J. P., U.S. Geological Survey",
    year: 1987,
    edition: "Professional Paper 1395",
    locator: "Chapter 3: auxiliary latitudes, radii of curvature, and meridian distance",
    url: "https://pubs.usgs.gov/publication/pp1395",
};

const E: [Field; 3] = ellipsoid::FIELDS;
const M_P: Precision = Precision::Decimals(3);
const LAT_P: Precision = Precision::Decimals(10);

fn m(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Length, "m").expect("m"),
    }
}
fn deg(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Angle, "deg").expect("deg"),
    }
}

const fn len_out(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(M_P)
}
const fn len_in(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
}
const fn num_out(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Number {
            min: -1e300,
            max: 1e300,
        },
    )
    .precision(Precision::Significant(12))
}
const fn ang_out(name: &'static str, title: &'static str, range: &'static str) -> Field {
    Field::new(
        name,
        title,
        "Degrees",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(LAT_P)
    .angle_range(range)
}

fn length(ctx: &mut Ctx, name: &str) -> Result<Option<f64>, ToolError> {
    let meter = units::by_symbol(QT::Length, "m").expect("m");
    Ok(ctx.quantity(name)?.map(|q| q.to(meter)))
}

fn describe_model(ctx: &mut Ctx, ell: &Ellipsoid, what: &str) {
    ctx.model = Some(format!("{what}, {}", ell.describe()));
}

// ---------------------------------------------------------------- parameters

pub static PARAMETERS: ToolDef = ToolDef {
    id: "geodesy.ellipsoid.parameters",
    stability: gp_base::tool::Stability::Stable,
    title: "Ellipsoid parameters",
    summary: "The defining and derived parameters of a reference ellipsoid: semi-minor axis, flattening, eccentricities, third flattening, and the mean, authalic, and volumetric radii.",
    aliases: &[
        "ellipsoid calculator",
        "WGS 84 parameters",
        "semi-minor axis",
        "eccentricity",
    ],
    keywords: &[
        "ellipsoid",
        "flattening",
        "eccentricity",
        "semi-major axis",
        "authalic radius",
        "mean radius",
        "WGS 84",
        "GRS 80",
    ],
    inputs: &[
        E[0],
        E[1],
        E[2],
        len_in(
            "b",
            "Custom semi-minor axis",
            "Instead of inverse flattening, with a: like 6356752.314245 m",
        ),
    ],
    outputs: &[
        len_out("b", "Semi-minor axis b", "Polar radius"),
        len_out("a", "Semi-major axis a", "Equatorial radius"),
        num_out("flattening", "Flattening f", "(a − b)/a"),
        num_out(
            "inverse_flattening",
            "Inverse flattening 1/f",
            "Absent for a sphere",
        )
        .optional(),
        num_out("e2", "First eccentricity squared e²", "f(2 − f)"),
        num_out("ep2", "Second eccentricity squared e′²", "e²/(1 − e²)"),
        num_out("n", "Third flattening n", "(a − b)/(a + b)"),
        len_out("mean_radius", "Mean radius R1", "(2a + b)/3"),
        len_out(
            "authalic_radius",
            "Authalic radius R2",
            "The sphere with the same surface area",
        ),
        len_out(
            "volumetric_radius",
            "Volumetric radius R3",
            "The sphere with the same volume",
        ),
        len_out(
            "quarter_meridian",
            "Quarter meridian",
            "Equator to pole along a meridian",
        ),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &[],
    when_to_use: "Use this when a calculation needs the shape of the Earth written out: the semi-minor axis, the flattening, the two eccentricities, the third flattening, and the mean, authalic, and volumetric radii of a named ellipsoid, or of one you give by its own two defining numbers. It is what a projection, a datum transformation, or a geodesic calculation is set up from, and what to quote when a result has to say which figure of the Earth it used.",
    limitations: "An ellipsoid is a figure, not a datum: WGS 84 and GRS 80 differ in the last digits of their flattening and are still different realizations of position, so naming the ellipsoid does not pin down the coordinates. The derived values follow exactly from the two defining ones, so a custom ellipsoid given with a rounded flattening carries that rounding into everything below it. The radii are the ones a sphere would need to match the ellipsoid in one respect each -- mean, equal area, equal volume -- and they are not interchangeable: using the volumetric radius where an area calculation wanted the authalic one is a quiet error of about a part in a million.",
    model: "Closed-form ellipsoid relations; quarter meridian by Carlson's elliptic integrals",
    accuracy: "Exact formulas evaluated in double precision (relative error near 1e-16)",
    references: &[NGA_WGS84, GRS80_REF, SNYDER],
    examples: &[Example {
        id: "primary",
        title: "WGS 84",
        input: r#"{"ellipsoid":"wgs84"}"#,
        source: "add-geodesy-suite scenario: a = 6,378,137 m, 1/f = 298.257223563, b = 6,356,752.314245… m",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geodesy.ellipsoid.radii",
            reason: "next",
        },
        Related {
            id: "geodesy.ellipsoid.auxiliary-latitude",
            reason: "next",
        },
        Related {
            id: "geodesy.frame.geodetic-to-ecef",
            reason: "next",
        },
    ],
    sentence: "The semi-minor axis is {b}, with flattening {flattening}.",
    limits: &[("batchRows", 10_000)],
    run: run_parameters,
    ..ToolDef::BLANK
};

fn run_parameters(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let ell = match length(ctx, "b")? {
        Some(b) => {
            let a = length(ctx, "a")?.ok_or_else(|| {
                ToolError::invalid(
                    "/a",
                    "A custom semi-minor axis needs the semi-major axis a too.",
                )
            })?;
            if ctx.is_set("inverse_flattening") || ctx.is_set("ellipsoid") {
                return Err(ToolError::invalid(
                    "/b",
                    "Give a and b, or a and inverse flattening, or a catalog ellipsoid: one at a time.",
                ));
            }
            if !(1_000.0..=1e8).contains(&a) {
                return Err(ToolError::new(
                    ErrorCode::OutOfDomain,
                    "The semi-major axis must be between 1 km and 100,000 km.",
                )
                .at("/a"));
            }
            if !(b > 0.0 && b <= a) {
                return Err(ToolError::new(ErrorCode::OutOfDomain, "The semi-minor axis must be positive and no larger than a (an oblate ellipsoid or a sphere).").at("/b"));
            }
            Ellipsoid {
                id: "custom",
                name: "custom",
                a,
                f: (a - b) / a,
            }
        }
        None => Ellipsoid::from_ctx(ctx)?,
    };
    describe_model(ctx, &ell, "Ellipsoid relations");
    let d = ell.derived();
    let mut out = vec![
        ("b", ctx.out("b", m(d.b))),
        ("a", ctx.out("a", m(ell.a))),
        ("flattening", Json::Num(ell.f)),
    ];
    if ell.f != 0.0 {
        out.push(("inverse_flattening", Json::Num(1.0 / ell.f)));
    }
    out.extend([
        ("e2", Json::Num(d.e2)),
        ("ep2", Json::Num(d.ep2)),
        ("n", Json::Num(d.n)),
        ("mean_radius", ctx.out("mean_radius", m(d.r1))),
        ("authalic_radius", ctx.out("authalic_radius", m(d.r2))),
        ("volumetric_radius", ctx.out("volumetric_radius", m(d.r3))),
        (
            "quarter_meridian",
            ctx.out("quarter_meridian", m(ell.quarter_meridian())),
        ),
    ]);
    Ok(Json::obj(out))
}

// ---------------------------------------------------------------- radii

pub static RADII: ToolDef = ToolDef {
    id: "geodesy.ellipsoid.radii",
    title: "Radii of curvature and degree lengths",
    summary: "At a latitude: the meridional and prime-vertical radii of curvature, their Gaussian mean, the radius in any azimuth, the length of one degree of latitude and longitude, and the meridian arc from the equator or between two latitudes.",
    aliases: &[
        "length of a degree",
        "meridian arc length",
        "radius of curvature",
        "meters per degree",
    ],
    keywords: &[
        "radius of curvature",
        "meridian",
        "prime vertical",
        "degree of latitude",
        "degree of longitude",
        "meridian arc",
    ],
    inputs: &[
        point::lat_field("lat", "Latitude"),
        Field::new(
            "azimuth",
            "Azimuth",
            "Optional: for the radius of curvature in this direction, like 45",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        ),
        Field::new(
            "lat2",
            "Second latitude",
            "Optional: for the meridian arc between the two latitudes, like 45",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .angle_range("[-90,90]"),
        E[0],
        E[1],
        E[2],
    ],
    outputs: &[
        len_out(
            "degree_lat",
            "One degree of latitude",
            "Meridian arc from 0.5° below to 0.5° above",
        ),
        len_out(
            "degree_lon",
            "One degree of longitude",
            "Along the parallel",
        ),
        len_out("meridional", "Meridional radius M", "North-south curvature"),
        len_out(
            "prime_vertical",
            "Prime-vertical radius N",
            "East-west curvature",
        ),
        len_out(
            "gaussian",
            "Gaussian mean radius √(MN)",
            "Best local sphere",
        ),
        len_out("in_azimuth", "Radius in the azimuth", "Euler's formula").optional(),
        len_out(
            "meridian_arc",
            "Meridian arc from the equator",
            "Signed, north positive",
        ),
        len_out(
            "arc_between",
            "Meridian arc between the latitudes",
            "Signed, from the first to the second",
        )
        .optional(),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "Closed-form radii; meridian arc a[E(φ, e) − e² sinφ cosφ/W] by Carlson's elliptic integrals",
    accuracy: "Meridian arcs within 1 nm of GeographicLib's geodesic along the meridian; radii exact in double precision",
    references: &[SNYDER, GEOGRAPHICLIB_GEOCENTRIC],
    examples: &[Example {
        id: "primary",
        title: "45° N on WGS 84",
        input: r#"{"lat":45}"#,
        source: "add-geodesy-suite scenario: one degree of latitude ≈ 111,131.78 m, one degree of longitude ≈ 78,846.84 m",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geodesy.ellipsoid.parameters",
            reason: "alternative",
        },
        Related {
            id: "geodesy.ellipsoid.auxiliary-latitude",
            reason: "next",
        },
    ],
    sentence: "At this latitude one degree of latitude is {degree_lat} and one degree of longitude is {degree_lon}.",
    limits: &[("batchRows", 10_000)],
    run: run_radii,
    ..ToolDef::BLANK
};

fn run_radii(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let lat = point::plain_angle(ctx, "lat")?.expect("required");
    let lat = gp_base::angle::check_lat(lat, "/lat")?;
    let az = point::plain_angle(ctx, "azimuth")?;
    let lat2 = match point::plain_angle(ctx, "lat2")? {
        Some(x) => Some(gp_base::angle::check_lat(x, "/lat2")?),
        None => None,
    };
    let ell = Ellipsoid::from_ctx(ctx)?;
    describe_model(ctx, &ell, "Radii of curvature and meridian arc");
    let phi = lat.to_radians();
    let (mr, nr) = (ell.meridional_radius(phi), ell.prime_vertical_radius(phi));
    // One degree centered on the latitude, folded at the poles.
    let lo = (lat - 0.5).max(-90.0);
    let hi = (lat + 0.5).min(90.0);
    let degree_lat =
        (ell.meridian_arc(hi.to_radians()) - ell.meridian_arc(lo.to_radians())) / (hi - lo);
    let degree_lon = 1f64.to_radians()
        * nr
        * if lat.abs() == 90.0 {
            0.0
        } else {
            lat.to_radians().cos()
        };
    let mut out = vec![
        ("degree_lat", ctx.out("degree_lat", m(degree_lat))),
        ("degree_lon", ctx.out("degree_lon", m(degree_lon.max(0.0)))),
        ("meridional", ctx.out("meridional", m(mr))),
        ("prime_vertical", ctx.out("prime_vertical", m(nr))),
        ("gaussian", ctx.out("gaussian", m((mr * nr).sqrt()))),
    ];
    if let Some(az) = az {
        out.push((
            "in_azimuth",
            ctx.out("in_azimuth", m(ell.radius_in_azimuth(phi, az.to_radians()))),
        ));
    }
    let arc = ell.meridian_arc(phi);
    out.push(("meridian_arc", ctx.out("meridian_arc", m(arc))));
    if let Some(l2) = lat2 {
        out.push((
            "arc_between",
            ctx.out("arc_between", m(ell.meridian_arc(l2.to_radians()) - arc)),
        ));
    }
    Ok(Json::obj(out))
}

// ---------------------------------------------------------------- auxiliary latitudes

const AUX_KINDS: &[&str] = &[
    "geodetic",
    "geocentric",
    "parametric",
    "rectifying",
    "conformal",
    "authalic",
    "isometric",
];

pub static AUXILIARY: ToolDef = ToolDef {
    id: "geodesy.ellipsoid.auxiliary-latitude",
    title: "Auxiliary latitudes",
    summary: "Converts between geodetic latitude and the geocentric, parametric (reduced), rectifying, conformal, authalic, and isometric latitudes, in either direction.",
    aliases: &["geocentric latitude", "reduced latitude", "conformal latitude", "authalic latitude", "isometric latitude"],
    keywords: &["auxiliary latitude", "geocentric", "parametric", "rectifying", "conformal", "authalic", "isometric", "geodetic latitude"],
    inputs: &[
        Field::new(
            "latitude",
            "Latitude",
            "Degrees, like 45 (isometric latitude may exceed 90)",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core(),
        Field::new("from", "Latitude kind", "geodetic (default), geocentric, parametric, rectifying, conformal, authalic, or isometric", Kind::Choice(AUX_KINDS)).core(),
        E[0],
        E[1],
        E[2],
    ],
    outputs: &[
        ang_out("geocentric", "Geocentric latitude", "[-90,90]"),
        ang_out("geodetic", "Geodetic latitude", "[-90,90]"),
        ang_out("parametric", "Parametric (reduced) latitude", "[-90,90]"),
        ang_out("rectifying", "Rectifying latitude", "[-90,90]"),
        ang_out("conformal", "Conformal latitude", "[-90,90]"),
        ang_out("authalic", "Authalic latitude", "[-90,90]"),
        ang_out("isometric", "Isometric latitude", "unbounded"),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain, ErrorCode::Unsupported],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "Closed forms for geocentric, parametric, conformal (Karney's τ′), and authalic (cancellation-free qp − q) latitudes; rectifying by the exact meridian arc; inverses by closed form or Newton's method",
    accuracy: "Within 1e-13° of 40-digit reference values on WGS 84; round trips within 1e-12°",
    references: &[SNYDER, super::KARNEY_TM],
    examples: &[Example {
        id: "primary",
        title: "45° geodetic on WGS 84",
        input: r#"{"latitude":45}"#,
        source: "add-geodesy-suite scenario: geocentric latitude ≈ 44.8076°",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[Related {
        id: "geodesy.ellipsoid.radii",
        reason: "alternative",
    }],
    sentence: "The geocentric latitude is {geocentric} and the geodetic latitude is {geodetic}.",
    limits: &[("batchRows", 10_000)],
    run: run_auxiliary,
    ..ToolDef::BLANK
};

fn run_auxiliary(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let x = point::plain_angle(ctx, "latitude")?.expect("required");
    let from = match ctx.choice("from")?.unwrap_or("geodetic") {
        "geocentric" => Aux::Geocentric,
        "parametric" => Aux::Parametric,
        "rectifying" => Aux::Rectifying,
        "conformal" => Aux::Conformal,
        "authalic" => Aux::Authalic,
        "isometric" => Aux::Isometric,
        _ => Aux::Geodetic,
    };
    if from != Aux::Isometric {
        gp_base::angle::check_lat(x, "/latitude")?;
    } else if !x.is_finite() || x.abs() > 1e4 {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "Isometric latitude beyond ±10,000° is the pole itself in double precision.",
        )
        .at("/latitude"));
    }
    let ell = Ellipsoid::from_ctx(ctx)?;
    // Newton's method for the conformal latitude converges only for moderate flattening.
    ell.geodesic()?;
    describe_model(ctx, &ell, "Auxiliary latitudes");
    let phi = ell.geodetic_from(from, x.to_radians());
    let a = ell.auxiliary(phi);
    Ok(Json::obj([
        (
            "geocentric",
            ctx.out("geocentric", deg(a.geocentric.to_degrees())),
        ),
        ("geodetic", ctx.out("geodetic", deg(phi.to_degrees()))),
        (
            "parametric",
            ctx.out("parametric", deg(a.parametric.to_degrees())),
        ),
        (
            "rectifying",
            ctx.out("rectifying", deg(a.rectifying.to_degrees())),
        ),
        (
            "conformal",
            ctx.out("conformal", deg(a.conformal.to_degrees())),
        ),
        (
            "authalic",
            ctx.out("authalic", deg(a.authalic.to_degrees())),
        ),
        (
            "isometric",
            ctx.out("isometric", deg(a.isometric.to_degrees())),
        ),
    ]))
}

// ---------------------------------------------------------------- ECEF

const HEIGHT: Field = Field::new(
    "height",
    "Ellipsoidal height",
    "Height above the ellipsoid (not sea level), like 300 m; default 0",
    Kind::Quantity {
        q: QT::Length,
        unit: "m",
    },
)
.core();

pub static TO_ECEF: ToolDef = ToolDef {
    id: "geodesy.frame.geodetic-to-ecef",
    stability: gp_base::tool::Stability::Stable,
    title: "Latitude, longitude, and height to ECEF",
    summary: "Converts geodetic latitude, longitude, and ellipsoidal height to Earth-centered, Earth-fixed X, Y, Z.",
    aliases: &[
        "LLA to ECEF",
        "geodetic to geocentric",
        "lat lon to XYZ",
        "ECEF converter",
    ],
    keywords: &[
        "ECEF",
        "geocentric",
        "XYZ",
        "cartesian",
        "earth-centered",
        "LLA",
    ],
    inputs: &[
        point::lat_field("lat", "Latitude"),
        point::lon_field("lon", "Longitude"),
        HEIGHT,
        E[0],
        E[1],
        E[2],
    ],
    outputs: &[
        len_out("x", "X", "Toward latitude 0, longitude 0"),
        len_out("y", "Y", "Toward latitude 0, longitude 90° E"),
        len_out("z", "Z", "Toward the north pole"),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["INPUT_NORMALIZED", "EXPERIMENTAL_TOOL"],
    model: "X = (N + h) cosφ cosλ, Y = (N + h) cosφ sinλ, Z = (N(1 − e²) + h) sinφ",
    accuracy: "Exact formula in double precision; matches GeographicLib CartConvert to 1 µm",
    when_to_use: "Use this when a calculation wants Cartesian coordinates rather than angles: baselines between stations, vector arithmetic on positions, satellite geometry, or feeding a system that works in X, Y, Z. It takes latitude, longitude, and ellipsoidal height on the ellipsoid you choose. It is also the step before any vector arithmetic between two positions: differences, baselines, and rotations are done in Cartesian space, not on latitudes and longitudes.",
    limitations: "The height must be ellipsoidal; passing a height above mean sea level puts the point off by the geoid separation, which reaches tens of meters. The result is only as well defined as the frame its input belongs to, and this tool converts coordinates rather than datums, frames, or epochs. The ellipsoid matters: the same latitude, longitude, and height on a different ellipsoid gives different X, Y, Z, so the ellipsoid has to be the one the coordinates belong to.",
    references: &[GEOGRAPHICLIB_GEOCENTRIC, NGA_WGS84],
    examples: &[Example {
        id: "primary",
        title: "Pittsburgh at 300 m",
        input: r#"{"lat":40.446111,"lon":-79.982222,"height":300}"#,
        source: "add-geodesy-suite scenario: X ≈ 845,580.010 m, Y ≈ −4,786,836.717 m, Z ≈ 4,116,002.385 m",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geodesy.frame.ecef-to-geodetic",
            reason: "inverse",
        },
        Related {
            id: "geodesy.frame.to-local",
            reason: "next",
        },
        Related {
            id: "geodesy.parse.coordinates",
            reason: "parent",
        },
    ],
    sentence: "ECEF X {x}, Y {y}, Z {z}.",
    limits: &[("batchRows", 10_000)],
    run: run_to_ecef,
    ..ToolDef::BLANK
};

fn run_to_ecef(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let h = height(ctx, "height")?;
    let ell = Ellipsoid::from_ctx(ctx)?;
    describe_model(ctx, &ell, "Geodetic to ECEF");
    let (x, y, z) = fr::to_ecef(&ell, lat.to_radians(), lon.to_radians(), h);
    Ok(Json::obj([
        ("x", ctx.out("x", m(x))),
        ("y", ctx.out("y", m(y))),
        ("z", ctx.out("z", m(z))),
    ]))
}

/// An ellipsoidal height within −10 km … 100,000 km (default 0).
fn height(ctx: &mut Ctx, name: &str) -> Result<f64, ToolError> {
    let h = length(ctx, name)?.unwrap_or(0.0);
    if !(-10_000.0..=1e8).contains(&h) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "Ellipsoidal height must be between −10 km and 100,000 km.",
        )
        .at(&format!("/{name}")));
    }
    Ok(h)
}

const fn xyz_in(name: &'static str, title: &'static str) -> Field {
    Field::new(
        name,
        title,
        "Meters from the Earth's center, like 845580.010",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .required()
    .core()
}

pub static FROM_ECEF: ToolDef = ToolDef {
    id: "geodesy.frame.ecef-to-geodetic",
    stability: gp_base::tool::Stability::Stable,
    title: "ECEF to latitude, longitude, and height",
    summary: "Converts Earth-centered, Earth-fixed X, Y, Z to geodetic latitude, longitude, and ellipsoidal height, in closed form at any height.",
    aliases: &["ECEF to LLA", "geocentric to geodetic", "XYZ to lat lon"],
    keywords: &[
        "ECEF",
        "geocentric",
        "XYZ",
        "cartesian",
        "earth-centered",
        "LLA",
    ],
    inputs: &[
        xyz_in("x", "X"),
        xyz_in("y", "Y"),
        xyz_in("z", "Z"),
        E[0],
        E[1],
        E[2],
    ],
    outputs: &[
        ang_out("lat", "Latitude", "[-90,90]"),
        ang_out("lon", "Longitude", "[-180,180)"),
        len_out(
            "height",
            "Ellipsoidal height",
            "Above the ellipsoid, not sea level",
        ),
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::DegenerateGeometry,
    ],
    warnings: &["LONGITUDE_UNDEFINED", "EXPERIMENTAL_TOOL"],
    model: "Vermeille (2011) closed form, as in GeographicLib's Geocentric class: no iteration",
    accuracy: "Round trips close within 5 nm near the Earth's surface (a few units in the last place) and within 1e-15 of the distance at any height; matches GeographicLib CartConvert",
    when_to_use: "Use this when a position arrives as Earth-centered X, Y, Z — from a GNSS receiver's raw output, a satellite product, or a coordinate exchange — and you need it as latitude, longitude, and height to plot or compare it. It is the inverse of the geodetic-to-ECEF tool and holds at any height, from below the surface to orbit.",
    limitations: "The height is above the ellipsoid, not above mean sea level: the geoid tool supplies the separation that turns one into the other. The frame the X, Y, Z came in matters as much as the arithmetic — ITRF, a WGS 84 realization, and a regional frame differ by centimeters to meters — and this conversion does not change frames or epochs.",
    references: &[GEOGRAPHICLIB_GEOCENTRIC, NGA_WGS84],
    examples: &[Example {
        id: "primary",
        title: "Pittsburgh at 300 m",
        input: r#"{"x":845580.010370,"y":-4786836.716953,"z":4116002.385436}"#,
        source: "GeographicLib CartConvert -r: 40.446111, -79.982222, 300 m",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "point",
        map: &[("lat", "lat"), ("lon", "lon")],
    }],
    related: &[
        Related {
            id: "geodesy.frame.geodetic-to-ecef",
            reason: "inverse",
        },
        Related {
            id: "geodesy.frame.to-local",
            reason: "next",
        },
        Related {
            id: "geodesy.parse.coordinates",
            reason: "next",
        },
    ],
    sentence: "Latitude {lat}, longitude {lon}, ellipsoidal height {height}.{warn LONGITUDE_UNDEFINED} The point is on the polar axis, so the longitude is shown as 0.{/warn}",
    limits: &[("batchRows", 10_000)],
    run: run_from_ecef,
    ..ToolDef::BLANK
};

fn run_from_ecef(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let mut xyz = [0.0; 3];
    for (i, k) in ["x", "y", "z"].into_iter().enumerate() {
        let v = length(ctx, k)?.expect("required");
        if v.abs() > 1e9 {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "ECEF coordinates beyond 1,000,000 km are outside this tool's range.",
            )
            .at(&format!("/{k}")));
        }
        xyz[i] = v;
    }
    let [x, y, z] = xyz;
    let ell = Ellipsoid::from_ctx(ctx)?;
    if (x * x + y * y + z * z).sqrt() < 1.0 {
        return Err(ToolError::new(
            ErrorCode::DegenerateGeometry,
            "The point is at the Earth's center, where latitude and longitude are undefined.",
        )
        .at("/x"));
    }
    describe_model(ctx, &ell, "ECEF to geodetic (Vermeille closed form)");
    let (phi, lam, h) = fr::from_ecef(&ell, x, y, z).expect("not the center");
    if x == 0.0 && y == 0.0 {
        ctx.warnings.push(Warning::new(
            "LONGITUDE_UNDEFINED",
            "The point is on the polar axis, so longitude is undefined; 0 is returned.",
        ));
    }
    let lon = gp_base::angle::wrap_lon(lam.to_degrees());
    Ok(Json::obj([
        ("lat", ctx.out("lat", deg(phi.to_degrees()))),
        ("lon", ctx.out("lon", deg(lon + 0.0))),
        ("height", ctx.out("height", m(h))),
    ]))
}

// ---------------------------------------------------------------- local frames

const ORIGIN: [Field; 3] = [
    point::lat_field("lat0", "Origin latitude"),
    point::lon_field("lon0", "Origin longitude"),
    Field::new(
        "h0",
        "Origin height",
        "Ellipsoidal height of the origin, like 300 m; default 0",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    ),
];

const ROW: &[Field] = &[
    Field::new(
        "x",
        "X",
        "ECEF X component",
        Kind::Number {
            min: -1.0,
            max: 1.0,
        },
    )
    .precision(Precision::Decimals(12)),
    Field::new(
        "y",
        "Y",
        "ECEF Y component",
        Kind::Number {
            min: -1.0,
            max: 1.0,
        },
    )
    .precision(Precision::Decimals(12)),
    Field::new(
        "z",
        "Z",
        "ECEF Z component",
        Kind::Number {
            min: -1.0,
            max: 1.0,
        },
    )
    .precision(Precision::Decimals(12)),
];

pub static TO_LOCAL: ToolDef = ToolDef {
    id: "geodesy.frame.to-local",
    diagram_inline: true,
    stability: gp_base::tool::Stability::Stable,
    title: "Point to local ENU, NED, and AER",
    summary: "Expresses a target point in a local tangent plane at an origin: east-north-up, north-east-down, and azimuth-elevation-range, with the rotation matrix on request.",
    aliases: &[
        "ENU converter",
        "NED coordinates",
        "azimuth elevation range",
        "look angles",
        "LLA to ENU",
    ],
    keywords: &[
        "ENU",
        "NED",
        "AER",
        "local tangent plane",
        "azimuth",
        "elevation",
        "slant range",
        "look angle",
    ],
    inputs: &[
        ORIGIN[0],
        ORIGIN[1],
        ORIGIN[2],
        point::lat_field("lat", "Target latitude"),
        point::lon_field("lon", "Target longitude"),
        Field::new(
            "height",
            "Target height",
            "Ellipsoidal height of the target, like 1300 m; default 0",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        ),
        Field::new(
            "matrix",
            "Rotation matrix",
            "yes to include the ECEF → ENU rotation matrix",
            Kind::Choice(&["no", "yes"]),
        ),
        E[0],
        E[1],
        E[2],
    ],
    outputs: &[
        ang_out("azimuth", "Azimuth", "[0,360)").precision(Precision::Decimals(6)),
        ang_out("elevation", "Elevation", "[-90,90]").precision(Precision::Decimals(6)),
        len_out("range", "Slant range", "Straight-line distance"),
        len_out("east", "East", "ENU"),
        len_out("north", "North", "ENU"),
        len_out("up", "Up", "ENU; down is its negative in NED"),
        len_out("down", "Down", "NED"),
        len_out("horizontal", "Horizontal distance", "In the tangent plane"),
        Field::new(
            "rotation",
            "Rotation ECEF → ENU",
            "Rows: east, north, up unit vectors in ECEF",
            Kind::List {
                items: ROW,
                min: 3,
                max: 3,
            },
        )
        .optional(),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["AZIMUTH_UNDEFINED", "INPUT_NORMALIZED", "EXPERIMENTAL_TOOL"],
    model: "ECEF difference rotated into the origin's east-north-up frame (GeographicLib LocalCartesian)",
    accuracy: "Round trips within 1e-8 m for targets within 1,000 km",
    when_to_use: "Use this when you want a target described the way an observer at a station sees it: east, north and up, north, east and down, or an azimuth, elevation and range. It is the tool behind sky plots, antenna and camera aiming, and any check of where one point lies relative to another.",
    limitations: "The plane is tangent at the origin, so it is a local description rather than a map projection, and it takes no account of refraction, which bends both light and radio near the horizon, or of obstacles between the two points. The azimuth is true, and heights are above the ellipsoid rather than above sea level.",
    references: &[GEOGRAPHICLIB_GEOCENTRIC],
    examples: &[Example {
        id: "primary",
        title: "A point 1,000 m overhead",
        input: r#"{"lat0":40.446111,"lon0":-79.982222,"h0":300,"lat":40.446111,"lon":-79.982222,"height":1300}"#,
        source: "add-geodesy-suite scenario: elevation 90°, range 1,000 m, azimuth 0 with AZIMUTH_UNDEFINED",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[("azimuth", "azimuth"), ("elevation", "elevation")],
    }],
    related: &[
        Related {
            id: "geodesy.frame.from-local",
            reason: "inverse",
        },
        Related {
            id: "geodesy.frame.geodetic-to-ecef",
            reason: "alternative",
        },
        Related {
            id: "geodesy.frame.ecef-to-geodetic",
            reason: "next",
        },
    ],
    sentence: "The target is at azimuth {azimuth}, elevation {elevation}, range {range}.{warn AZIMUTH_UNDEFINED} It is straight up or down, so the azimuth is shown as 0.{/warn}",
    limits: &[("batchRows", 10_000)],
    run: run_to_local,
    ..ToolDef::BLANK
};

/// A local frame's origin: its ECEF position, and its latitude and longitude
/// in radians.
type Origin = ((f64, f64, f64), f64, f64);

fn origin(ctx: &mut Ctx, ell: &Ellipsoid) -> Result<Origin, ToolError> {
    let (lat0, lon0) = point::read(ctx, "lat0", "lon0")?;
    let h0 = height(ctx, "h0")?;
    let (phi0, lam0) = (lat0.to_radians(), lon0.to_radians());
    Ok((fr::to_ecef(ell, phi0, lam0, h0), phi0, lam0))
}

fn run_to_local(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let ell = Ellipsoid::from_ctx(ctx)?;
    let (o, phi0, lam0) = origin(ctx, &ell)?;
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let h = height(ctx, "height")?;
    describe_model(ctx, &ell, "Local east-north-up frame");
    let p = fr::to_ecef(&ell, lat.to_radians(), lon.to_radians(), h);
    let enu = fr::ecef_to_enu(o, phi0, lam0, p);
    let (az, el, range) = fr::enu_to_aer(enu);
    let horizontal = enu[0].hypot(enu[1]);
    // Straight up or down (or the origin itself): no direction in the plane,
    // and the elevation is exactly ±90° rather than rounding residue.
    let straight = horizontal <= 1e-9 * range.max(1.0);
    let (az, el) = if straight {
        ctx.warnings.push(Warning::new("AZIMUTH_UNDEFINED", "The target is straight above or below the origin, so azimuth is undefined; 0 is returned."));
        (0.0, if enu[2] >= 0.0 { 90.0 } else { -90.0 })
    } else {
        (
            gp_base::angle::wrap_azimuth(az.to_degrees()),
            el.to_degrees(),
        )
    };
    let mut out = vec![
        ("azimuth", ctx.out("azimuth", deg(az))),
        ("elevation", ctx.out("elevation", deg(el))),
        ("range", ctx.out("range", m(range))),
        ("east", ctx.out("east", m(enu[0]))),
        ("north", ctx.out("north", m(enu[1]))),
        ("up", ctx.out("up", m(enu[2]))),
        ("down", ctx.out("down", m(-enu[2] + 0.0))),
        ("horizontal", ctx.out("horizontal", m(horizontal))),
    ];
    if ctx.choice("matrix")? == Some("yes") {
        let r = fr::enu_rotation(phi0, lam0);
        out.push((
            "rotation",
            Json::Arr(
                r.iter()
                    .map(|row| {
                        Json::obj([
                            ("x", Json::Num(row[0] + 0.0)),
                            ("y", Json::Num(row[1] + 0.0)),
                            ("z", Json::Num(row[2] + 0.0)),
                        ])
                    })
                    .collect(),
            ),
        ));
    }
    Ok(Json::obj(out))
}

const fn local_len(name: &'static str, title: &'static str) -> Field {
    Field::new(
        name,
        title,
        "Meters, like 1000",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
}

pub static FROM_LOCAL: ToolDef = ToolDef {
    id: "geodesy.frame.from-local",
    stability: gp_base::tool::Stability::Stable,
    title: "Local ENU, NED, or AER to point",
    summary: "Finds the latitude, longitude, height, and ECEF position of a point given in a local tangent plane at an origin: east-north-up, north-east-down, or azimuth-elevation-range.",
    aliases: &[
        "ENU to LLA",
        "AER to lat lon",
        "NED to geodetic",
        "project a look angle",
    ],
    keywords: &[
        "ENU",
        "NED",
        "AER",
        "local tangent plane",
        "azimuth",
        "elevation",
        "slant range",
    ],
    inputs: &[
        ORIGIN[0],
        ORIGIN[1],
        ORIGIN[2],
        Field::new(
            "frame",
            "Frame",
            "enu (default), ned, or aer",
            Kind::Choice(&["enu", "ned", "aer"]),
        )
        .core(),
        local_len("east", "East"),
        local_len("north", "North"),
        local_len("up", "Up"),
        local_len("down", "Down"),
        Field::new(
            "azimuth",
            "Azimuth",
            "Degrees clockwise from true north, like 45",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        ),
        Field::new(
            "elevation",
            "Elevation",
            "Degrees above the horizontal plane, like 30",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .angle_range("[-90,90]"),
        local_len("range", "Slant range"),
        E[0],
        E[1],
        E[2],
    ],
    outputs: &[
        ang_out("lat", "Latitude", "[-90,90]"),
        ang_out("lon", "Longitude", "[-180,180)"),
        len_out(
            "height",
            "Ellipsoidal height",
            "Above the ellipsoid, not sea level",
        ),
        len_out("x", "ECEF X", "Meters"),
        len_out("y", "ECEF Y, like -4819000 m", "Meters"),
        len_out("z", "ECEF Z, like 3976000 m", "Meters"),
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::DegenerateGeometry,
    ],
    warnings: &[
        "INPUT_NORMALIZED",
        "LONGITUDE_UNDEFINED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Local offset rotated from the origin's east-north-up frame to ECEF, then Vermeille's closed-form inverse",
    accuracy: "Round trips within 1e-8 m for points within 1,000 km of the origin",
    when_to_use: "Use this when a point is described relative to a known station: east, north and up from an origin, north, east and down in an aircraft or vehicle frame, or as an azimuth, elevation and range from a total station or a tracker. It returns the point's latitude, longitude, height, and ECEF position.",
    limitations: "The local frame is tangent at the origin you give, so the further out the point lies the more the Earth's curvature matters; round trips stay within a hundredth of a micrometer out to a thousand kilometers. Heights are ellipsoidal. An azimuth here is true and geometric: it carries no refraction, no magnetic variation, and no instrument correction.",
    references: &[GEOGRAPHICLIB_GEOCENTRIC],
    examples: &[Example {
        id: "primary",
        title: "10 km out at 045°, 5° up",
        input: r#"{"lat0":40.446111,"lon0":-79.982222,"h0":300,"frame":"aer","azimuth":45,"elevation":5,"range":10000}"#,
        source: "GeographicLib LocalCartesian reverse of the AER offset",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "point",
        map: &[("lat", "lat"), ("lon", "lon")],
    }],
    related: &[
        Related {
            id: "geodesy.frame.to-local",
            reason: "inverse",
        },
        Related {
            id: "geodesy.frame.ecef-to-geodetic",
            reason: "next",
        },
        Related {
            id: "geodesy.frame.geodetic-to-ecef",
            reason: "alternative",
        },
    ],
    sentence: "The point is at latitude {lat}, longitude {lon}, ellipsoidal height {height}.",
    limits: &[("batchRows", 10_000)],
    run: run_from_local,
    ..ToolDef::BLANK
};

fn run_from_local(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let ell = Ellipsoid::from_ctx(ctx)?;
    let (o, phi0, lam0) = origin(ctx, &ell)?;
    let frame = ctx.choice("frame")?.unwrap_or("enu");
    let need = |ctx: &mut Ctx, k: &str| -> Result<f64, ToolError> {
        length(ctx, k)?.ok_or_else(|| {
            ToolError::invalid(&format!("/{k}"), format!("The {frame} frame needs {k}."))
        })
    };
    let enu = match frame {
        "ned" => {
            let (n, e, d) = (need(ctx, "north")?, need(ctx, "east")?, need(ctx, "down")?);
            [e, n, -d]
        }
        "aer" => {
            let az = point::plain_angle(ctx, "azimuth")?
                .ok_or_else(|| ToolError::invalid("/azimuth", "The aer frame needs azimuth."))?;
            let el = point::plain_angle(ctx, "elevation")?.ok_or_else(|| {
                ToolError::invalid("/elevation", "The aer frame needs elevation.")
            })?;
            if !(-90.0..=90.0).contains(&el) {
                return Err(
                    ToolError::new(ErrorCode::OutOfDomain, "Elevation is −90° to 90°.")
                        .at("/elevation"),
                );
            }
            let range = need(ctx, "range")?;
            if range < 0.0 {
                return Err(ToolError::new(
                    ErrorCode::OutOfDomain,
                    "Slant range cannot be negative.",
                )
                .at("/range"));
            }
            fr::aer_to_enu(az.to_radians(), el.to_radians(), range)
        }
        _ => [need(ctx, "east")?, need(ctx, "north")?, need(ctx, "up")?],
    };
    if enu.iter().any(|v| v.abs() > 1e8) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "Local offsets beyond 100,000 km are outside this tool's range.",
        )
        .at("/range"));
    }
    describe_model(ctx, &ell, "Local east-north-up frame");
    let (x, y, z) = fr::enu_to_ecef(o, phi0, lam0, enu);
    let Some((phi, lam, h)) =
        fr::from_ecef(&ell, x, y, z).filter(|_| (x * x + y * y + z * z).sqrt() >= 1.0)
    else {
        return Err(ToolError::new(
            ErrorCode::DegenerateGeometry,
            "The point lands at the Earth's center, where latitude and longitude are undefined.",
        ));
    };
    if x == 0.0 && y == 0.0 {
        ctx.warnings.push(Warning::new(
            "LONGITUDE_UNDEFINED",
            "The point is on the polar axis, so longitude is undefined; 0 is returned.",
        ));
    }
    let lon = gp_base::angle::wrap_lon(lam.to_degrees());
    Ok(Json::obj([
        ("lat", ctx.out("lat", deg(phi.to_degrees()))),
        ("lon", ctx.out("lon", deg(lon + 0.0))),
        ("height", ctx.out("height", m(h))),
        ("x", ctx.out("x", m(x))),
        ("y", ctx.out("y", m(y))),
        ("z", ctx.out("z", m(z))),
    ]))
}
