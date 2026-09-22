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
