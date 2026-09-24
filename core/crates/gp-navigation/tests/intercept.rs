//! Intercepting a moving target: agreement with a separate solver, and the
//! properties every answer must have.

use gp_navigation::REGISTRY;
use serde_json::{Value, json};

fn call(id: &str, input: Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).expect("JSON")
}

fn num(r: &Value, k: &str) -> f64 {
    r["result"][k]["value"]
        .as_f64()
        .unwrap_or_else(|| panic!("{k} in {r}"))
}

fn intercept(c: &Value) -> Value {
    call(
        "navigation.route.intercept",
        json!({"lat": c["lat"], "lon": c["lon"], "speed": format!("{} kt", c["speed"]),
               "target_lat": c["target_lat"], "target_lon": c["target_lon"],
               "target_course": format!("{} deg", c["target_course"]),
               "target_speed": format!("{} kt", c["target_speed"])}),
    )
}

/// 200 cases from 1 to 150 NM, 5 to 40 kt against 0 to 30 kt, 59 of them
/// unreachable, against the first root found on a fixed 0.01 h grid and
/// bisection with GeographicLib's Python package
/// (tools/vectors/gen_intercept_more.py).
#[test]
fn intercept_matches_a_grid_solver() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/intercept.json");
    let fx: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("JSON");
    let cases = fx["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 200);
    let mut wrong = Vec::new();
    for (i, c) in cases.iter().enumerate() {
        let r = intercept(c);
        if c["unreachable"] == true {
            if r["error"]["code"] != "NO_SOLUTION" {
                wrong.push(format!(
                    "case {i}: the reference finds no intercept, the tool {r}"
                ));
            }
            continue;
        }
        if r["ok"] != true {
            wrong.push(format!("case {i}: {r}"));
            continue;
        }
        let t = num(&r, "time") / 60.0;
        let want = c["time_h"].as_f64().unwrap();
        if (t - want).abs() > 1e-6
            || (num(&r, "distance") - c["distance"].as_f64().unwrap()).abs() > 0.02
        {
            wrong.push(format!("case {i}: {t} h, reference {want} h"));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of 200 differ:\n{}",
        wrong.len(),
        wrong[..wrong.len().min(10)].join("\n")
    );
}

#[test]
fn intercept_invariants() {
    let base = json!({"lat": 40.0, "lon": -70.0, "speed": "20 kt", "target_lat": 40.1665,
                      "target_lon": -70.0, "target_course": "90 deg", "target_speed": "12 kt"});
    let r = call("navigation.route.intercept", base.clone());
    let (t_min, run, course) = (num(&r, "time"), num(&r, "distance"), num(&r, "course"));
    // The run is the pursuer's speed times the time.
    assert!((run - 20.0 * 1852.0 * t_min / 60.0).abs() < 0.01, "{r}");
    // Flying the course for the run, and the target running its course for
    // the time, both arrive at the meeting point (through the direct tool).
    let (mla, mlo) = (num(&r, "meet_lat"), num(&r, "meet_lon"));
    let mine = call(
        "navigation.geodesic.direct",
        json!({"lat1": 40.0, "lon1": -70.0, "azimuth": format!("{course} deg"), "distance": format!("{run} m")}),
    );
    let theirs = call(
        "navigation.geodesic.direct",
        json!({"lat1": 40.1665, "lon1": -70.0, "azimuth": "90 deg", "distance": format!("{} m", 12.0 * 1852.0 * t_min / 60.0)}),
    );
    for d in [&mine, &theirs] {
        assert!(
            (num(d, "lat2") - mla).abs() < 1e-7 && (num(d, "lon2") - mlo).abs() < 1e-7,
            "{d} vs {mla}, {mlo}"
        );
    }
    // Faster is sooner; a stationary target is met after the plain distance.
    let mut fast = base.clone();
    fast["speed"] = json!("25 kt");
    assert!(num(&call("navigation.route.intercept", fast), "time") < t_min);
    let mut still = base.clone();
    still["target_speed"] = json!("0 kt");
    let s = call("navigation.route.intercept", still);
    let inv = call(
        "navigation.geodesic.inverse",
        json!({"lat1": 40.0, "lon1": -70.0, "lat2": 40.1665, "lon2": -70.0}),
    );
    assert!(
        // The inverse tool reports kilometers.
        (num(&s, "distance") - 1000.0 * num(&inv, "distance")).abs() < 0.01,
        "{s}"
    );
    // A target running straight away faster than the pursuer is never met.
    let mut away = base;
    away["target_course"] = json!("0 deg");
    away["target_speed"] = json!("25 kt");
    assert_eq!(
        call("navigation.route.intercept", away)["error"]["code"],
        "NO_SOLUTION"
    );
}
