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

#[test]
fn waypoint_outside_the_fence_is_flagged_with_index_and_distance() {
    // A survey grid over a field, one waypoint 12 m past a 50 m fence.
    let field = json!([{"lat":40.0,"lon":-105.0},{"lat":40.0,"lon":-104.998},{"lat":40.0015,"lon":-104.998},{"lat":40.0015,"lon":-105.0}]);
    let grid = call(
        "drone.mission.survey-grid",
        &json!({"area": field, "line_spacing":"30 m", "photo_spacing":"20 m"}),
    );
    let mut wps: Vec<Value> = grid["result"]["waypoints"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| json!({"lat": w["lat"]["value"], "lon": w["lon"]["value"]}))
        .collect();
    assert!(!wps.is_empty(), "{grid}");
    // 62 m due south of the field's south edge (40.0°), along the meridian.
    let g = Geodesic::wgs84();
    let (lat, _, _): (f64, f64, f64) =
        geographiclib_rs::DirectGeodesic::direct(&g, 40.0, -104.999, 180.0, 62.0);
    wps.push(json!({"lat": lat, "lon": -104.999}));
    let n = wps.len();
    let r = call(
        "drone.mission.geofence",
        &json!({"area": field, "distance":"50 m", "waypoints": wps}),
    );
    assert_eq!(
        r["result"]["outside_count"], 1.0,
        "{}",
        r["result"]["flagged"]
    );
    let f = &r["result"]["flagged"][0];
    assert_eq!(f["waypoint"].as_f64().unwrap() as usize, n);
    assert!(
        (f["beyond"]["value"].as_f64().unwrap() - 12.0).abs() < 0.01,
        "{f}"
    );
    let codes: Vec<&str> = r["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap())
        .collect();
    assert!(codes.contains(&"WAYPOINT_OUTSIDE_GEOFENCE"));
}

fn export(format: &str, reference: &str, extra: Value) -> Value {
    let mut input = json!({
        "waypoints": [
            {"lat":40.4406,"lon":-80.002,"height":"80 m","heading":90,"gimbal_pitch":-90,"action":"photo, then \"hover\" <5 s>"},
            {"lat":40.4406,"lon":-80.0005,"height":"80 m"},
            {"lat":40.4412,"lon":-80.0005,"height":"95 m","heading":270}
        ],
        "height_reference": reference, "format": format, "name": "North & South"
    });
    for (k, v) in extra.as_object().unwrap() {
        input[k] = v.clone();
    }
    let r = call("drone.mission.export", &input);
    assert_eq!(r["ok"], true, "{r}");
    r
}

/// Every opened tag closes in order (and the declaration and comment are skipped).
fn well_formed(xml: &str) -> bool {
    let mut stack: Vec<&str> = Vec::new();
    let mut rest = xml;
    while let Some(i) = rest.find('<') {
        let j = rest[i..].find('>').map(|j| i + j).unwrap();
        let tag = &rest[i + 1..j];
        rest = &rest[j + 1..];
        if tag.starts_with('?') || tag.starts_with('!') {
            continue;
        }
        if let Some(name) = tag.strip_prefix('/') {
            if stack.pop() != Some(name) {
                return false;
            }
        } else if !tag.ends_with('/') {
            stack.push(tag.split_whitespace().next().unwrap());
        }
    }
    stack.is_empty()
}

#[test]
fn kml_altitude_mode_follows_the_height_reference() {
    let agl = export("kml", "agl", json!({}));
    let file = agl["result"]["file"].as_str().unwrap();
    assert!(well_formed(file), "{file}");
    assert_eq!(
        file.matches("<altitudeMode>relativeToGround</altitudeMode>")
            .count(),
        4
    );
    assert!(file.contains("<!-- geoprims drone.mission.export"));
    assert!(file.contains("not for navigation"));
    assert!(file.contains("North &amp; South"));
    assert!(file.contains("&quot;hover&quot; &lt;5 s&gt;"));
    assert!(file.contains("-80.002,40.4406,80 "));
    // Above takeoff and HAE become absolute, shifted to sea level.
    let takeoff = export("kml", "takeoff", json!({"takeoff_elevation":"312 m"}));
    let f = takeoff["result"]["file"].as_str().unwrap();
    assert!(
        f.contains("<altitudeMode>absolute</altitudeMode>") && f.contains("-80.0005,40.4412,407"),
        "{f}"
    );
    let hae = export("kml", "hae", json!({"geoid_height":"-33.9 m"}));
    let f = hae["result"]["file"].as_str().unwrap();
    assert!(f.contains("-80.002,40.4406,113.9"), "{f}");
}

#[test]
fn geojson_and_csv_carry_every_waypoint() {
    let g = export("geojson", "agl", json!({}));
    let doc: Value = serde_json::from_str(g["result"]["file"].as_str().unwrap()).unwrap();
    assert_eq!(doc["type"], "FeatureCollection");
    assert_eq!(doc["geoprims"]["height_reference"], "AGL");
    let feats = doc["features"].as_array().unwrap();
    assert_eq!(feats.len(), 4);
    assert_eq!(feats[0]["geometry"]["type"], "LineString");
    // RFC 7946: longitude, latitude, then height.
    assert_eq!(
        feats[1]["geometry"]["coordinates"],
        json!([-80.002, 40.4406, 80])
    );
    assert_eq!(feats[1]["properties"]["gimbal_pitch"], -90.0);
    assert!(feats[2]["properties"].get("heading").is_none());
    let c = export("csv", "msl", json!({}));
    let text = c["result"]["file"].as_str().unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines[0].starts_with("# geoprims drone.mission.export"));
    assert_eq!(
        lines[2],
        "index,lat,lon,height_m,reference,heading_deg,gimbal_pitch_deg,action"
    );
    assert_eq!(
        lines[3],
        "1,40.4406,-80.002,80,MSL,90,-90,\"photo, then \"\"hover\"\" <5 s>\""
    );
    assert_eq!(lines[4], "2,40.4406,-80.0005,80,MSL,,,");
    assert_eq!(lines.len(), 6);
}

#[test]
fn a_huge_focal_length_is_a_limit_not_a_crash() {
    // Found by the fuzzer: a 1e9 mm lens asked for billions of photo stations.
    let r = call(
        "drone.mission.facade",
        &json!({"facade":[{"lat":40.4406,"lon":-80.002},{"lat":40.4406,"lon":-80.001411}],"focal_length":"1000000000 mm","sensor_height":"8.8 mm","sensor_width":"13.2 mm","standoff":"30 m","top_height":"25 m"}),
    );
    assert_eq!(r["error"]["code"], "LIMIT_EXCEEDED", "{r}");
}

#[test]
fn trigger_points_sit_on_the_lines_at_the_photo_spacing() {
    // add-drone-suite 2.8: every photo the plan counts has a place on the map.
    let g = Geodesic::wgs84();
    let rect = [(0.0, 0.0), (600.0, 0.0), (600.0, 150.0), (0.0, 150.0)];
    let r = call(
        "drone.mission.survey-grid",
        &json!({"area": area(&rect, 0), "line_spacing": "52.5 m", "photo_spacing": "30 m"}),
    );
    let shots = r["result"]["photo_points"]
        .as_array()
        .expect("trigger points");
    assert_eq!(
        shots.len() as f64,
        num(&r, "result.photos"),
        "one point per photo the plan counts"
    );
    let at = |i: usize| {
        (
            shots[i]["lat"]["value"].as_f64().unwrap(),
            shots[i]["lon"]["value"].as_f64().unwrap(),
        )
    };
    // Numbered in flight order, and spaced along each line at the photo
    // spacing; the jumps between lines are the turns, which are longer.
    for (i, s) in shots.iter().enumerate() {
        assert_eq!(
            s["photo"].as_f64().unwrap(),
            (i + 1) as f64,
            "photo numbering"
        );
    }
    let mut along = 0;
    for i in 1..shots.len() {
        let (a, b) = (at(i - 1), at(i));
        let (d, _, _, _): (f64, f64, f64, f64) = g.inverse(a.0, a.1, b.0, b.1);
        if d < 40.0 {
            assert!(
                (d - 30.0).abs() < 0.1,
                "step {i} is {d} m, not the 30 m spacing"
            );
            along += 1;
        }
    }
    assert!(
        along >= shots.len() - 6,
        "most steps are along a line: {along} of {}",
        shots.len() - 1
    );
    // Each point lies on the swept area, not outside it.
    let waypoints = r["result"]["waypoints"].as_array().unwrap();
    let ends: Vec<(f64, f64)> = waypoints
        .iter()
        .map(|w| {
            (
                w["lat"]["value"].as_f64().unwrap(),
                w["lon"]["value"].as_f64().unwrap(),
            )
        })
        .collect();
    let first = at(0);
    let (d0, _, _, _): (f64, f64, f64, f64) = g.inverse(first.0, first.1, ends[0].0, ends[0].1);
    assert!(
        d0 < 1e-6,
        "the first photo is at the first line's start: {d0} m"
    );
}
