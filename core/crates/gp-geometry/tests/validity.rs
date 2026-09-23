//! Validity and repair: the property a repair must deliver, and the
//! relationships between what it reports.

use gp_geometry::REGISTRY;
use serde_json::{Value, json};

fn make_valid(polygon: &Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(
        "geometry.validity.make-valid",
        &json!({"polygon": polygon, "options": {"outputUnits": {"area": "m2"}}}).to_string(),
    ))
    .expect("JSON")
}

fn polygon_area(polygon: &Value) -> f64 {
    let r: Value = serde_json::from_str(&REGISTRY.invoke(
        "geometry.area.polygon",
        &json!({"polygon": polygon, "options": {"outputUnits": {"area": "m2"}}}).to_string(),
    ))
    .expect("JSON");
    r["result"]["area"]["value"].as_f64().unwrap()
}

fn area(r: &Value) -> f64 {
    r["result"]["area"]["value"]
        .as_f64()
        .unwrap_or_else(|| panic!("{r}"))
}

/// One part of a repaired result, as a polygon to feed back in.
fn part(r: &Value, which: i64) -> Value {
    Value::Array(
        r["result"]["repaired"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p["part"].as_i64() == Some(which))
            .map(|p| json!({"lat": p["lat"]["value"], "lon": p["lon"]["value"]}))
            .collect(),
    )
}

#[test]
fn make_valid_invariants() {
    let square = json!([{"lat":40.0,"lon":-105.0},{"lat":40.0,"lon":-104.99},
                        {"lat":40.01,"lon":-104.99},{"lat":40.01,"lon":-105.0}]);
    let bow_tie = json!([{"lat":40.0,"lon":-105.0},{"lat":40.01,"lon":-104.99},
                         {"lat":40.0,"lon":-104.99},{"lat":40.01,"lon":-105.0}]);

    // A valid polygon comes back as itself, and its area is the area tool's.
    let ok = make_valid(&square);
    assert_eq!(ok["result"]["valid"], "yes", "{ok}");
    assert_eq!(ok["result"]["parts"].as_i64().unwrap(), 1, "{ok}");
    let measured = polygon_area(&square);
    assert!(
        (area(&ok) - measured).abs() / measured < 1e-9,
        "a valid square's area came back as {} against {measured}",
        area(&ok)
    );

    // Winding is not part of validity.
    let reversed = Value::Array(square.as_array().unwrap().iter().rev().cloned().collect());
    let rev = make_valid(&reversed);
    assert_eq!(rev["result"]["valid"], "yes", "{rev}");
    assert!(
        (area(&rev) - area(&ok)).abs() / area(&ok) < 1e-9,
        "winding changed the area"
    );

    // The bow tie: invalid, two parts, and every problem located.
    let bad = make_valid(&bow_tie);
    assert_eq!(bad["result"]["valid"], "no", "{bad}");
    assert_eq!(bad["result"]["parts"].as_i64().unwrap(), 2, "{bad}");
    let problems = bad["result"]["problems"].as_array().unwrap();
    assert_eq!(
        problems.len() as i64,
        bad["result"]["problem_count"].as_i64().unwrap(),
        "the problem list is not as long as the count it reports"
    );
    assert!(
        !problems.is_empty(),
        "an invalid polygon reported no problems\n{bad}"
    );
    for p in problems {
        let (lat, lon) = (
            p["lat"]["value"].as_f64().unwrap(),
            p["lon"]["value"].as_f64().unwrap(),
        );
        assert!(
            (40.0..=40.01).contains(&lat) && (-105.0..=-104.99).contains(&lon),
            "a problem is located at {lat},{lon}, outside the polygon's own box"
        );
    }

    // The property the tool exists to deliver: what comes back is valid. A
    // partial repair would fail here and nowhere else.
    let mut sum = 0.0;
    for which in 0..bad["result"]["parts"].as_i64().unwrap() {
        let piece = part(&bad, which);
        assert!(
            piece.as_array().unwrap().len() >= 3,
            "repaired part {which} has fewer than three corners"
        );
        let again = make_valid(&piece);
        assert_eq!(
            again["result"]["valid"], "yes",
            "repaired part {which} is still invalid\n{again}"
        );
        sum += area(&again);
    }
    assert!(
        (sum - area(&bad)).abs() / area(&bad) < 1e-9,
        "the parts total {sum} against the reported {}",
        area(&bad)
    );

    // Nothing to repair is refused, naming what it needs.
    let degenerate = make_valid(&json!([{"lat":40.0,"lon":-105.0},{"lat":40.0,"lon":-105.0},
                                        {"lat":40.0,"lon":-105.0}]));
    assert!(!degenerate["ok"].as_bool().unwrap_or(true), "{degenerate}");
    assert!(
        degenerate["error"]["message"]
            .as_str()
            .unwrap_or("")
            .contains("distinct corners"),
        "the refusal does not say what it needs: {degenerate}"
    );
}
