//! OSTN15 and OSGM15 (add-international-grids, geodesy/national-grids,
//! "OSTN15 and OSGM15"): ETRS89 to the OSGB36 National Grid and an orthometric
//! height on the datum OSGM15 flags, and back, on Ordnance Survey's native
//! 1 km grid. The math is in gp_geo::ostn15.

use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::ostn15::{self, Grid};
use gp_geo::point;

pub const OSTN15_ID: &str = "ostn15";
pub const OSTN15_VERSION: &str = "2016";
const OSTN15_FILE: &str = "ostn15-osgm15.grid";

const NOTICE: &str =
    "© Copyright and database rights Ordnance Survey Limited 2016. All rights reserved.";
const ACCURACY: &str = "Ordnance Survey states OSTN15 at 0.1 m RMS against the triangulation it replaced, and OSGM15's standard error as 0.01 m in Great Britain, 0.02 m in Orkney and Shetland, and 0.03 m on the Isle of Man; this tool reproduces Ordnance Survey's own published test results to the millimetre.";

const OS_USER_GUIDE: Reference = Reference {
    title: "Transformations and OSGM15 User Guide",
    issuer: "Ordnance Survey",
    year: 2020,
    edition: "v1.3, November 2020, in the OSTN15-OSGM15 Developers Pack, read 2026-11-16",
    locator: "Chapter 3, OSTN15 and OSGM15 in Great Britain: the record-number rule, bilinear interpolation from the four corners of the kilometre square, the datum flag by quadrant, and the iterative inverse; Table 1, the height datum flags; annexe A, the Caister Water Tower worked example both ways; annexe B, the National Grid projection",
    url: "https://www.ordnancesurvey.co.uk/documents/resources/OSTN15-OSGM15-DevelopersPack.zip",
};

fn unit(q: QT, s: &str) -> &'static gp_base::units::Unit {
    units::by_symbol(q, s).expect("registered unit")
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
const fn length(name: &'static str, title: &'static str, help: &'static str) -> Field {
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

fn grid<'a>(bytes: &'a [u8]) -> Result<Grid<'a>, ToolError> {
    Grid::parse(bytes).map_err(|e| {
        ToolError::new(
            ErrorCode::AssetIntegrity,
            format!("The OSTN15 grid could not be read: {e}."),
        )
    })
}

fn outside(at: &str) -> ToolError {
    ToolError::new(
        ErrorCode::OutOfDomain,
        "The point is outside the area OSTN15 and OSGM15 cover: Great Britain, the Isle of Man, and their waters out to the transformation's edge.",
    )
    .at(at)
}

const SHIFT_OUTPUTS: [Field; 6] = [
    length(
        "east_shift",
        "Easting shift",
        "OSTN15's ETRS89 to OSGB36 easting shift here, interpolated",
    )
    .precision(Precision::Decimals(4)),
    length(
        "north_shift",
        "Northing shift",
        "OSTN15's ETRS89 to OSGB36 northing shift here, interpolated",
    )
    .precision(Precision::Decimals(4)),
    length(
        "geoid_height",
        "Geoid height",
        "OSGM15's height of the geoid above the ellipsoid here",
    )
    .precision(Precision::Decimals(4)),
    Field::new(
        "datum",
        "Height datum",
        "The local vertical datum the orthometric height is on",
        Kind::Text { max_len: 80 },
    ),
    Field::new(
        "datum_flag",
        "Datum flag",
        "OSGM15's code for that datum, 1 for Newlyn",
        Kind::Number {
            min: 0.0,
            max: 255.0,
        },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "notice",
        "Notice",
        "The copyright notice OSTN15's licence requires",
        Kind::Text { max_len: 100 },
    ),
];

pub static OSTN15_FORWARD: ToolDef = ToolDef {
    id: "geodesy.datum.ostn15",
    title: "GNSS position to British National Grid by OSTN15",
    summary: "Turns an ETRS89 latitude, longitude, and ellipsoidal height from a GNSS receiver into an OSGB36 National Grid easting and northing by OSTN15, and an orthometric height above the local datum by OSGM15, exactly as Ordnance Survey defines them.",
    aliases: &[
        "OSTN15 transformation",
        "ETRS89 to OSGB36",
        "GPS to National Grid",
        "WGS84 to British National Grid accurate",
        "GNSS height to ODN",
    ],
    keywords: &[
        "OSTN15",
        "OSGM15",
        "ETRS89",
        "OSGB36",
        "National Grid",
        "ODN",
        "Newlyn",
        "Britain",
    ],
    inputs: &[
        Field::new(
            "lat",
            "Latitude",
            "ETRS89, north positive, like 52.658007833",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core()
        .precision(Precision::Decimals(9))
        .angle_range("[-90,90]"),
        Field::new(
            "lon",
            "Longitude",
            "ETRS89, east positive, like 1.716073972",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core()
        .precision(Precision::Decimals(9))
        .angle_range("[-180,180)"),
        length(
            "height",
            "Ellipsoidal height",
            "The GNSS height above the ellipsoid, like 108.05 m; the easting and northing do not depend on it",
        )
        .required()
        .core(),
    ],
    outputs: &[
        length(
            "easting",
            "Easting",
            "On the OSGB36 National Grid",
        )
        .precision(Precision::Decimals(3)),
        length(
            "northing",
            "Northing",
            "On the OSGB36 National Grid",
        )
        .precision(Precision::Decimals(3)),
        length(
            "orthometric_height",
            "Orthometric height",
            "Above the local datum the flag names, Newlyn on the mainland",
        )
        .precision(Precision::Decimals(3)),
        SHIFT_OUTPUTS[0],
        SHIFT_OUTPUTS[1],
        SHIFT_OUTPUTS[2],
        SHIFT_OUTPUTS[3],
        SHIFT_OUTPUTS[4],
        SHIFT_OUTPUTS[5],
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::AssetUnavailable,
        ErrorCode::AssetIntegrity,
    ],
    warnings: &["REALIZATION_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "OSTN15 and OSGM15 on Ordnance Survey's native 1 km grid: the ETRS89 latitude and longitude projected on the National Grid transverse Mercator with GRS 80, the easting, northing and geoid shifts bilinearly interpolated from the four corners of the kilometre square and added (the geoid height subtracted from the ellipsoidal height), and the height datum flag taken by quadrant where the corners disagree",
    accuracy: ACCURACY,
    when_to_use: "Use this to put a GNSS survey onto Ordnance Survey mapping and levels: it is the transformation that defines the National Grid, so the easting and northing it gives are the right ones by definition, and the orthometric height is on the local datum, Newlyn on the mainland or the island datums offshore. For a quick map position the seven-parameter path in geodesy.grid-ref.bng-forward is about 2 m; this is the one to use for survey work.",
    limitations: "The input is taken to be ETRS89. A WGS 84 or ITRF position from a consumer receiver is about 0.8 m away in Britain today, which geodesy.datum.etrf removes. Outside Great Britain, the Isle of Man, and their waters there is no answer. Northern Ireland uses the Irish grids and OSGM15's Belfast and Malin models, which are separate.",
    references: &[OS_USER_GUIDE],
    examples: &[Example {
        id: "primary",
        title: "Caister Water Tower",
        source: "Ordnance Survey, Transformations and OSGM15 User Guide v1.3, annexe A: 52°39'28.8282\"N 1°42'57.8663\"E, 108.05 m on ETRS89 gives E 651409.804 N 313177.450 and 63.822 m above Newlyn",
        input: r#"{"lat":"52°39'28.8282\"N","lon":"1°42'57.8663\"E","height":"108.05 m"}"#,
    }],
    primary_example: "primary",
    assets: &[OSTN15_ID],
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geodesy.datum.ostn15-inverse",
            reason: "inverse",
        },
        Related {
            id: "geodesy.grid-ref.bng-forward",
            reason: "next",
        },
        Related {
            id: "geodesy.datum.etrf",
            reason: "parent",
        },
    ],
    sentence: "That point is E {easting}, N {northing} on the National Grid, {orthometric_height} above {datum}.",
    limits: &[("batchRows", 10_000)],
    run: run_forward,
    ..ToolDef::BLANK
};

const FRAME: &str = "The latitude, longitude, and height are taken to be ETRS89, the frame OSTN15 is defined from. A WGS 84 or ITRF position differs from it by about 0.8 m in Britain today.";

fn run_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let h = ctx.req_quantity("height")?.to(unit(QT::Length, "m"));
    if !h.is_finite() {
        return Err(ToolError::invalid(
            "/height",
            "The height is metres above the ellipsoid.",
        ));
    }
    let bytes = ctx.asset(OSTN15_ID, OSTN15_VERSION, OSTN15_FILE)?;
    let grid = grid(&bytes)?;
    let p = ostn15::etrs89_grid().forward(lat, lon);
    let (e, n, oh, s) = grid.forward(p.e, p.n, h).ok_or_else(|| outside("/lat"))?;
    if s.flag == 16 {
        return Err(outside("/lat"));
    }
    ctx.warnings
        .push(Warning::new("REALIZATION_ASSUMED", FRAME));
    ctx.accuracy = Some(ACCURACY.into());
    ctx.context.push((
        "records",
        Json::Arr(s.records.iter().map(|&r| Json::Num(r as f64)).collect()),
    ));
    if ctx.explaining() {
        ctx.step(
            "ETRS89 on the grid's projection",
            "the National Grid transverse Mercator, on GRS 80",
            format!("{lat:.9}°, {lon:.9}°"),
            format!("x {:.3} m, y {:.3} m", p.e, p.n),
        );
        ctx.step(
            "The kilometre square",
            "record = east index + north index × 701 + 1, for each of its four corners",
            format!("x {:.3} m, y {:.3} m", p.e, p.n),
            format!("records {} to {}", s.records[0], s.records[2]),
        );
        ctx.step(
            "The shifts here",
            "bilinear: (1 − t)(1 − u)s0 + t(1 − u)s1 + tu·s2 + (1 − t)u·s3",
            format!(
                "t {:.7}, u {:.7}",
                (p.e % 1000.0) / 1000.0,
                (p.n % 1000.0) / 1000.0
            ),
            format!("se {:.4} m, sn {:.4} m, sg {:.4} m", s.se, s.sn, s.sg),
        );
        ctx.step(
            "On the National Grid",
            "x + se, y + sn, and the height less the geoid",
            format!("x {:.3} m, y {:.3} m, {h:.3} m", p.e, p.n),
            format!(
                "{} m",
                gp_base::display::number(e, Precision::Decimals(3), ctx.options.format)
            ),
        );
    }
    Ok(Json::obj([
        ("easting", ctx.out("easting", m(e))),
        ("northing", ctx.out("northing", m(n))),
        ("orthometric_height", ctx.out("orthometric_height", m(oh))),
        ("east_shift", ctx.out("east_shift", m(s.se))),
        ("north_shift", ctx.out("north_shift", m(s.sn))),
        ("geoid_height", ctx.out("geoid_height", m(s.sg))),
        ("datum", Json::str(ostn15::datum_name(s.flag))),
        ("datum_flag", Json::Num(f64::from(s.flag))),
        ("notice", Json::str(NOTICE)),
    ]))
}

pub static OSTN15_INVERSE: ToolDef = ToolDef {
    id: "geodesy.datum.ostn15-inverse",
    title: "British National Grid to GNSS position by OSTN15",
    summary: "Turns an OSGB36 National Grid easting and northing and an orthometric height into the ETRS89 latitude, longitude, and ellipsoidal height a GNSS receiver works in, by Ordnance Survey's iterative inverse of OSTN15 and OSGM15.",
    aliases: &[
        "OSGB36 to ETRS89",
        "National Grid to GPS",
        "OSTN15 inverse",
        "British National Grid to WGS84 accurate",
        "ODN height to ellipsoidal",
    ],
    keywords: &[
        "OSTN15",
        "OSGM15",
        "ETRS89",
        "OSGB36",
        "National Grid",
        "ODN",
        "inverse",
        "Britain",
    ],
    inputs: &[
        length(
            "easting",
            "Easting",
            "On the OSGB36 National Grid, like 651409.804 m",
        )
        .required()
        .core(),
        length(
            "northing",
            "Northing",
            "On the OSGB36 National Grid, like 313177.450 m",
        )
        .required()
        .core(),
        length(
            "height",
            "Orthometric height",
            "Above the local datum, like 63.822 m; the latitude and longitude do not depend on it",
        )
        .required()
        .core(),
    ],
    outputs: &[
        Field::new(
            "lat",
            "Latitude",
            "On ETRS89, north positive",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(9))
        .angle_range("[-90,90]"),
        Field::new(
            "lon",
            "Longitude",
            "On ETRS89, east positive",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(9))
        .angle_range("[-180,180)"),
        length(
            "ellipsoidal_height",
            "Ellipsoidal height",
            "Above the GRS 80 ellipsoid, as a GNSS receiver reports it",
        )
        .precision(Precision::Decimals(3)),
        length(
            "etrs89_easting",
            "ETRS89 easting",
            "The point on the National Grid projection before the shifts",
        )
        .precision(Precision::Decimals(4)),
        length(
            "etrs89_northing",
            "ETRS89 northing",
            "The point on the National Grid projection before the shifts",
        )
        .precision(Precision::Decimals(4)),
        Field::new(
            "iterations",
            "Iterations",
            "How many times the shifts were recomputed, as OS's procedure does",
            Kind::Number {
                min: 1.0,
                max: 50.0,
            },
        )
        .precision(Precision::Decimals(0)),
        SHIFT_OUTPUTS[0],
        SHIFT_OUTPUTS[1],
        SHIFT_OUTPUTS[2],
        SHIFT_OUTPUTS[3],
        SHIFT_OUTPUTS[4],
        SHIFT_OUTPUTS[5],
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::AssetUnavailable,
        ErrorCode::AssetIntegrity,
    ],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "Ordnance Survey's iterative inverse of OSTN15 and OSGM15: the shifts at the OSGB36 point subtracted for a first ETRS89 estimate, then the shifts at each estimate, until they change by no more than 0.1 mm; the ETRS89 easting and northing then unprojected from the National Grid transverse Mercator on GRS 80",
    accuracy: ACCURACY,
    when_to_use: "Use this to set out a National Grid design or a levelled height with a GNSS receiver: it gives the ETRS89 position and ellipsoidal height that OSTN15 and OSGM15 map onto the grid coordinates and the orthometric height you have.",
    limitations: "The answer is ETRS89; a receiver working in WGS 84 or ITRF needs geodesy.datum.etrf for the 0.8 m between them. The height you give is taken to be on the datum OSGM15 flags at the point, Newlyn on the mainland. Outside Great Britain, the Isle of Man, and their waters there is no answer.",
    references: &[OS_USER_GUIDE],
    examples: &[Example {
        id: "primary",
        title: "Caister Water Tower",
        source: "Ordnance Survey, Transformations and OSGM15 User Guide v1.3, annexe A: E 651409.804 N 313177.450, 63.822 m converges in three iterations to ETRS89 E 651307.0032 N 313255.6860, 108.0504 m, which is 52.658007833°N 1.716073972°E",
        input: r#"{"easting":"651409.804 m","northing":"313177.450 m","height":"63.822 m"}"#,
    }],
    primary_example: "primary",
    assets: &[OSTN15_ID],
    visualization: &[Layer {
        kind: "point",
        map: &[("lat", "lat"), ("lon", "lon")],
    }],
    related: &[
        Related {
            id: "geodesy.datum.ostn15",
            reason: "inverse",
        },
        Related {
            id: "geodesy.grid-ref.bng-inverse",
            reason: "alternative",
        },
    ],
    sentence: "That is {lat}, {lon} on ETRS89, {ellipsoidal_height} above the ellipsoid.",
    limits: &[("batchRows", 10_000)],
    run: run_inverse,
    ..ToolDef::BLANK
};

fn run_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let mm = unit(QT::Length, "m");
    let e = ctx.req_quantity("easting")?.to(mm);
    let n = ctx.req_quantity("northing")?.to(mm);
    let oh = ctx.req_quantity("height")?.to(mm);
    if !e.is_finite() || !n.is_finite() || !oh.is_finite() {
        return Err(ToolError::invalid(
            "/easting",
            "The easting, northing, and height are metres.",
        ));
    }
    let bytes = ctx.asset(OSTN15_ID, OSTN15_VERSION, OSTN15_FILE)?;
    let grid = grid(&bytes)?;
    let steps = grid.inverse(e, n, oh).ok_or_else(|| outside("/easting"))?;
    let (x, y, h, s) = steps[steps.len() - 1].clone();
    if s.flag == 16 {
        return Err(outside("/easting"));
    }
    let (lat, lon) = ostn15::etrs89_grid().inverse(x, y);
    ctx.accuracy = Some(ACCURACY.into());
    if ctx.explaining() {
        for (i, (sx, sy, sz, ss)) in steps.iter().enumerate() {
            ctx.step(
                if i == 0 {
                    "First estimate"
                } else {
                    "Next estimate"
                },
                "the OSGB36 point less the shifts at the latest estimate",
                format!("se {:.4} m, sn {:.4} m, sg {:.4} m", ss.se, ss.sn, ss.sg),
                format!("E {sx:.4} m, N {sy:.4} m, {sz:.4} m"),
            );
        }
        ctx.step(
            "Off the projection",
            "the inverse National Grid transverse Mercator on GRS 80",
            format!("E {x:.4} m, N {y:.4} m"),
            format!(
                "{}°",
                gp_base::display::number(lat, Precision::Decimals(9), ctx.options.format)
            ),
        );
    }
    Ok(Json::obj([
        ("lat", ctx.out("lat", deg(lat))),
        ("lon", ctx.out("lon", deg(lon))),
        ("ellipsoidal_height", ctx.out("ellipsoidal_height", m(h))),
        ("etrs89_easting", ctx.out("etrs89_easting", m(x))),
        ("etrs89_northing", ctx.out("etrs89_northing", m(y))),
        ("iterations", Json::Num(steps.len() as f64)),
        ("east_shift", ctx.out("east_shift", m(s.se))),
        ("north_shift", ctx.out("north_shift", m(s.sn))),
        ("geoid_height", ctx.out("geoid_height", m(s.sg))),
        ("datum", Json::str(ostn15::datum_name(s.flag))),
        ("datum_flag", Json::Num(f64::from(s.flag))),
        ("notice", Json::str(NOTICE)),
    ]))
}
