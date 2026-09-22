//! Positions off the ground (navigation/vector-3d): the straight-line 3D
//! distance between two geodetic points through ECEF, with the height
//! references reconciled through a geoid, and look angles (azimuth,
//! elevation, slant range) from an observer to a target, including targets
//! below the horizon.

use geographiclib_rs::{Geodesic, InverseGeodesic};
use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use gp_geo::ellipsoid::CATALOG;
use gp_geo::frames as fr;
use gp_geo::geoid::Grid;
use gp_geo::point;
use libm::acos;

use crate::los::R_DEFAULT;
use crate::unit;

const LOCAL_CARTESIAN: Reference = Reference {
    title: "GeographicLib Geocentric and LocalCartesian classes",
    issuer: "Karney, C. F. F., GeographicLib",
    year: 2022,
    edition: "GeographicLib 2.x",
    locator: "Geocentric.cpp: geodetic to ECEF; LocalCartesian.cpp: the east-north-up rotation",
    url: "https://geographiclib.sourceforge.io/C++/doc/classGeographicLib_1_1LocalCartesian.html",
};

const EGM96_ID: &str = "egm96-15";
const EGM96_VERSION: &str = "2009-08-29";
const EGM96_FILE: &str = "egm96-15.pgm";

const fn len(name: &'static str, title: &'static str, help: &'static str) -> Field {
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

const fn ang(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
}

fn m(v: f64) -> Q {
    Q {
        value: v,
        unit: unit(QT::Length, "m"),
    }
}

fn deg(v: f64) -> Q {
    Q {
        value: v,
        unit: unit(QT::Angle, "deg"),
    }
}

fn height(ctx: &mut Ctx, name: &str) -> Result<f64, ToolError> {
    let h = ctx.req_quantity(name)?.base();
    if !(-12_000.0..=1e8).contains(&h) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "Heights must be between -12 km and 100,000 km.",
        )
        .at(&format!("/{name}")));
    }
    Ok(h)
}

/// ECEF of a WGS 84 point with an ellipsoidal height.
fn ecef(lat: f64, lon: f64, h: f64) -> (f64, f64, f64) {
    fr::to_ecef(&CATALOG[0], lat.to_radians(), lon.to_radians(), h)
}

/// East, north, up of `b` seen from `a` (both WGS 84, ellipsoidal heights).
fn enu(a: (f64, f64, f64), b: (f64, f64, f64)) -> [f64; 3] {
    fr::ecef_to_enu(
        ecef(a.0, a.1, a.2),
        a.0.to_radians(),
        a.1.to_radians(),
        ecef(b.0, b.1, b.2),
    )
}

// ---------------------------------------------------------------- 3D distance

pub static DISTANCE_3D: ToolDef = ToolDef {
    id: "navigation.vector.distance-3d",
    title: "3D distance between two points",
    summary: "The straight-line distance between two points with heights, such as a drone and its ground station, through Earth-centered coordinates, with the ground distance, height difference, and elevation angle.",
    aliases: &[
        "3D distance calculator",
        "slant range calculator",
        "distance with altitude",
        "drone to ground station distance",
    ],
    keywords: &[
        "3D distance",
        "slant range",
        "ECEF",
        "chord",
        "elevation angle",
        "height",
        "drone",
        "antenna",
    ],
    inputs: &[
        point::lat_field("lat1", "First point latitude"),
        point::lon_field("lon1", "First point longitude"),
        len("height1", "First point height", "Like 250 m")
            .required()
            .core(),
        point::lat_field("lat2", "Second point latitude"),
        point::lon_field("lon2", "Second point longitude"),
        len("height2", "Second point height", "Like 370 m")
            .required()
            .core(),
        Field::new(
            "reference1",
            "First height is",
            "hae (above the ellipsoid, like GPS; default) or msl (above sea level)",
            Kind::Choice(&["hae", "msl"]),
        ),
        Field::new(
            "reference2",
            "Second height is",
            "hae (above the ellipsoid, like GPS; default) or msl (above sea level)",
            Kind::Choice(&["hae", "msl"]),
        ),
        Field::new(
            "geoid",
            "Geoid model",
            "none (default) or egm96, to turn sea-level heights into ellipsoidal ones",
            Kind::Choice(&["none", "egm96"]),
        ),
    ],
    outputs: &[
        len(
            "slant_range",
            "Straight-line distance",
            "Through Earth-centered coordinates",
        )
        .precision(Precision::Decimals(3)),
        len(
            "ground_distance",
            "Ground distance",
            "Along the ellipsoid between the points below",
        )
        .precision(Precision::Decimals(3)),
        len(
            "height_difference",
            "Height difference",
            "Second − first, ellipsoidal",
        )
        .precision(Precision::Decimals(3)),
        ang(
            "elevation_angle",
            "Elevation angle",
            "Of the second point seen from the first, above the local horizontal",
        )
        .precision(Precision::Decimals(4))
        .angle_range("unbounded"),
        ang(
            "azimuth",
            "Azimuth",
            "Of the second point seen from the first, from true north",
        )
        .precision(Precision::Decimals(2))
        .angle_range("[0,360)"),
        len(
            "flat_earth_range",
            "Flat-Earth distance",
            "√(ground² + height difference²): ignores the Earth's curve",
        )
        .precision(Precision::Decimals(3)),
        ang(
            "flat_earth_elevation",
            "Flat-Earth elevation angle",
            "atan(height difference / ground): ignores the Earth's curve",
        )
        .precision(Precision::Decimals(4))
        .angle_range("unbounded"),
        Field::new(
            "heights_used",
            "Heights used",
            "Which reference each height was taken in, and the geoid applied",
            Kind::Text { max_len: 200 },
        ),
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::AssetUnavailable,
        ErrorCode::AssetIntegrity,
    ],
    warnings: &[
        "ORTHOMETRIC_AS_ELLIPSOIDAL",
        "INPUT_NORMALIZED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Both points to WGS 84 ECEF (sea-level heights first become ellipsoidal, h = H + N, with EGM96); straight-line distance = |ΔECEF|; elevation and azimuth from the east-north-up frame at the first point; ground distance by the Karney geodesic inverse",
    accuracy: "Exact geometry for the heights given. A sea-level height carries the geoid's error, about 0.5-1 m for EGM96.",
    references: &[LOCAL_CARTESIAN],
    examples: &[Example {
        id: "primary",
        title: "A ground station at 250 m and a drone 2,000 m north at 370 m",
        input: r#"{"lat1":40,"lon1":-105,"height1":"250 m","lat2":40.018012369978514,"lon2":-105,"height2":"370 m"}"#,
        source: "add-navigation-and-geometry drone scenario: straight-line 2,003.694 m, elevation 3.4245°; flat Earth 2,003.597 m and 3.4336°",
    }],
    primary_example: "primary",
    assets: &[EGM96_ID],
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "navigation.vector.look-angles",
            reason: "alternative",
        },
        Related {
            id: "geodesy.height.convert",
            reason: "parent",
        },
        Related {
            id: "navigation.geodesic.inverse",
            reason: "alternative",
        },
    ],
    sentence: "The points are {slant_range} apart in a straight line, and {ground_distance} apart on the ground.",
    limits: &[("batchRows", 10_000)],
    run: run_distance,
    ..ToolDef::BLANK
};

fn run_distance(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat1, lon1) = point::read(ctx, "lat1", "lon1")?;
    let (lat2, lon2) = point::read(ctx, "lat2", "lon2")?;
    let mut h1 = height(ctx, "height1")?;
    let mut h2 = height(ctx, "height2")?;
    let msl1 = ctx.choice("reference1")? == Some("msl");
    let msl2 = ctx.choice("reference2")? == Some("msl");
    let egm96 = ctx.choice("geoid")? == Some("egm96");
    let fmt = ctx.options.format;
    let n = move |x: f64, d: u8| display::number(x, Precision::Decimals(d), fmt);
    let used = if !msl1 && !msl2 {
        "Both heights above the WGS 84 ellipsoid".to_owned()
    } else if egm96 {
        let bytes = ctx.asset(EGM96_ID, EGM96_VERSION, EGM96_FILE)?;
        let grid = Grid::parse(&bytes).map_err(|e| {
            ToolError::new(
                ErrorCode::AssetIntegrity,
                format!("The EGM96 grid could not be read: {e}."),
            )
        })?;
        let mut parts = Vec::new();
        for (i, msl, lat, lon, h) in [
            (1, msl1, lat1, lon1, &mut h1),
            (2, msl2, lat2, lon2, &mut h2),
        ] {
            if msl {
                let nn = grid.height(lat, lon, true);
                *h += nn;
                parts.push(format!("point {i} sea level + EGM96 N {} m", n(nn, 3)));
            } else {
                parts.push(format!("point {i} ellipsoidal"));
            }
        }
        format!("Heights: {}", parts.join("; "))
    } else if msl1 && msl2 {
        ctx.warnings.push(Warning::new(
            "ORTHOMETRIC_AS_ELLIPSOIDAL",
            "Both heights are above sea level and no geoid was chosen, so they were used as ellipsoidal heights. The height difference is right; the straight-line distance is off by about distance × N / 6,371 km (N, the geoid height, is up to about 100 m). Choose egm96 to remove it.",
        ));
        "Both heights above sea level, used as ellipsoidal (no geoid)".to_owned()
    } else {
        return Err(ToolError::invalid(
            "/geoid",
            "One height is above the ellipsoid and the other above sea level. Choose a geoid model (egm96) to reconcile them.",
        ));
    };
    let (a, b) = (ecef(lat1, lon1, h1), ecef(lat2, lon2, h2));
    let slant = libm::sqrt((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2) + (b.2 - a.2).powi(2));
    let (az, el, _) = fr::enu_to_aer(enu((lat1, lon1, h1), (lat2, lon2, h2)));
    let ground: f64 = Geodesic::wgs84().inverse(lat1, lon1, lat2, lon2);
    let dh = h2 - h1;
    let az = if ground == 0.0 {
        0.0
    } else {
        gp_base::angle::wrap_azimuth(az.to_degrees())
    };
    if ctx.explaining() {
        ctx.step(
            "Ground distance",
            "geodesic inverse on WGS 84",
            format!(
                "{}°, {}° to {}°, {}°",
                n(lat1, 6),
                n(lon1, 6),
                n(lat2, 6),
                n(lon2, 6)
            ),
            format!("{} m", n(ground, 3)),
        );
        ctx.step(
            "Straight-line distance",
            "√(ΔX² + ΔY² + ΔZ²) in Earth-centered coordinates",
            format!(
                "√({}² + {}² + {}²)",
                n(b.0 - a.0, 3),
                n(b.1 - a.1, 3),
                n(b.2 - a.2, 3)
            ),
            display::quantity(slant, "m", Precision::Decimals(3), fmt),
        );
    }
    Ok(Json::obj(vec![
        ("slant_range", ctx.out("slant_range", m(slant))),
        ("ground_distance", ctx.out("ground_distance", m(ground))),
        ("height_difference", ctx.out("height_difference", m(dh))),
        (
            "elevation_angle",
            ctx.out("elevation_angle", deg(el.to_degrees())),
        ),
        ("azimuth", ctx.out("azimuth", deg(az))),
        (
            "flat_earth_range",
            ctx.out("flat_earth_range", m(ground.hypot(dh))),
        ),
        (
            "flat_earth_elevation",
            ctx.out(
                "flat_earth_elevation",
                deg(libm::atan2(dh, ground).to_degrees()),
            ),
        ),
        ("heights_used", Json::str(used)),
    ]))
}

// ---------------------------------------------------------------- look angles

pub static LOOK_ANGLES: ToolDef = ToolDef {
    id: "navigation.vector.look-angles",
    title: "Look angles to a target",
    summary: "Azimuth, elevation, and straight-line range from an observer to a target, both with heights above the ellipsoid, with a warning when the Earth's curve hides the target.",
    aliases: &[
        "look angle calculator",
        "azimuth and elevation calculator",
        "antenna pointing angles",
        "AER calculator",
    ],
    keywords: &[
        "look angles",
        "azimuth",
        "elevation",
        "slant range",
        "antenna pointing",
        "horizon",
        "refraction",
    ],
    inputs: &[
        point::lat_field("observer_lat", "Observer latitude"),
        point::lon_field("observer_lon", "Observer longitude"),
        len(
            "observer_height",
            "Observer height",
            "Above the ellipsoid, like 10 m",
        )
        .required()
        .core(),
        point::lat_field("target_lat", "Target latitude"),
        point::lon_field("target_lon", "Target longitude"),
        len(
            "target_height",
            "Target height",
            "Above the ellipsoid, like 3000 m",
        )
        .required()
        .core(),
        Field::new(
            "k",
            "Refraction coefficient k",
            "0 none (default), 0.13 optical, 0.25 radio; for targets within the lower atmosphere",
            Kind::Number { min: 0.0, max: 0.5 },
        ),
    ],
    outputs: &[
        ang("azimuth", "Azimuth", "From true north, clockwise")
            .precision(Precision::Decimals(2))
            .angle_range("[0,360)"),
        ang(
            "elevation",
            "Elevation",
            "Above the observer's local horizontal; negative below it",
        )
        .precision(Precision::Decimals(4))
        .angle_range("unbounded"),
        len(
            "slant_range",
            "Straight-line range",
            "Through Earth-centered coordinates",
        )
        .precision(Precision::Decimals(3)),
        ang(
            "apparent_elevation",
            "Apparent elevation",
            "Raised by refraction, k × central angle / 2",
        )
        .precision(Precision::Decimals(4))
        .angle_range("unbounded")
        .optional(),
        ang(
            "horizon_elevation",
            "Horizon elevation",
            "Where the Earth's curve cuts the view, with the same refraction",
        )
        .precision(Precision::Decimals(4))
        .angle_range("unbounded"),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &[
        "BELOW_HORIZON",
        "INPUT_NORMALIZED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Both points to WGS 84 ECEF, then east-north-up at the observer: azimuth = atan2(E, N), elevation = atan2(U, √(E² + N²)). The horizon is at −acos(Rₑ / (Rₑ + h)) with Rₑ = 6,371 km / (1 − k); refraction raises the elevation by k × (ground distance / 6,371 km) / 2",
    accuracy: "Exact geometry; the horizon and refraction use a mean-radius sphere and a single refraction coefficient, and ignore terrain.",
    references: &[LOCAL_CARTESIAN],
    examples: &[Example {
        id: "primary",
        title: "A 3,000 m aircraft 150 km east of a 10 m mast",
        input: r#"{"observer_lat":40,"observer_lon":-105,"observer_height":"10 m","target_lat":40,"target_lon":-103.24,"target_height":"3000 m"}"#,
        source: "Local east-north-up from GeographicLib LocalCartesian conventions",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "navigation.vector.distance-3d",
            reason: "alternative",
        },
        Related {
            id: "navigation.los.visibility",
            reason: "next",
        },
        Related {
            id: "geodesy.frame.to-local",
            reason: "alternative",
        },
    ],
    sentence: "Point at {azimuth} and {elevation}, {slant_range} away.{warn BELOW_HORIZON} The Earth's curve hides the target.{/warn}",
    limits: &[("batchRows", 10_000)],
    run: run_look,
    ..ToolDef::BLANK
};

fn run_look(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (la1, lo1) = point::read(ctx, "observer_lat", "observer_lon")?;
    let (la2, lo2) = point::read(ctx, "target_lat", "target_lon")?;
    let h1 = height(ctx, "observer_height")?;
    let h2 = height(ctx, "target_height")?;
    let k = ctx.number("k")?;
    let e = enu((la1, lo1, h1), (la2, lo2, h2));
    let (az, el, range) = fr::enu_to_aer(e);
    let az = if e[0] == 0.0 && e[1] == 0.0 {
        0.0
    } else {
        gp_base::angle::wrap_azimuth(az.to_degrees())
    };
    let el = el.to_degrees();
    let kk = k.unwrap_or(0.0);
    let ground: f64 = Geodesic::wgs84().inverse(la1, lo1, la2, lo2);
    let lift = (kk * ground / R_DEFAULT / 2.0).to_degrees();
    let re = R_DEFAULT / (1.0 - kk);
    let horizon = -acos(re / (re + h1.max(0.0))).to_degrees();
    let seen = el + lift;
    if seen < horizon {
        let fmt = ctx.options.format;
        ctx.warnings.push(Warning::new(
            "BELOW_HORIZON",
            format!(
                "The target is {}° below the horizon, so the Earth's curve blocks the straight line (terrain aside).",
                display::number(horizon - seen, Precision::Decimals(3), fmt)
            ),
        ));
    }
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "East, north, up at the observer",
            "rotate the Earth-centered offset into the local frame",
            format!("{} m apart", n(range, 3)),
            format!("E {} m, N {} m, U {} m", n(e[0], 3), n(e[1], 3), n(e[2], 3)),
        );
        ctx.step(
            "Azimuth",
            "atan2(E, N)",
            format!("atan2({}, {})", n(e[0], 3), n(e[1], 3)),
            display::quantity(az, "deg", Precision::Decimals(2), fmt),
        );
    }
    let mut out = vec![
        ("azimuth", ctx.out("azimuth", deg(az))),
        ("elevation", ctx.out("elevation", deg(el))),
        ("slant_range", ctx.out("slant_range", m(range))),
    ];
    if k.is_some() {
        out.push((
            "apparent_elevation",
            ctx.out("apparent_elevation", deg(seen)),
        ));
    }
    out.push((
        "horizon_elevation",
        ctx.out("horizon_elevation", deg(horizon)),
    ));
    Ok(Json::obj(out))
}

// ---------------------------------------------------------------- vector algebra

const CONVENTION: Field = Field::new(
    "convention",
    "Direction measured",
    "navigational (clockwise from north, default) or mathematical (counterclockwise from +x, east)",
    Kind::Choice(&["navigational", "mathematical"]),
);

const fn num(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Number {
            min: -1e15,
            max: 1e15,
        },
    )
}

/// Direction of (x east, y north) in the chosen convention, degrees in [0, 360).
fn direction(x: f64, y: f64, nav: bool) -> f64 {
    let a = if nav {
        libm::atan2(x, y)
    } else {
        libm::atan2(y, x)
    };
    gp_base::angle::wrap_azimuth(a.to_degrees())
}

fn convention_note(nav: bool) -> &'static str {
    if nav {
        "Directions clockwise from north (+y); x is east, y is north, z is up."
    } else {
        "Directions counterclockwise from +x (east); y is north, z is up."
    }
}

pub static POLAR_CARTESIAN: ToolDef = ToolDef {
    id: "navigation.vector.polar-cartesian",
    title: "Vector components and direction",
    summary: "Turns a magnitude and direction (and elevation, for 3D) into x, y, and z components, or components back into magnitude and direction, with the direction convention stated: navigational from north or mathematical from +x.",
    aliases: &[
        "polar to cartesian",
        "cartesian to polar",
        "vector components calculator",
        "resolve a vector",
    ],
    keywords: &[
        "vector",
        "components",
        "polar",
        "cartesian",
        "magnitude",
        "direction",
        "spherical",
    ],
    inputs: &[
        num("magnitude", "Magnitude", "Like 10, in any unit").core(),
        ang("direction", "Direction", "Like 090").core(),
        num(
            "x",
            "x component",
            "Toward east, instead of magnitude and direction, like 10",
        )
        .core(),
        num("y", "y component", "Toward north, like 0").core(),
        CONVENTION.core(),
        ang(
            "elevation",
            "Elevation",
            "Above the horizontal, for a 3D vector, like 30; default 0",
        ),
        num("z", "z component", "Up, for a 3D vector, like 5; default 0"),
    ],
    outputs: &[
        num("magnitude", "Magnitude", "√(x² + y² + z²)").precision(Precision::Decimals(6)),
        num("x", "x component", "Toward east").precision(Precision::Decimals(6)),
        num("y", "y component", "Toward north").precision(Precision::Decimals(6)),
        num("z", "z component", "Up, for 3D")
            .precision(Precision::Decimals(6))
            .optional(),
        ang("direction", "Direction", "In the convention chosen")
            .precision(Precision::Decimals(4))
            .angle_range("[0,360)"),
        ang("elevation", "Elevation", "Above the horizontal, for 3D")
            .precision(Precision::Decimals(4))
            .angle_range("unbounded")
            .optional(),
        Field::new(
            "convention_used",
            "Convention",
            "How the direction is measured",
            Kind::Text { max_len: 120 },
        ),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Navigational: x = m·cos e·sin θ, y = m·cos e·cos θ; mathematical: x = m·cos e·cos θ, y = m·cos e·sin θ; z = m·sin e. Back: m = √(x² + y² + z²), θ = atan2 in the chosen convention, e = atan2(z, √(x² + y²))",
    accuracy: "Exact arithmetic",
    references: &[LOCAL_CARTESIAN],
    examples: &[Example {
        id: "primary",
        title: "Magnitude 10 at 090, navigational",
        input: r#"{"magnitude":10,"direction":"090 deg"}"#,
        source: "add-navigation-and-geometry navigational-convention scenario: (10, 0)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "navigation.vector.operations",
            reason: "next",
        },
        Related {
            id: "navigation.vector.look-angles",
            reason: "alternative",
        },
    ],
    sentence: "The vector has components {x} east and {y} north, magnitude {magnitude}.",
    limits: &[("batchRows", 10_000)],
    run: run_polar,
    ..ToolDef::BLANK
};

fn run_polar(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let dg = unit(QT::Angle, "deg");
    let nav = ctx.choice("convention")? != Some("mathematical");
    let mag = ctx.number("magnitude")?;
    let dir = ctx.quantity("direction")?.map(|q| q.to(dg));
    let el = ctx.quantity("elevation")?.map(|q| q.to(dg));
    let (xi, yi, zi) = (ctx.number("x")?, ctx.number("y")?, ctx.number("z")?);
    let polar = mag.is_some() || dir.is_some() || el.is_some();
    let cart = xi.is_some() || yi.is_some() || zi.is_some();
    let (x, y, z, three_d) = match (polar, cart) {
        (true, false) => {
            let (Some(m), Some(t)) = (mag, dir) else {
                return Err(ToolError::invalid(
                    if mag.is_none() {
                        "/magnitude"
                    } else {
                        "/direction"
                    },
                    "Give both a magnitude and a direction.",
                ));
            };
            let e = el.unwrap_or(0.0);
            if !(-90.0..=90.0).contains(&e) {
                return Err(ToolError::invalid(
                    "/elevation",
                    "Elevation is between -90° and 90°.",
                ));
            }
            let h = m * e.to_radians().cos();
            let (s, c) = (t.to_radians().sin(), t.to_radians().cos());
            let (x, y) = if nav { (h * s, h * c) } else { (h * c, h * s) };
            (x, y, m * e.to_radians().sin(), el.is_some())
        }
        (false, true) => {
            let (Some(x), Some(y)) = (xi, yi) else {
                return Err(ToolError::invalid(
                    if xi.is_none() { "/x" } else { "/y" },
                    "Give both x and y.",
                ));
            };
            (x, y, zi.unwrap_or(0.0), zi.is_some())
        }
        (true, true) => {
            return Err(ToolError::invalid(
                "/x",
                "Give a magnitude and direction, or components, not both.",
            ));
        }
        (false, false) => {
            return Err(ToolError::invalid(
                "/magnitude",
                "Give a magnitude and direction, or x and y components.",
            )
            .hint("Example: magnitude 10, direction 090"));
        }
    };
    let m = libm::sqrt(x * x + y * y + z * z);
    let theta = direction(x, y, nav);
    let elev = libm::atan2(z, libm::hypot(x, y)).to_degrees();
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |v: f64, d: u8| display::number(v, Precision::Decimals(d), fmt);
        if polar {
            ctx.step(
                "Components",
                if nav {
                    "x = m·cos e·sin θ, y = m·cos e·cos θ, z = m·sin e"
                } else {
                    "x = m·cos e·cos θ, y = m·cos e·sin θ, z = m·sin e"
                },
                format!(
                    "m = {}, θ = {}°, e = {}°",
                    n(mag.unwrap_or(m), 6),
                    n(dir.unwrap_or(theta), 4),
                    n(el.unwrap_or(0.0), 4)
                ),
                format!("({}, {}, {})", n(x, 6), n(y, 6), n(z, 6)),
            );
        } else {
            ctx.step(
                "Direction",
                if nav {
                    "θ = atan2(x, y), clockwise from north"
                } else {
                    "θ = atan2(y, x), counterclockwise from +x"
                },
                format!("x = {}, y = {}", n(x, 6), n(y, 6)),
                format!("{}°", n(theta, 4)),
            );
        }
        ctx.step(
            "Magnitude",
            "m = √(x² + y² + z²)",
            format!("√({}² + {}² + {}²)", n(x, 6), n(y, 6), n(z, 6)),
            n(m, 6),
        );
    }
    let mut out = vec![
        ("magnitude", Json::Num(m)),
        ("x", Json::Num(x)),
        ("y", Json::Num(y)),
    ];
    if three_d {
        out.push(("z", Json::Num(z)));
    }
    out.push(("direction", ctx.out("direction", deg(theta))));
    if three_d {
        out.push(("elevation", ctx.out("elevation", deg(elev))));
    }
    out.push(("convention_used", Json::str(convention_note(nav))));
    Ok(Json::obj(out))
}

const VECTOR_ROW: &[Field] = &[
    num("x", "x component", "Toward east, like 3").required(),
    num("y", "y component", "Toward north, like 4").required(),
    num("z", "z component", "Up, for 3D; default 0"),
];

pub static OPERATIONS: ToolDef = ToolDef {
    id: "navigation.vector.operations",
    title: "Vector sum, dot, and cross product",
    summary: "Adds any number of 2D or 3D vectors head to tail; for two vectors, also their difference, dot and cross products, the angle between them, and the projection of one on the other.",
    aliases: &[
        "vector addition calculator",
        "dot product calculator",
        "cross product calculator",
        "angle between vectors",
        "resultant vector",
    ],
    keywords: &[
        "vector",
        "resultant",
        "sum",
        "dot product",
        "cross product",
        "projection",
        "angle between",
        "unit vector",
    ],
    inputs: &[
        Field::new(
            "vectors",
            "Vectors",
            "x, y, and optional z of each, like 3, 4",
            Kind::List {
                items: VECTOR_ROW,
                min: 1,
                max: 1000,
            },
        )
        .required()
        .core(),
        CONVENTION.core(),
        num("scale", "Scale factor", "Multiplies the resultant, like 2"),
    ],
    outputs: &[
        num("magnitude", "Resultant magnitude", "|Σ vectors|").precision(Precision::Decimals(6)),
        num("x", "Resultant x", "Σ x").precision(Precision::Decimals(6)),
        num("y", "Resultant y", "Σ y").precision(Precision::Decimals(6)),
        num("z", "Resultant z", "Σ z, for 3D")
            .precision(Precision::Decimals(6))
            .optional(),
        ang(
            "direction",
            "Resultant direction",
            "In the convention chosen",
        )
        .precision(Precision::Decimals(4))
        .angle_range("[0,360)")
        .optional(),
        Field::new(
            "unit_vector",
            "Unit vector",
            "The resultant ÷ its magnitude",
            Kind::Text { max_len: 120 },
        )
        .optional(),
        Field::new(
            "scaled",
            "Scaled resultant",
            "Scale factor × resultant",
            Kind::Text { max_len: 120 },
        )
        .optional(),
        Field::new(
            "difference",
            "Difference",
            "First − second, for two vectors",
            Kind::Text { max_len: 120 },
        )
        .optional(),
        num("dot", "Dot product", "a·b, for two vectors")
            .precision(Precision::Decimals(6))
            .optional(),
        Field::new(
            "cross",
            "Cross product",
            "a × b, for two vectors",
            Kind::Text { max_len: 120 },
        )
        .optional(),
        ang("angle_between", "Angle between", "acos(a·b / |a||b|)")
            .precision(Precision::Decimals(4))
            .angle_range("unbounded")
            .optional(),
        num(
            "projection",
            "Projection of the first on the second",
            "a·b / |b|, signed",
        )
        .precision(Precision::Decimals(6))
        .optional(),
        Field::new(
            "convention_used",
            "Convention",
            "How the direction is measured",
            Kind::Text { max_len: 120 },
        ),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Component-wise sums; a·b = Σ aᵢbᵢ; a × b = (a_y b_z − a_z b_y, a_z b_x − a_x b_z, a_x b_y − a_y b_x); angle = acos(a·b / |a||b|); projection = a·b / |b|",
    accuracy: "Exact arithmetic. All vectors must share one unit; the tool does not convert them",
    references: &[LOCAL_CARTESIAN],
    examples: &[Example {
        id: "primary",
        title: "Three vectors head to tail",
        input: r#"{"vectors":[{"x":3,"y":4},{"x":-1,"y":2},{"x":2,"y":-3}]}"#,
        source: "Component-wise sum: (4, 3), magnitude 5, direction 053.1301° from north",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "navigation.vector.polar-cartesian",
            reason: "parent",
        },
        Related {
            id: "navigation.route.cpa",
            reason: "next",
        },
    ],
    sentence: "The resultant has a magnitude of {magnitude}.",
    limits: &[("batchRows", 1_000)],
    run: run_operations,
    ..ToolDef::BLANK
};

fn triple(v: [f64; 3], three_d: bool, fmt: gp_base::parse::NumberFormat) -> String {
    let n = |x: f64| display::number(x, Precision::Decimals(6), fmt);
    if three_d {
        format!("({}, {}, {})", n(v[0]), n(v[1]), n(v[2]))
    } else {
        format!("({}, {})", n(v[0]), n(v[1]))
    }
}

fn run_operations(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let rows = ctx.rows("vectors")?;
    let mut vs: Vec<[f64; 3]> = Vec::with_capacity(rows.len());
    let mut three_d = false;
    for (i, row) in rows.iter().enumerate() {
        let mut v = [0.0; 3];
        for (k, name) in ["x", "y", "z"].into_iter().enumerate() {
            let raw = row.get(name).filter(|x| !x.is_null());
            let val = match raw {
                None => continue,
                Some(x) => x
                    .as_f64()
                    .or_else(|| x.as_str().and_then(|s| s.trim().parse().ok())),
            };
            match val {
                Some(f) if f.is_finite() && f.abs() <= 1e15 => {
                    v[k] = f;
                    three_d |= k == 2;
                }
                _ => {
                    return Err(ToolError::invalid(
                        &format!("/vectors/{i}/{name}"),
                        format!("{name} must be a number up to 1e15 in size."),
                    ));
                }
            }
        }
        vs.push(v);
    }
    let nav = ctx.choice("convention")? != Some("mathematical");
    let scale = ctx.number("scale")?;
    let fmt = ctx.options.format;
    let sum = vs
        .iter()
        .fold([0.0; 3], |a, v| [a[0] + v[0], a[1] + v[1], a[2] + v[2]]);
    let norm = |v: [f64; 3]| libm::sqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2]);
    let mag = norm(sum);
    if ctx.explaining() {
        let n = move |v: f64| display::number(v, Precision::Decimals(6), fmt);
        let list = |k: usize| vs.iter().map(|v| n(v[k])).collect::<Vec<_>>().join(" + ");
        ctx.step(
            "Add the components",
            "Σ x, Σ y, Σ z",
            format!("x: {}; y: {}", list(0), list(1)),
            triple(sum, three_d, fmt),
        );
        ctx.step(
            "Resultant magnitude",
            "√(x² + y² + z²)",
            format!("√({}² + {}² + {}²)", n(sum[0]), n(sum[1]), n(sum[2])),
            n(mag),
        );
    }
    let mut out = vec![
        ("magnitude", Json::Num(mag)),
        ("x", Json::Num(sum[0])),
        ("y", Json::Num(sum[1])),
    ];
    if three_d {
        out.push(("z", Json::Num(sum[2])));
    }
    if mag > 0.0 {
        out.push((
            "direction",
            ctx.out("direction", deg(direction(sum[0], sum[1], nav))),
        ));
        out.push((
            "unit_vector",
            Json::str(triple(sum.map(|c| c / mag), three_d, fmt)),
        ));
    }
    if let Some(k) = scale {
        out.push((
            "scaled",
            Json::str(triple(sum.map(|c| c * k), three_d, fmt)),
        ));
    }
    if let [a, b] = vs[..] {
        let dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let cross = [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ];
        out.push((
            "difference",
            Json::str(triple(
                [a[0] - b[0], a[1] - b[1], a[2] - b[2]],
                three_d,
                fmt,
            )),
        ));
        out.push(("dot", Json::Num(dot)));
        // A 2D cross product still points along z.
        out.push(("cross", Json::str(triple(cross, true, fmt))));
        let (na, nb) = (norm(a), norm(b));
        if na > 0.0 && nb > 0.0 {
            let c = (dot / (na * nb)).clamp(-1.0, 1.0);
            out.push((
                "angle_between",
                ctx.out("angle_between", deg(acos(c).to_degrees())),
            ));
        }
        if nb > 0.0 {
            out.push(("projection", Json::Num(dot / nb)));
        }
    }
    out.push(("convention_used", Json::str(convention_note(nav))));
    Ok(Json::obj(out))
}
