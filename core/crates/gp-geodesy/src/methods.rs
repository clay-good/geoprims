//! Projection methods with user-set parameters (geodesy/projections spec,
//! "General projection methods with custom parameters"): Web Mercator,
//! Lambert Conic Conformal (1SP and 2SP), Albers Equal Area, Polar
//! Stereographic (variants A and B), and Equidistant Cylindrical, forward and
//! inverse, with IOGP Guidance Note 7-2's parameter names. The math lives in
//! `gp_geo::proj`.

use gp_base::ErrorCode;
use gp_base::angle::check_lat;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{
    Assumption, Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, Stability,
    ToolDef,
};
use gp_base::units::{self, Quantity as QT, Unit};
use gp_geo::ellipsoid::{self, Ellipsoid};
use gp_geo::point;
use gp_geo::proj::{
    self, Albers, Azimuthal, AzimuthalProj, EquidistantCylindrical, Grid, Hotine, Laea, Lcc,
    Orthographic, PolarStereo, TmGrid, WebMercator,
};
use gp_geo::tmexact::TmExactGrid;

const G7_2: Reference = Reference {
    title: "Coordinate Conversions and Transformations including Formulas, IOGP Publication 373-7-2 (Guidance Note 7-2)",
    issuer: "International Association of Oil & Gas Producers (IOGP)",
    year: 2019,
    edition: "Revised September 2019",
    locator: "Sections 3.2.1 (Lambert Conic Conformal), 3.2.2 (Polar Stereographic), 3.2.4 (Albers Equal Area), 3.3.3 (Equidistant Cylindrical), and 3.4.5 (Popular Visualisation Pseudo Mercator)",
    url: "https://idms.iogp.org/Documents/474",
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
    url: "https://web.archive.org/web/20201114220929/https://earth-info.nga.mil/GandG/wgs84/web_mercator/(U)%20NGA_SIG_0011_1.0.0_WEBMERC.pdf",
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
    assumptions: &[
        Assumption {
            name: "Sphere radius, the WGS 84 semi-major axis",
            value: "6378137",
            unit: "m",
            source: "nga-webmerc",
        },
        Assumption {
            name: "Latitude limit",
            value: "85.05112878",
            unit: "deg",
            source: "nga-webmerc",
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
        "Web Mercator inverse",
        "3857",
        "pseudo Mercator inverse",
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
    assumptions: &[
        Assumption {
            name: "Sphere radius, the WGS 84 semi-major axis",
            value: "6378137",
            unit: "m",
            source: "nga-webmerc",
        },
        Assumption {
            name: "Latitude limit",
            value: "85.05112878",
            unit: "deg",
            source: "nga-webmerc",
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
    assumptions: &gp_base::tool::WGS84_ASSUMPTIONS,
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
        "Lambert inverse",
        "conformal conic inverse",
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
    assumptions: &gp_base::tool::WGS84_ASSUMPTIONS,
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
    keywords: &[
        "Albers",
        "equal area",
        "conic",
        "projection",
        "standard parallels",
        "9822",
    ],
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
    assumptions: &gp_base::tool::WGS84_ASSUMPTIONS,
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
    keywords: &[
        "Albers inverse",
        "equal area inverse",
        "inverse",
        "projection",
        "9822",
    ],
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
    assumptions: &gp_base::tool::WGS84_ASSUMPTIONS,
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
    assumptions: &gp_base::tool::WGS84_ASSUMPTIONS,
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
        "polar stereographic inverse",
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
    assumptions: &gp_base::tool::WGS84_ASSUMPTIONS,
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
    assumptions: &gp_base::tool::WGS84_ASSUMPTIONS,
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
        "equidistant cylindrical inverse",
        "plate carrée inverse",
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
    assumptions: &gp_base::tool::WGS84_ASSUMPTIONS,
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
    assumptions: &gp_base::tool::WGS84_ASSUMPTIONS,
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
        "azimuthal equidistant inverse",
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
    assumptions: &gp_base::tool::WGS84_ASSUMPTIONS,
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
    assumptions: &gp_base::tool::WGS84_ASSUMPTIONS,
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
    keywords: &[
        "gnomonic inverse",
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
    assumptions: &gp_base::tool::WGS84_ASSUMPTIONS,
    ..ToolDef::BLANK
};

fn run_gnomonic_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (x, y) = grid_in(ctx)?;
    let p = azimuthal(ctx, Azimuthal::Gnomonic)?;
    inverse_json(ctx, p.inverse(x, y))
}

// ------------------------------------------------------------ Orthographic

fn orthographic(ctx: &mut Ctx) -> Result<Orthographic, ToolError> {
    let e = ellipsoid(ctx)?;
    let (lat0, lon0) = point::read(ctx, "latitude_of_origin", "longitude_of_origin")?;
    let (fe, fn_) = (
        opt_len(ctx, "false_easting")?,
        opt_len(ctx, "false_northing")?,
    );
    Ok(Orthographic::new(e.a, e.f, lat0, lon0, fe, fn_))
}

pub static ORTHO_FORWARD: ToolDef = ToolDef {
    id: "geodesy.projection.orthographic-forward",
    title: "Latitude and longitude to Orthographic",
    summary: "Projects a latitude and longitude onto the ellipsoidal Orthographic, the Earth as seen from far out in space above a center you choose.",
    aliases: &[
        "orthographic calculator",
        "globe view projection",
        "lat long to orthographic",
    ],
    keywords: &[
        "orthographic",
        "globe",
        "view from space",
        "projection",
        "9840",
    ],
    inputs: &[LAT, LON, CENTER_LAT, CENTER_LON, FE, FN, E[0], E[1], E[2]],
    outputs: FORWARD_OUT,
    errors: &[ErrorCode::OutOfDomain, ErrorCode::Unsupported],
    stability: Stability::Stable,
    when_to_use: "Use this to draw the Earth as a globe seen from space, or to place a point on a satellite-view or perspective map centered where you choose. Each point drops straight down onto the flat plane touching the ellipsoid at the center, so the near side keeps its familiar look while its edges foreshorten.",
    limitations: "Only the half of the Earth facing the viewer has an image: a point whose vertical is 90 degrees or more from the center's is on the far side and is refused. Toward that rim the scale along the direction from the center falls to zero, so shapes flatten and distances there mean little. It keeps neither areas, shapes, nor distances, and suits pictures more than measurement. This is EPSG's ellipsoidal method, not the sphere.",
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED"],
    model: "Orthographic (EPSG method 9840), ellipsoidal",
    accuracy: "Exact to double precision; agrees with PROJ within 1 mm",
    references: &[G7_2],
    examples: &[Example {
        id: "primary",
        title: "Central Europe from above the North Sea",
        input: r#"{"lat":50,"lon":9,"latitude_of_origin":55,"longitude_of_origin":5}"#,
        source: "PROJ 9.9 +proj=ortho +lat_0=55 +lon_0=5 +ellps=WGS84: E = 286,550.114 m, N = -547,480.621 m",
    }],
    primary_example: "primary",
    visualization: FORWARD_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.orthographic-inverse",
            reason: "inverse",
        },
        Related {
            id: "geodesy.projection.gnomonic-forward",
            reason: "alternative",
        },
        Related {
            id: "geodesy.projection.azimuthal-equidistant-forward",
            reason: "alternative",
        },
    ],
    sentence: FORWARD_SENTENCE,
    limits: &[("batchRows", 10_000)],
    run: run_ortho_forward,
    assumptions: &gp_base::tool::WGS84_ASSUMPTIONS,
    ..ToolDef::BLANK
};

fn run_ortho_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let p = orthographic(ctx)?;
    match p.forward(lat, lon) {
        Some(g) => forward_json(ctx, g),
        None => Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "This point is on the far side of the Earth from the center, which the orthographic does not show.",
        )
        .at("/lat")),
    }
}

pub static ORTHO_INVERSE: ToolDef = ToolDef {
    id: "geodesy.projection.orthographic-inverse",
    title: "Orthographic to latitude and longitude",
    summary: "Converts an easting and northing on the ellipsoidal Orthographic you center back to latitude and longitude on the near side.",
    aliases: &["orthographic to lat long", "orthographic inverse"],
    keywords: &[
        "orthographic inverse",
        "globe",
        "inverse",
        "projection",
        "9840",
    ],
    inputs: &[
        EASTING, NORTHING, CENTER_LAT, CENTER_LON, FE, FN, E[0], E[1], E[2],
    ],
    outputs: INVERSE_OUT,
    errors: &[ErrorCode::OutOfDomain, ErrorCode::Unsupported],
    stability: Stability::Stable,
    when_to_use: "Use this to read a latitude and longitude off an orthographic picture of the globe: a point on a satellite-style view or a perspective map with a known center, back to the ground it shows.",
    limitations: "Each spot inside the disk the near side covers comes back to the one point on the near side that lands there; the far side, hidden behind it, is never returned. A spot outside that disk, or on its rim where the picture folds, is refused. The point is found by Newton's method from the sphere's answer, to about a nanometer. The center must be the picture's own.",
    warnings: &["UNIT_ASSUMED"],
    model: "Orthographic (EPSG method 9840), ellipsoidal, inverted by Newton's method",
    accuracy: "Agrees with PROJ within 1 mm; forward and back return the point within a nanometer",
    references: &[G7_2],
    examples: &[Example {
        id: "primary",
        title: "Back to Central Europe",
        input: r#"{"easting":"286550.1136 m","northing":"-547480.6206 m","latitude_of_origin":55,"longitude_of_origin":5}"#,
        source: "PROJ 9.9 +proj=ortho +lat_0=55 +lon_0=5 +ellps=WGS84, inverse: 50° N, 9° E",
    }],
    primary_example: "primary",
    visualization: INVERSE_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.orthographic-forward",
            reason: "inverse",
        },
        Related {
            id: "geodesy.projection.gnomonic-inverse",
            reason: "alternative",
        },
        Related {
            id: "geodesy.parse.format",
            reason: "next",
        },
    ],
    sentence: INVERSE_SENTENCE,
    limits: &[("batchRows", 10_000)],
    run: run_ortho_inverse,
    assumptions: &gp_base::tool::WGS84_ASSUMPTIONS,
    ..ToolDef::BLANK
};

fn run_ortho_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (x, y) = grid_in(ctx)?;
    let p = orthographic(ctx)?;
    match p.inverse(x, y) {
        Some(ll) => inverse_json(ctx, ll),
        None => Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "This easting and northing are outside the disk the near side of the Earth covers, or on its rim.",
        )
        .at("/easting")),
    }
}

// ------------------------------------------------------------ Hotine Oblique Mercator

const HOTINE_VARIANT: Field = Field::new(
    "variant",
    "Variant",
    "B (the default): the false easting and northing are at the projection center; A: they are at the natural origin",
    Kind::Choice(&["B", "A"]),
);
const HOTINE_LATC: Field = angle(
    "latitude_of_center",
    "Latitude of the projection center",
    "Like 4",
    "[-90,90]",
)
.required()
.core();
const HOTINE_LONC: Field = angle(
    "longitude_of_center",
    "Longitude of the projection center",
    "Like 115",
    "[-180,180)",
)
.required()
.core();
const HOTINE_ALPHA: Field = angle(
    "azimuth",
    "Azimuth of the initial line",
    "The central line's direction at the center, clockwise from north, like 53°18'56.9537\"",
    "[-360,360]",
)
.required()
.core();
const HOTINE_GAMMA: Field = angle(
    "rectified_grid_angle",
    "Angle from the rectified to the skew grid",
    "Like 53°07'48.3685\"; the azimuth if not given",
    "[-360,360]",
);
const HOTINE_K: Field = Field::new(
    "scale_factor",
    "Scale factor on the initial line",
    "Like 0.99984; 1 if not given",
    Kind::Number {
        min: 0.1,
        max: 10.0,
    },
);
const HOTINE_FE: Field = length(
    "false_easting",
    "Easting at the center (B) or false easting (A)",
    "Like 590476.87 m; 0 if not given",
);
const HOTINE_FN: Field = length(
    "false_northing",
    "Northing at the center (B) or false northing (A)",
    "Like 442857.65 m; 0 if not given",
);

fn hotine(ctx: &mut Ctx) -> Result<Hotine, ToolError> {
    let e = ellipsoid(ctx)?;
    let (latc, lonc) = point::read(ctx, "latitude_of_center", "longitude_of_center")?;
    if latc.abs() >= 90.0 {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The projection center can not be a pole: the initial line needs a direction there.",
        )
        .at("/latitude_of_center"));
    }
    let alpha = opt_angle(ctx, "azimuth")?.ok_or_else(|| {
        ToolError::invalid("/azimuth", "The azimuth of the initial line is required.")
    })?;
    let gamma = opt_angle(ctx, "rectified_grid_angle")?.unwrap_or(alpha);
    let k0 = ctx.number("scale_factor")?.unwrap_or(1.0);
    let (fe, fn_) = (
        opt_len(ctx, "false_easting")?,
        opt_len(ctx, "false_northing")?,
    );
    let b = ctx.choice("variant")? != Some("A");
    Ok(Hotine::new(
        e.a, e.f, latc, lonc, alpha, gamma, k0, fe, fn_, b,
    ))
}

pub static HOTINE_FORWARD: ToolDef = ToolDef {
    id: "geodesy.projection.hotine-forward",
    title: "Latitude and longitude to Hotine Oblique Mercator",
    summary: "Projects a latitude and longitude with a Hotine Oblique Mercator you define, a Mercator whose central line runs at any angle, for regions that lie along a slanted band.",
    aliases: &[
        "oblique Mercator calculator",
        "Hotine projection",
        "lat long to oblique Mercator",
    ],
    keywords: &[
        "Hotine",
        "oblique Mercator",
        "rectified skew orthomorphic",
        "projection",
        "9815",
    ],
    inputs: &[
        LAT,
        LON,
        HOTINE_VARIANT,
        HOTINE_LATC,
        HOTINE_LONC,
        HOTINE_ALPHA,
        HOTINE_GAMMA,
        HOTINE_K,
        HOTINE_FE,
        HOTINE_FN,
        E[0],
        E[1],
        E[2],
    ],
    outputs: FORWARD_OUT,
    errors: &[ErrorCode::OutOfDomain, ErrorCode::Unsupported],
    stability: Stability::Stable,
    when_to_use: "Use this for grids built on a slanted central line: the Alaska panhandle's state plane zone, the rectified skew orthomorphic grids of Malaysia and Borneo, the Swiss and Hungarian national grids in their Hotine form, and corridor projections for pipelines or railways that run diagonally. Give the center, the azimuth of the line, and the grid's other parameters.",
    limitations: "It keeps shapes and is true to scale times the scale factor along the central line, but the scale grows away from it, so it suits a band a few hundred kilometers wide. Far from the line, near the two poles of the oblique projection, points run off to infinity and are refused. Variant A and variant B differ only in where the falsings sit, so choosing the wrong one shifts every point by a constant amount. The convergence and scales come from differences of the map, good to about 1e-9.",
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED"],
    model: "Hotine Oblique Mercator variants A and B (EPSG methods 9812 and 9815)",
    accuracy: "Exact to double precision; agrees with PROJ within 1 mm",
    references: &[G7_2],
    examples: &[Example {
        id: "primary",
        title: "The IOGP example: Timbalai 1948 / RSO Borneo",
        input: r#"{"lat":"5°23'14.1129\"N","lon":"115°48'19.8196\"E","latitude_of_center":4,"longitude_of_center":115,"azimuth":"53°18'56.9537\"","rectified_grid_angle":"53°07'48.3685\"","scale_factor":0.99984,"false_easting":"590476.87 m","false_northing":"442857.65 m","a":"6377298.556 m","inverse_flattening":300.8017}"#,
        source: "IOGP Guidance Note 7-2, 3.2.4 example (variant B): E = 679,245.73 m, N = 596,562.78 m",
    }],
    primary_example: "primary",
    visualization: FORWARD_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.hotine-inverse",
            reason: "inverse",
        },
        Related {
            id: "geodesy.spcs.spcs83-forward",
            reason: "alternative",
        },
        Related {
            id: "geodesy.projection.lcc-forward",
            reason: "alternative",
        },
    ],
    sentence: FORWARD_SENTENCE,
    limits: &[("batchRows", 10_000)],
    run: run_hotine_forward,
    assumptions: &gp_base::tool::WGS84_ASSUMPTIONS,
    ..ToolDef::BLANK
};

fn run_hotine_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let p = hotine(ctx)?;
    forward_json(ctx, p.forward(lat, lon))
}

pub static HOTINE_INVERSE: ToolDef = ToolDef {
    id: "geodesy.projection.hotine-inverse",
    title: "Hotine Oblique Mercator to latitude and longitude",
    summary: "Converts an easting and northing on a Hotine Oblique Mercator you define back to latitude and longitude.",
    aliases: &["oblique Mercator to lat long", "Hotine inverse"],
    keywords: &[
        "Hotine inverse",
        "oblique Mercator inverse",
        "inverse",
        "projection",
        "9815",
    ],
    inputs: &[
        EASTING,
        NORTHING,
        HOTINE_VARIANT,
        HOTINE_LATC,
        HOTINE_LONC,
        HOTINE_ALPHA,
        HOTINE_GAMMA,
        HOTINE_K,
        HOTINE_FE,
        HOTINE_FN,
        E[0],
        E[1],
        E[2],
    ],
    outputs: INVERSE_OUT,
    errors: &[ErrorCode::OutOfDomain, ErrorCode::Unsupported],
    stability: Stability::Stable,
    when_to_use: "Use this to turn an easting and northing on a Hotine or rectified skew orthomorphic grid back into latitude and longitude: a coordinate from the Alaska panhandle zone, a Malaysian or Borneo grid, or a corridor projection, when you have the grid's parameters.",
    limitations: "The parameters must be the grid's own and in its units, and the variant must match how the grid places its falsings: the same numbers read with the other variant land a fixed distance away. The latitude is solved exactly rather than by the guidance note's truncated series. The result is on the ellipsoid you choose, which should be the grid's.",
    warnings: &["UNIT_ASSUMED"],
    model: "Hotine Oblique Mercator variants A and B (EPSG methods 9812 and 9815)",
    accuracy: "Exact to double precision; latitude solved by iteration to 1e-14 radians",
    references: &[G7_2],
    examples: &[Example {
        id: "primary",
        title: "Back to the IOGP example",
        input: r#"{"easting":"679245.73 m","northing":"596562.78 m","latitude_of_center":4,"longitude_of_center":115,"azimuth":"53°18'56.9537\"","rectified_grid_angle":"53°07'48.3685\"","scale_factor":0.99984,"false_easting":"590476.87 m","false_northing":"442857.65 m","a":"6377298.556 m","inverse_flattening":300.8017}"#,
        source: "IOGP Guidance Note 7-2, 3.2.4 example reversed: 5°23'14.1129\" N, 115°48'19.8196\" E",
    }],
    primary_example: "primary",
    visualization: INVERSE_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.hotine-forward",
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
    run: run_hotine_inverse,
    assumptions: &gp_base::tool::WGS84_ASSUMPTIONS,
    ..ToolDef::BLANK
};

fn run_hotine_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (x, y) = grid_in(ctx)?;
    let p = hotine(ctx)?;
    let (lat, lon) = p.inverse(x, y);
    inverse_json(ctx, (lat, proj::dlon(lon, 0.0)))
}

// ------------------------------------------------------------ Transverse Mercator

const TM_LON0: Field = angle(
    "longitude_of_origin",
    "Central meridian",
    "The longitude of the central meridian, like -2",
    "[-180,180)",
)
.required()
.core();
const TM_LAT0: Field = angle(
    "latitude_of_origin",
    "Latitude of origin",
    "The latitude the northing counts from, like 49; 0 if not given",
    "[-90,90]",
)
.core();
const TM_K0: Field = Field::new(
    "scale_factor",
    "Scale factor on the central meridian",
    "Like 0.9996012717; 1 if not given",
    Kind::Number {
        min: 0.1,
        max: 10.0,
    },
)
.core();

fn tm_grid(ctx: &mut Ctx) -> Result<(TmGrid, f64), ToolError> {
    let e = ellipsoid(ctx)?;
    e.geodesic()?;
    let lon0 = opt_angle(ctx, "longitude_of_origin")?.ok_or_else(|| {
        ToolError::invalid("/longitude_of_origin", "The central meridian is required.")
    })?;
    let lat0 = opt_lat(ctx, "latitude_of_origin")?.unwrap_or(0.0);
    let k0 = ctx.number("scale_factor")?.unwrap_or(1.0);
    let (fe, fn_) = (
        opt_len(ctx, "false_easting")?,
        opt_len(ctx, "false_northing")?,
    );
    // The series' reach scales with the ellipsoid.
    let reach = proj::TM_SERIES_REACH * e.a / proj::WGS84_A;
    Ok((TmGrid::new(e.a, e.f, lat0, lon0, k0, fe, fn_), reach))
}

fn tm_reach_warning(ctx: &mut Ctx, g: &TmGrid, e: f64, reach: f64, at: &str) {
    let off = g.offset(e);
    if off > reach {
        ctx.warnings.push(
            Warning::new(
                "ACCURACY_DEGRADED",
                format!(
                    "This point is {:.0} km from the central meridian; the series is good to 5 nm within {:.0} km and loses accuracy beyond (meters by 70 degrees of longitude).",
                    off / 1000.0,
                    reach / 1000.0
                ),
            )
            .at(at),
        );
    }
}

pub static TM_FORWARD: ToolDef = ToolDef {
    id: "geodesy.projection.tm-forward",
    title: "Latitude and longitude to Transverse Mercator",
    summary: "Projects a latitude and longitude with a Transverse Mercator you define: central meridian, latitude of origin, scale factor, and falsings, on any ellipsoid, to nanometers near the central meridian.",
    aliases: &[
        "transverse Mercator calculator",
        "Gauss-Krüger",
        "lat long to transverse Mercator",
    ],
    keywords: &[
        "transverse Mercator",
        "Gauss-Krüger",
        "central meridian",
        "projection",
        "9807",
        "British National Grid",
    ],
    inputs: &[LAT, LON, TM_LON0, TM_LAT0, TM_K0, FE, FN, E[0], E[1], E[2]],
    outputs: FORWARD_OUT,
    errors: &[ErrorCode::OutOfDomain, ErrorCode::Unsupported],
    stability: Stability::Stable,
    when_to_use: "Use this for any transverse Mercator grid that is not UTM: national grids like the British National Grid, Gauss-Krüger zones, state plane and county grids built on it, or a local low-distortion projection. Give the grid's central meridian, latitude of origin, scale factor, and falsings, and get the easting and northing with the convergence and scale.",
    limitations: "It uses Krüger's series to sixth order, as UTM does: good to 5 nanometers within 3,900 km of the central meridian, and warned beyond, where the error grows to meters. The projection itself suits a band a few hundred kilometers wide, since its scale grows with the square of the distance from the central meridian. Parameters must be the grid's own, and the result is on the ellipsoid you choose; a datum shift is a separate step.",
    warnings: &["ACCURACY_DEGRADED", "INPUT_NORMALIZED", "UNIT_ASSUMED"],
    model: "Transverse Mercator (EPSG method 9807) by Krüger's series to sixth order (Karney 2011)",
    accuracy: "5 nm within 3,900 km of the central meridian; agrees with GeographicLib's series to a nanometer",
    references: &[G7_2, crate::KARNEY_TM],
    examples: &[Example {
        id: "primary",
        title: "The IOGP example: British National Grid",
        input: r#"{"lat":"50°30'N","lon":"0°30'E","longitude_of_origin":-2,"latitude_of_origin":49,"scale_factor":0.9996012717,"false_easting":"400000 m","false_northing":"-100000 m","ellipsoid":"airy1830"}"#,
        source: "IOGP Guidance Note 7-2, 3.2.5 example (OSGB 1936 / British National Grid): E = 577,274.99 m, N = 69,740.50 m",
    }],
    primary_example: "primary",
    visualization: FORWARD_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.tm-inverse",
            reason: "inverse",
        },
        Related {
            id: "geodesy.utm.forward",
            reason: "alternative",
        },
        Related {
            id: "geodesy.spcs.spcs83-forward",
            reason: "alternative",
        },
    ],
    sentence: FORWARD_SENTENCE,
    limits: &[("batchRows", 10_000)],
    run: run_tm_forward,
    assumptions: &gp_base::tool::WGS84_ASSUMPTIONS,
    ..ToolDef::BLANK
};

fn run_tm_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let (p, reach) = tm_grid(ctx)?;
    let g = p.forward(lat, lon);
    tm_reach_warning(ctx, &p, g.e, reach, "/lon");
    forward_json(ctx, g)
}

pub static TM_INVERSE: ToolDef = ToolDef {
    id: "geodesy.projection.tm-inverse",
    title: "Transverse Mercator to latitude and longitude",
    summary: "Converts an easting and northing on a Transverse Mercator you define back to latitude and longitude.",
    aliases: &["transverse Mercator to lat long", "Gauss-Krüger inverse"],
    keywords: &[
        "transverse Mercator inverse",
        "Gauss-Krüger inverse",
        "inverse",
        "projection",
        "9807",
    ],
    inputs: &[
        EASTING, NORTHING, TM_LON0, TM_LAT0, TM_K0, FE, FN, E[0], E[1], E[2],
    ],
    outputs: INVERSE_OUT,
    errors: &[ErrorCode::OutOfDomain, ErrorCode::Unsupported],
    stability: Stability::Stable,
    when_to_use: "Use this to turn an easting and northing on a transverse Mercator grid other than UTM back into latitude and longitude: a British National Grid reference in meters, a Gauss-Krüger coordinate, or a local low-distortion grid, when you have its parameters. For UTM, the UTM tool already knows every zone's.",
    limitations: "The parameters must be the grid's own and in its units; nothing in the numbers says which grid they came from. Krüger's series is good to 5 nanometers within 3,900 km of the central meridian and is warned beyond. The latitude and longitude are on the ellipsoid you choose, which should be the grid's.",
    warnings: &["ACCURACY_DEGRADED", "UNIT_ASSUMED"],
    model: "Transverse Mercator (EPSG method 9807) by Krüger's series to sixth order (Karney 2011)",
    accuracy: "5 nm within 3,900 km of the central meridian; agrees with GeographicLib's series to a nanometer",
    references: &[G7_2, crate::KARNEY_TM],
    examples: &[Example {
        id: "primary",
        title: "Back to the IOGP example",
        input: r#"{"easting":"577274.99 m","northing":"69740.50 m","longitude_of_origin":-2,"latitude_of_origin":49,"scale_factor":0.9996012717,"false_easting":"400000 m","false_northing":"-100000 m","ellipsoid":"airy1830"}"#,
        source: "IOGP Guidance Note 7-2, 3.2.5 example reversed: 50°30' N, 0°30' E",
    }],
    primary_example: "primary",
    visualization: INVERSE_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.tm-forward",
            reason: "inverse",
        },
        Related {
            id: "geodesy.utm.inverse",
            reason: "alternative",
        },
        Related {
            id: "geodesy.parse.format",
            reason: "next",
        },
    ],
    sentence: INVERSE_SENTENCE,
    limits: &[("batchRows", 10_000)],
    run: run_tm_inverse,
    assumptions: &gp_base::tool::WGS84_ASSUMPTIONS,
    ..ToolDef::BLANK
};

fn run_tm_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (x, y) = grid_in(ctx)?;
    let (p, reach) = tm_grid(ctx)?;
    tm_reach_warning(ctx, &p, x, reach, "/easting");
    let (lat, lon) = p.inverse(x, y);
    inverse_json(ctx, (lat, proj::dlon(lon, 0.0)))
}

// ------------------------------------------------------------ Exact transverse Mercator

fn tm_exact_grid(ctx: &mut Ctx) -> Result<TmExactGrid, ToolError> {
    let e = ellipsoid(ctx)?;
    if e.f <= 0.0 || e.f > 0.1 {
        return Err(ToolError::new(
            ErrorCode::Unsupported,
            "The exact transverse Mercator needs an oblate ellipsoid (flattening above 0 and at most 0.1); for a sphere, use the series tool.",
        )
        .at("/inverse_flattening"));
    }
    let lon0 = opt_angle(ctx, "longitude_of_origin")?.ok_or_else(|| {
        ToolError::invalid("/longitude_of_origin", "The central meridian is required.")
    })?;
    let lat0 = opt_lat(ctx, "latitude_of_origin")?.unwrap_or(0.0);
    let k0 = ctx.number("scale_factor")?.unwrap_or(1.0);
    let (fe, fn_) = (
        opt_len(ctx, "false_easting")?,
        opt_len(ctx, "false_northing")?,
    );
    Ok(TmExactGrid::new(e.a, e.f, lat0, lon0, k0, fe, fn_))
}

pub static TM_EXACT_FORWARD: ToolDef = ToolDef {
    id: "geodesy.projection.tm-exact-forward",
    title: "Latitude and longitude to exact Transverse Mercator",
    summary: "Projects a latitude and longitude with the exact transverse Mercator, good to nanometers anywhere on the ellipsoid, not just near the central meridian.",
    aliases: &[
        "exact transverse Mercator",
        "Lee transverse Mercator",
        "transverse Mercator far from the central meridian",
    ],
    keywords: &[
        "transverse Mercator exact",
        "exact",
        "elliptic functions",
        "projection",
        "9807",
    ],
    inputs: &[LAT, LON, TM_LON0, TM_LAT0, TM_K0, FE, FN, E[0], E[1], E[2]],
    outputs: FORWARD_OUT,
    errors: &[ErrorCode::OutOfDomain, ErrorCode::Unsupported],
    stability: Stability::Stable,
    when_to_use: "Use this when a transverse Mercator must hold far from its central meridian: a grid stretched well beyond its zone, a whole-hemisphere map, or a check on the series another program uses. It is the same projection as the series tool, computed with elliptic functions instead of a truncated series, so it stays exact where the series drifts by meters.",
    limitations: "It is slower than the series, which is already exact to 5 nanometers within 3,900 km of the central meridian, so for ordinary grids the series tool is the better choice. On the equator, 90(1 − e) degrees from the central meridian (82.636 degrees on WGS 84), the projection has a branch point where the map folds and stops being conformal, so near it a small move on the ground can be a large one on the grid. Points more than 90 degrees from the central meridian fold onto the far side of the grid, as the standard domain does.",
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED"],
    model: "Exact transverse Mercator (Lee 1976) by Jacobi elliptic functions, ported from GeographicLib's TransverseMercatorExact",
    accuracy: "Agrees with GeographicLib's TransverseMercatorProj within 0.1 µm anywhere on the ellipsoid",
    references: &[crate::KARNEY_TM, G7_2],
    examples: &[Example {
        id: "primary",
        title: "75 degrees from the central meridian",
        input: r#"{"lat":30,"lon":75,"longitude_of_origin":0,"scale_factor":0.9996}"#,
        source: "GeographicLib 2.7 TransverseMercatorProj -l 0 -k 0.9996 (exact), an independent implementation of the same method: E = 7,707,953.714 m, N = 7,322,160.470 m",
    }],
    primary_example: "primary",
    visualization: FORWARD_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.tm-exact-inverse",
            reason: "inverse",
        },
        Related {
            id: "geodesy.projection.tm-forward",
            reason: "alternative",
        },
        Related {
            id: "geodesy.utm.forward",
            reason: "alternative",
        },
    ],
    sentence: FORWARD_SENTENCE,
    limits: &[("batchRows", 10_000)],
    run: run_tm_exact_forward,
    assumptions: &gp_base::tool::WGS84_ASSUMPTIONS,
    ..ToolDef::BLANK
};

fn run_tm_exact_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let p = tm_exact_grid(ctx)?;
    let (e, n, gamma, k) = p.forward(lat, lon);
    forward_json(
        ctx,
        Grid {
            e,
            n,
            convergence: gamma,
            h: k,
            k,
        },
    )
}

pub static TM_EXACT_INVERSE: ToolDef = ToolDef {
    id: "geodesy.projection.tm-exact-inverse",
    title: "Exact Transverse Mercator to latitude and longitude",
    summary: "Converts an easting and northing on the exact transverse Mercator back to latitude and longitude, to nanometers anywhere on the ellipsoid.",
    aliases: &[
        "exact transverse Mercator to lat long",
        "exact transverse Mercator inverse",
    ],
    keywords: &[
        "transverse Mercator exact inverse",
        "exact inverse",
        "inverse",
        "projection",
        "9807",
    ],
    inputs: &[
        EASTING, NORTHING, TM_LON0, TM_LAT0, TM_K0, FE, FN, E[0], E[1], E[2],
    ],
    outputs: INVERSE_OUT,
    errors: &[ErrorCode::OutOfDomain, ErrorCode::Unsupported],
    stability: Stability::Stable,
    when_to_use: "Use this to turn a transverse Mercator easting and northing back into latitude and longitude when the point may lie far from the central meridian, where the series loses accuracy, or when a result must be checked against an exact method.",
    limitations: "The parameters must be the grid's own and in its units. Near the branch point on the equator, 90(1 − e) degrees from the central meridian, where the map folds, the latitude and longitude from an easting and northing are less certain. The latitude and longitude are on the ellipsoid you choose, which should be the grid's; a sphere is not supported, since the series tool is exact for it.",
    warnings: &["UNIT_ASSUMED"],
    model: "Exact transverse Mercator (Lee 1976) by Jacobi elliptic functions, inverted by Newton's method as in GeographicLib",
    accuracy: "Agrees with GeographicLib's TransverseMercatorProj within 0.1 µm; forward and back return the point to about 1e-13 degrees",
    references: &[crate::KARNEY_TM, G7_2],
    examples: &[Example {
        id: "primary",
        title: "Back from 75 degrees out",
        input: r#"{"easting":"7707953.714163 m","northing":"7322160.469546 m","longitude_of_origin":0,"scale_factor":0.9996}"#,
        source: "GeographicLib 2.7 TransverseMercatorProj -r -l 0 -k 0.9996 (exact): 30° N, 75° E",
    }],
    primary_example: "primary",
    visualization: INVERSE_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.tm-exact-forward",
            reason: "inverse",
        },
        Related {
            id: "geodesy.projection.tm-inverse",
            reason: "alternative",
        },
        Related {
            id: "geodesy.parse.format",
            reason: "next",
        },
    ],
    sentence: INVERSE_SENTENCE,
    limits: &[("batchRows", 10_000)],
    run: run_tm_exact_inverse,
    assumptions: &gp_base::tool::WGS84_ASSUMPTIONS,
    ..ToolDef::BLANK
};

fn run_tm_exact_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (x, y) = grid_in(ctx)?;
    let p = tm_exact_grid(ctx)?;
    inverse_json(ctx, p.inverse(x, y))
}

// ------------------------------------------------------------------- LV95

const SWISSTOPO: Reference = Reference {
    title: "Formulas and constants for the calculation of the Swiss conformal cylindrical projection and for the transformation between coordinate systems",
    issuer: "Federal Office of Topography swisstopo",
    year: 2016,
    edition: "December 2016 edition",
    locator: "Section 3.1 (constants and auxiliary values), 3.2 and 3.3 (the rigorous formulas each way), with the worked example for the station Rigi: 47°03′28.95659233″N 8°29′11.11127154″E on Bessel gives E 2,679,520.05 N 1,212,273.44",
    url: "https://www.swisstopo.admin.ch/dam/en/sd-web/c46Fz-MHIc3u/refsys-EN.pdf",
};
const LV95_NOTE: &str = "The latitude and longitude are on CH1903 (the Bessel ellipsoid), which is what the projection is defined for; a satellite position on WGS 84 is a datum step away.";

pub static LV95_FORWARD: ToolDef = ToolDef {
    id: "geodesy.projection.lv95-forward",
    stability: gp_base::tool::Stability::Stable,
    title: "Latitude and longitude to Swiss LV95",
    summary: "Converts a CH1903 latitude and longitude to LV95 easting and northing, the Swiss national grid, by swisstopo's rigorous formulas.",
    aliases: &[
        "lat long to LV95",
        "Swiss grid converter",
        "CH1903+ coordinates",
        "LV95 from latitude longitude",
    ],
    keywords: &[
        "LV95",
        "CH1903",
        "Swiss grid",
        "swisstopo",
        "oblique Mercator",
        "Switzerland",
        "projection",
        "EPSG 2056",
    ],
    inputs: &[LAT, LON],
    outputs: &[
        Field::new(
            "easting",
            "Easting",
            "LV95, counting from its false origin",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "northing",
            "Northing",
            "LV95, counting from its false origin",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "lv03_easting",
            "LV03 easting",
            "The old grid: the same point, 2,000,000 m less",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "lv03_northing",
            "LV03 northing",
            "The old grid: the same point, 1,000,000 m less",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "datum",
            "Datum of the latitude and longitude",
            "What the projection was given",
            Kind::Text { max_len: 200 },
        ),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &["REALIZATION_ASSUMED"],
    model: "The Swiss conformal cylindrical projection, swisstopo's rigorous formulas: the Bessel ellipsoid onto a sphere of radius 6,378,815.90365 m, a rotation carrying the fundamental point in Bern (46°57′08.66″N 7°26′22.50″E) onto that sphere's equator, then a Mercator of the rotated sphere, with LV95's false origin of 2,600,000 m east and 1,200,000 m north",
    accuracy: "The rigorous formulas, not the approximate ones: swisstopo's own worked example for the station Rigi comes back to the centimetre they print, and the projection and its inverse agree to a nanodegree over the country",
    when_to_use: "Use this to put a latitude and longitude onto the Swiss national grid, the coordinates every Swiss map, cadastral plan, and address file is written in. It gives the LV03 coordinates beside the LV95 ones, since the two differ by exactly two million east and one million north and plenty of records are still in the old grid.",
    limitations: "The projection is defined for CH1903 coordinates, on the Bessel ellipsoid: a satellite position on WGS 84 or ETRS89 is a datum step away, which is separate from this tool and which swisstopo publishes a grid for. The formulas hold for Switzerland and its surroundings; far outside, the rotation that makes the projection conformal there stops meaning anything. Heights are untouched: swisstopo's height transformation is its own step.",
    references: &[SWISSTOPO],
    examples: &[Example {
        id: "primary",
        title: "The station Rigi, swisstopo's own example",
        input: r#"{"lat":"47°03'28.95659233\"N","lon":"8°29'11.11127154\"E"}"#,
        source: "swisstopo, Formulas and constants (December 2016), section 3.2: E = 2,679,520.05 N = 1,212,273.44",
    }],
    primary_example: "primary",
    visualization: FORWARD_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.lv95-inverse",
            reason: "inverse",
        },
        Related {
            id: "geodesy.projection.hotine-forward",
            reason: "alternative",
        },
        Related {
            id: "geodesy.datum.legacy",
            reason: "next",
        },
    ],
    sentence: "That point is E {easting}, N {northing} on LV95.",
    limits: &[("batchRows", 10_000)],
    run: run_lv95_forward,
    ..ToolDef::BLANK
};

fn run_lv95_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let (e, n) = gp_geo::proj::swiss::forward(lat.to_radians(), lon.to_radians());
    if !(e.is_finite() && n.is_finite()) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "That point is too far from Switzerland for the Swiss projection to place.",
        )
        .at("/lat"));
    }
    ctx.warnings
        .push(Warning::new("REALIZATION_ASSUMED", LV95_NOTE));
    if ctx.explaining() {
        ctx.step(
            "Onto the projection sphere",
            "the Bessel ellipsoid mapped conformally onto a sphere of 6,378,815.90365 m",
            format!("{lat:.9}°, {lon:.9}°"),
            "a spherical latitude and longitude".to_string(),
        );
        ctx.step(
            "Turned onto the equator",
            "the fundamental point in Bern carried onto the sphere's equator",
            "46°57′08.66″N 7°26′22.50″E".to_string(),
            "the pseudo-equator system".to_string(),
        );
        ctx.step(
            "Mercator, with the false origin",
            "E = R·l̄ + 2,600,000; N = (R/2)·ln((1 + sin b̄)/(1 − sin b̄)) + 1,200,000",
            format!("{lat:.6}°, {lon:.6}°"),
            format!(
                "{} m",
                gp_base::display::number(e, Precision::Decimals(3), ctx.options.format)
            ),
        );
    }
    Ok(Json::obj([
        ("easting", ctx.out("easting", meters(e))),
        ("northing", ctx.out("northing", meters(n))),
        (
            "lv03_easting",
            ctx.out("lv03_easting", meters(e - 2_000_000.0)),
        ),
        (
            "lv03_northing",
            ctx.out("lv03_northing", meters(n - 1_000_000.0)),
        ),
        ("datum", Json::str(LV95_NOTE)),
    ]))
}

pub static LV95_INVERSE: ToolDef = ToolDef {
    id: "geodesy.projection.lv95-inverse",
    stability: gp_base::tool::Stability::Stable,
    title: "Swiss LV95 to latitude and longitude",
    summary: "Converts LV95 easting and northing back to a CH1903 latitude and longitude by swisstopo's rigorous formulas, taking LV03 coordinates too.",
    aliases: &[
        "LV95 to lat long",
        "Swiss grid to coordinates",
        "LV03 to latitude longitude",
        "read Swiss coordinates",
    ],
    keywords: &[
        "LV95",
        "LV03",
        "CH1903",
        "Swiss grid",
        "swisstopo",
        "Switzerland",
        "inverse",
        "EPSG 2056",
    ],
    inputs: &[EASTING, NORTHING],
    outputs: &[
        Field::new(
            "lat",
            "Latitude",
            "On CH1903, north positive",
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
            "On CH1903, east positive",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(9))
        .angle_range("[-180,180)"),
        Field::new(
            "grid",
            "Grid read",
            "Whether the coordinates were taken as LV95 or LV03",
            Kind::Text { max_len: 120 },
        ),
        Field::new(
            "datum",
            "Datum of the answer",
            "What the latitude and longitude are on",
            Kind::Text { max_len: 200 },
        ),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &["REALIZATION_ASSUMED", "INPUT_NORMALIZED"],
    model: "The inverse of the Swiss conformal cylindrical projection, swisstopo's rigorous formulas: Mercator back to the rotated sphere, the rotation undone, and the latitude found by the iteration swisstopo gives, which settles in six steps. An easting under 2,000,000 m is read as LV03 and 2,000,000 m east and 1,000,000 m north are added first",
    accuracy: "The rigorous formulas: swisstopo's own worked example for the station Rigi comes back to the digits they print, and the projection and its inverse agree to a nanodegree over the country",
    when_to_use: "Use this to read Swiss coordinates off a map, a cadastral extract, or an address file and get the latitude and longitude. It takes LV95 and the older LV03 both: a six-figure easting is read as LV03 and shifted, and the result says which grid it read.",
    limitations: "The answer is on CH1903, the Bessel ellipsoid, which is what the projection is defined for; a position on WGS 84 or ETRS89 is a datum step away, which swisstopo publishes a grid for and which is not part of this tool. Heights are untouched. Coordinates far outside Switzerland are arithmetic rather than a national grid.",
    references: &[SWISSTOPO],
    examples: &[Example {
        id: "primary",
        title: "The station Rigi, swisstopo's own example",
        input: r#"{"easting":"2679520.05 m","northing":"1212273.44 m"}"#,
        source: "swisstopo, Formulas and constants (December 2016), section 3.3: 47°03′28.956592″N 8°29′11.111272″E on CH1903",
    }],
    primary_example: "primary",
    visualization: INVERSE_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.lv95-forward",
            reason: "inverse",
        },
        Related {
            id: "geodesy.projection.hotine-inverse",
            reason: "alternative",
        },
        Related {
            id: "geodesy.datum.legacy",
            reason: "next",
        },
    ],
    sentence: "That is {lat}, {lon} on CH1903.",
    limits: &[("batchRows", 10_000)],
    run: run_lv95_inverse,
    ..ToolDef::BLANK
};

fn run_lv95_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (mut e, mut n) = grid_in(ctx)?;
    // LV03 runs from 600,000 and 200,000; LV95 from 2,600,000 and 1,200,000.
    let lv03 = e < 2_000_000.0;
    let grid = if lv03 {
        ctx.warnings.push(Warning::new(
            "INPUT_NORMALIZED",
            "The easting is under 2,000,000 m, so the coordinates were read as LV03 and shifted onto LV95 first.",
        ));
        e += 2_000_000.0;
        n += 1_000_000.0;
        "read as LV03 and shifted onto LV95"
    } else {
        "read as LV95"
    };
    let (phi, lam) = gp_geo::proj::swiss::inverse(e, n);
    let (lat, lon) = (phi.to_degrees(), lam.to_degrees());
    if !(lat.is_finite() && lon.is_finite() && (-90.0..=90.0).contains(&lat)) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "That easting and northing is not a place the Swiss projection can read.",
        )
        .at("/easting"));
    }
    ctx.warnings
        .push(Warning::new("REALIZATION_ASSUMED", LV95_NOTE));
    if ctx.explaining() {
        ctx.step(
            "Off the false origin",
            "the grid counts 2,600,000 m east and 1,200,000 m north of its true origin",
            format!("E {e:.3} m, N {n:.3} m"),
            format!("{:.3} m, {:.3} m", e - 2_600_000.0, n - 1_200_000.0),
        );
        ctx.step(
            "Back through the sphere",
            "Mercator undone, the rotation undone, then swisstopo's iteration for the latitude",
            grid.to_string(),
            format!(
                "{}°",
                gp_base::display::number(lat, Precision::Decimals(9), ctx.options.format)
            ),
        );
    }
    Ok(Json::obj([
        ("lat", ctx.out("lat", deg(lat))),
        ("lon", ctx.out("lon", deg(lon))),
        ("grid", Json::str(grid)),
        ("datum", Json::str(LV95_NOTE)),
    ]))
}

// ------------------------------------------------------- New Zealand (NZTM2000)

const LINZ_NZTM: Reference = Reference {
    title: "New Zealand Transverse Mercator 2000 (NZTM2000), defined by LINZS25002 Standard for New Zealand Geodetic Datum 2000 Projections",
    issuer: "Land Information New Zealand (LINZ)",
    year: 2008,
    edition: "LINZS25002 version 2, 24 July 2008",
    locator: "The projection's defining parameters: reference ellipsoid GRS80, datum NZGD2000, origin latitude 0°, central meridian 173°E, false northing 10,000,000 m, false easting 1,600,000 m, central meridian scale factor 0.9996; registered as EPSG 2193",
    url: "https://www.linz.govt.nz/guidance/geodetic-system/coordinate-systems-used-new-zealand/projections/new-zealand-transverse-mercator-2000-nztm2000",
};
const LINZ_DEFORMATION: Reference = Reference {
    title: "New Zealand Geodetic Datum 2000 deformation model",
    issuer: "Land Information New Zealand (LINZ)",
    year: 2024,
    edition: "Geodetic guidance, as published 2024",
    locator: "NZGD2000 was aligned with ITRF96 on 1 January 2000 (epoch 2000.0), and treating WGS84 coordinates as equivalent to NZGD2000 \"has an associated error of 5cm per year since 2000, or 1.25m in 2025\"",
    url: "https://www.linz.govt.nz/guidance/geodetic-system/coordinate-systems-used-new-zealand/geodetic-datums/new-zealand-geodetic-datum-2000-nzgd2000/nzgd2000-deformation-model",
};

/// NZTM2000 (EPSG 2193), on GRS 80 and NZGD2000.
fn nztm_grid() -> TmGrid {
    let g = crate::gridref::grs80();
    TmGrid::new(g.a, g.f, 0.0, 173.0, 0.9996, 1_600_000.0, 10_000_000.0)
}

const NZTM_FRAME: &str = "NZTM2000 is defined on NZGD2000, which was aligned with ITRF96 at epoch 2000.0 and then held there. LINZ puts the error in treating a WGS 84 position as NZGD2000 at 5 cm a year since 2000, which is 1.25 m in 2025; closing it needs LINZ's deformation model.";

const NZTM_EASTING: Field = length(
    "easting",
    "Easting",
    "NZTM2000, from its false origin, like 1748815.78 m",
);
const NZTM_NORTHING: Field = length(
    "northing",
    "Northing",
    "NZTM2000, from its false origin, like 5428791.06 m",
);

const NZTM_GRID_OUT: [Field; 2] = [
    Field::new(
        "easting",
        "Easting",
        "NZTM2000, from its false origin",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(Precision::Decimals(3)),
    Field::new(
        "northing",
        "Northing",
        "NZTM2000, from its false origin",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(Precision::Decimals(3)),
];

const NZTM_CONVERGENCE: Field = Field::new(
    "convergence",
    "Grid convergence",
    "Grid north minus true north at the point",
    Kind::Quantity {
        q: QT::Angle,
        unit: "deg",
    },
)
.precision(Precision::Decimals(6))
.angle_range("[-90,90]");

const NZTM_SCALE: Field = Field::new(
    "scale_factor",
    "Point scale factor",
    "Grid distance over ground distance at the point",
    Kind::Number { min: 0.0, max: 2.0 },
)
.precision(Precision::Decimals(9));

const NZTM_FRAME_OUT: Field = Field::new(
    "frame",
    "Reference frame",
    "What the coordinates are taken to be",
    Kind::Text { max_len: 260 },
);

pub static NZTM_FORWARD: ToolDef = ToolDef {
    id: "geodesy.projection.nztm-forward",
    stability: gp_base::tool::Stability::Stable,
    title: "Latitude and longitude to NZTM2000",
    summary: "Converts an NZGD2000 latitude and longitude to New Zealand Transverse Mercator 2000 easting and northing, the national grid, with the grid convergence and the point scale factor.",
    aliases: &[
        "lat long to NZTM",
        "NZTM2000 converter",
        "New Zealand grid coordinates",
        "latlon to NZTM2000",
    ],
    keywords: &[
        "NZTM2000",
        "NZTM",
        "NZGD2000",
        "New Zealand",
        "transverse Mercator",
        "LINZ",
        "projection",
        "EPSG 2193",
    ],
    inputs: &[
        Field::new(
            "lat",
            "Latitude",
            "North positive, so New Zealand is negative, like -41.278611",
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
            "East positive, like 174.776944",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core()
        .precision(Precision::Decimals(9))
        .angle_range("[-180,180)"),
    ],
    outputs: &[
        NZTM_GRID_OUT[0],
        NZTM_GRID_OUT[1],
        NZTM_CONVERGENCE,
        NZTM_SCALE,
        NZTM_FRAME_OUT,
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["REALIZATION_ASSUMED"],
    model: "Transverse Mercator on GRS 80 with the parameters LINZS25002 states and EPSG registers as 2193: origin latitude 0°, central meridian 173°E, scale 0.9996 on that meridian, false easting 1,600,000 m and false northing 10,000,000 m, by Karney's series",
    accuracy: "The projection is exact to the series' nanometre over New Zealand, and agrees with PROJ applying EPSG 2193 within a millimetre at every test place. The coordinates are taken as NZGD2000: read as a satellite's WGS 84 they are out by the plate motion LINZ states, 5 cm a year since 2000",
    when_to_use: "Use this to put a latitude and longitude onto the grid every New Zealand map, title, and council dataset is drawn in. It gives the grid convergence and the point scale factor beside the coordinates, which is what turns a true bearing into a grid bearing and a ground distance into a grid distance.",
    limitations: "NZTM2000 is defined on NZGD2000, a datum pinned to ITRF96 at the start of 2000 while the ground kept moving: LINZ puts the error in treating a WGS 84 position as NZGD2000 at 5 cm a year, 1.25 m in 2025, and closing it needs LINZ's deformation model, which is not part of this tool. The projection is for the main islands and their shelf; the Chatham Islands and the other outlying groups have their own circuits, and so does cadastral work, which uses the meridional circuits rather than NZTM2000. Heights are untouched.",
    references: &[LINZ_NZTM, LINZ_DEFORMATION],
    examples: &[Example {
        id: "primary",
        title: "The Beehive, Wellington",
        source: "PROJ 9.9.0 applying EPSG 2193: 41.278611°S 174.776944°E on NZGD2000 gives E 1748815.784 N 5428791.056",
        input: r#"{"lat":-41.278611,"lon":174.776944}"#,
    }],
    primary_example: "primary",
    visualization: FORWARD_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.nztm-inverse",
            reason: "inverse",
        },
        Related {
            id: "geodesy.projection.tm-forward",
            reason: "parent",
        },
        Related {
            id: "geodesy.datum.itrf",
            reason: "next",
        },
    ],
    sentence: "That point is E {easting}, N {northing} on NZTM2000.",
    limits: &[("batchRows", 10_000)],
    run: run_nztm_forward,
    ..ToolDef::BLANK
};

fn run_nztm_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let g = nztm_grid().forward(lat, lon);
    ctx.warnings
        .push(Warning::new("REALIZATION_ASSUMED", NZTM_FRAME));
    if ctx.explaining() {
        ctx.step(
            "The frame",
            "NZTM2000 is on NZGD2000, held at epoch 2000.0; a satellite position is on WGS 84",
            format!("{lat:.7}°, {lon:.7}°"),
            "taken as NZGD2000".to_string(),
        );
        ctx.step(
            "Off the central meridian",
            "the projection measures the longitude from 173°E",
            format!("{lon:.7}° against 173°"),
            format!("{:.7}°", lon - 173.0),
        );
        ctx.step(
            "On the NZTM2000 grid",
            "transverse Mercator, GRS 80, origin 0° 173°E, scale 0.9996, false origin 1,600 km east and 10,000 km north",
            format!("{lat:.7}°, {lon:.7}°"),
            format!(
                "{} m",
                gp_base::display::number(g.e, Precision::Decimals(3), ctx.options.format)
            ),
        );
    }
    Ok(Json::obj([
        ("easting", ctx.out("easting", meters(g.e))),
        ("northing", ctx.out("northing", meters(g.n))),
        ("convergence", ctx.out("convergence", deg(g.convergence))),
        ("scale_factor", Json::Num(g.k)),
        ("frame", Json::str(NZTM_FRAME)),
    ]))
}

pub static NZTM_INVERSE: ToolDef = ToolDef {
    id: "geodesy.projection.nztm-inverse",
    stability: gp_base::tool::Stability::Stable,
    title: "NZTM2000 to latitude and longitude",
    summary: "Converts New Zealand Transverse Mercator 2000 easting and northing back to an NZGD2000 latitude and longitude, with the grid convergence and the point scale factor there.",
    aliases: &[
        "NZTM to lat long",
        "NZTM2000 to latitude longitude",
        "read New Zealand grid coordinates",
        "NZTM easting northing to coordinates",
    ],
    keywords: &[
        "NZTM2000",
        "NZTM",
        "NZGD2000",
        "New Zealand",
        "transverse Mercator",
        "LINZ",
        "inverse",
        "EPSG 2193",
    ],
    inputs: &[
        NZTM_EASTING.required().core(),
        NZTM_NORTHING.required().core(),
    ],
    outputs: &[
        Field::new(
            "lat",
            "Latitude",
            "On NZGD2000, north positive",
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
            "On NZGD2000, east positive",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(9))
        .angle_range("[-180,180)"),
        NZTM_CONVERGENCE,
        NZTM_SCALE,
        NZTM_FRAME_OUT,
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["REALIZATION_ASSUMED"],
    model: "The inverse transverse Mercator on GRS 80 with LINZS25002's parameters, which EPSG registers as 2193 (origin 0° 173°E, scale 0.9996, false easting 1,600,000 m, false northing 10,000,000 m), by Karney's series",
    accuracy: "The inverse is the forward's own inverse rather than a separate approximation, and the round trip returns every test place to within a nanodegree. The answer is on NZGD2000: read as a satellite's WGS 84 it is out by the plate motion LINZ states, 5 cm a year since 2000",
    when_to_use: "Use this to read an easting and northing off a New Zealand map, title plan, or council dataset and get the latitude and longitude a phone or a web map wants. It returns the convergence and the point scale factor as well, which is what turns a grid bearing back into a true one.",
    limitations: "The answer is on NZGD2000, which LINZ puts 5 cm a year from a satellite's WGS 84 since 2000, 1.25 m in 2025; closing that needs LINZ's deformation model, which is not part of this tool. These are not meridional circuit coordinates, which New Zealand cadastral surveys use and which are a different projection with its own origin for each circuit; nor are they the older New Zealand Map Grid. Heights are untouched.",
    references: &[LINZ_NZTM, LINZ_DEFORMATION],
    examples: &[Example {
        id: "primary",
        title: "The Beehive, Wellington",
        source: "PROJ 9.9.0 applying EPSG 2193 in reverse: E 1748815.784172 N 5428791.056045 gives 41.278611°S 174.776944°E on NZGD2000",
        input: r#"{"easting":"1748815.784172 m","northing":"5428791.056045 m"}"#,
    }],
    primary_example: "primary",
    visualization: INVERSE_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.nztm-forward",
            reason: "inverse",
        },
        Related {
            id: "geodesy.projection.tm-inverse",
            reason: "parent",
        },
        Related {
            id: "geodesy.datum.itrf",
            reason: "next",
        },
    ],
    sentence: "That is {lat}, {lon} on NZGD2000.",
    limits: &[("batchRows", 10_000)],
    run: run_nztm_inverse,
    ..ToolDef::BLANK
};

fn run_nztm_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (e, n) = grid_in(ctx)?;
    if !e.is_finite() || !n.is_finite() {
        return Err(ToolError::invalid(
            "/easting",
            "The easting and northing are metres on the NZTM2000 grid.",
        ));
    }
    let grid = nztm_grid();
    let (lat, lon) = grid.inverse(e, n);
    if !lat.is_finite() || !lon.is_finite() || !(-90.0..=90.0).contains(&lat) {
        return Err(ToolError::invalid(
            "/northing",
            "That easting and northing is not a place on the NZTM2000 grid.",
        ));
    }
    let back = grid.forward(lat, lon);
    ctx.warnings
        .push(Warning::new("REALIZATION_ASSUMED", NZTM_FRAME));
    if ctx.explaining() {
        ctx.step(
            "From the false origin",
            "the grid counts 1,600 km east and 10,000 km north of its true origin",
            format!("E {e:.3} m, N {n:.3} m"),
            format!(
                "{:.3} m east, {:.3} m north of it",
                e - 1_600_000.0,
                n - 10_000_000.0
            ),
        );
        ctx.step(
            "Off the NZTM2000 grid",
            "the inverse transverse Mercator, GRS 80, origin 0° 173°E, scale 0.9996",
            format!("E {e:.3} m, N {n:.3} m"),
            format!(
                "{}°",
                gp_base::display::number(lat, Precision::Decimals(9), ctx.options.format)
            ),
        );
    }
    Ok(Json::obj([
        ("lat", ctx.out("lat", deg(lat))),
        ("lon", ctx.out("lon", deg(lon))),
        ("convergence", ctx.out("convergence", deg(back.convergence))),
        ("scale_factor", Json::Num(back.k)),
        ("frame", Json::str(NZTM_FRAME)),
    ]))
}

// ------------------------------------------------- Australia (MGA2020, MGA94)

const GA_MGA: Reference = Reference {
    title: "Map Grid of Australia 2020 (MGA2020), Geoscience Australia's datums and projections guidance",
    issuer: "Geoscience Australia",
    year: 2026,
    edition: "As published, read 2026-11-14",
    locator: "The grid is the Universal Transverse Mercator system on the GRS80 ellipsoid applied to GDA2020 latitudes and longitudes: 60 zones six degrees of longitude wide, zone 1 beginning at 177 degrees west, a central meridian in the middle of each; Table 1 gives the central scale factor 0.9996, the false easting 500,000 m, and the false northing 10,000,000 m in the southern hemisphere",
    url: "https://www.ga.gov.au/scientific-topics/positioning-navigation/geodesy/datums-projections/grid2020",
};

const EPSG_MGA: Reference = Reference {
    title: "EPSG Geodetic Parameter Dataset (EPSG v13.102 CRS registry)",
    issuer: "IOGP, from the Intergovernmental Committee on Surveying and Mapping (ICSM)",
    year: 2026,
    edition: "v13.102, as shipped in PROJ 9.9.0",
    locator: "GDA2020 / MGA zones 46 to 59 as EPSG 7846 to 7859, and GDA94 / MGA zones 48 to 58 as EPSG 28348 to 28358; each declares transverse Mercator on GRS 80, scale 0.9996 on the central meridian, false easting 500,000 m and false northing 10,000,000 m, with the zone's own area of use",
    url: "https://epsg.org/crs/7856",
};

/// The MGA zones EPSG registers on GDA2020: 46 through 59.
const MGA_ZONE_MIN: i32 = 46;
const MGA_ZONE_MAX: i32 = 59;

/// An MGA zone's central meridian, degrees east. Zone 1 is centred on 177 W,
/// and each zone is six degrees wide, so zone n is centred on 6n - 183.
fn mga_central_meridian(zone: i32) -> f64 {
    6.0 * f64::from(zone) - 183.0
}

/// The zone whose six-degree band holds this longitude.
fn mga_zone_for(lon: f64) -> i32 {
    let wrapped = ((lon + 180.0).rem_euclid(360.0)) / 6.0;
    (wrapped.floor() as i32) + 1
}

/// MGA zone `zone`: UTM on GRS 80, the same projection for MGA2020 and MGA94.
fn mga_grid(zone: i32) -> TmGrid {
    let g = crate::gridref::grs80();
    TmGrid::new(
        g.a,
        g.f,
        0.0,
        mga_central_meridian(zone),
        0.9996,
        500_000.0,
        10_000_000.0,
    )
}

const MGA2020_FRAME: &str = "MGA2020 is the Universal Transverse Mercator grid on GRS 80 applied to GDA2020 coordinates. The projection is taken to be given GDA2020 latitude and longitude; a GDA94 position put through it is MGA94, not MGA2020, and the two datums place the same ground point about 1.8 m apart.";

const MGA94_FRAME: &str = "MGA94 is the same Universal Transverse Mercator grid on GRS 80, applied to GDA94 coordinates instead. MGA94 and MGA2020 share every projection parameter, so the same latitude and longitude gives the same easting and northing on both; what differs is the datum the latitude and longitude are on, and the same ground point is about 1.8 m apart on the two. Moving between the grids is a datum step, not a projection.";

const MGA_AREA: &str = "The point is outside the area EPSG publishes for the MGA zones, which runs from 90 degrees to 174 degrees east and from 60 degrees to 8 degrees south. The arithmetic still holds, but this is not Australian national grid territory.";

const MGA_DATUM_IN: Field = Field::new(
    "datum",
    "Grid",
    "MGA2020 on GDA2020, or MGA94 on GDA94; the default is MGA2020",
    Kind::Choice(&["MGA2020", "MGA94"]),
);

const MGA_ZONE_OUT: Field = Field::new(
    "zone",
    "Zone",
    "The MGA zone the coordinates are on",
    Kind::Number {
        min: MGA_ZONE_MIN as f64,
        max: MGA_ZONE_MAX as f64,
    },
)
.precision(Precision::Decimals(0));

const MGA_CM_OUT: Field = Field::new(
    "central_meridian",
    "Central meridian",
    "The meridian the zone is drawn about, east positive",
    Kind::Quantity {
        q: QT::Angle,
        unit: "deg",
    },
)
.precision(Precision::Decimals(0))
.angle_range("[-180,180]");

const MGA_EPSG_OUT: Field = Field::new(
    "epsg",
    "EPSG code",
    "The CRS code for this zone and grid, where one is registered",
    Kind::Text { max_len: 80 },
);

const MGA_EASTING_OUT: Field = Field::new(
    "easting",
    "Easting",
    "Metres, with the 500,000 m false easting",
    Kind::Quantity {
        q: QT::Length,
        unit: "m",
    },
)
.precision(Precision::Decimals(3));

const MGA_NORTHING_OUT: Field = Field::new(
    "northing",
    "Northing",
    "Metres, with the 10,000,000 m southern false northing",
    Kind::Quantity {
        q: QT::Length,
        unit: "m",
    },
)
.precision(Precision::Decimals(3));

const MGA_CONVERGENCE: Field = Field::new(
    "convergence",
    "Grid convergence",
    "Grid north minus true north at the point",
    Kind::Quantity {
        q: QT::Angle,
        unit: "deg",
    },
)
.precision(Precision::Decimals(6))
.angle_range("[-90,90]");

const MGA_SCALE: Field = Field::new(
    "scale_factor",
    "Point scale factor",
    "Grid distance over ground distance at the point",
    Kind::Number { min: 0.0, max: 2.0 },
)
.precision(Precision::Decimals(9));

const MGA_FRAME_OUT: Field = Field::new(
    "frame",
    "Reference frame",
    "What the coordinates are taken to be",
    Kind::Text { max_len: 420 },
);

/// Reads the `datum` choice, returning the grid's name, its frame note, and
/// the EPSG code base for a zone (`None` where the grid has no registered
/// code for that zone).
fn mga_which(ctx: &Ctx) -> Result<(&'static str, &'static str), ToolError> {
    Ok(match ctx.choice("datum")? {
        Some("MGA94") => ("MGA94", MGA94_FRAME),
        _ => ("MGA2020", MGA2020_FRAME),
    })
}

/// The EPSG code for a grid and zone, as a sentence. MGA2020 is registered
/// for zones 46 to 59; MGA94 only for 48 to 58, which is why this is text
/// rather than a number.
fn mga_epsg(grid: &str, zone: i32) -> String {
    if grid == "MGA94" {
        if (48..=58).contains(&zone) {
            return format!("EPSG:{} (GDA94 / MGA zone {zone})", 28_300 + zone);
        }
        return format!("none: EPSG registers GDA94 / MGA only for zones 48 to 58, not {zone}");
    }
    format!("EPSG:{} (GDA2020 / MGA zone {zone})", 7_800 + zone)
}

/// Checks a zone against the registered range, naming it on failure.
fn mga_check_zone(zone: i32, at: &str) -> Result<(), ToolError> {
    if (MGA_ZONE_MIN..=MGA_ZONE_MAX).contains(&zone) {
        return Ok(());
    }
    Err(ToolError::new(
        ErrorCode::OutOfDomain,
        format!(
            "MGA zone {zone} is not one of the Australian zones. The Map Grid of Australia runs from zone {MGA_ZONE_MIN} to zone {MGA_ZONE_MAX}."
        ),
    )
    .at(at))
}

/// Warns when the point sits outside the published Australian extent.
fn mga_area_warning(ctx: &mut Ctx, lat: f64, lon: f64) {
    let inside = (-60.0..=-8.0).contains(&lat) && (90.0..=174.0).contains(&lon);
    if !inside {
        ctx.warnings
            .push(Warning::new("OUTSIDE_AREA_OF_USE", MGA_AREA));
    }
}

pub static MGA_FORWARD: ToolDef = ToolDef {
    id: "geodesy.projection.mga-forward",
    stability: gp_base::tool::Stability::Stable,
    title: "Latitude and longitude to MGA2020",
    summary: "Converts a GDA2020 latitude and longitude to Map Grid of Australia easting and northing, picking the zone or taking yours, with the grid convergence and the point scale factor; takes MGA94 on GDA94 too.",
    aliases: &[
        "lat long to MGA",
        "MGA2020 converter",
        "Australian grid coordinates",
        "latlon to MGA2020",
    ],
    keywords: &[
        "MGA2020",
        "MGA",
        "MGA94",
        "GDA2020",
        "Australia",
        "transverse Mercator",
        "UTM",
        "EPSG 7856",
    ],
    inputs: &[
        Field::new(
            "lat",
            "Latitude",
            "North positive, so Australia is negative, like -33.858611",
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
            "East positive, like 151.214167",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core()
        .precision(Precision::Decimals(9))
        .angle_range("[-180,180)"),
        Field::new(
            "zone",
            "Force zone",
            "Optional: an MGA zone 46 to 59 to use instead of the one the longitude falls in",
            Kind::Number {
                min: MGA_ZONE_MIN as f64,
                max: MGA_ZONE_MAX as f64,
            },
        ),
        MGA_DATUM_IN,
    ],
    outputs: &[
        // The coordinates are the answer and lead; the zone is the context
        // needed to read them, and follows. The last explain step ends on
        // the easting, which the trace gate requires to be this first output.
        MGA_EASTING_OUT,
        MGA_NORTHING_OUT,
        MGA_ZONE_OUT,
        MGA_CONVERGENCE,
        MGA_SCALE,
        MGA_CM_OUT,
        MGA_EPSG_OUT,
        MGA_FRAME_OUT,
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &[
        "REALIZATION_ASSUMED",
        "NONSTANDARD_ZONE",
        "OUTSIDE_AREA_OF_USE",
    ],
    model: "Universal Transverse Mercator on GRS 80 with the parameters Geoscience Australia states and EPSG registers for the MGA zones: origin latitude 0°, the zone's central meridian at 6n − 183, scale 0.9996 on it, false easting 500,000 m and false northing 10,000,000 m, by Karney's series",
    accuracy: "The projection is exact to the series' nanometre over Australia, and agrees with PROJ applying the zone's own EPSG record within a millimetre at every test place. The coordinates are taken to be on the datum named: a GDA94 latitude and longitude run as MGA2020 is out by the 1.8 m between the datums",
    when_to_use: "Use this to put a latitude and longitude onto the grid Australian survey plans, cadastral datasets, and state spatial services are drawn in. It picks the zone from the longitude, or takes a zone you name when a job runs across a boundary, and returns the grid convergence and the point scale factor beside the coordinates, which is what turns a true bearing into a grid bearing and a ground distance into a grid distance.",
    limitations: "MGA94 and MGA2020 are the same projection on two different datums, so this tool does not move a position between them: the same latitude and longitude gives the same easting and northing under either name, and the ground point moves about 1.8 m when the datum changes. The datum step is geodesy.datum.gda2020. The tool does not apply the conformal-plus-distortion grid, and it leaves heights untouched: AUSGeoid2020 is a separate asset. Coordinates far outside Australia are arithmetic rather than a national grid.",
    references: &[GA_MGA, EPSG_MGA],
    examples: &[Example {
        id: "primary",
        title: "The Sydney Opera House",
        source: "PROJ 9.9.0 applying EPSG:7856 (GDA2020 / MGA zone 56): 33.858611°S 151.214167°E gives E 334799.233 N 6252086.100",
        input: r#"{"lat":-33.858611,"lon":151.214167}"#,
    }],
    primary_example: "primary",
    visualization: FORWARD_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.mga-inverse",
            reason: "inverse",
        },
        Related {
            id: "geodesy.datum.gda2020",
            reason: "next",
        },
        Related {
            id: "geodesy.utm.forward",
            reason: "parent",
        },
    ],
    sentence: "That point is E {easting}, N {northing} in MGA zone {zone}.",
    limits: &[("batchRows", 10_000)],
    run: run_mga_forward,
    ..ToolDef::BLANK
};

fn run_mga_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let (grid_name, frame) = mga_which(ctx)?;
    let standard = mga_zone_for(lon);
    let zone = match ctx.number("zone")? {
        Some(z) => {
            // The field declares 46 to 59, so the input validator has already
            // refused anything outside it by the time this runs.
            let z = z.round() as i32;
            if z != standard {
                ctx.warnings.push(Warning::new(
                    "NONSTANDARD_ZONE",
                    format!(
                        "The longitude {lon:.6}° falls in MGA zone {standard}, and the answer is on zone {z} as asked. Coordinates on a forced zone are further from its central meridian, so the point scale factor is further from 1."
                    ),
                ));
            }
            z
        }
        None => {
            mga_check_zone(standard, "/lon")?;
            standard
        }
    };
    let g = mga_grid(zone).forward(lat, lon);
    ctx.warnings
        .push(Warning::new("REALIZATION_ASSUMED", frame));
    mga_area_warning(ctx, lat, lon);
    let cm = mga_central_meridian(zone);
    if ctx.explaining() {
        ctx.step(
            "The zone",
            "the zones are six degrees wide, and zone n is drawn about the meridian 6n − 183",
            format!("{lon:.6}° east"),
            format!("zone {zone}, central meridian {cm:.0}°E"),
        );
        ctx.step(
            "Off the central meridian",
            "the projection measures the longitude from the zone's meridian",
            format!("{lon:.6}° against {cm:.0}°"),
            format!("{:.6}°", lon - cm),
        );
        ctx.step(
            "On the grid",
            "transverse Mercator, GRS 80, scale 0.9996 on the central meridian, false origin 500 km east and 10,000 km north",
            format!("{lat:.6}°, {lon:.6}°"),
            format!(
                "{} m",
                gp_base::display::number(g.e, Precision::Decimals(3), ctx.options.format)
            ),
        );
    }
    Ok(Json::obj([
        ("zone", Json::Num(f64::from(zone))),
        ("easting", ctx.out("easting", meters(g.e))),
        ("northing", ctx.out("northing", meters(g.n))),
        ("convergence", ctx.out("convergence", deg(g.convergence))),
        ("scale_factor", Json::Num(g.k)),
        ("central_meridian", ctx.out("central_meridian", deg(cm))),
        ("epsg", Json::str(mga_epsg(grid_name, zone))),
        ("frame", Json::str(frame)),
    ]))
}

pub static MGA_INVERSE: ToolDef = ToolDef {
    id: "geodesy.projection.mga-inverse",
    stability: gp_base::tool::Stability::Stable,
    title: "MGA2020 to latitude and longitude",
    summary: "Converts a Map Grid of Australia zone, easting, and northing back to a GDA2020 latitude and longitude, with the grid convergence and the point scale factor there; reads MGA94 on GDA94 too.",
    aliases: &[
        "MGA to lat long",
        "MGA2020 to latitude longitude",
        "read Australian grid coordinates",
        "MGA easting northing to coordinates",
    ],
    keywords: &[
        "MGA2020",
        "MGA",
        "MGA94",
        "GDA2020",
        "Australia",
        "transverse Mercator",
        "inverse",
        "EPSG 7856",
    ],
    inputs: &[
        Field::new(
            "zone",
            "Zone",
            "The MGA zone the coordinates are on, 46 to 59, like 56",
            Kind::Number {
                min: MGA_ZONE_MIN as f64,
                max: MGA_ZONE_MAX as f64,
            },
        )
        .required()
        .core(),
        length(
            "easting",
            "Easting",
            "On the zone, from its false origin, like 334799.233 m",
        )
        .required()
        .core(),
        length(
            "northing",
            "Northing",
            "On the zone, from its false origin, like 6252086.100 m",
        )
        .required()
        .core(),
        MGA_DATUM_IN,
    ],
    outputs: &[
        Field::new(
            "lat",
            "Latitude",
            "On the datum named, north positive",
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
            "On the datum named, east positive",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(9))
        .angle_range("[-180,180)"),
        MGA_CONVERGENCE,
        MGA_SCALE,
        MGA_CM_OUT,
        MGA_EPSG_OUT,
        MGA_FRAME_OUT,
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["REALIZATION_ASSUMED", "OUTSIDE_AREA_OF_USE"],
    model: "The inverse Universal Transverse Mercator on GRS 80 with the MGA zone's parameters (origin 0°, central meridian 6n − 183, scale 0.9996, false easting 500,000 m, false northing 10,000,000 m), by Karney's series",
    accuracy: "The inverse is the forward's own inverse rather than a separate approximation, and the round trip returns every test place to within a nanodegree. The answer is on the datum named: read as the other one it is out by the 1.8 m between GDA94 and GDA2020",
    when_to_use: "Use this to read an easting and northing off an Australian survey plan, title, or council dataset and get the latitude and longitude a phone or a web map wants. MGA coordinates do not carry their zone, so the zone is asked for; the answer returns the convergence and the point scale factor as well, which is what turns a grid bearing back into a true one.",
    limitations: "An MGA easting and northing without its zone is ambiguous, and nothing in the numbers says which of the fourteen zones they belong to, so the zone is an input rather than a guess. MGA94 and MGA2020 read the same numbers to the same latitude and longitude, on two datums about 1.8 m apart; moving between the datums is geodesy.datum.gda2020, not this tool. Heights are untouched.",
    references: &[GA_MGA, EPSG_MGA],
    examples: &[Example {
        id: "primary",
        title: "The Sydney Opera House",
        source: "PROJ 9.9.0 applying EPSG:7856 in reverse: zone 56, E 334799.233 N 6252086.100 gives 33.858611°S 151.214167°E on GDA2020",
        input: r#"{"zone":56,"easting":"334799.2334757 m","northing":"6252086.0999992 m"}"#,
    }],
    primary_example: "primary",
    visualization: INVERSE_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.mga-forward",
            reason: "inverse",
        },
        Related {
            id: "geodesy.datum.gda2020",
            reason: "next",
        },
        Related {
            id: "geodesy.utm.inverse",
            reason: "parent",
        },
    ],
    sentence: "That is {lat}, {lon}.",
    limits: &[("batchRows", 10_000)],
    run: run_mga_inverse,
    ..ToolDef::BLANK
};

fn run_mga_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let zone = ctx
        .number("zone")?
        .ok_or_else(|| ToolError::invalid("/zone", "Name the MGA zone, 46 to 59."))?
        .round() as i32;
    mga_check_zone(zone, "/zone")?;
    let (grid_name, frame) = mga_which(ctx)?;
    let (e, n) = grid_in(ctx)?;
    if !e.is_finite() || !n.is_finite() {
        return Err(ToolError::invalid(
            "/easting",
            "The easting and northing are metres on an MGA zone.",
        ));
    }
    let grid = mga_grid(zone);
    let (lat, lon) = grid.inverse(e, n);
    if !lat.is_finite() || !lon.is_finite() || !(-90.0..=90.0).contains(&lat) {
        return Err(ToolError::invalid(
            "/northing",
            "That easting and northing is not a place on the MGA grid.",
        ));
    }
    let back = grid.forward(lat, lon);
    ctx.warnings
        .push(Warning::new("REALIZATION_ASSUMED", frame));
    mga_area_warning(ctx, lat, lon);
    let cm = mga_central_meridian(zone);
    if ctx.explaining() {
        ctx.step(
            "From the false origin",
            "each zone counts 500 km east and 10,000 km north of its true origin",
            format!("E {e:.3} m, N {n:.3} m"),
            format!(
                "{:.3} m east, {:.3} m north of it",
                e - 500_000.0,
                n - 10_000_000.0
            ),
        );
        ctx.step(
            "Off the grid",
            "the inverse transverse Mercator, GRS 80, scale 0.9996 on the zone's central meridian",
            format!("zone {zone}, central meridian {cm:.0}°E"),
            format!(
                "{}°",
                gp_base::display::number(lat, Precision::Decimals(9), ctx.options.format)
            ),
        );
    }
    Ok(Json::obj([
        ("lat", ctx.out("lat", deg(lat))),
        ("lon", ctx.out("lon", deg(lon))),
        ("convergence", ctx.out("convergence", deg(back.convergence))),
        ("scale_factor", Json::Num(back.k)),
        ("central_meridian", ctx.out("central_meridian", deg(cm))),
        ("epsg", Json::str(mga_epsg(grid_name, zone))),
        ("frame", Json::str(frame)),
    ]))
}

// ------------------------------------------- Japan (JGD2011 plane rectangular)

const MLIT_JPC: Reference = Reference {
    title: "Plane rectangular coordinate systems (平面直角座標系), Ministry of Land, Infrastructure, Transport and Tourism Notice No. 9 of 2002",
    issuer: "Ministry of Land, Infrastructure, Transport and Tourism of Japan, as published by the Geospatial Information Authority of Japan (GSI)",
    year: 2010,
    edition: "Notice No. 9 of 2002, last amended by Notice No. 289 of 2010, read 2026-11-16",
    locator: "The table of systems I to XIX gives each origin's longitude and latitude and the prefectures it applies to; the remarks state the Gauss conformal projection, the X axis along the origin's meridian positive to true north, the Y axis positive to true east, the scale factor 0.9999 on the X axis, and X = 0.000 m, Y = 0.000 m at the origin",
    url: "https://www.gsi.go.jp/LAW/heimencho.html",
};

const EPSG_JGD: Reference = Reference {
    title: "EPSG Geodetic Parameter Dataset (EPSG v13.102 CRS registry)",
    issuer: "IOGP, from the Geospatial Information Authority of Japan",
    year: 2026,
    edition: "v13.102, as shipped in PROJ 9.9.0",
    locator: "JGD2011 / Japan Plane Rectangular CS I to XIX as EPSG 6669 to 6687, each transverse Mercator on GRS 1980 with the notice's origin, scale 0.9999 and false easting and northing 0 m, and JGD2011 itself (EPSG 6668) with its area of use",
    url: "https://epsg.org/crs/6677",
};

const JGD_ZONES: i32 = 19;

/// Each system's origin, latitude then longitude in degrees, from the notice's
/// table (system I first). Minutes are written as fractions so the parameters
/// read as the notice prints them.
const JGD_ORIGINS: [(f64, f64); 19] = [
    (33.0, 129.0 + 30.0 / 60.0),
    (33.0, 131.0),
    (36.0, 132.0 + 10.0 / 60.0),
    (33.0, 133.0 + 30.0 / 60.0),
    (36.0, 134.0 + 20.0 / 60.0),
    (36.0, 136.0),
    (36.0, 137.0 + 10.0 / 60.0),
    (36.0, 138.0 + 30.0 / 60.0),
    (36.0, 139.0 + 50.0 / 60.0),
    (40.0, 140.0 + 50.0 / 60.0),
    (44.0, 140.0 + 15.0 / 60.0),
    (44.0, 142.0 + 15.0 / 60.0),
    (44.0, 144.0 + 15.0 / 60.0),
    (26.0, 142.0),
    (26.0, 127.0 + 30.0 / 60.0),
    (26.0, 124.0),
    (26.0, 131.0),
    (20.0, 136.0),
    (26.0, 154.0),
];

const ROMAN: [&str; 19] = [
    "I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X", "XI", "XII", "XIII", "XIV", "XV",
    "XVI", "XVII", "XVIII", "XIX",
];

/// System `zone` (1 to 19): transverse Mercator on GRS 80 about the notice's
/// origin, scale 0.9999, the origin at X = 0, Y = 0.
fn jgd_grid(zone: i32) -> TmGrid {
    let g = crate::gridref::grs80();
    let (lat0, lon0) = JGD_ORIGINS[(zone - 1) as usize];
    TmGrid::new(g.a, g.f, lat0, lon0, 0.9999, 0.0, 0.0)
}

fn jgd_epsg(zone: i32) -> String {
    format!(
        "EPSG:{} (JGD2011 / Japan Plane Rectangular CS {})",
        6668 + zone,
        ROMAN[(zone - 1) as usize]
    )
}

fn jgd_zone(ctx: &mut Ctx) -> Result<i32, ToolError> {
    // The field declares 1 to 19, so the input validator has already refused
    // anything outside it by the time this runs.
    Ok(ctx
        .number("zone")?
        .ok_or_else(|| {
            ToolError::invalid(
                "/zone",
                "Name the plane rectangular system, 1 to 19. It follows the prefecture, not the longitude.",
            )
        })?
        .round() as i32)
}

const JGD_FRAME: &str = "The coordinates are taken to be on JGD2011, Japan's datum since the 2011 Tohoku earthquake. JGD2000 uses the same nineteen systems with the same parameters, so the same latitude and longitude gives the same X and Y on both; what differs is the datum, which the earthquake moved by up to about 5 m in eastern Japan. Coordinates on the older Tokyo datum are on a different ellipsoid and are not this grid.";

const JGD_AREA: &str = "The point is outside the area EPSG publishes for JGD2011, about 122 to 158 degrees east and 17 to 46 degrees north. The arithmetic still holds, but this is not Japanese national grid territory.";

fn jgd_area_warning(ctx: &mut Ctx, lat: f64, lon: f64) {
    let inside = (17.09..=46.05).contains(&lat) && (122.38..=157.65).contains(&lon);
    if !inside {
        ctx.warnings
            .push(Warning::new("OUTSIDE_AREA_OF_USE", JGD_AREA));
    }
}

const JGD_ZONE_IN: Field = Field::new(
    "zone",
    "System",
    "The plane rectangular system, 1 to 19, set by prefecture: 9 for Tokyo, 6 for Osaka and Kyoto, 12 for most of Hokkaido",
    Kind::Number {
        min: 1.0,
        max: JGD_ZONES as f64,
    },
)
.required()
.core();

const JGD_ZONE_OUT: Field = Field::new(
    "zone",
    "System",
    "The plane rectangular system the coordinates are on",
    Kind::Number {
        min: 1.0,
        max: JGD_ZONES as f64,
    },
)
.precision(Precision::Decimals(0));

const JGD_ORIGIN_LAT_OUT: Field = Field::new(
    "origin_lat",
    "Origin latitude",
    "Where the system's X is zero",
    Kind::Quantity {
        q: QT::Angle,
        unit: "deg",
    },
)
.precision(Precision::Decimals(6))
.angle_range("[-90,90]");

const JGD_ORIGIN_LON_OUT: Field = Field::new(
    "central_meridian",
    "Origin longitude",
    "The meridian the system is drawn about, where Y is zero",
    Kind::Quantity {
        q: QT::Angle,
        unit: "deg",
    },
)
.precision(Precision::Decimals(6))
.angle_range("[-180,180]");

const JGD_EASTING_OUT: Field = Field::new(
    "easting",
    "Y (easting)",
    "Metres east of the system's origin, negative to the west; Japanese surveys call this Y",
    Kind::Quantity {
        q: QT::Length,
        unit: "m",
    },
)
.precision(Precision::Decimals(3));

const JGD_NORTHING_OUT: Field = Field::new(
    "northing",
    "X (northing)",
    "Metres north of the system's origin, negative to the south; Japanese surveys call this X",
    Kind::Quantity {
        q: QT::Length,
        unit: "m",
    },
)
.precision(Precision::Decimals(3));

pub static JGD2011_FORWARD: ToolDef = ToolDef {
    id: "geodesy.projection.jgd2011-forward",
    stability: gp_base::tool::Stability::Stable,
    title: "Latitude and longitude to Japan plane rectangular",
    summary: "Converts a JGD2011 latitude and longitude to X and Y on one of Japan's nineteen plane rectangular systems, with the grid convergence and the point scale factor.",
    aliases: &[
        "lat long to Japan plane rectangular",
        "JGD2011 plane rectangular converter",
        "Japanese survey coordinates",
        "heimen chokaku zahyo",
    ],
    keywords: &[
        "JGD2011",
        "Japan",
        "plane rectangular",
        "平面直角座標系",
        "transverse Mercator",
        "EPSG 6677",
        "JGD2000",
    ],
    inputs: &[
        Field::new(
            "lat",
            "Latitude",
            "North positive, like 35.681236",
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
            "East positive, like 139.767125",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core()
        .precision(Precision::Decimals(9))
        .angle_range("[-180,180)"),
        JGD_ZONE_IN,
    ],
    outputs: &[
        // The last explain step ends on the easting, which the trace gate
        // requires to be this first output.
        JGD_EASTING_OUT,
        JGD_NORTHING_OUT,
        JGD_ZONE_OUT,
        MGA_CONVERGENCE,
        MGA_SCALE,
        JGD_ORIGIN_LAT_OUT,
        JGD_ORIGIN_LON_OUT,
        MGA_EPSG_OUT,
        MGA_FRAME_OUT,
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["REALIZATION_ASSUMED", "OUTSIDE_AREA_OF_USE"],
    model: "Gauss conformal (transverse Mercator) projection on GRS 80 with the parameters of MLIT Notice No. 9 of 2002 and EPSG 6669 to 6687: the system's own origin latitude and longitude, scale 0.9999 on its meridian, X = 0 and Y = 0 at the origin, by Karney's series",
    accuracy: "The projection is exact to the series' nanometre across Japan, and agrees with PROJ applying each system's own EPSG record within a millimetre at every test place. The coordinates are taken to be on JGD2011: a latitude and longitude on the older Tokyo datum is out by about 400 m",
    when_to_use: "Use this to put a latitude and longitude onto the grid Japanese public surveys, cadastral maps, and municipal datasets are drawn in. The system is set by prefecture rather than by longitude, so it is asked for: 9 for Tokyo and the Kanto prefectures, 6 for Osaka and Kyoto, 12 for most of Hokkaido. The answer returns the grid convergence and the point scale factor beside X and Y.",
    limitations: "The tool does not pick the system, because the notice assigns systems by prefecture and by island group, not by a band of longitude; a point projected on the wrong system gives valid but useless numbers. It projects and does not transform: a Tokyo-datum latitude and longitude must first be moved to JGD2011, and GSI's grids for that step need Survey Act approval to use, so they are not here. Heights are untouched.",
    references: &[MLIT_JPC, EPSG_JGD],
    examples: &[Example {
        id: "primary",
        title: "Tokyo Station",
        source: "PROJ 9.9.0 applying EPSG:6677 (JGD2011 / Japan Plane Rectangular CS IX): 35.681236°N 139.767125°E gives X -35363.238 m, Y -5992.920 m",
        input: r#"{"lat":35.681236,"lon":139.767125,"zone":9}"#,
    }],
    primary_example: "primary",
    visualization: FORWARD_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.jgd2011-inverse",
            reason: "inverse",
        },
        Related {
            id: "geodesy.utm.forward",
            reason: "parent",
        },
        Related {
            id: "geodesy.projection.tm-forward",
            reason: "alternative",
        },
    ],
    sentence: "That point is X {northing}, Y {easting} on system {zone}.",
    limits: &[("batchRows", 10_000)],
    run: run_jgd2011_forward,
    ..ToolDef::BLANK
};

fn run_jgd2011_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let zone = jgd_zone(ctx)?;
    let (lat0, lon0) = JGD_ORIGINS[(zone - 1) as usize];
    let g = jgd_grid(zone).forward(lat, lon);
    ctx.warnings
        .push(Warning::new("REALIZATION_ASSUMED", JGD_FRAME));
    jgd_area_warning(ctx, lat, lon);
    if ctx.explaining() {
        let roman = ROMAN[(zone - 1) as usize];
        ctx.step(
            "The system's origin",
            "the notice fixes each system's origin, where X and Y are both zero",
            format!("system {roman}"),
            format!("{lat0:.6}°N, {lon0:.6}°E"),
        );
        ctx.step(
            "Off the origin",
            "the projection measures the longitude from the origin's meridian",
            format!("{lon:.6}° against {lon0:.6}°"),
            format!("{:.6}°", lon - lon0),
        );
        ctx.step(
            "On the grid",
            "transverse Mercator, GRS 80, scale 0.9999 on the origin's meridian; X counts north and Y east of the origin",
            format!("{lat:.6}°, {lon:.6}°"),
            format!(
                "{} m",
                gp_base::display::number(g.e, Precision::Decimals(3), ctx.options.format)
            ),
        );
    }
    Ok(Json::obj([
        ("zone", Json::Num(f64::from(zone))),
        ("easting", ctx.out("easting", meters(g.e))),
        ("northing", ctx.out("northing", meters(g.n))),
        ("convergence", ctx.out("convergence", deg(g.convergence))),
        ("scale_factor", Json::Num(g.k)),
        ("origin_lat", ctx.out("origin_lat", deg(lat0))),
        ("central_meridian", ctx.out("central_meridian", deg(lon0))),
        ("epsg", Json::str(jgd_epsg(zone))),
        ("frame", Json::str(JGD_FRAME)),
    ]))
}

pub static JGD2011_INVERSE: ToolDef = ToolDef {
    id: "geodesy.projection.jgd2011-inverse",
    stability: gp_base::tool::Stability::Stable,
    title: "Japan plane rectangular to latitude and longitude",
    summary: "Converts X and Y on one of Japan's nineteen plane rectangular systems back to a JGD2011 latitude and longitude, with the grid convergence and the point scale factor there.",
    aliases: &[
        "Japan plane rectangular to lat long",
        "JGD2011 X Y to latitude longitude",
        "read Japanese survey coordinates",
        "heimen chokaku zahyo to lat long",
    ],
    keywords: &[
        "JGD2011",
        "Japan",
        "plane rectangular",
        "平面直角座標系",
        "transverse Mercator",
        "inverse",
        "EPSG 6677",
    ],
    inputs: &[
        JGD_ZONE_IN,
        length(
            "northing",
            "X (northing)",
            "Metres north of the system's origin, negative to the south, like -35363.238 m",
        )
        .required()
        .core(),
        length(
            "easting",
            "Y (easting)",
            "Metres east of the system's origin, negative to the west, like -5992.920 m",
        )
        .required()
        .core(),
    ],
    outputs: &[
        Field::new(
            "lat",
            "Latitude",
            "On JGD2011, north positive",
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
            "On JGD2011, east positive",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(9))
        .angle_range("[-180,180)"),
        MGA_CONVERGENCE,
        MGA_SCALE,
        JGD_ORIGIN_LAT_OUT,
        JGD_ORIGIN_LON_OUT,
        MGA_EPSG_OUT,
        MGA_FRAME_OUT,
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["REALIZATION_ASSUMED", "OUTSIDE_AREA_OF_USE"],
    model: "The inverse Gauss conformal (transverse Mercator) projection on GRS 80 with the system's parameters from MLIT Notice No. 9 of 2002 (its origin, scale 0.9999, X = 0 and Y = 0 at the origin), by Karney's series",
    accuracy: "The inverse is the forward's own inverse rather than a separate approximation, and the round trip returns every test place to within a nanodegree. The answer is on JGD2011",
    when_to_use: "Use this to read X and Y off a Japanese survey drawing, cadastral map, or municipal dataset and get the latitude and longitude a phone or a web map wants. X is the northing and Y the easting, the opposite of the order most other grids print, so check which column is which. The coordinates do not carry their system, so it is asked for.",
    limitations: "X and Y without their system are ambiguous: every system has its origin at zero, so the same numbers name a place in each of the nineteen. The answer is on JGD2011; X and Y read off a JGD2000 drawing give a JGD2000 position, which in eastern Japan differs from JGD2011 by up to about 5 m because of the 2011 earthquake. Tokyo-datum coordinates are on another ellipsoid and are not this grid. Heights are untouched.",
    references: &[MLIT_JPC, EPSG_JGD],
    examples: &[Example {
        id: "primary",
        title: "Tokyo Station",
        source: "PROJ 9.9.0 applying EPSG:6677 in reverse: system IX, X -35363.238 m, Y -5992.920 m gives 35.681236°N 139.767125°E on JGD2011",
        input: r#"{"zone":9,"northing":"-35363.2377449 m","easting":"-5992.9195704 m"}"#,
    }],
    primary_example: "primary",
    visualization: INVERSE_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.jgd2011-forward",
            reason: "inverse",
        },
        Related {
            id: "geodesy.utm.inverse",
            reason: "parent",
        },
        Related {
            id: "geodesy.projection.tm-inverse",
            reason: "alternative",
        },
    ],
    sentence: "That is {lat}, {lon}.",
    limits: &[("batchRows", 10_000)],
    run: run_jgd2011_inverse,
    ..ToolDef::BLANK
};

fn run_jgd2011_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let zone = jgd_zone(ctx)?;
    let (e, n) = grid_in(ctx)?;
    if !e.is_finite() || !n.is_finite() {
        return Err(ToolError::invalid(
            "/easting",
            "X and Y are metres on a plane rectangular system.",
        ));
    }
    let grid = jgd_grid(zone);
    let (lat, lon) = grid.inverse(e, n);
    if !lat.is_finite() || !lon.is_finite() || !(-90.0..=90.0).contains(&lat) {
        return Err(ToolError::invalid(
            "/northing",
            "That X and Y is not a place on the system.",
        ));
    }
    let back = grid.forward(lat, lon);
    ctx.warnings
        .push(Warning::new("REALIZATION_ASSUMED", JGD_FRAME));
    jgd_area_warning(ctx, lat, lon);
    let (lat0, lon0) = JGD_ORIGINS[(zone - 1) as usize];
    if ctx.explaining() {
        let roman = ROMAN[(zone - 1) as usize];
        ctx.step(
            "The system's origin",
            "the notice fixes each system's origin, where X and Y are both zero",
            format!("system {roman}"),
            format!("{lat0:.6}°N, {lon0:.6}°E"),
        );
        ctx.step(
            "Off the grid",
            "the inverse transverse Mercator, GRS 80, scale 0.9999 on the origin's meridian",
            format!("X {n:.3} m, Y {e:.3} m"),
            format!(
                "{}°",
                gp_base::display::number(lat, Precision::Decimals(9), ctx.options.format)
            ),
        );
    }
    Ok(Json::obj([
        ("lat", ctx.out("lat", deg(lat))),
        ("lon", ctx.out("lon", deg(lon))),
        ("convergence", ctx.out("convergence", deg(back.convergence))),
        ("scale_factor", Json::Num(back.k)),
        ("origin_lat", ctx.out("origin_lat", deg(lat0))),
        ("central_meridian", ctx.out("central_meridian", deg(lon0))),
        ("epsg", Json::str(jgd_epsg(zone))),
        ("frame", Json::str(JGD_FRAME)),
    ]))
}

// ------------------------------------------ South Africa (Hartebeesthoek94 Lo)

const G7_2_TMSO: Reference = Reference {
    title: "Coordinate Conversions and Transformations including Formulas, IOGP Publication 373-7-2 (Guidance Note 7-2)",
    issuer: "International Association of Oil & Gas Producers (IOGP)",
    year: 2019,
    edition: "Revised September 2019",
    locator: "Section 3.2.3.3, Transverse Mercator (South Orientated), EPSG method 9808: westings and southings increase to the west and south from the origin, with the worked example for Hartebeesthoek94 / Lo29 at 25°43'55.302\"S 28°16'57.479\"E giving Y 71984.49 m, X 2847342.74 m",
    url: "https://idms.iogp.org/Documents/474",
};

const EPSG_LO: Reference = Reference {
    title: "EPSG Geodetic Parameter Dataset (EPSG v13.102 CRS registry)",
    issuer: "IOGP, from the Chief Directorate: National Geo-spatial Information, South Africa",
    year: 2026,
    edition: "v13.102, as shipped in PROJ 9.9.0",
    locator: "Hartebeesthoek94 / Lo15 to Lo33 as EPSG 2046 to 2055, each Transverse Mercator (South Orientated) on WGS 84 about an odd meridian from 15°E to 33°E, scale 1, no false westing or southing, axes westing (Y) and southing (X); Hartebeesthoek94 itself (EPSG 4148) with its area of use",
    url: "https://epsg.org/crs/2053",
};

const LO_MIN: i32 = 15;
const LO_MAX: i32 = 33;

/// The Lo belt a longitude falls in: two degrees wide about each odd meridian.
fn lo_for(lon: f64) -> i32 {
    2 * (lon / 2.0).floor() as i32 + 1
}

/// Lo belt `cm`: transverse Mercator on WGS 84 about meridian `cm`, scale 1,
/// origin at the equator. The south-orientated axes are the negated easting
/// and northing.
fn lo_grid(cm: i32) -> TmGrid {
    let w = ellipsoid::CATALOG
        .iter()
        .find(|e| e.id == "wgs84")
        .expect("the ellipsoid catalog lists wgs84");
    TmGrid::new(w.a, w.f, 0.0, f64::from(cm), 1.0, 0.0, 0.0)
}

fn lo_epsg(cm: i32) -> String {
    format!(
        "EPSG:{} (Hartebeesthoek94 / Lo{cm})",
        2046 + (cm - LO_MIN) / 2
    )
}

fn lo_check(cm: i32, at: &str) -> Result<(), ToolError> {
    if (LO_MIN..=LO_MAX).contains(&cm) && cm % 2 == 1 {
        return Ok(());
    }
    Err(ToolError::new(
        ErrorCode::OutOfDomain,
        format!(
            "Lo{cm} is not one of South Africa's belts. The Lo system runs on the odd meridians from Lo{LO_MIN} to Lo{LO_MAX}."
        ),
    )
    .at(at))
}

const LO_FRAME: &str = "The coordinates are taken to be on Hartebeesthoek94, South Africa's datum since 1999: ITRF91 at epoch 1994.0 on the WGS 84 ellipsoid. A modern GNSS position has moved with the African plate since then, a few centimetres a year. Coordinates on the older Cape datum are on another ellipsoid and are not this grid.";

const LO_AREA: &str = "The point is outside the area EPSG publishes for Hartebeesthoek94, South Africa, Lesotho, and Eswatini with their offshore waters, about 13 to 43 degrees east and 22 to 50 degrees south. The arithmetic still holds, but this is not South African national grid territory.";

fn lo_area_warning(ctx: &mut Ctx, lat: f64, lon: f64) {
    let inside = (-50.32..=-22.13).contains(&lat) && (13.33..=42.85).contains(&lon);
    if !inside {
        ctx.warnings
            .push(Warning::new("OUTSIDE_AREA_OF_USE", LO_AREA));
    }
}

const LO_ZONE_OUT: Field = Field::new(
    "zone",
    "Lo belt",
    "The central meridian of the belt the coordinates are on, in degrees east, like 29 for Lo29",
    Kind::Number {
        min: LO_MIN as f64,
        max: LO_MAX as f64,
    },
)
.precision(Precision::Decimals(0));

const LO_WESTING_OUT: Field = Field::new(
    "westing",
    "Y (westing)",
    "Metres west of the belt's central meridian, negative to the east; South African surveys call this Y",
    Kind::Quantity {
        q: QT::Length,
        unit: "m",
    },
)
.precision(Precision::Decimals(3));

const LO_SOUTHING_OUT: Field = Field::new(
    "southing",
    "X (southing)",
    "Metres south of the equator along the belt's grid; South African surveys call this X",
    Kind::Quantity {
        q: QT::Length,
        unit: "m",
    },
)
.precision(Precision::Decimals(3));

pub static LO_FORWARD: ToolDef = ToolDef {
    id: "geodesy.projection.lo-forward",
    stability: gp_base::tool::Stability::Stable,
    title: "Latitude and longitude to South African Lo",
    summary: "Converts a Hartebeesthoek94 latitude and longitude to the Y and X of South Africa's Lo system, picking the two-degree belt or taking yours, with the grid convergence and the point scale factor.",
    aliases: &[
        "lat long to Lo coordinates",
        "South African Lo converter",
        "Hartebeesthoek94 Lo29",
        "latlon to Lo system",
    ],
    keywords: &[
        "Lo",
        "Hartebeesthoek94",
        "South Africa",
        "south orientated",
        "transverse Mercator",
        "westing",
        "southing",
        "EPSG 2053",
    ],
    inputs: &[
        Field::new(
            "lat",
            "Latitude",
            "North positive, so South Africa is negative, like -25.745500",
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
            "East positive, like 28.188000",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core()
        .precision(Precision::Decimals(9))
        .angle_range("[-180,180)"),
        Field::new(
            "zone",
            "Force belt",
            "Optional: an odd central meridian from 15 to 33, like 29 for Lo29, to use instead of the belt the longitude falls in",
            Kind::Number {
                min: LO_MIN as f64,
                max: LO_MAX as f64,
            },
        ),
    ],
    outputs: &[
        // The last explain step ends on the westing, which the trace gate
        // requires to be this first output.
        LO_WESTING_OUT,
        LO_SOUTHING_OUT,
        LO_ZONE_OUT,
        MGA_CONVERGENCE,
        MGA_SCALE,
        MGA_EPSG_OUT,
        MGA_FRAME_OUT,
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &[
        "REALIZATION_ASSUMED",
        "NONSTANDARD_ZONE",
        "OUTSIDE_AREA_OF_USE",
    ],
    model: "Transverse Mercator (South Orientated), EPSG method 9808, on WGS 84 with the Lo belt's parameters (EPSG 2046 to 2055): origin on the equator and the belt's odd central meridian, scale 1, no false westing or southing, westing and southing positive to the west and south, by Karney's series",
    accuracy: "The projection is exact to the series' nanometre across South Africa, and agrees with PROJ applying each belt's own EPSG record within a millimetre at every test place. IOGP's worked example prints a westing 1 cm larger, because its formula is a shorter series. The coordinates are taken to be on Hartebeesthoek94",
    when_to_use: "Use this to put a latitude and longitude onto the grid South African survey diagrams, general plans, and municipal datasets are drawn in. It picks the two-degree belt from the longitude, or takes one you name when a job runs across a belt edge, and returns the grid convergence and the point scale factor beside Y and X.",
    limitations: "Y is the westing and X the southing, both measured from the equator on the belt's central meridian: Y grows to the west and is negative east of the meridian, the reverse of an easting. The tool projects and does not transform: a Cape datum latitude and longitude is on the Clarke 1880 ellipsoid and must first be moved to Hartebeesthoek94. Heights are untouched; SAGEOID2010 is a separate asset.",
    references: &[G7_2_TMSO, EPSG_LO],
    examples: &[Example {
        id: "primary",
        title: "IOGP's Lo29 worked example",
        source: "IOGP Guidance Note 7-2, section 3.2.3.3: 25°43'55.302\"S 28°16'57.479\"E on Hartebeesthoek94 / Lo29 gives Y 71984.49 m, X 2847342.74 m; PROJ 9.9.0 and GeographicLib both give Y 71984.480 m, X 2847342.735 m",
        input: r#"{"lat":"25°43'55.302\"S","lon":"28°16'57.479\"E"}"#,
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "geodesy.projection.lo-inverse",
            reason: "inverse",
        },
        Related {
            id: "geodesy.projection.tm-forward",
            reason: "parent",
        },
        Related {
            id: "geodesy.utm.forward",
            reason: "alternative",
        },
    ],
    sentence: "That point is Y {westing}, X {southing} on Lo{zone}.",
    limits: &[("batchRows", 10_000)],
    run: run_lo_forward,
    ..ToolDef::BLANK
};

fn run_lo_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let standard = lo_for(lon);
    let cm = match ctx.number("zone")? {
        Some(z) => {
            let z = z.round() as i32;
            lo_check(z, "/zone")?;
            if z != standard {
                ctx.warnings.push(Warning::new(
                    "NONSTANDARD_ZONE",
                    format!(
                        "The longitude {lon:.6}° falls in Lo{standard}, and the answer is on Lo{z} as asked. Coordinates on another belt are further from its central meridian, so the point scale factor is further from 1."
                    ),
                ));
            }
            z
        }
        None => {
            lo_check(standard, "/lon")?;
            standard
        }
    };
    let g = lo_grid(cm).forward(lat, lon);
    let (y, x) = (-g.e, -g.n);
    ctx.warnings
        .push(Warning::new("REALIZATION_ASSUMED", LO_FRAME));
    lo_area_warning(ctx, lat, lon);
    if ctx.explaining() {
        ctx.step(
            "The belt",
            "the belts are two degrees wide, each about an odd meridian",
            format!("{lon:.6}° east"),
            format!("Lo{cm}, central meridian {cm}°E"),
        );
        ctx.step(
            "On the grid",
            "transverse Mercator on WGS 84, scale 1 on the central meridian, from the equator; turned to count west and south",
            format!("{lat:.6}°, {lon:.6}°"),
            format!(
                "{} m",
                gp_base::display::number(y, Precision::Decimals(3), ctx.options.format)
            ),
        );
    }
    Ok(Json::obj([
        ("zone", Json::Num(f64::from(cm))),
        ("westing", ctx.out("westing", meters(y))),
        ("southing", ctx.out("southing", meters(x))),
        ("convergence", ctx.out("convergence", deg(g.convergence))),
        ("scale_factor", Json::Num(g.k)),
        ("epsg", Json::str(lo_epsg(cm))),
        ("frame", Json::str(LO_FRAME)),
    ]))
}

pub static LO_INVERSE: ToolDef = ToolDef {
    id: "geodesy.projection.lo-inverse",
    stability: gp_base::tool::Stability::Stable,
    title: "South African Lo to latitude and longitude",
    summary: "Converts the Y and X of a South African Lo belt back to a Hartebeesthoek94 latitude and longitude, with the grid convergence and the point scale factor there.",
    aliases: &[
        "Lo to lat long",
        "South African Lo coordinates to latitude longitude",
        "read Lo westing southing",
        "Hartebeesthoek94 Lo to coordinates",
    ],
    keywords: &[
        "Lo",
        "Hartebeesthoek94",
        "South Africa",
        "south orientated",
        "transverse Mercator",
        "inverse",
        "EPSG 2053",
    ],
    inputs: &[
        Field::new(
            "zone",
            "Lo belt",
            "The belt's central meridian, an odd number from 15 to 33, like 29 for Lo29",
            Kind::Number {
                min: LO_MIN as f64,
                max: LO_MAX as f64,
            },
        )
        .required()
        .core(),
        length(
            "westing",
            "Y (westing)",
            "Metres west of the belt's central meridian, negative to the east, like 71984.480 m",
        )
        .required()
        .core(),
        length(
            "southing",
            "X (southing)",
            "Metres south of the equator, like 2847342.735 m",
        )
        .required()
        .core(),
    ],
    outputs: &[
        Field::new(
            "lat",
            "Latitude",
            "On Hartebeesthoek94, north positive",
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
            "On Hartebeesthoek94, east positive",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(9))
        .angle_range("[-180,180)"),
        MGA_CONVERGENCE,
        MGA_SCALE,
        MGA_EPSG_OUT,
        MGA_FRAME_OUT,
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["REALIZATION_ASSUMED", "OUTSIDE_AREA_OF_USE"],
    model: "The inverse Transverse Mercator (South Orientated), EPSG method 9808, on WGS 84 with the Lo belt's parameters (origin on the equator and the belt's central meridian, scale 1, no false westing or southing), by Karney's series",
    accuracy: "The inverse is the forward's own inverse rather than a separate approximation, and the round trip returns every test place to within a nanodegree. The answer is on Hartebeesthoek94",
    when_to_use: "Use this to read Y and X off a South African survey diagram, general plan, or municipal dataset and get the latitude and longitude a phone or a web map wants. Y is the westing and X the southing, so a point east of the belt's meridian has a negative Y. The coordinates do not carry their belt, so it is asked for.",
    limitations: "Y and X without their belt are ambiguous: every belt counts from its own meridian, so the same numbers name a place in each, two degrees of longitude apart. Coordinates on the older Cape datum are on the Clarke 1880 ellipsoid and are not this grid. Heights are untouched.",
    references: &[G7_2_TMSO, EPSG_LO],
    examples: &[Example {
        id: "primary",
        title: "IOGP's Lo29 worked example",
        source: "IOGP Guidance Note 7-2, section 3.2.3.3, reverse: Lo29, Y 71984.49 m, X 2847342.74 m gives 25°43'55.302\"S 28°16'57.479\"E; here at PROJ's Y 71984.4797 m, X 2847342.7352 m",
        input: r#"{"zone":29,"westing":"71984.4797 m","southing":"2847342.7352 m"}"#,
    }],
    primary_example: "primary",
    visualization: INVERSE_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.lo-forward",
            reason: "inverse",
        },
        Related {
            id: "geodesy.projection.tm-inverse",
            reason: "parent",
        },
        Related {
            id: "geodesy.utm.inverse",
            reason: "alternative",
        },
    ],
    sentence: "That is {lat}, {lon}.",
    limits: &[("batchRows", 10_000)],
    run: run_lo_inverse,
    ..ToolDef::BLANK
};

fn run_lo_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let cm = ctx
        .number("zone")?
        .ok_or_else(|| ToolError::invalid("/zone", "Name the Lo belt, 15 to 33."))?
        .round() as i32;
    lo_check(cm, "/zone")?;
    let m = unit(QT::Length, "m");
    let y = ctx.req_quantity("westing")?.to(m);
    let x = ctx.req_quantity("southing")?.to(m);
    if !y.is_finite() || !x.is_finite() {
        return Err(ToolError::invalid(
            "/westing",
            "Y and X are metres on a Lo belt.",
        ));
    }
    let grid = lo_grid(cm);
    let (lat, lon) = grid.inverse(-y, -x);
    if !lat.is_finite() || !lon.is_finite() || !(-90.0..=90.0).contains(&lat) {
        return Err(ToolError::invalid(
            "/southing",
            "That Y and X is not a place on the belt.",
        ));
    }
    let back = grid.forward(lat, lon);
    ctx.warnings
        .push(Warning::new("REALIZATION_ASSUMED", LO_FRAME));
    lo_area_warning(ctx, lat, lon);
    if ctx.explaining() {
        ctx.step(
            "Turned back",
            "the Lo axes count west and south, so the easting and northing are their negatives",
            format!("Y {y:.3} m, X {x:.3} m"),
            format!("E {:.3} m, N {:.3} m", -y, -x),
        );
        ctx.step(
            "Off the grid",
            "the inverse transverse Mercator on WGS 84, scale 1 on the belt's central meridian",
            format!("Lo{cm}"),
            format!(
                "{}°",
                gp_base::display::number(lat, Precision::Decimals(9), ctx.options.format)
            ),
        );
    }
    Ok(Json::obj([
        ("lat", ctx.out("lat", deg(lat))),
        ("lon", ctx.out("lon", deg(lon))),
        ("convergence", ctx.out("convergence", deg(back.convergence))),
        ("scale_factor", Json::Num(back.k)),
        ("epsg", Json::str(lo_epsg(cm))),
        ("frame", Json::str(LO_FRAME)),
    ]))
}

// -------------------------------------------- France (RGF93 Lambert-93, CC)

const IGN_NTG71: Reference = Reference {
    title: "Projection cartographique conique conforme de Lambert, IGN Notes techniques NT/G 71",
    issuer: "Institut national de l'information géographique et forestière (IGN), Service de géodésie et nivellement",
    year: 1995,
    edition: "1st edition, January 1995, as IGN publishes it, read 2026-11-16",
    locator: "Algorithms ALG0001 (isometric latitude), ALG0003 (geographic to Lambert), ALG0004 (Lambert to geographic), and ALG0054 (secant cone constants), and the table of constants of the Lambert projections in use in France: Lambert-93 n = 0.7256077650, c = 11 754 255.426 m, Xs = 700 000.0 m, Ys = 12 655 612.050 m",
    url: "https://data.geopf.fr/annexes/ressources/documentation/geodesie/algorithmes/NTG_71.pdf",
};

const EPSG_L93: Reference = Reference {
    title: "EPSG Geodetic Parameter Dataset (EPSG v13.102 CRS registry)",
    issuer: "IOGP, from IGN France",
    year: 2026,
    edition: "v13.102, as shipped in PROJ 9.9.0",
    locator: "RGF93 v1 / Lambert-93 (EPSG 2154): Lambert Conic Conformal (2SP) on GRS 1980, false origin 46°30'N 3°E, standard parallels 49°N and 44°N, false easting 700,000 m, false northing 6,600,000 m; RGF93 v1 / CC42 to CC50 (EPSG 3942 to 3950): false origin on the zone's parallel and 3°E, standard parallels 0.75° either side, false easting 1,700,000 m, false northing (zone − 41) × 1,000,000 + 200,000 m; each with its area of use",
    url: "https://epsg.org/crs/2154",
};

/// Lambert-93 (0) or a CC zone by its origin latitude (42 to 50).
fn l93_which(ctx: &Ctx) -> Result<i32, ToolError> {
    Ok(match ctx.choice("grid")? {
        Some(g) if g.starts_with("CC") => g[2..].parse().expect("the choice lists CC42 to CC50"),
        _ => 0,
    })
}

fn l93_grid(zone: i32) -> Lcc {
    let g = crate::gridref::grs80();
    if zone == 0 {
        Lcc::two_sp(g.a, g.f, 46.5, 3.0, 49.0, 44.0, 700_000.0, 6_600_000.0)
    } else {
        let lat0 = f64::from(zone);
        Lcc::two_sp(
            g.a,
            g.f,
            lat0,
            3.0,
            lat0 - 0.75,
            lat0 + 0.75,
            1_700_000.0,
            f64::from(zone - 41) * 1_000_000.0 + 200_000.0,
        )
    }
}

fn l93_name(zone: i32) -> String {
    if zone == 0 {
        "Lambert-93".into()
    } else {
        format!("CC{zone}")
    }
}

fn l93_epsg(zone: i32) -> String {
    if zone == 0 {
        "EPSG:2154 (RGF93 v1 / Lambert-93)".into()
    } else {
        format!("EPSG:{} (RGF93 v1 / CC{zone})", 3900 + zone)
    }
}

/// EPSG's area of use for each grid: south, west, north, east.
fn l93_area(zone: i32) -> [f64; 4] {
    match zone {
        42 => [41.31, -1.06, 43.07, 9.63],
        43 => [42.33, -1.79, 44.01, 7.65],
        44 => [42.92, -1.79, 45.0, 7.71],
        45 => [44.0, -1.46, 46.0, 7.71],
        46 => [45.0, -2.21, 47.0, 7.16],
        47 => [46.0, -4.77, 48.0, 7.63],
        48 => [47.0, -4.87, 49.0, 8.23],
        49 => [48.0, -4.87, 50.0, 8.23],
        50 => [49.0, -2.03, 51.14, 8.08],
        _ => [41.15, -9.86, 51.56, 10.38],
    }
}

fn l93_area_warning(ctx: &mut Ctx, zone: i32, lat: f64, lon: f64) {
    let [s, w, n, e] = l93_area(zone);
    if !((s..=n).contains(&lat) && (w..=e).contains(&lon)) {
        let name = l93_name(zone);
        ctx.warnings.push(Warning::new(
            "OUTSIDE_AREA_OF_USE",
            format!(
                "The point is outside the area EPSG publishes for {name}, {s}° to {n}° north and {w}° to {e}° east. The arithmetic still holds, but the grid's scale error grows quickly away from it."
            ),
        ));
    }
}

const L93_FRAME: &str = "The coordinates are taken to be on RGF93, France's datum, the European ETRS89 as realized in France. Its realizations v1, v2, and v2b differ by a few centimetres and share these grids. Coordinates on the older NTF datum are on the Clarke 1880 ellipsoid with Paris as prime meridian, and go through the NTF Lambert zones, not this grid.";

const L93_GRID_IN: Field = Field::new(
    "grid",
    "Grid",
    "Lambert-93, or a CC conic zone named by its origin latitude, like CC49 around Paris; the default is Lambert-93",
    Kind::Choice(&[
        "Lambert-93",
        "CC42",
        "CC43",
        "CC44",
        "CC45",
        "CC46",
        "CC47",
        "CC48",
        "CC49",
        "CC50",
    ]),
);

const L93_GRID_OUT: Field = Field::new(
    "grid",
    "Grid",
    "The grid the coordinates are on",
    Kind::Text { max_len: 20 },
);

pub static LAMBERT93_FORWARD: ToolDef = ToolDef {
    id: "geodesy.projection.lambert93-forward",
    stability: gp_base::tool::Stability::Stable,
    title: "Latitude and longitude to Lambert-93",
    summary: "Converts an RGF93 latitude and longitude to Lambert-93 easting and northing, France's national grid, or to one of the nine CC conic zones, with the grid convergence and the point scale factor.",
    aliases: &[
        "lat long to Lambert 93",
        "Lambert-93 converter",
        "French grid coordinates",
        "RGF93 to Lambert 93",
        "conique conforme CC49",
    ],
    keywords: &[
        "Lambert-93",
        "Lambert 93",
        "RGF93",
        "France",
        "CC49",
        "conic conformal",
        "EPSG 2154",
    ],
    inputs: &[
        Field::new(
            "lat",
            "Latitude",
            "North positive, like 48.858370",
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
            "East positive, so west of Greenwich is negative, like 2.294481",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core()
        .precision(Precision::Decimals(9))
        .angle_range("[-180,180)"),
        L93_GRID_IN,
    ],
    outputs: &[
        // The last explain step ends on the easting, which the trace gate
        // requires to be this first output.
        MGA_EASTING_OUT,
        MGA_NORTHING_OUT,
        L93_GRID_OUT,
        MGA_CONVERGENCE,
        MGA_SCALE,
        MGA_EPSG_OUT,
        MGA_FRAME_OUT,
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["REALIZATION_ASSUMED", "OUTSIDE_AREA_OF_USE"],
    model: "Lambert Conic Conformal (2SP), EPSG method 9802, on GRS 80 with the parameters EPSG registers for RGF93 / Lambert-93 (origin 46°30'N 3°E, standard parallels 49°N and 44°N, false origin 700,000 m east and 6,600,000 m north) and for CC42 to CC50 (origin on the zone's parallel and 3°E, standard parallels 0.75° either side, false origin 1,700,000 m east and (zone − 41) × 1,000,000 + 200,000 m north), in closed form",
    accuracy: "The projection is closed form and agrees with PROJ applying each grid's EPSG record, and with the constants IGN prints for Lambert-93 in NT/G 71, within a millimetre at every test place. The coordinates are taken to be on RGF93",
    when_to_use: "Use this to put a latitude and longitude onto the grid French cadastral plans, IGN maps, and public datasets are drawn in. Lambert-93 covers mainland France and Corsica in one grid; a CC zone keeps the scale error under 1 in 100,000 within its band, which is why large-scale surveys use them. The answer returns the grid convergence and the point scale factor beside the coordinates.",
    limitations: "The tool projects and does not transform: a position on the older NTF datum must first be moved to RGF93 by IGN's grid, which this tool does not carry. It covers mainland France and Corsica; the overseas departments use UTM on their own datums. Heights are untouched: RAF20 is a separate asset. Lambert-93's scale error reaches about 3 in 1,000 at the edges of France, so a ground distance needs the point scale factor.",
    references: &[IGN_NTG71, EPSG_L93],
    examples: &[Example {
        id: "primary",
        title: "The Eiffel Tower",
        source: "PROJ 9.9.0 applying EPSG:2154 (RGF93 v1 / Lambert-93): 48.858370°N 2.294481°E gives E 648235.878 N 6862268.358",
        input: r#"{"lat":48.85837,"lon":2.294481}"#,
    }],
    primary_example: "primary",
    visualization: FORWARD_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.lambert93-inverse",
            reason: "inverse",
        },
        Related {
            id: "geodesy.projection.lcc-forward",
            reason: "parent",
        },
        Related {
            id: "geodesy.projection.etrs89-forward",
            reason: "alternative",
        },
    ],
    sentence: "That point is E {easting}, N {northing} on {grid}.",
    limits: &[("batchRows", 10_000)],
    run: run_lambert93_forward,
    ..ToolDef::BLANK
};

fn run_lambert93_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let zone = l93_which(ctx)?;
    if lat <= -90.0 {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The cone opens toward the South Pole, which is at infinity on this grid.",
        )
        .at("/lat"));
    }
    let g = l93_grid(zone).forward(lat, lon);
    ctx.warnings
        .push(Warning::new("REALIZATION_ASSUMED", L93_FRAME));
    l93_area_warning(ctx, zone, lat, lon);
    let name = l93_name(zone);
    if ctx.explaining() {
        ctx.step(
            "The grid",
            "Lambert-93 for all of France, or a CC zone one degree of latitude wide",
            name.clone(),
            l93_epsg(zone),
        );
        ctx.step(
            "On the cone",
            "Lambert conic conformal on GRS 80, two standard parallels, central meridian 3°E",
            format!("{lat:.6}°, {lon:.6}°"),
            format!(
                "{} m",
                gp_base::display::number(g.e, Precision::Decimals(3), ctx.options.format)
            ),
        );
    }
    Ok(Json::obj([
        ("easting", ctx.out("easting", meters(g.e))),
        ("northing", ctx.out("northing", meters(g.n))),
        ("grid", Json::str(name)),
        ("convergence", ctx.out("convergence", deg(g.convergence))),
        ("scale_factor", Json::Num(g.k)),
        ("epsg", Json::str(l93_epsg(zone))),
        ("frame", Json::str(L93_FRAME)),
    ]))
}

pub static LAMBERT93_INVERSE: ToolDef = ToolDef {
    id: "geodesy.projection.lambert93-inverse",
    stability: gp_base::tool::Stability::Stable,
    title: "Lambert-93 to latitude and longitude",
    summary: "Converts a Lambert-93 easting and northing, or one on a CC conic zone, back to an RGF93 latitude and longitude, with the grid convergence and the point scale factor there.",
    aliases: &[
        "Lambert 93 to lat long",
        "Lambert-93 to latitude longitude",
        "read French grid coordinates",
        "CC49 to lat long",
    ],
    keywords: &[
        "Lambert-93",
        "Lambert 93",
        "RGF93",
        "France",
        "CC49",
        "inverse",
        "EPSG 2154",
    ],
    inputs: &[
        length(
            "easting",
            "Easting",
            "Metres on the grid, with its false easting, like 648235.878 m",
        )
        .required()
        .core(),
        length(
            "northing",
            "Northing",
            "Metres on the grid, with its false northing, like 6862268.358 m",
        )
        .required()
        .core(),
        L93_GRID_IN,
    ],
    outputs: &[
        Field::new(
            "lat",
            "Latitude",
            "On RGF93, north positive",
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
            "On RGF93, east positive",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(9))
        .angle_range("[-180,180)"),
        L93_GRID_OUT,
        MGA_CONVERGENCE,
        MGA_SCALE,
        MGA_EPSG_OUT,
        MGA_FRAME_OUT,
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["REALIZATION_ASSUMED", "OUTSIDE_AREA_OF_USE"],
    model: "The inverse Lambert Conic Conformal (2SP), EPSG method 9802, on GRS 80 with the grid's EPSG parameters, the latitude recovered from the isometric latitude by iteration",
    accuracy: "The round trip returns every test place to within a nanodegree, and the answer agrees with PROJ applying the grid's EPSG record. The answer is on RGF93",
    when_to_use: "Use this to read an easting and northing off a French cadastral plan, IGN map, or public dataset and get the latitude and longitude a phone or a web map wants. Lambert-93 is the default; a CC zone is named on the plan, and its northing in millions of metres is the zone number less 41, like 7.2 million for CC48.",
    limitations: "Lambert-93 and the CC zones overlap in their numbers' range, so read the grid off the plan rather than guessing it. Coordinates on the older NTF Lambert zones are another datum and another projection, and are not this grid. Heights are untouched.",
    references: &[IGN_NTG71, EPSG_L93],
    examples: &[Example {
        id: "primary",
        title: "The Eiffel Tower",
        source: "PROJ 9.9.0 applying EPSG:2154 in reverse: E 648235.878 N 6862268.358 on Lambert-93 gives 48.858370°N 2.294481°E on RGF93",
        input: r#"{"easting":"648235.8777 m","northing":"6862268.3583 m"}"#,
    }],
    primary_example: "primary",
    visualization: INVERSE_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.lambert93-forward",
            reason: "inverse",
        },
        Related {
            id: "geodesy.projection.lcc-inverse",
            reason: "parent",
        },
        Related {
            id: "geodesy.projection.etrs89-inverse",
            reason: "alternative",
        },
    ],
    sentence: "That is {lat}, {lon}.",
    limits: &[("batchRows", 10_000)],
    run: run_lambert93_inverse,
    ..ToolDef::BLANK
};

fn run_lambert93_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let zone = l93_which(ctx)?;
    let (e, n) = grid_in(ctx)?;
    if !e.is_finite() || !n.is_finite() {
        return Err(ToolError::invalid(
            "/easting",
            "The easting and northing are metres on the grid.",
        ));
    }
    let grid = l93_grid(zone);
    let (lat, lon) = grid.inverse(e, n);
    if !lat.is_finite() || !lon.is_finite() || !(-90.0..=90.0).contains(&lat) {
        return Err(ToolError::invalid(
            "/northing",
            "That easting and northing is not a place on the grid.",
        ));
    }
    let back = grid.forward(lat, lon);
    ctx.warnings
        .push(Warning::new("REALIZATION_ASSUMED", L93_FRAME));
    l93_area_warning(ctx, zone, lat, lon);
    let name = l93_name(zone);
    if ctx.explaining() {
        ctx.step(
            "The grid",
            "Lambert-93 for all of France, or a CC zone one degree of latitude wide",
            name.clone(),
            l93_epsg(zone),
        );
        ctx.step(
            "Off the cone",
            "the inverse Lambert conic conformal on GRS 80, central meridian 3°E",
            format!("E {e:.3} m, N {n:.3} m"),
            format!(
                "{}°",
                gp_base::display::number(lat, Precision::Decimals(9), ctx.options.format)
            ),
        );
    }
    Ok(Json::obj([
        ("lat", ctx.out("lat", deg(lat))),
        ("lon", ctx.out("lon", deg(lon))),
        ("grid", Json::str(name)),
        ("convergence", ctx.out("convergence", deg(back.convergence))),
        ("scale_factor", Json::Num(back.k)),
        ("epsg", Json::str(l93_epsg(zone))),
        ("frame", Json::str(L93_FRAME)),
    ]))
}

// ------------------------------------------- Europe (ETRS89-LAEA, ETRS89-LCC)

const G7_2_LAEA: Reference = Reference {
    title: "Coordinate Conversions and Transformations including Formulas, IOGP Publication 373-7-2 (Guidance Note 7-2)",
    issuer: "International Association of Oil & Gas Producers (IOGP)",
    year: 2019,
    edition: "Revised September 2019",
    locator: "Section 3.3.2, Lambert Azimuthal Equal Area, EPSG method 9820, oblique aspect, with the worked example for ETRS89 / ETRS-LAEA at 50°N 5°E giving E 3962799.45 m, N 2999718.85 m; section 3.2.1.1, Lambert Conic Conformal (2SP)",
    url: "https://idms.iogp.org/Documents/474",
};

const EPSG_ETRS: Reference = Reference {
    title: "EPSG Geodetic Parameter Dataset (EPSG v13.102 CRS registry)",
    issuer: "IOGP, from EUREF and the European Commission",
    year: 2026,
    edition: "v13.102, as shipped in PROJ 9.9.0",
    locator: "ETRS89-extended / LAEA Europe (EPSG 3035): Lambert Azimuthal Equal Area on GRS 1980, origin 52°N 10°E, false easting 4,321,000 m, false northing 3,210,000 m; ETRS89-extended / LCC Europe (EPSG 3034): Lambert Conic Conformal (2SP), origin 52°N 10°E, standard parallels 35°N and 65°N, false easting 4,000,000 m, false northing 2,800,000 m; both with Europe's area of use",
    url: "https://epsg.org/crs/3035",
};

const EU_1089: Reference = Reference {
    title: "Commission Regulation (EU) No 1089/2010 implementing Directive 2007/2/EC (INSPIRE) as regards interoperability of spatial data sets and services",
    issuer: "European Commission",
    year: 2010,
    edition: "Of 23 November 2010, as published on EUR-Lex, read 2026-11-16",
    locator: "Annex II, section 2.2.1, Grid for pan-European spatial analysis and reporting: the grid is based on ETRS89 Lambert Azimuthal Equal Area with the centre of the projection at 52°N 10°E, false easting 4 321 000 m and false northing 3 210 000 m, designated Grid_ETRS89-LAEA, with cells of 1 m to 100 000 m referenced by their lower left corner",
    url: "https://eur-lex.europa.eu/eli/reg/2010/1089/oj/eng",
};

const ETRS_FRAME: &str = "The coordinates are taken to be on ETRS89, the European frame fixed to the stable part of the Eurasian plate. A GNSS position on WGS 84 or ITRF differs from ETRS89 by about 84 cm at Dublin in 2026, growing by about 2.3 cm a year; geodesy.datum.etrf makes that step. For statistics and a European map the difference rarely matters.";

const ETRS_AREA: &str = "The point is outside the area EPSG publishes for these grids, Europe from about 36°W to 45°E and 25°N to 85°N. The arithmetic still holds, but the distortion grows quickly away from Europe.";

/// LAEA (true) or LCC (false).
fn etrs_which(ctx: &Ctx) -> Result<bool, ToolError> {
    Ok(ctx.choice("grid")? != Some("LCC Europe"))
}

fn etrs_laea() -> Laea {
    let g = crate::gridref::grs80();
    Laea::new(g.a, g.f, 52.0, 10.0, 4_321_000.0, 3_210_000.0)
}

fn etrs_lcc() -> Lcc {
    let g = crate::gridref::grs80();
    Lcc::two_sp(g.a, g.f, 52.0, 10.0, 35.0, 65.0, 4_000_000.0, 2_800_000.0)
}

fn etrs_name(laea: bool) -> &'static str {
    if laea { "LAEA Europe" } else { "LCC Europe" }
}

fn etrs_epsg(laea: bool) -> &'static str {
    if laea {
        "EPSG:3035 (ETRS89-extended / LAEA Europe)"
    } else {
        "EPSG:3034 (ETRS89-extended / LCC Europe)"
    }
}

fn etrs_area_warning(ctx: &mut Ctx, lat: f64, lon: f64) {
    if !((24.6..=84.73).contains(&lat) && (-35.58..=44.83).contains(&lon)) {
        ctx.warnings
            .push(Warning::new("OUTSIDE_AREA_OF_USE", ETRS_AREA));
    }
}

const ETRS_GRID_IN: Field = Field::new(
    "grid",
    "Grid",
    "LAEA Europe, the equal-area grid of European statistics and the INSPIRE grids, or LCC Europe, the conformal one for maps; the default is LAEA Europe",
    Kind::Choice(&["LAEA Europe", "LCC Europe"]),
);

const ETRS_H: Field = Field::new(
    "scale_meridian",
    "Scale along the meridian",
    "Grid distance over ground distance north and south at the point",
    Kind::Number {
        min: 0.0,
        max: 10.0,
    },
)
.precision(Precision::Decimals(9));

const ETRS_K: Field = Field::new(
    "scale_parallel",
    "Scale along the parallel",
    "Grid distance over ground distance east and west at the point; on LAEA it is the reciprocal of the meridian scale, near enough, so areas are true",
    Kind::Number { min: 0.0, max: 10.0 },
)
.precision(Precision::Decimals(9));

pub static ETRS89_FORWARD: ToolDef = ToolDef {
    id: "geodesy.projection.etrs89-forward",
    stability: gp_base::tool::Stability::Stable,
    title: "Latitude and longitude to LAEA Europe",
    summary: "Converts an ETRS89 latitude and longitude to the pan-European grids: LAEA Europe (EPSG 3035), the equal-area grid of European statistics and the INSPIRE grid cells, or LCC Europe (EPSG 3034) for conformal maps, with the convergence and the scales.",
    aliases: &[
        "lat long to EPSG 3035",
        "LAEA Europe converter",
        "ETRS89 LAEA coordinates",
        "INSPIRE grid coordinates",
        "lat long to LCC Europe",
    ],
    keywords: &[
        "ETRS89",
        "LAEA",
        "EPSG 3035",
        "EPSG 3034",
        "Europe",
        "equal area",
        "INSPIRE",
        "Eurostat",
    ],
    inputs: &[
        Field::new(
            "lat",
            "Latitude",
            "North positive, like 50.0",
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
            "East positive, so west of Greenwich is negative, like 5.0",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core()
        .precision(Precision::Decimals(9))
        .angle_range("[-180,180)"),
        ETRS_GRID_IN,
    ],
    outputs: &[
        // The last explain step ends on the easting, which the trace gate
        // requires to be this first output.
        MGA_EASTING_OUT,
        MGA_NORTHING_OUT,
        L93_GRID_OUT,
        MGA_CONVERGENCE,
        ETRS_H,
        ETRS_K,
        MGA_EPSG_OUT,
        MGA_FRAME_OUT,
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["REALIZATION_ASSUMED", "OUTSIDE_AREA_OF_USE"],
    model: "LAEA Europe: Lambert Azimuthal Equal Area (EPSG method 9820), oblique ellipsoidal aspect on GRS 80, origin 52°N 10°E, false easting 4,321,000 m and false northing 3,210,000 m, by Guidance Note 7-2's formulas. LCC Europe: Lambert Conic Conformal (2SP, EPSG 9802), origin 52°N 10°E, standard parallels 35°N and 65°N, false easting 4,000,000 m and false northing 2,800,000 m, in closed form. The convergence and the scales of LAEA are measured from the projection itself over 100 m chords",
    accuracy: "Both grids are closed form and agree with PROJ applying their EPSG records within a millimetre at every test place, and LAEA Europe reproduces Guidance Note 7-2's worked example to the centimetre it prints. The convergence and scales agree with PROJ's to 1e-8. The coordinates are taken to be on ETRS89",
    when_to_use: "Use LAEA Europe to place a point in the grid of European statistics: Eurostat's population grid, the INSPIRE 1 km and 10 km grid cells, and the European Environment Agency's reference grid are all squares in it, because areas are true. Use LCC Europe for a conformal map of the continent, where shapes and angles are true over small areas.",
    limitations: "LAEA keeps areas true and pays in shape: away from 52°N 10°E the meridian and parallel scales part, by up to about 2 percent at the edges of Europe, so a ground distance on it needs both scales and a direction. Neither grid is for surveying: a national grid is. The coordinates are taken to be on ETRS89; a WGS 84 or ITRF position differs from it by about 84 cm at Dublin in 2026. Heights are untouched.",
    references: &[G7_2_LAEA, EPSG_ETRS, EU_1089],
    examples: &[Example {
        id: "primary",
        title: "IOGP's LAEA Europe worked example",
        source: "IOGP Guidance Note 7-2, section 3.3.2: 50°N 5°E on ETRS89 / ETRS-LAEA gives E 3962799.45 m, N 2999718.85 m; PROJ 9.9.0 gives the same to the millimetre",
        input: r#"{"lat":50,"lon":5}"#,
    }],
    primary_example: "primary",
    visualization: FORWARD_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.etrs89-inverse",
            reason: "inverse",
        },
        Related {
            id: "geodesy.datum.etrf",
            reason: "next",
        },
        Related {
            id: "geodesy.projection.lcc-forward",
            reason: "parent",
        },
    ],
    sentence: "That point is E {easting}, N {northing} on {grid}.",
    limits: &[("batchRows", 10_000)],
    run: run_etrs89_forward,
    ..ToolDef::BLANK
};

fn run_etrs89_forward(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lat, lon) = point::read(ctx, "lat", "lon")?;
    let laea = etrs_which(ctx)?;
    let g = if laea {
        etrs_laea().forward(lat, lon)
    } else {
        if lat <= -90.0 {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "The cone opens toward the South Pole, which is at infinity on LCC Europe.",
            )
            .at("/lat"));
        }
        etrs_lcc().forward(lat, lon)
    };
    if !g.e.is_finite() || !g.n.is_finite() {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "That point is the far side of the Earth from 52°N 10°E, which LAEA Europe spreads into a circle rather than a point.",
        )
        .at("/lat"));
    }
    ctx.warnings
        .push(Warning::new("REALIZATION_ASSUMED", ETRS_FRAME));
    etrs_area_warning(ctx, lat, lon);
    let name = etrs_name(laea);
    if ctx.explaining() {
        ctx.step(
            "The grid",
            if laea {
                "Lambert azimuthal equal area on GRS 80, centred on 52°N 10°E"
            } else {
                "Lambert conic conformal on GRS 80, standard parallels 35°N and 65°N"
            },
            name.to_string(),
            etrs_epsg(laea).to_string(),
        );
        ctx.step(
            "On the grid",
            "the projection, then the false origin added",
            format!("{lat:.6}°, {lon:.6}°"),
            format!(
                "{} m",
                gp_base::display::number(g.e, Precision::Decimals(3), ctx.options.format)
            ),
        );
    }
    Ok(Json::obj([
        ("easting", ctx.out("easting", meters(g.e))),
        ("northing", ctx.out("northing", meters(g.n))),
        ("grid", Json::str(name)),
        ("convergence", ctx.out("convergence", deg(g.convergence))),
        ("scale_meridian", Json::Num(g.h)),
        ("scale_parallel", Json::Num(g.k)),
        ("epsg", Json::str(etrs_epsg(laea))),
        ("frame", Json::str(ETRS_FRAME)),
    ]))
}

pub static ETRS89_INVERSE: ToolDef = ToolDef {
    id: "geodesy.projection.etrs89-inverse",
    stability: gp_base::tool::Stability::Stable,
    title: "LAEA Europe to latitude and longitude",
    summary: "Converts an easting and northing on LAEA Europe (EPSG 3035) or LCC Europe (EPSG 3034) back to an ETRS89 latitude and longitude, with the convergence and the scales there.",
    aliases: &[
        "EPSG 3035 to lat long",
        "LAEA Europe to latitude longitude",
        "INSPIRE grid cell to lat long",
        "LCC Europe to lat long",
    ],
    keywords: &[
        "ETRS89",
        "LAEA",
        "EPSG 3035",
        "EPSG 3034",
        "Europe",
        "inverse",
        "INSPIRE",
        "Eurostat",
    ],
    inputs: &[
        length(
            "easting",
            "Easting",
            "Metres on the grid, with its false easting, like 3962799.45 m",
        )
        .required()
        .core(),
        length(
            "northing",
            "Northing",
            "Metres on the grid, with its false northing, like 2999718.85 m",
        )
        .required()
        .core(),
        ETRS_GRID_IN,
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
        L93_GRID_OUT,
        MGA_CONVERGENCE,
        ETRS_H,
        ETRS_K,
        MGA_EPSG_OUT,
        MGA_FRAME_OUT,
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["REALIZATION_ASSUMED", "OUTSIDE_AREA_OF_USE"],
    model: "The inverse of LAEA Europe (EPSG 3035) or LCC Europe (EPSG 3034) on GRS 80; the LAEA latitude is recovered from the authalic latitude by Newton's method rather than Guidance Note 7-2's series, which is truncated near a centimetre",
    accuracy: "The round trip returns every test place to within a nanodegree, and the answer agrees with PROJ applying the grid's EPSG record. The answer is on ETRS89",
    when_to_use: "Use this to read a coordinate off a European statistical grid, an INSPIRE grid cell's corner, or a Eurostat or European Environment Agency dataset, and get the latitude and longitude a web map wants. LAEA Europe is the default; LCC Europe is named where a dataset uses it.",
    limitations: "The two grids use different false origins, but their numbers overlap in range, so read the grid off the dataset rather than guessing it. A point beyond the circle LAEA Europe maps the far side of the Earth to is refused. The answer is on ETRS89; heights are untouched.",
    references: &[G7_2_LAEA, EPSG_ETRS, EU_1089],
    examples: &[Example {
        id: "primary",
        title: "IOGP's LAEA Europe worked example",
        source: "IOGP Guidance Note 7-2, section 3.3.2, reverse: E 3962799.45 m, N 2999718.85 m on ETRS89 / ETRS-LAEA gives 50°N 5°E; here at PROJ's E 3962799.4510 m, N 2999718.8532 m",
        input: r#"{"easting":"3962799.4510 m","northing":"2999718.8532 m"}"#,
    }],
    primary_example: "primary",
    visualization: INVERSE_LAYER,
    related: &[
        Related {
            id: "geodesy.projection.etrs89-forward",
            reason: "inverse",
        },
        Related {
            id: "geodesy.projection.lcc-inverse",
            reason: "parent",
        },
        Related {
            id: "geodesy.datum.etrf",
            reason: "next",
        },
    ],
    sentence: "That is {lat}, {lon}.",
    limits: &[("batchRows", 10_000)],
    run: run_etrs89_inverse,
    ..ToolDef::BLANK
};

fn run_etrs89_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let laea = etrs_which(ctx)?;
    let (e, n) = grid_in(ctx)?;
    if !e.is_finite() || !n.is_finite() {
        return Err(ToolError::invalid(
            "/easting",
            "The easting and northing are metres on the grid.",
        ));
    }
    let off = || {
        ToolError::invalid(
            "/northing",
            "That easting and northing is not a place on the grid: it is beyond the circle the far side of the Earth maps to.",
        )
    };
    let (lat, lon, back) = if laea {
        let p = etrs_laea();
        let (lat, lon) = p.inverse(e, n).ok_or_else(off)?;
        (lat, lon, p.forward(lat, lon))
    } else {
        let p = etrs_lcc();
        let (lat, lon) = p.inverse(e, n);
        (lat, lon, p.forward(lat, lon))
    };
    if !lat.is_finite() || !lon.is_finite() || !(-90.0..=90.0).contains(&lat) {
        return Err(off());
    }
    ctx.warnings
        .push(Warning::new("REALIZATION_ASSUMED", ETRS_FRAME));
    etrs_area_warning(ctx, lat, lon);
    let name = etrs_name(laea);
    if ctx.explaining() {
        ctx.step(
            "The grid",
            "LAEA Europe or LCC Europe, both on GRS 80 about 52°N 10°E",
            name.to_string(),
            etrs_epsg(laea).to_string(),
        );
        ctx.step(
            "Off the grid",
            "the false origin taken off, then the inverse projection",
            format!("E {e:.3} m, N {n:.3} m"),
            format!(
                "{}°",
                gp_base::display::number(lat, Precision::Decimals(9), ctx.options.format)
            ),
        );
    }
    Ok(Json::obj([
        ("lat", ctx.out("lat", deg(lat))),
        ("lon", ctx.out("lon", deg(lon))),
        ("grid", Json::str(name)),
        ("convergence", ctx.out("convergence", deg(back.convergence))),
        ("scale_meridian", Json::Num(back.h)),
        ("scale_parallel", Json::Num(back.k)),
        ("epsg", Json::str(etrs_epsg(laea))),
        ("frame", Json::str(ETRS_FRAME)),
    ]))
}
