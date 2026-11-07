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
