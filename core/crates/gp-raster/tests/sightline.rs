//! Terrain line of sight: the properties every answer must have.

use gp_raster::REGISTRY;
use serde_json::{Value, json};

fn invoke(input: Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke("raster.terrain.line-of-sight", &input.to_string()))
        .expect("JSON")
}

fn input(profile: &[(f64, f64)], ho: f64, ht: f64, extra: Value) -> Value {
    let pts: Vec<Value> = profile
        .iter()
        .map(|(d, e)| json!({"distance": format!("{d} m"), "elevation": format!("{e} m")}))
        .collect();
    let mut v = json!({"points": pts, "observer_height": format!("{ho} m"), "target_height": format!("{ht} m")});
    if let (Some(o), Some(x)) = (v.as_object_mut(), extra.as_object()) {
        o.extend(x.clone());
    }
    v
}

fn run(profile: &[(f64, f64)], ho: f64, ht: f64, extra: Value) -> Value {
    let r = invoke(input(profile, ho, ht, extra));
    assert_eq!(r["ok"], true, "{r}");
    r["result"].clone()
}

fn val(v: &Value) -> f64 {
    v["value"].as_f64().unwrap()
}

/// A rough profile from a fixed seed, in meters.
fn terrain(n: usize, length: f64, seed: f64) -> Vec<(f64, f64)> {
    (0..n)
        .map(|i| {
            let x = i as f64 / (n - 1) as f64;
            let z = 200.0
                + 150.0 * (3.1 * x + seed).sin()
                + 60.0 * (11.0 * x - seed).cos()
                + 25.0 * (29.0 * x * seed).sin();
            (x * length, (z * 100.0).round() / 100.0)
        })
        .collect()
}

#[test]
fn the_needed_height_grazes_the_profile() {
    for seed in 1..=12 {
        let p = terrain(60, 30_000.0, seed as f64 * 0.7);
        let need = val(&run(&p, 2.0, 20.0, json!({}))["observer_height_needed"]);
        if need == 0.0 {
            continue;
        }
        // At the needed height the sight line just touches the profile.
        let at = run(&p, need, 20.0, json!({}));
        assert!(val(&at["clearance"]).abs() < 1e-6, "seed {seed}: {at}");
        assert!(run(&p, need + 0.01, 20.0, json!({}))["visible"] == "yes");
        assert!(run(&p, need - 0.01, 20.0, json!({}))["visible"] == "no");
    }
}

#[test]
fn more_refraction_never_lowers_clearance() {
    for seed in 1..=8 {
        let p = terrain(40, 60_000.0, seed as f64);
        let mut last = f64::NEG_INFINITY;
        for k in [-0.5, 0.0, 0.13, 0.25, 0.5] {
            let c = val(&run(&p, 30.0, 30.0, json!({"k": k}))["clearance"]);
            assert!(c >= last - 1e-9, "seed {seed}, k {k}: {c} < {last}");
            last = c;
        }
    }
}

#[test]
fn swapping_the_ends_mirrors_the_answer() {
    for seed in 1..=8 {
        let p = terrain(50, 20_000.0, seed as f64 * 1.3);
        let total = p[p.len() - 1].0;
        let back: Vec<(f64, f64)> = p.iter().rev().map(|&(d, e)| (total - d, e)).collect();
        let (a, b) = (
            run(&p, 5.0, 40.0, json!({"frequency": "2.4 GHz"})),
            run(&back, 40.0, 5.0, json!({"frequency": "2.4 GHz"})),
        );
        assert_eq!(a["visible"], b["visible"]);
        assert_eq!(a["obstructions"], b["obstructions"]);
        assert_eq!(a["fresnel_short"], b["fresnel_short"]);
        assert!((val(&a["clearance"]) - val(&b["clearance"])).abs() < 1e-6);
        let at = |r: &Value| val(&r["clearance_at"]);
        assert!((at(&a) - (total - at(&b))).abs() < 1e-6, "{a} / {b}");
    }
}

#[test]
fn the_planner_bulge_agrees_on_short_paths() {
    // Link planners draw the ground plus d1 d2 / (2 Re) under a straight line;
    // over 50 km that parabola is within a centimeter of the exact chord.
    let re = 6_371_000.0 / (1.0 - 0.13);
    for seed in 1..=6 {
        let p = terrain(30, 50_000.0, seed as f64 * 0.9);
        let (ho, ht) = (25.0, 35.0);
        let r = run(&p, ho, ht, json!({}));
        let total = p[p.len() - 1].0;
        let (a, b) = (p[0].1 + ho, p[p.len() - 1].1 + ht);
        for (i, row) in r["profile"].as_array().unwrap().iter().enumerate() {
            let (s, e) = p[i + 1];
            let approx = a + (b - a) * s / total - (e + s * (total - s) / (2.0 * re));
            let exact = val(&row["clearance"]);
            assert!(
                (approx - exact).abs() < 0.01,
                "seed {seed}, {s} m: {approx} vs {exact}"
            );
        }
    }
}

#[test]
fn fresnel_needs_more_room_than_sight() {
    for seed in 1..=8 {
        let p = terrain(40, 15_000.0, seed as f64 * 2.1);
        let r = run(&p, 10.0, 10.0, json!({"frequency": "5.8 GHz"}));
        let short = r["fresnel_short"].as_f64().unwrap();
        assert!(short >= r["obstructions"].as_f64().unwrap());
        if r["visible"] == "no" {
            assert_eq!(r["fresnel_clear"], "no");
        }
        // A lower frequency has a wider zone, so no fewer points fall short.
        let low = run(&p, 10.0, 10.0, json!({"frequency": "900 MHz"}));
        assert!(low["fresnel_short"].as_f64().unwrap() >= short);
    }
}

#[test]
fn bad_profiles_are_refused() {
    let ok = [(0.0, 10.0), (500.0, 12.0), (1000.0, 11.0)];
    for (profile, ho, at) in [
        (
            vec![(0.0, 10.0), (500.0, 12.0), (500.0, 11.0)],
            2.0,
            "/points/2/distance",
        ),
        (vec![(0.0, 10.0), (1000.0, 11.0)], 2.0, "/points"),
        (ok.to_vec(), -1.0, "/observer_height"),
        (
            vec![(0.0, 10.0), (500.0, 20_000.0), (1000.0, 11.0)],
            2.0,
            "/points/1/elevation",
        ),
    ] {
        let r = invoke(input(&profile, ho, 2.0, json!({})));
        assert_eq!(r["ok"], false, "{r}");
        assert_eq!(r["error"]["field"], at, "{r}");
    }
    let r = invoke(input(&ok, 2.0, 2.0, json!({"frequency": "0 GHz"})));
    assert_eq!(r["error"]["field"], "/frequency", "{r}");
}
