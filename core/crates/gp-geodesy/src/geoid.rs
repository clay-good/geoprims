//! Geoid and height tools (geodesy/heights-and-geoid spec): geoid height N from
//! EGM96, and ellipsoidal ↔ orthometric height conversion, h = H + N. The grid
//! is a host-supplied asset (`egm96-15`); the math lives in gp-geo.

use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{
    Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, Stability, ToolDef,
};
use gp_base::units::{self, Quantity as QT};
use gp_geo::ellipsoid::CATALOG;
use gp_geo::frames as fr;
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

// ------------------------------------------------------------ frame of h

/// The frame an ellipsoidal height is on. EGM96 is referred to WGS 84, and a
/// US survey height (an OPUS solution, a CORS position) is on NAD 83 (2011):
/// across the conterminous United States the two differ in height by about
/// 0.24 m in the northwest to 1.72 m in the southeast (measured at 2026.0).
const FRAME_IN: Field = Field::new(
    "frame",
    "Frame of the ellipsoidal height",
    "WGS84 (the default: a satellite receiver or a drone) or NAD83(2011) (an OPUS or a US survey height), like NAD83(2011)",
    Kind::Choice(&["WGS84", "NAD83(2011)"]),
);

const EPOCH_IN: Field = Field::new(
    "epoch",
    "Epoch",
    "With NAD83(2011), the date the step to WGS 84 is computed for: a date or a decimal year, like 2026.87",
    Kind::Text { max_len: 40 },
);

const FRAME_STEP_OUT: Field = Field::new(
    "frame_step",
    "Frame step",
    "WGS 84 ellipsoidal height less NAD 83 (2011) ellipsoidal height at the point and epoch, applied before using EGM96",
    Kind::Quantity {
        q: QT::Length,
        unit: "m",
    },
)
.precision(Precision::Decimals(3))
.optional();

/// NAD 83 (2011)'s reference epoch, at which a mismatch is sized when the
/// caller gives no epoch.
const NAD83_REFERENCE_EPOCH: f64 = 2010.0;

/// What to do about the height's frame: nothing (it is WGS 84), apply the
/// step at an epoch, or warn that the frames differ.
enum FrameStep {
    Wgs84,
    Applied(f64),
    Mismatch,
}

fn frame_step(ctx: &mut Ctx) -> Result<FrameStep, ToolError> {
    let nad83 = ctx.choice("frame")? == Some("NAD83(2011)");
    let t = super::datum::epoch(ctx, "epoch")?;
    if let Some(t) = t
        && !(1980.0..=2100.0).contains(&t)
    {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The epoch must be between 1980 and 2100.",
        )
        .at("/epoch"));
    }
    Ok(match (nad83, t) {
        (false, Some(_)) => {
            ctx.warnings.push(Warning::new(
                "INPUT_NORMALIZED",
                "The epoch only matters for a NAD 83 (2011) height, so it was not used.",
            ));
            FrameStep::Wgs84
        }
        (false, None) => FrameStep::Wgs84,
        (true, Some(t)) => {
            ctx.context.push(("epoch", Json::Num(t)));
            FrameStep::Applied(t)
        }
        (true, None) => FrameStep::Mismatch,
    })
}

/// An ellipsoidal height moved between NAD 83 (2011) and WGS 84 (G2296),
/// which is aligned with ITRF2020, by NGS's HTDP parameters at epoch `t`.
fn shift_height(lat: f64, lon: f64, h: f64, from: &str, to: &str, t: f64) -> f64 {
    let grs80 = CATALOG
        .iter()
        .find(|e| e.id == "grs80")
        .copied()
        .expect("the ellipsoid catalog lists grs80");
    let (phi, lam) = (lat.to_radians(), lon.to_radians());
    let p = fr::to_ecef(&grs80, phi, lam, h);
    let q = gp_geo::htdp::transform(from, to, [p.0, p.1, p.2], t)
        .expect("both frames are in HTDP's table");
    fr::from_ecef(&grs80, q[0], q[1], q[2])
        .expect("not the Earth's center")
        .2
}

/// The WGS 84 height of a NAD 83 (2011) one.
fn nad83_to_wgs84(lat: f64, lon: f64, h: f64, t: f64) -> f64 {
    shift_height(lat, lon, h, "NAD83(2011)", "ITRF2020", t)
}

/// The NAD 83 (2011) height of a WGS 84 one.
fn wgs84_to_nad83(lat: f64, lon: f64, h: f64, t: f64) -> f64 {
    shift_height(lat, lon, h, "ITRF2020", "NAD83(2011)", t)
}

/// The FRAME_MISMATCH warning, sized at NAD 83 (2011)'s reference epoch.
fn mismatch(ctx: &mut Ctx, lat: f64, lon: f64, h: f64) {
    let dh = nad83_to_wgs84(lat, lon, h, NAD83_REFERENCE_EPOCH) - h;
    ctx.warnings.push(
        Warning::new(
            "FRAME_MISMATCH",
            format!(
                "EGM96 is referred to WGS 84, and this height is on NAD 83 (2011). Here the two ellipsoidal heights differ by about {:.2} m (computed for 2010.0, NAD 83 (2011)'s reference epoch), and the answer treats the height as WGS 84, so it is off by about that much. Give the epoch and the tool applies the step instead.",
                dh.abs()
            ),
        )
        .at("/frame"),
    );
}

/// The WGS 84 height to subtract N from, given an ellipsoidal height on the
/// caller's frame, and the frame step applied (WGS 84 less NAD 83 (2011)).
fn wgs84_from(ctx: &mut Ctx, lat: f64, lon: f64, h: f64, step: &FrameStep) -> (f64, Option<f64>) {
    match *step {
        FrameStep::Wgs84 => (h, None),
        FrameStep::Applied(t) => {
            let wgs = nad83_to_wgs84(lat, lon, h, t);
            (wgs, Some(wgs - h))
        }
        FrameStep::Mismatch => {
            mismatch(ctx, lat, lon, h);
            (h, None)
        }
    }
}

/// The ellipsoidal height on the caller's frame, given the WGS 84 one that
/// H + N makes, and the frame step applied (WGS 84 less NAD 83 (2011)).
fn ellipsoidal_from(
    ctx: &mut Ctx,
    lat: f64,
    lon: f64,
    wgs: f64,
    step: &FrameStep,
) -> (f64, Option<f64>) {
    match *step {
        FrameStep::Wgs84 => (wgs, None),
        FrameStep::Applied(t) => {
            let nad = wgs84_to_nad83(lat, lon, wgs, t);
            (nad, Some(wgs - nad))
        }
        FrameStep::Mismatch => {
            mismatch(ctx, lat, lon, wgs);
            (wgs, None)
        }
    }
}

// ---------------------------------------------------------------- geoid height

pub static GEOID_HEIGHT: ToolDef = ToolDef {
    id: "geodesy.geoid.geoid-height",
    version: "1.0.1",
    stability: gp_base::tool::Stability::Stable,
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
    when_to_use: "Use this whenever a GPS height has to become a height above mean sea level: the separation N between the ellipsoid and the geoid at a point is the number that connects them, and it reaches tens of meters. It is also what a drone, a survey, or an elevation product needs before its heights can be compared with published ones.",
    limitations: "This is the EGM96 global model: it is a smooth global surface, and a national or regional geoid model, such as a GEOID published for your country, is the one a survey is required to use. The gridded form here departs from full EGM96 by up to about 0.17 m, and the model itself has its own error against the true geoid.",
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
    related: &[
        Related {
            id: "geodesy.height.convert",
            reason: "next",
        },
        Related {
            id: "geodesy.parse.coordinates",
            reason: "parent",
        },
        Related {
            id: "survey.reduction.combined-factor",
            reason: "next",
        },
    ],
    sentence: "The geoid is {abs(geoid_height)} {if geoid_height < 0}below{else}above{/if} the ellipsoid here.",
    limits: &[("batchRows", 10_000)],
    run: run_geoid,
    ..ToolDef::BLANK
};

fn run_geoid(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let (n, err) = geoid_height(ctx, lat, lon)?;
    msl_note(ctx);
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let nn = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        // A grid lookup has no formula to substitute; what a reader needs is
        // which grid, where in it, and how much the interpolation can be off.
        ctx.step(
            "Where in the grid",
            "the EGM96 15-arc-minute grid, 0.25° between posts",
            format!("{}°, {}°", nn(lat, 6), nn(lon, 6)),
            format!("{}° cell", nn(0.25, 2)),
        );
        ctx.step(
            "Geoid height",
            "N = bilinear interpolation between the four surrounding posts",
            format!(
                "at {}°, {}°, ±{} m from interpolation",
                nn(lat, 6),
                nn(lon, 6),
                nn(err, 3)
            ),
            format!("{} m", nn(n, 3)),
        );
    }
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
    version: "1.1.0",
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
            "ellipsoidal (HAE, default), orthometric (MSL), or agl above the terrain you give",
            Kind::Choice(&["ellipsoidal", "orthometric", "agl"]),
        )
        .core(),
        Field::new(
            "terrain",
            "Terrain elevation",
            "The ground at this point, above mean sea level, like 250 m",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .core(),
        MODEL,
        INTERP,
        FRAME_IN,
        EPOCH_IN,
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
            "Above the ellipsoid, on the frame given (WGS 84 unless NAD 83 (2011) is chosen)",
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
        Field::new(
            "agl",
            "Height above ground",
            "Orthometric height less the terrain elevation",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(3)),
        FRAME_STEP_OUT,
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::AssetUnavailable,
        ErrorCode::AssetIntegrity,
    ],
    warnings: &[
        "BELOW_TERRAIN",
        "ORTHOMETRIC_AS_ELLIPSOIDAL",
        "INPUT_NORMALIZED",
        "UNIT_ASSUMED",
        "FRAME_MISMATCH",
    ],
    stability: Stability::Stable,
    when_to_use: "Use this whenever a GPS height has to be compared with anything on a map. A receiver gives height above the ellipsoid; a benchmark, a chart, a flood map and an airfield elevation all give height above sea level, and the two differ by tens of metres. Give it either and it returns both, with the geoid height it used.",
    limitations: "EGM96 is a global model of the geoid, good to about half a metre to a metre against levelled benchmarks and worse in mountains. For survey work the right reference is a national model -- GEOID18 in the United States -- and this is not that. The difference between the two heights is not small anywhere: it runs from about 85 m to -106 m across the world, so a GPS height read as a sea-level height is wrong by that much, not by a rounding. Bilinear interpolation is offered to reproduce an older number, not because it is better.",
    model: "h = H + N with N from the EGM96 15′ grid",
    accuracy: "As good as the geoid: about 0.5-1 m for EGM96. Exact arithmetic otherwise.",
    references: &[EGM96_REF, GEOGRAPHICLIB_GEOID],
    examples: &[
        Example {
            id: "primary",
            title: "A GPS height of 100 m at Timbuktu",
            input: r#"{"lat":16.776,"lon":-3.009,"height":"100 m"}"#,
            source: "h − N with N = 28.7079 m from GeographicLib GeoidEval (egm96-15, cubic): 71.292 m above sea level",
        },
        Example {
            id: "drone-agl",
            title: "A drone reporting 180 m above the ellipsoid over 40 m terrain",
            input: r#"{"lat":-33.8688,"lon":151.2093,"height":"180 m","terrain":"40 m"}"#,
            source: "h − N − terrain with N = 22.461 m from GeographicLib GeoidEval (egm96-15, cubic): 157.539 m above sea level, 117.539 m above the ground",
        },
    ],
    primary_example: "primary",
    assets: &[EGM96_ID],
    visualization: &[Layer {
        kind: "profile-chart",
        map: &[("value", "converted")],
    }],
    related: &[
        Related {
            id: "geodesy.geoid.geoid-height",
            reason: "parent",
        },
        Related {
            id: "geodesy.parse.coordinates",
            reason: "alternative",
        },
        Related {
            id: "units.length.convert",
            reason: "alternative",
        },
    ],
    sentence: "The converted height is {converted}. The geoid is {abs(geoid_height)} {if geoid_height < 0}below{else}above{/if} the ellipsoid here.{if agl != 0} That is {abs(agl)} {if agl < 0}below{else}above{/if} the ground.{/if}",
    limits: &[("batchRows", 10_000)],
    run: run_convert,
    ..ToolDef::BLANK
};

pub static ELLIPSOIDAL_TO_ORTHOMETRIC: ToolDef = ToolDef {
    version: "1.1.0",
    stability: gp_base::tool::Stability::Stable,
    id: "geodesy.height.ellipsoidal-to-orthometric",
    title: "Ellipsoidal height to orthometric height",
    summary: "Converts a GPS height above the WGS 84 ellipsoid (h) to a height above the EGM96 geoid, close to mean sea level (H = h − N).",
    aliases: &[
        "h to H",
        "HAE to MSL calculator",
        "GPS height to orthometric height",
    ],
    keywords: &[
        "ellipsoidal height",
        "HAE",
        "orthometric",
        "MSL",
        "geoid",
        "EGM96",
        "GNSS",
    ],
    inputs: &[
        HEIGHT_CONVERT.inputs[0],
        HEIGHT_CONVERT.inputs[1],
        Field::new(
            "height",
            "Ellipsoidal height h",
            "Height above the ellipsoid, on WGS 84 unless the frame says otherwise, like 312.6 m",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .required()
        .core(),
        HEIGHT_CONVERT.inputs[5],
        HEIGHT_CONVERT.inputs[6],
        FRAME_IN,
        EPOCH_IN,
    ],
    outputs: &[
        HEIGHT_CONVERT.outputs[2],
        HEIGHT_CONVERT.outputs[3],
        HEIGHT_CONVERT.outputs[1],
        FRAME_STEP_OUT,
    ],
    errors: HEIGHT_CONVERT.errors,
    warnings: &[
        "ORTHOMETRIC_AS_ELLIPSOIDAL",
        "FRAME_MISMATCH",
        "INPUT_NORMALIZED",
    ],
    model: "H = h − N, with N the EGM96 geoid height at the point, interpolated from the 15-minute grid",
    accuracy: HEIGHT_CONVERT.accuracy,
    when_to_use: "Use this when a GNSS receiver, a drone log, or a lidar point cloud gives height above the ellipsoid and you need it against sea level, a map, or an airfield elevation. Enter the latitude, longitude, and ellipsoidal height. The tool returns the orthometric height above the EGM96 geoid and the geoid height N it subtracted.",
    limitations: "A GNSS height is ellipsoidal unless the receiver says otherwise, and some apps already show MSL, so check before converting twice. EGM96 is a global geoid model good to about half a meter to a meter against leveled benchmarks, worse in mountains, and it is not a national vertical datum: in the United States, NAVD 88 heights need GEOID18, which this does not use. A NAD 83 (2011) ellipsoidal height, the kind OPUS returns, differs from a WGS 84 one by about 0.2 m to 1.7 m across the lower 48: name its frame and give the epoch and the tool moves it to WGS 84 by NGS's HTDP parameters, or name the frame alone and it warns you by how much the answer is off.",
    references: HEIGHT_CONVERT.references,
    examples: &[Example {
        id: "primary",
        title: "A drone over Lake Geneva",
        input: r#"{"lat":46.452,"lon":6.583,"height":"448.2 m"}"#,
        source: "EGM96 15-minute grid, interpolated as GeographicLib does",
    }],
    primary_example: "primary",
    assets: HEIGHT_CONVERT.assets,
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geodesy.height.convert",
            reason: "parent",
        },
        Related {
            id: "geodesy.height.orthometric-to-ellipsoidal",
            reason: "inverse",
        },
        Related {
            id: "geodesy.geoid.geoid-height",
            reason: "parent",
        },
    ],
    sentence: "The height above the geoid is {orthometric}. The geoid is {abs(geoid_height)} {if geoid_height < 0}below{else}above{/if} the ellipsoid here.",
    limits: HEIGHT_CONVERT.limits,
    run: run_ellipsoidal_to_orthometric,
    ..ToolDef::BLANK
};

pub static ORTHOMETRIC_TO_ELLIPSOIDAL: ToolDef = ToolDef {
    version: "1.1.0",
    stability: gp_base::tool::Stability::Stable,
    id: "geodesy.height.orthometric-to-ellipsoidal",
    title: "Orthometric height to ellipsoidal height",
    summary: "Converts a height above the EGM96 geoid, close to mean sea level (H), to a height above the WGS 84 ellipsoid (h = H + N).",
    aliases: &[
        "H to h",
        "MSL to HAE calculator",
        "orthometric to ellipsoidal height",
    ],
    keywords: &[
        "orthometric",
        "MSL",
        "ellipsoidal height",
        "HAE",
        "geoid",
        "EGM96",
        "waypoint",
    ],
    inputs: &[
        HEIGHT_CONVERT.inputs[0],
        HEIGHT_CONVERT.inputs[1],
        Field::new(
            "height",
            "Orthometric height H",
            "Height above mean sea level (EGM96), like 1,655 m",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .required()
        .core(),
        HEIGHT_CONVERT.inputs[5],
        HEIGHT_CONVERT.inputs[6],
        FRAME_IN,
        EPOCH_IN,
    ],
    outputs: &[
        HEIGHT_CONVERT.outputs[1],
        HEIGHT_CONVERT.outputs[3],
        HEIGHT_CONVERT.outputs[2],
        FRAME_STEP_OUT,
    ],
    errors: HEIGHT_CONVERT.errors,
    warnings: &[
        "ORTHOMETRIC_AS_ELLIPSOIDAL",
        "FRAME_MISMATCH",
        "INPUT_NORMALIZED",
    ],
    model: "h = H + N, with N the EGM96 geoid height at the point, interpolated from the 15-minute grid",
    accuracy: HEIGHT_CONVERT.accuracy,
    when_to_use: "Use this to give a GNSS system a height it understands: a waypoint altitude for a drone that flies in ellipsoidal height, a control point from a map elevation, or a terrain height for a ground-clearance check against GNSS. Enter the latitude, longitude, and height above sea level. The tool returns the ellipsoidal height and the geoid height N it added.",
    limitations: "A map or chart elevation is usually on a national vertical datum, not EGM96, so the result can be off by the difference between the two. EGM96 is a global geoid model good to about half a meter to a meter against leveled benchmarks, worse in mountains, and it is not a national vertical datum: in the United States, NAVD 88 heights need GEOID18, which this does not use. A NAD 83 (2011) ellipsoidal height, the kind OPUS returns, differs from a WGS 84 one by about 0.2 m to 1.7 m across the lower 48: name its frame and give the epoch and the tool moves it to WGS 84 by NGS's HTDP parameters, or name the frame alone and it warns you by how much the answer is off.",
    references: HEIGHT_CONVERT.references,
    examples: &[Example {
        id: "primary",
        title: "A summit elevation near Boulder, Colorado",
        input: r#"{"lat":40.015,"lon":-105.2705,"height":"1655 m"}"#,
        source: "EGM96 15-minute grid, interpolated as GeographicLib does",
    }],
    primary_example: "primary",
    assets: HEIGHT_CONVERT.assets,
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geodesy.height.convert",
            reason: "parent",
        },
        Related {
            id: "geodesy.height.ellipsoidal-to-orthometric",
            reason: "inverse",
        },
        Related {
            id: "geodesy.geoid.geoid-height",
            reason: "parent",
        },
    ],
    sentence: "The ellipsoidal height is {ellipsoidal}. The geoid is {abs(geoid_height)} {if geoid_height < 0}below{else}above{/if} the ellipsoid here.",
    limits: HEIGHT_CONVERT.limits,
    run: run_orthometric_to_ellipsoidal,
    ..ToolDef::BLANK
};

fn run_ellipsoidal_to_orthometric(ctx: &mut Ctx) -> Result<Json, ToolError> {
    convert(ctx, Some(false))
}

fn run_orthometric_to_ellipsoidal(ctx: &mut Ctx) -> Result<Json, ToolError> {
    convert(ctx, Some(true))
}

fn run_convert(ctx: &mut Ctx) -> Result<Json, ToolError> {
    convert(ctx, None)
}

/// Converts between ellipsoidal and orthometric heights. A focused operation
/// fixes the direction (`Some(true)` for orthometric input) and declares no
/// `from` or `terrain`; the general one reads them.
fn convert(ctx: &mut Ctx, orthometric_in: Option<bool>) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let h = ctx.req_quantity("height")?;
    let (n, _) = geoid_height(ctx, lat, lon)?;
    msl_note(ctx);
    let step = frame_step(ctx)?;
    if let Some(orthometric) = orthometric_in {
        let (ell, orth, shift) = if orthometric {
            let (e, s) = ellipsoidal_from(ctx, lat, lon, h.base() + n, &step);
            (e, h.base(), s)
        } else {
            let (wgs, s) = wgs84_from(ctx, lat, lon, h.base(), &step);
            (h.base(), wgs - n, s)
        };
        let unit = h.unit;
        let geoid = ("geoid_height", ctx.out("geoid_height", m(n)));
        let ell = ("ellipsoidal", ctx.emit("ellipsoidal", m(ell), unit));
        let orth = ("orthometric", ctx.emit("orthometric", m(orth), unit));
        let mut out = if orthometric {
            vec![ell, geoid, orth]
        } else {
            vec![orth, geoid, ell]
        };
        if let Some(s) = shift {
            out.push(("frame_step", ctx.out("frame_step", m(s))));
        }
        return Ok(Json::obj(out));
    }
    let from = ctx.choice("from")?;
    let terrain = ctx.quantity("terrain")?.map(|t| t.base());
    // A height above ground says nothing without the ground it is above.
    if from == Some("agl") && terrain.is_none() {
        return Err(ToolError::invalid(
            "/terrain",
            "A height above ground needs the terrain elevation at this point.",
        )
        .hint("Give the ground elevation above mean sea level, like 250 m."));
    }
    let (ell, orth, shift) = match from {
        Some("orthometric") => {
            let (e, s) = ellipsoidal_from(ctx, lat, lon, h.base() + n, &step);
            (e, h.base(), s)
        }
        Some("agl") => {
            let g = terrain.expect("checked");
            let (e, s) = ellipsoidal_from(ctx, lat, lon, h.base() + g + n, &step);
            (e, h.base() + g, s)
        }
        _ => {
            let (wgs, s) = wgs84_from(ctx, lat, lon, h.base(), &step);
            (h.base(), wgs - n, s)
        }
    };
    let unit = h.unit;
    let conv = match from {
        Some("orthometric") => ell,
        Some("agl") => orth,
        _ => orth,
    };
    let mut out = vec![
        ("converted", ctx.emit("converted", m(conv), unit)),
        ("ellipsoidal", ctx.emit("ellipsoidal", m(ell), unit)),
        ("orthometric", ctx.emit("orthometric", m(orth), unit)),
        ("geoid_height", ctx.out("geoid_height", m(n))),
    ];
    if let Some(s) = shift {
        out.push(("frame_step", ctx.out("frame_step", m(s))));
    }
    if let Some(g) = terrain {
        let agl = orth - g;
        out.push(("agl", ctx.emit("agl", m(agl), unit)));
        if agl < 0.0 {
            ctx.warnings.push(Warning::new(
                "BELOW_TERRAIN",
                "This height is below the terrain elevation given, so the point is underground.",
            ).at("/height"));
        }
    }
    Ok(Json::obj(out))
}
