//! Thermal-footprint properties beyond the individual golden cases.

use gp_drone::REGISTRY;
use serde_json::{json, Value};

fn run(input: Value) -> Value {
    let response: Value = serde_json::from_str(
        &REGISTRY.invoke("drone.sensors.thermal-footprint", &input.to_string()),
    )
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

#[test]
fn angle_and_length_units_do_not_change_the_geometry() {
    let mrad = run(json!({"ifov": "1.3 mrad", "distance": "30 m"}));
    let radians = run(json!({"ifov": "0.0013 rad", "distance": "3000 cm"}));
    let degrees = run(json!({
        "ifov": format!("{} deg", 0.0013_f64.to_degrees()),
        "distance": "98.4251968504 ft",
    }));
    for key in ["footprint.value", "min_target.value"] {
        let want = number(&mrad, key);
        assert!((number(&radians, key) - want).abs() < 1e-9);
        assert!((number(&degrees, key) - want).abs() < 1e-8);
    }
}

#[test]
fn target_at_the_required_pixel_width_returns_the_starting_distance() {
    for (ifov, distance, pixels) in [(0.2, 100.0, 3.0), (1.3, 30.0, 5.0), (4.0, 2.5, 1.0)] {
        let target_mm = ifov * distance * pixels;
        let result = run(json!({
            "ifov": format!("{ifov} mrad"),
            "distance": format!("{distance} m"),
            "pixels": pixels,
            "target_size": format!("{target_mm} mm"),
        }));
        assert!((number(&result, "max_distance.value") - distance).abs() < 1e-10);
    }
}

#[test]
fn pixel_pitch_over_focal_length_matches_the_same_ifov() {
    let optics = run(json!({
        "pixel_pitch": "12 um",
        "focal_length": "9 mm",
        "distance": "50 m",
        "pixels": 5,
    }));
    let angular = run(json!({
        "ifov": format!("{} rad", 12e-6 / 9e-3),
        "distance": "50 m",
        "pixels": 5,
    }));
    assert!(
        (number(&optics, "footprint.value") - number(&angular, "footprint.value")).abs() < 1e-10
    );
    assert!(
        (number(&optics, "min_target.value") - number(&angular, "min_target.value")).abs() < 1e-10
    );
}

#[test]
fn flir_e8_worked_example_is_reproduced() {
    let result = run(json!({"ifov": "2.583333333333 mrad", "distance": "10 ft"}));
    assert!((number(&result, "footprint.value") - 0.31 * 25.4).abs() < 0.001);
    assert!((number(&result, "min_target.value") - 0.93 * 25.4).abs() < 0.003);
}
