//! ASPRS Edition 2 accuracy: the properties every answer must have.

use gp_drone::REGISTRY;
use serde_json::{Value, json};

fn call(input: Value) -> Value {
    let r: Value = serde_json::from_str(
        &REGISTRY.invoke("drone.photogrammetry.asprs-accuracy", &input.to_string()),
    )
    .expect("JSON");
    assert_eq!(r["ok"], true, "{r}");
    r
}

fn cm(r: &Value, k: &str) -> f64 {
    r["result"][k]["value"]
        .as_f64()
        .unwrap_or_else(|| panic!("{k} in {r}"))
}

fn codes(r: &Value) -> Vec<String> {
    r["meta"]["warnings"]
        .as_array()
        .map(|w| {
            w.iter()
                .map(|x| x["code"].as_str().unwrap().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// A repeatable spread of errors in centimeters.
fn errors(n: usize, seed: u64, spread: f64) -> Vec<(f64, f64, f64)> {
    let mut s = seed;
    let mut next = || {
        s = s
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((s >> 11) as f64 / (1u64 << 53) as f64 - 0.5) * 2.0 * spread
    };
    (0..n)
        .map(|_| {
            let (x, y, z) = (next(), next(), next());
            (
                (x * 10.0).round() / 10.0,
                (y * 10.0).round() / 10.0,
                (z * 10.0).round() / 10.0,
            )
        })
        .collect()
}

fn rows(e: &[(f64, f64, f64)]) -> Value {
    Value::Array(
        e.iter()
            .map(|(x, y, z)| json!({"dx": format!("{x} cm"), "dy": format!("{y} cm"), "dz": format!("{z} cm")}))
            .collect(),
    )
}

#[test]
fn asprs_invariants() {
    for seed in 1..=6 {
        let e = errors(40, seed, 3.0);
        let listed = call(json!({"errors": rows(&e), "checkpoint_rmse": "1 cm"}));
        // The list gives the same answer as its own RMSEs typed in.
        let rms = |f: fn(&(f64, f64, f64)) -> f64| {
            (e.iter().map(|p| f(p) * f(p)).sum::<f64>() / e.len() as f64).sqrt()
        };
        let typed = call(json!({
            "rmse_x": format!("{} cm", rms(|p| p.0)), "rmse_y": format!("{} cm", rms(|p| p.1)),
            "rmse_z": format!("{} cm", rms(|p| p.2)), "checkpoint_rmse": "1 cm", "checkpoints": 40,
        }));
        for k in ["horizontal", "vertical"] {
            assert!(
                (cm(&listed, k) - cm(&typed, k)).abs() < 1e-9,
                "seed {seed} {k}"
            );
        }
        // Product accuracy is never better than the fit, nor than the
        // checkpoints, and is exactly the fit with perfect checkpoints.
        let perfect = call(json!({"errors": rows(&e), "checkpoint_rmse": "0 cm"}));
        assert!(
            (cm(&perfect, "vertical") - rms(|p| p.2)).abs() < 1e-9,
            "seed {seed}"
        );
        assert!(
            cm(&listed, "vertical") >= cm(&perfect, "vertical"),
            "seed {seed}"
        );
        assert!(cm(&listed, "vertical") >= 1.0, "seed {seed}");
        // Order does not matter, and the sign of every error flips only the means.
        let mut shuffled = e.clone();
        shuffled.reverse();
        let flipped: Vec<_> = e.iter().map(|p| (-p.0, -p.1, -p.2)).collect();
        for other in [&shuffled, &flipped] {
            let r = call(json!({"errors": rows(other), "checkpoint_rmse": "1 cm"}));
            assert!((cm(&r, "horizontal") - cm(&listed, "horizontal")).abs() < 1e-9);
            assert!((cm(&r, "vertical") - cm(&listed, "vertical")).abs() < 1e-9);
        }
        let r = call(json!({"errors": rows(&flipped), "checkpoint_rmse": "1 cm"}));
        assert!((cm(&r, "mean_z") + cm(&listed, "mean_z")).abs() < 1e-9);
        // A class passes exactly when the accuracy is at or under it.
        let v = cm(&listed, "vertical");
        for (class, meets) in [(v * 1.001, true), (v * 0.999, false)] {
            let r = call(json!({"errors": rows(&e), "checkpoint_rmse": "1 cm",
                "target_vertical": format!("{class} cm")}));
            let said = r["result"]["vertical_result"].as_str().unwrap();
            assert_eq!(said.starts_with("meets"), meets, "seed {seed}: {said}");
        }
        // Vegetated heights feed only the VVA: adding them leaves the NVA alone.
        let mut veg = rows(&e);
        for z in [9.0, -7.5, 12.0] {
            veg.as_array_mut()
                .unwrap()
                .push(json!({"dz": format!("{z} cm"), "cover": "vegetated"}));
        }
        let r = call(json!({"errors": veg, "checkpoint_rmse": "1 cm", "target_vertical": "5 cm"}));
        assert!((cm(&r, "vertical") - cm(&listed, "vertical")).abs() < 1e-9);
        assert!(cm(&r, "vva") > cm(&r, "vertical"));
        assert!(
            r["result"]["blunders"].as_array().unwrap().is_empty(),
            "a vegetated height was called a blunder: {r}"
        );
    }
    // Blunders sit exactly past three times the target.
    let mut e = errors(30, 9, 1.0);
    e[4].2 = 15.0;
    let at =
        call(json!({"errors": rows(&e), "checkpoint_rmse": "1 cm", "target_vertical": "5 cm"}));
    assert!(
        at["result"]["blunders"].as_array().unwrap().is_empty(),
        "15 cm is at the limit, not past it: {at}"
    );
    e[4].2 = 15.1;
    let past =
        call(json!({"errors": rows(&e), "checkpoint_rmse": "1 cm", "target_vertical": "5 cm"}));
    assert_eq!(past["result"]["blunders"][0]["checkpoint"], 5, "{past}");
    assert!(codes(&past).contains(&"CHECKPOINT_BLUNDER".to_owned()));
    // The spec scenario: 20 checkpoints are too few.
    let few = call(json!({"errors": rows(&errors(20, 3, 1.0)), "checkpoint_rmse": "1 cm"}));
    assert!(codes(&few).contains(&"INSUFFICIENT_CHECKPOINTS".to_owned()));
}
