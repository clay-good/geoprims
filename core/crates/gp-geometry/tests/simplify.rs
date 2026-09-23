//! Simplification: the narrow-inlet scenario, the geodesic tolerance, and
//! the target count.

use gp_geometry::REGISTRY;
use serde_json::{Value, json};

fn call(id: &str, input: &Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).expect("JSON")
}

/// Local meters east and north of 40° N, 105° W, as vertices.
fn ring(xy: &[(f64, f64)]) -> Value {
    Value::Array(
        xy.iter()
            .map(|&(x, y)| json!({"lat": 40.0 + y / 111_034.0, "lon": -105.0 + x / 85_395.0}))
            .collect(),
    )
}

/// A square with a 60 m inlet from its east side. The inlet's upper wall
/// bows up by 40 m at x = 300 while a 75 m spike rises from its lower wall
/// just below, so at a 45 m tolerance plain RDP straightens the wall through
/// the spike.
fn inlet() -> Value {
    ring(&[
        (0.0, 0.0),
        (1000.0, 0.0),
        (1000.0, 470.0),
        (310.0, 470.0),
        (300.0, 545.0),
        (290.0, 470.0),
        (60.0, 470.0),
        (50.0, 530.0),
        (300.0, 570.0),
        (1000.0, 530.0),
        (1000.0, 1000.0),
        (0.0, 1000.0),
    ])
}

fn valid(points: &Value) -> String {
    // Output rows carry {value, unit}; feed plain degrees back in.
    let plain: Vec<Value> = points
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            let v = |k: &str| p[k].get("value").unwrap_or(&p[k]).clone();
            json!({"lat": v("lat"), "lon": v("lon")})
        })
        .collect();
    let r = call("geometry.validity.make-valid", &json!({ "polygon": plain }));
    r["result"]["valid"].as_str().unwrap_or("?").to_owned()
}

#[test]
fn a_narrow_inlet_keeps_its_topology() {
    let shape = inlet();
    assert_eq!(valid(&shape), "yes");
    let plain = call(
        "geometry.simplify.rdp",
        &json!({"points": shape, "tolerance": "45 m", "shape": "polygon", "preserve_topology": "no"}),
    );
    assert_eq!(plain["result"]["restored"], 0.0, "{plain}");
    assert_eq!(
        valid(&plain["result"]["simplified"]),
        "no",
        "plain RDP should cut through the spike: {plain}"
    );
    let kept = call(
        "geometry.simplify.rdp",
        &json!({"points": shape, "tolerance": "45 m", "shape": "polygon"}),
    );
    assert!(
        kept["result"]["restored"].as_f64().unwrap() >= 1.0,
        "{kept}"
    );
    assert_eq!(valid(&kept["result"]["simplified"]), "yes", "{kept}");
    // The deviation is reported, and it is within the tolerance: the plain
    // run dropped the 40 m bow; the kept run put it back.
    let dev = |r: &Value| r["result"]["max_deviation"]["value"].as_f64().unwrap();
    assert!(dev(&plain) > 35.0 && dev(&plain) <= 45.0, "{}", dev(&plain));
    assert!(dev(&kept) <= 45.0, "{}", dev(&kept));
    // Visvalingam keeps it too.
    let vw = call(
        "geometry.simplify.visvalingam",
        &json!({"points": shape, "target_vertices": 8, "shape": "polygon"}),
    );
    assert_eq!(valid(&vw["result"]["simplified"]), "yes", "{vw}");
}

#[test]
fn every_removed_vertex_is_within_the_tolerance_on_the_ellipsoid() {
    // A 60 km zigzag: each removed vertex's geodesic distance to its new edge
    // is at most the tolerance, and tightening the tolerance keeps more.
    let pts: Vec<(f64, f64)> = (0..200)
        .map(|i| {
            let x = i as f64 * 300.0;
            (x, 120.0 * (x / 900.0).sin() + 40.0 * (x / 170.0).cos())
        })
        .collect();
    let line = ring(&pts);
    let mut last = 0.0;
    for tol in [100.0, 30.0, 5.0] {
        let r = call(
            "geometry.simplify.rdp",
            &json!({"points": line, "tolerance": format!("{tol} m")}),
        );
        let dev = r["result"]["max_deviation"]["value"].as_f64().unwrap();
        assert!(dev <= tol, "{tol}: {dev}");
        let n = r["result"]["vertices_out"].as_f64().unwrap();
        assert!(n > last, "{tol}: {n} after {last}");
        last = n;
    }
}

#[test]
fn a_target_count_is_met_and_bad_input_is_refused() {
    let pts: Vec<(f64, f64)> = (0..50)
        .map(|i| {
            (
                i as f64 * 100.0,
                if i % 2 == 0 { 0.0 } else { 30.0 + i as f64 },
            )
        })
        .collect();
    let r = call(
        "geometry.simplify.visvalingam",
        &json!({"points": ring(&pts), "target_vertices": 10}),
    );
    assert_eq!(r["result"]["vertices_out"], 10.0, "{r}");
    // The ends of a line stay put.
    let out = r["result"]["simplified"].as_array().unwrap();
    assert!((out[0]["lon"]["value"].as_f64().unwrap() + 105.0).abs() < 1e-12);
    // Both an area and a count, or neither, is refused.
    let both = call(
        "geometry.simplify.visvalingam",
        &json!({"points": ring(&pts), "target_vertices": 10, "area": "5 m2"}),
    );
    assert_eq!(both["error"]["code"], "INVALID_INPUT");
    // A bow tie has no topology to keep.
    let bow = ring(&[(0.0, 0.0), (100.0, 100.0), (100.0, 0.0), (0.0, 100.0)]);
    let r = call(
        "geometry.simplify.rdp",
        &json!({"points": bow, "tolerance": "1 m", "shape": "polygon"}),
    );
    assert_eq!(r["error"]["code"], "INVALID_INPUT", "{r}");
}

/// Douglas-Peucker on a line, with the deviation in metres.
fn rdp(points: &Value, tolerance: &str, topology: &str) -> Value {
    serde_json::from_str(
        &REGISTRY.invoke(
            "geometry.simplify.rdp",
            &json!({"points": points, "tolerance": tolerance, "shape": "line",
                "preserve_topology": topology,
                "options": {"outputUnits": {"max_deviation": "m"}}})
            .to_string(),
        ),
    )
    .expect("JSON")
}

/// A zigzag: it keeps every vertex until the tolerance passes its amplitude,
/// then collapses, so one shape covers several regimes.
fn zigzag() -> Value {
    let (mut lat, mut lon) = (40.0f64, -105.0f64);
    let mut pts = Vec::new();
    for i in 0..14 {
        pts.push(json!({"lat": lat, "lon": lon}));
        let b: f64 = 90.0 + if i % 2 == 0 { -35.0 } else { 35.0 };
        lat += 250.0 * b.to_radians().cos() / 111_320.0;
        lon += 250.0 * b.to_radians().sin() / (111_320.0 * lat.to_radians().cos());
    }
    Value::Array(pts)
}

#[test]
fn rdp_invariants() {
    let shape = zigzag();
    let n_in = shape.as_array().unwrap().len();
    // Matched numerically, not by formatting: the input and the echoed output
    // travel through different serialisation paths and need not print alike.
    let orig: Vec<(f64, f64)> = shape
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["lat"].as_f64().unwrap(), p["lon"].as_f64().unwrap()))
        .collect();
    let same =
        |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).abs() < 1e-12 && (a.1 - b.1).abs() < 1e-12;

    let mut last_kept = usize::MAX;
    for tol in [1.0, 5.0, 25.0, 100.0, 200.0, 400.0, 2000.0] {
        let r = rdp(&shape, &format!("{tol} m"), "no");
        assert!(r["ok"].as_bool().unwrap_or(false), "{tol} m: {r}");
        let kept = r["result"]["vertices_out"].as_u64().unwrap() as usize;

        // The promise the tool makes, and the only one that matters.
        let dev = r["result"]["max_deviation"]["value"].as_f64().unwrap();
        assert!(
            dev <= tol + 1e-9,
            "at {tol} m a vertex ended up {dev} m away"
        );

        // Vertices are dropped, never moved or invented: the result is a
        // subsequence of the input, checked vertex by vertex rather than by
        // counting, which a re-ordering would pass.
        let simplified: Vec<(f64, f64)> = r["result"]["simplified"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                (
                    p["lat"]["value"].as_f64().unwrap(),
                    p["lon"]["value"].as_f64().unwrap(),
                )
            })
            .collect();
        let mut j = 0;
        for s in &simplified {
            while j < orig.len() && !same(orig[j], *s) {
                j += 1;
            }
            assert!(
                j < orig.len(),
                "at {tol} m the result is not a subsequence of the input"
            );
            j += 1;
        }
        assert!(same(simplified[0], orig[0]), "the first point was dropped");
        assert!(
            same(simplified[simplified.len() - 1], orig[orig.len() - 1]),
            "the last point was dropped"
        );
        assert_eq!(
            simplified.len(),
            kept,
            "vertices_out is not the number returned"
        );

        // A looser tolerance can never keep more.
        assert!(
            kept <= last_kept,
            "raising the tolerance to {tol} m kept more vertices"
        );
        last_kept = kept;
        assert_eq!(r["result"]["vertices_in"].as_u64().unwrap() as usize, n_in);
    }

    // Zero is refused rather than read as "keep everything".
    let zero = rdp(&shape, "0 m", "no");
    assert_eq!(zero["error"]["code"], "INVALID_INPUT", "{zero}");
    assert_eq!(zero["error"]["field"], "/tolerance", "{zero}");

    // Two points have nothing to drop.
    let two = json!([{"lat":40.0,"lon":-105.0},{"lat":40.01,"lon":-104.99}]);
    let r = rdp(&two, "100000 m", "no");
    assert_eq!(r["result"]["vertices_out"].as_u64().unwrap(), 2, "{r}");

    // Keeping topology puts vertices back; it can never take more away.
    for tol in [100.0, 200.0, 400.0] {
        let plain = rdp(&shape, &format!("{tol} m"), "no");
        let kept_topology = rdp(&shape, &format!("{tol} m"), "yes");
        assert!(
            kept_topology["result"]["vertices_out"].as_u64().unwrap()
                >= plain["result"]["vertices_out"].as_u64().unwrap(),
            "at {tol} m preserving topology returned fewer vertices"
        );
    }
}
