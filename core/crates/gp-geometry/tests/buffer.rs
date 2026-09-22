//! Geodesic buffers: the spec scenarios, measured against the geodesic
//! distance to the input, across the antimeridian, and near a pole.

use geographiclib_rs::{Geodesic, InverseGeodesic};
use gp_geometry::REGISTRY;
use serde_json::{Value, json};

fn call(input: &Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke("geometry.buffer.geodesic", &input.to_string()))
        .expect("JSON")
}

fn num(r: &Value, k: &str) -> f64 {
    r["result"][k]["value"]
        .as_f64()
        .or_else(|| r["result"][k].as_f64())
        .unwrap_or_else(|| panic!("{k} in {r}"))
}

fn codes(r: &Value) -> Vec<String> {
    r["meta"]["warnings"].as_array().map_or(vec![], |a| {
        a.iter()
            .map(|w| w["code"].as_str().unwrap().to_owned())
            .collect()
    })
}

fn field() -> Value {
    json!([{"lat":40.0,"lon":-105.0},{"lat":40.0,"lon":-104.99},{"lat":40.006,"lon":-104.99},{"lat":40.006,"lon":-105.0}])
}

#[test]
fn geofence_buffer_is_500_m_within_half_a_meter() {
    let t = std::time::Instant::now();
    let r = call(&json!({"vertices": field(), "distance": "500 m"}));
    assert_eq!(r["ok"], true, "{r}");
    assert!(
        num(&r, "max_deviation") <= 0.5,
        "{}",
        num(&r, "max_deviation")
    );
    assert_eq!(num(&r, "parts"), 1.0);
    assert!(codes(&r).iter().all(|c| c != "BUFFER_ACCURACY"));
    // Independently: every boundary vertex is 500 m from the nearest field corner or edge,
    // checked here by sampling each field edge densely.
    let g = Geodesic::wgs84();
    let f: Vec<(f64, f64)> = field()
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["lat"].as_f64().unwrap(), p["lon"].as_f64().unwrap()))
        .collect();
    let mut samples = Vec::new();
    for i in 0..4 {
        let (a, b) = (f[i], f[(i + 1) % 4]);
        for k in 0..=2000 {
            let t = k as f64 / 2000.0;
            samples.push((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t));
        }
    }
    for v in r["result"]["boundary"].as_array().unwrap() {
        let p = (
            v["lat"]["value"].as_f64().unwrap(),
            v["lon"]["value"].as_f64().unwrap(),
        );
        let d = samples
            .iter()
            .map(|s| {
                let x: f64 = g.inverse(s.0, s.1, p.0, p.1);
                x
            })
            .fold(f64::INFINITY, f64::min);
        // Sampling (and edges along meridians and parallels, not geodesics) adds a little.
        assert!((d - 500.0).abs() < 1.0, "{d}");
    }
    eprintln!("geofence buffer: {:?}", t.elapsed());
}

#[test]
fn negative_buffer_collapses() {
    // About 100 m wide (0.0009° of latitude), buffered in by 60 m.
    let r = call(
        &json!({"vertices":[{"lat":40.0,"lon":-105.0},{"lat":40.0,"lon":-104.99},{"lat":40.0009,"lon":-104.99},{"lat":40.0009,"lon":-105.0}],"distance":"-60 m"}),
    );
    assert_eq!(r["ok"], true, "{r}");
    assert_eq!(num(&r, "parts"), 0.0);
    assert!(codes(&r).iter().any(|c| c == "BUFFER_COLLAPSED"));
    // By 20 m it shrinks instead: about 60 m × 810 m.
    let r = call(
        &json!({"vertices":[{"lat":40.0,"lon":-105.0},{"lat":40.0,"lon":-104.99},{"lat":40.0009,"lon":-104.99},{"lat":40.0009,"lon":-105.0}],"distance":"-20 m"}),
    );
    assert_eq!(num(&r, "parts"), 1.0);
    assert!(num(&r, "max_deviation") <= 0.5);
}

#[test]
fn a_point_buffer_is_a_geodesic_circle_even_across_the_antimeridian_and_at_a_pole() {
    for (lat, lon) in [
        (40.0, -105.0),
        (0.0, 179.999),
        (89.999, 0.0),
        (-60.0, -180.0),
    ] {
        let r = call(&json!({"vertices":[{"lat":lat,"lon":lon}],"distance":"2 km"}));
        assert_eq!(r["ok"], true, "{r}");
        let area = num(&r, "area") * 1e6;
        let circle = core::f64::consts::PI * 2000.0 * 2000.0;
        assert!(
            (area / circle - 1.0).abs() < 1e-3,
            "{lat},{lon}: {area} vs {circle}"
        );
        assert!(
            num(&r, "max_deviation") <= 2.0,
            "{}",
            num(&r, "max_deviation")
        );
    }
}

#[test]
fn a_line_with_flat_ends_and_a_long_line() {
    // 1 km along the equator, 50 m each side: 0.1 km².
    let r = call(
        &json!({"vertices":[{"lat":0,"lon":0},{"lat":0,"lon":0.008983152841195214}],"distance":"50 m","cap":"flat"}),
    );
    assert!((num(&r, "area") - 0.1).abs() < 1e-4, "{}", num(&r, "area"));
    // 300 km across the antimeridian, 10 km each side.
    let t = std::time::Instant::now();
    let r = call(
        &json!({"vertices":[{"lat":10,"lon":178.5},{"lat":11,"lon":-179.0},{"lat":10.5,"lon":-178.0}],"distance":"10 km"}),
    );
    assert_eq!(r["ok"], true, "{r}");
    assert_eq!(num(&r, "parts"), 1.0);
    assert!(
        num(&r, "max_deviation") <= num(&r, "tolerance"),
        "{} > {}",
        num(&r, "max_deviation"),
        num(&r, "tolerance")
    );
    eprintln!("long line: {:?}", t.elapsed());
}

#[test]
fn mitre_corners_respect_the_limit() {
    let r = call(&json!({"vertices": field(), "distance": "100 m", "join": "mitre"}));
    assert_eq!(r["ok"], true, "{r}");
    // A rectangle's mitred buffer is a rectangle: four corners.
    assert_eq!(num(&r, "vertex_count"), 4.0, "{}", r["result"]["boundary"]);
    let r = call(
        &json!({"vertices": field(), "distance": "100 m", "join": "mitre", "mitre_limit": 1.2}),
    );
    // √2 > 1.2, so the right-angle corners are beveled: eight vertices.
    assert_eq!(num(&r, "vertex_count"), 8.0);
}
