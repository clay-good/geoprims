//! Contours (raster/terrain-analysis, "Slope, aspect, hillshade, and
//! contours"): lines of equal elevation at an interval, as vector lines, by
//! marching squares over a grid of elevations.
//!
//! Each grid square is looked at on its own. A contour crosses an edge of the
//! square where one corner is above the level and the other at or below it, at
//! the point found by linear interpolation along that edge, and the crossings
//! in a square are joined in pairs. When all four edges are crossed (a
//! saddle), the mean of the four corners decides whether the high corners
//! connect through the middle. Every line is oriented with higher ground on
//! its left, so crossings computed once per edge join the squares' pieces
//! into lines without searching.

use gp_base::ErrorCode;
use gp_base::angle::{accept_lon, check_lat, wrap_lon};
use gp_base::contour::{join, pieces_rect};
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{
    Assumption, Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef,
};
use gp_base::units::{self, Quantity as QT};

use crate::terrain::degree_lengths;

const LORENSEN: Reference = Reference {
    title: "Marching cubes: A high resolution 3D surface construction algorithm",
    issuer: "Lorensen, W. E., and Cline, H. E., ACM SIGGRAPH Computer Graphics",
    year: 1987,
    edition: "Volume 21, issue 4, pages 163-169",
    locator: "The cell-by-cell case table with linear interpolation along edges, of which marching squares is the two-dimensional case",
    url: "https://doi.org/10.1145/37402.37422",
};

/// Grid size limits, per side.
pub(crate) const MAX_SIDE: usize = 300;
/// Levels allowed in one run.
const MAX_LEVELS: usize = 500;
/// Contour vertices allowed in the output.
const MAX_VERTICES: usize = 100_000;

pub(crate) const ROW: &[Field] = &[Field::new(
    "row",
    "Elevations",
    "One row of elevations west to east, like 101.2, 100.6, 100.2",
    Kind::Text { max_len: 4_000 },
)];

const fn meters(name: &'static str, title: &'static str, help: &'static str) -> Field {
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

const LINE_ROW: &[Field] = &[
    Field::new(
        "line",
        "Line",
        "Its number, from 1, like 3",
        Kind::Number { min: 1.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
    meters("level", "Elevation", "The contour's elevation, like 110 m")
        .precision(Precision::Decimals(3)),
    meters(
        "east",
        "Distance east",
        "From the north-west grid point, like 45.5 m",
    )
    .precision(Precision::Decimals(3)),
    meters(
        "south",
        "Distance south",
        "From the north-west grid point, like 12.25 m",
    )
    .precision(Precision::Decimals(3)),
    Field::new(
        "lat",
        "Latitude",
        "Present for a geographic grid, north positive",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .optional()
    .precision(Precision::Decimals(7))
    .angle_range("[-90,90]"),
    Field::new(
        "lon",
        "Longitude",
        "Present for a geographic grid, east positive",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .optional()
    .precision(Precision::Decimals(7))
    .angle_range("[-180,180)"),
];
const LEVEL_ROW: &[Field] = &[
    meters("level", "Elevation", "The contour's elevation, like 110 m")
        .precision(Precision::Decimals(3)),
    Field::new(
        "lines",
        "Lines",
        "Separate lines at this elevation, like 2",
        Kind::Number { min: 0.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "closed",
        "Closed loops",
        "Lines that close on themselves, like 1",
        Kind::Number { min: 0.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
    meters(
        "length",
        "Length",
        "All the lines at this elevation, like 412.6 m",
    )
    .precision(Precision::Decimals(3)),
];

pub static CONTOURS: ToolDef = ToolDef {
    id: "raster.terrain.contours",
    stability: gp_base::tool::Stability::Stable,
    title: "Contour lines from a grid of elevations",
    summary: "Lines of equal elevation at a chosen interval from a grid of elevations, as vector lines with their lengths, by marching squares.",
    aliases: &[
        "contour lines",
        "contours from dem",
        "isolines",
        "topo lines",
        "generate contours",
        "marching squares",
    ],
    keywords: &[
        "contour",
        "isoline",
        "elevation",
        "interval",
        "dem",
        "topographic",
        "terrain",
    ],
    inputs: &[
        Field::new(
            "elevations",
            "Elevation grid",
            "Rows of elevations in meters, north row first, west to east, like 101.2, 100.6, 100.2",
            Kind::List {
                items: ROW,
                min: 2,
                max: MAX_SIDE,
            },
        )
        .required()
        .core(),
        meters(
            "interval",
            "Contour interval",
            "The rise between lines, like 5 m",
        )
        .required()
        .core(),
        meters(
            "cell_size",
            "Cell size",
            "The ground distance between grid points, like 30 m",
        )
        .core(),
        Field::new(
            "cell_degrees",
            "Cell size in degrees",
            "For a north-up geographic grid, like 0.000277778 (one arc second)",
            Kind::Number {
                min: 0.0,
                max: 10.0,
            },
        )
        .core(),
        Field::new(
            "north_west_lat",
            "North-west latitude",
            "Latitude of the first grid point, like 40.0",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .angle_range("[-90,90]"),
        Field::new(
            "north_west_lon",
            "North-west longitude",
            "Longitude of the first grid point, like -105.0",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .angle_range("[-180,180)"),
        meters(
            "base",
            "Base elevation",
            "Contours fall on this plus whole intervals, like 0 m (the default)",
        ),
        Field::new(
            "no_data",
            "No-data value",
            "A value that marks a missing elevation, like -9999",
            Kind::Number {
                min: -1e12,
                max: 1e12,
            },
        ),
    ],
    outputs: &[
        Field::new(
            "line_count",
            "Lines",
            "Separate contour lines at every level, like 7",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "level_count",
            "Levels",
            "Elevations with at least one line, like 4",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        meters(
            "length",
            "Total length",
            "All the lines together, like 1,250 m",
        )
        .precision(Precision::Decimals(3)),
        meters("lowest", "Lowest elevation", "In the grid, like 96.1 m")
            .precision(Precision::Decimals(3)),
        meters("highest", "Highest elevation", "In the grid, like 131.4 m")
            .precision(Precision::Decimals(3)),
        Field::new(
            "levels",
            "Each level",
            "Lines, closed loops, and length per elevation",
            Kind::List {
                items: LEVEL_ROW,
                min: 0,
                max: MAX_LEVELS,
            },
        ),
        Field::new(
            "contours",
            "Contour vertices",
            "Each line's points in order, higher ground on the left",
            Kind::List {
                items: LINE_ROW,
                min: 0,
                max: MAX_VERTICES,
            },
        ),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::LimitExceeded],
    warnings: &["INPUT_NORMALIZED", "UNIT_ASSUMED"],
    model: "Marching squares (the two-dimensional case of Lorensen and Cline 1987): in each grid square, a level crosses an edge whose corners lie on either side of it (a corner exactly at the level counts as below, as in contourpy), at the linearly interpolated point; the crossings are paired, and a square crossed on all four edges joins its high corners through the middle when the mean of its corners is above the level. Levels are the base plus whole intervals within the grid's range. Squares with a no-data corner are skipped. A geographic grid uses WGS 84 east-west and north-south degree lengths at its center latitude, and its vertices retain the grid's affine latitude-longitude placement",
    accuracy: "Contour placement is exact for the surface made of linear pieces along the grid lines. Geographic lengths use the WGS 84 degree lengths at the grid's center latitude; a real hillside between grid points is only as well known as the grid",
    when_to_use: "Use this to turn a small grid of elevations into contour lines: a survey of spot heights on a regular grid, a DEM clip exported as numbers, or a site model, when you want lines to draw, measure, or hand to CAD rather than a picture. It gives each line's points in order with higher ground on the left, the lines and closed loops per elevation, and their lengths, so a closed loop can be read as a hill or a hollow at a glance.",
    limitations: "The grid must be regular, north-up, and unrotated, with elevations in meters. A meter grid is square-celled and stays in local east-south coordinates; a geographic grid uses equal latitude and longitude steps, whose ground widths differ by latitude, and also returns map coordinates. Between grid points the ground is assumed to change linearly along each grid line, so a contour cannot show a feature smaller than the grid spacing, and where a square is crossed on all four sides the choice of which way to join is a convention (the mean of the corners, as contourpy and Matplotlib use) rather than something the grid knows. Squares that touch a no-data value are left out, so lines stop at gaps. At most 300 by 300 points, 500 levels, and 100,000 contour vertices.",
    references: &[LORENSEN],
    examples: &[
        Example {
            id: "primary",
            title: "5 m contours on a geographic hill",
            input: r#"{"elevations":[{"row":"100, 102, 104, 102, 100"},{"row":"102, 106, 110, 106, 102"},{"row":"104, 110, 118, 110, 104"},{"row":"102, 106, 110, 106, 102"},{"row":"100, 102, 104, 102, 100"}],"interval":"5 m","cell_degrees":0.0001,"north_west_lat":40,"north_west_lon":-105}"#,
            source: "contourpy 1.3.0 gives the same three closed loops at 105, 110, and 115 m; the geographic placement follows the grid coordinates and the WGS 84 degree lengths",
        },
        Example {
            id: "local",
            title: "The same hill on a 10 m local grid",
            input: r#"{"elevations":[{"row":"100, 102, 104, 102, 100"},{"row":"102, 106, 110, 106, 102"},{"row":"104, 110, 118, 110, 104"},{"row":"102, 106, 110, 106, 102"},{"row":"100, 102, 104, 102, 100"}],"interval":"5 m","cell_size":"10 m"}"#,
            source: "contourpy 1.3.0 (the contouring engine behind Matplotlib) gives the same three closed loops, at 105, 110, and 115 m, with the same lengths to 1e-9",
        },
    ],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "polyline",
        map: &[("lines", "contours")],
    }],
    related: &[
        Related {
            id: "raster.terrain.slope-aspect",
            reason: "alternative",
        },
        Related {
            id: "raster.terrain.ruggedness",
            reason: "alternative",
        },
        Related {
            id: "survey.earthwork.profile-grades",
            reason: "next",
        },
    ],
    assumptions: &[
        Assumption {
            name: "WGS 84 semi-major axis, for geographic grid spacing",
            value: "6378137",
            unit: "m",
            source: "nga-wgs84",
        },
        Assumption {
            name: "WGS 84 inverse flattening, for geographic grid spacing",
            value: "298.257223563",
            unit: "1",
            source: "nga-wgs84",
        },
    ],
    sentence: "{line_count} contour {plural line_count \"line\" \"lines\"} at {level_count} {plural level_count \"level\" \"levels\"}, {length} in all.",
    limits: &[("batchRows", MAX_SIDE as u64)],
    run: run_contours,
    ..ToolDef::BLANK
};

/// A north-up grid of elevations from the list-of-rows field `field`, with
/// the no-data value read as NaN.
pub(crate) fn read_grid(
    ctx: &mut Ctx,
    field: &str,
    no_data: Option<f64>,
) -> Result<Vec<Vec<f64>>, ToolError> {
    let rows = ctx.rows(field)?;
    let mut grid = Vec::with_capacity(rows.len());
    for (i, r) in rows.iter().enumerate() {
        let at = format!("/{field}/{i}/row");
        let text = r.get("row").and_then(|v| v.as_str()).ok_or_else(|| {
            ToolError::invalid(
                &at,
                "Each row is elevations west to east, like 101.2, 100.6, 100.2.",
            )
        })?;
        let mut row = Vec::new();
        for p in text
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|s| !s.is_empty())
        {
            let v: f64 = p
                .parse()
                .ok()
                .filter(|v: &f64| v.is_finite())
                .ok_or_else(|| ToolError::invalid(&at, format!("{p} is not a number.")))?;
            row.push(if Some(v) == no_data { f64::NAN } else { v });
        }
        if row.len() < 2 || row.len() > MAX_SIDE {
            return Err(ToolError::invalid(
                &at,
                format!(
                    "Row {} has {} elevations; each row needs 2 to {MAX_SIDE}.",
                    i + 1,
                    row.len()
                ),
            ));
        }
        if let Some(first) = grid.first().map(Vec::len)
            && row.len() != first
        {
            return Err(ToolError::invalid(
                &at,
                format!(
                    "Row {} has {} elevations and row 1 has {first}; every row needs the same number.",
                    i + 1,
                    row.len()
                ),
            ));
        }
        grid.push(row);
    }
    Ok(grid)
}

fn run_contours(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let m = units::by_symbol(QT::Length, "m").expect("m");
    let deg_unit = units::by_symbol(QT::Angle, "deg").expect("deg");
    let no_data = ctx.number("no_data")?;
    let z = read_grid(ctx, "elevations", no_data)?;
    let interval = ctx.quantity("interval")?.expect("required").to(m);
    let base = ctx.quantity("base")?.map_or(0.0, |v| v.to(m));
    if interval.is_nan() || interval <= 0.0 {
        return Err(ToolError::invalid(
            "/interval",
            "The contour interval must be more than zero.",
        ));
    }
    let cell_size = ctx.quantity("cell_size")?.map(|v| v.to(m));
    let cell_degrees = ctx.number("cell_degrees")?;
    let north_west_lat = ctx.quantity("north_west_lat")?.map(|v| v.to(deg_unit));
    let north_west_lon = ctx.quantity("north_west_lon")?.map(|v| v.to(deg_unit));
    let (dx, dy, georef) = match (cell_size, cell_degrees, north_west_lat, north_west_lon) {
        (Some(cell), None, None, None) if cell.is_finite() && cell > 0.0 => (cell, cell, None),
        (Some(_), None, Some(_), _) | (Some(_), None, _, Some(_)) => {
            return Err(ToolError::invalid(
                "/north_west_lat",
                "A meter grid stays in local coordinates. To place contours on the map, give cell_degrees with both north-west coordinates.",
            ));
        }
        (Some(_), Some(_), _, _) => {
            return Err(ToolError::invalid(
                "/cell_size",
                "Give the cell size in meters or in degrees, not both.",
            ));
        }
        (None, Some(step), Some(lat), Some(lon)) if step.is_finite() && step > 0.0 => {
            let lat = check_lat(lat, "/north_west_lat")?;
            let south = lat - (z.len() - 1) as f64 * step;
            check_lat(south, "/cell_degrees").map_err(|_| {
                ToolError::invalid(
                    "/cell_degrees",
                    "The geographic grid extends past 90 degrees south. Use a smaller cell size or a different north-west latitude.",
                )
            })?;
            if (z[0].len() - 1) as f64 * step > 360.0 {
                return Err(ToolError::invalid(
                    "/cell_degrees",
                    "The geographic grid spans more than 360 degrees of longitude.",
                ));
            }
            let (lon, warning) = accept_lon(lon, "/north_west_lon")?;
            ctx.warnings.extend(warning);
            let center_lat = (lat + south) / 2.0;
            let (per_lat, per_lon) = degree_lengths(center_lat);
            let (dx, dy) = (step * per_lon, step * per_lat);
            if dx <= 0.0 || dy <= 0.0 {
                return Err(ToolError::invalid(
                    "/north_west_lat",
                    "A geographic grid at a pole has no east-west cell width.",
                ));
            }
            (dx, dy, Some((lat, lon, step)))
        }
        (None, Some(_), _, _) => {
            return Err(ToolError::invalid(
                "/north_west_lat",
                "A cell size in degrees needs both north-west latitude and longitude.",
            ));
        }
        (None, None, None, None) => {
            return Err(ToolError::invalid(
                "/cell_size",
                "Give the cell size in meters, or in degrees with both north-west coordinates.",
            ));
        }
        _ => {
            return Err(ToolError::invalid(
                "/cell_size",
                "Give a positive cell size in meters, or a positive cell size in degrees with both north-west coordinates.",
            ));
        }
    };
    let known: Vec<f64> = z
        .iter()
        .flatten()
        .copied()
        .filter(|v| !v.is_nan())
        .collect();
    if known.is_empty() {
        return Err(ToolError::invalid(
            "/elevations",
            "Every elevation is the no-data value.",
        ));
    }
    let lo = known.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = known.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let (k0, k1) = (
        libm::ceil((lo - base) / interval),
        libm::floor((hi - base) / interval),
    );
    if k1 - k0 + 1.0 > MAX_LEVELS as f64 {
        return Err(ToolError::new(
            ErrorCode::LimitExceeded,
            format!(
                "That interval gives {} levels between {lo} and {hi} m; at most {MAX_LEVELS}. Use a larger interval.",
                k1 - k0 + 1.0
            ),
        )
        .at("/interval"));
    }
    let q = |v: f64| Q { value: v, unit: m }.to_json();
    let (mut vertices, mut rows, mut per_level) = (0usize, Vec::new(), Vec::new());
    let (mut line_count, mut total) = (0usize, 0.0);
    let mut k = k0;
    while k <= k1 {
        let level = base + k * interval;
        k += 1.0;
        let lines = join(&pieces_rect(&z, dx, dy, level));
        if lines.is_empty() {
            continue;
        }
        let (mut len_level, mut closed) = (0.0, 0usize);
        for (pts, is_closed) in &lines {
            line_count += 1;
            closed += usize::from(*is_closed);
            vertices += pts.len();
            if vertices > MAX_VERTICES {
                return Err(ToolError::new(
                    ErrorCode::LimitExceeded,
                    format!("Over {MAX_VERTICES} contour vertices. Use a larger interval or a smaller grid."),
                )
                .at("/interval"));
            }
            for w in pts.windows(2) {
                len_level += (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1);
            }
            for p in pts {
                let mut row = vec![
                    ("line", Json::Num(line_count as f64)),
                    ("level", q(level)),
                    ("east", q(p.0)),
                    ("south", q(p.1)),
                ];
                if let Some((lat, lon, step)) = georef {
                    row.push((
                        "lat",
                        Q {
                            value: lat - (p.1 / dy) * step,
                            unit: deg_unit,
                        }
                        .to_json(),
                    ));
                    row.push((
                        "lon",
                        Q {
                            value: wrap_lon(lon + (p.0 / dx) * step),
                            unit: deg_unit,
                        }
                        .to_json(),
                    ));
                }
                rows.push(Json::obj(row));
            }
        }
        total += len_level;
        per_level.push(Json::obj([
            ("level", q(level)),
            ("lines", Json::Num(lines.len() as f64)),
            ("closed", Json::Num(closed as f64)),
            ("length", q(len_level)),
        ]));
    }
    Ok(Json::obj([
        ("line_count", Json::Num(line_count as f64)),
        ("level_count", Json::Num(per_level.len() as f64)),
        (
            "length",
            ctx.out(
                "length",
                Q {
                    value: total,
                    unit: m,
                },
            ),
        ),
        ("lowest", ctx.out("lowest", Q { value: lo, unit: m })),
        ("highest", ctx.out("highest", Q { value: hi, unit: m })),
        ("levels", Json::Arr(per_level)),
        ("contours", Json::Arr(rows)),
    ]))
}
