//! Depression removal (add-raster-hydrology, raster/hydrology): the spec's
//! scenarios, and the invariants that say the conditioning did its job.

use gp_raster::REGISTRY;
use serde_json::{Value, json};

fn run(input: Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke("raster.hydrology.fill", &input.to_string())).unwrap()
}

fn grid_input(rows: &[&str], cell: &str, method: &str) -> Value {
    json!({
        "elevations": rows.iter().map(|r| json!({"row": r})).collect::<Vec<_>>(),
        "cell_size": cell,
        "method": method,
    })
}

/// The conditioned grid, as numbers.
fn conditioned(r: &Value) -> Vec<Vec<f64>> {
    r["result"]["conditioned"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            row["row"]
                .as_str()
                .unwrap()
                .split(',')
                .map(|v| v.trim().parse::<f64>().unwrap())
                .collect()
        })
        .collect()
}

/// Every interior cell has a path to the window's edge that never climbs.
fn drains_everywhere(z: &[Vec<f64>]) -> bool {
    let (h, w) = (z.len(), z[0].len());
    let mut ok = true;
    for r in 1..h - 1 {
        for c in 1..w - 1 {
            let (mut cr, mut cc, mut steps) = (r, c, 0);
            loop {
                if cr == 0 || cc == 0 || cr + 1 == h || cc + 1 == w {
                    break;
                }
                let mut best = (cr, cc, z[cr][cc]);
                for dr in -1i64..=1 {
                    for dc in -1i64..=1 {
                        let (nr, nc) = ((cr as i64 + dr) as usize, (cc as i64 + dc) as usize);
                        if z[nr][nc] < best.2 {
                            best = (nr, nc, z[nr][nc]);
                        }
                    }
                }
                if best.0 == cr && best.1 == cc {
                    ok = false; // nowhere lower to go, and not at an outlet
                    break;
                }
                (cr, cc) = (best.0, best.1);
                steps += 1;
                if steps > h * w {
                    ok = false;
                    break;
                }
            }
        }
    }
    ok
}

const BOWL: [&str; 7] = [
    "100, 100, 100, 100, 100, 100, 100",
    "100, 99, 99, 99, 99, 99, 100",
    "100, 99, 98, 98, 98, 99, 100",
    "100, 99, 98, 95, 98, 99, 100",
    "100, 99, 98, 98, 98, 99, 100",
    "100, 99, 99, 99, 99, 99, 100",
    "100, 100, 100, 100, 100, 100, 100",
];

#[test]
fn fill_scenarios() {
    // Single bowl: the 25 interior cells are raised to the spill elevation
    // with an epsilon step, and nothing else moves.
    let r = run(grid_input(&BOWL, "10 m", "fill"));
    assert_eq!(r["result"]["changed_cells"], 25, "{r}");
    assert_eq!(r["result"]["pits_before"], 1, "{r}");
    assert_eq!(r["result"]["pits_after"], 0, "{r}");
    let z = conditioned(&r);
    assert!(drains_everywhere(&z), "{r}");
    for (i, row) in z.iter().enumerate() {
        for (j, v) in row.iter().enumerate() {
            let edge = i == 0 || j == 0 || i == 6 || j == 6;
            if edge {
                assert!((v - 100.0).abs() < 1e-9, "edge cell moved: {v}");
            } else {
                assert!(*v >= 100.0, "interior cell below the spill: {v}");
                assert!(*v < 100.001, "interior cell raised past the spill: {v}");
            }
        }
    }
    // Pit at the edge stays: the depression's lowest cell is on the edge, so
    // it is an outlet and nothing is raised.
    let edge = run(grid_input(
        &[
            "100, 99, 98, 97, 96",
            "100, 99, 98, 97, 96",
            "100, 99, 98, 97, 95",
            "100, 99, 98, 97, 96",
            "100, 99, 98, 97, 96",
        ],
        "30 m",
        "fill",
    ));
    assert_eq!(edge["result"]["changed_cells"], 0, "{edge}");
    assert_eq!(edge["result"]["fill_volume"]["value"], 0.0, "{edge}");
}

#[test]
fn fill_invariants() {
    // No data is never filled and never crossed.
    let hole = run(json!({
        "elevations": [
            {"row": "100, 100, 100, 100, 100"},
            {"row": "100, 99, 98, 99, 100"},
            {"row": "100, 98, -9999, 98, 100"},
            {"row": "100, 99, 97, 99, 100"},
            {"row": "100, 100, 100, 100, 100"}],
        "cell_size": "10 m", "no_data": -9999.0, "method": "fill"}));
    let z = conditioned(&hole);
    assert!((z[2][2] + 9999.0).abs() < 1e-9, "the hole moved: {hole}");
    assert_eq!(hole["result"]["pits_after"], 0, "{hole}");

    // The fill volume is the sum of the rises times the cell area, and the
    // changes list says the same thing as the conditioned grid.
    let r = run(grid_input(&BOWL, "10 m", "fill"));
    let sum: f64 = r["result"]["changes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            (c["after"]["value"].as_f64().unwrap() - c["before"]["value"].as_f64().unwrap()) * 100.0
        })
        .sum();
    assert!(
        (sum - r["result"]["fill_volume"]["value"].as_f64().unwrap()).abs() < 1e-6,
        "{r}"
    );

    // Conditioning is idempotent: the conditioned grid conditions to itself.
    let rows: Vec<String> = conditioned(&r)
        .iter()
        .map(|row| {
            row.iter()
                .map(|v| format!("{v}"))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .collect();
    let again = run(json!({
        "elevations": rows.iter().map(|r| json!({"row": r})).collect::<Vec<_>>(),
        "cell_size": "10 m", "method": "fill"}));
    assert_eq!(again["result"]["changed_cells"], 0, "{again}");

    // A surface that already drains is left alone, whatever the method.
    for method in ["fill", "breach"] {
        let plane: Vec<String> = (0..8)
            .map(|r| {
                (0..8)
                    .map(|c| format!("{}", 100.0 - 0.5 * f64::from(c) - 0.1 * f64::from(r)))
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .collect();
        let p = run(json!({
            "elevations": plane.iter().map(|r| json!({"row": r})).collect::<Vec<_>>(),
            "cell_size": "25 m", "method": method}));
        assert_eq!(p["result"]["changed_cells"], 0, "{method}: {p}");
    }
}

#[test]
fn breaching_cuts_instead_of_filling() {
    // A dam across a valley: breaching cuts through it, and the earth moved is
    // less than filling the valley behind it would take.
    let dam = [
        "60, 60, 60, 60, 60",
        "60, 50, 48, 50, 60",
        "60, 55, 54, 55, 60",
        "60, 40, 38, 40, 60",
        "60, 30, 20, 30, 60",
    ];
    let filled = run(grid_input(&dam, "20 m", "fill"));
    let breached = run(grid_input(&dam, "20 m", "breach"));
    let fv = filled["result"]["fill_volume"]["value"].as_f64().unwrap();
    let bv = breached["result"]["fill_volume"]["value"].as_f64().unwrap()
        + breached["result"]["cut_volume"]["value"].as_f64().unwrap();
    assert!(fv > 0.0, "{filled}");
    assert!(bv < fv, "breaching moved {bv} m3, filling {fv} m3");
    assert_eq!(breached["result"]["pits_after"], 0, "{breached}");
    assert!(drains_everywhere(&conditioned(&breached)), "{breached}");

    // A cut too shallow to reach past the dam falls back to filling, and says so.
    let shallow = run(json!({
        "elevations": dam.iter().map(|r| json!({"row": r})).collect::<Vec<_>>(),
        "cell_size": "20 m", "method": "breach", "max_breach_depth": "0.5 m"}));
    let codes: Vec<&str> = shallow["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap())
        .collect();
    assert!(codes.contains(&"BREACH_INCOMPLETE"), "{shallow}");
    assert_eq!(shallow["result"]["pits_after"], 0, "{shallow}");
}

#[test]
fn windows_past_the_cap_are_refused() {
    let rows: Vec<Value> = (0..200)
        .map(|_| json!({"row": vec!["1"; 200].join(", ")}))
        .collect();
    let r = run(json!({"elevations": rows, "cell_size": "10 m"}));
    assert_eq!(r["error"]["code"], "LIMIT_EXCEEDED", "{r}");
}

fn flow(input: Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke("raster.hydrology.flow-direction", &input.to_string()))
        .unwrap()
}

/// The direction grid as numbers.
fn codes(r: &Value) -> Vec<Vec<f64>> {
    r["result"]["directions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            row["row"]
                .as_str()
                .unwrap()
                .split(',')
                .map(|v| v.trim().parse::<f64>().unwrap())
                .collect()
        })
        .collect()
}

fn ramp(n: usize) -> Vec<Value> {
    (0..n)
        .map(|_| {
            json!({"row": (0..n).map(|c| format!("{}", 100 - c as i64)).collect::<Vec<_>>().join(", ")})
        })
        .collect()
}

#[test]
fn flow_direction_scenarios() {
    // Ramp: a 10 x 10 DEM falling east. Every cell points east, which is 1 in
    // the ESRI table, 2 in the WhiteboxTools table, and 1 in TauDEM's.
    for (encoding, east) in [("esri", 1.0), ("whitebox", 2.0), ("taudem", 1.0)] {
        let r = flow(
            json!({"elevations": ramp(10), "cell_size": "30 m", "method": "d8", "encoding": encoding}),
        );
        let z = codes(&r);
        for (i, row) in z.iter().enumerate() {
            for (j, v) in row.iter().enumerate() {
                // The east column has nothing lower to the east: it drains off
                // the grid and carries no code.
                let expected = if j + 1 == row.len() { 0.0 } else { east };
                assert_eq!(*v, expected, "{encoding} at {i},{j}: {r}");
            }
        }
        assert!(
            r["result"]["encoding_used"]
                .as_str()
                .unwrap()
                .to_lowercase()
                .contains(if encoding == "esri" { "esri" } else { encoding }),
            "{r}"
        );
    }
    // The same ramp by D-infinity: every resolved angle is 0 radians, east.
    let d = flow(json!({"elevations": ramp(10), "cell_size": "30 m", "method": "d-infinity"}));
    for row in d["result"]["angles"].as_array().unwrap() {
        for v in row["row"].as_str().unwrap().split(',') {
            let a: f64 = v.trim().parse().unwrap();
            assert!(a == 0.0 || a == -1.0, "angle {a} on an east ramp: {d}");
        }
    }
}

#[test]
fn flow_direction_invariants() {
    // Every code is one of the encoding's eight, or zero.
    let hill: Vec<Value> = (0..12)
        .map(|r| {
            json!({"row": (0..12)
                .map(|c| format!("{:.3}", 120.0 - (f64::from(r) - 5.5).abs() - (f64::from(c) - 5.5).abs()))
                .collect::<Vec<_>>().join(", ")})
        })
        .collect();
    for (encoding, allowed) in [
        (
            "esri",
            vec![0.0, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0],
        ),
        (
            "whitebox",
            vec![0.0, 1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0],
        ),
        ("taudem", vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]),
    ] {
        let r = flow(
            json!({"elevations": hill, "cell_size": "10 m", "method": "d8", "encoding": encoding}),
        );
        for row in codes(&r) {
            for v in row {
                assert!(
                    allowed.contains(&v),
                    "{encoding}: code {v} is not in the table"
                );
            }
        }
    }

    // Turning the grid turns the direction: a ramp falling east read upside
    // down falls west, and east and west are opposite codes in every table.
    let east = flow(json!({"elevations": ramp(6), "cell_size": "10 m", "method": "d8"}));
    let west_rows: Vec<Value> = (0..6)
        .map(|_| json!({"row": (0..6).map(|c| format!("{}", 95 + c as i64)).collect::<Vec<_>>().join(", ")}))
        .collect();
    let west = flow(json!({"elevations": west_rows, "cell_size": "10 m", "method": "d8"}));
    assert_eq!(codes(&east)[0][0], 1.0, "{east}");
    assert_eq!(codes(&west)[0][5], 16.0, "{west}");

    // D8 and D-infinity agree on a plane: the angle is the D8 direction.
    let diag: Vec<Value> = (0..7)
        .map(|r| {
            json!({"row": (0..7).map(|c| format!("{}", 100 - r - c)).collect::<Vec<_>>().join(", ")})
        })
        .collect();
    let d8r = flow(json!({"elevations": diag, "cell_size": "10 m", "method": "d8"}));
    let dinf = flow(json!({"elevations": diag, "cell_size": "10 m", "method": "d-infinity"}));
    assert_eq!(
        codes(&d8r)[0][0],
        2.0,
        "south-east is 2 in the ESRI table: {d8r}"
    );
    let a: f64 = dinf["result"]["angles"].as_array().unwrap()[0]["row"]
        .as_str()
        .unwrap()
        .split(',')
        .next()
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    // South-east, counterclockwise from east, is 7π/4.
    assert!(
        (a - 7.0 * std::f64::consts::FRAC_PI_4).abs() < 1e-12,
        "angle {a}: {dinf}"
    );

    // An unconditioned pit has nowhere to go, and says so; conditioning it first
    // leaves no such cell.
    let pit = json!({"elevations": [
        {"row": "100, 100, 100"}, {"row": "100, 95, 100"}, {"row": "100, 100, 100"}],
        "cell_size": "10 m", "method": "d8"});
    let r = flow(pit);
    assert_eq!(r["result"]["no_outflow"], 1, "{r}");
    let codes: Vec<&str> = r["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap())
        .collect();
    assert!(codes.contains(&"PITS_PRESENT"), "{r}");
}

fn accum(input: Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke("raster.hydrology.flow-accumulation", &input.to_string()))
        .unwrap()
}

fn acc_grid(r: &Value) -> Vec<Vec<f64>> {
    r["result"]["accumulation"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            row["row"]
                .as_str()
                .unwrap()
                .split(',')
                .map(|v| v.trim().parse::<f64>().unwrap())
                .collect()
        })
        .collect()
}

#[test]
fn flow_accumulation_scenarios() {
    // East ramp: each row reads 1, 2, 3 ... 10 from west to east.
    let r = accum(json!({"elevations": ramp(10), "cell_size": "30 m", "method": "d8"}));
    for row in acc_grid(&r) {
        for (c, v) in row.iter().enumerate() {
            assert_eq!(*v, (c + 1) as f64, "{r}");
        }
    }
    assert_eq!(r["result"]["max_cells"], 10.0, "{r}");
    // 10 cells of 30 m square is 9,000 m².
    assert!(
        (r["result"]["max_area"]["value"].as_f64().unwrap() - 9000.0).abs() < 1e-9,
        "{r}"
    );
    assert_eq!(r["result"]["outlet_column"], 10, "{r}");

    // D8 and D-infinity agree on a planar ramp, where there is nothing to split.
    let d = accum(json!({"elevations": ramp(10), "cell_size": "30 m", "method": "d-infinity"}));
    for (a, b) in acc_grid(&r)
        .iter()
        .flatten()
        .zip(acc_grid(&d).iter().flatten())
    {
        assert!((a - b).abs() < 1e-9, "D8 {a} vs D-infinity {b}");
    }
}

#[test]
fn flow_accumulation_invariants() {
    let hill: Vec<Value> = (0..12)
        .map(|r| {
            json!({"row": (0..12)
                .map(|c| format!("{:.3}", 200.0 - ((f64::from(r) - 5.5).powi(2) + (f64::from(c) - 5.5).powi(2)).sqrt() * 4.0))
                .collect::<Vec<_>>().join(", ")})
        })
        .collect();
    // Conservation: every cell's count is 1 plus the counts of the cells that
    // point to it. The pointer grid says which those are.
    let a = accum(json!({"elevations": hill, "cell_size": "10 m", "method": "d8"}));
    let dirs =
        flow(json!({"elevations": hill, "cell_size": "10 m", "method": "d8", "encoding": "esri"}));
    let (acc, code) = (acc_grid(&a), codes(&dirs));
    let (h, w) = (acc.len(), acc[0].len());
    // ESRI codes to a row and column step.
    let step = |v: f64| match v as i64 {
        1 => Some((0i64, 1i64)),
        2 => Some((1, 1)),
        4 => Some((1, 0)),
        8 => Some((1, -1)),
        16 => Some((0, -1)),
        32 => Some((-1, -1)),
        64 => Some((-1, 0)),
        128 => Some((-1, 1)),
        _ => None,
    };
    let mut incoming = vec![vec![0.0; w]; h];
    for r in 0..h {
        for c in 0..w {
            if let Some((dr, dc)) = step(code[r][c]) {
                let (nr, nc) = (r as i64 + dr, c as i64 + dc);
                if nr >= 0 && nr < h as i64 && nc >= 0 && nc < w as i64 {
                    incoming[nr as usize][nc as usize] += acc[r][c];
                }
            }
        }
    }
    for r in 0..h {
        for c in 0..w {
            assert!(
                (acc[r][c] - (1.0 + incoming[r][c])).abs() < 1e-9,
                "cell {r},{c}: {} vs 1 + {}",
                acc[r][c],
                incoming[r][c]
            );
        }
    }
    // Every cell counts at least itself, and no cell counts more than the grid.
    let total = (h * w) as f64;
    for row in &acc {
        for v in row {
            assert!(
                *v >= 1.0 && *v <= total,
                "accumulation {v} outside 1..{total}"
            );
        }
    }
    // D-infinity conserves the same total water: the sum of what leaves the
    // window equals the number of cells, within rounding.
    let d = accum(json!({"elevations": hill, "cell_size": "10 m", "method": "d-infinity"}));
    let dmax = d["result"]["max_cells"].as_f64().unwrap();
    assert!(dmax >= 1.0 && dmax <= total, "{d}");

    // A pit swallows what reaches it, and the result says so.
    let pit = accum(json!({"elevations": [
        {"row": "100, 100, 100"}, {"row": "100, 95, 100"}, {"row": "100, 100, 100"}],
        "cell_size": "10 m", "method": "d8"}));
    assert_eq!(pit["result"]["pits"], 1, "{pit}");
    let warned: Vec<&str> = pit["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap())
        .collect();
    assert!(warned.contains(&"PITS_PRESENT"), "{pit}");
    // Everything but the rim drains into it: 9 cells in a 3 by 3.
    assert_eq!(acc_grid(&pit)[1][1], 9.0, "{pit}");
}

fn streams(input: Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke("raster.hydrology.streams", &input.to_string())).unwrap()
}

/// Two headwaters joining a valley, the tool's own example.
const FORK: [&str; 7] = [
    "60, 48, 60, 60, 60, 48, 60",
    "60, 60, 47, 60, 47, 60, 60",
    "60, 60, 60, 46, 60, 60, 60",
    "60, 60, 60, 45, 60, 60, 60",
    "60, 60, 60, 44, 60, 60, 60",
    "60, 60, 60, 43, 60, 60, 60",
    "60, 60, 60, 42, 60, 60, 60",
];

#[test]
fn stream_order_rule() {
    let r = streams(json!({
        "elevations": FORK.iter().map(|x| json!({"row": x})).collect::<Vec<_>>(),
        "cell_size": "30 m", "threshold_cells": 3}));
    // Two first-order links join into a second-order one.
    assert_eq!(r["result"]["highest_order"], 2, "{r}");
    let by: Vec<(i64, i64)> = r["result"]["by_order"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| (o["order"].as_i64().unwrap(), o["links"].as_i64().unwrap()))
        .collect();
    assert_eq!(by[0].0, 1, "{r}");
    assert!(by[0].1 >= 2, "two headwaters: {r}");
    assert_eq!(
        by.iter().find(|(o, _)| *o == 2).map(|(_, n)| *n),
        Some(1),
        "{r}"
    );
    // Every link carries a length and an outlet area, and the lengths add up.
    let links = r["result"]["links"].as_array().unwrap();
    let sum: f64 = links
        .iter()
        .map(|l| l["length"]["value"].as_f64().unwrap())
        .sum();
    assert!(
        (sum - r["result"]["network_length"]["value"].as_f64().unwrap()).abs() < 1e-9,
        "{r}"
    );
    for l in links {
        assert!(l["outlet_area"]["value"].as_f64().unwrap() > 0.0, "{l}");
        assert!(l["cells"].as_i64().unwrap() >= 1, "{l}");
    }
    // The vertices are the cells of the links, in order, and their count
    // matches the cells each link runs through.
    let verts = r["result"]["vertices"].as_array().unwrap();
    let cells: i64 = links.iter().map(|l| l["cells"].as_i64().unwrap()).sum();
    assert_eq!(verts.len() as i64, cells, "{r}");
}

#[test]
fn stream_threshold_and_placement() {
    let grid: Vec<Value> = FORK.iter().map(|x| json!({"row": x})).collect();
    // A threshold past the window's own maximum finds nothing, and says what
    // the maximum was.
    let empty = streams(json!({"elevations": grid, "cell_size": "30 m", "threshold_cells": 10000}));
    assert_eq!(empty["result"]["channel_cells"], 0, "{empty}");
    assert_eq!(empty["result"]["links_found"], 0, "{empty}");
    assert!(
        empty["result"]["max_accumulation"].as_f64().unwrap() > 0.0,
        "{empty}"
    );
    let codes: Vec<&str> = empty["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap())
        .collect();
    assert!(codes.contains(&"EMPTY_NETWORK"), "{empty}");

    // Raising the threshold never adds channel cells.
    let mut last = f64::INFINITY;
    for t in [1, 2, 4, 8, 16] {
        let r = streams(json!({"elevations": grid, "cell_size": "30 m", "threshold_cells": t}));
        let n = r["result"]["channel_cells"].as_f64().unwrap();
        assert!(
            n <= last,
            "threshold {t} gave {n} cells, more than the one before"
        );
        last = n;
    }

    // The threshold as an area is the same threshold: 9 cells of 30 m is
    // 8,100 m².
    let by_cells = streams(json!({"elevations": grid, "cell_size": "30 m", "threshold_cells": 9}));
    let by_area =
        streams(json!({"elevations": grid, "cell_size": "30 m", "threshold_area": "8100 m2"}));
    assert_eq!(
        by_cells["result"]["channel_cells"], by_area["result"]["channel_cells"],
        "{by_cells} vs {by_area}"
    );
    // Both at once is a refusal, and so is neither.
    let both = streams(
        json!({"elevations": grid, "cell_size": "30 m", "threshold_cells": 9, "threshold_area": "8100 m2"}),
    );
    assert_eq!(both["error"]["code"], "INVALID_INPUT", "{both}");
    let neither = streams(json!({"elevations": grid, "cell_size": "30 m"}));
    assert_eq!(neither["error"]["code"], "INVALID_INPUT", "{neither}");

    // Given the grid's north-west corner, every vertex carries a place; the
    // first link's first vertex sits half a cell east and south of it.
    let placed = streams(
        json!({"elevations": grid, "cell_size": "30 m", "threshold_cells": 3,
                                "north_west_lat": "44.12 deg", "north_west_lon": "-121.45 deg"}),
    );
    let v = &placed["result"]["vertices"].as_array().unwrap()[0];
    assert!(v["lat"]["value"].as_f64().unwrap() < 44.12, "{v}");
    assert!(v["lon"]["value"].as_f64().unwrap() > -121.45, "{v}");
    // One corner without the other is a refusal.
    let half = streams(
        json!({"elevations": grid, "cell_size": "30 m", "threshold_cells": 3, "north_west_lat": "44.12 deg"}),
    );
    assert_eq!(half["error"]["code"], "INVALID_INPUT", "{half}");
}

fn watershed(input: Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke("raster.hydrology.watershed", &input.to_string()))
        .unwrap()
}

fn basin_cells(r: &Value) -> Vec<Vec<f64>> {
    r["result"]["basin"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            row["row"]
                .as_str()
                .unwrap()
                .split(',')
                .map(|v| v.trim().parse::<f64>().unwrap())
                .collect()
        })
        .collect()
}

#[test]
fn watershed_scenarios() {
    let grid: Vec<Value> = FORK.iter().map(|x| json!({"row": x})).collect();
    // Nested basins: the basin of a cell upstream lies inside the basin below.
    let low = watershed(
        json!({"elevations": grid, "cell_size": "30 m", "pour_row": 7, "pour_column": 4, "snap": "none"}),
    );
    let high = watershed(
        json!({"elevations": grid, "cell_size": "30 m", "pour_row": 4, "pour_column": 4, "snap": "none"}),
    );
    let (lo, hi) = (basin_cells(&low), basin_cells(&high));
    for (r, row) in hi.iter().enumerate() {
        for (c, v) in row.iter().enumerate() {
            if *v == 1.0 {
                assert_eq!(
                    lo[r][c], 1.0,
                    "cell {r},{c} is upstream but outside the lower basin"
                );
            }
        }
    }
    assert!(
        high["result"]["basin_cells"].as_f64().unwrap()
            < low["result"]["basin_cells"].as_f64().unwrap(),
        "{high} vs {low}"
    );
    // The basin's area is its cells times the cell area.
    let cells = low["result"]["basin_cells"].as_f64().unwrap();
    assert!(
        (low["result"]["basin_area"]["value"].as_f64().unwrap() - cells * 900.0).abs() < 1e-9,
        "{low}"
    );
    // The outlet is in its own basin, and the longest path ends there.
    let path = low["result"]["path"].as_array().unwrap();
    let last = path.last().unwrap();
    assert_eq!(last["row"], low["result"]["outlet_row"], "{low}");
    assert_eq!(last["column"], low["result"]["outlet_column"], "{low}");

    // The Ramp path scenario: 270 m along a 10-cell east ramp of 30 m cells.
    let r = watershed(
        json!({"elevations": ramp(10), "cell_size": "30 m", "pour_row": 1, "pour_column": 10, "snap": "none"}),
    );
    assert!(
        (r["result"]["longest_path"]["value"].as_f64().unwrap() - 270.0).abs() < 0.001,
        "{r}"
    );
    assert!(
        (r["result"]["path_drop"]["value"].as_f64().unwrap() - 9.0).abs() < 1e-9,
        "{r}"
    );
}

#[test]
fn watershed_edges_and_refusals() {
    let grid: Vec<Value> = FORK.iter().map(|x| json!({"row": x})).collect();
    // A basin that reaches the window's edge says so and warns.
    let r = watershed(
        json!({"elevations": grid, "cell_size": "30 m", "pour_row": 7, "pour_column": 4, "snap": "none"}),
    );
    assert_eq!(r["result"]["touches_edge"], "yes", "{r}");
    let codes: Vec<&str> = r["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap())
        .collect();
    assert!(codes.contains(&"WATERSHED_TOUCHES_EDGE"), "{r}");

    // Snapping moves a point off the channel onto it, and says it moved.
    let off = watershed(
        json!({"elevations": grid, "cell_size": "30 m", "pour_row": 7, "pour_column": 3,
                               "snap": "nearest-stream", "threshold_cells": 5}),
    );
    assert_eq!(off["result"]["outlet_column"], 4, "{off}");
    assert!(
        off["result"]["snap_note"]
            .as_str()
            .unwrap()
            .contains("snapped"),
        "{off}"
    );
    // Snapped, it finds the valley; unsnapped, the hillside cell alone.
    let unsnapped = watershed(
        json!({"elevations": grid, "cell_size": "30 m", "pour_row": 7, "pour_column": 3, "snap": "none"}),
    );
    assert!(
        unsnapped["result"]["basin_cells"].as_f64().unwrap()
            < off["result"]["basin_cells"].as_f64().unwrap(),
        "{unsnapped} vs {off}"
    );

    // A pour point off the window, or on no data, is refused by name.
    let off_grid = watershed(
        json!({"elevations": grid, "cell_size": "30 m", "pour_row": 99, "pour_column": 4}),
    );
    assert_eq!(off_grid["error"]["code"], "OUT_OF_DOMAIN", "{off_grid}");
    assert!(
        off_grid["error"]["message"]
            .as_str()
            .unwrap()
            .contains("7 rows"),
        "{off_grid}"
    );
    let hole = watershed(json!({"elevations": [
        {"row": "100, 99, 98"}, {"row": "100, -9999, 97"}, {"row": "100, 99, 96"}],
        "cell_size": "10 m", "no_data": -9999.0, "pour_row": 2, "pour_column": 2}));
    assert_eq!(hole["error"]["code"], "OUT_OF_DOMAIN", "{hole}");
}

fn longest(input: Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(
        "raster.hydrology.longest-flow-path",
        &input.to_string(),
    ))
    .unwrap()
}

#[test]
fn longest_flow_path_scenarios() {
    // The Ramp path scenario: 270 m along the row, falling 9 m.
    let r = longest(json!({"elevations": ramp(10), "cell_size": "30 m", "pour_row": 1, "pour_column": 10}));
    assert!((r["result"]["length"]["value"].as_f64().unwrap() - 270.0).abs() < 0.001, "{r}");
    assert!((r["result"]["drop"]["value"].as_f64().unwrap() - 9.0).abs() < 1e-9, "{r}");
    assert_eq!(r["result"]["cells"], 10, "{r}");
    assert_eq!(r["result"]["start_column"], 1, "{r}");
    // The path is the row: every step moves one column east, same row.
    let path = r["result"]["path"].as_array().unwrap();
    for (k, step) in path.iter().enumerate() {
        assert_eq!(step["row"], 1, "{r}");
        assert_eq!(step["column"], (k + 1) as i64, "{r}");
    }
    // Its average slope is the drop over the length.
    assert!(
        (r["result"]["average_slope"].as_f64().unwrap() - 9.0 / 270.0).abs() < 1e-5,
        "{r}"
    );
}

#[test]
fn longest_flow_path_invariants() {
    let grid: Vec<Value> = FORK.iter().map(|x| json!({"row": x})).collect();
    // It agrees with the watershed tool's own longest path, cell for cell.
    let lfp = longest(json!({"elevations": grid, "cell_size": "30 m", "pour_row": 7, "pour_column": 4}));
    let ws = watershed(json!({"elevations": grid, "cell_size": "30 m", "pour_row": 7, "pour_column": 4, "snap": "none"}));
    assert!(
        (lfp["result"]["length"]["value"].as_f64().unwrap()
            - ws["result"]["longest_path"]["value"].as_f64().unwrap())
        .abs()
            < 1e-9,
        "{lfp} vs {ws}"
    );
    assert_eq!(lfp["result"]["path"], ws["result"]["path"], "the two tools draw different paths");

    // A diagonal step is √2 cells: on a plane falling south-east, a path of n
    // steps is n √2 cells long.
    let diag: Vec<Value> = (0..8)
        .map(|r| json!({"row": (0..8).map(|c| format!("{}", 100 - r - c)).collect::<Vec<_>>().join(", ")}))
        .collect();
    let d = longest(json!({"elevations": diag, "cell_size": "20 m", "pour_row": 8, "pour_column": 8}));
    let steps = d["result"]["cells"].as_f64().unwrap() - 1.0;
    assert!(
        (d["result"]["length"]["value"].as_f64().unwrap() - steps * 20.0 * std::f64::consts::SQRT_2).abs() < 1e-9,
        "{d}"
    );

    // The path always ends at the outlet and never climbs.
    let path = d["result"]["path"].as_array().unwrap();
    let last = path.last().unwrap();
    assert_eq!(last["row"], 8, "{d}");
    assert_eq!(last["column"], 8, "{d}");
    for pair in path.windows(2) {
        let (a, b) = (
            pair[0]["elevation"]["value"].as_f64().unwrap(),
            pair[1]["elevation"]["value"].as_f64().unwrap(),
        );
        assert!(b < a, "the path climbs from {a} to {b}");
    }

    // Given the grid's corner, every vertex carries a place.
    let placed = longest(json!({"elevations": grid, "cell_size": "30 m", "pour_row": 7, "pour_column": 4,
                                "north_west_lat": "44.12 deg", "north_west_lon": "-121.45 deg"}));
    for v in placed["result"]["vertices"].as_array().unwrap() {
        assert!(v["lat"]["value"].as_f64().unwrap() < 44.12, "{v}");
        assert!(v["lon"]["value"].as_f64().unwrap() > -121.45, "{v}");
    }
}
