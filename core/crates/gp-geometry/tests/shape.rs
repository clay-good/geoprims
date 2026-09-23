//! Centroids and interior points: the C-shape scenario.

use gp_geometry::REGISTRY;
use serde_json::{Value, json};

fn call(input: &Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke("geometry.shape.centroid", &input.to_string()))
        .expect("JSON")
}

#[test]
fn a_c_shapes_centroid_is_outside_and_its_interior_point_is_inside() {
    // A 333 m square with a notch cut from its east side, 200 m deep and 200 m tall.
    let c = json!([{"lat":40.0,"lon":-105.0},{"lat":40.0,"lon":-104.997},{"lat":40.0006,"lon":-104.997},{"lat":40.0006,"lon":-104.9994},
        {"lat":40.0024,"lon":-104.9994},{"lat":40.0024,"lon":-104.997},{"lat":40.003,"lon":-104.997},{"lat":40.003,"lon":-105.0}]);
    let r = call(&json!({"polygon": c}));
    assert_eq!(r["result"]["centroid_inside"], "no", "{r}");
    let codes: Vec<&str> = r["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap())
        .collect();
    assert!(codes.contains(&"CENTROID_OUTSIDE"));
    // The interior point lies in the solid west bar (lon under -104.9994), inside the shape.
    let (lat, lon) = (
        r["result"]["interior_lat"]["value"].as_f64().unwrap(),
        r["result"]["interior_lon"]["value"].as_f64().unwrap(),
    );
    let inside = call(&json!({"polygon": c}));
    assert!(
        lon < -104.9994 && (40.0..40.003).contains(&lat),
        "{lat}, {lon}; {inside}"
    );
    // The bar is 51 m wide (0.0006° of longitude) and the arms 67 m thick
    // (0.0006° of latitude); the widest circle sits where bar and arm meet,
    // pushed toward the notch's corner, like an L: between the half-widths
    // and the corner-fit radius of about 35 m.
    let clearance = r["result"]["clearance"]["value"].as_f64().unwrap();
    assert!(clearance > 33.0 && clearance < 38.0, "{clearance}");
}

#[test]
fn a_corner_pushed_to_the_pole_answers_quickly() {
    // Found by the fuzzer: one corner at 90° made the shape thousands of edge pieces long.
    let t = std::time::Instant::now();
    let r = call(
        &json!({"polygon":[{"lat":40,"lon":-105},{"lat":40,"lon":-104.997},{"lat":40.0006,"lon":-104.997},{"lat":40.0006,"lon":-104.9994},{"lat":40.0024,"lon":-104.9994},{"lat":40.0024,"lon":-104.997},{"lat":90,"lon":-104.997},{"lat":40.003,"lon":-105}]}),
    );
    assert_eq!(r["ok"], true, "{r}");
    assert_eq!(r["result"]["centroid_inside"], "yes");
    // A bounded search: well under a second natively, a few in size-optimized Wasm.
    assert!(t.elapsed().as_secs_f64() < 1.5, "{:?}", t.elapsed());
}

/// Any tool in this crate's registry, so the centroid can be checked against
/// its neighbours rather than against itself.
fn tool(id: &str, input: &Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).expect("JSON")
}

fn f(r: &Value, path: &str) -> f64 {
    path.split('.')
        .fold(r, |v, k| &v[k])
        .as_f64()
        .unwrap_or_else(|| panic!("{path} in {r}"))
}

fn warned(r: &Value, code: &str) -> bool {
    r["meta"]["warnings"]
        .as_array()
        .map(|a| a.iter().any(|w| w["code"] == code))
        .unwrap_or(false)
}

#[test]
fn centroid_invariants() {
    let centroid = |poly: &Value| {
        call(&json!({"polygon": poly,
                     "options": {"outputUnits": {"area": "km2", "clearance": "m"}}}))
    };

    // Symmetric about a meridian: the centroid must sit on it, exactly. No
    // amount of projection error could fake this, and a sign slip would break it.
    let square = json!([{"lat":40.0,"lon":-105.0},{"lat":40.0,"lon":-104.0},
                        {"lat":40.5,"lon":-104.0},{"lat":40.5,"lon":-105.0}]);
    let sq = centroid(&square);
    assert_eq!(
        f(&sq, "result.centroid_lon.value"),
        -104.5,
        "the centroid left the axis of symmetry\n{sq}"
    );
    // Symmetric about the equator too: the centroid is on it.
    let straddle = json!([{"lat":-0.25,"lon":-0.25},{"lat":-0.25,"lon":0.25},
                          {"lat":0.25,"lon":0.25},{"lat":0.25,"lon":-0.25}]);
    let eq = centroid(&straddle);
    assert!(
        f(&eq, "result.centroid_lat.value").abs() < 1e-12
            && f(&eq, "result.centroid_lon.value").abs() < 1e-12,
        "a shape symmetric about the equator and the prime meridian is not centred there\n{eq}"
    );

    // The area is geometry.area.polygon's, to a part in ten million. Not
    // identical: this measures the densified polygon on the equal-area plane,
    // that one integrates the geodesic edges.
    let by_area = tool(
        "geometry.area.polygon",
        &json!({"polygon": square, "options": {"outputUnits": {"area": "km2"}}}),
    );
    let (a, b) = (
        f(&sq, "result.area.value"),
        f(&by_area, "result.area.value"),
    );
    assert!(
        (a - b).abs() / b < 1e-7,
        "area {a} against geometry.area.polygon's {b}"
    );

    // Winding is not part of the answer.
    let reversed = json!([{"lat":40.5,"lon":-105.0},{"lat":40.5,"lon":-104.0},
                          {"lat":40.0,"lon":-104.0},{"lat":40.0,"lon":-105.0}]);
    let rev = centroid(&reversed);
    assert!(
        (f(&rev, "result.centroid_lat.value") - f(&sq, "result.centroid_lat.value")).abs() < 1e-12
            && (f(&rev, "result.area.value") - f(&sq, "result.area.value")).abs() / a < 1e-9,
        "reversing the winding changed the answer\n{rev}"
    );

    // A C shape's centre of mass falls outside it. That is correct, not an
    // error, and only that case is flagged.
    let c_shape = json!([{"lat":48.0,"lon":2.0},{"lat":48.0,"lon":2.6},{"lat":48.1,"lon":2.6},
                         {"lat":48.1,"lon":2.2},{"lat":48.4,"lon":2.2},{"lat":48.4,"lon":2.6},
                         {"lat":48.5,"lon":2.6},{"lat":48.5,"lon":2.0}]);
    let cs = centroid(&c_shape);
    assert_eq!(cs["result"]["centroid_inside"], "no", "{cs}");
    assert!(warned(&cs, "CENTROID_OUTSIDE"), "{cs}");
    assert_eq!(sq["result"]["centroid_inside"], "yes", "{sq}");
    assert!(
        !warned(&sq, "CENTROID_OUTSIDE"),
        "a convex shape was flagged\n{sq}"
    );

    // The interior point is what it promises: inside, in both cases, by a tool
    // that knows nothing of how it was chosen. And that tool's own distance to
    // the nearest edge is the clearance reported here.
    for (name, poly, r) in [("square", &square, &sq), ("C shape", &c_shape, &cs)] {
        let inside = tool(
            "geometry.predicate.point-in-polygon",
            &json!({"polygon": poly,
                    "points": [{"lat": f(r, "result.interior_lat.value"),
                                "lon": f(r, "result.interior_lon.value")}]}),
        );
        assert_eq!(
            inside["result"]["first"], "inside",
            "the interior point of the {name} is not inside it\n{inside}"
        );
        let edge = inside["result"]["results"][0]["distance"]["value"]
            .as_f64()
            .unwrap_or_else(|| panic!("{inside}"));
        assert!(
            (edge - f(r, "result.clearance.value")).abs() < 1e-6,
            "{name}: clearance {} against the {edge} m measured to the edge",
            f(r, "result.clearance.value")
        );
    }
}
