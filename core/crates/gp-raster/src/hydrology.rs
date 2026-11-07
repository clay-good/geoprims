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
