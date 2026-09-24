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
    self, Albers, Azimuthal, AzimuthalProj, EquidistantCylindrical, Grid, Hotine, Lcc,
    Orthographic, PolarStereo, TmGrid, WebMercator,
};
use gp_geo::tmexact::TmExactGrid;

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
    ..ToolDef::BLANK
};

fn run_tm_exact_inverse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (x, y) = grid_in(ctx)?;
    let p = tm_exact_grid(ctx)?;
    inverse_json(ctx, p.inverse(x, y))
}
