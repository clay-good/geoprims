//! Sampling layouts (add-sampling-and-grids).

use gp_geometry::REGISTRY;
use serde_json::{Value, json};

fn call(id: &str, input: &Value) -> Value {
    let r: Value = serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).unwrap();
    assert_eq!(r["ok"], true, "{id}: {r}");
    r
}

fn points(r: &Value) -> Vec<(f64, f64)> {
    r["result"]["points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["lat"]["value"].as_f64().unwrap(),
                p["lon"]["value"].as_f64().unwrap(),
            )
        })
        .collect()
}

/// Ray casting in (lon, lat); a point within `tol` degrees of a side counts as on it.
fn inside(ring: &[(f64, f64)], p: (f64, f64), tol: f64) -> Option<bool> {
    let (y, x) = p;
    let mut c = false;
    for i in 0..ring.len() {
        let (ay, ax) = ring[i];
        let (by, bx) = ring[(i + 1) % ring.len()];
        let (dx, dy) = (bx - ax, by - ay);
        let t = (((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
        if (x - ax - t * dx).hypot(y - ay - t * dy) < tol {
            return None;
        }
        if (ay > y) != (by > y) && x < ax + (y - ay) * dx / dy {
            c = !c;
        }
    }
    Some(c)
}

fn rows(rings: &[&[(f64, f64)]]) -> Vec<Value> {
    rings
        .iter()
        .enumerate()
        .flat_map(|(k, r)| {
            r.iter()
                .map(move |&(lat, lon)| json!({"lat": lat, "lon": lon, "ring": k}))
        })
        .collect()
}

#[test]
fn random_points_invariants() {
    // An L-shaped field with a hole, 10,000 points: all inside, none in the hole.
    let outline = [
        (40.0, -105.0),
        (40.0, -104.99),
        (40.004, -104.99),
        (40.004, -104.996),
        (40.01, -104.996),
        (40.01, -105.0),
    ];
    let hole = [
        (40.001, -104.999),
        (40.003, -104.999),
        (40.003, -104.997),
        (40.001, -104.997),
    ];
    let r = call(
        "geometry.sampling.random-points",
        &json!({"polygon": rows(&[&outline, &hole]), "count": 10_000, "seed": 7}),
    );
    let pts = points(&r);
    assert_eq!(pts.len(), 10_000);
    for (i, &p) in pts.iter().enumerate() {
        // 1e-7° is about 1 cm: the plane's straight sides and these map
        // sides differ by far less, so a point nearer than that is not judged.
        assert_ne!(
            inside(&outline, p, 1e-7),
            Some(false),
            "point {i} {p:?} is outside"
        );
        assert_ne!(
            inside(&hole, p, 1e-7),
            Some(true),
            "point {i} {p:?} is in the hole"
        );
    }
    // The same seed gives the same points; another seed does not.
    let again = call(
        "geometry.sampling.random-points",
        &json!({"polygon": rows(&[&outline, &hole]), "count": 10_000, "seed": "7"}),
    );
    assert_eq!(r["result"]["points"], again["result"]["points"]);
    let other = call(
        "geometry.sampling.random-points",
        &json!({"polygon": rows(&[&outline, &hole]), "count": 10, "seed": 8}),
    );
    assert_ne!(points(&other)[0], pts[0]);
    assert_eq!(r["result"]["seed"], "7");

    // Two equal halves of a square get half the points each.
    let square = [
        (40.0, -105.0),
        (40.0, -104.99),
        (40.01, -104.99),
        (40.01, -105.0),
    ];
    let r = call(
        "geometry.sampling.random-points",
        &json!({"polygon": rows(&[&square]), "count": 100_000, "seed": 1}),
    );
    let west = points(&r).iter().filter(|p| p.1 < -104.995).count();
    assert!(
        (49_000..=51_000).contains(&west),
        "{west} of 100,000 in the west half"
    );
}

#[test]
fn random_points_refusals() {
    let invoke = |input: Value| -> Value {
        serde_json::from_str(
            &REGISTRY.invoke("geometry.sampling.random-points", &input.to_string()),
        )
        .unwrap()
    };
    // Over 500 km across.
    let big = invoke(
        json!({"polygon": [{"lat": 30, "lon": -100}, {"lat": 30, "lon": -94}, {"lat": 35, "lon": -94}], "count": 5}),
    );
    assert_eq!(big["error"]["code"], "LIMIT_EXCEEDED", "{big}");
    let bad_seed = invoke(
        json!({"polygon": [{"lat": 40, "lon": -105}, {"lat": 40, "lon": -104.99}, {"lat": 40.01, "lon": -105}], "count": 5, "seed": "-1"}),
    );
    assert_eq!(bad_seed["error"]["field"], "/seed", "{bad_seed}");
    let max = invoke(
        json!({"polygon": [{"lat": 40, "lon": -105}, {"lat": 40, "lon": -104.99}, {"lat": 40.01, "lon": -105}], "count": 5, "seed": "18446744073709551615"}),
    );
    assert_eq!(max["ok"], true, "{max}");
}
