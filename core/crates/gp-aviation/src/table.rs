//! POH/AFM table interpolation (add-aviation-suite, aviation/fuel-and-loading,
//! "POH/AFM table interpolation"): a performance table of one, two, or three
//! variables read by linear, bilinear, or trilinear interpolation, never
//! extrapolated, with correction factors applied as separate labeled steps.

use crate::obj;
use crate::refs::*;
use gp_base::ErrorCode;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Reference, Related, ToolDef};
use serde_json::Value;

const PHAK_PERF: Reference = Reference {
    locator: "Aircraft Performance chapter (reading performance charts and tables, interpolating between listed values)",
    ..PHAK
};

const fn num(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Number {
            min: -1e9,
            max: 1e9,
        },
    )
}

const ROW: &[Field] = &[
    num("a", "First variable", "Like 2000 (pressure altitude, ft)").required(),
    num(
        "b",
        "Second variable",
        "Like 20 (temperature, °C); leave out for a one-variable table",
    ),
    num(
        "c",
        "Third variable",
        "Like 2400 (weight, lb); leave out for fewer",
    ),
    num("value", "Table value", "Like 1350 (takeoff distance, ft)").required(),
];
const CORRECTION: &[Field] = &[
    Field::new(
        "label",
        "Correction",
        "What it is for, like 10 kt headwind",
        Kind::Text { max_len: 60 },
    )
    .required(),
    Field::new(
        "kind",
        "Kind",
        "percent, add, or multiply",
        Kind::Choice(&["percent", "add", "multiply"]),
    )
    .required(),
    num(
        "amount",
        "Amount",
        "Like -10 (percent), 50 (add), or 1.15 (multiply)",
    )
    .required(),
];
const CELL: &[Field] = &[
    num("a", "First variable", "Of the cell").precision(Precision::Significant(8)),
    num("b", "Second variable", "Of the cell")
        .precision(Precision::Significant(8))
        .optional(),
    num("c", "Third variable", "Of the cell")
        .precision(Precision::Significant(8))
        .optional(),
    num("value", "Value", "In the table").precision(Precision::Significant(8)),
    num("weight", "Weight", "Its share of the answer").precision(Precision::Decimals(4)),
];
const STEP: &[Field] = &[
    Field::new(
        "step",
        "Step",
        "What was applied",
        Kind::Text { max_len: 80 },
    ),
    num("value", "Value", "After this step").precision(Precision::Significant(8)),
];

pub static TABLE: ToolDef = ToolDef {
    id: "aviation.loading.table-interpolate",
    title: "Performance table lookup",
    summary: "Reads a POH performance table of one, two, or three variables, like takeoff distance by altitude, temperature, and weight, between its listed values, refuses to guess beyond them, and applies corrections as labeled steps.",
    aliases: &[
        "POH table interpolation",
        "performance chart interpolation",
        "bilinear interpolation",
        "takeoff distance table",
        "interpolate table",
        "takeoff distance calculator",
        "landing distance calculator",
    ],
    keywords: &[
        "interpolate",
        "bilinear",
        "trilinear",
        "POH",
        "AFM",
        "performance table",
        "takeoff distance",
        "landing distance",
        "correction",
    ],
    inputs: &[
        Field::new(
            "table",
            "Table",
            "One row per cell: variables then the value, like 2000, 20, 1350",
            Kind::List {
                items: ROW,
                min: 2,
                max: 2_000,
            },
        )
        .core(),
        num("at_a", "Look up first variable at", "Like 3000")
            .required()
            .core(),
        num(
            "at_b",
            "Second variable at",
            "Like 25; needed for a two- or three-variable table",
        )
        .core(),
        num(
            "at_c",
            "Third variable at",
            "Like 2300; needed for a three-variable table",
        )
        .core(),
        Field::new(
            "corrections",
            "Corrections",
            "Applied in order after the lookup, like 10 kt headwind, percent, -10",
            Kind::List {
                items: CORRECTION,
                min: 0,
                max: 20,
            },
        )
        .core(),
        Field::new(
            "names",
            "Variable names",
            "For messages, like pressure altitude, temperature, weight",
            Kind::Text { max_len: 120 },
        ),
    ],
    outputs: &[
        num("value", "Answer", "After every correction").precision(Precision::Significant(8)),
        num(
            "table_value",
            "From the table",
            "Interpolated, before corrections",
        )
        .precision(Precision::Significant(8)),
        Field::new(
            "cells",
            "Cells used",
            "The surrounding cells and their weights",
            Kind::List {
                items: CELL,
                min: 0,
                max: 8,
            },
        ),
        Field::new(
            "steps",
            "Steps",
            "The lookup, then each correction",
            Kind::List {
                items: STEP,
                min: 0,
                max: 21,
            },
        ),
    ],
    errors: &[ErrorCode::OutOfDomain, ErrorCode::InvalidInput],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "Linear interpolation along each variable between the listed values that bracket the query: 2 cells for one variable, 4 (bilinear) for two, 8 (trilinear) for three, each weighted by the product of its distances. No extrapolation. Corrections apply in order: percent (× (1 + p/100)), add, or multiply (PHAK, aircraft performance)",
    accuracy: "Exact interpolation of the table as entered; real performance between table rows is not linear, and the table is only as good as the book it came from",
    references: &[PHAK_PERF],
    examples: &[Example {
        id: "primary",
        title: "Takeoff distance at 3,000 ft and 25 °C, less 10% for a headwind",
        input: r#"{"table":[{"a":0,"b":0,"value":1000},{"a":0,"b":10,"value":1070},{"a":0,"b":20,"value":1145},{"a":0,"b":30,"value":1225},{"a":2000,"b":0,"value":1150},{"a":2000,"b":10,"value":1235},{"a":2000,"b":20,"value":1325},{"a":2000,"b":30,"value":1420},{"a":4000,"b":0,"value":1335},{"a":4000,"b":10,"value":1435},{"a":4000,"b":20,"value":1540},{"a":4000,"b":30,"value":1655}],"at_a":3000,"at_b":25,"corrections":[{"label":"10 kt headwind","kind":"percent","amount":-10}],"names":"pressure altitude, temperature"}"#,
        source: "add-aviation-suite bilinear scenario: the four cells around 3,000 ft and 25 °C",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.altimetry.density-altitude",
            reason: "parent",
        },
        Related {
            id: "aviation.loading.weight-balance",
            reason: "alternative",
        },
    ],
    sentence: "The table gives {table_value}; with the corrections, {value}.",
    limits: &[("batchRows", 10_000)],
    run: run_table,
    ..ToolDef::BLANK
};

fn run_table(ctx: &mut Ctx) -> Result<Json, ToolError> {
    // aviation/flight-performance "Density-altitude effects need aircraft data":
    // no generic takeoff or landing estimate, only the user's own table.
    if !ctx.is_set("table") {
        return Err(ToolError::invalid(
            "/table",
            "Takeoff and landing distances need your aircraft's own data: enter the table from its POH or AFM. There is no generic estimate here, because one could be mistaken for your aircraft's numbers.",
        )
        .hint("Copy the table's rows here, one cell per row, like 2000, 20, 1350. To see how altitude and temperature change performance, use aviation.altimetry.density-altitude."));
    }
    let rows = ctx.rows("table")?;
    let get = |r: &serde_json::Map<String, Value>, k: &str| -> Option<f64> {
        r.get(k).and_then(Value::as_f64)
    };
    let mut cells: Vec<([Option<f64>; 3], f64)> = Vec::with_capacity(rows.len());
    for (i, r) in rows.iter().enumerate() {
        let a = get(r, "a").ok_or_else(|| {
            ToolError::invalid(
                &format!("/table/{i}/a"),
                "Each row needs its first variable.",
            )
        })?;
        let v = get(r, "value").ok_or_else(|| {
            ToolError::invalid(&format!("/table/{i}/value"), "Each row needs its value.")
        })?;
        cells.push(([Some(a), get(r, "b"), get(r, "c")], v));
    }
    // How many variables: every row must give the same ones.
    let dims = [0, 1, 2]
        .iter()
        .filter(|&&k| cells[0].0[k].is_some())
        .count();
    if (0..3).any(|k| {
        cells
            .iter()
            .any(|c| c.0[k].is_some() != cells[0].0[k].is_some())
    }) || (dims == 2 && cells[0].0[1].is_none())
    {
        return Err(ToolError::invalid(
            "/table",
            "Every row must give the same variables: the first, then the second, then the third.",
        ));
    }
    let names_text = ctx.text("names")?.unwrap_or_default();
    let mut names: Vec<String> = names_text
        .split(',')
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect();
    for (k, fallback) in ["first variable", "second variable", "third variable"]
        .iter()
        .enumerate()
    {
        if names.len() <= k {
            names.push((*fallback).to_owned());
        }
    }
    let queries = [
        ctx.number("at_a")?,
        ctx.number("at_b")?,
        ctx.number("at_c")?,
    ];
    // The listed values on each axis, and the full grid they must fill.
    let mut axes: Vec<Vec<f64>> = Vec::new();
    for (k, name) in names.iter().enumerate().take(dims) {
        let mut v: Vec<f64> = cells.iter().map(|c| c.0[k].expect("present")).collect();
        v.sort_by(f64::total_cmp);
        v.dedup();
        if v.len() < 2 {
            return Err(ToolError::invalid(
                "/table",
                format!("The {name} needs at least two listed values to interpolate between."),
            ));
        }
        axes.push(v);
    }
    let size: usize = axes.iter().map(Vec::len).product();
    if size != cells.len() {
        return Err(ToolError::invalid(
            "/table",
            format!(
                "The table has {} rows but its values make a grid of {size}: every combination must appear exactly once.",
                cells.len()
            ),
        ));
    }
    let key = |p: &[Option<f64>; 3]| -> Vec<u64> {
        (0..dims)
            .map(|k| p[k].expect("present").to_bits())
            .collect()
    };
    let mut map = std::collections::HashMap::new();
    for c in &cells {
        if map.insert(key(&c.0), c.1).is_some() {
            return Err(ToolError::invalid(
                "/table",
                "A combination of variables appears twice.",
            ));
        }
    }
    // Bracket each query; refuse anything outside the table.
    let mut brackets = Vec::with_capacity(dims);
    for k in 0..dims {
        let field = ["/at_a", "/at_b", "/at_c"][k];
        let Some(q) = queries[k] else {
            return Err(ToolError::invalid(
                field,
                format!(
                    "This table has {dims} variables: give the {} to look up.",
                    names[k]
                ),
            ));
        };
        let ax = &axes[k];
        let (lo, hi) = (ax[0], ax[ax.len() - 1]);
        if q < lo || q > hi {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                format!("The {} {q} is outside the table's {lo} to {hi}; this tool does not extrapolate.", names[k]),
            )
            .at(field));
        }
        let i = ax.windows(2).position(|w| q <= w[1]).expect("bracketed");
        let t = (q - ax[i]) / (ax[i + 1] - ax[i]);
        brackets.push((ax[i], ax[i + 1], t));
    }
    let mut value = 0.0;
    let mut used = Vec::new();
    for corner in 0..(1usize << dims) {
        let mut p = [None; 3];
        let mut w = 1.0;
        for (k, &(lo, hi, t)) in brackets.iter().enumerate() {
            let upper = corner >> k & 1 == 1;
            p[k] = Some(if upper { hi } else { lo });
            w *= if upper { t } else { 1.0 - t };
        }
        let v = map[&key(&p)];
        value += w * v;
        let mut row = vec![("a", Json::Num(p[0].expect("first")))];
        if dims > 1 {
            row.push(("b", Json::Num(p[1].expect("second"))));
        }
        if dims > 2 {
            row.push(("c", Json::Num(p[2].expect("third"))));
        }
        row.push(("value", Json::Num(v)));
        row.push(("weight", Json::Num(w)));
        used.push(Json::obj(row));
    }
    let table_value = value;
    let mut steps = vec![Json::obj([
        (
            "step",
            Json::str(format!(
                "{} interpolation of the table",
                ["linear", "linear", "bilinear", "trilinear"][dims]
            )),
        ),
        ("value", Json::Num(value)),
    ])];
    if ctx.is_set("corrections") {
        let rows = ctx.rows("corrections")?;
        for (i, r) in rows.iter().enumerate() {
            let label = r
                .get("label")
                .and_then(Value::as_str)
                .unwrap_or("correction")
                .trim()
                .to_owned();
            let kind = r.get("kind").and_then(Value::as_str).unwrap_or("");
            let amount = get(r, "amount").ok_or_else(|| {
                ToolError::invalid(
                    &format!("/corrections/{i}/amount"),
                    "Each correction needs an amount.",
                )
            })?;
            value = match kind {
                "percent" => value * (1.0 + amount / 100.0),
                "add" => value + amount,
                "multiply" => value * amount,
                _ => {
                    return Err(ToolError::invalid(
                        &format!("/corrections/{i}/kind"),
                        "A correction is percent, add, or multiply.",
                    ));
                }
            };
            let how = match kind {
                "percent" => format!("{amount:+}%"),
                "add" => format!("{amount:+}"),
                _ => format!("× {amount}"),
            };
            steps.push(Json::obj([
                ("step", Json::str(format!("{label} ({how})"))),
                ("value", Json::Num(value)),
            ]));
        }
    }
    Ok(obj(vec![
        ("value", Json::Num(value)),
        ("table_value", Json::Num(table_value)),
        ("cells", Json::Arr(used)),
        ("steps", Json::Arr(steps)),
    ]))
}
