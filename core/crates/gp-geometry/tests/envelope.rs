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

fn enclosing(points: &Value) -> Value {
    serde_json::from_str(
        &REGISTRY.invoke(
            "geometry.shape.enclosing",
            &json!({"points": points,
                "options": {"outputUnits": {"circle_radius": "m", "hull_area": "m2",
                                            "rect_width": "m", "rect_length": "m"}}})
            .to_string(),
        ),
    )
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

#[test]
fn enclosing_invariants() {
    use geographiclib_rs::{Geodesic, InverseGeodesic};
    let g = Geodesic::wgs84();

    let sets = [
        (
            "a quadrilateral",
            json!([{"lat":40.0,"lon":-105.0},{"lat":40.01,"lon":-104.99},
                                   {"lat":40.005,"lon":-104.97},{"lat":39.995,"lon":-104.98}]),
        ),
        (
            "a triangle",
            json!([{"lat":-20.0,"lon":30.0},{"lat":-20.05,"lon":30.12},
                              {"lat":-20.11,"lon":30.03}]),
        ),
        (
            "a cluster",
            json!([{"lat":35.0,"lon":139.0},{"lat":35.02,"lon":139.03},
                             {"lat":35.01,"lon":139.06},{"lat":34.98,"lon":139.04},
                             {"lat":34.99,"lon":139.01},{"lat":35.03,"lon":139.02}]),
        ),
    ];

    for (name, points) in &sets {
        let r = enclosing(points);
        assert!(r["ok"].as_bool().unwrap_or(false), "{name}: {r}");
        let (clat, clon, radius) = (
            f(&r, "circle_lat"),
            f(&r, "circle_lon"),
            f(&r, "circle_radius"),
        );

        // Every point in the circle, and at least one on it. A circle with room
        // to spare is not the smallest one. Measured with geographiclib-rs, not
        // with anything the tool produced.
        let mut furthest = 0.0f64;
        for p in points.as_array().unwrap() {
            let d: f64 = g.inverse(
                clat,
                clon,
                p["lat"].as_f64().unwrap(),
                p["lon"].as_f64().unwrap(),
            );
            assert!(
                d <= radius + 1e-6,
                "{name}: a point is {d} m out of a {radius} m circle"
            );
            furthest = furthest.max(d);
        }
        assert!(
            (furthest - radius).abs() < 1e-6,
            "{name}: the furthest point is {furthest} m inside a {radius} m circle -- not the smallest"
        );

        // Hull corners are input points, never new positions, and there are
        // between two and all of them.
        let n = points.as_array().unwrap().len();
        let corners = r["result"]["hull_count"].as_u64().unwrap() as usize;
        assert!(
            (2..=n).contains(&corners),
            "{name}: {corners} corners from {n} points"
        );

        // The hull's area is geometry.area.polygon's for its own corners.
        let outline: Vec<Value> = r["result"]["outlines"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p["shape"] == "hull")
            .map(|p| json!({"lat": p["lat"]["value"], "lon": p["lon"]["value"]}))
            .collect();
        // The filter must actually find the hull: getting the field name wrong
        // leaves an empty list and an assertion that passes without testing
        // anything, which is how this was written the first time.
        assert_eq!(
            outline.len(),
            corners,
            "{name}: the outlines carry {} hull points, not the {corners} reported",
            outline.len()
        );
        if outline.len() >= 3 {
            let measured = polygon_area(&Value::Array(outline));
            let reported = f(&r, "hull_area");
            assert!(
                (measured - reported).abs() / reported < 1e-6,
                "{name}: hull area {reported} against geometry.area.polygon's {measured}"
            );
        }

        // The rectangle holds the hull, so it cannot be smaller.
        let rect = f(&r, "rect_width") * f(&r, "rect_length");
        assert!(
            rect >= f(&r, "hull_area") - 1e-6,
            "{name}: a {rect} m2 rectangle around a {} m2 hull",
            f(&r, "hull_area")
        );

        // A theorem about triangles: every minimal enclosing rectangle of a
        // triangle has exactly twice its area. That is why the vectors do not
        // pin which of the three the tool returns -- there is nothing to
        // choose between them -- and why this holds whichever it returned.
        if corners == 3 {
            assert!(
                (rect / (2.0 * f(&r, "hull_area")) - 1.0).abs() < 1e-5,
                "{name}: the rectangle is {rect} m2 around a triangle of {} m2",
                f(&r, "hull_area")
            );
        }
    }

    // Two points degenerate honestly.
    let two = enclosing(&json!([{"lat":51.5,"lon":-0.12},{"lat":48.86,"lon":2.35}]));
    assert_eq!(two["result"]["hull_count"].as_u64().unwrap(), 2, "{two}");
    assert_eq!(f(&two, "hull_area"), 0.0, "{two}");
    // Not exactly zero: the rectangle's short side comes out at about 15
    // picometres, which is what subtracting two nearly equal projected
    // coordinates leaves behind. Held at a micrometre rather than at zero.
    assert!(f(&two, "rect_width") < 1e-6, "{two}");
}
