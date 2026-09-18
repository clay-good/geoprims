//! Geoid and height tools (geodesy/heights-and-geoid spec): geoid height N from
//! EGM96, and ellipsoidal ↔ orthometric height conversion, h = H + N. The grid
//! is a host-supplied asset (`egm96-15`); the math lives in gp-geo.

use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::geoid::Grid;
use gp_geo::point;

const EGM96_REF: Reference = Reference {
    title: "The Development of the Joint NASA GSFC and NIMA Geopotential Model EGM96, NASA/TP-1998-206861",
    issuer: "Lemoine, F. G., et al., NASA Goddard Space Flight Center",
    year: 1998,
    edition: "NASA/TP-1998-206861",
    locator: "Geoid undulations relative to WGS 84",
    url: "https://ntrs.nasa.gov/citations/19980218814",
};
const GEOGRAPHICLIB_GEOID: Reference = Reference {
    title: "GeographicLib Geoid class and geoid data",
    issuer: "Karney, C. F. F., GeographicLib",
    year: 2022,
    edition: "GeographicLib 2.x (egm96-15 grid, 2009-08-29)",
    locator: "Geoid height interpolation: bilinear and 12-point cubic least-squares fit",
    url: "https://geographiclib.sourceforge.io/C++/doc/geoid.html",
};

pub const EGM96_ID: &str = "egm96-15";
pub const EGM96_VERSION: &str = "2009-08-29";
const EGM96_FILE: &str = "egm96-15.pgm";

const LAT: Field = point::lat_field("lat", "Latitude");
const LON: Field = point::lon_field("lon", "Longitude");
const MODEL: Field = Field::new(
    "model",
    "Geoid model",
    "egm96 (global, default)",
    Kind::Choice(&["egm96"]),
);
const INTERP: Field = Field::new(
    "interpolation",
    "Interpolation",
    "cubic (default, error under 0.17 m) or bilinear (under 1.2 m)",
    Kind::Choice(&["cubic", "bilinear"]),
);

fn m(v: f64) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(QT::Length, "m").expect("m"),
    }
}

/// N (m) at the point, from the host-supplied grid.
fn geoid_height(ctx: &mut Ctx, lat: f64, lon: f64) -> Result<(f64, f64), ToolError> {
    let cubic = ctx.choice("interpolation")? != Some("bilinear");
    let bytes = ctx.asset(EGM96_ID, EGM96_VERSION, EGM96_FILE)?;
    let grid = Grid::parse(&bytes).map_err(|e| {
        ToolError::new(
            ErrorCode::AssetIntegrity,
            format!("The EGM96 grid could not be read: {e}."),
        )
    })?;
    let err = if cubic {
        grid.max_cubic_error
    } else {
        grid.max_bilinear_error
    }
    .unwrap_or(f64::NAN);
    ctx.model = Some(format!(
        "EGM96 geoid, 15′ grid, {} interpolation",
        if cubic { "12-point cubic" } else { "bilinear" }
    ));
    Ok((grid.height(lat, lon, cubic), err))
}

fn msl_note(ctx: &mut Ctx) {
    ctx.warnings.push(Warning::new(
        "ORTHOMETRIC_AS_ELLIPSOIDAL",
        "EGM96 heights approximate global mean sea level. They are not NAVD 88 (use GEOID18 in the US) and can differ from local vertical datums by a meter or more.",
    ));
}

// ---------------------------------------------------------------- geoid height

pub static GEOID_HEIGHT: ToolDef = ToolDef {
    id: "geodesy.geoid.geoid-height",
    title: "Geoid height (EGM96)",
    summary: "The geoid height N (geoid undulation) above the WGS 84 ellipsoid at any point from the EGM96 global geoid, the number that turns GPS ellipsoidal heights into heights above mean sea level.",
    aliases: &[
        "geoid height calculator",
        "geoid undulation",
        "EGM96 calculator",
        "geoid separation",
    ],
    keywords: &[
        "geoid",
        "EGM96",
        "undulation",
        "N",
        "ellipsoid height",
        "MSL",
        "orthometric",
    ],
    inputs: &[LAT, LON, MODEL.core(), INTERP],
    outputs: &[
        Field::new(
            "geoid_height",
            "Geoid height N",
            "Geoid above the WGS 84 ellipsoid",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "interpolation_error",
            "Interpolation error bound",
            "The grid's stated maximum error for this interpolation",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(3)),
    ],
    errors: &[ErrorCode::AssetUnavailable, ErrorCode::AssetIntegrity],
    warnings: &[
        "ORTHOMETRIC_AS_ELLIPSOIDAL",
        "INPUT_NORMALIZED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "EGM96 geoid on a 15′ grid (GeographicLib packaging), cubic or bilinear interpolation",
    accuracy: "Matches GeographicLib GeoidEval at its printed 0.1 mm on 2,010 points, including both poles. The grid itself departs from full EGM96 by up to 0.17 m (cubic); EGM96 is good to about 0.5-1 m worldwide.",
    references: &[EGM96_REF, GEOGRAPHICLIB_GEOID],
    examples: &[Example {
        id: "primary",
        title: "Timbuktu, Mali",
        input: r#"{"lat":16.776,"lon":-3.009}"#,
        source: "GeographicLib GeoidEval with egm96-15 (cubic): 28.7079 m",
    }],
    primary_example: "primary",
    assets: &[EGM96_ID],
    visualization: &[Layer {
        kind: "point",
        map: &[("value", "geoid_height")],
    }],
    related: &[Related {
        id: "geodesy.height.convert",
        reason: "next",
    }],
    sentence: "The geoid is {geoid_height} above the ellipsoid here.",
    limits: &[("batchRows", 10_000)],
    run: run_geoid,
    ..ToolDef::BLANK
};

fn run_geoid(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let (n, err) = geoid_height(ctx, lat, lon)?;
    msl_note(ctx);
    Ok(Json::obj(vec![
        ("geoid_height", ctx.out("geoid_height", m(n))),
        (
            "interpolation_error",
            ctx.out("interpolation_error", m(err)),
        ),
    ]))
}

// ---------------------------------------------------------------- height conversion

pub static HEIGHT_CONVERT: ToolDef = ToolDef {
    id: "geodesy.height.convert",
    title: "Ellipsoidal and orthometric height",
    summary: "Converts a GPS ellipsoidal height to height above mean sea level (orthometric) or back, with h = H + N and the EGM96 geoid height N.",
    aliases: &[
        "ellipsoid height to MSL",
        "HAE to MSL",
        "GPS height to sea level",
        "orthometric height calculator",
    ],
    keywords: &[
        "ellipsoidal height",
        "orthometric height",
        "HAE",
        "MSL",
        "geoid",
        "EGM96",
        "GPS altitude",
    ],
    inputs: &[
        LAT,
        LON,
        Field::new(
            "height",
            "Height",
            "Like 1655 m",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .required()
        .core(),
        Field::new(
            "from",
            "Height type",
            "ellipsoidal (HAE, default) or orthometric (MSL)",
            Kind::Choice(&["ellipsoidal", "orthometric"]),
        )
        .core(),
        MODEL,
        INTERP,
    ],
    outputs: &[
        Field::new(
            "converted",
            "Converted height",
            "Orthometric H = h − N, or ellipsoidal h = H + N",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "ellipsoidal",
            "Ellipsoidal height h",
            "Above the WGS 84 ellipsoid",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "orthometric",
            "Orthometric height H",
            "Above the EGM96 geoid (mean sea level)",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "geoid_height",
            "Geoid height N",
            "Geoid above the ellipsoid",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(3)),
    ],
    errors: &[ErrorCode::AssetUnavailable, ErrorCode::AssetIntegrity],
    warnings: &[
        "ORTHOMETRIC_AS_ELLIPSOIDAL",
        "INPUT_NORMALIZED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "h = H + N with N from the EGM96 15′ grid",
    accuracy: "As good as the geoid: about 0.5-1 m for EGM96. Exact arithmetic otherwise.",
    references: &[EGM96_REF, GEOGRAPHICLIB_GEOID],
    examples: &[Example {
        id: "primary",
        title: "A GPS height of 100 m at Timbuktu",
        input: r#"{"lat":16.776,"lon":-3.009,"height":"100 m"}"#,
        source: "h − N with N = 28.7079 m from GeographicLib GeoidEval (egm96-15, cubic): 71.292 m above sea level",
    }],
    primary_example: "primary",
    assets: &[EGM96_ID],
    visualization: &[Layer {
        kind: "profile-chart",
        map: &[("value", "converted")],
    }],
    related: &[Related {
        id: "geodesy.geoid.geoid-height",
        reason: "parent",
    }],
    sentence: "The converted height is {converted}. The geoid is {geoid_height} above the ellipsoid here.",
    limits: &[("batchRows", 10_000)],
    run: run_convert,
    ..ToolDef::BLANK
};

fn run_convert(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let h = ctx.req_quantity("height")?;
    let (n, _) = geoid_height(ctx, lat, lon)?;
    msl_note(ctx);
    let from_ortho = ctx.choice("from")? == Some("orthometric");
    let (ell, orth) = if from_ortho {
        (h.base() + n, h.base())
    } else {
        (h.base(), h.base() - n)
    };
    let unit = h.unit;
    let conv = if from_ortho { ell } else { orth };
    Ok(Json::obj(vec![
        ("converted", ctx.emit("converted", m(conv), unit)),
        ("ellipsoidal", ctx.emit("ellipsoidal", m(ell), unit)),
        ("orthometric", ctx.emit("orthometric", m(orth), unit)),
        ("geoid_height", ctx.out("geoid_height", m(n))),
    ]))
}
