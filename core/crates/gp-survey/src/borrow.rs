//! Grid (borrow-pit) volumes (add-survey-suite, survey/earthwork-and-grade,
//! "Grid (borrow-pit) method"): the net volume by corner weights 1, 2, and 4,
//! cut and fill by the four-point method with transition cells split, and the
//! balance line where cut turns to fill along the grid's edges.

use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use serde_json::{Map, Value};

use crate::{common_unit, len, len_out, unit};

const GHILANI: Reference = Reference {
    title: "Elementary Surveying: An Introduction to Geomatics",
    issuer: "Ghilani, C. D., and Wolf, P. R., Pearson",
    year: 2018,
    edition: "15th edition",
    locator: "Chapter 26 (volumes: unit-area or borrow-pit method)",
    url: "https://www.pearson.com/en-us/subject-catalog/p/elementary-surveying-an-introduction-to-geomatics/P200000003148",
};

const ROW: &[Field] = &[Field::new(
    "elevations",
    "Elevations",
    "One grid row, west to east, comma separated, like 101.2, 100.8, 100.1",
    Kind::Text { max_len: 4000 },
)
.required()];
const BALANCE: &[Field] = &[
    len_out("x", "East", "From the grid's first column"),
    len_out("y", "South", "From the grid's first row"),
];

pub static BORROW_PIT: ToolDef = ToolDef {
    id: "survey.earthwork.borrow-pit",
    title: "Grid (borrow-pit) volume",
    summary: "Cut and fill over a grid of existing elevations against a finished grade: the net volume by corner weights and the four-point method, with the balance line where cut turns to fill.",
    aliases: &[
        "borrow pit method",
        "grid volume",
        "unit area method",
        "cut and fill grid",
    ],
    keywords: &[
        "borrow pit",
        "grid",
        "cut",
        "fill",
        "volume",
        "corner weights",
        "four point",
        "balance line",
        "earthwork",
    ],
    inputs: &[
        Field::new(
            "existing",
            "Existing elevations",
            "Grid rows north to south, one per line, like 101.2, 100.8, 100.1",
            Kind::List {
                items: ROW,
                min: 2,
                max: 500,
            },
        )
        .required()
        .core(),
        len(
            "cell_size",
            "Cell size",
            "The grid spacing, the same both ways, like 25 ft",
        )
        .required()
        .core(),
        len(
            "finished_grade",
            "Finished grade",
            "One design elevation for the whole grid, like 100.0 ft",
        )
        .core(),
        Field::new(
            "proposed",
            "Proposed elevations",
            "Instead of one grade: a matching grid, like 100.5, 100.3, 100.1",
            Kind::List {
                items: ROW,
                min: 2,
                max: 500,
            },
        ),
    ],
    outputs: &[
        Field::new(
            "net_volume",
            "Net volume (corner weights)",
            "Σ(weight × depth) × cell area / 4, cut positive",
            Kind::Quantity {
                q: QT::Volume,
                unit: "ft3",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "cut_volume",
            "Cut volume (four-point)",
            "Cells averaged, transition cells split",
            Kind::Quantity {
                q: QT::Volume,
                unit: "ft3",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "fill_volume",
            "Fill volume (four-point)",
            "Cells averaged, transition cells split",
            Kind::Quantity {
                q: QT::Volume,
                unit: "ft3",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "net_cubic_yards",
            "Net volume, cubic yards",
            "Net volume ÷ 27 when the grid is in feet",
            Kind::Quantity {
                q: QT::Volume,
                unit: "yd3",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "balance_points",
            "Balance line",
            "Where cut turns to fill along the grid edges",
            Kind::List {
                items: BALANCE,
                min: 0,
                max: 100_000,
            },
        ),
    ],
    errors: &[gp_base::ErrorCode::UnitMismatch],
    warnings: &["LEGACY_UNIT", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Depth h = existing − proposed at each node. Net V = A/4 · Σ(w·h), w = 1 at corners, 2 on edges, 4 inside. Four-point: each cell's V = A·mean(h), or for a cell with both cut and fill, cut = A·(Σh⁺)²/(4Σ|h|) and fill = A·(Σh⁻)²/(4Σ|h|). Balance points by linear interpolation along cell edges (Ghilani & Wolf 2018, ch. 26)",
    accuracy: "Exact for the grid; the volume is as good as the grid spacing is fine against the ground's shape",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "A 3 × 3 grid at 25 ft against a 100 ft grade",
        input: r#"{"existing":[{"elevations":"102.0, 101.5, 101.0"},{"elevations":"101.2, 100.6, 100.2"},{"elevations":"100.4, 99.8, 99.2"}],"cell_size":"25 ft","finished_grade":"100 ft"}"#,
        source: "add-survey-suite corner-weights scenario",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "survey.earthwork.average-end-area",
            reason: "alternative",
        },
        Related {
            id: "survey.earthwork.shrink-swell",
            reason: "next",
        },
    ],
    sentence: "The net volume is {net_volume}: {cut_volume} of cut and {fill_volume} of fill by the four-point method.",
    limits: &[("batchRows", 1_000)],
    run: run_borrow,
    ..ToolDef::BLANK
};

fn grid(ctx: &mut Ctx, list: &str) -> Result<Option<Vec<Vec<f64>>>, ToolError> {
    if !ctx.is_set(list) {
        return Ok(None);
    }
    let rows: Vec<Map<String, Value>> = ctx.rows(list)?;
    let mut out = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let text = row.get("elevations").and_then(Value::as_str).unwrap_or("");
        let vals: Result<Vec<f64>, _> = text.split(',').map(|v| v.trim().parse::<f64>()).collect();
        let vals = vals.map_err(|_| {
            ToolError::invalid(
                &format!("/{list}/{i}/elevations"),
                format!("Row {} has something that is not a number.", i + 1),
            )
        })?;
        out.push(vals);
    }
    let cols = out[0].len();
    if cols < 2 {
        return Err(ToolError::invalid(
            &format!("/{list}/0/elevations"),
            "Each row needs at least two elevations.",
        ));
    }
    if let Some(i) = out.iter().position(|r| r.len() != cols) {
        return Err(ToolError::invalid(
            &format!("/{list}/{i}/elevations"),
            format!("Every row needs {cols} elevations, like the first."),
        ));
    }
    Ok(Some(out))
}

fn run_borrow(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let cell = ctx.req_quantity("cell_size")?;
    let fg = ctx.quantity("finished_grade")?;
    let mut qs = vec![("cell_size", cell)];
    qs.extend(fg.map(|q| ("finished_grade", q)));
    let u = common_unit(&qs)?;
    let a = cell.to(u);
    if a <= 0.0 {
        return Err(ToolError::invalid(
            "/cell_size",
            "The cell size must be positive.",
        ));
    }
    // Plain numbers in the grids are in the cell size's unit.
    let ex = grid(ctx, "existing")?.expect("required");
    let pr = grid(ctx, "proposed")?;
    let (rows, cols) = (ex.len(), ex[0].len());
    let depth: Vec<Vec<f64>> = match (&pr, fg) {
        (Some(p), None) => {
            if p.len() != rows || p[0].len() != cols {
                return Err(ToolError::invalid(
                    "/proposed",
                    format!(
                        "The proposed grid must be {rows} rows by {cols}, like the existing one."
                    ),
                ));
            }
            (0..rows)
                .map(|i| (0..cols).map(|j| ex[i][j] - p[i][j]).collect())
                .collect()
        }
        (None, Some(g)) => {
            let g = g.to(u);
            ex.iter()
                .map(|r| r.iter().map(|e| e - g).collect())
                .collect()
        }
        (Some(_), Some(_)) => {
            return Err(ToolError::invalid(
                "/proposed",
                "Give one finished grade or a proposed grid, not both.",
            ));
        }
        (None, None) => {
            return Err(ToolError::invalid(
                "/finished_grade",
                "Give the finished grade, or a proposed grid.",
            ));
        }
    };
    let area = a * a;
    // Corner weights: how many cells share each node.
    let mut net = 0.0;
    for (i, r) in depth.iter().enumerate() {
        for (j, h) in r.iter().enumerate() {
            let edges = [i == 0, i == rows - 1, j == 0, j == cols - 1]
                .iter()
                .filter(|b| **b)
                .count();
            let w = match edges {
                0 => 4.0,
                1 => 2.0,
                _ => 1.0,
            };
            net += w * h;
        }
    }
    net *= area / 4.0;
    let (mut cut, mut fill) = (0.0, 0.0);
    let mut balance = Vec::new();
    for i in 0..rows - 1 {
        for j in 0..cols - 1 {
            let hs = [
                depth[i][j],
                depth[i][j + 1],
                depth[i + 1][j + 1],
                depth[i + 1][j],
            ];
            let pos: f64 = hs.iter().filter(|h| **h > 0.0).sum();
            let neg: f64 = -hs.iter().filter(|h| **h < 0.0).sum::<f64>();
            if neg == 0.0 {
                cut += area * pos / 4.0;
            } else if pos == 0.0 {
                fill += area * neg / 4.0;
            } else {
                // A transition cell: split in proportion to the depths on each side.
                cut += area * pos * pos / (4.0 * (pos + neg));
                fill += area * neg * neg / (4.0 * (pos + neg));
            }
        }
    }
    // The balance line crosses each grid edge where the depth changes sign.
    for i in 0..rows {
        for j in 0..cols {
            let h = depth[i][j];
            for (di, dj) in [(0, 1), (1, 0)] {
                let (ni, nj) = (i + di, j + dj);
                if ni >= rows || nj >= cols {
                    continue;
                }
                let h2 = depth[ni][nj];
                if h * h2 < 0.0 {
                    let t = h / (h - h2);
                    balance.push((
                        a * (j as f64 + t * dj as f64),
                        a * (i as f64 + t * di as f64),
                    ));
                }
            }
        }
    }
    let per_m = Q {
        value: 1.0,
        unit: u,
    }
    .to(unit(QT::Length, "m"));
    let m3 = unit(QT::Volume, "m3");
    // Feet of either kind report in cubic feet (converted exactly through m³).
    let vu = unit(
        QT::Volume,
        if u.symbol.starts_with("ft") {
            "ft3"
        } else {
            "m3"
        },
    );
    let vol = |v: f64| Q {
        value: v * per_m * per_m * per_m,
        unit: m3,
    };
    let q = |v: f64| Q { value: v, unit: u };
    Ok(Json::obj(vec![
        ("net_volume", ctx.emit("net_volume", vol(net), vu)),
        ("cut_volume", ctx.emit("cut_volume", vol(cut), vu)),
        ("fill_volume", ctx.emit("fill_volume", vol(fill), vu)),
        (
            "net_cubic_yards",
            ctx.emit("net_cubic_yards", vol(net), unit(QT::Volume, "yd3")),
        ),
        (
            "balance_points",
            Json::Arr(
                balance
                    .iter()
                    .map(|&(x, y)| {
                        Json::obj(vec![
                            ("x", ctx.emit("x", q(x), u)),
                            ("y", ctx.emit("y", q(y), u)),
                        ])
                    })
                    .collect(),
            ),
        ),
    ]))
}
