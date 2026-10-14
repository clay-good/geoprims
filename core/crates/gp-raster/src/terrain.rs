//! Terrain derivatives on a 3 x 3 window (raster/terrain-analysis, "Slope,
//! aspect, hillshade, and contours"): slope and aspect by Horn's method,
//! hillshade from them, and the ruggedness measures read beside them.
//!
//! The window is the nine elevations around the cell, which is what every
//! implementation of these measures works from, including GDAL's. Cell size is
//! given in meters, or in degrees with a latitude: a degree of longitude is
//! shorter than a degree of latitude everywhere but the equator, and a slope
//! computed without that correction is wrong by the cosine of the latitude.

use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{
    Assumption, Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef,
};
use gp_base::units::{self, Quantity as QT};
use libm::{atan, atan2, cos, sin, sqrt};

pub const HORN: Reference = Reference {
    title: "Hill shading and the reflectance map",
    issuer: "Horn, B. K. P., Proceedings of the IEEE",
    year: 1981,
    edition: "Volume 69, issue 1, pages 14-47",
    locator: "The eight-neighbor gradient estimate used for slope, aspect, and hill shading",
    url: "https://doi.org/10.1109/PROC.1981.11918",
};
const GDAL_HILLSHADE: Reference = Reference {
    title: "gdal raster hillshade",
    issuer: "GDAL/OGR contributors, Open Source Geospatial Foundation",
    year: 2026,
    edition: "GDAL 3.13.3 documentation",
    locator: "Options: z-factor 1, azimuth 315°, altitude 45°, and Horn gradient defaults",
    url: "https://gdal.org/en/stable/programs/gdal_raster_hillshade.html",
};
pub const RILEY: Reference = Reference {
    title: "A terrain ruggedness index that quantifies topographic heterogeneity",
    issuer: "Riley, S. J., DeGloria, S. D., and Elliot, R., Intermountain Journal of Sciences",
    year: 1999,
    edition: "Volume 5, issues 1-4, pages 23-27",
    locator: "TRI as the root of the summed squared differences from the center cell, per the erratum printed with the reprint, which corrects the code in the manuscript",
    url: "https://download.osgeo.org/qgis/doc/reference-docs/Terrain_Ruggedness_Index.pdf",
};
pub const WILSON: Reference = Reference {
    title: "Multiscale terrain analysis of multibeam bathymetry data for habitat mapping on the continental slope",
    issuer: "Wilson, M. F. J., O'Connell, B., Brown, C., Guinan, J. C., and Grehan, A. J., Marine Geodesy",
    year: 2007,
    edition: "Volume 30, issues 1-2, pages 3-35",
    locator: "Topographic position index and roughness on a 3 x 3 window",
    url: "https://doi.org/10.1080/01490410701295962",
};

const WGS84_A: f64 = 6_378_137.0;
const WGS84_F: f64 = 1.0 / 298.257_223_563;

/// The nine elevations, row by row from the north, with the cell sizes already
/// in meters: a degree-based grid is converted on the way in.
struct Window {
    z: [f64; 9],
    dx: f64,
    dy: f64,
}

fn q(v: f64, qt: QT, s: &str) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(qt, s).expect("registered unit"),
    }
}

/// Meters per degree of latitude and of longitude on WGS 84 at `lat`.
pub(crate) fn degree_lengths(lat: f64) -> (f64, f64) {
    let e2 = WGS84_F * (2.0 - WGS84_F);
    let s = sin(lat.to_radians());
    let w = sqrt(1.0 - e2 * s * s);
    let m = WGS84_A * (1.0 - e2) / (w * w * w); // meridian radius
    let n = WGS84_A / w; // prime vertical radius
    (m.to_radians(), n.to_radians() * cos(lat.to_radians()))
}

fn read_window(ctx: &mut Ctx) -> Result<Window, ToolError> {
    let rows = ctx.rows("elevations")?;
    if rows.len() != 3 {
        return Err(ToolError::invalid(
            "/elevations",
            format!(
                "Give three rows of three elevations, north row first; this has {}.",
                rows.len()
            ),
        ));
    }
    let mut z = [0.0; 9];
    for (i, r) in rows.iter().enumerate() {
        let text = r.get("row").and_then(|v| v.as_str()).ok_or_else(|| {
            ToolError::invalid(
                &format!("/elevations/{i}/row"),
                "Each row is three elevations, like 101.2, 100.6, 100.2.",
            )
        })?;
        let parts: Vec<&str> = text
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .collect();
        if parts.len() != 3 {
            return Err(ToolError::invalid(
                &format!("/elevations/{i}/row"),
                format!(
                    "Row {} has {} elevations; each row has three.",
                    i + 1,
                    parts.len()
                ),
            ));
        }
        for (j, p) in parts.iter().enumerate() {
            z[i * 3 + j] = p.parse::<f64>().map_err(|_| {
                ToolError::invalid(
                    &format!("/elevations/{i}/row"),
                    format!("{p} is not a number."),
                )
            })?;
        }
    }
    // Cell size: meters outright, or degrees with the latitude that fixes what
    // a degree is worth here.
    let lat = ctx
        .quantity("lat")?
        .map(|v| v.to(units::by_symbol(QT::Angle, "deg").expect("deg")));
    let (dx, dy) = match (ctx.quantity("cell_size")?, ctx.number("cell_degrees")?) {
        (Some(m), None) => {
            let m = m.base();
            (m, m)
        }
        (None, Some(deg)) => {
            let Some(lat) = lat else {
                return Err(ToolError::invalid(
                    "/lat",
                    "A cell size in degrees needs the latitude: a degree of longitude is shorter away from the equator.",
                ));
            };
            let (per_lat, per_lon) = degree_lengths(lat);
            (deg * per_lon, deg * per_lat)
        }
        (Some(_), Some(_)) => {
            return Err(ToolError::invalid(
                "/cell_size",
                "Give the cell size in meters or in degrees, not both.",
            ));
        }
        (None, None) => {
            return Err(ToolError::invalid(
                "/cell_size",
                "Give the cell size, in meters, or in degrees with the latitude.",
            ));
        }
    };
    if dx <= 0.0 || dy <= 0.0 {
        return Err(ToolError::invalid(
            "/cell_size",
            "The cell size must be positive.",
        ));
    }
    Ok(Window { z, dx, dy })
}

/// Horn's gradients (dz/dx, dz/dy) for the center of the window.
fn gradients(w: &Window) -> (f64, f64) {
    let z = &w.z;
    // a b c / d e f / g h i, with a at the north-west.
    let dzdx = ((z[2] + 2.0 * z[5] + z[8]) - (z[0] + 2.0 * z[3] + z[6])) / (8.0 * w.dx);
    let dzdy = ((z[6] + 2.0 * z[7] + z[8]) - (z[0] + 2.0 * z[1] + z[2])) / (8.0 * w.dy);
    (dzdx, dzdy)
}

/// Horn's gradients with the vertical exaggeration applied, and the window.
fn scaled_gradients(ctx: &mut Ctx) -> Result<(Window, f64, f64), ToolError> {
    let w = read_window(ctx)?;
    let zf = ctx.number("z_factor")?.unwrap_or(1.0);
    let (dzdx, dzdy) = gradients(&w);
    Ok((w, zf * dzdx, zf * dzdy))
}

/// Aspect as GDAL reports it: degrees clockwise from north, downslope; none
/// where the window is flat, with a warning, since flat ground faces nowhere.
fn aspect_of(ctx: &mut Ctx, gx: f64, gy: f64) -> Option<f64> {
    if gx == 0.0 && gy == 0.0 {
        ctx.warnings.push(Warning::new(
            "FLAT_CELL",
            "The window has no gradient, so it has no aspect; flat ground faces no direction.",
        ));
        return None;
    }
    let a = 90.0 - atan2(gy, -gx).to_degrees();
    Some(if a < 0.0 {
        a + 360.0
    } else if a >= 360.0 {
        a - 360.0
    } else {
        a
    })
}

fn facing(aspect: Option<f64>) -> &'static str {
    aspect.map_or("flat", |a| {
        [
            "north",
            "north-east",
            "east",
            "south-east",
            "south",
            "south-west",
            "west",
            "north-west",
        ][(((a + 22.5) % 360.0) / 45.0) as usize]
    })
}

/// Hillshade, as gdaldem computes it, from the sun's position: the cosine
/// of the angle to the sun scaled to 1 to 255 (0 is gdaldem's no-data),
/// and 1 where the cell faces away from the sun. Rounded, as gdaldem's byte.
fn hillshade(ctx: &mut Ctx, gx: f64, gy: f64) -> Result<f64, ToolError> {
    let az = ctx.quantity("sun_azimuth")?.map_or(315.0, |v| {
        v.to(units::by_symbol(QT::Angle, "deg").expect("deg"))
    });
    let alt = ctx.quantity("sun_altitude")?.map_or(45.0, |v| {
        v.to(units::by_symbol(QT::Angle, "deg").expect("deg"))
    });
    let zenith = (90.0 - alt).to_radians();
    let slope_rad = atan(sqrt(gx * gx + gy * gy));
    let aspect_rad = if gx == 0.0 && gy == 0.0 {
        0.0
    } else {
        atan2(gy, -gx)
    };
    let sun = (90.0 - az).to_radians();
    let cang = cos(zenith) * cos(slope_rad) + sin(zenith) * sin(slope_rad) * cos(sun - aspect_rad);
    Ok(if cang <= 0.0 { 1.0 } else { 1.0 + 254.0 * cang }.round())
}

fn run_slope(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (w, gx, gy) = scaled_gradients(ctx)?;
    let rise = sqrt(gx * gx + gy * gy);
    let slope = atan(rise).to_degrees();
    let aspect = aspect_of(ctx, gx, gy);
    let shade = hillshade(ctx, gx, gy)?;
    let mut out = vec![
        ("slope", ctx.out("slope", q(slope, QT::Angle, "deg"))),
        ("slope_percent", Json::Num(rise * 100.0)),
        ("hillshade", Json::Num(shade)),
        (
            "cell_size_east",
            ctx.out("cell_size_east", q(w.dx, QT::Length, "m")),
        ),
        (
            "cell_size_north",
            ctx.out("cell_size_north", q(w.dy, QT::Length, "m")),
        ),
    ];
    if let Some(a) = aspect {
        out.push(("aspect", ctx.out("aspect", q(a, QT::Angle, "deg"))));
    }
    out.push(("aspect_text", Json::str(facing(aspect))));
    Ok(Json::obj(out))
}

/// TRI both ways, TPI, and roughness for the window's center.
struct Rugged {
    tri: f64,
    tri_mean: f64,
    tpi: f64,
    roughness: f64,
}

fn rugged(w: &Window) -> Rugged {
    let center = w.z[4];
    let neighbors: Vec<f64> = (0..9).filter(|i| *i != 4).map(|i| w.z[i]).collect();
    // Two indices share the name TRI. Riley's own, as the erratum printed with
    // the paper corrects it, is the root of the summed squared differences;
    // the mean absolute difference is the other one in common use, which GDAL
    // offers as its Wilson algorithm. They differ by about a factor of three
    // on the same ground, so both are reported rather than one picked.
    let tri = sqrt(
        neighbors
            .iter()
            .map(|z| (z - center) * (z - center))
            .sum::<f64>(),
    );
    let tri_mean = neighbors.iter().map(|z| (z - center).abs()).sum::<f64>() / 8.0;
    let tpi = center - neighbors.iter().sum::<f64>() / 8.0;
    let (lo, hi) =
        w.z.iter()
            .fold((f64::MAX, f64::MIN), |(a, b), z| (a.min(*z), b.max(*z)));
    Rugged {
        tri,
        tri_mean,
        tpi,
        roughness: hi - lo,
    }
}

fn position(tpi: f64) -> &'static str {
    if tpi > 0.0 {
        "above its neighbours: a ridge, a summit, or a spur"
    } else if tpi < 0.0 {
        "below its neighbours: a valley, a pit, or a hollow"
    } else {
        "level with its neighbours"
    }
}

fn run_ruggedness(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let r = rugged(&read_window(ctx)?);
    Ok(Json::obj([
        ("tri", ctx.out("tri", q(r.tri, QT::Length, "m"))),
        (
            "tri_mean",
            ctx.out("tri_mean", q(r.tri_mean, QT::Length, "m")),
        ),
        ("tpi", ctx.out("tpi", q(r.tpi, QT::Length, "m"))),
        (
            "roughness",
            ctx.out("roughness", q(r.roughness, QT::Length, "m")),
        ),
        ("position", Json::str(position(r.tpi))),
    ]))
}

fn run_slope_only(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (_, gx, gy) = scaled_gradients(ctx)?;
    let rise = sqrt(gx * gx + gy * gy);
    Ok(Json::obj([
        (
            "slope",
            ctx.out("slope", q(atan(rise).to_degrees(), QT::Angle, "deg")),
        ),
        ("slope_percent", Json::Num(rise * 100.0)),
    ]))
}

fn run_aspect_only(ctx: &mut Ctx) -> Result<Json, ToolError> {
    // Aspect does not depend on the vertical exaggeration.
    let (dzdx, dzdy) = gradients(&read_window(ctx)?);
    let aspect = aspect_of(ctx, dzdx, dzdy);
    let mut out = vec![("aspect_text", Json::str(facing(aspect)))];
    if let Some(a) = aspect {
        out.push(("aspect", ctx.out("aspect", q(a, QT::Angle, "deg"))));
    }
    Ok(Json::obj(out))
}

fn run_hillshade_only(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (_, gx, gy) = scaled_gradients(ctx)?;
    let shade = hillshade(ctx, gx, gy)?;
    Ok(Json::obj([("hillshade", Json::Num(shade))]))
}

fn run_tri(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let r = rugged(&read_window(ctx)?);
    Ok(Json::obj([
        ("tri", ctx.out("tri", q(r.tri, QT::Length, "m"))),
        (
            "tri_mean",
            ctx.out("tri_mean", q(r.tri_mean, QT::Length, "m")),
        ),
    ]))
}

fn run_tpi(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let r = rugged(&read_window(ctx)?);
    Ok(Json::obj([
        ("tpi", ctx.out("tpi", q(r.tpi, QT::Length, "m"))),
        ("position", Json::str(position(r.tpi))),
    ]))
}

const ROW: &[Field] = &[Field::new(
    "row",
    "Elevations",
    "Three elevations across, like 101.2, 100.6, 100.2",
    Kind::Text { max_len: 120 },
)];

const WINDOW_INPUTS: [Field; 4] = [
    Field::new(
        "elevations",
        "Elevation window",
        "Three rows of three elevations in meters, north row first, each like 101.2, 100.6, 100.2",
        Kind::List {
            items: ROW,
            min: 3,
            max: 3,
        },
    )
    .required()
    .core(),
    Field::new(
        "cell_size",
        "Cell size",
        "The ground size of one cell, like 30 m",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .core(),
    Field::new(
        "cell_degrees",
        "Cell size in degrees",
        "For a geographic grid, like 0.000277778 (one arc second)",
        Kind::Number {
            min: 0.0,
            max: 10.0,
        },
    )
    .core(),
    Field::new(
        "lat",
        "Latitude",
        "Needed with a cell size in degrees, like 60",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .core(),
];

pub static SLOPE: ToolDef = ToolDef {
    id: "raster.terrain.slope-aspect",
    version: "1.1.0",
    title: "Slope, aspect, and hillshade",
    summary: "Slope, aspect, and hillshade for the center of a 3 x 3 elevation window by Horn's method, with the cell size corrected for latitude on a degree-based grid.",
    aliases: &["terrain slope from DEM", "slope and aspect"],
    keywords: &[
        "slope",
        "aspect",
        "hillshade",
        "terrain",
        "DEM",
        "Horn",
        "gradient",
        "gdaldem",
    ],
    inputs: &[
        WINDOW_INPUTS[0],
        WINDOW_INPUTS[1],
        WINDOW_INPUTS[2],
        WINDOW_INPUTS[3],
        Field::new(
            "z_factor",
            "Vertical exaggeration",
            "1 unless the elevations are in different units from the cell size",
            Kind::Number {
                min: 0.0,
                max: 1000.0,
            },
        ),
        Field::new(
            "sun_azimuth",
            "Sun azimuth",
            "For the hillshade, like 315 deg (the cartographic default)",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        ),
        Field::new(
            "sun_altitude",
            "Sun altitude",
            "For the hillshade, like 45 deg",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        ),
    ],
    outputs: &[
        Field::new(
            "slope",
            "Slope",
            "From the horizontal",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "slope_percent",
            "Slope, percent",
            "Rise over run as a percentage",
            Kind::Number { min: 0.0, max: 1e9 },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "aspect",
            "Aspect",
            "Downslope direction, clockwise from north",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "aspect_text",
            "Facing",
            "The aspect in words, or flat",
            Kind::Text { max_len: 16 },
        ),
        Field::new(
            "hillshade",
            "Hillshade",
            "1 to 255, as gdaldem computes it",
            Kind::Number {
                min: 1.0,
                max: 255.0,
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "cell_size_east",
            "Cell size east",
            "The east-west ground size used",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "cell_size_north",
            "Cell size north",
            "The north-south ground size used",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(3)),
    ],
    errors: &[ErrorCode::InvalidInput],
    stability: gp_base::tool::Stability::Stable,
    warnings: &["FLAT_CELL", "UNIT_ASSUMED"],
    model: "Horn's (1981) eight-neighbor gradients: dz/dx = ((c + 2f + i) - (a + 2d + g)) / 8dx, dz/dy from the rows; slope = atan(z x hypot), aspect clockwise from north, hillshade from the sun's zenith and azimuth as gdaldem computes it",
    accuracy: "Exact evaluation of Horn's formulas. On a degree-based grid the cell sizes come from the WGS 84 meridian and prime-vertical radii at the latitude given.",
    when_to_use: "Use this to see what a DEM says about one cell, and to check a raster pipeline against a hand-worked case: the nine elevations in, the slope, the aspect, and the hillshade out, with the cell sizes actually used. It is also the way to see how much a degree-based grid's latitude correction matters, which is where slope from an unprojected DEM usually goes wrong.",
    limitations: "It works on the nine elevations you give, not on a raster: a whole-DEM version belongs with the GeoTIFF tools. Horn's method smooths, so a slope from a coarse DEM is gentler than the ground; and the answer is only as good as the elevation model, which for a surface model includes buildings and canopy rather than the ground under them.",
    references: &[HORN, GDAL_HILLSHADE],
    examples: &[Example {
        id: "primary",
        title: "A 30 m window falling to the south-east",
        input: r#"{"elevations":[{"row":"101.2, 100.6, 100.2"},{"row":"100.4, 99.8, 99.2"},{"row":"99.6, 99.0, 98.4"}],"cell_size":"30 m"}"#,
        source: "Horn's formulas worked independently in Python (tools/vectors/gen_raster.py)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "raster.terrain.ruggedness",
            reason: "next",
        },
        Related {
            id: "survey.earthwork.profile-grades",
            reason: "alternative",
        },
        Related {
            id: "navigation.los.visibility",
            reason: "next",
        },
    ],
    assumptions: &[
        Assumption {
            name: "WGS 84 semi-major axis, for cell sizes given in degrees",
            value: "6378137",
            unit: "m",
            source: "nga-wgs84",
        },
        Assumption {
            name: "WGS 84 inverse flattening, for cell sizes given in degrees",
            value: "298.257223563",
            unit: "1",
            source: "nga-wgs84",
        },
        Assumption {
            name: "Default hillshade vertical exaggeration",
            value: "1",
            unit: "1",
            source: "gdal-hillshade",
        },
        Assumption {
            name: "Default hillshade light azimuth",
            value: "315",
            unit: "deg",
            source: "gdal-hillshade",
        },
        Assumption {
            name: "Default hillshade light altitude",
            value: "45",
            unit: "deg",
            source: "gdal-hillshade",
        },
    ],
    sentence: "The slope is {slope} ({slope_percent}%), facing {aspect_text}.",
    limits: &[("batchRows", 10_000)],
    run: run_slope,
    ..ToolDef::BLANK
};

pub static RUGGEDNESS: ToolDef = ToolDef {
    id: "raster.terrain.ruggedness",
    title: "Terrain ruggedness and position",
    summary: "The terrain ruggedness index, topographic position index, and roughness for the center of a 3 x 3 elevation window.",
    aliases: &["terrain roughness", "landform position"],
    keywords: &[
        "TRI",
        "TPI",
        "ruggedness",
        "roughness",
        "terrain position",
        "ridge",
        "valley",
    ],
    inputs: &[
        WINDOW_INPUTS[0],
        WINDOW_INPUTS[1],
        WINDOW_INPUTS[2],
        WINDOW_INPUTS[3],
    ],
    outputs: &[
        Field::new(
            "tri",
            "Terrain ruggedness index (Riley)",
            "Root of the summed squared differences from the center cell",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "tri_mean",
            "Ruggedness, mean absolute (Wilson)",
            "Mean absolute difference from the center cell: the other index of the same name",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "tpi",
            "Topographic position index",
            "The center cell less the mean of its neighbours",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "roughness",
            "Roughness",
            "Highest less lowest in the window",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "position",
            "Position",
            "Where the cell stands against its neighbours",
            Kind::Text { max_len: 64 },
        ),
    ],
    errors: &[ErrorCode::InvalidInput],
    stability: gp_base::tool::Stability::Stable,
    warnings: &["UNIT_ASSUMED"],
    model: "TRI as the root of the summed squared differences from the center cell (Riley and others 1999, as the erratum printed with the paper corrects it) and also as the mean absolute difference (Wilson and others 2007, which GDAL offers as its Wilson algorithm); TPI as the center less the mean of its eight neighbours; roughness as the range across the window",
    accuracy: "Exact arithmetic on the elevations given.",
    when_to_use: "Use this to describe the shape of the ground around a cell rather than its steepness: ruggedness for how broken the terrain is, the position index for whether the cell sits above its surroundings (a ridge or summit) or below them (a valley or hollow), and roughness for the range within the window. They are the measures behind habitat, accessibility, and landform classification.",
    limitations: "Two different indices are called TRI, and on the same ground they differ by about a factor of three: Riley's own root of summed squares, and the mean absolute difference that many tools report under the name. Both are given, so a value can be matched to whichever one a paper or a dataset used. They all depend on the cell size and on the DEM's own smoothing: the same ground gives smaller numbers on a coarser grid, so values are comparable only within one resolution. The position index is a single scale — a cell can sit above its immediate neighbours and below the wider landscape — and none of these say anything about the direction the ground faces.",
    references: &[RILEY, WILSON],
    examples: &[Example {
        id: "primary",
        title: "A cell on a slope",
        input: r#"{"elevations":[{"row":"101.2, 100.6, 100.2"},{"row":"100.4, 99.8, 99.2"},{"row":"99.6, 99.0, 98.4"}],"cell_size":"30 m"}"#,
        source: "The definitions worked independently in Python (tools/vectors/gen_raster.py)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "raster.terrain.slope-aspect",
            reason: "parent",
        },
        Related {
            id: "survey.earthwork.profile-grades",
            reason: "alternative",
        },
        Related {
            id: "raster.index.ndvi",
            reason: "alternative",
        },
    ],
    assumptions: &[
        Assumption {
            name: "WGS 84 semi-major axis, for cell sizes given in degrees",
            value: "6378137",
            unit: "m",
            source: "nga-wgs84",
        },
        Assumption {
            name: "WGS 84 inverse flattening, for cell sizes given in degrees",
            value: "298.257223563",
            unit: "1",
            source: "nga-wgs84",
        },
    ],
    sentence: "Ruggedness is {tri} by Riley, or {tri_mean} as a mean difference, and the cell sits {position}.",
    limits: &[("batchRows", 10_000)],
    run: run_ruggedness,
    ..ToolDef::BLANK
};

const fn parent(id: &'static str) -> Related {
    Related {
        id,
        reason: "parent",
    }
}

const fn sibling(id: &'static str) -> Related {
    Related {
        id,
        reason: "alternative",
    }
}

const TABLE: &[Layer] = &[Layer {
    kind: "table-only",
    map: &[],
}];

pub static SLOPE_ONLY: ToolDef = ToolDef {
    stability: gp_base::tool::Stability::Stable,
    id: "raster.terrain.slope",
    title: "DEM slope by Horn's method",
    summary: "The slope at the center of a 3 x 3 elevation window, in degrees and percent, by Horn's method as gdaldem computes it.",
    aliases: &["slope calculator", "slope from elevations"],
    keywords: &[
        "slope",
        "gradient",
        "DEM",
        "Horn",
        "gdaldem",
        "percent grade",
    ],
    inputs: &[
        WINDOW_INPUTS[0],
        WINDOW_INPUTS[1],
        WINDOW_INPUTS[2],
        WINDOW_INPUTS[3],
        SLOPE.inputs[4],
    ],
    outputs: &[SLOPE.outputs[0], SLOPE.outputs[1]],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED"],
    model: "Horn's (1981) eight-neighbor gradients, slope = atan(z x hypot(dz/dx, dz/dy)), as in raster.terrain.slope-aspect",
    accuracy: SLOPE.accuracy,
    when_to_use: "Use this when the slope is all you need from a DEM cell: for a grade limit, a site that has to stay under a slope for a building or a drone landing, or a check of one cell in a slope raster. Enter the nine elevations around the cell and the cell size. The answer is the slope in degrees and in percent.",
    limitations: "It works on the nine elevations you give, not on a raster. Horn's method smooths, so a slope from a coarse DEM is gentler than the ground, and a surface model includes buildings and canopy rather than the ground under them.",
    references: &[HORN],
    examples: &[Example {
        id: "primary",
        title: "A 10 m window on a hillside",
        input: r#"{"elevations":[{"row":"250.0, 252.5, 255.1"},{"row":"248.2, 251.0, 254.9"},{"row":"246.0, 249.3, 253.3"}],"cell_size":"10 m"}"#,
        source: "Horn's formulas in Python, checked against gdaldem 3.13.3 (tools/vectors/gen_terrain_views.py)",
    }],
    primary_example: "primary",
    visualization: TABLE,
    related: &[
        parent("raster.terrain.slope-aspect"),
        sibling("raster.terrain.aspect"),
        sibling("survey.earthwork.profile-grades"),
    ],
    assumptions: RUGGEDNESS.assumptions,
    sentence: "The slope is {slope}, or {slope_percent}%.",
    limits: &[("batchRows", 10_000)],
    run: run_slope_only,
    ..ToolDef::BLANK
};

pub static ASPECT: ToolDef = ToolDef {
    stability: gp_base::tool::Stability::Stable,
    id: "raster.terrain.aspect",
    title: "DEM aspect, the way the ground faces",
    summary: "The aspect at the center of a 3 x 3 elevation window: the downslope direction clockwise from north, by Horn's method as gdaldem computes it.",
    aliases: &["aspect calculator", "which way a slope faces"],
    keywords: &[
        "aspect",
        "facing",
        "downslope",
        "DEM",
        "Horn",
        "gdaldem",
        "north-facing",
    ],
    inputs: &[
        WINDOW_INPUTS[0],
        WINDOW_INPUTS[1],
        WINDOW_INPUTS[2],
        WINDOW_INPUTS[3],
    ],
    outputs: &[SLOPE.outputs[3], SLOPE.outputs[2]],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["FLAT_CELL", "UNIT_ASSUMED"],
    model: "Horn's (1981) eight-neighbor gradients, aspect = 90 - atan2(dz/dy, -dz/dx) in degrees, clockwise from north and downslope, as in raster.terrain.slope-aspect; none where both gradients are zero",
    accuracy: SLOPE.accuracy,
    when_to_use: "Use this to find which way a piece of ground faces: for sun on a solar site or a garden, snow and ice that lingers on north-facing slopes, or a check of one cell in an aspect raster. Enter the nine elevations around the cell and the cell size. The answer is the compass direction the ground falls toward.",
    limitations: "It works on the nine elevations you give, not on a raster. On nearly flat ground the aspect swings with small errors in the elevations, so treat it as unreliable under about half a degree of slope; a window with no slope at all has no aspect and is reported as flat.",
    references: &[HORN],
    examples: &[Example {
        id: "primary",
        title: "A 10 m window falling to the north-east",
        input: r#"{"elevations":[{"row":"500, 498, 495"},{"row":"503, 500, 497"},{"row":"507, 504, 500"}],"cell_size":"10 m"}"#,
        source: "Horn's formulas in Python, checked against gdaldem 3.13.3 (tools/vectors/gen_terrain_views.py)",
    }],
    primary_example: "primary",
    visualization: TABLE,
    related: &[
        parent("raster.terrain.slope-aspect"),
        sibling("raster.terrain.slope"),
        sibling("raster.terrain.hillshade"),
    ],
    assumptions: RUGGEDNESS.assumptions,
    sentence: "The ground falls toward the {aspect_text}.",
    limits: &[("batchRows", 10_000)],
    run: run_aspect_only,
    ..ToolDef::BLANK
};

pub static HILLSHADE: ToolDef = ToolDef {
    stability: gp_base::tool::Stability::Stable,
    id: "raster.terrain.hillshade",
    title: "Hillshade value for one DEM cell",
    summary: "The hillshade brightness, 1 to 255, at the center of a 3 x 3 elevation window for a sun at a given azimuth and altitude, as gdaldem computes it.",
    aliases: &["hillshade", "shaded relief value"],
    keywords: &[
        "hillshade",
        "shaded relief",
        "illumination",
        "sun azimuth",
        "sun altitude",
        "gdaldem",
        "DEM",
    ],
    inputs: &[
        WINDOW_INPUTS[0],
        WINDOW_INPUTS[1],
        WINDOW_INPUTS[2],
        WINDOW_INPUTS[3],
        SLOPE.inputs[4],
        SLOPE.inputs[5],
        SLOPE.inputs[6],
    ],
    outputs: &[SLOPE.outputs[4]],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED"],
    model: "gdaldem's hillshade: 1 + 254 (cos Z cos S + sin Z sin S cos(A_sun - A)), rounded, and 1 where that cosine is zero or negative, from Horn's slope S and aspect A and the sun's zenith Z",
    accuracy: "Exact evaluation of gdaldem's formula; it matches gdaldem 3.13.3 on all 1,452 windows of the GDAL agreement test",
    when_to_use: "Use this to check one pixel of a shaded-relief map, or to see how the sun's direction changes the shading of a slope: a lower sun or one from another side brings out different ridges. Enter the nine elevations, the cell size, and if you like the sun's azimuth and altitude; the defaults are the cartographic 315° and 45°.",
    limitations: "It works on the nine elevations you give, not on a raster. The value is a picture brightness, not a measure of real sunlight: it ignores shadows cast by other terrain, the atmosphere, and the time of day.",
    references: &[HORN, GDAL_HILLSHADE],
    examples: &[Example {
        id: "primary",
        title: "A 5 m window lit from the west at 30°",
        input: r#"{"elevations":[{"row":"10, 12, 11"},{"row":"9, 13, 14"},{"row":"8.5, 11.5, 12.5"}],"cell_size":"5 m","sun_azimuth":"270 deg","sun_altitude":"30 deg"}"#,
        source: "gdaldem 3.13.3 hillshade, matched by Horn's formulas in Python (tools/vectors/gen_terrain_views.py)",
    }],
    primary_example: "primary",
    visualization: TABLE,
    related: &[
        parent("raster.terrain.slope-aspect"),
        sibling("raster.terrain.aspect"),
        sibling("raster.terrain.contours"),
    ],
    assumptions: SLOPE.assumptions,
    sentence: "The hillshade value is {hillshade} on a scale of 1 to 255.",
    limits: &[("batchRows", 10_000)],
    run: run_hillshade_only,
    ..ToolDef::BLANK
};

pub static ROUGHNESS_TRI: ToolDef = ToolDef {
    stability: gp_base::tool::Stability::Stable,
    id: "raster.terrain.roughness-tri",
    title: "Terrain ruggedness index (TRI)",
    summary: "The terrain ruggedness index at the center of a 3 x 3 elevation window, both as Riley's root of summed squares and as the mean absolute difference.",
    aliases: &["TRI calculator", "terrain ruggedness index"],
    keywords: &[
        "TRI",
        "ruggedness",
        "Riley",
        "Wilson",
        "roughness",
        "gdaldem",
    ],
    inputs: &[
        WINDOW_INPUTS[0],
        WINDOW_INPUTS[1],
        WINDOW_INPUTS[2],
        WINDOW_INPUTS[3],
    ],
    outputs: &[RUGGEDNESS.outputs[0], RUGGEDNESS.outputs[1]],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED"],
    model: "TRI as the root of the summed squared differences from the center cell (Riley and others 1999, as corrected) and as the mean absolute difference (Wilson and others 2007), as in raster.terrain.ruggedness",
    accuracy: RUGGEDNESS.accuracy,
    when_to_use: "Use this to put a number on how broken the ground is around a cell: for habitat studies, access planning, or matching a TRI value from a paper or a GIS layer. Enter the nine elevations around the cell. Both forms of the index are given, so you can match whichever one a source used.",
    limitations: "Two different indices are called TRI, and they differ by about a factor of three on the same ground; check which one a value you compare against used. Both depend on the cell size, so values are comparable only within one resolution.",
    references: &[RILEY, WILSON],
    examples: &[Example {
        id: "primary",
        title: "Broken ground on a 30 m grid",
        input: r#"{"elevations":[{"row":"1210.4, 1225.9, 1218.2"},{"row":"1204.7, 1220.0, 1231.6"},{"row":"1199.3, 1212.8, 1226.1"}],"cell_size":"30 m"}"#,
        source: "The definitions in Python, checked against gdaldem 3.13.3 TRI (tools/vectors/gen_terrain_views.py)",
    }],
    primary_example: "primary",
    visualization: TABLE,
    related: &[
        parent("raster.terrain.ruggedness"),
        sibling("raster.terrain.tpi"),
        sibling("raster.terrain.slope"),
    ],
    assumptions: RUGGEDNESS.assumptions,
    sentence: "Ruggedness is {tri} by Riley, or {tri_mean} as a mean difference.",
    limits: &[("batchRows", 10_000)],
    run: run_tri,
    ..ToolDef::BLANK
};

pub static TPI: ToolDef = ToolDef {
    stability: gp_base::tool::Stability::Stable,
    id: "raster.terrain.tpi",
    title: "Topographic position index (TPI)",
    summary: "The topographic position index at the center of a 3 x 3 elevation window: the cell less the mean of its eight neighbors, and whether it sits above or below them.",
    aliases: &["TPI calculator", "topographic position index"],
    keywords: &[
        "TPI",
        "topographic position",
        "ridge",
        "valley",
        "landform",
        "gdaldem",
    ],
    inputs: &[
        WINDOW_INPUTS[0],
        WINDOW_INPUTS[1],
        WINDOW_INPUTS[2],
        WINDOW_INPUTS[3],
    ],
    outputs: &[RUGGEDNESS.outputs[2], RUGGEDNESS.outputs[4]],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED"],
    model: "TPI as the center elevation less the mean of its eight neighbors (Wilson and others 2007), as in raster.terrain.ruggedness",
    accuracy: RUGGEDNESS.accuracy,
    when_to_use: "Use this to tell whether a cell stands above its surroundings, like a ridge or a summit, or below them, like a valley or a hollow. It is the first step of most landform classifications. Enter the nine elevations around the cell; a positive index is high ground and a negative one is low ground.",
    limitations: "It looks only at the eight cells around the center, so it sees small landforms: a cell can sit above its neighbors and still lie in a broad valley. Values depend on the cell size and are comparable only within one resolution.",
    references: &[WILSON],
    examples: &[Example {
        id: "primary",
        title: "A ridge crest on a 30 m grid",
        input: r#"{"elevations":[{"row":"842.0, 845.5, 841.2"},{"row":"846.1, 851.3, 847.0"},{"row":"840.4, 844.8, 839.9"}],"cell_size":"30 m"}"#,
        source: "The definition in Python, checked against gdaldem 3.13.3 TPI (tools/vectors/gen_terrain_views.py)",
    }],
    primary_example: "primary",
    visualization: TABLE,
    related: &[
        parent("raster.terrain.ruggedness"),
        sibling("raster.terrain.roughness-tri"),
        sibling("raster.terrain.curvature"),
    ],
    assumptions: RUGGEDNESS.assumptions,
    sentence: "The position index is {tpi}, so the cell sits {position}.",
    limits: &[("batchRows", 10_000)],
    run: run_tpi,
    ..ToolDef::BLANK
};

pub const ZEVENBERGEN_THORNE: Reference = Reference {
    title: "Quantitative analysis of land surface topography",
    issuer: "Zevenbergen, L. W., and Thorne, C. R., Earth Surface Processes and Landforms",
    year: 1987,
    edition: "Volume 12, issue 1, pages 47-56",
    locator: "The partial quartic fitted through a 3 x 3 window, and profile and plan curvature from its coefficients",
    url: "https://doi.org/10.1002/esp.3290120107",
};
const XDEM_CURVATURE: Reference = Reference {
    title: "xDEM terrain curvature",
    issuer: "xDEM contributors, GlacioHack",
    year: 2026,
    edition: "xDEM 0.2.3",
    locator: "profile_curvature example using the Zevenbergen-Thorne surface fit and directional method",
    url: "https://github.com/GlacioHack/xdem/blob/v0.2.3/xdem/terrain/terrain.py",
};

/// Curvatures are reported per 100 m, where a value of 1 means the slope
/// changes by 1% over a meter (a tenth of a degree, roughly, every 10 m).
const PER_100M: f64 = 100.0;

pub static CURVATURE: ToolDef = ToolDef {
    id: "raster.terrain.curvature",
    version: "1.1.0",
    stability: gp_base::tool::Stability::Stable,
    title: "Terrain curvature",
    summary: "Total, profile, and plan curvature for the center of a 3 x 3 elevation window by Zevenbergen and Thorne’s method, with what each means for water running over the cell.",
    aliases: &[
        "curvature calculator",
        "profile curvature",
        "plan curvature",
        "planform curvature",
    ],
    keywords: &[
        "curvature",
        "convex",
        "concave",
        "terrain",
        "DEM",
        "Zevenbergen",
        "flow",
        "ridge",
        "hollow",
    ],
    inputs: &[
        WINDOW_INPUTS[0],
        WINDOW_INPUTS[1],
        WINDOW_INPUTS[2],
        WINDOW_INPUTS[3],
    ],
    outputs: &[
        Field::new(
            "curvature",
            "Total curvature",
            "Per 100 m; positive where the ground bulges up like a hilltop, negative where it dips like a hollow",
            Kind::Number {
                min: -1e12,
                max: 1e12,
            },
        )
        .precision(Precision::Decimals(4)),
        Field::new(
            "profile",
            "Profile curvature",
            "Per 100 m, down the slope; positive where the slope steepens downhill",
            Kind::Number {
                min: -1e12,
                max: 1e12,
            },
        )
        .precision(Precision::Decimals(4))
        .optional(),
        Field::new(
            "plan",
            "Plan curvature",
            "Per 100 m, across the slope; positive where the contours bow outward, as on a spur",
            Kind::Number {
                min: -1e12,
                max: 1e12,
            },
        )
        .precision(Precision::Decimals(4))
        .optional(),
        Field::new(
            "profile_text",
            "Down the slope",
            "What the profile curvature does to running water",
            Kind::Text { max_len: 64 },
        )
        .optional(),
        Field::new(
            "plan_text",
            "Across the slope",
            "What the plan curvature does to running water",
            Kind::Text { max_len: 64 },
        )
        .optional(),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["FLAT_CELL", "UNIT_ASSUMED"],
    model: "Zevenbergen and Thorne (1987): D = ((z4 + z6)/2 − z5)/dx², E = ((z2 + z8)/2 − z5)/dy², F = (z3 + z7 − z1 − z9)/(4 dx dy), G = (z6 − z4)/(2 dx), H = (z2 − z8)/(2 dy); total = −2(D + E), profile = −2(DG² + EH² + FGH)/(G² + H²), plan = −2(DH² + EG² − FGH)/(G² + H²), each times 100, positive for convex",
    accuracy: "Exact evaluation of the formulas; exact for any surface that is quadratic across the window.",
    when_to_use: "Use this to see how the ground bends at a cell, which slope alone does not say: whether a slope steepens or flattens downhill, which speeds up or slows down running water, and whether the contours bow out over a spur, where water spreads, or curl into a hollow, where it gathers. These are the measures behind erosion, landslide, and soil-moisture mapping, and checking one cell by hand is how a GIS layer of them is tested.",
    limitations: "Programs disagree on the sign and the scale of these measures: some multiply by 100 as this does, some do not, and some flip the sign of profile curvature so that negative means convex. Here positive is convex in all three, so check the convention before comparing numbers. The values depend strongly on the cell size and on noise in the elevation model, since they are second differences: a small error in one elevation moves them a lot. On flat ground there is no downhill direction, so profile and plan curvature are not given.",
    references: &[ZEVENBERGEN_THORNE, XDEM_CURVATURE],
    examples: &[Example {
        id: "primary",
        title: "xDEM directional-curvature example",
        input: r#"{"elevations":[{"row":"1, 2, 4"},{"row":"1, 2, 4"},{"row":"1, 2, 4"}],"cell_size":"1 m"}"#,
        source: "xDEM 0.2.3 profile_curvature example: profile curvature -100 per 100 m; its matching tangential-curvature example gives 0",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "raster.terrain.slope-aspect",
            reason: "parent",
        },
        Related {
            id: "raster.terrain.ruggedness",
            reason: "alternative",
        },
        Related {
            id: "raster.terrain.contours",
            reason: "alternative",
        },
    ],
    assumptions: &[
        Assumption {
            name: "WGS 84 semi-major axis, for cell sizes given in degrees",
            value: "6378137",
            unit: "m",
            source: "nga-wgs84",
        },
        Assumption {
            name: "WGS 84 inverse flattening, for cell sizes given in degrees",
            value: "298.257223563",
            unit: "1",
            source: "nga-wgs84",
        },
    ],
    sentence: "The total curvature is {curvature} per 100 m.{if profile != 0} Downhill the ground {profile_text}.{/if}{if plan != 0} Across the slope the {plan_text}.{/if}",
    limits: &[("batchRows", 10_000)],
    run: run_curvature,
    ..ToolDef::BLANK
};

fn run_curvature(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let w = read_window(ctx)?;
    let z = &w.z;
    let (dx, dy) = (w.dx, w.dy);
    // z1 … z9 row by row from the north-west, so z2 is north and z4 west.
    let d = ((z[3] + z[5]) / 2.0 - z[4]) / (dx * dx);
    let e = ((z[1] + z[7]) / 2.0 - z[4]) / (dy * dy);
    let f = (z[2] + z[6] - z[0] - z[8]) / (4.0 * dx * dy);
    let g = (z[5] - z[3]) / (2.0 * dx);
    let h = (z[1] - z[7]) / (2.0 * dy);
    let total = -2.0 * (d + e) * PER_100M;
    let mut out = vec![("curvature", Json::Num(total))];
    let g2h2 = g * g + h * h;
    if g2h2 == 0.0 {
        ctx.warnings.push(Warning::new(
            "FLAT_CELL",
            "The window has no gradient, so there is no downhill direction to measure profile and plan curvature along.",
        ));
        return Ok(Json::obj(out));
    }
    let profile = -2.0 * (d * g * g + e * h * h + f * g * h) / g2h2 * PER_100M;
    let plan = -2.0 * (d * h * h + e * g * g - f * g * h) / g2h2 * PER_100M;
    out.push(("profile", Json::Num(profile)));
    out.push(("plan", Json::Num(plan)));
    out.push((
        "profile_text",
        Json::str(if profile > 0.0 {
            "steepens, so water speeds up"
        } else if profile < 0.0 {
            "flattens, so water slows down"
        } else {
            "keeps an even grade"
        }),
    ));
    out.push((
        "plan_text",
        Json::str(if plan > 0.0 {
            "contours bow outward, so water spreads"
        } else if plan < 0.0 {
            "contours curl inward, so water gathers"
        } else {
            "contours run straight"
        }),
    ));
    Ok(Json::obj(out))
}
