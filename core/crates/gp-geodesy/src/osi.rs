//! The OSi/LPS polynomial and OSGM15 in Ireland (add-international-grids,
//! geodesy/national-grids, "Irish Grid and ITM"): Irish Grid (TM75) eastings,
//! northings and heights to ETRS89 and back, with heights on the Malin Head or
//! Belfast datum. The math is in gp_geo::osi.

use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use gp_geo::osi::{self, Geoid};
use gp_geo::ostn15::{irish_grid, itm_grid};
use gp_geo::point;

pub const OSGM15_IE_ID: &str = "osgm15-ireland";
pub const OSGM15_IE_VERSION: &str = "2016";

const NOTICE: &str = "© Copyright and database rights Ordnance Survey Limited 2016, © Crown copyright and database rights Land & Property Services 2016 and/or © Ordnance Survey Ireland, 2016. All rights reserved.";
const ACCURACY: &str = "Ordnance Survey, OSi and LPS state that ETRS89 coordinates from the polynomial agree with Irish Grid coordinates from traditional survey control to 0.4 m (95%), and OSGM15's standard error as 0.02 m in the Republic of Ireland and 0.01 m in Northern Ireland; this tool reproduces their published test results to the printed millionth of a second horizontally and within a millimetre in height.";

const OS_USER_GUIDE: Reference = Reference {
    title: "Transformations and OSGM15 User Guide",
    issuer: "Ordnance Survey, with Ordnance Survey Ireland and Land & Property Services",
    year: 2020,
    edition: "v1.3, November 2020, in the OSTN15-OSGM15 Developers Pack, read 2026-11-16",
    locator: "Chapter 3, the OSi/LPS polynomial transformation and its table of coefficients (φm 53.5, λm −7.7, K0 0.1), the iterative reverse, and OSGM15 in Ireland and Northern Ireland; annexe B, the Irish Grid and ITM projection constants",
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
fn arcsec(v: f64) -> Q {
    Q {
        value: v,
        unit: unit(QT::Angle, "arcsec"),
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

const DATUM_IN: Field = Field::new(
    "datum",
    "Height datum",
    "Belfast in Northern Ireland, Malin Head in the Republic of Ireland, like Belfast",
    Kind::Choice(&["Belfast", "Malin Head"]),
)
.required()
.core();

/// The datum's grid file and its name in answers.
fn datum(ctx: &Ctx) -> Result<(&'static str, &'static str), ToolError> {
    Ok(match ctx.choice("datum")? {
        Some("Malin Head") => ("malin.grid", "Malin Head (Republic of Ireland)"),
        _ => ("belfast.grid", "Belfast (Northern Ireland)"),
    })
}

fn outside(at: &str) -> ToolError {
    ToolError::new(
        ErrorCode::OutOfDomain,
        "The point is outside the area OSGM15's Irish grids cover, 51°N to 56°N and 11°30'W to 5°W, where the polynomial and the geoid have no values.",
    )
    .at(at)
}

fn geoid_height(ctx: &mut Ctx, file: &str, lat: f64, lon: f64, at: &str) -> Result<f64, ToolError> {
    let bytes = ctx.asset(OSGM15_IE_ID, OSGM15_IE_VERSION, file)?;
    let g = Geoid::parse(&bytes).map_err(|e| {
        ToolError::new(
            ErrorCode::AssetIntegrity,
            format!("The OSGM15 Ireland grid could not be read: {e}."),
        )
    })?;
    g.height(lat, lon).ok_or_else(|| outside(at))
}

const SHARED_OUT: [Field; 5] = [
    Field::new(
        "lat_shift",
        "Latitude shift",
        "The polynomial's shift in latitude, TM75 to ETRS89",
        Kind::Quantity {
            q: QT::Angle,
            unit: "arcsec",
        },
    )
    .precision(Precision::Decimals(4)),
    Field::new(
        "lon_shift",
        "Longitude shift",
        "The polynomial's shift in longitude, TM75 to ETRS89",
        Kind::Quantity {
            q: QT::Angle,
            unit: "arcsec",
        },
    )
    .precision(Precision::Decimals(4)),
    length(
        "geoid_height",
        "Geoid height",
        "OSGM15's height of the geoid above the ellipsoid here, on the datum named",
    )
    .precision(Precision::Decimals(4)),
    Field::new(
        "height_datum",
        "Height datum",
        "The datum the orthometric height is on",
        Kind::Text { max_len: 60 },
    ),
    Field::new(
        "notice",
        "Notice",
        "The copyright notice the transformations' licence requires",
        Kind::Text { max_len: 200 },
    ),
];

pub static OSI_POLYNOMIAL: ToolDef = ToolDef {
    id: "geodesy.datum.osi-polynomial",
    title: "Irish Grid to ETRS89 by the OSi and LPS polynomial",
    summary: "Turns an Irish Grid easting, northing, and height on the Malin Head or Belfast datum into an ETRS89 latitude, longitude, and ellipsoidal height, and Irish Transverse Mercator, by the official polynomial and OSGM15.",
    aliases: &[
        "Irish Grid to ITM",
        "Irish Grid to ETRS89",
        "TM75 to ETRS89",
        "Irish grid to GPS",
        "OSi polynomial transformation",
    ],
    keywords: &[
        "Irish Grid",
        "TM75",
        "ETRS89",
        "ITM",
        "OSGM15",
        "Malin Head",
        "Belfast",
        "Ireland",
    ],
    inputs: &[
        length("easting", "Easting", "On the Irish Grid, like 223657.156 m")
            .required()
            .core(),
        length(
            "northing",
            "Northing",
            "On the Irish Grid, like 340618.902 m",
        )
        .required()
        .core(),
        length(
            "height",
            "Orthometric height",
            "Above the datum named, like 37.958 m; the latitude and longitude do not depend on it",
        )
        .required()
        .core(),
        DATUM_IN,
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
            "On ETRS89, east positive, so Ireland is negative",
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
        .precision(Precision::Decimals(4)),
        length(
            "itm_easting",
            "ITM easting",
            "The same point on Irish Transverse Mercator",
        )
        .precision(Precision::Decimals(3)),
        length(
            "itm_northing",
            "ITM northing",
            "The same point on Irish Transverse Mercator",
        )
        .precision(Precision::Decimals(3)),
        SHARED_OUT[0],
        SHARED_OUT[1],
        SHARED_OUT[2],
        SHARED_OUT[3],
        SHARED_OUT[4],
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::AssetUnavailable,
        ErrorCode::AssetIntegrity,
    ],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "The Irish Grid easting and northing off the Irish Grid transverse Mercator on Airy Modified 1849 by Ordnance Survey's annexe C formulas, the OSi/LPS third-order polynomial's latitude and longitude shifts added, and the OSGM15 geoid height on the datum named added to the orthometric height; the ETRS89 point then on Irish Transverse Mercator by annexe B",
    accuracy: ACCURACY,
    when_to_use: "Use this to bring Irish Grid survey data, older Ordnance Survey Ireland and OSNI mapping, or a levelled height into ETRS89 and Irish Transverse Mercator, the frames GNSS and current Irish mapping work in. Name the height datum: Belfast for Northern Ireland, Malin Head for the Republic.",
    limitations: "The polynomial is good to 0.4 m (95%) against the old triangulation, because it fits a smooth surface to a network that was never perfectly regular. The tool does not know which side of the border a point is on, so the height datum is asked for. Outside OSGM15's Irish grids, 51°N to 56°N and 11°30'W to 5°W, there is no answer. ETRS89 differs from WGS 84 and ITRF by about 0.8 m today.",
    references: &[OS_USER_GUIDE],
    examples: &[Example {
        id: "primary",
        title: "Ordnance Survey's Northern Ireland test point",
        source: "Ordnance Survey, OSGM15_NI_TestOutput_IGtoETRS89.txt, NI_IG_1: IG 223657.156, 340618.902, 37.958 m on Belfast gives 54°18'49.673618\"N 7°38'14.133209\"W, 95.4675 m",
        input: r#"{"easting":"223657.156 m","northing":"340618.902 m","height":"37.958 m","datum":"Belfast"}"#,
    }],
    primary_example: "primary",
    assets: &[OSGM15_IE_ID],
    visualization: &[Layer {
        kind: "point",
        map: &[("lat", "lat"), ("lon", "lon")],
    }],
    related: &[
        Related {
            id: "geodesy.datum.osi-polynomial-inverse",
            reason: "inverse",
        },
        Related {
            id: "geodesy.grid-ref.irish-inverse",
            reason: "alternative",
        },
        Related {
            id: "geodesy.grid-ref.itm-inverse",
            reason: "next",
        },
    ],
    sentence: "That is {lat}, {lon} on ETRS89, {ellipsoidal_height} above the ellipsoid.",
    limits: &[("batchRows", 10_000)],
    run: run_forward,
    ..ToolDef::BLANK
};

fn run_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let mm = unit(QT::Length, "m");
    let e = ctx.req_quantity("easting")?.to(mm);
    let n = ctx.req_quantity("northing")?.to(mm);
    let h = ctx.req_quantity("height")?.to(mm);
    if !e.is_finite() || !n.is_finite() || !h.is_finite() {
        return Err(ToolError::invalid(
            "/easting",
            "The easting, northing, and height are metres.",
        ));
    }
    let (file, datum_name) = datum(ctx)?;
    let (tlat, tlon) = irish_grid().inverse(e, n);
    let (dp, dl) = osi::shifts(tlat, tlon);
    let (lat, lon) = osi::to_etrs89(tlat, tlon);
    let gh = geoid_height(ctx, file, lat, lon, "/easting")?;
    let itm = itm_grid().forward(lat, lon);
    ctx.accuracy = Some(ACCURACY.into());
    if ctx.explaining() {
        ctx.step(
            "Off the Irish Grid",
            "the inverse transverse Mercator on Airy Modified 1849, OS annexe C",
            format!("E {e:.3} m, N {n:.3} m"),
            format!("{tlat:.9}°, {tlon:.9}° on TM75"),
        );
        ctx.step(
            "The polynomial",
            "third order in U = 0.1(φ − 53.5) and V = 0.1(λ + 7.7), in seconds",
            format!("{tlat:.6}°, {tlon:.6}°"),
            format!("Δφ {dp:.4}\", Δλ {dl:.4}\""),
        );
        ctx.step(
            "The height",
            "the orthometric height plus OSGM15's geoid height on the datum",
            format!("{h:.4} m on {datum_name}"),
            format!("{:.4} m", h + gh),
        );
        ctx.step(
            "On ETRS89",
            "TM75 plus the shifts",
            format!("{tlat:.9}°, {tlon:.9}°"),
            format!(
                "{}°",
                gp_base::display::number(lat, Precision::Decimals(9), ctx.options.format)
            ),
        );
    }
    Ok(Json::obj([
        ("lat", ctx.out("lat", deg(lat))),
        ("lon", ctx.out("lon", deg(lon))),
        (
            "ellipsoidal_height",
            ctx.out("ellipsoidal_height", m(h + gh)),
        ),
        ("itm_easting", ctx.out("itm_easting", m(itm.e))),
        ("itm_northing", ctx.out("itm_northing", m(itm.n))),
        ("lat_shift", ctx.out("lat_shift", arcsec(dp))),
        ("lon_shift", ctx.out("lon_shift", arcsec(dl))),
        ("geoid_height", ctx.out("geoid_height", m(gh))),
        ("height_datum", Json::str(datum_name)),
        ("notice", Json::str(NOTICE)),
    ]))
}

pub static OSI_POLYNOMIAL_INVERSE: ToolDef = ToolDef {
    id: "geodesy.datum.osi-polynomial-inverse",
    title: "ETRS89 to Irish Grid by the OSi and LPS polynomial",
    summary: "Turns an ETRS89 latitude, longitude, and ellipsoidal height from a GNSS receiver into an Irish Grid easting and northing and a height above Malin Head or Belfast, by iterating the official polynomial and OSGM15.",
    aliases: &[
        "ETRS89 to Irish Grid",
        "GPS to Irish Grid",
        "ETRS89 to TM75",
        "ITM to Irish Grid",
        "OSi polynomial inverse",
    ],
    keywords: &[
        "Irish Grid",
        "TM75",
        "ETRS89",
        "OSGM15",
        "Malin Head",
        "Belfast",
        "Ireland",
        "inverse",
    ],
    inputs: &[
        Field::new(
            "lat",
            "Latitude",
            "ETRS89, north positive, like 55.182667",
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
            "ETRS89, east positive, so Ireland is negative, like -6.385028",
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
            "The GNSS height above the ellipsoid, like 124.5604 m; the easting and northing do not depend on it",
        )
        .required()
        .core(),
        DATUM_IN,
    ],
    outputs: &[
        length("easting", "Easting", "On the Irish Grid").precision(Precision::Decimals(3)),
        length("northing", "Northing", "On the Irish Grid").precision(Precision::Decimals(3)),
        length(
            "orthometric_height",
            "Orthometric height",
            "Above the datum named",
        )
        .precision(Precision::Decimals(4)),
        Field::new(
            "iterations",
            "Iterations",
            "How many times the polynomial was evaluated to invert it",
            Kind::Number {
                min: 1.0,
                max: 50.0,
            },
        )
        .precision(Precision::Decimals(0)),
        SHARED_OUT[0],
        SHARED_OUT[1],
        SHARED_OUT[2],
        SHARED_OUT[3],
        SHARED_OUT[4],
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::AssetUnavailable,
        ErrorCode::AssetIntegrity,
    ],
    warnings: &["REALIZATION_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "The OSi/LPS polynomial inverted by fixed-point iteration, TM75 = ETRS89 less the shifts at the latest TM75 estimate, until the estimate moves by under 1e-11 degrees; the TM75 point onto the Irish Grid transverse Mercator by Ordnance Survey's annexe B, and the OSGM15 geoid height on the datum named subtracted from the ellipsoidal height",
    accuracy: ACCURACY,
    when_to_use: "Use this to put GNSS survey data into Irish Grid coordinates and levelled heights, to match older Ordnance Survey Ireland and OSNI mapping, plans, and records. Name the height datum: Belfast for Northern Ireland, Malin Head for the Republic.",
    limitations: "The polynomial is good to 0.4 m (95%) against the old triangulation. The input is taken to be ETRS89; WGS 84 and ITRF differ from it by about 0.8 m today. The tool does not know which side of the border a point is on, so the height datum is asked for. Outside OSGM15's Irish grids there is no answer.",
    references: &[OS_USER_GUIDE],
    examples: &[Example {
        id: "primary",
        title: "Ordnance Survey's Northern Ireland test point NI_ETRS89_1",
        source: "Ordnance Survey, OSGM15_NI_TestOutput_ETRS89toIG.txt, NI_ETRS89_1: 55°10'57.600354\"N 6°23'6.102018\"W, 124.5604 m gives IG 302925.602, 438473.935, 68.4590 m on Belfast",
        input: r#"{"lat":"55°10'57.600354\"N","lon":"6°23'6.102018\"W","height":"124.5604 m","datum":"Belfast"}"#,
    }],
    primary_example: "primary",
    assets: &[OSGM15_IE_ID],
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geodesy.datum.osi-polynomial",
            reason: "inverse",
        },
        Related {
            id: "geodesy.grid-ref.irish-forward",
            reason: "next",
        },
    ],
    sentence: "That point is E {easting}, N {northing} on the Irish Grid, {orthometric_height} above {height_datum}.",
    limits: &[("batchRows", 10_000)],
    run: run_inverse,
    ..ToolDef::BLANK
};

fn run_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let h = ctx.req_quantity("height")?.to(unit(QT::Length, "m"));
    if !h.is_finite() {
        return Err(ToolError::invalid(
            "/height",
            "The height is metres above the ellipsoid.",
        ));
    }
    let (file, datum_name) = datum(ctx)?;
    let gh = geoid_height(ctx, file, lat, lon, "/lat")?;
    let (tlat, tlon, k) = osi::to_tm75(lat, lon);
    let (dp, dl) = osi::shifts(tlat, tlon);
    let p = irish_grid().forward(tlat, tlon);
    ctx.accuracy = Some(ACCURACY.into());
    ctx.warnings.push(Warning::new(
        "REALIZATION_ASSUMED",
        "The latitude, longitude, and height are taken to be ETRS89, the frame the polynomial is defined from.",
    ));
    if ctx.explaining() {
        ctx.step(
            "Off the polynomial",
            "TM75 = ETRS89 less the shifts at the latest estimate, repeated until it settles",
            format!("{lat:.9}°, {lon:.9}°"),
            format!("{tlat:.9}°, {tlon:.9}° after {k} iterations"),
        );
        ctx.step(
            "The height",
            "the ellipsoidal height less OSGM15's geoid height on the datum",
            format!("{h:.4} m"),
            format!("{:.4} m on {datum_name}", h - gh),
        );
        ctx.step(
            "On the Irish Grid",
            "transverse Mercator on Airy Modified 1849, OS annexe B",
            format!("{tlat:.9}°, {tlon:.9}°"),
            format!(
                "{} m",
                gp_base::display::number(p.e, Precision::Decimals(3), ctx.options.format)
            ),
        );
    }
    Ok(Json::obj([
        ("easting", ctx.out("easting", m(p.e))),
        ("northing", ctx.out("northing", m(p.n))),
        (
            "orthometric_height",
            ctx.out("orthometric_height", m(h - gh)),
        ),
        ("iterations", Json::Num(k as f64)),
        ("lat_shift", ctx.out("lat_shift", arcsec(dp))),
        ("lon_shift", ctx.out("lon_shift", arcsec(dl))),
        ("geoid_height", ctx.out("geoid_height", m(gh))),
        ("height_datum", Json::str(datum_name)),
        ("notice", Json::str(NOTICE)),
    ]))
}
