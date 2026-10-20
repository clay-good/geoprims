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

fn geodesic(a: (f64, f64), b: (f64, f64)) -> f64 {
    use geographiclib_rs::{Geodesic, InverseGeodesic};
    let s: f64 = Geodesic::wgs84().inverse(a.0, a.1, b.0, b.1);
    s
}

#[test]
fn systematic_points_scenarios() {
    let field = [
        (40.0, -105.0),
        (40.0, -104.996),
        (40.003, -104.996),
        (40.003, -105.0),
    ];
    for (pattern, k) in [("triangular", 6), ("square", 4)] {
        let r = call(
            "geometry.sampling.systematic-points",
            &json!({"polygon": rows(&[&field]), "spacing": "50 m", "pattern": pattern, "seed": 3}),
        );
        let pts = points(&r);
        assert!(pts.len() > 20, "{pattern}: {}", pts.len());
        let mut interior = 0;
        for &p in &pts {
            let mut d: Vec<f64> = pts
                .iter()
                .filter(|&&q| q != p)
                .map(|&q| geodesic(p, q))
                .collect();
            d.sort_by(f64::total_cmp);
            // An interior point has all k neighbors at the spacing; one by the
            // boundary has fewer within 51 m.
            if d.len() >= k && d[k - 1] < 51.0 {
                interior += 1;
                for &x in &d[..k] {
                    assert!((x - 50.0).abs() < 1e-3, "{pattern}: neighbor at {x} m");
                }
            }
        }
        assert!(interior > 10, "{pattern}: only {interior} interior points");
    }
    // A corner start ignores the seed; a random start does not.
    let corner = |seed: u64| {
        call(
            "geometry.sampling.systematic-points",
            &json!({"polygon": rows(&[&field]), "spacing": "40 m", "start": "corner", "seed": seed}),
        )
    };
    assert_eq!(corner(1)["result"]["points"], corner(2)["result"]["points"]);
    let random = |seed: u64| {
        call(
            "geometry.sampling.systematic-points",
            &json!({"polygon": rows(&[&field]), "spacing": "40 m", "seed": seed}),
        )
    };
    assert_ne!(random(1)["result"]["points"], random(2)["result"]["points"]);
    assert_eq!(random(5)["result"]["points"], random(5)["result"]["points"]);
    // A rotated grid keeps its spacing.
    let r = call(
        "geometry.sampling.systematic-points",
        &json!({"polygon": rows(&[&field]), "spacing": "50 m", "rotation": "30 deg", "seed": 3}),
    );
    let pts = points(&r);
    let nearest = pts
        .iter()
        .skip(1)
        .map(|&q| geodesic(pts[0], q))
        .fold(f64::INFINITY, f64::min);
    assert!((nearest - 50.0).abs() < 1e-3, "rotated nearest {nearest}");

    // A 5 m grid over about 100 km² is refused before any point is made.
    let big = [
        (40.0, -105.0),
        (40.0, -104.8826),
        (40.09, -104.8826),
        (40.09, -105.0),
    ];
    let r: Value = serde_json::from_str(&REGISTRY.invoke(
        "geometry.sampling.systematic-points",
        &json!({"polygon": rows(&[&big]), "spacing": "5 m"}).to_string(),
    ))
    .unwrap();
    assert_eq!(r["error"]["code"], "LIMIT_EXCEEDED", "{r}");
    let msg = r["error"]["message"].as_str().unwrap();
    let n: f64 = msg
        .split_whitespace()
        .nth(4)
        .unwrap()
        .replace(',', "")
        .parse()
        .unwrap();
    assert!((3.9e6..4.1e6).contains(&n), "{msg}");
    assert!(msg.contains("100,000"), "{msg}");
}

#[test]
fn stratified_points_scenarios() {
    use gp_geo::utmups::utm_forward;
    let field = [
        (40.0, -105.0),
        (40.0, -104.996),
        (40.003, -104.996),
        (40.003, -105.0),
    ];
    let r = call(
        "geometry.sampling.stratified-points",
        &json!({"polygon": rows(&[&field]), "cell": "100 m", "per_stratum": 3, "seed": 9}),
    );
    let strata = r["result"]["strata"].as_f64().unwrap() as usize;
    assert_eq!(points(&r).len(), 3 * strata);
    // Each stratum label appears exactly per_stratum times.
    let labels: Vec<&str> = r["result"]["points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["stratum"].as_str().unwrap())
        .collect();
    for l in &labels {
        assert_eq!(labels.iter().filter(|m| *m == l).count(), 3, "{l}");
    }
    // A cell side that leaves a 2 mm sliver on the east: that column's cells
    // hold under 1 m² of the field and are skipped.
    let g = geographiclib_rs::Geodesic::wgs84();
    let fw: Vec<_> = field
        .iter()
        .map(|&(lat, lon)| utm_forward(g.a, g.f, lat, lon, 13))
        .collect();
    let w = fw
        .iter()
        .map(|p| p.easting)
        .fold(f64::NEG_INFINITY, f64::max)
        - fw.iter().map(|p| p.easting).fold(f64::INFINITY, f64::min);
    let center = utm_forward(g.a, g.f, 40.0015, -104.998, 13).scale;
    let side = (w - 0.002) / 2.0 / center;
    let r = call(
        "geometry.sampling.stratified-points",
        &json!({"polygon": rows(&[&field]), "cell": format!("{side} m"), "seed": 9}),
    );
    let skipped = r["result"]["skipped"].as_str().unwrap();
    assert!(skipped.contains("c3"), "skipped: {skipped}");
    assert!(
        !r["result"]["points"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["stratum"].as_str().unwrap().ends_with("c3"))
    );
}
