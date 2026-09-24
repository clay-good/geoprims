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
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
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
fn degree_lengths(lat: f64) -> (f64, f64) {
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

fn run_slope(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let w = read_window(ctx)?;
    let zf = ctx.number("z_factor")?.unwrap_or(1.0);
    let (dzdx, dzdy) = gradients(&w);
    let (gx, gy) = (zf * dzdx, zf * dzdy);
    let rise = sqrt(gx * gx + gy * gy);
    let slope = atan(rise).to_degrees();
    // Aspect as GDAL reports it: degrees clockwise from north, downslope.
    let flat = rise == 0.0;
    let aspect = if flat {
        -1.0
    } else {
        let a = atan2(gy, -gx).to_degrees();
        let mut a = 90.0 - a;
        if a < 0.0 {
            a += 360.0;
        }
        if a >= 360.0 {
            a -= 360.0;
        }
        a
    };
    if flat {
        ctx.warnings.push(Warning::new(
            "FLAT_CELL",
            "The window has no gradient, so it has no aspect; flat ground faces no direction.",
        ));
    }
    // Hillshade, as gdaldem computes it, from the sun's position.
    let az = ctx.quantity("sun_azimuth")?.map_or(315.0, |v| {
        v.to(units::by_symbol(QT::Angle, "deg").expect("deg"))
    });
    let alt = ctx.quantity("sun_altitude")?.map_or(45.0, |v| {
        v.to(units::by_symbol(QT::Angle, "deg").expect("deg"))
    });
    let zenith = (90.0 - alt).to_radians();
    let slope_rad = atan(rise);
    let aspect_rad = if flat { 0.0 } else { atan2(gy, -gx) };
    let sun = (90.0 - az).to_radians();
    let shade = 255.0
        * (cos(zenith) * cos(slope_rad) + sin(zenith) * sin(slope_rad) * cos(sun - aspect_rad));
    let shade = shade.clamp(0.0, 255.0);
    let mut out = vec![
        ("slope", ctx.out("slope", q(slope, QT::Angle, "deg"))),
        ("slope_percent", Json::Num(rise * 100.0)),
        ("hillshade", Json::Num(shade.round())),
        (
            "cell_size_east",
            ctx.out("cell_size_east", q(w.dx, QT::Length, "m")),
        ),
        (
            "cell_size_north",
            ctx.out("cell_size_north", q(w.dy, QT::Length, "m")),
        ),
    ];
    if flat {
        out.push(("aspect_text", Json::str("flat")));
    } else {
        out.push(("aspect", ctx.out("aspect", q(aspect, QT::Angle, "deg"))));
        out.push((
            "aspect_text",
            Json::str(
                [
                    "north",
                    "north-east",
                    "east",
                    "south-east",
                    "south",
                    "south-west",
                    "west",
                    "north-west",
                ][(((aspect + 22.5) % 360.0) / 45.0) as usize],
            ),
        ));
    }
    Ok(Json::obj(out))
}

fn run_ruggedness(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let w = read_window(ctx)?;
    let center = w.z[4];
    let neighbors: Vec<f64> = (0..9).filter(|i| *i != 4).map(|i| w.z[i]).collect();
    // Two indices share the name TRI. Riley's own, as the erratum printed with
    // the paper corrects it, is the root of the summed squared differences;
    // the mean absolute difference is the other one in common use, which GDAL
    // offers as its Wilson algorithm. They differ by about a factor of three
    // on the same ground, so both are reported rather than one picked.
    let tri_riley = sqrt(
        neighbors
            .iter()
            .map(|z| (z - center) * (z - center))
            .sum::<f64>(),
    );
    let tri_mean = neighbors.iter().map(|z| (z - center).abs()).sum::<f64>() / 8.0;
    let mean = neighbors.iter().sum::<f64>() / 8.0;
    let tpi = center - mean;
    let (lo, hi) =
        w.z.iter()
            .fold((f64::MAX, f64::MIN), |(a, b), z| (a.min(*z), b.max(*z)));
    Ok(Json::obj([
        ("tri", ctx.out("tri", q(tri_riley, QT::Length, "m"))),
        (
            "tri_mean",
            ctx.out("tri_mean", q(tri_mean, QT::Length, "m")),
        ),
        ("tpi", ctx.out("tpi", q(tpi, QT::Length, "m"))),
        (
            "roughness",
            ctx.out("roughness", q(hi - lo, QT::Length, "m")),
        ),
        (
            "position",
            Json::str(if tpi > 0.0 {
                "above its neighbours: a ridge, a summit, or a spur"
            } else if tpi < 0.0 {
                "below its neighbours: a valley, a pit, or a hollow"
            } else {
                "level with its neighbours"
            }),
        ),
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
    title: "Slope, aspect, and hillshade",
    summary: "Slope, aspect, and hillshade for the center of a 3 x 3 elevation window by Horn's method, with the cell size corrected for latitude on a degree-based grid.",
    aliases: &[
        "slope calculator",
        "aspect calculator",
        "hillshade",
        "terrain slope from DEM",
    ],
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
            "0 to 255, as gdaldem computes it",
            Kind::Number {
                min: 0.0,
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
    references: &[HORN],
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
    sentence: "The slope is {slope} ({slope_percent}%), facing {aspect_text}.",
    limits: &[("batchRows", 10_000)],
    run: run_slope,
    ..ToolDef::BLANK
};

pub static RUGGEDNESS: ToolDef = ToolDef {
    id: "raster.terrain.ruggedness",
    title: "Terrain ruggedness and position",
    summary: "The terrain ruggedness index, topographic position index, and roughness for the center of a 3 x 3 elevation window.",
    aliases: &["TRI calculator", "TPI calculator", "terrain roughness"],
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
    sentence: "Ruggedness is {tri} by Riley, or {tri_mean} as a mean difference, and the cell sits {position}.",
    limits: &[("batchRows", 10_000)],
    run: run_ruggedness,
    ..ToolDef::BLANK
};
