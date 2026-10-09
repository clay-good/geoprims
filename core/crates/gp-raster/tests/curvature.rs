//! Terrain curvature: the properties every answer must have.

use gp_raster::REGISTRY;
use serde_json::{Value, json};

type Grid = [[f64; 3]; 3];

fn run(g: &Grid, cell: f64) -> Value {
    let rows: Vec<Value> = g
        .iter()
        .map(|r| json!({"row": r.iter().map(|v| format!("{v}")).collect::<Vec<_>>().join(", ")}))
        .collect();
    let r: Value = serde_json::from_str(&REGISTRY.invoke(
        "raster.terrain.curvature",
        &json!({"elevations": rows, "cell_size": format!("{cell} m")}).to_string(),
    ))
    .expect("JSON");
    assert_eq!(r["ok"], true, "{r}");
    r["result"].clone()
}

fn nums(r: &Value) -> [f64; 3] {
    ["curvature", "profile", "plan"].map(|k| r[k].as_f64().unwrap_or(f64::NAN))
}

fn close(a: [f64; 3], b: [f64; 3], what: &str) {
    for (x, y) in a.iter().zip(b) {
        assert!(
            (x - y).abs() <= 1e-9 * (1.0 + y.abs()),
            "{what}: {a:?} vs {b:?}"
        );
    }
}

/// Bumpy windows from a fixed seed, none flat at the center.
fn windows() -> Vec<Grid> {
    (1..=40)
        .map(|s| {
            let s = s as f64;
            let mut g = [[0.0; 3]; 3];
            for (i, row) in g.iter_mut().enumerate() {
                for (j, v) in row.iter_mut().enumerate() {
                    let (x, y) = (j as f64, i as f64);
                    *v = 100.0
                        + 3.0 * (s * 0.7 + x * 1.3).sin()
                        + 2.0 * (s * 1.1 - y * 0.9).cos()
                        + 0.5 * (s + x * y).sin();
                }
            }
            g
        })
        .collect()
}

/// The window turned a quarter turn clockwise: the north row becomes the east column.
fn rotate(g: &Grid) -> Grid {
    let mut r = [[0.0; 3]; 3];
    for (i, row) in g.iter().enumerate() {
        for (j, v) in row.iter().enumerate() {
            r[j][2 - i] = *v;
        }
    }
    r
}

#[test]
fn turning_or_mirroring_the_ground_changes_nothing() {
    for g in windows() {
        let base = nums(&run(&g, 10.0));
        let mut turned = g;
        for k in 1..4 {
            turned = rotate(&turned);
            close(
                nums(&run(&turned, 10.0)),
                base,
                &format!("{k} quarter turns"),
            );
        }
        let mirrored = g.map(|mut r| {
            r.reverse();
            r
        });
        close(nums(&run(&mirrored, 10.0)), base, "mirrored east to west");
    }
}

#[test]
fn profile_and_plan_add_up_to_the_total() {
    for g in windows() {
        let [total, profile, plan] = nums(&run(&g, 5.0));
        assert!((profile + plan - total).abs() < 1e-9 * (1.0 + total.abs()));
    }
}

#[test]
fn tilting_the_ground_leaves_the_total_alone() {
    for g in windows() {
        let [total, ..] = nums(&run(&g, 10.0));
        let mut tilted = g;
        for (i, row) in tilted.iter_mut().enumerate() {
            for (j, v) in row.iter_mut().enumerate() {
                *v += 0.7 * j as f64 - 1.9 * i as f64;
            }
        }
        let [t2, ..] = nums(&run(&tilted, 10.0));
        assert!(
            (t2 - total).abs() < 1e-9 * (1.0 + total.abs()),
            "{t2} vs {total}"
        );
    }
}

#[test]
fn heights_scale_and_flip_the_answer() {
    for g in windows() {
        let base = nums(&run(&g, 10.0));
        let scaled = g.map(|r| r.map(|v| 3.0 * v));
        close(
            nums(&run(&scaled, 10.0)),
            base.map(|v| 3.0 * v),
            "elevations tripled",
        );
        let flipped = g.map(|r| r.map(|v| -v));
        close(
            nums(&run(&flipped, 10.0)),
            base.map(|v| -v),
            "elevations negated",
        );
        // Doubling the cell size quarters every second difference.
        close(nums(&run(&g, 20.0)), base.map(|v| v / 4.0), "cells doubled");
    }
}

#[test]
fn a_flat_center_has_no_downhill_direction() {
    let summit = [[98.0, 99.0, 98.0], [99.0, 100.0, 99.0], [98.0, 99.0, 98.0]];
    let r: Value = serde_json::from_str(&REGISTRY.invoke(
        "raster.terrain.curvature",
        &json!({"elevations": summit.map(|r| json!({"row": r.map(|v| v.to_string()).join(", ")})), "cell_size": "10 m"}).to_string(),
    ))
    .unwrap();
    assert!(
        r["result"]["curvature"].as_f64().unwrap() > 0.0,
        "a summit is convex"
    );
    assert!(r["result"].get("profile").is_none() && r["result"].get("plan").is_none());
    assert!(
        r["meta"]["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["code"] == "FLAT_CELL")
    );
}
