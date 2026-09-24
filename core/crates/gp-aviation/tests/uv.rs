//! Wind components: agreement with MetPy, and the properties every answer
//! must have.

use gp_aviation::REGISTRY;
use serde_json::{Value, json};

fn uv(input: Value) -> Value {
    let r: Value = serde_json::from_str(&REGISTRY.invoke("aviation.wind.uv", &input.to_string()))
        .expect("JSON");
    assert_eq!(r["ok"], true, "{r}");
    r
}

fn num(r: &Value, k: &str) -> f64 {
    r["result"][k]["value"]
        .as_f64()
        .unwrap_or_else(|| panic!("{k} in {r}"))
}

/// Angles that differ by a multiple of 360 degrees are the same direction.
fn same_angle(a: f64, b: f64) -> bool {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d) < 1e-9
}

/// 500 winds each way against MetPy's wind_components, wind_direction, and
/// wind_speed (tools/vectors/gen_uv_metpy.py).
#[test]
fn uv_matches_metpy() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/uv_metpy.json");
    let fx: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("JSON");
    let mut wrong = Vec::new();
    for c in fx["forward"].as_array().unwrap() {
        let r = uv(
            json!({"direction": format!("{} deg", c["direction"]), "speed": format!("{} kt", c["speed"])}),
        );
        let (u, v) = (c["u"].as_f64().unwrap(), c["v"].as_f64().unwrap());
        if (num(&r, "u") - u).abs() > 1e-9 || (num(&r, "v") - v).abs() > 1e-9 {
            wrong.push(format!("{c}: {} {}", num(&r, "u"), num(&r, "v")));
        }
    }
    for c in fx["reverse"].as_array().unwrap() {
        let r = uv(json!({"u": format!("{} kt", c["u"]), "v": format!("{} kt", c["v"])}));
        if !same_angle(num(&r, "direction"), c["direction"].as_f64().unwrap())
            || (num(&r, "speed") - c["speed"].as_f64().unwrap()).abs() > 1e-9
        {
            wrong.push(format!(
                "{c}: {} {}",
                num(&r, "direction"),
                num(&r, "speed")
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of 1,000 differ:\n{}",
        wrong.len(),
        wrong[..wrong.len().min(10)].join("\n")
    );
}

#[test]
fn uv_invariants() {
    for (d, s) in [
        (0.0, 10.0),
        (90.0, 25.0),
        (225.0, 7.5),
        (300.0, 40.0),
        (359.9, 3.0),
    ] {
        let f = uv(json!({"direction": format!("{d} deg"), "speed": format!("{s} kt")}));
        let (u, v) = (num(&f, "u"), num(&f, "v"));
        // The components' length is the speed.
        assert!((u.hypot(v) - s).abs() < 1e-9, "{d} {s}");
        // And back again.
        let b = uv(json!({"u": format!("{u} kt"), "v": format!("{v} kt")}));
        assert!(same_angle(num(&b, "direction"), d), "{d}: {b}");
        assert!((num(&b, "speed") - s).abs() < 1e-9);
        // Turning the wind a quarter turn clockwise turns the vector too:
        // from d + 90, (u, v) becomes (v, -u).
        let t = uv(json!({"direction": format!("{} deg", d + 90.0), "speed": format!("{s} kt")}));
        assert!(
            (num(&t, "u") - v).abs() < 1e-9 && (num(&t, "v") + u).abs() < 1e-9,
            "{d}"
        );
    }
    // A wind from the west blows toward the east: u positive, v nothing.
    let w = uv(json!({"direction": "270 deg", "speed": "20 kt"}));
    assert!((num(&w, "u") - 20.0).abs() < 1e-9 && num(&w, "v").abs() < 1e-9);
}
