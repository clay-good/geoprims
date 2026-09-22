//! Mission patterns: every spec scenario, geodesic spacing, holes, and the
//! image-count agreement on 20 polygons.

use geographiclib_rs::{Geodesic, InverseGeodesic};
use gp_drone::REGISTRY;
use gp_drone::mission::{self, Plane};
use serde_json::{Value, json};

fn call(id: &str, input: &Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).expect("envelope is JSON")
}

fn num(r: &Value, path: &str) -> f64 {
    path.split('.')
        .fold(r, |v, k| &v[k])
        .as_f64()
        .unwrap_or_else(|| panic!("{path} missing in {r}"))
}

/// A polygon from plane corners (m) around (40, -105).
fn area(corners: &[(f64, f64)], ring: u32) -> Vec<Value> {
    let p = Plane::new(40.0, -105.0);
    corners
        .iter()
        .map(|&(x, y)| {
            let (la, lo) = p.inv(x, y);
            json!({"lat": la, "lon": lo, "ring": ring})
        })
        .collect()
}

#[test]
fn auto_direction_minimizes_lines() {
    // A 2,000 m by 150 m rectangle rotated 30°.
    let (c, s) = (30f64.to_radians().cos(), 30f64.to_radians().sin());
    let rect: Vec<(f64, f64)> = [(0.0, 0.0), (2000.0, 0.0), (2000.0, 150.0), (0.0, 150.0)]
        .iter()
        .map(|&(x, y)| (x * c - y * s, x * s + y * c))
        .collect();
    let r = call(
        "drone.mission.survey-grid",
        &json!({"area": area(&rect, 0), "line_spacing": "52.5 m", "photo_spacing": "30 m"}),
    );
    assert_eq!(num(&r, "result.lines"), (150.0f64 / 52.5).ceil(), "{r}");
    // Lines parallel to the long axis: azimuth 60° (math angle 30°).
    assert!(
        (num(&r, "result.direction.value") - 60.0).abs() < 0.5,
        "{r}"
    );
}

#[test]
fn line_spacing_is_true_geodesically() {
    let sq = [
        (-500.0, -500.0),
        (500.0, -500.0),
        (500.0, 500.0),
        (-500.0, 500.0),
    ];
    let r = call(
        "drone.mission.survey-grid",
        &json!({"area": area(&sq, 0), "line_spacing": "100 m", "photo_spacing": "25 m", "direction": "0 deg"}),
    );
    let w = r["result"]["waypoints"].as_array().unwrap();
    let ll = |p: &Value| {
        (
            p["lat"]["value"].as_f64().unwrap(),
            p["lon"]["value"].as_f64().unwrap(),
        )
    };
    let lines: Vec<((f64, f64), (f64, f64))> = w
        .windows(2)
        .filter(|p| p[0]["kind"] == "line_start" && p[1]["kind"] == "line_end")
        .map(|p| (ll(&p[0]), ll(&p[1])))
        .collect();
    assert_eq!(lines.len(), 10);
    let g = Geodesic::wgs84();
    let d = |a: (f64, f64), b: (f64, f64)| -> f64 { g.inverse(a.0, a.1, b.0, b.1) };
    for k in 0..lines.len() - 1 {
        let (a, b) = lines[k];
        let (c, e) = lines[k + 1];
        let m = ((c.0 + e.0) / 2.0, (c.1 + e.1) / 2.0);
        // Height of the geodesic triangle (a, b, m) over base ab, by Heron's formula.
        let (ab, am, bm) = (d(a, b), d(a, m), d(b, m));
        let s2 = (ab + am + bm) / 2.0;
        let h = 2.0 * (s2 * (s2 - ab) * (s2 - am) * (s2 - bm)).sqrt() / ab;
        assert!((h - 100.0).abs() < 1e-3, "line {k}: spacing {h}");
    }
}

#[test]
fn polygon_with_a_hole() {
    let outer = [(0.0, 0.0), (800.0, 0.0), (800.0, 600.0), (0.0, 600.0)];
    let hole = [
        (300.0, 200.0),
        (500.0, 200.0),
        (500.0, 400.0),
        (300.0, 400.0),
    ];
    let mut a = area(&outer, 0);
    a.extend(area(&hole, 1));
    let r = call(
        "drone.mission.survey-grid",
        &json!({"area": a, "line_spacing": "40 m", "photo_spacing": "20 m", "direction": "90 deg", "hole_buffer": "10 m"}),
    );
    let p = Plane::new(40.0, -105.0);
    let pts: Vec<(f64, f64)> = r["result"]["waypoints"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| {
            p.fwd(
                w["lat"]["value"].as_f64().unwrap(),
                w["lon"]["value"].as_f64().unwrap(),
            )
        })
        .collect();
    // No point of the path comes within 9.9 m of the hole (the buffer is 10 m).
    let inside = |x: f64, y: f64| {
        let dx = (300.0 - x).max(0.0).max(x - 500.0);
        let dy = (200.0 - y).max(0.0).max(y - 400.0);
        dx.hypot(dy) < 9.9
    };
    for seg in pts.windows(2) {
        for k in 0..=100 {
            let t = k as f64 / 100.0;
            let (x, y) = (
                seg[0].0 + t * (seg[1].0 - seg[0].0),
                seg[0].1 + t * (seg[1].1 - seg[0].1),
            );
            assert!(
                !inside(x, y),
                "segment {seg:?} crosses the hole at ({x}, {y})"
            );
        }
    }
    assert!(
        r["result"]["waypoints"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["kind"] == "transit")
    );
}

#[test]
fn image_count_matches_the_pattern() {
    let mut seed = 7u64;
    let mut rnd = || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    for _ in 0..20 {
        // A random star-shaped polygon of 5 to 12 corners, 200 m to 1.5 km across.
        let n = 5 + (rnd() * 8.0) as usize;
        let rad = 200.0 + rnd() * 1300.0;
        let corners: Vec<(f64, f64)> = (0..n)
            .map(|k| {
                let a = k as f64 / n as f64 * std::f64::consts::TAU;
                let r = rad * (0.6 + 0.4 * rnd());
                (r * a.cos(), r * a.sin())
            })
            .collect();
        let input =
            json!({"area": area(&corners, 0), "line_spacing": "45 m", "photo_spacing": "25 m"});
        let est = call("drone.photogrammetry.image-count", &input);
        let grid = call("drone.mission.survey-grid", &input);
        let (a, b) = (num(&est, "result.photos"), num(&grid, "result.photos"));
        assert!((a - b).abs() <= 0.02 * b, "estimate {a} vs pattern {b}");
    }
}

#[test]
fn pipeline_corridor() {
    // About 5 km of centerline.
    let r = call(
        "drone.mission.corridor",
        &json!({"centerline": [{"lat": 40.0, "lon": -105.0}, {"lat": 40.03, "lon": -104.98}, {"lat": 40.035, "lon": -104.95}], "width": "120 m", "line_spacing": "52.5 m"}),
    );
    assert_eq!(num(&r, "result.line_count"), 3.0, "{r}");
    let offs: Vec<f64> = r["result"]["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["offset"]["value"].as_f64().unwrap())
        .collect();
    assert_eq!(offs, [-52.5, 0.0, 52.5]);
}

#[test]
fn gimbal_pitch_for_a_tower_top() {
    let r = call(
        "drone.mission.orbit",
        &json!({"lat": 40.0, "lon": -105.0, "radius": "50 m", "height": "80 m", "target_height": "60 m", "photos": 12}),
    );
    assert!(
        (num(&r, "result.gimbal_pitch.value") - (-(20.0f64 / 50.0).atan().to_degrees())).abs()
            < 1e-12,
        "{r}"
    );
    let w = r["result"]["waypoints"].as_array().unwrap();
    assert_eq!(w.len(), 12);
    // The heading points at the center: from the east waypoint, heading ≈ 270°.
    let east = &w[3];
    assert!(
        (east["heading"]["value"].as_f64().unwrap() - 270.0).abs() < 0.01,
        "{east}"
    );
    assert!(
        (num(&r, "result.circumference.value") - 2.0 * std::f64::consts::PI * 50.0).abs() < 0.01
    );
}

#[test]
fn hull_and_width() {
    let pts = [(0.0, 0.0), (10.0, 0.0), (10.0, 1.0), (0.0, 1.0), (5.0, 0.5)];
    assert_eq!(mission::hull(&pts).len(), 4);
    assert!(
        mission::min_width_angle(&pts).abs() < 1e-12
            || (mission::min_width_angle(&pts).abs() - std::f64::consts::PI).abs() < 1e-12
    );
}

#[test]
fn facade_gsd_uses_the_standoff() {
    let r = call(
        "drone.mission.facade",
        &serde_json::json!({"facade":[{"lat":40.4406,"lon":-80.0020},{"lat":40.4406,"lon":-80.001411}],
            "standoff":"30 m","top_height":"25 m","sensor_width":"13.2 mm","focal_length":"8.8 mm",
            "image_width":5472,"sensor_height":"8.8 mm"}),
    );
    let gsd = r["result"]["gsd"]["value"].as_f64().unwrap();
    assert!(
        (gsd - 13.2 / 5472.0 * 30.0 / 8.8 * 100.0).abs() < 1e-9,
        "{gsd}"
    );
    // A 45 m photo height covers a 25 m wall in one pass; passes alternate direction.
    assert_eq!(r["result"]["passes"], 1.0);
    let wps = r["result"]["waypoints"].as_array().unwrap();
    assert_eq!(wps.len() as f64, r["result"]["photos"].as_f64().unwrap());
    // Flying on the right of a wall that runs east puts the drone south, facing north.
    let lat = wps[0]["lat"]["value"].as_f64().unwrap();
    assert!(lat < 40.4406);
    let hd = wps[0]["heading"]["value"].as_f64().unwrap();
    assert!(hd.min(360.0 - hd) < 0.01, "{hd}");
}
