//! Site-localization properties that must hold beyond individual examples.

use gp_survey::REGISTRY;
use serde_json::{Value, json};

fn run(input: Value) -> Value {
    let response: Value =
        serde_json::from_str(&REGISTRY.invoke("survey.gnss.localization", &input.to_string()))
            .unwrap();
    assert_eq!(response["ok"], true, "{response}");
    response["result"].clone()
}

fn number(value: &Value, path: &str) -> f64 {
    path.split('.')
        .fold(value, |current, key| &current[key])
        .as_f64()
        .unwrap()
}

fn meters(value: f64) -> String {
    format!("{value:.9} m")
}

fn similarity_rows(
    local: &[(f64, f64)],
    scale: f64,
    angle_deg: f64,
    shift_e: f64,
    shift_n: f64,
) -> Vec<Value> {
    let angle = angle_deg.to_radians();
    local
        .iter()
        .enumerate()
        .map(|(index, &(e, n))| {
            let ge = scale * (angle.cos() * e - angle.sin() * n) + shift_e;
            let gn = scale * (angle.sin() * e + angle.cos() * n) + shift_n;
            json!({
                "name": format!("P{}", index + 1),
                "local_n": meters(n),
                "local_e": meters(e),
                "grid_n": meters(gn),
                "grid_e": meters(ge),
            })
        })
        .collect()
}

#[test]
fn similarity_is_invariant_to_point_order_and_a_large_coordinate_origin() {
    let compact = [(0.0, 0.0), (3.0, 0.0), (0.0, 4.0), (3.0, 4.0)];
    let shifted = compact.map(|(e, n)| (e + 1_000_000_000.0, n + 1_000_000_000.0));
    let ordinary = run(json!({
        "pairs": similarity_rows(&compact, 0.99998, 0.4, 300_000.0, 400_000.0),
        "expected_scale": 0.99998,
    }));
    let mut rows = similarity_rows(&shifted, 0.99998, 0.4, 300_000.0, 400_000.0);
    rows.reverse();
    let large = run(json!({"pairs": rows, "expected_scale": 0.99998}));

    assert!((number(&ordinary, "scale") - number(&large, "scale")).abs() < 2e-8);
    assert!((number(&ordinary, "rotation.value") - number(&large, "rotation.value")).abs() < 2e-6);
    assert!(number(&large, "rms.value") < 2e-7, "{large}");

    for result in [&ordinary, &large] {
        let residuals = result["residuals"].as_array().unwrap();
        let sum_n: f64 = residuals.iter().map(|r| number(r, "dn.value")).sum();
        let sum_e: f64 = residuals.iter().map(|r| number(r, "de.value")).sum();
        assert!(sum_n.abs() < 1e-6 && sum_e.abs() < 1e-6, "{result}");
    }
}

#[test]
fn affine_fit_recovers_shear_and_nonuniform_scale() {
    let local = [(0.0, 0.0), (100.0, 0.0), (0.0, 80.0), (120.0, 90.0)];
    let (a, b, c, d) = (1.0002, 0.0004, -0.0003, 0.9998);
    let rows: Vec<Value> = local
        .iter()
        .enumerate()
        .map(|(index, &(e, n))| {
            json!({
                "name": format!("A{}", index + 1),
                "local_n": meters(n),
                "local_e": meters(e),
                "grid_n": meters(c * e + d * n - 900.0),
                "grid_e": meters(a * e + b * n + 600.0),
            })
        })
        .collect();
    let result = run(json!({"pairs": rows, "model": "affine"}));

    for (key, want) in [("a", a), ("b", b), ("c", c), ("d", d)] {
        assert!(
            (number(&result, key) - want).abs() < 1e-10,
            "{key}: {result}"
        );
    }
    assert!(number(&result, "rms.value") < 1e-9, "{result}");
}

#[test]
fn iogp_method_9621_parameters_are_recovered_with_the_documented_sign() {
    let scale = 1.00000155;
    // IOGP's axis-rotation convention is the opposite sign of the tool's
    // counterclockwise point-vector rotation.
    let rotation = -(1.56504 / 3600.0);
    let local = [
        (300_000.0, 4_500_000.0),
        (301_000.0, 4_500_000.0),
        (300_000.0, 4_501_000.0),
        (301_000.0, 4_501_000.0),
    ];
    let result = run(json!({
        "pairs": similarity_rows(&local, scale, rotation, -129.549, -208.185),
        "expected_scale": scale,
    }));

    assert!((number(&result, "scale") - scale).abs() < 1e-9, "{result}");
    assert!(
        (number(&result, "rotation.value") - rotation).abs() < 1e-8,
        "{result}"
    );
    assert!(
        (number(&result, "shift_e.value") + 129.549).abs() < 2e-5,
        "{result}"
    );
    assert!(
        (number(&result, "shift_n.value") + 208.185).abs() < 2e-5,
        "{result}"
    );
}
