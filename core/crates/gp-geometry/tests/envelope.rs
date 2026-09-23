//! Bounding boxes: the properties a box must have, and the two that make a
//! geodesic box different from taking the minimum and maximum of the corners.

use gp_geometry::REGISTRY;
use serde_json::{Value, json};

fn bbox(points: &Value, shape: &str) -> Value {
    serde_json::from_str(&REGISTRY.invoke(
        "geometry.shape.bbox",
        &json!({"points": points, "shape": shape}).to_string(),
    ))
    .expect("JSON")
}

fn f(r: &Value, key: &str) -> f64 {
    r["result"][key]["value"]
        .as_f64()
        .unwrap_or_else(|| panic!("{key} in {r}"))
}

fn warned(r: &Value, code: &str) -> bool {
    r["meta"]["warnings"]
        .as_array()
        .map(|a| a.iter().any(|w| w["code"] == code))
        .unwrap_or(false)
}

/// Whether a point falls in the box, handling the crossing case where west is
/// greater than east. A comparison that forgets this is the classic bug.
fn holds(r: &Value, lat: f64, lon: f64) -> bool {
    let (w, e) = (f(r, "west"), f(r, "east"));
    let lon_ok = if w <= e {
        lon >= w - 1e-9 && lon <= e + 1e-9
    } else {
        lon >= w - 1e-9 || lon <= e + 1e-9
    };
    lon_ok && lat >= f(r, "south") - 1e-9 && lat <= f(r, "north") + 1e-9
}

#[test]
fn bbox_invariants() {
    let line = json!([{"lat":60.0,"lon":-60.0},{"lat":60.0,"lon":60.0}]);
    let across = json!([{"lat":-17.0,"lon":178.0},{"lat":-17.0,"lon":-178.0},
                        {"lat":-15.0,"lon":-178.0},{"lat":-15.0,"lon":178.0}]);

    // The one thing a bounding box must never get wrong.
    for (name, pts, shape) in [
        ("a bulging line", &line, "line"),
        ("the same as points", &line, "points"),
        ("a box across the antimeridian", &across, "polygon"),
    ] {
        let r = bbox(pts, shape);
        for p in pts.as_array().unwrap() {
            let (lat, lon) = (p["lat"].as_f64().unwrap(), p["lon"].as_f64().unwrap());
            assert!(
                holds(&r, lat, lon),
                "{name}: {lat},{lon} is outside its own box\n{r}"
            );
        }
    }

    // Edges bow poleward, and only a line or polygon has edges. The same two
    // points bound differently depending on what you say they are.
    let as_line = bbox(&line, "line");
    let as_points = bbox(&line, "points");
    assert_eq!(
        f(&as_points, "north"),
        60.0,
        "points should not bulge\n{as_points}"
    );
    assert!(
        f(&as_line, "north") > 73.0,
        "a 120 deg line at 60 N reaches {} N, not past 73",
        f(&as_line, "north")
    );
    // Adding edges can only grow the box, never shrink it.
    assert!(
        f(&as_line, "north") >= f(&as_points, "north")
            && f(&as_line, "south") <= f(&as_points, "south"),
        "the line's box is not at least the points' box"
    );

    // The short way across the antimeridian, written west > east as RFC 7946
    // asks, and flagged. A box that does not cross says so by staying quiet.
    let am = bbox(&across, "polygon");
    assert!(
        f(&am, "west") > f(&am, "east"),
        "a crossing box wrote west < east\n{am}"
    );
    assert_eq!(am["result"]["crosses_antimeridian"], "yes", "{am}");
    assert!(warned(&am, "CROSSES_ANTIMERIDIAN"), "{am}");
    assert!(
        (f(&am, "lon_span") - 4.0).abs() < 1e-9,
        "the short box is {} deg wide, not 4",
        f(&am, "lon_span")
    );
    let plain = bbox(
        &json!([{"lat":40.0,"lon":-105.0},{"lat":41.0,"lon":-104.0}]),
        "points",
    );
    assert!(f(&plain, "west") < f(&plain, "east"), "{plain}");
    assert_eq!(plain["result"]["crosses_antimeridian"], "no", "{plain}");
    assert!(!warned(&plain, "CROSSES_ANTIMERIDIAN"), "{plain}");

    // The span is what going east from west to east covers, always.
    for r in [&as_line, &as_points, &am, &plain] {
        let span = f(r, "lon_span");
        assert!((0.0..=360.0).contains(&span), "a span of {span} deg\n{r}");
        let walked = (f(r, "east") - f(r, "west")).rem_euclid(360.0);
        assert!(
            (walked - span).abs() < 1e-9 || (span - 360.0).abs() < 1e-9,
            "span {span} against {walked} walked east from west\n{r}"
        );
    }

    // The order the points arrive in is not part of the answer.
    let reversed: Vec<Value> = across.as_array().unwrap().iter().rev().cloned().collect();
    let rev = bbox(&Value::Array(reversed), "polygon");
    for k in ["west", "south", "east", "north"] {
        assert_eq!(f(&rev, k), f(&am, k), "reversing the points changed {k}");
    }
}
