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
