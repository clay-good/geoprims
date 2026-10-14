//! Whole-grid DEM measures (raster/terrain-analysis inventory: dem-stats,
//! highest-point, dem-difference-volume) on a grid of elevations the user
//! enters, read the way the contour tool reads it. Until the terrain tiles are
//! hosted, the grid comes from a survey, a site model, or a DEM clip exported
//! as numbers.

use gp_base::ErrorCode;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use libm::sqrt;

use crate::contour::{MAX_SIDE, ROW, read_grid};

const GDALINFO: Reference = Reference {
    title: "gdalinfo",
    issuer: "GDAL/OGR contributors, Open Source Geospatial Foundation",
    year: 2026,
    edition: "GDAL 3.13.3 documentation",
    locator: "-stats: read and display image statistics (minimum, maximum, mean, and standard deviation of the valid pixels)",
    url: "https://gdal.org/en/stable/programs/gdalinfo.html",
};
const WHEATON: Reference = Reference {
    title: "Accounting for uncertainty in DEMs from repeat topographic surveys: improved sediment budgets",
    issuer: "Wheaton, J. M., Brasington, J., Darby, S. E., and Sear, D. A., Earth Surface Processes and Landforms",
    year: 2010,
    edition: "Volume 35, issue 2, pages 136-156",
    locator: "The DEM of difference, its volumes of erosion and deposition, and a minimum level of detection",
    url: "https://doi.org/10.1002/esp.1886",
};

const TABLE: &[Layer] = &[Layer {
    kind: "table-only",
    map: &[],
}];

const fn grid(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::List {
            items: ROW,
            min: 2,
            max: MAX_SIDE,
        },
    )
    .required()
    .core()
}

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

const fn count(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(name, title, help, Kind::Number { min: 0.0, max: 1e6 })
        .precision(Precision::Decimals(0))
}

const fn cubic(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Volume,
            unit: "m3",
        },
    )
    .precision(Precision::Decimals(2))
}

const ELEVATIONS: Field = grid(
    "elevations",
    "Elevation grid",
    "Rows of elevations in meters, north row first, west to east, like 101.2, 100.6, 100.2",
);
const NO_DATA: Field = Field::new(
    "no_data",
    "No-data value",
    "A value that marks a missing elevation, like -9999",
    Kind::Number {
        min: -1e12,
        max: 1e12,
    },
);

fn q(v: f64, qt: QT, s: &str) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(qt, s).expect("registered unit"),
    }
}

fn known(z: &[Vec<f64>]) -> Result<Vec<f64>, ToolError> {
    let v: Vec<f64> = z
        .iter()
        .flatten()
        .copied()
        .filter(|v| !v.is_nan())
        .collect();
    if v.is_empty() {
        return Err(ToolError::invalid(
            "/elevations",
            "Every elevation is the no-data value.",
        ));
    }
    Ok(v)
}

fn cell_size(ctx: &mut Ctx) -> Result<Option<f64>, ToolError> {
    let m = units::by_symbol(QT::Length, "m").expect("m");
    match ctx.quantity("cell_size")?.map(|v| v.to(m)) {
        Some(c) if !(c.is_finite() && c > 0.0) => Err(ToolError::invalid(
            "/cell_size",
            "The cell size must be more than zero.",
        )),
        c => Ok(c),
    }
}

pub static DEM_STATS: ToolDef = ToolDef {
    stability: gp_base::tool::Stability::Stable,
    id: "raster.terrain.dem-stats",
    title: "Elevation statistics for a DEM grid",
    summary: "The mean, lowest, highest, relief, standard deviation, and median of a grid of elevations, with how many cells hold data.",
    aliases: &[
        "DEM statistics",
        "elevation statistics",
        "mean elevation of a grid",
    ],
    keywords: &[
        "DEM",
        "statistics",
        "mean",
        "median",
        "standard deviation",
        "relief",
        "minimum",
        "maximum",
        "gdalinfo",
    ],
    inputs: &[ELEVATIONS, NO_DATA],
    outputs: &[
        meters("mean", "Mean elevation", "Of the cells that hold data")
            .precision(Precision::Decimals(3)),
        meters("lowest", "Lowest elevation", "In the grid").precision(Precision::Decimals(3)),
        meters("highest", "Highest elevation", "In the grid").precision(Precision::Decimals(3)),
        meters("relief", "Relief", "Highest less lowest").precision(Precision::Decimals(3)),
        meters(
            "std_dev",
            "Standard deviation",
            "Population standard deviation, as gdalinfo reports it",
        )
        .precision(Precision::Decimals(3)),
        meters(
            "median",
            "Median elevation",
            "The middle value, or the mean of the two middle values",
        )
        .precision(Precision::Decimals(3)),
        count("cells", "Cells with data", "Counted in the statistics"),
        count("no_data_cells", "No-data cells", "Left out"),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &[],
    model: "Over the cells that hold data: mean = Σz / n, population standard deviation = √(Σ(z − mean)² / n), median the middle of the sorted values (the mean of the two middle ones when n is even), relief = highest − lowest",
    accuracy: "Exact arithmetic on the elevations given; the mean and standard deviation match gdalinfo -stats, which uses the same population formula",
    when_to_use: "Use this to summarize a small DEM, a site survey on a grid, or a clip of a larger model: its average and middle elevations, its range, and how much it varies. It is also a quick check of what a GIS reports for the same cells, since the standard deviation here is the population form that gdalinfo uses.",
    limitations: "The statistics treat every cell alike, so they describe the grid, not the ground: on a geographic grid, cells far from the equator cover less ground and still count fully. Cells marked with the no-data value are left out, so a grid mostly of gaps describes only the cells left. At most 300 by 300 cells.",
    references: &[GDALINFO],
    examples: &[Example {
        id: "primary",
        title: "A 4 by 5 site grid",
        input: r#"{"elevations":[{"row":"212.4, 213.1, 214.0, 214.8, 215.2"},{"row":"211.9, 212.6, 213.7, 214.5, 215.0"},{"row":"211.2, 212.0, 213.1, 214.1, 214.6"},{"row":"210.8, 211.5, 212.4, 213.6, 214.2"}]}"#,
        source: "NumPy, with gdalinfo 3.13.3 -stats for the mean and standard deviation (tools/vectors/gen_dem_grid.py)",
    }],
    primary_example: "primary",
    visualization: TABLE,
    related: &[
        Related {
            id: "raster.terrain.highest-point",
            reason: "next",
        },
        Related {
            id: "raster.terrain.contours",
            reason: "next",
        },
        Related {
            id: "raster.terrain.dem-difference-volume",
            reason: "alternative",
        },
    ],
    sentence: "The {cells} cells average {mean}, from {lowest} to {highest}.",
    limits: &[("batchRows", 1_000)],
    run: run_stats,
    ..ToolDef::BLANK
};

fn run_stats(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let no_data = ctx.number("no_data")?;
    let z = read_grid(ctx, "elevations", no_data)?;
    let mut v = known(&z)?;
    let n = v.len() as f64;
    let mean = v.iter().sum::<f64>() / n;
    let std_dev = sqrt(v.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / n);
    v.sort_by(f64::total_cmp);
    let (lo, hi) = (v[0], v[v.len() - 1]);
    let mid = v.len() / 2;
    let median = if v.len() % 2 == 1 {
        v[mid]
    } else {
        (v[mid - 1] + v[mid]) / 2.0
    };
    let total = z.iter().map(Vec::len).sum::<usize>();
    let m = |x| q(x, QT::Length, "m");
    Ok(Json::obj([
        ("mean", ctx.out("mean", m(mean))),
        ("lowest", ctx.out("lowest", m(lo))),
        ("highest", ctx.out("highest", m(hi))),
        ("relief", ctx.out("relief", m(hi - lo))),
        ("std_dev", ctx.out("std_dev", m(std_dev))),
        ("median", ctx.out("median", m(median))),
        ("cells", Json::Num(n)),
        ("no_data_cells", Json::Num((total - v.len()) as f64)),
    ]))
}

pub static HIGHEST_POINT: ToolDef = ToolDef {
    stability: gp_base::tool::Stability::Stable,
    id: "raster.terrain.highest-point",
    title: "Highest point in a DEM grid",
    summary: "The highest elevation in a grid of elevations, the row and column where it first occurs, its distance from the north-west corner, and how many cells tie with it.",
    aliases: &[
        "highest point in a DEM",
        "summit of a grid",
        "peak elevation in a DEM",
    ],
    keywords: &[
        "highest point",
        "summit",
        "peak",
        "maximum",
        "DEM",
        "grid",
        "elevation",
    ],
    inputs: &[
        ELEVATIONS,
        meters(
            "cell_size",
            "Cell size",
            "The ground distance between grid points, like 2 m, to place the point",
        )
        .core(),
        NO_DATA,
    ],
    outputs: &[
        meters("highest", "Highest elevation", "In the grid").precision(Precision::Decimals(3)),
        count("row", "Row", "Counted from 1 at the north"),
        count("column", "Column", "Counted from 1 at the west"),
        meters(
            "east",
            "Distance east",
            "From the north-west grid point, when a cell size is given",
        )
        .optional()
        .precision(Precision::Decimals(3)),
        meters(
            "south",
            "Distance south",
            "From the north-west grid point, when a cell size is given",
        )
        .optional()
        .precision(Precision::Decimals(3)),
        count(
            "ties",
            "Cells at that elevation",
            "1 when the highest point is unique",
        ),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED"],
    model: "The largest elevation among the cells that hold data; where several cells share it, the first in reading order (north row first, west to east) is reported with the count of cells that share it. Distances are (column − 1) and (row − 1) cell sizes from the north-west grid point",
    accuracy: "Exact on the elevations given; the true summit can lie between grid points and be higher than any of them",
    when_to_use: "Use this to find the top of a small DEM, a site survey, or a stockpile model: the elevation, where it is in the grid, and, with a cell size, how far it lies east and south of the first point. Ties are counted, so a flat top shows as many cells rather than one.",
    limitations: "The answer is a grid point, not the ground's true high point, which can sit between points and stand higher; a finer grid finds it more closely. Where several cells tie, only the first in reading order is placed. Cells marked with the no-data value are skipped. At most 300 by 300 cells.",
    references: &[GDALINFO],
    examples: &[Example {
        id: "primary",
        title: "A knoll on a 2 m grid, with a tie at the top",
        input: r#"{"elevations":[{"row":"48.2, 48.9, 49.1, 48.7"},{"row":"48.8, 50.3, 50.6, 49.0"},{"row":"48.5, 50.1, 50.6, 48.9"},{"row":"48.0, 48.6, 48.8, 48.4"}],"cell_size":"2 m"}"#,
        source: "NumPy argmax on the same grid (tools/vectors/gen_dem_grid.py)",
    }],
    primary_example: "primary",
    visualization: TABLE,
    related: &[
        Related {
            id: "raster.terrain.dem-stats",
            reason: "alternative",
        },
        Related {
            id: "raster.terrain.tpi",
            reason: "next",
        },
        Related {
            id: "raster.terrain.contours",
            reason: "next",
        },
    ],
    sentence: "The highest point is {highest}, in row {row}, column {column}.",
    limits: &[("batchRows", 1_000)],
    run: run_highest,
    ..ToolDef::BLANK
};

fn run_highest(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let no_data = ctx.number("no_data")?;
    let z = read_grid(ctx, "elevations", no_data)?;
    let cell = cell_size(ctx)?;
    let hi = known(&z)?.into_iter().fold(f64::NEG_INFINITY, f64::max);
    let (mut at, mut ties) = (None, 0.0);
    for (r, row) in z.iter().enumerate() {
        for (c, v) in row.iter().enumerate() {
            if *v == hi {
                at.get_or_insert((r, c));
                ties += 1.0;
            }
        }
    }
    let (r, c) = at.expect("the highest value is in the grid");
    let m = |x| q(x, QT::Length, "m");
    let mut out = vec![
        ("highest", ctx.out("highest", m(hi))),
        ("row", Json::Num((r + 1) as f64)),
        ("column", Json::Num((c + 1) as f64)),
    ];
    if let Some(cell) = cell {
        out.push(("east", ctx.out("east", m(c as f64 * cell))));
        out.push(("south", ctx.out("south", m(r as f64 * cell))));
    }
    out.push(("ties", Json::Num(ties)));
    Ok(Json::obj(out))
}

pub static DEM_DIFFERENCE_VOLUME: ToolDef = ToolDef {
    stability: gp_base::tool::Stability::Stable,
    id: "raster.terrain.dem-difference-volume",
    title: "Volume change between two DEM grids",
    summary: "The volume gained, lost, and net between a before and an after grid of elevations on the same cells, with an optional minimum change below which a cell counts as unchanged.",
    aliases: &[
        "DEM of difference",
        "DoD volume",
        "volume change between surveys",
    ],
    keywords: &[
        "DEM of difference",
        "DoD",
        "volume change",
        "erosion",
        "deposition",
        "stockpile",
        "cut and fill",
        "repeat survey",
    ],
    inputs: &[
        grid(
            "before",
            "Before grid",
            "Rows of elevations in meters from the first survey, north row first, like 10.0, 10.1, 10.2",
        ),
        grid(
            "after",
            "After grid",
            "The same cells from the second survey, like 10.4, 10.6, 10.1",
        ),
        meters(
            "cell_size",
            "Cell size",
            "The ground distance between grid points, like 1 m",
        )
        .required()
        .core(),
        meters(
            "min_change",
            "Minimum change",
            "Smaller changes count as none, like 0.05 m (the default is 0 m)",
        ),
        NO_DATA,
    ],
    outputs: &[
        cubic(
            "net_volume",
            "Net volume change",
            "After less before; positive is a gain",
        ),
        cubic(
            "gained_volume",
            "Volume gained",
            "Where the after surface is higher",
        ),
        cubic(
            "lost_volume",
            "Volume lost",
            "Where the after surface is lower",
        ),
        Field::new(
            "changed_area",
            "Area changed",
            "Cells whose change counts",
            Kind::Quantity {
                q: QT::Area,
                unit: "m2",
            },
        )
        .precision(Precision::Decimals(2)),
        meters(
            "mean_change",
            "Mean change",
            "Over every cell compared, after less before",
        )
        .precision(Precision::Decimals(3)),
        count("cells", "Cells compared", "With data in both grids"),
        count("skipped_cells", "Cells skipped", "No data in either grid"),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED"],
    model: "The DEM of difference: each grid point stands for a cell of the cell size squared, as in a raster, and its change d = after − before is set to zero when |d| is under the minimum change; net volume = Σd × cell², gained = Σ(d > 0) × cell², lost = Σ(−d for d < 0) × cell²",
    accuracy: "Exact for the cell model on the elevations given; the true volume also depends on each survey's vertical error, which the minimum change is there to screen out",
    when_to_use: "Use this when two surveys of the same ground are on the same grid and you want how much material moved: a stockpile measured twice, a slope after a slide, a river bar after a flood, or cut and fill against an existing surface. Give both grids, the cell size, and, if you know your surveys' vertical error, a minimum change so noise does not count as volume.",
    limitations: "Both grids must have the same cells in the same order; this does not resample or align them. Each point stands for a whole square cell, so volumes along the edges and on coarse grids are approximate. A minimum change set too low counts survey noise as volume, and one set too high drops real change; Wheaton and others (2010) set it from the two surveys' errors. At most 300 by 300 cells.",
    references: &[WHEATON],
    examples: &[Example {
        id: "primary",
        title: "A stockpile on a 1 m grid, with a 5 cm minimum change",
        input: r#"{"before":[{"row":"10.0, 10.0, 10.0, 10.0"},{"row":"10.0, 10.0, 10.0, 10.0"},{"row":"10.0, 10.0, 10.0, 10.0"}],"after":[{"row":"10.0, 10.4, 10.6, 10.1"},{"row":"10.2, 11.3, 11.8, 10.3"},{"row":"9.9, 10.2, 10.5, 9.8"}],"cell_size":"1 m","min_change":"0.05 m"}"#,
        source: "NumPy on the same grids (tools/vectors/gen_dem_grid.py)",
    }],
    primary_example: "primary",
    visualization: TABLE,
    related: &[
        Related {
            id: "survey.earthwork.borrow-pit",
            reason: "alternative",
        },
        Related {
            id: "raster.terrain.dem-stats",
            reason: "alternative",
        },
        Related {
            id: "survey.earthwork.shrink-swell",
            reason: "next",
        },
    ],
    sentence: "The surface gained {gained_volume} and lost {lost_volume}, a net change of {net_volume}.",
    limits: &[("batchRows", 1_000)],
    run: run_difference,
    ..ToolDef::BLANK
};

fn run_difference(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let no_data = ctx.number("no_data")?;
    let before = read_grid(ctx, "before", no_data)?;
    let after = read_grid(ctx, "after", no_data)?;
    let shape = |g: &[Vec<f64>]| (g.len(), g[0].len());
    let ((r0, c0), (r1, c1)) = (shape(&before), shape(&after));
    if (r0, c0) != (r1, c1) {
        return Err(ToolError::invalid(
            "/after",
            format!(
                "The after grid is {r1} rows by {c1} columns and the before grid is {r0} by {c0}; they must match cell for cell."
            ),
        ));
    }
    let cell = cell_size(ctx)?.ok_or_else(|| {
        ToolError::invalid("/cell_size", "The cell size is required.").hint("Like 1 m")
    })?;
    let m = units::by_symbol(QT::Length, "m").expect("m");
    let floor = ctx.quantity("min_change")?.map_or(0.0, |v| v.to(m));
    if !(floor.is_finite() && floor >= 0.0) {
        return Err(ToolError::invalid(
            "/min_change",
            "The minimum change cannot be negative.",
        ));
    }
    let (mut gained, mut lost, mut changed, mut compared, mut skipped) = (0.0, 0.0, 0.0, 0.0, 0.0);
    let mut sum = 0.0;
    for (b, a) in before.iter().flatten().zip(after.iter().flatten()) {
        if b.is_nan() || a.is_nan() {
            skipped += 1.0;
            continue;
        }
        compared += 1.0;
        let d = a - b;
        if d.abs() < floor || d == 0.0 {
            continue;
        }
        changed += 1.0;
        sum += d;
        if d > 0.0 {
            gained += d;
        } else {
            lost -= d;
        }
    }
    if compared == 0.0 {
        return Err(ToolError::invalid(
            "/before",
            "No cell has data in both grids.",
        ));
    }
    let area = cell * cell;
    let vol = |x: f64| q(x * area, QT::Volume, "m3");
    Ok(Json::obj([
        ("net_volume", ctx.out("net_volume", vol(sum))),
        ("gained_volume", ctx.out("gained_volume", vol(gained))),
        ("lost_volume", ctx.out("lost_volume", vol(lost))),
        (
            "changed_area",
            ctx.out("changed_area", q(changed * area, QT::Area, "m2")),
        ),
        (
            "mean_change",
            ctx.out("mean_change", q(sum / compared, QT::Length, "m")),
        ),
        ("cells", Json::Num(compared)),
        ("skipped_cells", Json::Num(skipped)),
    ]))
}
