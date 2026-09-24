//! Projection methods with user-set parameters (geodesy/projections spec,
//! "General projection methods with custom parameters"): Web Mercator,
//! Lambert Conic Conformal (1SP and 2SP), Albers Equal Area, Polar
//! Stereographic (variants A and B), and Equidistant Cylindrical, forward and
//! inverse, with IOGP Guidance Note 7-2's parameter names. The math lives in
//! `gp_geo::proj`.

use gp_base::ErrorCode;
use gp_base::angle::check_lat;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{
    Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, Stability, ToolDef,
};
use gp_base::units::{self, Quantity as QT, Unit};
use gp_geo::ellipsoid::{self, Ellipsoid};
use gp_geo::point;
use gp_geo::proj::{
    self, Albers, Azimuthal, AzimuthalProj, EquidistantCylindrical, Grid, Lcc, PolarStereo,
    WebMercator,
};

const G7_2: Reference = Reference {
    title: "Coordinate Conversions and Transformations including Formulas, IOGP Publication 373-7-2 (Guidance Note 7-2)",
    issuer: "International Association of Oil & Gas Producers (IOGP)",
    year: 2019,
    edition: "Revised September 2019",
    locator: "Sections 3.2.1 (Lambert Conic Conformal), 3.2.2 (Polar Stereographic), 3.2.4 (Albers Equal Area), 3.3.3 (Equidistant Cylindrical), and 3.4.5 (Popular Visualisation Pseudo Mercator)",
    url: "https://www.iogp.org/bookstore/product/coordinate-conversions-and-transformation-including-formulas/",
};
const SNYDER: Reference = Reference {
    title: "Map Projections: A Working Manual, U.S. Geological Survey Professional Paper 1395",
    issuer: "Snyder, J. P., U.S. Geological Survey",
    year: 1987,
    edition: "Professional Paper 1395",
    locator: "Chapter 14 (Albers Equal-Area Conic), with the ellipsoidal numerical example on p. 292",
    url: "https://doi.org/10.3133/pp1395",
};
const NGA_WM: Reference = Reference {
    title: "Implementation Practice Web Mercator Map Projection, NGA.SIG.0011",
    issuer: "National Geospatial-Intelligence Agency",
    year: 2014,
    edition: "NGA.SIG.0011_1.0.0_WEBMERC",
    locator: "Sections 2 and 3 (the pseudo-Mercator on the WGS 84 ellipsoid and its limits)",
    url: "https://earth-info.nga.mil/php/download.php?file=wgs-webmerc",
};

fn unit(q: QT, s: &str) -> &'static Unit {
    units::by_symbol(q, s).expect("registered unit")
}

fn deg(v: f64) -> Q {
    Q {
        value: v,
        unit: unit(QT::Angle, "deg"),
    }
}

fn meters(v: f64) -> Q {
    Q {
        value: v,
        unit: unit(QT::Length, "m"),
    }
}

const LAT: Field = point::lat_field("lat", "Latitude");
const LON: Field = point::lon_field("lon", "Longitude");
const E: [Field; 3] = ellipsoid::FIELDS;

const fn angle(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    range: &'static str,
) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .angle_range(range)
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

const LAT0: Field = angle(
    "latitude_of_origin",
    "Latitude of origin",
    "The latitude the northing counts from, like 27°50'; 0 if not given",
    "[-90,90]",
);
const LON0: Field = angle(
    "longitude_of_origin",
    "Longitude of origin",
    "The central meridian, like -99; 0 if not given",
    "[-180,180)",
)
.core();
const SP1: Field = angle(
    "standard_parallel_1",
    "First standard parallel",
    "Like 28°23'",
    "[-90,90]",
)
.core();
const SP2: Field = angle(
    "standard_parallel_2",
    "Second standard parallel",
    "Like 30°17'",
    "[-90,90]",
)
.core();
const K0: Field = Field::new(
    "scale_factor",
    "Scale factor at the origin",
    "Like 0.994; 1 if not given",
    Kind::Number {
        min: 0.1,
        max: 10.0,
    },
);
const FE: Field = length(
    "false_easting",
    "False easting",
    "Added to every easting, like 2000000 ftUS; 0 if not given",
);
const FN: Field = length(
    "false_northing",
    "False northing",
    "Added to every northing, like 150000 m; 0 if not given",
);
const EASTING: Field = length("easting", "Easting", "Like 903277.8 m")
    .required()
    .core();
const NORTHING: Field = length("northing", "Northing", "Like 77650.9 m")
    .required()
    .core();
const LCC_VARIANT: Field = Field::new(
    "variant",
    "Variant",
    "2SP (two standard parallels, the default) or 1SP (one parallel, the origin's, with a scale factor)",
    Kind::Choice(&["2SP", "1SP"]),
);
const POLE: Field = Field::new(
    "pole",
    "Pole",
    "N or S: the pole at the center; N if not given",
    Kind::Choice(&["N", "S"]),
)
.core();
const PS_PARALLEL: Field = angle(
    "standard_parallel",
    "Standard parallel",
    "Variant B: the latitude where the scale is true, like -71; leave empty for variant A",
    "[-90,90]",
);
const EQC_PARALLEL: Field = angle(
    "standard_parallel",
    "Standard parallel",
    "The parallel where east-west distances are true, like 30; 0 (the equator) if not given",
    "[-90,90]",
)
.core();

const FORWARD_OUT: &[Field] = &[
    Field::new(
        "easting",
        "Easting",
        "Meters, with the false easting",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(Precision::Decimals(3)),
    Field::new(
        "northing",
        "Northing",
        "Meters, with the false northing",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(Precision::Decimals(3)),
    Field::new(
        "convergence",
        "Grid convergence",
        "Bearing of grid north clockwise from true north",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(7))
    .angle_range("unbounded"),
    Field::new(
        "scale_meridian",
        "Scale along the meridian",
        "Grid distance over ground distance, north-south (h)",
        Kind::Number { min: 0.0, max: 1e6 },
    )
    .precision(Precision::Decimals(9)),
    Field::new(
        "scale_parallel",
        "Scale along the parallel",
        "Grid distance over ground distance, east-west (k)",
        Kind::Number { min: 0.0, max: 1e6 },
    )
    .precision(Precision::Decimals(9)),
];

const INVERSE_OUT: &[Field] = &[
    Field::new(
        "lat",
        "Latitude",
        "Degrees, north positive",
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
        "Degrees, east positive",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .precision(Precision::Decimals(9))
    .angle_range("[-180,180)"),
];

const FORWARD_SENTENCE: &str = "Easting {easting}, northing {northing}.";
const INVERSE_SENTENCE: &str = "That is {lat}, {lon}.";
const FORWARD_LAYER: &[Layer] = &[Layer {
    kind: "point",
    map: &[("easting", "easting"), ("northing", "northing")],
}];
const INVERSE_LAYER: &[Layer] = &[Layer {
    kind: "point",
    map: &[("lat", "lat"), ("lon", "lon")],
}];

// ------------------------------------------------------------ reading

fn opt_angle(ctx: &mut Ctx, name: &str) -> Result<Option<f64>, ToolError> {
    point::plain_angle(ctx, name)
}

fn opt_lat(ctx: &mut Ctx, name: &str) -> Result<Option<f64>, ToolError> {
    match opt_angle(ctx, name)? {
        Some(v) => Ok(Some(check_lat(v, &format!("/{name}"))?)),
        None => Ok(None),
    }
}

fn req_lat(ctx: &mut Ctx, name: &str, why: &str) -> Result<f64, ToolError> {
    opt_lat(ctx, name)?.ok_or_else(|| ToolError::invalid(&format!("/{name}"), why.to_owned()))
}

fn opt_len(ctx: &mut Ctx, name: &str) -> Result<f64, ToolError> {
    let m = unit(QT::Length, "m");
    Ok(ctx.quantity(name)?.map(|q| q.to(m)).unwrap_or(0.0))
}

fn ellipsoid(ctx: &mut Ctx) -> Result<Ellipsoid, ToolError> {
    let e = Ellipsoid::from_ctx(ctx)?;
    if e.f < 0.0 || e.f >= 0.5 {
        return Err(ToolError::new(
            ErrorCode::Unsupported,
            "These projections need an oblate ellipsoid or a sphere (flattening from 0 up to 0.5).",
        )
        .at("/inverse_flattening"));
    }
    Ok(e)
}

/// A parameter that belongs to another variant is refused rather than ignored.
fn refuse(ctx: &Ctx, name: &str, why: &str) -> Result<(), ToolError> {
    if ctx.is_set(name) {
        return Err(ToolError::invalid(&format!("/{name}"), why.to_owned()));
    }
    Ok(())
}

/// The standard parallel of a conic can not be a pole, and two parallels
/// mirrored about the equator make a cylinder, not a cone.
fn check_conic(p1: f64, p2: f64) -> Result<(), ToolError> {
    for (v, at) in [(p1, "/standard_parallel_1"), (p2, "/standard_parallel_2")] {
        if v.abs() >= 90.0 {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "A standard parallel must lie between the poles, not on one.",
            )
            .at(at));
        }
    }
    if (p1 + p2).abs() < 1e-9 {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "Standard parallels mirrored about the equator make the cone a cylinder; this method needs them on the same side, or unequal.",
        )
        .at("/standard_parallel_2")
        .hint("For a cylinder, use Web Mercator, Equidistant Cylindrical, or a transverse Mercator."));
    }
    Ok(())
}

fn finite(lat: f64, lon: f64, at: &str) -> Result<(f64, f64), ToolError> {
    if lat.is_finite() && lon.is_finite() {
        Ok((lat, lon))
    } else {
        Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "These coordinates are outside the projection's domain.",
        )
        .at(at))
    }
}

fn forward_json(ctx: &mut Ctx, g: Grid) -> Result<Json, ToolError> {
    if !(g.e.is_finite() && g.n.is_finite()) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "This point projects to infinity: it is the pole the projection can not show.",
        )
        .at("/lat"));
    }
    Ok(Json::obj([
        ("easting", ctx.out("easting", meters(g.e))),
        ("northing", ctx.out("northing", meters(g.n))),
        ("convergence", ctx.out("convergence", deg(g.convergence))),
        ("scale_meridian", Json::Num(g.h)),
        ("scale_parallel", Json::Num(g.k)),
    ]))
}

fn inverse_json(ctx: &mut Ctx, (lat, lon): (f64, f64)) -> Result<Json, ToolError> {
    let (lat, lon) = finite(lat, lon, "/easting")?;
    Ok(Json::obj([
        ("lat", ctx.out("lat", deg(lat))),
        ("lon", ctx.out("lon", deg(lon))),
    ]))
}

fn grid_in(ctx: &mut Ctx) -> Result<(f64, f64), ToolError> {
    let m = unit(QT::Length, "m");
    Ok((
        ctx.req_quantity("easting")?.to(m),
        ctx.req_quantity("northing")?.to(m),
    ))
}

// ------------------------------------------------------------ Web Mercator

pub static WEB_MERCATOR_FORWARD: ToolDef = ToolDef {
    id: "geodesy.projection.web-mercator-forward",
    title: "Latitude and longitude to Web Mercator",
    summary: "Converts a WGS 84 latitude and longitude to Web Mercator (EPSG:3857) x and y, the grid of web map tiles, with its scale along the meridian and the parallel.",
    aliases: &[
        "lat long to Web Mercator",
        "EPSG:3857 converter",
        "pseudo Mercator",
    ],
    keywords: &[
        "Web Mercator",
        "3857",
        "pseudo Mercator",
        "map tiles",
        "projection",
    ],
    inputs: &[LAT, LON],
    outputs: FORWARD_OUT,
    errors: &[ErrorCode::OutOfDomain],
    stability: Stability::Stable,
    when_to_use: "Use this to place a latitude and longitude on a web map's grid: the meters that tile servers, vector tiles, and most online maps work in. It is the projection behind EPSG:3857, and the numbers match what those maps and PROJ produce.",
    limitations: "Web Mercator applies the sphere's Mercator formulas to WGS 84 latitudes, so it is not conformal on the ellipsoid: the scale north-south and east-west differ by up to about 0.7 percent, and angles are slightly off. Its scale grows without bound toward the poles, and the map stops at 85.05112878 degrees north and south, where it becomes square; points beyond are refused. Its distances and areas are not ground distances and areas: at 60 degrees a map meter is half a ground meter.",
    warnings: &["INPUT_NORMALIZED"],
    model: "Popular Visualisation Pseudo Mercator (EPSG method 1024), WGS 84 semi-major axis",
    accuracy: "Exact to double precision; agrees with PROJ within 1 mm",
    references: &[G7_2, NGA_WM],
    examples: &[Example {
        id: "primary",
        title: "The IOGP example in Mexico",
        input: r#"{"lat":"24°22'54.433\"N","lon":"100°20'00.000\"W"}"#,
        source: "IOGP Guidance Note 7-2, 3.4.5 example: E = -11,169,055.58 m, N = 2,800,000.00 m",
    }],
    primary_example: "primary",
    visualization: FORWARD_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.web-mercator-inverse",
            reason: "inverse",
        },
        Related {
            id: "indexing.tile.from-point",
            reason: "next",
        },
        Related {
            id: "geodesy.utm.forward",
            reason: "alternative",
        },
    ],
    sentence: FORWARD_SENTENCE,
    limits: &[("batchRows", 10_000)],
    run: run_web_mercator_forward,
    ..ToolDef::BLANK
};

fn web_mercator_limit_error(at: &str) -> ToolError {
    ToolError::new(
        ErrorCode::OutOfDomain,
        format!(
            "Web Mercator stops at ±{:.8}°, where the map becomes square.",
            proj::web_mercator_limit()
        ),
    )
    .at(at)
}

fn run_web_mercator_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    if lat.abs() > proj::web_mercator_limit() {
        return Err(web_mercator_limit_error("/lat"));
    }
    forward_json(ctx, WebMercator::forward(lat, lon))
}

pub static WEB_MERCATOR_INVERSE: ToolDef = ToolDef {
    id: "geodesy.projection.web-mercator-inverse",
    title: "Web Mercator to latitude and longitude",
    summary: "Converts Web Mercator (EPSG:3857) x and y back to a WGS 84 latitude and longitude.",
    aliases: &["Web Mercator to lat long", "EPSG:3857 to WGS 84"],
    keywords: &[
        "Web Mercator",
        "3857",
        "pseudo Mercator",
        "inverse",
        "projection",
    ],
    inputs: &[EASTING, NORTHING],
    outputs: INVERSE_OUT,
    errors: &[ErrorCode::OutOfDomain],
    stability: Stability::Stable,
    when_to_use: "Use this to turn web map meters back into a latitude and longitude: a coordinate copied from a tile server, a vector tile, or a GIS layer in EPSG:3857. The result is on WGS 84, ready for a GPS, a geodesic distance, or another grid.",
    limitations: "The easting and northing must lie on the map: within 20,037,508.34 m of the origin each way, the half circumference of the sphere the formulas use. Beyond that the point is off the edge and is refused rather than wrapped around. The latitude comes back on WGS 84 as the map assumes, but a coordinate from a map that was drawn on another datum carries that datum's offset, which this can not see.",
    warnings: &["UNIT_ASSUMED"],
    model: "Popular Visualisation Pseudo Mercator (EPSG method 1024), WGS 84 semi-major axis",
    accuracy: "Exact to double precision",
    references: &[G7_2, NGA_WM],
    examples: &[Example {
        id: "primary",
        title: "Back to the IOGP example",
        input: r#"{"easting":"-11169055.58 m","northing":"2800000 m"}"#,
        source: "IOGP Guidance Note 7-2, 3.4.5 example reversed: 24°22'54.433\" N, 100°20'00.000\" W",
    }],
    primary_example: "primary",
    visualization: INVERSE_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.web-mercator-forward",
            reason: "inverse",
        },
        Related {
            id: "geodesy.parse.format",
            reason: "next",
        },
        Related {
            id: "geodesy.utm.inverse",
            reason: "alternative",
        },
    ],
    sentence: INVERSE_SENTENCE,
    limits: &[("batchRows", 10_000)],
    run: run_web_mercator_inverse,
    ..ToolDef::BLANK
};

fn run_web_mercator_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (x, y) = grid_in(ctx)?;
    let edge = core::f64::consts::PI * proj::WGS84_A;
    // A millimeter of slack for a corner written to the millimeter.
    if x.abs() > edge + 1e-3 {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            format!(
                "The easting is off the map, which ends {edge:.2} m east and west of the origin."
            ),
        )
        .at("/easting"));
    }
    if y.abs() > edge + 1e-3 {
        return Err(web_mercator_limit_error("/northing"));
    }
    inverse_json(ctx, WebMercator::inverse(x.clamp(-edge, edge), y))
}

// ------------------------------------------------------------ Lambert Conic Conformal

fn lcc(ctx: &mut Ctx) -> Result<Lcc, ToolError> {
    let e = ellipsoid(ctx)?;
    let lon0 = opt_angle(ctx, "longitude_of_origin")?.unwrap_or(0.0);
    let (fe, fn_) = (
        opt_len(ctx, "false_easting")?,
        opt_len(ctx, "false_northing")?,
    );
    if ctx.choice("variant")? == Some("1SP") {
        let why = "The one-parallel variant uses the origin's parallel; give the latitude of origin, not standard parallels.";
        refuse(ctx, "standard_parallel_1", why)?;
        refuse(ctx, "standard_parallel_2", why)?;
        let lat0 = req_lat(
            ctx,
            "latitude_of_origin",
            "The one-parallel variant needs the latitude of origin, which is its standard parallel.",
        )?;
        if lat0.abs() >= 90.0 || lat0 == 0.0 {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "The latitude of origin must lie between the equator and a pole: on the equator the cone is a cylinder.",
            )
            .at("/latitude_of_origin"));
        }
        let k0 = ctx.number("scale_factor")?.unwrap_or(1.0);
        return Ok(Lcc::one_sp(e.a, e.f, lat0, lon0, k0, fe, fn_));
    }
    refuse(
        ctx,
        "scale_factor",
        "The two-parallel variant is true to scale on both standard parallels; a scale factor belongs to the one-parallel variant.",
    )?;
    let why = "The two-parallel variant needs both standard parallels.";
    let p1 = req_lat(ctx, "standard_parallel_1", why)?;
    let p2 = req_lat(ctx, "standard_parallel_2", why)?;
    check_conic(p1, p2)?;
    let lat0 = opt_lat(ctx, "latitude_of_origin")?.unwrap_or(0.0);
    Ok(Lcc::two_sp(e.a, e.f, lat0, lon0, p1, p2, fe, fn_))
}

pub static LCC_FORWARD: ToolDef = ToolDef {
    id: "geodesy.projection.lcc-forward",
    title: "Latitude and longitude to Lambert Conformal Conic",
    summary: "Projects a latitude and longitude with a Lambert Conformal Conic you define: one or two standard parallels, origin, false easting and northing, on any ellipsoid.",
    aliases: &[
        "Lambert conformal conic calculator",
        "LCC projection",
        "lat long to Lambert",
    ],
    keywords: &[
        "Lambert",
        "conformal conic",
        "projection",
        "standard parallels",
        "9802",
    ],
    inputs: &[
        LAT,
        LON,
        LCC_VARIANT,
        SP1,
        SP2,
        LAT0,
        LON0,
        K0,
        FE,
        FN,
        E[0],
        E[1],
        E[2],
    ],
    outputs: FORWARD_OUT,
    errors: &[ErrorCode::OutOfDomain, ErrorCode::Unsupported],
    stability: Stability::Stable,
    when_to_use: "Use this when a grid is a Lambert Conformal Conic that is not in a ready-made list: a national or regional grid, an aeronautical chart, a local engineering projection, or one you are designing. Give its parameters as its definition states them and get the easting and northing, with the grid convergence and the scale at the point.",
    limitations: "It computes the projection you describe; it does not know which grid a coordinate belongs to, and parameters copied with a wrong sign, a latitude of origin swapped for a standard parallel, or feet taken for meters give a clean but wrong answer. The pole on the far side of the cone projects to infinity and is refused. For State Plane zones, the state plane tool already has every zone's parameters. Coordinates are on the ellipsoid you choose; a datum shift is a separate step.",
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED"],
    model: "Lambert Conic Conformal 1SP and 2SP (EPSG methods 9801 and 9802)",
    accuracy: "Exact to double precision; agrees with PROJ within 1 mm",
    references: &[G7_2],
    examples: &[Example {
        id: "primary",
        title: "The IOGP example: NAD27 Texas South Central",
        input: r#"{"lat":"28°30'00\"N","lon":"96°00'00\"W","standard_parallel_1":"28°23'","standard_parallel_2":"30°17'","latitude_of_origin":"27°50'","longitude_of_origin":-99,"false_easting":"2000000 ftUS","ellipsoid":"clarke1866"}"#,
        source: "IOGP Guidance Note 7-2, 3.2.1.1 example: E = 2,963,503.91 ftUS, N = 254,759.80 ftUS",
    }],
    primary_example: "primary",
    visualization: FORWARD_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.lcc-inverse",
            reason: "inverse",
        },
        Related {
            id: "geodesy.spcs.spcs83-forward",
            reason: "alternative",
        },
        Related {
            id: "geodesy.projection.albers-forward",
            reason: "alternative",
        },
    ],
    sentence: FORWARD_SENTENCE,
    limits: &[("batchRows", 10_000)],
    run: run_lcc_forward,
    ..ToolDef::BLANK
};

fn run_lcc_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let p = lcc(ctx)?;
    forward_json(ctx, p.forward(lat, lon))
}

pub static LCC_INVERSE: ToolDef = ToolDef {
    id: "geodesy.projection.lcc-inverse",
    title: "Lambert Conformal Conic to latitude and longitude",
    summary: "Converts an easting and northing on a Lambert Conformal Conic you define back to latitude and longitude.",
    aliases: &["Lambert to lat long", "LCC inverse"],
    keywords: &[
        "Lambert",
        "conformal conic",
        "inverse",
        "projection",
        "9802",
    ],
    inputs: &[
        EASTING,
        NORTHING,
        LCC_VARIANT,
        SP1,
        SP2,
        LAT0,
        LON0,
        K0,
        FE,
        FN,
        E[0],
        E[1],
        E[2],
    ],
    outputs: INVERSE_OUT,
    errors: &[ErrorCode::OutOfDomain, ErrorCode::Unsupported],
    stability: Stability::Stable,
    when_to_use: "Use this to turn an easting and northing on a Lambert Conformal Conic grid back into latitude and longitude, when the grid is one you have the parameters for but no ready-made converter: a national grid, a chart, or a local engineering projection.",
    limitations: "The parameters must be the grid's own, in its units: the result is only as right as they are, and nothing in an easting and northing says which grid they came from. The latitude and longitude are on the ellipsoid you choose, which should be the grid's; moving them to another datum is a separate step. For State Plane zones, the state plane tool already knows each zone.",
    warnings: &["UNIT_ASSUMED"],
    model: "Lambert Conic Conformal 1SP and 2SP (EPSG methods 9801 and 9802)",
    accuracy: "Exact to double precision; latitude solved by iteration to 1e-14 radians",
    references: &[G7_2],
    examples: &[Example {
        id: "primary",
        title: "Back to the IOGP example",
        input: r#"{"easting":"2963503.91 ftUS","northing":"254759.80 ftUS","standard_parallel_1":"28°23'","standard_parallel_2":"30°17'","latitude_of_origin":"27°50'","longitude_of_origin":-99,"false_easting":"2000000 ftUS","ellipsoid":"clarke1866"}"#,
        source: "IOGP Guidance Note 7-2, 3.2.1.1 example reversed: 28°30' N, 96°00' W",
    }],
    primary_example: "primary",
    visualization: INVERSE_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.lcc-forward",
            reason: "inverse",
        },
        Related {
            id: "geodesy.spcs.spcs83-inverse",
            reason: "alternative",
        },
        Related {
            id: "geodesy.parse.format",
            reason: "next",
        },
    ],
    sentence: INVERSE_SENTENCE,
    limits: &[("batchRows", 10_000)],
    run: run_lcc_inverse,
    ..ToolDef::BLANK
};

fn run_lcc_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (x, y) = grid_in(ctx)?;
    let p = lcc(ctx)?;
    inverse_json(ctx, p.inverse(x, y))
}

// ------------------------------------------------------------ Albers

fn albers(ctx: &mut Ctx) -> Result<Albers, ToolError> {
    let e = ellipsoid(ctx)?;
    let why = "Albers needs both standard parallels (they may be equal).";
    let p1 = req_lat(ctx, "standard_parallel_1", why)?;
    let p2 = req_lat(ctx, "standard_parallel_2", why)?;
    check_conic(p1, p2)?;
    let lat0 = opt_lat(ctx, "latitude_of_origin")?.unwrap_or(0.0);
    let lon0 = opt_angle(ctx, "longitude_of_origin")?.unwrap_or(0.0);
    let (fe, fn_) = (
        opt_len(ctx, "false_easting")?,
        opt_len(ctx, "false_northing")?,
    );
    Ok(Albers::new(e.a, e.f, lat0, lon0, p1, p2, fe, fn_))
}

pub static ALBERS_FORWARD: ToolDef = ToolDef {
    id: "geodesy.projection.albers-forward",
    title: "Latitude and longitude to Albers Equal Area",
    summary: "Projects a latitude and longitude with an Albers Equal Area conic you define, the projection for maps where areas must be true, on any ellipsoid.",
    aliases: &[
        "Albers equal area calculator",
        "Albers projection",
        "lat long to Albers",
    ],
    keywords: &["Albers", "equal area", "conic", "projection", "9822"],
    inputs: &[LAT, LON, SP1, SP2, LAT0, LON0, FE, FN, E[0], E[1], E[2]],
    outputs: FORWARD_OUT,
    errors: &[ErrorCode::OutOfDomain, ErrorCode::Unsupported],
    stability: Stability::Stable,
    when_to_use: "Use this to put a point on an Albers Equal Area grid, the conic that keeps areas true: the one behind the conterminous US maps of the USGS and Census Bureau, many national land-cover and statistics grids, and any map you measure areas on. Give its standard parallels and origin as the grid's definition states them.",
    limitations: "Albers keeps areas and gives up shapes and distances: east-west and north-south scales multiply to 1 but differ away from the standard parallels, so a distance measured on the grid is not a ground distance. Both scales are reported so you can see how far apart they are. Parameters must be the grid's own; nothing here checks that a coordinate belongs to the grid. Coordinates are on the ellipsoid you choose.",
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED"],
    model: "Albers Equal Area (EPSG method 9822)",
    accuracy: "Exact to double precision; agrees with PROJ within 1 mm",
    references: &[G7_2, SNYDER],
    examples: &[Example {
        id: "primary",
        title: "Snyder's example: the conterminous US conic",
        input: r#"{"lat":35,"lon":-75,"standard_parallel_1":29.5,"standard_parallel_2":45.5,"latitude_of_origin":23,"longitude_of_origin":-96,"ellipsoid":"clarke1866"}"#,
        source: "Snyder (1987), USGS Professional Paper 1395, p. 292: x = 1,885,472.7 m, y = 1,535,925.0 m",
    }],
    primary_example: "primary",
    visualization: FORWARD_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.albers-inverse",
            reason: "inverse",
        },
        Related {
            id: "geodesy.projection.lcc-forward",
            reason: "alternative",
        },
        Related {
            id: "geometry.area.polygon",
            reason: "next",
        },
    ],
    sentence: FORWARD_SENTENCE,
    limits: &[("batchRows", 10_000)],
    run: run_albers_forward,
    ..ToolDef::BLANK
};

fn run_albers_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let p = albers(ctx)?;
    forward_json(ctx, p.forward(lat, lon))
}

pub static ALBERS_INVERSE: ToolDef = ToolDef {
    id: "geodesy.projection.albers-inverse",
    title: "Albers Equal Area to latitude and longitude",
    summary: "Converts an easting and northing on an Albers Equal Area conic you define back to latitude and longitude.",
    aliases: &["Albers to lat long", "Albers inverse"],
    keywords: &["Albers", "equal area", "inverse", "projection", "9822"],
    inputs: &[
        EASTING, NORTHING, SP1, SP2, LAT0, LON0, FE, FN, E[0], E[1], E[2],
    ],
    outputs: INVERSE_OUT,
    errors: &[ErrorCode::OutOfDomain, ErrorCode::Unsupported],
    stability: Stability::Stable,
    when_to_use: "Use this to turn an Albers Equal Area easting and northing back into latitude and longitude: a cell corner from a land-cover or statistics grid, or a coordinate from a GIS layer in an Albers projection, when you have the grid's parameters.",
    limitations: "A point outside the ring the projection can reach, beyond where the poles land, has no latitude and is refused. The parameters must be the grid's own and in its units; nothing in the numbers says which grid they came from. The latitude is solved exactly rather than by the short series in the guidance note, so it agrees with PROJ to well under a millimeter. The result is on the ellipsoid you choose.",
    warnings: &["UNIT_ASSUMED"],
    model: "Albers Equal Area (EPSG method 9822)",
    accuracy: "Agrees with PROJ within 0.01 mm; forward and back return the point within 0.1 µm to 80° of latitude",
    references: &[G7_2, SNYDER],
    examples: &[Example {
        id: "primary",
        title: "Back to Snyder's example",
        input: r#"{"easting":"1885472.7 m","northing":"1535925 m","standard_parallel_1":29.5,"standard_parallel_2":45.5,"latitude_of_origin":23,"longitude_of_origin":-96,"ellipsoid":"clarke1866"}"#,
        source: "Snyder (1987), USGS Professional Paper 1395, p. 293: 35° N, 75° W",
    }],
    primary_example: "primary",
    visualization: INVERSE_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.albers-forward",
            reason: "inverse",
        },
        Related {
            id: "geodesy.projection.lcc-inverse",
            reason: "alternative",
        },
        Related {
            id: "geodesy.parse.format",
            reason: "next",
        },
    ],
    sentence: INVERSE_SENTENCE,
    limits: &[("batchRows", 10_000)],
    run: run_albers_inverse,
    ..ToolDef::BLANK
};

fn run_albers_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (x, y) = grid_in(ctx)?;
    let p = albers(ctx)?;
    match p.inverse_checked(x, y) {
        Some(ll) => inverse_json(ctx, ll),
        None => Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "This easting and northing are outside the area the projection covers: beyond where the poles land.",
        )
        .at("/easting")),
    }
}

// ------------------------------------------------------------ Polar Stereographic

fn polar(ctx: &mut Ctx) -> Result<PolarStereo, ToolError> {
    let e = ellipsoid(ctx)?;
    let lon0 = opt_angle(ctx, "longitude_of_origin")?.unwrap_or(0.0);
    let (fe, fn_) = (
        opt_len(ctx, "false_easting")?,
        opt_len(ctx, "false_northing")?,
    );
    let pole = ctx.choice("pole")?;
    if let Some(sp) = opt_lat(ctx, "standard_parallel")? {
        refuse(
            ctx,
            "scale_factor",
            "Give a standard parallel (variant B) or a scale factor at the pole (variant A), not both.",
        )?;
        if sp == 0.0 {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "The standard parallel can not be the equator: it must lie in the pole's hemisphere.",
            )
            .at("/standard_parallel"));
        }
        if pole.is_some_and(|p| (p == "N") != (sp > 0.0)) {
            return Err(ToolError::invalid(
                "/standard_parallel",
                "The standard parallel must lie in the hemisphere of the chosen pole.",
            ));
        }
        return Ok(PolarStereo::variant_b(e.a, e.f, sp, lon0, fe, fn_));
    }
    let k0 = ctx.number("scale_factor")?.unwrap_or(1.0);
    Ok(PolarStereo::variant_a(
        e.a,
        e.f,
        pole != Some("S"),
        lon0,
        k0,
        fe,
        fn_,
    ))
}

pub static POLAR_FORWARD: ToolDef = ToolDef {
    id: "geodesy.projection.polar-stereographic-forward",
    title: "Latitude and longitude to Polar Stereographic",
    summary: "Projects a latitude and longitude with a Polar Stereographic you define, by the scale at the pole (variant A) or a standard parallel (variant B), on any ellipsoid.",
    aliases: &[
        "polar stereographic calculator",
        "lat long to polar stereographic",
        "Antarctic grid",
    ],
    keywords: &[
        "polar stereographic",
        "Arctic",
        "Antarctic",
        "projection",
        "9810",
        "9829",
    ],
    inputs: &[
        LAT,
        LON,
        POLE,
        PS_PARALLEL,
        LON0,
        K0,
        FE,
        FN,
        E[0],
        E[1],
        E[2],
    ],
    outputs: FORWARD_OUT,
    errors: &[ErrorCode::OutOfDomain, ErrorCode::Unsupported],
    stability: Stability::Stable,
    when_to_use: "Use this for a polar grid other than UPS: the Antarctic and Arctic polar stereographic grids of sea-ice, ice-sheet, and satellite products, or national polar grids. Give the pole and either the scale at the pole or the latitude of true scale, as the grid's definition does; UPS itself is the pole with a scale of 0.994.",
    limitations: "The projection is centered on a pole and grows without bound toward the other one, which is refused; far from its pole the scale is large, so it suits the polar regions, not the whole globe. Grid north is the central meridian's direction, so the convergence equals the longitude difference and can be large. For UPS proper, the UPS tool adds the zone letters and limits. Coordinates are on the ellipsoid you choose.",
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED"],
    model: "Polar Stereographic variants A and B (EPSG methods 9810 and 9829)",
    accuracy: "Exact to double precision; agrees with PROJ within 1 mm",
    references: &[G7_2],
    examples: &[Example {
        id: "primary",
        title: "The IOGP example: Australian Antarctic grid",
        input: r#"{"lat":-75,"lon":120,"pole":"S","standard_parallel":-71,"longitude_of_origin":70,"false_easting":"6000000 m","false_northing":"6000000 m"}"#,
        source: "IOGP Guidance Note 7-2, 3.2.2.2 example: E = 7,255,380.79 m, N = 7,053,389.56 m",
    }],
    primary_example: "primary",
    visualization: FORWARD_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.polar-stereographic-inverse",
            reason: "inverse",
        },
        Related {
            id: "geodesy.ups.forward",
            reason: "alternative",
        },
        Related {
            id: "geodesy.magnetic.grivation",
            reason: "next",
        },
    ],
    sentence: FORWARD_SENTENCE,
    limits: &[("batchRows", 10_000)],
    run: run_polar_forward,
    ..ToolDef::BLANK
};

fn run_polar_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let p = polar(ctx)?;
    forward_json(ctx, p.forward(lat, lon))
}

pub static POLAR_INVERSE: ToolDef = ToolDef {
    id: "geodesy.projection.polar-stereographic-inverse",
    title: "Polar Stereographic to latitude and longitude",
    summary: "Converts an easting and northing on a Polar Stereographic you define back to latitude and longitude.",
    aliases: &[
        "polar stereographic to lat long",
        "polar stereographic inverse",
    ],
    keywords: &[
        "polar stereographic",
        "Arctic",
        "Antarctic",
        "inverse",
        "projection",
    ],
    inputs: &[
        EASTING,
        NORTHING,
        POLE,
        PS_PARALLEL,
        LON0,
        K0,
        FE,
        FN,
        E[0],
        E[1],
        E[2],
    ],
    outputs: INVERSE_OUT,
    errors: &[ErrorCode::OutOfDomain, ErrorCode::Unsupported],
    stability: Stability::Stable,
    when_to_use: "Use this to turn a polar stereographic easting and northing back into latitude and longitude: a grid cell of a sea-ice or ice-sheet product, a polar satellite pixel, or a coordinate from an Antarctic or Arctic grid, when you have its parameters. Give them as the grid's definition does: the pole, and a scale factor or a standard parallel.",
    limitations: "Say which pole the grid is centered on: the same numbers mean different places around the north and south poles, and the pole is not guessed. At the pole itself the longitude has no meaning and the central meridian is returned. Parameters must be the grid's own and in its units. The result is on the ellipsoid you choose.",
    warnings: &["UNIT_ASSUMED"],
    model: "Polar Stereographic variants A and B (EPSG methods 9810 and 9829)",
    accuracy: "Exact to double precision; latitude solved by iteration to 1e-14 radians",
    references: &[G7_2],
    examples: &[Example {
        id: "primary",
        title: "Back to the IOGP example",
        input: r#"{"easting":"7255380.79 m","northing":"7053389.56 m","pole":"S","standard_parallel":-71,"longitude_of_origin":70,"false_easting":"6000000 m","false_northing":"6000000 m"}"#,
        source: "IOGP Guidance Note 7-2, 3.2.2.2 example reversed: 75° S, 120° E",
    }],
    primary_example: "primary",
    visualization: INVERSE_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.polar-stereographic-forward",
            reason: "inverse",
        },
        Related {
            id: "geodesy.ups.inverse",
            reason: "alternative",
        },
        Related {
            id: "geodesy.parse.format",
            reason: "next",
        },
    ],
    sentence: INVERSE_SENTENCE,
    limits: &[("batchRows", 10_000)],
    run: run_polar_inverse,
    ..ToolDef::BLANK
};

fn run_polar_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (x, y) = grid_in(ctx)?;
    let p = polar(ctx)?;
    inverse_json(ctx, p.inverse(x, y))
}

// ------------------------------------------------------------ Equidistant Cylindrical

fn eqc(ctx: &mut Ctx) -> Result<EquidistantCylindrical, ToolError> {
    let e = ellipsoid(ctx)?;
    let sp = opt_lat(ctx, "standard_parallel")?.unwrap_or(0.0);
    if sp.abs() >= 90.0 {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The standard parallel can not be a pole: there the parallel has no length.",
        )
        .at("/standard_parallel"));
    }
    let lon0 = opt_angle(ctx, "longitude_of_origin")?.unwrap_or(0.0);
    let (fe, fn_) = (
        opt_len(ctx, "false_easting")?,
        opt_len(ctx, "false_northing")?,
    );
    Ok(EquidistantCylindrical::new(e.a, e.f, sp, lon0, fe, fn_))
}

pub static EQC_FORWARD: ToolDef = ToolDef {
    id: "geodesy.projection.equidistant-cylindrical-forward",
    title: "Latitude and longitude to Equidistant Cylindrical",
    summary: "Projects a latitude and longitude with the ellipsoidal Equidistant Cylindrical (plate carrée), where the northing is the true distance from the equator along the meridian.",
    aliases: &[
        "plate carree",
        "equirectangular projection",
        "lat long to equidistant cylindrical",
    ],
    keywords: &[
        "equidistant cylindrical",
        "plate carrée",
        "equirectangular",
        "projection",
        "1028",
    ],
    inputs: &[LAT, LON, EQC_PARALLEL, LON0, FE, FN, E[0], E[1], E[2]],
    outputs: FORWARD_OUT,
    errors: &[ErrorCode::OutOfDomain, ErrorCode::Unsupported],
    stability: Stability::Stable,
    when_to_use: "Use this for the ellipsoidal Equidistant Cylindrical of EPSG:4087 and similar grids: the northing is the true distance along the meridian from the equator and the easting the distance along the standard parallel, so either can be read as a ground distance in its own direction.",
    limitations: "Distances are true only north-south, and east-west only on the standard parallel; elsewhere the east-west scale is the ratio of the two parallels' radii and grows to infinity at the poles. This is the ellipsoidal method of EPSG. PROJ's eqc is the spherical form, whose northing is the latitude times the semi-major axis, so on WGS 84 the two differ by up to 21 km at the poles. Coordinates are on the ellipsoid you choose.",
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED"],
    model: "Equidistant Cylindrical (EPSG method 1028), meridian distance by the Krüger series",
    accuracy: "Exact to double precision; the meridian distance agrees with GeographicLib to a nanometer",
    references: &[G7_2, crate::KARNEY_TM],
    examples: &[Example {
        id: "primary",
        title: "The IOGP example: WGS 84 / World Equidistant Cylindrical",
        input: r#"{"lat":55,"lon":10}"#,
        source: "IOGP Guidance Note 7-2, 3.3.3 example: E = 1,113,194.91 m, N = 6,097,230.31 m",
    }],
    primary_example: "primary",
    visualization: FORWARD_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.equidistant-cylindrical-inverse",
            reason: "inverse",
        },
        Related {
            id: "geodesy.projection.web-mercator-forward",
            reason: "alternative",
        },
        Related {
            id: "geodesy.ellipsoid.radii",
            reason: "next",
        },
    ],
    sentence: FORWARD_SENTENCE,
    limits: &[("batchRows", 10_000)],
    run: run_eqc_forward,
    ..ToolDef::BLANK
};

fn run_eqc_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let p = eqc(ctx)?;
    forward_json(ctx, p.forward(lat, lon))
}

pub static EQC_INVERSE: ToolDef = ToolDef {
    id: "geodesy.projection.equidistant-cylindrical-inverse",
    title: "Equidistant Cylindrical to latitude and longitude",
    summary: "Converts an easting and northing on the ellipsoidal Equidistant Cylindrical back to latitude and longitude.",
    aliases: &["plate carree to lat long", "equirectangular inverse"],
    keywords: &[
        "equidistant cylindrical",
        "plate carrée",
        "inverse",
        "projection",
        "1028",
    ],
    inputs: &[
        EASTING,
        NORTHING,
        EQC_PARALLEL,
        LON0,
        FE,
        FN,
        E[0],
        E[1],
        E[2],
    ],
    outputs: INVERSE_OUT,
    errors: &[ErrorCode::OutOfDomain, ErrorCode::Unsupported],
    stability: Stability::Stable,
    when_to_use: "Use this to turn an ellipsoidal Equidistant Cylindrical easting and northing back into latitude and longitude, as in EPSG:4087, when you have the grid's standard parallel and origin. The northing is a true distance along the meridian, so the latitude it gives back is the one that distance reaches from the equator.",
    limitations: "A northing beyond the pole, more than the meridian quadrant from the equator (10,001,965.73 m on WGS 84), has no latitude and is refused. An easting past half the parallel's circumference wraps to the other side of the world. The spherical form PROJ calls eqc gives different numbers on an ellipsoid; use the parameters of the grid you have. The result is on the ellipsoid you choose.",
    warnings: &["UNIT_ASSUMED"],
    model: "Equidistant Cylindrical (EPSG method 1028), meridian distance inverted by the Krüger series",
    accuracy: "Exact to double precision",
    references: &[G7_2, crate::KARNEY_TM],
    examples: &[Example {
        id: "primary",
        title: "Back to the IOGP example",
        input: r#"{"easting":"1113194.91 m","northing":"6097230.31 m"}"#,
        source: "IOGP Guidance Note 7-2, 3.3.3 example reversed: 55° N, 10° E",
    }],
    primary_example: "primary",
    visualization: INVERSE_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.equidistant-cylindrical-forward",
            reason: "inverse",
        },
        Related {
            id: "geodesy.projection.web-mercator-inverse",
            reason: "alternative",
        },
        Related {
            id: "geodesy.parse.format",
            reason: "next",
        },
    ],
    sentence: INVERSE_SENTENCE,
    limits: &[("batchRows", 10_000)],
    run: run_eqc_inverse,
    ..ToolDef::BLANK
};

fn run_eqc_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (x, y) = grid_in(ctx)?;
    let p = eqc(ctx)?;
    match p.inverse_checked(x, y) {
        Some(ll) => inverse_json(ctx, ll),
        None => Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "This northing is beyond the pole: more than a quarter meridian from the equator.",
        )
        .at("/northing")),
    }
}

// ------------------------------------------------------------ Azimuthal

const KARNEY_GEODESICS: Reference = Reference {
    title: "Algorithms for geodesics",
    issuer: "Karney, C. F. F., Journal of Geodesy",
    year: 2013,
    edition: "Vol. 87, No. 1",
    locator: "pp. 43-55, sections 8 (gnomonic) and 9 (azimuthal equidistant)",
    url: "https://doi.org/10.1007/s00190-012-0578-z",
};
const CENTER_LAT: Field = angle(
    "latitude_of_origin",
    "Latitude of the center",
    "The center of the projection, like 40",
    "[-90,90]",
)
.required()
.core();
const CENTER_LON: Field = angle(
    "longitude_of_origin",
    "Longitude of the center",
    "The center of the projection, like -100",
    "[-180,180)",
)
.required()
.core();

fn azimuthal(ctx: &mut Ctx, kind: Azimuthal) -> Result<AzimuthalProj, ToolError> {
    let e = ellipsoid(ctx)?;
    e.geodesic()?;
    let (lat0, lon0) = point::read(ctx, "latitude_of_origin", "longitude_of_origin")?;
    let (fe, fn_) = (
        opt_len(ctx, "false_easting")?,
        opt_len(ctx, "false_northing")?,
    );
    Ok(AzimuthalProj::new(kind, e.a, e.f, lat0, lon0, fe, fn_))
}

pub static AEQD_FORWARD: ToolDef = ToolDef {
    id: "geodesy.projection.azimuthal-equidistant-forward",
    title: "Latitude and longitude to Azimuthal Equidistant",
    summary: "Projects a latitude and longitude onto an Azimuthal Equidistant centered where you choose, so the distance and direction from the center are the true geodesic ones.",
    aliases: &[
        "azimuthal equidistant calculator",
        "distance and bearing map",
        "lat long to azimuthal equidistant",
    ],
    keywords: &[
        "azimuthal equidistant",
        "geodesic",
        "range rings",
        "projection",
        "center",
    ],
    inputs: &[LAT, LON, CENTER_LAT, CENTER_LON, FE, FN, E[0], E[1], E[2]],
    outputs: FORWARD_OUT,
    errors: &[ErrorCode::OutOfDomain, ErrorCode::Unsupported],
    stability: Stability::Stable,
    when_to_use: "Use this for a map centered on one place where every distance and direction from that place must be true: range rings around an airport or a transmitter, a radio or seismic station's map, or a local grid around a survey origin. The easting and northing are the geodesic distance from the center split by its direction.",
    limitations: "Only distances and directions from the center are true. Between two other points the grid distance is not the ground distance, and away from the center the scale across the radius grows, reaching infinity at the point opposite the center. Grid north is the direction of north at the center, so far from it the convergence is large. The distance comes from the exact geodesic, not a spherical or series approximation, so it agrees with the geodesic distance tool.",
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED"],
    model: "Azimuthal equidistant on the ellipsoid by the geodesic (Karney 2013)",
    accuracy: "Agrees with GeographicLib's GeodesicProj to a nanometer",
    references: &[KARNEY_GEODESICS, G7_2],
    examples: &[Example {
        id: "primary",
        title: "The IOGP example: Yap Islands",
        input: r#"{"lat":"9°35'47.493\"N","lon":"138°11'34.908\"E","latitude_of_origin":"9°32'48.15\"","longitude_of_origin":"138°10'07.48\"","false_easting":"40000 m","false_northing":"60000 m","ellipsoid":"clarke1866"}"#,
        source: "IOGP Guidance Note 7-2, 3.4.1 example (Modified Azimuthal Equidistant, which agrees with the exact method to a millimeter this close): E = 42,665.90 m, N = 65,509.82 m",
    }],
    primary_example: "primary",
    visualization: FORWARD_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.azimuthal-equidistant-inverse",
            reason: "inverse",
        },
        Related {
            id: "navigation.geodesic.inverse",
            reason: "alternative",
        },
        Related {
            id: "geodesy.projection.gnomonic-forward",
            reason: "alternative",
        },
    ],
    sentence: FORWARD_SENTENCE,
    limits: &[("batchRows", 10_000)],
    run: run_aeqd_forward,
    ..ToolDef::BLANK
};

fn run_aeqd_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let p = azimuthal(ctx, Azimuthal::Equidistant)?;
    match p.forward(lat, lon) {
        Some(g) => forward_json(ctx, g),
        None => Err(ToolError::new(ErrorCode::OutOfDomain, "This point has no image.").at("/lat")),
    }
}

pub static AEQD_INVERSE: ToolDef = ToolDef {
    id: "geodesy.projection.azimuthal-equidistant-inverse",
    title: "Azimuthal Equidistant to latitude and longitude",
    summary: "Converts an easting and northing on an Azimuthal Equidistant you center back to latitude and longitude: the point at that distance and direction from the center.",
    aliases: &[
        "azimuthal equidistant to lat long",
        "azimuthal equidistant inverse",
    ],
    keywords: &[
        "azimuthal equidistant",
        "geodesic",
        "inverse",
        "projection",
        "center",
    ],
    inputs: &[
        EASTING, NORTHING, CENTER_LAT, CENTER_LON, FE, FN, E[0], E[1], E[2],
    ],
    outputs: INVERSE_OUT,
    errors: &[ErrorCode::OutOfDomain, ErrorCode::Unsupported],
    stability: Stability::Stable,
    when_to_use: "Use this to turn an azimuthal equidistant easting and northing back into latitude and longitude: a point read off a range-ring map, a radar or station grid, or a local grid built around a survey origin, when you know its center.",
    limitations: "The center must be the grid's own; the same numbers around another center are another place. The easting and northing are read as a distance and a direction from the center, and the point is found by the exact geodesic, so a distance past half the way around the Earth comes back from the other side. The result is on the ellipsoid you choose, which should be the grid's.",
    warnings: &["UNIT_ASSUMED"],
    model: "Azimuthal equidistant on the ellipsoid by the geodesic (Karney 2013)",
    accuracy: "Agrees with GeographicLib's GeodesicProj to a nanometer",
    references: &[KARNEY_GEODESICS, G7_2],
    examples: &[Example {
        id: "primary",
        title: "Back to the IOGP example",
        input: r#"{"easting":"42665.90 m","northing":"65509.82 m","latitude_of_origin":"9°32'48.15\"","longitude_of_origin":"138°10'07.48\"","false_easting":"40000 m","false_northing":"60000 m","ellipsoid":"clarke1866"}"#,
        source: "IOGP Guidance Note 7-2, 3.4.1 example reversed: 9°35'47.493\" N, 138°11'34.908\" E",
    }],
    primary_example: "primary",
    visualization: INVERSE_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.azimuthal-equidistant-forward",
            reason: "inverse",
        },
        Related {
            id: "navigation.geodesic.direct",
            reason: "alternative",
        },
        Related {
            id: "geodesy.parse.format",
            reason: "next",
        },
    ],
    sentence: INVERSE_SENTENCE,
    limits: &[("batchRows", 10_000)],
    run: run_aeqd_inverse,
    ..ToolDef::BLANK
};

fn run_aeqd_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (x, y) = grid_in(ctx)?;
    let p = azimuthal(ctx, Azimuthal::Equidistant)?;
    inverse_json(ctx, p.inverse(x, y))
}

pub static GNOMONIC_FORWARD: ToolDef = ToolDef {
    id: "geodesy.projection.gnomonic-forward",
    title: "Latitude and longitude to Gnomonic",
    summary: "Projects a latitude and longitude onto the ellipsoidal gnomonic centered where you choose, where geodesics through the center are straight lines and all others nearly are.",
    aliases: &[
        "gnomonic calculator",
        "great circle map",
        "lat long to gnomonic",
    ],
    keywords: &[
        "gnomonic",
        "geodesic",
        "straight lines",
        "projection",
        "center",
    ],
    inputs: &[LAT, LON, CENTER_LAT, CENTER_LON, FE, FN, E[0], E[1], E[2]],
    outputs: FORWARD_OUT,
    errors: &[ErrorCode::OutOfDomain, ErrorCode::Unsupported],
    stability: Stability::Stable,
    when_to_use: "Use this when the shortest path must be a straight line on the map: planning great-circle routes on a chart, intersecting geodesics with plane geometry, or testing whether a point lies on the line between two others. Every geodesic through the center is straight, and near the center every geodesic is straight to within a tiny error.",
    limitations: "The gnomonic shows less than a hemisphere: a point a quarter of the way round the Earth or more from the center has no image and is refused. Toward that horizon the scale grows without bound, so it suits a region a few thousand kilometers across. On the ellipsoid, geodesics that do not pass through the center are only nearly straight: within 1,000 km of the center they bow by well under a millimeter.",
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED"],
    model: "Ellipsoidal gnomonic by the geodesic (Karney 2013, section 8)",
    accuracy: "Agrees with GeographicLib's GeodesicProj to a nanometer near the center",
    references: &[KARNEY_GEODESICS],
    examples: &[Example {
        id: "primary",
        title: "The Arctic, seen from the middle of North America",
        input: r#"{"lat":60,"lon":-30,"latitude_of_origin":40,"longitude_of_origin":-100}"#,
        source: "GeographicLib 2.x GeodesicProj -g 40 -100: x = 4,371,212.819 m, y = 5,140,517.234 m",
    }],
    primary_example: "primary",
    visualization: FORWARD_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.gnomonic-inverse",
            reason: "inverse",
        },
        Related {
            id: "geodesy.projection.azimuthal-equidistant-forward",
            reason: "alternative",
        },
        Related {
            id: "navigation.geodesic.inverse",
            reason: "next",
        },
    ],
    sentence: FORWARD_SENTENCE,
    limits: &[("batchRows", 10_000)],
    run: run_gnomonic_forward,
    ..ToolDef::BLANK
};

fn run_gnomonic_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let p = azimuthal(ctx, Azimuthal::Gnomonic)?;
    match p.forward(lat, lon) {
        Some(g) => forward_json(ctx, g),
        None => Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The gnomonic projection shows less than a hemisphere, and this point is 90° or more from the center (the angle between their verticals).",
        )
        .at("/lat")
        .hint("Move the center closer, or use the azimuthal equidistant, which reaches the whole globe.")),
    }
}

pub static GNOMONIC_INVERSE: ToolDef = ToolDef {
    id: "geodesy.projection.gnomonic-inverse",
    title: "Gnomonic to latitude and longitude",
    summary: "Converts an easting and northing on the ellipsoidal gnomonic you center back to latitude and longitude.",
    aliases: &["gnomonic to lat long", "gnomonic inverse"],
    keywords: &["gnomonic", "geodesic", "inverse", "projection", "center"],
    inputs: &[
        EASTING, NORTHING, CENTER_LAT, CENTER_LON, FE, FN, E[0], E[1], E[2],
    ],
    outputs: INVERSE_OUT,
    errors: &[ErrorCode::OutOfDomain, ErrorCode::Unsupported],
    stability: Stability::Stable,
    when_to_use: "Use this to bring a point found on a gnomonic plane back to latitude and longitude: the crossing of two routes drawn as straight lines on a gnomonic chart, or any result of plane geometry done where geodesics are straight, when you know the center.",
    limitations: "Every easting and northing has a point, but one far from the center lies close to the horizon a quarter of the way round the Earth, where a small move on the grid is a large one on the ground, so the latitude and longitude are less certain there. The point is found by Newton's method on the geodesic from the center, as GeographicLib does, and one that does not settle is refused. The center must be the grid's own.",
    warnings: &["UNIT_ASSUMED"],
    model: "Ellipsoidal gnomonic by the geodesic (Karney 2013, section 8), inverted by Newton's method",
    accuracy: "Agrees with GeographicLib's GeodesicProj to a nanometer near the center",
    references: &[KARNEY_GEODESICS],
    examples: &[Example {
        id: "primary",
        title: "Back to the Arctic point",
        input: r#"{"easting":"4371212.818782 m","northing":"5140517.234103 m","latitude_of_origin":40,"longitude_of_origin":-100}"#,
        source: "GeographicLib 2.x GeodesicProj -g 40 -100 -r: 60° N, 30° W",
    }],
    primary_example: "primary",
    visualization: INVERSE_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.gnomonic-forward",
            reason: "inverse",
        },
        Related {
            id: "geodesy.projection.azimuthal-equidistant-inverse",
            reason: "alternative",
        },
        Related {
            id: "geodesy.parse.format",
            reason: "next",
        },
    ],
    sentence: INVERSE_SENTENCE,
    limits: &[("batchRows", 10_000)],
    run: run_gnomonic_inverse,
    ..ToolDef::BLANK
};

fn run_gnomonic_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (x, y) = grid_in(ctx)?;
    let p = azimuthal(ctx, Azimuthal::Gnomonic)?;
    inverse_json(ctx, p.inverse(x, y))
}
