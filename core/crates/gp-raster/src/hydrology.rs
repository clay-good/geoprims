//! Hydrology on a DEM window (add-raster-hydrology, raster/hydrology).
//! Depression removal first: Priority-Flood with an ε slope (Barnes, Lehman,
//! and Mulla 2014), or least-cost breaching with filling for what cannot be
//! breached (Lindsay 2016's hybrid). The grid is the one the reader enters,
//! read the way the contour and DEM-grid tools read it.

use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::contour::{MAX_SIDE, ROW, read_grid};

const BARNES: Reference = Reference {
    title: "Priority-flood: An optimal depression-filling and watershed-labeling algorithm for digital elevation models",
    issuer: "Barnes, R., Lehman, C., and Mulla, D., Computers & Geosciences",
    year: 2014,
    edition: "Volume 62, pages 117-127 (DOI 10.1016/j.cageo.2013.04.024); preprint arXiv:1511.04463",
    locator: "Algorithm 2 (Improved Priority-Flood) and Algorithm 3 (Priority-Flood+ε), which raises each cell to just above the one it was reached from so that every filled cell keeps a descending path to the outlet",
    url: "https://arxiv.org/abs/1511.04463",
};
const LINDSAY: Reference = Reference {
    title: "Efficient hybrid breaching-filling sink removal methods for flow path enforcement in digital elevation models",
    issuer: "Lindsay, J. B., Hydrological Processes",
    year: 2016,
    edition: "Volume 30, issue 6, pages 846-857 (DOI 10.1002/hyp.10648)",
    locator: "The hybrid method: breach a pit along the least-cost path, the cost being the elevation that must be dug out, within a maximum depth and length, and fill what cannot be breached within those limits",
    url: "https://jblindsay.github.io/ghrg/pubs/2016_Lindsay_HP.pdf",
};

/// Cells the window may hold, so one call stays inside the group's memory.
const MAX_CELLS: usize = 25_000;

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
    .precision(Precision::Decimals(3))
}

/// A cell on the priority queue, ordered by elevation, lowest first, with the
/// insertion order breaking ties so one run always gives one answer.
struct Cell {
    z: f64,
    seq: usize,
    idx: usize,
}
impl PartialEq for Cell {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for Cell {}
impl Ord for Cell {
    fn cmp(&self, other: &Self) -> Ordering {
        // BinaryHeap is a max-heap, so reverse: the lowest cell pops first.
        other
            .z
            .partial_cmp(&self.z)
            .unwrap_or(Ordering::Equal)
            .then(other.seq.cmp(&self.seq))
    }
}
impl PartialOrd for Cell {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The eight neighbours of cell `i` on a `w` by `h` grid.
fn neighbours(i: usize, w: usize, h: usize) -> impl Iterator<Item = usize> {
    let (r, c) = (i / w, i % w);
    (-1i64..=1)
        .flat_map(|dr| (-1i64..=1).map(move |dc| (dr, dc)))
        .filter(|(dr, dc)| !(*dr == 0 && *dc == 0))
        .filter_map(move |(dr, dc)| {
            let (nr, nc) = (r as i64 + dr, c as i64 + dc);
            (nr >= 0 && nr < h as i64 && nc >= 0 && nc < w as i64)
                .then(|| nr as usize * w + nc as usize)
        })
}

/// Priority-Flood + ε on a flat grid of `w` by `h`, NaN for no data. Returns
/// the conditioned elevations. Every non-NoData cell ends with a path to the
/// window's edge or to a NoData cell that descends by at least ε a step.
pub(crate) fn priority_flood(z: &[f64], w: usize, h: usize, epsilon: f64) -> Vec<f64> {
    let mut out = z.to_vec();
    let mut closed = vec![false; z.len()];
    let mut open: BinaryHeap<Cell> = BinaryHeap::new();
    let mut seq = 0usize;
    let push =
        |open: &mut BinaryHeap<Cell>, closed: &mut Vec<bool>, i: usize, z: f64, seq: &mut usize| {
            closed[i] = true;
            open.push(Cell {
                z,
                seq: *seq,
                idx: i,
            });
            *seq += 1;
        };
    // Seeds: the window's edge, and every cell that touches no data. Both are
    // outlets, since water leaves the data there.
    for i in 0..z.len() {
        if z[i].is_nan() || closed[i] {
            continue;
        }
        let (r, c) = (i / w, i % w);
        let edge = r == 0 || c == 0 || r + 1 == h || c + 1 == w;
        if edge || neighbours(i, w, h).any(|n| z[n].is_nan()) {
            push(&mut open, &mut closed, i, z[i], &mut seq);
        }
    }
    while let Some(Cell { z: zc, idx, .. }) = open.pop() {
        for n in neighbours(idx, w, h) {
            if closed[n] || out[n].is_nan() {
                continue;
            }
            // Raise the neighbour to just above the cell it was reached from,
            // so the path back out descends (Barnes Algorithm 3).
            if out[n] <= zc + epsilon {
                out[n] = zc + epsilon;
            }
            let zn = out[n];
            push(&mut open, &mut closed, n, zn, &mut seq);
        }
    }
    out
}

/// One pit: a cell with no neighbour at or below it that traps water. A cell
/// on the window's edge or beside no data is not a pit however low it is,
/// because water leaves the data there, which is the outlet the conditioning
/// drains to.
fn pits(z: &[f64], w: usize, h: usize) -> Vec<usize> {
    (0..z.len())
        .filter(|&i| {
            let (r, c) = (i / w, i % w);
            !z[i].is_nan()
                && !(r == 0 || c == 0 || r + 1 == h || c + 1 == w)
                && !neighbours(i, w, h).any(|n| z[n].is_nan())
                && neighbours(i, w, h).all(|n| z[n] > z[i])
        })
        .collect()
}

/// Least-cost breaching of one pit (Lindsay 2016): Dijkstra outward with the
/// cost of a cell the elevation that must be dug from it to carry water at the
/// pit's level, stopping at the first cell lower than the pit. The channel is
/// then cut to descend by ε a step. Returns the cells cut, or None when no
/// outlet is within the depth and length allowed.
fn breach(
    z: &mut [f64],
    w: usize,
    h: usize,
    pit: usize,
    max_depth: f64,
    max_length: usize,
    epsilon: f64,
) -> Option<Vec<usize>> {
    let zp = z[pit];
    let mut cost = vec![f64::INFINITY; z.len()];
    let mut from = vec![usize::MAX; z.len()];
    let mut steps = vec![usize::MAX; z.len()];
    let mut open: BinaryHeap<Cell> = BinaryHeap::new();
    let mut seq = 0usize;
    cost[pit] = 0.0;
    steps[pit] = 0;
    open.push(Cell {
        z: 0.0,
        seq,
        idx: pit,
    });
    seq += 1;
    let mut target = None;
    while let Some(Cell { z: c, idx, .. }) = open.pop() {
        if c > cost[idx] {
            continue;
        }
        if idx != pit && (z[idx] < zp || z[idx].is_nan()) {
            target = Some(idx);
            break;
        }
        if steps[idx] >= max_length {
            continue;
        }
        for n in neighbours(idx, w, h) {
            // Digging to the pit's level, less ε a step down the channel.
            let dig = if z[n].is_nan() {
                0.0
            } else {
                (z[n] - (zp - epsilon * (steps[idx] + 1) as f64)).max(0.0)
            };
            if dig > max_depth {
                continue;
            }
            let next = c + dig;
            if next < cost[n] {
                cost[n] = next;
                from[n] = idx;
                steps[n] = steps[idx] + 1;
                open.push(Cell {
                    z: next,
                    seq,
                    idx: n,
                });
                seq += 1;
            }
        }
    }
    let end = target?;
    // Walk the path back to the pit, cutting each cell to descend by ε.
    let mut path = vec![];
    let mut at = end;
    while at != pit && at != usize::MAX {
        path.push(at);
        at = from[at];
    }
    path.reverse();
    let mut cut = vec![];
    for (k, &i) in path.iter().enumerate() {
        if z[i].is_nan() {
            continue;
        }
        let want = zp - epsilon * (k + 1) as f64;
        if z[i] > want {
            z[i] = want;
            cut.push(i);
        }
    }
    Some(cut)
}

pub static FILL: ToolDef = ToolDef {
    id: "raster.hydrology.fill",
    title: "Fill or breach the pits in a DEM",
    summary: "Removes the depressions that trap water in an elevation grid, by Priority-Flood with a small slope or by cutting a channel out of each pit, and reports every cell it changed and how much earth that is.",
    aliases: &[
        "fill sinks in a DEM",
        "depression filling",
        "remove pits from elevation grid",
        "breach depressions",
    ],
    keywords: &[
        "fill",
        "sink",
        "pit",
        "depression",
        "breach",
        "hydrology",
        "DEM",
        "conditioning",
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
        Field::new(
            "cell_size",
            "Cell size",
            "The ground size of one cell, like 30 m",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .required()
        .core(),
        Field::new(
            "method",
            "Method",
            "Fill the pits, or breach a channel out of each one and fill what is left, like fill",
            Kind::Choice(&["fill", "breach"]),
        )
        .core(),
        Field::new(
            "no_data",
            "No-data value",
            "A value that marks a missing elevation, like -9999",
            Kind::Number {
                min: -1e12,
                max: 1e12,
            },
        ),
        Field::new(
            "epsilon",
            "Slope per cell",
            "How much each filled cell drops toward the outlet, in meters, like 0.00001",
            Kind::Number { min: 0.0, max: 1.0 },
        ),
        meters(
            "max_breach_depth",
            "Deepest cut",
            "How deep a breach channel may be cut, like 10 m",
        ),
        Field::new(
            "max_breach_length",
            "Longest channel",
            "How many cells a breach channel may run, like 100",
            Kind::Number {
                min: 1.0,
                max: 1000.0,
            },
        )
        .precision(Precision::Decimals(0)),
    ],
    outputs: &[
        Field::new(
            "changed_cells",
            "Cells changed",
            "How many cells the conditioning moved",
            Kind::Number { min: 0.0, max: 1e9 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "fill_volume",
            "Fill volume",
            "The earth added, raised height times cell area",
            Kind::Quantity {
                q: QT::Volume,
                unit: "m3",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "cut_volume",
            "Cut volume",
            "The earth taken out by breaching",
            Kind::Quantity {
                q: QT::Volume,
                unit: "m3",
            },
        )
        .precision(Precision::Decimals(2)),
        meters(
            "max_raise",
            "Deepest pit filled",
            "The largest rise at one cell",
        ),
        meters(
            "max_cut",
            "Deepest cut made",
            "The largest drop at one cell",
        ),
        Field::new(
            "pits_before",
            "Pits before",
            "Cells with no neighbor at or below them",
            Kind::Number { min: 0.0, max: 1e9 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "pits_after",
            "Pits left",
            "The same count on the conditioned grid",
            Kind::Number { min: 0.0, max: 1e9 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "conditioned",
            "Conditioned grid",
            "The elevations after conditioning, north row first",
            Kind::List {
                items: ROW,
                min: 2,
                max: MAX_SIDE,
            },
        ),
        Field::new(
            "changes",
            "What moved",
            "Each changed cell: row, column, before, after",
            Kind::List {
                items: CHANGE_ROW,
                min: 0,
                max: 2_000,
            },
        ),
        Field::new(
            "method_used",
            "Method",
            "The method and its settings",
            Kind::Text { max_len: 160 },
        ),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::LimitExceeded],
    warnings: &["BREACH_INCOMPLETE", "EXPERIMENTAL_TOOL"],
    model: "Priority-Flood + ε (Barnes, Lehman, and Mulla 2014, Algorithm 3): every edge cell and every cell touching no data seeds a priority queue ordered by elevation; a cell reached from a lower one is raised to that cell's elevation plus ε, so every filled cell keeps a descending path out. Breaching (Lindsay 2016) first cuts each pit a least-cost channel to a lower cell, the cost being the earth dug at the pit's level less ε a step, within the depth and length allowed, then fills what is left. No-data cells are never changed and never crossed",
    accuracy: "The conditioned grid is exact in the arithmetic of the method: the fill volume is the sum of each cell's rise times the cell area, and after conditioning no cell is left without a path out of the window",
    when_to_use: "Use this before any flow work on a DEM clip: flow directions, accumulation, streams, and watersheds all assume water can leave every cell, and a raw DEM has pits from noise, bridges, and culverts that trap it. Enter the grid and its cell size, pick filling or breaching, and it returns the conditioned grid with every cell it moved and the earth that represents, so you can judge whether the conditioning changed the terrain more than you want.",
    limitations: "It conditions the window you give and treats its edge as an outlet, so a depression that continues past the edge is treated as drained there; clip wider than the basin you care about. Breaching cuts a one-cell channel along a least-cost path and does not widen it, and pits it cannot reach an outlet for within the depth and length allowed are filled instead, which the result says. The grid is plane: cell area is the cell size squared, so a geographic grid in degrees needs its cell size in meters at the latitude. The window holds at most 25,000 cells.",
    references: &[BARNES, LINDSAY],
    examples: &[Example {
        id: "primary",
        title: "A 7 by 7 grid with one pit 3 m deep",
        source: "add-raster-hydrology raster/hydrology, the Single bowl scenario: the pit is raised to the elevation it spills at, and only the bowl's cells move",
        input: r#"{"elevations":[{"row":"100, 100, 100, 100, 100, 100, 100"},{"row":"100, 99, 99, 99, 99, 99, 100"},{"row":"100, 99, 98, 98, 98, 99, 100"},{"row":"100, 99, 98, 95, 98, 99, 100"},{"row":"100, 99, 98, 98, 98, 99, 100"},{"row":"100, 99, 99, 99, 99, 99, 100"},{"row":"100, 100, 100, 100, 100, 100, 100"}],"cell_size":"10 m","method":"fill"}"#,
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "raster.terrain.dem-stats",
            reason: "parent",
        },
        Related {
            id: "raster.terrain.slope",
            reason: "next",
        },
        Related {
            id: "raster.terrain.contours",
            reason: "alternative",
        },
    ],
    sentence: "Conditioning moved {changed_cells} cells.",
    limits: &[("batchRows", 1)],
    run: run_fill,
    ..ToolDef::BLANK
};

const CHANGE_ROW: &[Field] = &[
    Field::new(
        "row",
        "Row",
        "From 1 at the north",
        Kind::Number { min: 1.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "column",
        "Column",
        "From 1 at the west",
        Kind::Number { min: 1.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "before",
        "Before",
        "The elevation entered",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(Precision::Decimals(3)),
    Field::new(
        "after",
        "After",
        "The elevation after conditioning",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(Precision::Decimals(3)),
];

fn run_fill(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let m = units::by_symbol(QT::Length, "m").expect("m");
    let m3 = units::by_symbol(QT::Volume, "m3").expect("m3");
    let no_data = ctx.number("no_data")?;
    let grid = read_grid(ctx, "elevations", no_data)?;
    let (h, w) = (grid.len(), grid[0].len());
    if h * w > MAX_CELLS {
        return Err(ToolError::new(
            ErrorCode::LimitExceeded,
            format!(
                "The window holds {} cells; this tool takes {MAX_CELLS}. Clip it smaller.",
                h * w
            ),
        )
        .at("/elevations"));
    }
    let cell = ctx.req_quantity("cell_size")?.to(m);
    if !(cell.is_finite() && cell > 0.0) {
        return Err(ToolError::invalid(
            "/cell_size",
            "The cell size must be more than zero.",
        ));
    }
    let epsilon = ctx.number("epsilon")?.unwrap_or(1e-5);
    if !(epsilon.is_finite() && epsilon >= 0.0) {
        return Err(ToolError::invalid(
            "/epsilon",
            "The slope per cell must be zero or more.",
        ));
    }
    let method = ctx.choice("method")?.unwrap_or("fill");
    let max_depth = ctx.quantity("max_breach_depth")?.map_or(10.0, |v| v.to(m));
    let max_length = ctx.number("max_breach_length")?.unwrap_or(100.0) as usize;
    if max_depth < 0.0 {
        return Err(ToolError::invalid(
            "/max_breach_depth",
            "The deepest cut must be zero or more.",
        ));
    }

    let before: Vec<f64> = grid.iter().flatten().copied().collect();
    let pits_before = pits(&before, w, h).len();
    let mut work = before.clone();
    let mut unbreached = 0usize;
    if method == "breach" {
        for pit in pits(&before, w, h) {
            if breach(&mut work, w, h, pit, max_depth, max_length, epsilon).is_none() {
                unbreached += 1;
            }
        }
    }
    let after = priority_flood(&work, w, h, epsilon);
    if unbreached > 0 {
        ctx.warnings.push(Warning::new(
            "BREACH_INCOMPLETE",
            format!(
                "{unbreached} of {pits_before} pits had no outlet within {max_depth} m and {max_length} cells, so they were filled instead."
            ),
        ));
    }

    let area = cell * cell;
    let (mut fill_v, mut cut_v, mut max_raise, mut max_cut) = (0.0, 0.0, 0.0f64, 0.0f64);
    let mut changes = vec![];
    for i in 0..after.len() {
        if after[i].is_nan() || before[i].is_nan() {
            continue;
        }
        let d = after[i] - before[i];
        if d.abs() <= 0.0 {
            continue;
        }
        if d > 0.0 {
            fill_v += d * area;
            max_raise = max_raise.max(d);
        } else {
            cut_v += -d * area;
            max_cut = max_cut.max(-d);
        }
        if changes.len() < 2_000 {
            changes.push(Json::obj([
                ("row", Json::Num((i / w + 1) as f64)),
                ("column", Json::Num((i % w + 1) as f64)),
                (
                    "before",
                    Q {
                        value: before[i],
                        unit: m,
                    }
                    .to_json(),
                ),
                (
                    "after",
                    Q {
                        value: after[i],
                        unit: m,
                    }
                    .to_json(),
                ),
            ]));
        }
    }
    let changed = after
        .iter()
        .zip(&before)
        .filter(|(a, b)| !a.is_nan() && !b.is_nan() && *a != *b)
        .count();
    let pits_after = pits(&after, w, h).len();
    let rows: Vec<Json> = after
        .chunks(w)
        .map(|r| {
            let text = r
                .iter()
                .map(|v| {
                    // Shortest round trip, not a fixed three decimals: the ε
                    // slope that makes the grid drain is 1e-5 m, and rounding
                    // it away would hand the flow tools a flat.
                    if v.is_nan() {
                        no_data.map_or_else(
                            || "NaN".to_string(),
                            |n| gp_base::num::format_f64(n).unwrap_or_else(|| "NaN".to_string()),
                        )
                    } else {
                        gp_base::num::format_f64(*v).unwrap_or_else(|| "NaN".to_string())
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            Json::obj([("row", Json::str(text))])
        })
        .collect();
    let used = match method {
        "breach" => format!(
            "least-cost breaching to {max_depth} m deep and {max_length} cells, then Priority-Flood with {epsilon} m a cell"
        ),
        _ => format!("Priority-Flood with {epsilon} m a cell"),
    };
    if ctx.explaining() {
        ctx.step(
            "Pits found",
            "A cell with no neighbor at or below it",
            format!("{} cells", h * w),
            format!("{pits_before} pits"),
        );
        ctx.step(
            "Conditioning",
            "Priority-Flood from the window's edge and its no-data boundary",
            used.clone(),
            format!("{pits_after} pits left"),
        );
        ctx.step(
            "What moved",
            "Cells whose elevation changed",
            format!("{fill_v:.2} m³ added, {cut_v:.2} m³ taken out"),
            format!("{changed}"),
        );
    }
    Ok(Json::obj([
        ("changed_cells", Json::Num(changed as f64)),
        (
            "fill_volume",
            ctx.out(
                "fill_volume",
                Q {
                    value: fill_v,
                    unit: m3,
                },
            ),
        ),
        (
            "cut_volume",
            ctx.out(
                "cut_volume",
                Q {
                    value: cut_v,
                    unit: m3,
                },
            ),
        ),
        (
            "max_raise",
            ctx.out(
                "max_raise",
                Q {
                    value: max_raise,
                    unit: m,
                },
            ),
        ),
        (
            "max_cut",
            ctx.out(
                "max_cut",
                Q {
                    value: max_cut,
                    unit: m,
                },
            ),
        ),
        ("pits_before", Json::Num(pits_before as f64)),
        ("pits_after", Json::Num(pits_after as f64)),
        ("conditioned", Json::Arr(rows)),
        ("changes", Json::Arr(changes)),
        ("method_used", Json::str(used)),
    ]))
}

// --------------------------------------------------------- flow direction

const OCALLAGHAN: Reference = Reference {
    title: "The extraction of drainage networks from digital elevation data",
    issuer: "O'Callaghan, J. F., and Mark, D. M., Computer Vision, Graphics, and Image Processing",
    year: 1984,
    edition: "Volume 28, issue 3, pages 323-344 (DOI 10.1016/S0734-189X(84)80011-0)",
    locator: "The steepest-descent rule that sends each cell's water to one of its eight neighbours, the D8 method",
    url: "https://doi.org/10.1016/S0734-189X(84)80011-0",
};
const TARBOTON: Reference = Reference {
    title: "A new method for the determination of flow directions and upslope areas in grid digital elevation models",
    issuer: "Tarboton, D. G., Water Resources Research",
    year: 1997,
    edition: "Volume 33, issue 2, pages 309-319 (DOI 10.1029/96WR03137)",
    locator: "Equations (1) to (6) and Table 1: the steepest slope over the eight triangular facets of a 3 by 3 window, and the facet factors a_c and a_f that turn a facet's angle into one measured counterclockwise from east",
    url: "https://hydrology.usu.edu/dtarb/96WR03137.pdf",
};
const WHITEBOX_POINTER: Reference = Reference {
    title: "D8 flow pointer (direction), Whitebox GAT help",
    issuer: "Lindsay, J. B., Guelph Hydrologic Research Group",
    year: 2026,
    edition: "Whitebox GAT help page, read 2026-11-06",
    locator: "The clockwise base-2 pointer convention, in which a cell that flows to the northwest is assigned 64, and cells with no lower neighbour are assigned zero",
    url: "https://jblindsay.github.io/ghrg/Whitebox/Help/FlowPointerD8.html",
};
const TAUDEM_D8: Reference = Reference {
    title: "D8 Flow Directions, TauDEM 5 help",
    issuer: "Tarboton, D. G., Utah State University",
    year: 2026,
    edition: "TauDEM 5 documentation, read 2026-11-06",
    locator: "D8 flow direction coding: 1 east through 8 southeast, counterclockwise from east",
    url: "https://hydrology.usu.edu/taudem/taudem5/help/d8flowdirections.html",
};

/// The eight neighbours in ESRI's order, east first and clockwise, with the
/// row and column step, the ESRI code, the WhiteboxTools code, and the TauDEM
/// code. A tie in slope goes to the first of these, as the spec requires.
const DIRS: [(i64, i64, u32, u32, u32); 8] = [
    (0, 1, 1, 2, 1),     // east
    (1, 1, 2, 4, 8),     // south-east
    (1, 0, 4, 8, 7),     // south
    (1, -1, 8, 16, 6),   // south-west
    (0, -1, 16, 32, 5),  // west
    (-1, -1, 32, 64, 4), // north-west
    (-1, 0, 64, 128, 3), // north
    (-1, 1, 128, 1, 2),  // north-east
];

/// A step from the centre cell, as a row and column offset.
type Step = (i64, i64);
/// Tarboton's Table 1: for each facet, the cardinal neighbour e1, the diagonal
/// neighbour e2, and the factors a_c and a_f of his equation (6).
const FACETS: [(Step, Step, f64, f64); 8] = [
    ((0, 1), (-1, 1), 0.0, 1.0),
    ((-1, 0), (-1, 1), 1.0, -1.0),
    ((-1, 0), (-1, -1), 1.0, 1.0),
    ((0, -1), (-1, -1), 2.0, -1.0),
    ((0, -1), (1, -1), 2.0, 1.0),
    ((1, 0), (1, -1), 3.0, -1.0),
    ((1, 0), (1, 1), 3.0, 1.0),
    ((0, 1), (1, 1), 4.0, -1.0),
];

fn at(z: &[f64], w: usize, h: usize, r: i64, c: i64) -> Option<f64> {
    (r >= 0 && r < h as i64 && c >= 0 && c < w as i64)
        .then(|| z[r as usize * w + c as usize])
        .filter(|v| !v.is_nan())
}

/// D8 for one cell: the neighbour of steepest descent, as an index into DIRS,
/// with its slope. None when nothing around it is lower.
fn d8(z: &[f64], w: usize, h: usize, i: usize, cell: f64) -> Option<(usize, f64)> {
    let (r, c) = ((i / w) as i64, (i % w) as i64);
    let z0 = z[i];
    let mut best: Option<(usize, f64)> = None;
    for (k, (dr, dc, ..)) in DIRS.iter().enumerate() {
        let Some(zn) = at(z, w, h, r + dr, c + dc) else {
            continue;
        };
        let dist = if dr.abs() + dc.abs() == 2 {
            cell * std::f64::consts::SQRT_2
        } else {
            cell
        };
        let slope = (z0 - zn) / dist;
        if slope > 0.0 && best.is_none_or(|(_, s)| slope > s) {
            best = Some((k, slope));
        }
    }
    best
}

/// D-infinity for one cell (Tarboton 1997): the steepest of the eight facets,
/// as an angle counterclockwise from east in radians and its slope. None when
/// no facet descends.
fn dinf(z: &[f64], w: usize, h: usize, i: usize, cell: f64) -> Option<(f64, f64)> {
    let (r, c) = ((i / w) as i64, (i % w) as i64);
    let e0 = z[i];
    // atan2(d2, d1) of Tarboton's equation (5). The cells here are square, so
    // d1 = d2 = the cell size and the facet's far edge is at π/4.
    let mut best: Option<(f64, f64)> = None;
    for ((dr1, dc1), (dr2, dc2), ac, af) in FACETS {
        let (Some(e1), Some(e2)) = (at(z, w, h, r + dr1, c + dc1), at(z, w, h, r + dr2, c + dc2))
        else {
            continue;
        };
        let (s1, s2) = ((e0 - e1) / cell, (e1 - e2) / cell);
        let (mut rr, mut s) = (s2.atan2(s1), s1.hypot(s2));
        if rr < 0.0 {
            rr = 0.0;
            s = s1;
        } else if rr > std::f64::consts::FRAC_PI_4 {
            rr = std::f64::consts::FRAC_PI_4;
            s = (e0 - e2) / (cell * cell + cell * cell).sqrt();
        }
        if s > 0.0 && best.is_none_or(|(_, bs)| s > bs) {
            let rg = (af * rr + ac * std::f64::consts::FRAC_PI_2).rem_euclid(std::f64::consts::TAU);
            best = Some((rg, s));
        }
    }
    best
}

pub static FLOW_DIRECTION: ToolDef = ToolDef {
    id: "raster.hydrology.flow-direction",
    title: "Flow direction over a DEM",
    summary: "Which way water leaves every cell of an elevation grid, by steepest descent to one of eight neighbors or by the continuous D-infinity angle, in the pointer encoding your software reads.",
    aliases: &[
        "D8 flow direction",
        "flow pointer grid",
        "D-infinity flow direction",
        "steepest descent direction",
    ],
    keywords: &[
        "flow",
        "direction",
        "D8",
        "pointer",
        "D-infinity",
        "hydrology",
        "DEM",
        "drainage",
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
        Field::new(
            "cell_size",
            "Cell size",
            "The ground size of one cell, like 30 m",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .required()
        .core(),
        Field::new(
            "method",
            "Method",
            "One neighbor by steepest descent, or the D-infinity angle, like d8",
            Kind::Choice(&["d8", "d-infinity"]),
        )
        .core(),
        Field::new(
            "encoding",
            "Pointer encoding",
            "Which table of codes the directions use, like esri",
            Kind::Choice(&["esri", "whitebox", "taudem"]),
        )
        .core(),
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
            "resolved",
            "Cells with a direction",
            "How many cells drain somewhere",
            Kind::Number { min: 0.0, max: 1e9 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "off_grid",
            "Cells draining out of the window",
            "On the edge or beside no data, with nothing lower inside",
            Kind::Number { min: 0.0, max: 1e9 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "no_outflow",
            "Cells with nowhere to go",
            "Interior cells with no lower neighbor; condition the grid first",
            Kind::Number { min: 0.0, max: 1e9 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "directions",
            "Direction grid",
            "The pointer code for each cell, north row first",
            Kind::List {
                items: ROW,
                min: 2,
                max: MAX_SIDE,
            },
        ),
        Field::new(
            "angles",
            "Angle grid",
            "D-infinity angles in radians counterclockwise from east, −1 where unresolved",
            Kind::List {
                items: ROW,
                min: 0,
                max: MAX_SIDE,
            },
        )
        .optional(),
        Field::new(
            "steepest",
            "Steepest descent",
            "The largest slope found, rise over run",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Decimals(6)),
        Field::new(
            "encoding_used",
            "Encoding",
            "The table the codes come from",
            Kind::Text { max_len: 200 },
        ),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::LimitExceeded],
    warnings: &["PITS_PRESENT", "EXPERIMENTAL_TOOL"],
    model: "D8 (O'Callaghan and Mark 1984): the neighbour of steepest descent, slope (z0 − zn) / d with d the cell size, or √2 times it diagonally; ties go to the first neighbour in ESRI's order, east and clockwise. D-infinity (Tarboton 1997, equations 1 to 6 and Table 1): the steepest of the eight triangular facets of the 3 by 3 window, as an angle counterclockwise from east. Codes: ESRI (E 1, SE 2, S 4, SW 8, W 16, NW 32, N 64, NE 128), WhiteboxTools (clockwise base two from NE 1, so E 2 and NW 64), or TauDEM (1 east through 8 south-east, counterclockwise); 0 where a cell has no lower neighbour",
    accuracy: "Exact in the arithmetic of each published rule: the direction is the steepest of the eight, and the D-infinity angle is the facet angle of Tarboton's equation (6)",
    when_to_use: "Use this after conditioning a DEM to see where water goes from each cell, or to produce the pointer grid another tool or GIS expects. Pick the encoding your software reads: ESRI's powers of two, WhiteboxTools's clockwise table, or TauDEM's 1 to 8. D-infinity gives the continuous angle instead, which spreads flow more faithfully on a hillslope but cannot be followed cell to cell.",
    limitations: "It is the published rule and nothing more: flats have no steepest descent, so a cell with no lower neighbour gets 0 and the result counts them, which is why a conditioned grid comes first. Cells on the window's edge, and cells beside no data, drain out of the data when nothing inside is lower. The grid is plane: cell area and diagonal distance come from the cell size, so a geographic grid needs its cell size in meters at the latitude. The window holds at most 25,000 cells.",
    references: &[OCALLAGHAN, TARBOTON, WHITEBOX_POINTER, TAUDEM_D8],
    examples: &[Example {
        id: "primary",
        title: "A 10 by 10 ramp falling to the east",
        source: "add-raster-hydrology raster/hydrology, the Ramp scenario: every cell points east, which is 1 in the ESRI table",
        input: r#"{"elevations":[{"row":"100, 99, 98, 97, 96, 95, 94, 93, 92, 91"},{"row":"100, 99, 98, 97, 96, 95, 94, 93, 92, 91"},{"row":"100, 99, 98, 97, 96, 95, 94, 93, 92, 91"},{"row":"100, 99, 98, 97, 96, 95, 94, 93, 92, 91"},{"row":"100, 99, 98, 97, 96, 95, 94, 93, 92, 91"},{"row":"100, 99, 98, 97, 96, 95, 94, 93, 92, 91"},{"row":"100, 99, 98, 97, 96, 95, 94, 93, 92, 91"},{"row":"100, 99, 98, 97, 96, 95, 94, 93, 92, 91"},{"row":"100, 99, 98, 97, 96, 95, 94, 93, 92, 91"},{"row":"100, 99, 98, 97, 96, 95, 94, 93, 92, 91"}],"cell_size":"30 m","method":"d8","encoding":"esri"}"#,
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "raster.hydrology.fill",
            reason: "parent",
        },
        Related {
            id: "raster.terrain.aspect",
            reason: "alternative",
        },
        Related {
            id: "raster.terrain.slope",
            reason: "next",
        },
    ],
    sentence: "{resolved} cells have a flow direction.",
    limits: &[("batchRows", 1)],
    run: run_flow_direction,
    ..ToolDef::BLANK
};

fn run_flow_direction(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let m = units::by_symbol(QT::Length, "m").expect("m");
    let no_data = ctx.number("no_data")?;
    let grid = read_grid(ctx, "elevations", no_data)?;
    let (h, w) = (grid.len(), grid[0].len());
    if h * w > MAX_CELLS {
        return Err(ToolError::new(
            ErrorCode::LimitExceeded,
            format!(
                "The window holds {} cells; this tool takes {MAX_CELLS}. Clip it smaller.",
                h * w
            ),
        )
        .at("/elevations"));
    }
    let cell = ctx.req_quantity("cell_size")?.to(m);
    if !(cell.is_finite() && cell > 0.0) {
        return Err(ToolError::invalid(
            "/cell_size",
            "The cell size must be more than zero.",
        ));
    }
    let method = ctx.choice("method")?.unwrap_or("d8");
    let encoding = ctx.choice("encoding")?.unwrap_or("esri");
    let code = |k: usize| match encoding {
        "whitebox" => DIRS[k].3,
        "taudem" => DIRS[k].4,
        _ => DIRS[k].2,
    };
    let z: Vec<f64> = grid.iter().flatten().copied().collect();
    let (mut resolved, mut off_grid, mut no_outflow, mut steepest) =
        (0usize, 0usize, 0usize, 0.0f64);
    let mut codes = vec![0u32; z.len()];
    let mut angles = vec![-1.0f64; z.len()];
    for i in 0..z.len() {
        if z[i].is_nan() {
            continue;
        }
        let (r, c) = (i / w, i % w);
        let border = r == 0
            || c == 0
            || r + 1 == h
            || c + 1 == w
            || neighbours(i, w, h).any(|n| z[n].is_nan());
        match method {
            "d-infinity" => match dinf(&z, w, h, i, cell) {
                Some((angle, slope)) => {
                    angles[i] = angle;
                    steepest = steepest.max(slope);
                    resolved += 1;
                    // The pointer grid still carries D8, so a reader can follow
                    // the water cell to cell, as every published D-infinity
                    // implementation keeps one.
                    if let Some((k, _)) = d8(&z, w, h, i, cell) {
                        codes[i] = code(k);
                    }
                }
                None => {
                    if border {
                        off_grid += 1;
                    } else {
                        no_outflow += 1;
                    }
                }
            },
            _ => match d8(&z, w, h, i, cell) {
                Some((k, slope)) => {
                    codes[i] = code(k);
                    steepest = steepest.max(slope);
                    resolved += 1;
                }
                None => {
                    if border {
                        off_grid += 1;
                    } else {
                        no_outflow += 1;
                    }
                }
            },
        }
    }
    if no_outflow > 0 {
        ctx.warnings.push(Warning::new(
            "PITS_PRESENT",
            format!(
                "{no_outflow} interior cells have no lower neighbour, so water stops there. Condition the grid with the fill tool first."
            ),
        ));
    }
    let rows_of = |f: &dyn Fn(usize) -> String| -> Vec<Json> {
        (0..h)
            .map(|r| {
                let text = (0..w).map(|c| f(r * w + c)).collect::<Vec<_>>().join(", ");
                Json::obj([("row", Json::str(text))])
            })
            .collect()
    };
    let nd = |i: usize| z[i].is_nan();
    let dir_rows = rows_of(&|i| {
        if nd(i) {
            no_data.map_or_else(
                || "NaN".to_string(),
                |n| gp_base::num::format_f64(n).unwrap_or_else(|| "NaN".to_string()),
            )
        } else {
            codes[i].to_string()
        }
    });
    let named = match encoding {
        "whitebox" => {
            "WhiteboxTools: clockwise base two from NE 1, so E 2, SE 4, S 8, SW 16, W 32, NW 64, N 128; 0 where nothing is lower"
        }
        "taudem" => {
            "TauDEM: 1 east, 2 north-east, 3 north, 4 north-west, 5 west, 6 south-west, 7 south, 8 south-east; 0 where nothing is lower"
        }
        _ => "ESRI: E 1, SE 2, S 4, SW 8, W 16, NW 32, N 64, NE 128; 0 where nothing is lower",
    };
    if ctx.explaining() {
        ctx.step(
            "Steepest descent",
            "slope = (z0 − zn) / d, d the cell size or √2 times it diagonally",
            format!("{} cells of {cell} m", h * w),
            format!("steepest {steepest:.6}"),
        );
        ctx.step(
            "Direction",
            if method == "d-infinity" {
                "the steepest of the eight triangular facets (Tarboton 1997)"
            } else {
                "the neighbour of steepest descent (D8)"
            },
            named.to_string(),
            format!("{resolved}"),
        );
    }
    let mut out = vec![
        ("resolved", Json::Num(resolved as f64)),
        ("off_grid", Json::Num(off_grid as f64)),
        ("no_outflow", Json::Num(no_outflow as f64)),
        ("directions", Json::Arr(dir_rows)),
    ];
    if method == "d-infinity" {
        let angle_rows = rows_of(&|i| {
            if nd(i) {
                no_data.map_or_else(
                    || "NaN".to_string(),
                    |n| gp_base::num::format_f64(n).unwrap_or_else(|| "NaN".to_string()),
                )
            } else {
                gp_base::num::format_f64(angles[i]).unwrap_or_else(|| "NaN".to_string())
            }
        });
        out.push(("angles", Json::Arr(angle_rows)));
    }
    out.push(("steepest", Json::Num(steepest)));
    out.push(("encoding_used", Json::str(named)));
    Ok(Json::obj(out))
}

// ------------------------------------------------------ flow accumulation

/// D8 accumulation: every cell counts itself plus everything that drains
/// through it, summed in topological order (Kahn), so each cell is added once.
fn accumulate_d8(z: &[f64], w: usize, h: usize, cell: f64) -> Vec<f64> {
    let n = z.len();
    let mut to = vec![usize::MAX; n];
    let mut indegree = vec![0u32; n];
    for i in 0..n {
        if z[i].is_nan() {
            continue;
        }
        if let Some((k, _)) = d8(z, w, h, i, cell) {
            let (dr, dc, ..) = DIRS[k];
            let j = ((i / w) as i64 + dr) as usize * w + ((i % w) as i64 + dc) as usize;
            to[i] = j;
            indegree[j] += 1;
        }
    }
    let mut acc = vec![0.0; n];
    for (i, a) in acc.iter_mut().enumerate() {
        if !z[i].is_nan() {
            *a = 1.0;
        }
    }
    let mut queue: Vec<usize> = (0..n)
        .filter(|&i| !z[i].is_nan() && indegree[i] == 0)
        .collect();
    while let Some(i) = queue.pop() {
        let j = to[i];
        if j == usize::MAX {
            continue;
        }
        acc[j] += acc[i];
        indegree[j] -= 1;
        if indegree[j] == 0 {
            queue.push(j);
        }
    }
    acc
}

/// D-infinity accumulation (Tarboton 1997): a cell's flow is split between the
/// two neighbours its angle lies between, in proportion to how close it is to
/// each. Cells are summed from the highest down, so every contribution is in
/// before a cell is passed on.
fn accumulate_dinf(z: &[f64], w: usize, h: usize, cell: f64) -> Vec<f64> {
    let n = z.len();
    let mut order: Vec<usize> = (0..n).filter(|&i| !z[i].is_nan()).collect();
    order.sort_by(|a, b| z[*b].partial_cmp(&z[*a]).unwrap_or(Ordering::Equal));
    let mut acc = vec![0.0; n];
    for &i in &order {
        acc[i] += 1.0;
    }
    for &i in &order {
        let Some((angle, _)) = dinf(z, w, h, i, cell) else {
            continue;
        };
        // The angle sits between two of the eight directions, 45° apart,
        // counted counterclockwise from east.
        let step = std::f64::consts::FRAC_PI_4;
        let k = (angle / step).floor();
        let frac = angle / step - k;
        // DIRS is clockwise from east; the angle is counterclockwise.
        let ccw = |m: i64| DIRS[((8 - m).rem_euclid(8)) as usize];
        let (first, second) = (ccw(k as i64), ccw(k as i64 + 1));
        for ((dr, dc, ..), share) in [(first, 1.0 - frac), (second, frac)] {
            if share <= 0.0 {
                continue;
            }
            let (r, c) = ((i / w) as i64 + dr, (i % w) as i64 + dc);
            if r < 0 || r >= h as i64 || c < 0 || c >= w as i64 {
                continue;
            }
            let j = r as usize * w + c as usize;
            if z[j].is_nan() {
                continue;
            }
            acc[j] += acc[i] * share;
        }
    }
    acc
}

pub static FLOW_ACCUMULATION: ToolDef = ToolDef {
    id: "raster.hydrology.flow-accumulation",
    title: "Flow accumulation over a DEM",
    summary: "How much ground drains through every cell of an elevation grid, as a count of upslope cells and an area, by D8 or by D-infinity's split flow.",
    aliases: &[
        "flow accumulation grid",
        "upslope area",
        "contributing area",
        "drainage area from a DEM",
    ],
    keywords: &[
        "accumulation",
        "upslope",
        "contributing",
        "catchment",
        "drainage",
        "area",
        "hydrology",
        "DEM",
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
        Field::new(
            "cell_size",
            "Cell size",
            "The ground size of one cell, like 30 m",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .required()
        .core(),
        Field::new(
            "method",
            "Method",
            "One neighbor by steepest descent, or D-infinity's split flow, like d8",
            Kind::Choice(&["d8", "d-infinity"]),
        )
        .core(),
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
            "max_cells",
            "Most cells draining through one cell",
            "Counting the cell itself",
            Kind::Number {
                min: 0.0,
                max: 1e12,
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "max_area",
            "The area that drains through it",
            "Cells times the area of one cell",
            Kind::Quantity {
                q: QT::Area,
                unit: "m2",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "outlet_row",
            "Its row",
            "From 1 at the north",
            Kind::Number { min: 1.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "outlet_column",
            "Its column",
            "From 1 at the west",
            Kind::Number { min: 1.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "accumulation",
            "Accumulation grid",
            "Cells draining through each cell, north row first",
            Kind::List {
                items: ROW,
                min: 2,
                max: MAX_SIDE,
            },
        ),
        Field::new(
            "pits",
            "Cells with nowhere to go",
            "Interior cells that trap what reaches them",
            Kind::Number { min: 0.0, max: 1e9 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "edge_note",
            "At the window's edge",
            "Why an edge value is a lower bound",
            Kind::Text { max_len: 200 },
        ),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::LimitExceeded],
    warnings: &["PITS_PRESENT", "EXPERIMENTAL_TOOL"],
    model: "D8: each cell sends its whole count to the neighbour of steepest descent, summed in topological order, so a cell's count is 1 plus the counts of every cell pointing to it. D-infinity (Tarboton 1997): a cell's flow is split between the two neighbours its angle lies between, in proportion to the angle, and cells are summed from the highest down. Area = count × cell size²",
    accuracy: "Exact in the arithmetic of each rule: the D8 count is an integer and satisfies the conservation identity at every cell, and the D-infinity split is the angle's own proportion",
    when_to_use: "Use this after conditioning a DEM to find where water collects: the count is how many cells drain through each one, and the area is that count times a cell. Channels show up as the cells with high accumulation, which is what a stream threshold picks out. D8 keeps whole cells and is what a watershed needs; D-infinity splits the flow, which is smoother on a hillslope.",
    limitations: "The window is all the tool knows: a cell near the edge counts only what drains to it from inside the grid, so its value is a lower bound, and a basin that reaches in from outside is not counted. An unconditioned pit swallows everything that reaches it, so fill or breach the grid first; the result counts the pits it found. The grid is plane: the cell area is the cell size squared, so a geographic grid needs its cell size in meters at the latitude. The window holds at most 25,000 cells.",
    references: &[OCALLAGHAN, TARBOTON],
    examples: &[Example {
        id: "primary",
        title: "A 10 by 10 ramp falling to the east",
        source: "add-raster-hydrology raster/hydrology, the East ramp accumulation scenario: each row reads 1, 2, 3 … 10 from west to east, counting the cell itself",
        input: r#"{"elevations":[{"row":"100, 99, 98, 97, 96, 95, 94, 93, 92, 91"},{"row":"100, 99, 98, 97, 96, 95, 94, 93, 92, 91"},{"row":"100, 99, 98, 97, 96, 95, 94, 93, 92, 91"},{"row":"100, 99, 98, 97, 96, 95, 94, 93, 92, 91"},{"row":"100, 99, 98, 97, 96, 95, 94, 93, 92, 91"},{"row":"100, 99, 98, 97, 96, 95, 94, 93, 92, 91"},{"row":"100, 99, 98, 97, 96, 95, 94, 93, 92, 91"},{"row":"100, 99, 98, 97, 96, 95, 94, 93, 92, 91"},{"row":"100, 99, 98, 97, 96, 95, 94, 93, 92, 91"},{"row":"100, 99, 98, 97, 96, 95, 94, 93, 92, 91"}],"cell_size":"30 m","method":"d8"}"#,
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "raster.hydrology.flow-direction",
            reason: "parent",
        },
        Related {
            id: "raster.hydrology.fill",
            reason: "alternative",
        },
        Related {
            id: "raster.terrain.dem-stats",
            reason: "next",
        },
    ],
    sentence: "At most {max_cells} cells drain through one cell.",
    limits: &[("batchRows", 1)],
    run: run_flow_accumulation,
    ..ToolDef::BLANK
};

fn run_flow_accumulation(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let m = units::by_symbol(QT::Length, "m").expect("m");
    let m2 = units::by_symbol(QT::Area, "m2").expect("m2");
    let no_data = ctx.number("no_data")?;
    let grid = read_grid(ctx, "elevations", no_data)?;
    let (h, w) = (grid.len(), grid[0].len());
    if h * w > MAX_CELLS {
        return Err(ToolError::new(
            ErrorCode::LimitExceeded,
            format!(
                "The window holds {} cells; this tool takes {MAX_CELLS}. Clip it smaller.",
                h * w
            ),
        )
        .at("/elevations"));
    }
    let cell = ctx.req_quantity("cell_size")?.to(m);
    if !(cell.is_finite() && cell > 0.0) {
        return Err(ToolError::invalid(
            "/cell_size",
            "The cell size must be more than zero.",
        ));
    }
    let method = ctx.choice("method")?.unwrap_or("d8");
    let z: Vec<f64> = grid.iter().flatten().copied().collect();
    let acc = if method == "d-infinity" {
        accumulate_dinf(&z, w, h, cell)
    } else {
        accumulate_d8(&z, w, h, cell)
    };
    let pit_count = pits(&z, w, h).len();
    if pit_count > 0 {
        ctx.warnings.push(Warning::new(
            "PITS_PRESENT",
            format!(
                "{pit_count} interior cells have no lower neighbour, so everything reaching them stops there. Condition the grid with the fill tool first."
            ),
        ));
    }
    let (mut best, mut best_at) = (0.0f64, 0usize);
    for (i, a) in acc.iter().enumerate() {
        if !z[i].is_nan() && *a > best {
            best = *a;
            best_at = i;
        }
    }
    let rows: Vec<Json> = (0..h)
        .map(|r| {
            let text = (0..w)
                .map(|c| {
                    let i = r * w + c;
                    if z[i].is_nan() {
                        no_data.map_or_else(
                            || "NaN".to_string(),
                            |n| gp_base::num::format_f64(n).unwrap_or_else(|| "NaN".to_string()),
                        )
                    } else {
                        gp_base::num::format_f64(acc[i]).unwrap_or_else(|| "NaN".to_string())
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            Json::obj([("row", Json::str(text))])
        })
        .collect();
    if ctx.explaining() {
        ctx.step(
            "Where the water goes",
            if method == "d-infinity" {
                "each cell's flow split between the two neighbours its angle lies between"
            } else {
                "each cell's flow to the neighbour of steepest descent"
            },
            format!("{} cells of {cell} m", h * w),
            format!("{pit_count} pits"),
        );
        ctx.step(
            "Accumulation",
            "a cell counts itself plus everything draining through it",
            format!("{} cells", h * w),
            // As the answer card shows it: two decimals with the trailing
            // zeros dropped, so a whole count reads as a whole number.
            {
                let s = format!("{:.2}", (best * 100.0).round() / 100.0);
                if s.contains('.') {
                    s.trim_end_matches('0').trim_end_matches('.').to_string()
                } else {
                    s
                }
            },
        );
    }
    Ok(Json::obj([
        ("max_cells", Json::Num((best * 100.0).round() / 100.0)),
        (
            "max_area",
            ctx.out(
                "max_area",
                Q {
                    value: best * cell * cell,
                    unit: m2,
                },
            ),
        ),
        ("outlet_row", Json::Num((best_at / w + 1) as f64)),
        ("outlet_column", Json::Num((best_at % w + 1) as f64)),
        ("accumulation", Json::Arr(rows)),
        ("pits", Json::Num(pit_count as f64)),
        (
            "edge_note",
            Json::str(
                "A cell near the window's edge counts only what drains to it from inside the grid, so its value is a lower bound.",
            ),
        ),
    ]))
}

// ------------------------------------------------------------- streams

const STRAHLER: Reference = Reference {
    title: "Quantitative analysis of watershed geomorphology",
    issuer: "Strahler, A. N., Transactions, American Geophysical Union",
    year: 1957,
    edition: "Volume 38, issue 6, pages 913-920 (DOI 10.1029/TR038i006p00913)",
    locator: "The stream-order rule: a channel with no tributary is first order, two channels of the same order join into the next order, and a channel joined by a lower order keeps its own",
    url: "https://doi.org/10.1029/TR038i006p00913",
};

const LINK_ROW: &[Field] = &[
    Field::new(
        "link",
        "Link",
        "Its number, from 1, like 3",
        Kind::Number { min: 1.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "order",
        "Strahler order",
        "1 for a headwater, like 2",
        Kind::Number {
            min: 1.0,
            max: 20.0,
        },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "length",
        "Length",
        "Along the cells it runs through",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(Precision::Decimals(2)),
    Field::new(
        "outlet_area",
        "Drainage area at its outlet",
        "Cells draining through its last cell, times the cell area",
        Kind::Quantity {
            q: QT::Area,
            unit: "m2",
        },
    )
    .precision(Precision::Decimals(1)),
    Field::new(
        "cells",
        "Cells",
        "How many channel cells it runs through",
        Kind::Number { min: 1.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
];
const VERTEX_ROW: &[Field] = &[
    Field::new(
        "link",
        "Link",
        "The link this vertex belongs to, like 3",
        Kind::Number { min: 1.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "east",
        "Distance east",
        "From the north-west grid point, like 45.5 m",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(Precision::Decimals(3)),
    Field::new(
        "south",
        "Distance south",
        "From the north-west grid point, like 12.25 m",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(Precision::Decimals(3)),
    Field::new(
        "lat",
        "Latitude",
        "Present when the grid's north-west corner is given, north positive",
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
        "Present when the grid's north-west corner is given, east positive",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .optional()
    .precision(Precision::Decimals(7))
    .angle_range("[-180,180)"),
];
const ORDER_ROW: &[Field] = &[
    Field::new(
        "order",
        "Strahler order",
        "1 for a headwater, like 2",
        Kind::Number {
            min: 1.0,
            max: 20.0,
        },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "links",
        "Links of this order",
        "How many, like 7",
        Kind::Number { min: 0.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "length",
        "Their length in all",
        "Added along the cells",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(Precision::Decimals(2)),
];

pub static STREAMS: ToolDef = ToolDef {
    id: "raster.hydrology.streams",
    title: "Stream network and Strahler order",
    summary: "The channels in an elevation grid: the cells where enough ground drains through, split into links, each with its Strahler order, its length, and the area draining through its outlet.",
    aliases: &[
        "stream network from a DEM",
        "Strahler stream order",
        "channel extraction",
        "drainage network",
    ],
    keywords: &[
        "stream",
        "channel",
        "Strahler",
        "order",
        "network",
        "drainage",
        "hydrology",
        "DEM",
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
        Field::new(
            "cell_size",
            "Cell size",
            "The ground size of one cell, like 30 m",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .required()
        .core(),
        Field::new(
            "threshold_cells",
            "Threshold, in cells",
            "How many cells must drain through a cell for it to be a channel, like 20",
            Kind::Number { min: 1.0, max: 1e9 },
        )
        .core()
        .precision(Precision::Decimals(0)),
        Field::new(
            "threshold_area",
            "Threshold, as an area",
            "The same threshold as ground area instead, like 0.02 km2",
            Kind::Quantity {
                q: QT::Area,
                unit: "m2",
            },
        )
        .core(),
        Field::new(
            "north_west_lat",
            "North-west latitude",
            "Of the grid's first cell center, if you want the links placed, like 44.12",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        ),
        Field::new(
            "north_west_lon",
            "North-west longitude",
            "Of the grid's first cell center, like -121.45",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
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
            "highest_order",
            "Highest order",
            "The main stem's Strahler order",
            Kind::Number {
                min: 0.0,
                max: 20.0,
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "links_found",
            "Links",
            "Stretches between junctions",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "channel_cells",
            "Channel cells",
            "Cells at or above the threshold",
            Kind::Number { min: 0.0, max: 1e9 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "network_length",
            "Network length",
            "Every link added together",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "threshold_used",
            "Threshold used",
            "In cells, whichever way you gave it",
            Kind::Number { min: 1.0, max: 1e9 },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "max_accumulation",
            "Most cells draining through one cell",
            "So a threshold past this finds nothing",
            Kind::Number {
                min: 0.0,
                max: 1e12,
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "by_order",
            "Links by order",
            "How many links of each order, and their length",
            Kind::List {
                items: ORDER_ROW,
                min: 0,
                max: 20,
            },
        ),
        Field::new(
            "links",
            "The links",
            "Each stretch between junctions",
            Kind::List {
                items: LINK_ROW,
                min: 0,
                max: 5_000,
            },
        ),
        Field::new(
            "vertices",
            "Link vertices",
            "The cell centers each link runs through, in order",
            Kind::List {
                items: VERTEX_ROW,
                min: 0,
                max: 25_000,
            },
        ),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::LimitExceeded],
    warnings: &["PITS_PRESENT", "EMPTY_NETWORK", "EXPERIMENTAL_TOOL"],
    model: "Channel cells are those whose D8 accumulation reaches the threshold. Strahler (1957): a channel cell with no channel cell draining into it is order 1; where channels meet, two of the same order make the next order and a lower one leaves the order alone. A link runs from a source or a junction down to the next junction or the network's outlet; its length adds the cell size along sides and √2 times it along diagonals, and its outlet area is the accumulation at its last cell times the cell area",
    accuracy: "Exact in the arithmetic of the rules: the order follows Strahler's definition cell by cell, and the length is the walked path of cell centers",
    when_to_use: "Use this to pull the channels out of a conditioned DEM and see how the network branches: which stretches are headwaters, which are the main stem, how long each is, and how much ground drains through its outlet. Set the threshold by how much upslope area makes a channel in your landscape, as cells or as an area; raise it for fewer, longer streams.",
    limitations: "The threshold is the whole model of what a channel is: there is no rule here for picking it, and a different threshold gives a different network. Links are the cells themselves, so a length is the walked path of cell centers and reads longer than a smooth channel, especially where the path staircases. The window is all the tool knows, so a stream entering from outside starts at the edge. An unconditioned pit ends a channel in the middle of the grid; condition the grid first. The window holds at most 25,000 cells.",
    references: &[OCALLAGHAN, STRAHLER],
    examples: &[Example {
        id: "primary",
        title: "Two headwaters joining into one stream",
        source: "add-raster-hydrology raster/hydrology, the Order rule scenario: two first-order links join into a second-order link",
        input: r#"{"elevations":[{"row":"60, 48, 60, 60, 60, 48, 60"},{"row":"60, 60, 47, 60, 47, 60, 60"},{"row":"60, 60, 60, 46, 60, 60, 60"},{"row":"60, 60, 60, 45, 60, 60, 60"},{"row":"60, 60, 60, 44, 60, 60, 60"},{"row":"60, 60, 60, 43, 60, 60, 60"},{"row":"60, 60, 60, 42, 60, 60, 60"}],"cell_size":"30 m","threshold_cells":3}"#,
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "raster.hydrology.flow-accumulation",
            reason: "parent",
        },
        Related {
            id: "raster.hydrology.fill",
            reason: "alternative",
        },
        Related {
            id: "raster.terrain.contours",
            reason: "next",
        },
    ],
    sentence: "The network reaches Strahler order {highest_order}.",
    limits: &[("batchRows", 1)],
    run: run_streams,
    ..ToolDef::BLANK
};

fn run_streams(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let m = units::by_symbol(QT::Length, "m").expect("m");
    let m2 = units::by_symbol(QT::Area, "m2").expect("m2");
    let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let no_data = ctx.number("no_data")?;
    let grid = read_grid(ctx, "elevations", no_data)?;
    let (h, w) = (grid.len(), grid[0].len());
    if h * w > MAX_CELLS {
        return Err(ToolError::new(
            ErrorCode::LimitExceeded,
            format!(
                "The window holds {} cells; this tool takes {MAX_CELLS}. Clip it smaller.",
                h * w
            ),
        )
        .at("/elevations"));
    }
    let cell = ctx.req_quantity("cell_size")?.to(m);
    if !(cell.is_finite() && cell > 0.0) {
        return Err(ToolError::invalid(
            "/cell_size",
            "The cell size must be more than zero.",
        ));
    }
    let by_cells = ctx.number("threshold_cells")?;
    let by_area = ctx.quantity("threshold_area")?.map(|q| q.to(m2));
    let threshold = match (by_cells, by_area) {
        (Some(_), Some(_)) => {
            return Err(ToolError::invalid(
                "/threshold_area",
                "Give the threshold as cells or as an area, not both.",
            ));
        }
        (Some(c), None) => c,
        (None, Some(a)) => a / (cell * cell),
        (None, None) => {
            return Err(ToolError::invalid(
                "/threshold_cells",
                "Give the threshold: how many cells, or how much ground, must drain through a cell for it to count as a channel.",
            ));
        }
    };
    if !(threshold.is_finite() && threshold >= 1.0) {
        return Err(ToolError::invalid(
            "/threshold_cells",
            "The threshold is at least one cell.",
        ));
    }
    let lat0 = ctx.quantity("north_west_lat")?.map(|q| q.to(deg));
    let lon0 = ctx.quantity("north_west_lon")?.map(|q| q.to(deg));
    if lat0.is_some() != lon0.is_some() {
        return Err(ToolError::invalid(
            "/north_west_lon",
            "Give both the latitude and the longitude of the north-west cell, or neither.",
        ));
    }

    let z: Vec<f64> = grid.iter().flatten().copied().collect();
    let acc = accumulate_d8(&z, w, h, cell);
    let pit_count = pits(&z, w, h).len();
    if pit_count > 0 {
        ctx.warnings.push(Warning::new(
            "PITS_PRESENT",
            format!(
                "{pit_count} interior cells have no lower neighbour, so a channel can end in the middle of the grid. Condition it with the fill tool first."
            ),
        ));
    }
    let max_acc = acc
        .iter()
        .zip(&z)
        .filter(|(_, e)| !e.is_nan())
        .map(|(a, _)| *a)
        .fold(0.0, f64::max);
    let channel: Vec<bool> = (0..z.len())
        .map(|i| !z[i].is_nan() && acc[i] >= threshold)
        .collect();
    // Where each channel cell sends its water, and what drains into it.
    let mut to = vec![usize::MAX; z.len()];
    let mut upstream = vec![0u32; z.len()];
    for i in 0..z.len() {
        if !channel[i] {
            continue;
        }
        if let Some((k, _)) = d8(&z, w, h, i, cell) {
            let (dr, dc, ..) = DIRS[k];
            let j = ((i / w) as i64 + dr) as usize * w + ((i % w) as i64 + dc) as usize;
            if channel[j] {
                to[i] = j;
                upstream[j] += 1;
            }
        }
    }
    // Strahler order, upstream first: Kahn over the channel graph.
    let mut order = vec![0u32; z.len()];
    let mut best_in: Vec<(u32, u32)> = vec![(0, 0); z.len()]; // highest order seen, how many times
    let mut left = upstream.clone();
    let mut ready: Vec<usize> = (0..z.len())
        .filter(|&i| channel[i] && left[i] == 0)
        .collect();
    let mut sorted = vec![];
    while let Some(i) = ready.pop() {
        let (top, count) = best_in[i];
        order[i] = if top == 0 {
            1
        } else if count >= 2 {
            top + 1
        } else {
            top
        };
        sorted.push(i);
        let j = to[i];
        if j == usize::MAX {
            continue;
        }
        let (top_j, count_j) = best_in[j];
        best_in[j] = match order[i].cmp(&top_j) {
            Ordering::Greater => (order[i], 1),
            Ordering::Equal => (top_j, count_j + 1),
            Ordering::Less => (top_j, count_j),
        };
        left[j] -= 1;
        if left[j] == 0 {
            ready.push(j);
        }
    }
    // Links: from a source or a junction down to the next junction, or to the
    // network's outlet.
    let head = |i: usize| channel[i] && (upstream[i] == 0 || upstream[i] >= 2);
    let mut links: Vec<Json> = vec![];
    let mut vertices: Vec<Json> = vec![];
    let mut by_order: Vec<(u32, u32, f64)> = vec![];
    let mut total_length = 0.0;
    let place = |i: usize| -> (f64, f64) {
        (
            (i % w) as f64 * cell + cell / 2.0,
            (i / w) as f64 * cell + cell / 2.0,
        )
    };
    for start in (0..z.len()).filter(|&i| head(i)) {
        let number = links.len() + 1;
        let (mut at, mut length, mut cells) = (start, 0.0, 1usize);
        let push_vertex = |at: usize, vertices: &mut Vec<Json>| {
            let (east, south) = place(at);
            let mut row = vec![
                ("link", Json::Num(number as f64)),
                (
                    "east",
                    Q {
                        value: east,
                        unit: m,
                    }
                    .to_json(),
                ),
                (
                    "south",
                    Q {
                        value: south,
                        unit: m,
                    }
                    .to_json(),
                ),
            ];
            if let (Some(la), Some(lo)) = (lat0, lon0) {
                // Degrees of latitude and longitude at the grid's own latitude,
                // the placement the contour tool uses.
                let dlat = south / 111_132.0;
                let lat = la - dlat;
                let dlon = east / (111_320.0 * la.to_radians().cos()).max(1e-9);
                row.push((
                    "lat",
                    Q {
                        value: lat,
                        unit: deg,
                    }
                    .to_json(),
                ));
                row.push((
                    "lon",
                    Q {
                        value: lo + dlon,
                        unit: deg,
                    }
                    .to_json(),
                ));
            }
            vertices.push(Json::obj(row));
        };
        push_vertex(at, &mut vertices);
        loop {
            let next = to[at];
            if next == usize::MAX {
                break;
            }
            let (dr, dc) = (
                (next / w) as i64 - (at / w) as i64,
                (next % w) as i64 - (at % w) as i64,
            );
            length += if dr.abs() + dc.abs() == 2 {
                cell * std::f64::consts::SQRT_2
            } else {
                cell
            };
            at = next;
            cells += 1;
            push_vertex(at, &mut vertices);
            if head(at) {
                break; // the next junction ends this link
            }
        }
        let ord = order[start];
        total_length += length;
        match by_order.iter_mut().find(|(o, ..)| *o == ord) {
            Some((_, n, l)) => {
                *n += 1;
                *l += length;
            }
            None => by_order.push((ord, 1, length)),
        }
        links.push(Json::obj([
            ("link", Json::Num(number as f64)),
            ("order", Json::Num(f64::from(ord))),
            (
                "length",
                Q {
                    value: length,
                    unit: m,
                }
                .to_json(),
            ),
            (
                "outlet_area",
                Q {
                    value: acc[at] * cell * cell,
                    unit: m2,
                }
                .to_json(),
            ),
            ("cells", Json::Num(cells as f64)),
        ]));
    }
    by_order.sort_by_key(|(o, ..)| *o);
    let highest = order.iter().copied().max().unwrap_or(0);
    let channel_cells = channel.iter().filter(|c| **c).count();
    if channel_cells == 0 {
        ctx.warnings.push(Warning::new(
            "EMPTY_NETWORK",
            format!(
                "No cell reaches the threshold of {threshold} cells; the most that drains through any cell here is {max_acc}."
            ),
        ));
    }
    if ctx.explaining() {
        ctx.step(
            "Channel cells",
            "cells whose accumulation reaches the threshold",
            format!("threshold {threshold} cells of {}", h * w),
            format!("{channel_cells} cells"),
        );
        ctx.step(
            "Strahler order",
            "two channels of one order make the next; a lower one leaves it alone",
            format!("{} links", links.len()),
            format!("{highest}"),
        );
    }
    Ok(Json::obj([
        ("highest_order", Json::Num(f64::from(highest))),
        ("links_found", Json::Num(links.len() as f64)),
        ("channel_cells", Json::Num(channel_cells as f64)),
        (
            "network_length",
            ctx.out(
                "network_length",
                Q {
                    value: total_length,
                    unit: m,
                },
            ),
        ),
        ("threshold_used", Json::Num(threshold)),
        ("max_accumulation", Json::Num(max_acc)),
        (
            "by_order",
            Json::Arr(
                by_order
                    .iter()
                    .map(|(o, n, l)| {
                        Json::obj([
                            ("order", Json::Num(f64::from(*o))),
                            ("links", Json::Num(f64::from(*n))),
                            ("length", Q { value: *l, unit: m }.to_json()),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("links", Json::Arr(links)),
        ("vertices", Json::Arr(vertices)),
    ]))
}

// ----------------------------------------------------- watershed and path

const JENSON: Reference = Reference {
    title: "Extracting topographic structure from digital elevation data for geographic information system analysis",
    issuer: "Jenson, S. K., and Domingue, J. O., Photogrammetric Engineering and Remote Sensing",
    year: 1988,
    edition: "Volume 54, issue 11, pages 1593-1600",
    locator: "Delineating a watershed as every cell whose flow path reaches the outlet, and snapping an outlet onto the drainage network before delineating",
    url: "https://www.asprs.org/wp-content/uploads/pers/1988journal/nov/1988_nov_1593-1600.pdf",
};

/// Every cell whose D8 path reaches `outlet`, the outlet included.
fn basin_of(z: &[f64], w: usize, h: usize, cell: f64, outlet: usize) -> Vec<bool> {
    let n = z.len();
    let mut from: Vec<Vec<usize>> = vec![vec![]; n];
    for i in 0..n {
        if z[i].is_nan() {
            continue;
        }
        if let Some((k, _)) = d8(z, w, h, i, cell) {
            let (dr, dc, ..) = DIRS[k];
            let j = ((i / w) as i64 + dr) as usize * w + ((i % w) as i64 + dc) as usize;
            from[j].push(i);
        }
    }
    let mut inside = vec![false; n];
    let mut stack = vec![outlet];
    inside[outlet] = true;
    while let Some(i) = stack.pop() {
        for &u in &from[i] {
            if !inside[u] {
                inside[u] = true;
                stack.push(u);
            }
        }
    }
    inside
}

/// The longest D8 path ending at `outlet`, as the cells along it from the top
/// down, with its length in meters.
fn longest_path(z: &[f64], w: usize, h: usize, cell: f64, outlet: usize) -> (Vec<usize>, f64) {
    let n = z.len();
    let mut from: Vec<Vec<usize>> = vec![vec![]; n];
    for i in 0..n {
        if z[i].is_nan() {
            continue;
        }
        if let Some((k, _)) = d8(z, w, h, i, cell) {
            let (dr, dc, ..) = DIRS[k];
            let j = ((i / w) as i64 + dr) as usize * w + ((i % w) as i64 + dc) as usize;
            from[j].push(i);
        }
    }
    // Depth-first from the outlet upstream, keeping the longest branch.
    let mut best_len = vec![0.0f64; n];
    let mut best_up = vec![usize::MAX; n];
    let mut order = vec![];
    let mut stack = vec![(outlet, false)];
    let mut seen = vec![false; n];
    seen[outlet] = true;
    while let Some((i, done)) = stack.pop() {
        if done {
            order.push(i);
            continue;
        }
        stack.push((i, true));
        for &u in &from[i] {
            if !seen[u] {
                seen[u] = true;
                stack.push((u, false));
            }
        }
    }
    for &i in &order {
        for &u in &from[i] {
            let (dr, dc) = (
                (i / w) as i64 - (u / w) as i64,
                (i % w) as i64 - (u % w) as i64,
            );
            let step = if dr.abs() + dc.abs() == 2 {
                cell * std::f64::consts::SQRT_2
            } else {
                cell
            };
            let len = best_len[u] + step;
            if len > best_len[i] {
                best_len[i] = len;
                best_up[i] = u;
            }
        }
    }
    let mut path = vec![outlet];
    let mut at = outlet;
    while best_up[at] != usize::MAX {
        at = best_up[at];
        path.push(at);
    }
    path.reverse();
    (path, best_len[outlet])
}

/// Where a pour point lands once snapped, and how it was snapped.
/// How a pour point is moved onto the network before delineating.
struct Snap<'a> {
    rule: &'a str,
    threshold: f64,
    within: usize,
}

fn snap(z: &[f64], acc: &[f64], w: usize, h: usize, at: usize, how: &Snap<'_>) -> (usize, String) {
    let Snap {
        rule,
        threshold,
        within,
    } = *how;
    let (r0, c0) = ((at / w) as i64, (at % w) as i64);
    let mut best: Option<(usize, f64, i64)> = None;
    for dr in -(within as i64)..=(within as i64) {
        for dc in -(within as i64)..=(within as i64) {
            let (r, c) = (r0 + dr, c0 + dc);
            if r < 0 || r >= h as i64 || c < 0 || c >= w as i64 {
                continue;
            }
            let i = r as usize * w + c as usize;
            if z[i].is_nan() {
                continue;
            }
            let d2 = dr * dr + dc * dc;
            let take = match rule {
                "highest-accumulation" => best.is_none_or(|(_, a, _)| acc[i] > a),
                // Nearest stream cell: the closest cell at or above the
                // threshold, the one with more water breaking a tie.
                _ => {
                    acc[i] >= threshold
                        && best.is_none_or(|(_, a, bd)| d2 < bd || (d2 == bd && acc[i] > a))
                }
            };
            if take {
                best = Some((i, acc[i], d2));
            }
        }
    }
    match best {
        Some((i, _, d2)) if i != at => (
            i,
            format!(
                "snapped {} cells to the {}",
                (f64::from(d2 as i32)).sqrt().round(),
                if rule == "highest-accumulation" {
                    "cell with the most water within reach"
                } else {
                    "nearest channel cell"
                }
            ),
        ),
        _ => (at, "the cell you gave, unsnapped".to_string()),
    }
}

const PATH_ROW: &[Field] = &[
    Field::new(
        "step",
        "Step",
        "From 1 at the top of the path, like 3",
        Kind::Number { min: 1.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "row",
        "Row",
        "From 1 at the north",
        Kind::Number { min: 1.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "column",
        "Column",
        "From 1 at the west",
        Kind::Number { min: 1.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "elevation",
        "Elevation",
        "At that cell",
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(Precision::Decimals(3)),
];

fn pour_point(ctx: &mut Ctx, w: usize, h: usize, z: &[f64]) -> Result<usize, ToolError> {
    let row = ctx.number("pour_row")?.ok_or_else(|| {
        ToolError::invalid(
            "/pour_row",
            "Give the pour point's row, counting from 1 at the north.",
        )
    })?;
    let col = ctx.number("pour_column")?.ok_or_else(|| {
        ToolError::invalid(
            "/pour_column",
            "Give the pour point's column, counting from 1 at the west.",
        )
    })?;
    let (r, c) = (row.round() as i64 - 1, col.round() as i64 - 1);
    if r < 0 || r >= h as i64 || c < 0 || c >= w as i64 {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            format!("The pour point is outside the window, which is {h} rows by {w} columns."),
        )
        .at("/pour_row"));
    }
    let i = r as usize * w + c as usize;
    if z[i].is_nan() {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The pour point is on a no-data cell; pick one with an elevation.",
        )
        .at("/pour_row"));
    }
    Ok(i)
}

pub static WATERSHED: ToolDef = ToolDef {
    id: "raster.hydrology.watershed",
    title: "Watershed above a pour point",
    summary: "Every cell of an elevation grid whose water reaches the point you pick, with the basin's area, relief, mean slope, outline, and its longest flow path.",
    aliases: &[
        "watershed delineation",
        "catchment above a point",
        "drainage basin from a DEM",
        "pour point watershed",
    ],
    keywords: &[
        "watershed",
        "catchment",
        "basin",
        "pour",
        "outlet",
        "delineation",
        "hydrology",
        "DEM",
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
        Field::new(
            "cell_size",
            "Cell size",
            "The ground size of one cell, like 30 m",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .required()
        .core(),
        Field::new(
            "pour_row",
            "Pour point row",
            "From 1 at the north, like 7",
            Kind::Number { min: 1.0, max: 1e6 },
        )
        .required()
        .core(),
        Field::new(
            "pour_column",
            "Pour point column",
            "From 1 at the west, like 4",
            Kind::Number { min: 1.0, max: 1e6 },
        )
        .required()
        .core(),
        Field::new(
            "snap",
            "Snap rule",
            "Move the point onto the network first, like nearest-stream",
            Kind::Choice(&["nearest-stream", "highest-accumulation", "none"]),
        )
        .core(),
        Field::new(
            "threshold_cells",
            "Stream threshold, in cells",
            "What counts as a channel for the nearest-stream rule, like 20",
            Kind::Number { min: 1.0, max: 1e9 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "snap_within",
            "Search distance",
            "How many cells out to look when snapping, like 3",
            Kind::Number {
                min: 0.0,
                max: 50.0,
            },
        )
        .precision(Precision::Decimals(0)),
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
            "basin_area",
            "Basin area",
            "The cells that drain to the outlet, times the cell area",
            Kind::Quantity {
                q: QT::Area,
                unit: "m2",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "basin_cells",
            "Basin cells",
            "How many cells drain to the outlet",
            Kind::Number { min: 1.0, max: 1e9 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "outlet_row",
            "Outlet row",
            "After snapping, from 1 at the north",
            Kind::Number { min: 1.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "outlet_column",
            "Outlet column",
            "After snapping, from 1 at the west",
            Kind::Number { min: 1.0, max: 1e6 },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "snap_note",
            "How the outlet was placed",
            "Whether the point moved, and why",
            Kind::Text { max_len: 120 },
        ),
        Field::new(
            "relief",
            "Relief",
            "Highest cell in the basin above the outlet",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "mean_slope",
            "Mean slope",
            "Averaged over the basin's cells, rise over run",
            Kind::Number { min: 0.0, max: 1e4 },
        )
        .precision(Precision::Decimals(4)),
        Field::new(
            "longest_path",
            "Longest flow path",
            "From the farthest cell down to the outlet",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "path_drop",
            "Drop along that path",
            "Its first cell above the outlet",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "touches_edge",
            "Reaches the window's edge",
            "Yes when the basin may continue beyond the data",
            Kind::Text { max_len: 8 },
        ),
        Field::new(
            "basin",
            "Basin grid",
            "1 for a cell in the basin, 0 for the rest, north row first",
            Kind::List {
                items: ROW,
                min: 2,
                max: MAX_SIDE,
            },
        ),
        Field::new(
            "path",
            "The longest path",
            "Its cells from the top down",
            Kind::List {
                items: PATH_ROW,
                min: 1,
                max: 25_000,
            },
        ),
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::OutOfDomain,
        ErrorCode::LimitExceeded,
    ],
    warnings: &[
        "PITS_PRESENT",
        "WATERSHED_TOUCHES_EDGE",
        "EXPERIMENTAL_TOOL",
    ],
    model: "The basin is every cell whose D8 path reaches the outlet (Jenson and Domingue 1988), found by walking the pointers upstream from it. The outlet is first snapped, by the nearest channel cell at the threshold or by the most water within the search distance. Area = cells × cell size²; relief = the basin's highest cell above the outlet; mean slope averages each cell's steepest descent; the longest flow path is the longest chain of pointers ending at the outlet, summing the cell size along sides and √2 times it along diagonals",
    accuracy: "Exact in the arithmetic of the rules over the window given: the basin is every cell the pointers lead from, and the path length is the walked chain of cell centres",
    when_to_use: "Use this to find what drains to a point: a culvert, a gauge, a reservoir, a sample site. Give the conditioned grid, the cell size, and the point's row and column, and let the tool snap it onto the channel first, since a point one cell off the channel delineates a hillside instead of a basin. It returns the basin's cells and area, its relief and mean slope, the longest flow path for a time-of-concentration estimate, and the basin as a grid you can draw.",
    limitations: "The window bounds the answer: when the basin reaches the edge, the real basin may continue beyond the data, which the result says and warns. An unconditioned pit cuts a basin short, so fill or breach first. The grid is plane, so the area is the cell count times the cell size squared rather than a geodesic area; a geographic grid needs its cell size in meters at the latitude. Mean slope is the average of the cells' steepest descents, not a fitted surface. The window holds at most 25,000 cells.",
    references: &[JENSON, OCALLAGHAN],
    examples: &[Example {
        id: "primary",
        title: "The basin above the bottom of a valley",
        source: "add-raster-hydrology raster/hydrology: the cells whose water reaches the valley's outlet, with the longest path down it",
        input: r#"{"elevations":[{"row":"60, 48, 60, 60, 60, 48, 60"},{"row":"60, 60, 47, 60, 47, 60, 60"},{"row":"60, 60, 60, 46, 60, 60, 60"},{"row":"60, 60, 60, 45, 60, 60, 60"},{"row":"60, 60, 60, 44, 60, 60, 60"},{"row":"60, 60, 60, 43, 60, 60, 60"},{"row":"60, 60, 60, 42, 60, 60, 60"}],"cell_size":"30 m","pour_row":7,"pour_column":4,"snap":"none"}"#,
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "raster.hydrology.streams",
            reason: "parent",
        },
        Related {
            id: "raster.hydrology.flow-accumulation",
            reason: "alternative",
        },
        Related {
            id: "raster.hydrology.fill",
            reason: "next",
        },
    ],
    sentence: "That point drains {basin_area}.",
    limits: &[("batchRows", 1)],
    run: run_watershed,
    ..ToolDef::BLANK
};

fn run_watershed(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let m = units::by_symbol(QT::Length, "m").expect("m");
    let m2 = units::by_symbol(QT::Area, "m2").expect("m2");
    let no_data = ctx.number("no_data")?;
    let grid = read_grid(ctx, "elevations", no_data)?;
    let (h, w) = (grid.len(), grid[0].len());
    if h * w > MAX_CELLS {
        return Err(ToolError::new(
            ErrorCode::LimitExceeded,
            format!(
                "The window holds {} cells; this tool takes {MAX_CELLS}. Clip it smaller.",
                h * w
            ),
        )
        .at("/elevations"));
    }
    let cell = ctx.req_quantity("cell_size")?.to(m);
    if !(cell.is_finite() && cell > 0.0) {
        return Err(ToolError::invalid(
            "/cell_size",
            "The cell size must be more than zero.",
        ));
    }
    let z: Vec<f64> = grid.iter().flatten().copied().collect();
    let given = pour_point(ctx, w, h, &z)?;
    let acc = accumulate_d8(&z, w, h, cell);
    let rule = ctx.choice("snap")?.unwrap_or("nearest-stream");
    let threshold = ctx.number("threshold_cells")?.unwrap_or_else(|| {
        // A tenth of a percent of the window, at least five cells: enough to
        // be a channel on a small grid, and stated in the note either way.
        ((h * w) as f64 * 0.001).max(5.0).round()
    });
    let within = ctx.number("snap_within")?.unwrap_or(3.0).round() as usize;
    let (outlet, note) = if rule == "none" {
        (given, "the cell you gave, unsnapped".to_string())
    } else {
        snap(
            &z,
            &acc,
            w,
            h,
            given,
            &Snap {
                rule,
                threshold,
                within,
            },
        )
    };
    let pit_count = pits(&z, w, h).len();
    if pit_count > 0 {
        ctx.warnings.push(Warning::new(
            "PITS_PRESENT",
            format!(
                "{pit_count} interior cells have no lower neighbour, so a basin can stop short of its real edge. Condition the grid with the fill tool first."
            ),
        ));
    }
    let inside = basin_of(&z, w, h, cell, outlet);
    let cells = inside.iter().filter(|b| **b).count();
    let mut touches = false;
    let (mut high, mut slope_sum, mut slope_n) = (z[outlet], 0.0, 0usize);
    for (i, b) in inside.iter().enumerate() {
        if !b {
            continue;
        }
        let (r, c) = (i / w, i % w);
        if r == 0 || c == 0 || r + 1 == h || c + 1 == w {
            touches = true;
        }
        high = high.max(z[i]);
        if let Some((_, s)) = d8(&z, w, h, i, cell) {
            slope_sum += s;
            slope_n += 1;
        }
    }
    if touches {
        ctx.warnings.push(Warning::new(
            "WATERSHED_TOUCHES_EDGE",
            "The basin reaches the window's edge, so the real basin may continue beyond the data.",
        ));
    }
    let (path, path_len) = longest_path(&z, w, h, cell, outlet);
    let drop = z[*path.first().expect("the outlet is on the path")] - z[outlet];
    let basin_rows: Vec<Json> = (0..h)
        .map(|r| {
            let text = (0..w)
                .map(|c| {
                    let i = r * w + c;
                    if z[i].is_nan() {
                        no_data.map_or_else(
                            || "NaN".to_string(),
                            |n| gp_base::num::format_f64(n).unwrap_or_else(|| "NaN".to_string()),
                        )
                    } else if inside[i] {
                        "1".to_string()
                    } else {
                        "0".to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            Json::obj([("row", Json::str(text))])
        })
        .collect();
    let path_rows: Vec<Json> = path
        .iter()
        .enumerate()
        .map(|(k, &i)| {
            Json::obj([
                ("step", Json::Num((k + 1) as f64)),
                ("row", Json::Num((i / w + 1) as f64)),
                ("column", Json::Num((i % w + 1) as f64)),
                (
                    "elevation",
                    Q {
                        value: z[i],
                        unit: m,
                    }
                    .to_json(),
                ),
            ])
        })
        .collect();
    if ctx.explaining() {
        ctx.step(
            "The outlet",
            "the pour point, snapped onto the network",
            format!("row {}, column {}", given / w + 1, given % w + 1),
            note.clone(),
        );
        ctx.step(
            "The basin",
            "every cell whose flow path reaches the outlet",
            format!("{} cells in the window", h * w),
            format!("{cells} cells"),
        );
        let area = cells as f64 * cell * cell;
        ctx.step(
            "Its area",
            "cells × cell size²",
            format!("{cells} × {cell} m × {cell} m"),
            // Grouped the way the answer card shows it, so the trace ends on
            // exactly the number the reader is looking at.
            format!(
                "{} m²",
                gp_base::display::number(area, Precision::Decimals(1), ctx.options.format)
            ),
        );
    }
    Ok(Json::obj([
        (
            "basin_area",
            ctx.out(
                "basin_area",
                Q {
                    value: cells as f64 * cell * cell,
                    unit: m2,
                },
            ),
        ),
        ("basin_cells", Json::Num(cells as f64)),
        ("outlet_row", Json::Num((outlet / w + 1) as f64)),
        ("outlet_column", Json::Num((outlet % w + 1) as f64)),
        ("snap_note", Json::str(note)),
        (
            "relief",
            ctx.out(
                "relief",
                Q {
                    value: high - z[outlet],
                    unit: m,
                },
            ),
        ),
        (
            "mean_slope",
            Json::Num(if slope_n == 0 {
                0.0
            } else {
                slope_sum / slope_n as f64
            }),
        ),
        (
            "longest_path",
            ctx.out(
                "longest_path",
                Q {
                    value: path_len,
                    unit: m,
                },
            ),
        ),
        (
            "path_drop",
            ctx.out(
                "path_drop",
                Q {
                    value: drop,
                    unit: m,
                },
            ),
        ),
        (
            "touches_edge",
            Json::str(if touches { "yes" } else { "no" }),
        ),
        ("basin", Json::Arr(basin_rows)),
        ("path", Json::Arr(path_rows)),
    ]))
}
