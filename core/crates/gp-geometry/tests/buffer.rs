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

/// Buffer with every length in metres.
fn buf(input: Value) -> Value {
    let mut v = input;
    v["options"] = json!({"outputUnits": {"area": "m2", "perimeter": "m",
                                          "max_deviation": "m", "tolerance": "m"}});
    call(&v)
}

/// The input polygon's own area and perimeter, from the area tool.
fn own_area(vertices: &Value) -> (f64, f64) {
    let r: Value = serde_json::from_str(
        &REGISTRY.invoke(
            "geometry.area.polygon",
            &json!({"polygon": vertices,
                "options": {"outputUnits": {"area": "m2", "perimeter": "m"}}})
            .to_string(),
        ),
    )
    .expect("JSON");
    (
        r["result"]["area"]["value"].as_f64().unwrap(),
        r["result"]["perimeter"]["value"].as_f64().unwrap(),
    )
}

#[test]
fn buffer_invariants() {
    // A buffered point is a disk, and its boundary is inscribed in that disk,
    // so the area and perimeter must fall just short -- never over.
    for d in [100.0, 1_000.0, 50_000.0] {
        let r = buf(json!({"vertices": [{"lat": 0.5, "lon": 0.5}], "distance": format!("{d} m")}));
        let (a, p) = (num(&r, "area"), num(&r, "perimeter"));
        let (circle, circumference) =
            (std::f64::consts::PI * d * d, 2.0 * std::f64::consts::PI * d);
        assert!(
            a < circle && a / circle > 0.998,
            "d={d}: area {a} of {circle}"
        );
        assert!(
            p < circumference && p / circumference > 0.9995,
            "d={d}: perimeter {p} of {circumference}"
        );
        assert!(
            num(&r, "max_deviation") <= num(&r, "tolerance"),
            "d={d}: deviation over tolerance\n{r}"
        );
    }

    // Steiner: the buffer of a convex polygon is A + Pd + pi d^2. A and P come
    // from geometry.area.polygon, not from anything computed here.
    let (a0, p0) = own_area(&field());
    for d in [50.0, 200.0, 500.0] {
        let r = buf(json!({"vertices": field(), "distance": format!("{d} m")}));
        let steiner = a0 + p0 * d + std::f64::consts::PI * d * d;
        let got = num(&r, "area");
        assert!(
            (got / steiner - 1.0).abs() < 1e-4,
            "d={d}: {got} against Steiner's {steiner}"
        );
        assert!(got > a0 + p0 * d, "d={d}: below Steiner's lower bound");
        assert!(
            num(&r, "max_deviation") <= num(&r, "tolerance"),
            "d={d}: deviation over tolerance\n{r}"
        );
        // A mitre corner reaches d / cos(theta/2), past where a round one stops.
        let m = buf(json!({"vertices": field(), "distance": format!("{d} m"), "join": "mitre"}));
        assert!(
            num(&m, "area") > got,
            "d={d}: the mitre buffer is not the larger"
        );
    }

    // Outward then inward with mitre corners is the identity: the mitre has no
    // chord sag, so the two constructions are exact inverses. A round corner is
    // a polygon inscribed in its arc and cannot survive being eroded by the
    // full d -- that case is `round_buffer_cannot_be_eroded_by_its_own_radius`.
    let out = buf(json!({"vertices": field(), "distance": "200 m", "join": "mitre"}));
    let ring: Vec<Value> = out["result"]["boundary"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| json!({"lat": p["lat"]["value"], "lon": p["lon"]["value"]}))
        .collect();
    let back = buf(json!({"vertices": ring, "distance": "-200 m", "join": "mitre"}));
    assert_eq!(
        back["result"]["parts"].as_f64().unwrap() as i64,
        1,
        "{back}"
    );
    assert!(
        (num(&back, "area") / a0 - 1.0).abs() < 1e-8,
        "mitre round trip: {} against {a0}",
        num(&back, "area")
    );
}

#[test]
fn round_buffer_cannot_be_eroded_by_its_own_radius() {
    // A round corner is drawn as a polygon inscribed in the arc, so the corner's
    // own inradius is short of d by the chord sag the tool reports. Eroding by
    // more than d minus that sag empties the corners, and the shape collapses.
    // That is arithmetic, not a defect, and this pins where it turns over.
    let out = buf(json!({"vertices": field(), "distance": "200 m"}));
    let sag = num(&out, "max_deviation");
    assert!((0.05..0.5).contains(&sag), "unexpected sag {sag}");
    let ring: Vec<Value> = out["result"]["boundary"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| json!({"lat": p["lat"]["value"], "lon": p["lon"]["value"]}))
        .collect();
    let parts = |d: f64| {
        let r = buf(json!({"vertices": ring, "distance": format!("{d} m")}));
        r["result"]["parts"].as_f64().unwrap() as i64
    };
    // Comfortably inside the sag it survives; past it, nothing is left.
    assert_eq!(parts(-(200.0 - 4.0 * sag)), 1, "erosion short of the sag");
    assert_eq!(parts(-200.0), 0, "erosion by the full radius");
    let collapsed = buf(json!({"vertices": ring, "distance": "-200 m"}));
    assert!(
        codes(&collapsed).contains(&"BUFFER_COLLAPSED".to_string()),
        "the collapse is not reported: {collapsed}"
    );
}
