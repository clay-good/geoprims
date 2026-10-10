//! Mission patterns: every spec scenario, geodesic spacing, holes, the
//! image-count agreement on 20 polygons, and the image-count invariants.

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
    // Published flight planning: the outer lines on the edges, ⌈width / spacing⌉ + 1.
    assert_eq!(
        num(&r, "result.lines"),
        (150.0f64 / 52.5).ceil() + 1.0,
        "{r}"
    );
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
    // 1,000 m at 100 m: 10 spacings, so 11 lines, the outer two on the edges.
    assert_eq!(lines.len(), 11);
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

#[test]
fn image_count_invariants() {
    // Plane rectangles (m) around (40, -105), lines along x (direction 90°).
    let rect = |w: f64, l: f64| area(&[(0.0, 0.0), (l, 0.0), (l, w), (0.0, w)], 0);
    let run = |id: &str, w: f64, l: f64, s: f64, p: f64, extra: Option<u32>, over: f64| {
        let mut input = json!({"area": rect(w, l), "line_spacing": format!("{s} m"),
            "photo_spacing": format!("{p} m"), "direction": "90 deg", "overshoot": format!("{over} m")});
        if let Some(e) = extra {
            input["end_photos"] = json!(e);
        }
        let r = call(id, &input);
        assert_eq!(r["ok"], true, "{r}");
        r
    };
    let ic = "drone.photogrammetry.image-count";
    let sg = "drone.mission.survey-grid";
    let blocks: [(f64, f64, f64, f64); 4] = [
        (150.0, 600.0, 52.5, 30.0),
        (333.0, 1210.0, 40.0, 17.0),
        (95.0, 480.0, 100.0, 45.0),
        (1000.0, 2000.0, 70.0, 26.0),
    ];
    for (w, l, s, p) in blocks {
        // The published count, closed form on a rectangle.
        let lines = (w / s - 1e-4).ceil() + 1.0;
        let per = (l / p - 1e-4).ceil() + 1.0;
        for e in 0..=3u32 {
            let r = run(ic, w, l, s, p, Some(e), 0.0);
            assert_eq!(num(&r, "result.lines"), lines, "{r}");
            assert_eq!(
                num(&r, "result.photos"),
                lines * (per + 2.0 * e as f64),
                "{r}"
            );
            // Every line crosses the whole length (to 0.01%: the block's ends
            // are tilted by the meridian convergence between the two planes).
            assert!(
                (num(&r, "result.survey_length.value") * 1000.0 - lines * l).abs()
                    < 1e-4 * lines * l
            );
            // The survey grid flies the same sweep, photo for photo.
            let g = run(sg, w, l, s, p, Some(e), 0.0);
            assert_eq!(num(&g, "result.photos"), num(&r, "result.photos"));
            assert_eq!(num(&g, "result.lines"), lines);
            assert_eq!(
                g["result"]["photo_points"].as_array().unwrap().len() as f64,
                num(&g, "result.photos")
            );
        }
        // The default is the published two photos past each end.
        assert_eq!(
            num(&run(ic, w, l, s, p, None, 0.0), "result.photos"),
            num(&run(ic, w, l, s, p, Some(2), 0.0), "result.photos")
        );
        // Closer lines or photos never mean fewer of them.
        let base = run(ic, w, l, s, p, Some(2), 0.0);
        let dense = run(ic, w, l, s * 0.7, p * 0.7, Some(2), 0.0);
        assert!(num(&dense, "result.lines") >= num(&base, "result.lines"));
        assert!(num(&dense, "result.photos") >= num(&base, "result.photos"));
        // Overshoot moves the turns, never the photos; the path only grows.
        let step = l / (per - 1.0);
        let short = run(ic, w, l, s, p, Some(2), 1.5 * step);
        let long = run(ic, w, l, s, p, Some(2), 3.0 * step);
        assert_eq!(num(&short, "result.photos"), num(&base, "result.photos"));
        assert_eq!(num(&long, "result.photos"), num(&base, "result.photos"));
        assert!(
            (num(&short, "result.path_length.value") - num(&base, "result.path_length.value"))
                .abs()
                < 1e-9,
            "an overshoot shorter than the extra photos adds nothing"
        );
        assert!(num(&long, "result.path_length.value") > num(&base, "result.path_length.value"));
    }
}

/// The primary examples of image count and the survey grid name the field by
/// its size. The corners span 602 m by 150 m (geodesic), not 600 m, and the
/// titles say so: 4 lines of 602 m give ⌈602 / 30⌉ + 1 + 4 = 26 photos each.
#[test]
fn field_example_titles_match_their_geometry() {
    let g = Geodesic::wgs84();
    let long: f64 = g.inverse(40.0, -105.0, 40.0, -104.99295);
    let short: f64 = g.inverse(40.0, -105.0, 40.00135, -105.0);
    assert_eq!((long.round(), short.round()), (602.0, 150.0));
    for id in [
        "drone.photogrammetry.image-count",
        "drone.mission.survey-grid",
    ] {
        let t = REGISTRY.find(id).expect("tool");
        let ex = t
            .examples
            .iter()
            .find(|e| e.id == "primary")
            .expect("primary");
        assert!(ex.input.contains("-104.99295"), "{id} example moved");
        assert!(ex.title.contains("602 m by 150 m"), "{id}: {}", ex.title);
    }
}

#[test]
fn photo_footprints_have_the_camera_size_and_follow_the_lines() {
    // add-drone-suite 2.8, the coverage-overlay scenario: each photo's ground
    // rectangle, its sides measured geodesically, along the line it was taken on.
    let g = Geodesic::wgs84();
    let rect = area(&[(0.0, 0.0), (600.0, 0.0), (600.0, 300.0), (0.0, 300.0)], 0);
    for (crosshatch, direction) in [("no", "0 deg"), ("no", "35 deg"), ("yes", "0 deg")] {
        let r = call(
            "drone.mission.survey-grid",
            &json!({"area": rect, "line_spacing": "50 m", "photo_spacing": "30 m",
                "direction": direction, "crosshatch": crosshatch,
                "footprint_across": "75 m", "footprint_along": "50 m"}),
        );
        assert_eq!(r["ok"], true, "{r}");
        let res = &r["result"];
        assert!((num(res, "forward_overlap") - 40.0).abs() < 1e-9);
        assert!((num(res, "side_overlap") - 100.0 / 3.0).abs() < 1e-9);
        let corners = res["footprints"].as_array().unwrap();
        let photos = res["photos"].as_f64().unwrap() as usize;
        assert_eq!(corners.len(), 4 * photos.min(2_000));
        let base = direction.trim_end_matches(" deg").parse::<f64>().unwrap();
        let (mut on_base, mut turned) = (0, 0);
        for (k, quad) in corners.chunks(4).enumerate() {
            let pt = |i: usize| {
                (
                    quad[i]["lat"]["value"].as_f64().unwrap(),
                    quad[i]["lon"]["value"].as_f64().unwrap(),
                )
            };
            let side = |i: usize, j: usize| -> (f64, f64) {
                let ((a, b), (c, d)) = (pt(i), pt(j));
                let (s, az, _, _) = g.inverse(a, b, c, d);
                (s, az)
            };
            // Corners go +along +across, +along −across, −along −across, −along +across.
            let (across, _) = side(0, 1);
            let (along, az) = side(2, 1);
            assert!((across - 75.0).abs() < 1e-3, "{across}");
            assert!((along - 50.0).abs() < 1e-3, "{along}");
            // Along the grid's azimuth, or for a crosshatch's second set a
            // quarter turn from it.
            let off = ((az - base).rem_euclid(180.0) + 90.0).rem_euclid(180.0) - 90.0;
            if off.abs() < 0.01 {
                on_base += 1;
            } else {
                assert!(
                    crosshatch == "yes" && (off.abs() - 90.0).abs() < 0.01,
                    "photo {k}: {az} against {base}"
                );
                turned += 1;
            }
            assert_eq!(quad[0]["part"].as_f64(), Some((k + 1) as f64));
        }
        assert!(on_base > 0);
        assert_eq!(turned > 0, crosshatch == "yes");
    }
}

#[test]
fn a_footprint_smaller_than_the_spacing_leaves_gaps() {
    let rect = area(&[(0.0, 0.0), (400.0, 0.0), (400.0, 200.0), (0.0, 200.0)], 0);
    let r = call(
        "drone.mission.survey-grid",
        &json!({"area": rect, "line_spacing": "60 m", "photo_spacing": "30 m",
            "footprint_across": "50 m", "footprint_along": "40 m"}),
    );
    assert_eq!(r["ok"], true, "{r}");
    assert!(num(&r["result"], "side_overlap") < 0.0);
    assert!(
        r["meta"]["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["code"] == "COVERAGE_GAP")
    );
    // One without the other is refused, and without either nothing changes.
    let half = call(
        "drone.mission.survey-grid",
        &json!({"area": rect, "line_spacing": "60 m", "photo_spacing": "30 m", "footprint_across": "50 m"}),
    );
    assert_eq!(half["error"]["field"], "/footprint_along");
    let none = call(
        "drone.mission.survey-grid",
        &json!({"area": rect, "line_spacing": "60 m", "photo_spacing": "30 m"}),
    );
    assert!(none["result"].get("footprints").is_none());
}

#[test]
fn orbit_invariants() {
    // Every waypoint stands the radius from the center, measured back with the
    // geodesic inverse; its heading is the way to the center; the waypoints
    // are evenly spaced; counterclockwise is the same ring in reverse; and the
    // circumference is the photo spacing times the count.
    let g = Geodesic::wgs84();
    for (lat, lon, radius, photos) in [
        (40.0, -105.0, 50.0, 12),
        (-33.86, 151.21, 250.0, 36),
        (70.0, 25.0, 30.0, 8),
    ] {
        let run = |extra: &str| {
            call(
                "drone.mission.orbit",
                &json!({"lat": lat, "lon": lon, "radius": format!("{radius} m"), "height": "80 m", "photos": photos, "rotation": extra}),
            )
        };
        let cw = run("clockwise");
        let w = cw["result"]["waypoints"].as_array().unwrap();
        assert_eq!(w.len(), photos);
        let at = |p: &Value| {
            (
                p["lat"]["value"].as_f64().unwrap(),
                p["lon"]["value"].as_f64().unwrap(),
            )
        };
        let mut steps = Vec::new();
        for (k, p) in w.iter().enumerate() {
            let (la, lo) = at(p);
            let (d, to_center, _, _): (f64, f64, f64, f64) = g.inverse(la, lo, lat, lon);
            assert!((d - radius).abs() < 1e-6, "waypoint {k} is {d} m out");
            let heading = p["heading"]["value"].as_f64().unwrap();
            assert!(
                ((heading - to_center + 540.0).rem_euclid(360.0) - 180.0).abs() < 1e-6,
                "waypoint {k}"
            );
            let (nla, nlo) = at(&w[(k + 1) % photos]);
            let step: f64 = g.inverse(la, lo, nla, nlo);
            steps.push(step);
        }
        let (lo_step, hi_step) = steps
            .iter()
            .fold((f64::MAX, 0.0_f64), |(a, b), s| (a.min(*s), b.max(*s)));
        assert!(
            hi_step - lo_step < 1e-3 * radius,
            "uneven: {lo_step} to {hi_step}"
        );
        let ccw = run("counterclockwise");
        let v = ccw["result"]["waypoints"].as_array().unwrap();
        for k in 1..photos {
            let ((a, b), (c, d)) = (at(&w[k]), at(&v[photos - k]));
            assert!(
                (a - c).abs() < 1e-9 && (b - d).abs() < 1e-9,
                "waypoint {k} mirrored"
            );
        }
        let circumference = num(&cw, "result.circumference.value");
        assert!(
            (circumference - num(&cw, "result.photo_spacing.value") * photos as f64).abs() < 1e-9
        );
        assert!((circumference / (2.0 * std::f64::consts::PI * radius) - 1.0).abs() < 1e-6);
    }
}

#[test]
fn geofence_invariants() {
    // A waypoint placed a known geodesic distance straight out from the middle
    // of a field's south edge is outside the fence by that distance less the
    // fence's, at any latitude; one inside the field or inside the fence is
    // not flagged; and a wider fence encloses more ground and flags fewer.
    let g = Geodesic::wgs84();
    for lat0 in [-60.0, -10.0, 35.0, 68.0] {
        let lon0 = 20.0;
        // A field about 200 m square with its south-west corner at (lat0, lon0).
        let (north, _, _): (f64, f64, f64) =
            geographiclib_rs::DirectGeodesic::direct(&g, lat0, lon0, 0.0, 200.0);
        let (_, east, _): (f64, f64, f64) =
            geographiclib_rs::DirectGeodesic::direct(&g, lat0, lon0, 90.0, 200.0);
        let field = json!([{"lat": lat0, "lon": lon0}, {"lat": lat0, "lon": east}, {"lat": north, "lon": east}, {"lat": north, "lon": lon0}]);
        let mid = (lon0 + east) / 2.0;
        let mut wps = vec![json!({"lat": (lat0 + north) / 2.0, "lon": mid})];
        let out = [10.0, 49.0, 51.0, 80.0, 140.0];
        for d in out {
            let (la, lo, _): (f64, f64, f64) =
                geographiclib_rs::DirectGeodesic::direct(&g, lat0, mid, 180.0, d);
            wps.push(json!({"lat": la, "lon": lo}));
        }
        let run = |fence: f64| {
            call(
                "drone.mission.geofence",
                &json!({"area": field, "distance": format!("{fence} m"), "waypoints": wps}),
            )
        };
        let r = run(50.0);
        let flagged = r["result"]["flagged"].as_array().unwrap();
        // The field's edge is the geodesic between its corners, and the test
        // starts from the parallel through them, a few millimeters off it over
        // 200 m at 68 degrees: hence the centimeter allowed below.
        let want: Vec<(usize, f64)> = out
            .iter()
            .enumerate()
            .filter(|(_, d)| **d > 50.0)
            .map(|(i, d)| (i + 2, d - 50.0))
            .collect();
        assert_eq!(flagged.len(), want.len(), "{lat0}: {r}");
        for (f, (index, beyond)) in flagged.iter().zip(&want) {
            assert_eq!(f["waypoint"].as_f64().unwrap() as usize, *index);
            assert!(
                (f["beyond"]["value"].as_f64().unwrap() - beyond).abs() < 0.01,
                "{lat0}: {f}"
            );
        }
        assert_eq!(r["result"]["outside_count"], want.len() as f64);
        let wide = run(100.0);
        assert_eq!(wide["result"]["outside_count"], 1.0);
        assert!(num(&wide, "result.area_enclosed.value") > num(&r, "result.area_enclosed.value"));
    }
}

#[test]
fn corridor_invariants() {
    // Along a straight centerline every line end is its offset from the
    // matching centerline end, measured geodesically and square to the line;
    // the offsets are centered and one spacing apart; the lines are flown back
    // and forth; and the path is the lines plus the hops between them.
    let g = Geodesic::wgs84();
    for (lat0, lon0, az) in [
        (40.0, -105.0, 30.0),
        (-55.0, 20.0, 100.0),
        (2.0, 100.0, 0.0),
        (68.0, -150.0, 250.0),
    ] {
        let (lat1, lon1, _): (f64, f64, f64) =
            geographiclib_rs::DirectGeodesic::direct(&g, lat0, lon0, az, 1500.0);
        for (width, spacing) in [(120.0, 52.5), (30.0, 40.0), (200.0, 40.0), (90.0, 30.0)] {
            let r = call(
                "drone.mission.corridor",
                &json!({"centerline": [{"lat": lat0, "lon": lon0}, {"lat": lat1, "lon": lon1}], "width": format!("{width} m"), "line_spacing": format!("{spacing} m")}),
            );
            assert_eq!(r["ok"], true, "{r}");
            let n = num(&r, "result.line_count");
            assert_eq!(n, (width / spacing - 1e-9_f64).ceil().max(1.0));
            assert!((num(&r, "result.centerline_length.value") - 1.5).abs() < 1e-9);
            let lines = r["result"]["lines"].as_array().unwrap();
            assert_eq!(lines.len() as f64, n);
            let mut flown = 0.0;
            let mut last: Option<(f64, f64)> = None;
            for (k, line) in lines.iter().enumerate() {
                let off = line["offset"]["value"].as_f64().unwrap();
                assert!((off - (k as f64 - (n - 1.0) / 2.0) * spacing).abs() < 1e-9);
                let w: Vec<(f64, f64)> = line["waypoints"]
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
                assert_eq!(w.len(), 2);
                assert_eq!(line["waypoints"][0]["kind"], "line_start");
                assert_eq!(line["waypoints"][1]["kind"], "line_end");
                // Odd lines run back toward the start of the centerline.
                let (near_start, near_end) = if k % 2 == 0 {
                    (w[0], w[1])
                } else {
                    (w[1], w[0])
                };
                for (c, p, course) in [((lat0, lon0), near_start, az), ((lat1, lon1), near_end, az)]
                {
                    let (s, a1, _, _): (f64, f64, f64, f64) = g.inverse(c.0, c.1, p.0, p.1);
                    assert!((s - off.abs()).abs() < 1e-4, "{lat0} {off}: {s}");
                    if off != 0.0 {
                        // Right of the line for a positive offset. The far end's
                        // course has turned a little along the geodesic.
                        let side =
                            (a1 - course - 90.0 * off.signum() + 540.0).rem_euclid(360.0) - 180.0;
                        assert!(side.abs() < 0.05, "{lat0} {off}: {side}");
                    }
                }
                let leg: f64 = g.inverse(w[0].0, w[0].1, w[1].0, w[1].1);
                assert!((leg - 1500.0).abs() < 1e-3, "{leg}");
                flown += leg;
                if let Some(prev) = last {
                    let hop: f64 = g.inverse(prev.0, prev.1, w[0].0, w[0].1);
                    assert!((hop - spacing).abs() < 1e-4, "{hop}");
                    flown += hop;
                }
                last = Some(w[1]);
            }
            assert!(
                (num(&r, "result.path_length.value") * 1000.0 - flown).abs() < 1e-2,
                "{r}"
            );
        }
    }
}

#[test]
fn facade_invariants() {
    // Every station looks square at the wall from the standoff: carried along
    // its heading by the standoff it lands on the wall between the two ends.
    // The photos cover the wall end to end and bottom to top with at least the
    // overlap asked for, and the other side of the wall is the mirror image.
    // The heading is the bearing from the wall turned about, which differs from
    // the true bearing back by the meridians' convergence over the standoff:
    // under 0.003° here, a few millimeters along the wall.
    const TOL: f64 = 0.01;
    let g = Geodesic::wgs84();
    let direct = |la: f64, lo: f64, az: f64, s: f64| -> (f64, f64) {
        geographiclib_rs::DirectGeodesic::direct(&g, la, lo, az, s)
    };
    let dist = |a: (f64, f64), b: (f64, f64)| -> f64 { g.inverse(a.0, a.1, b.0, b.1) };
    for (lat, lon, brg, len, standoff, top, bottom, oh, ov) in [
        (40.0, -105.0, 73.0, 120.0, 25.0, 40.0, 0.0, 75.0, 60.0),
        (-45.0, 170.0, 200.0, 60.0, 12.0, 30.0, 5.0, 80.0, 70.0),
        (70.0, 25.0, 0.0, 300.0, 80.0, 90.0, 10.0, 65.0, 55.0),
        (0.5, -60.0, 315.0, 18.0, 30.0, 15.0, 0.0, 75.0, 60.0),
    ] {
        let a = (lat, lon);
        let b = direct(lat, lon, brg, len);
        let run = |side: &str| {
            let r = call(
                "drone.mission.facade",
                &json!({"facade": [{"lat": a.0, "lon": a.1}, {"lat": b.0, "lon": b.1}], "standoff": format!("{standoff} m"),
                    "top_height": format!("{top} m"), "bottom_height": format!("{bottom} m"), "sensor_width": "13.2 mm",
                    "sensor_height": "8.8 mm", "focal_length": "8.8 mm", "horizontal_overlap": oh, "vertical_overlap": ov, "side": side}),
            );
            assert_eq!(r["ok"], true, "{r}");
            r
        };
        let r = run("right");
        let (w, h) = (
            num(&r, "result.footprint_width.value"),
            num(&r, "result.footprint_height.value"),
        );
        assert!((w - 1.5 * standoff).abs() < 1e-9 && (h - standoff).abs() < 1e-9);
        let (passes, per) = (
            num(&r, "result.passes") as usize,
            num(&r, "result.photos_per_pass") as usize,
        );
        assert_eq!(num(&r, "result.photos") as usize, passes * per);
        assert!((num(&r, "result.facade_length.value") - len).abs() < 1e-6);
        let wps = r["result"]["waypoints"].as_array().unwrap();
        assert_eq!(wps.len(), passes * per);
        let f = |p: &Value, k: &str| p[k]["value"].as_f64().unwrap();
        let mut along = Vec::new();
        for p in wps {
            let foot = direct(f(p, "lat"), f(p, "lon"), f(p, "heading"), standoff);
            let (da, db) = (dist(a, foot), dist(foot, b));
            assert!(
                (da + db - len).abs() < 1e-6,
                "{lat}: off the wall by {}",
                da + db - len
            );
            along.push(da);
        }
        // The first pass runs from the start of the wall; the next comes back.
        let first = &along[..per];
        assert!(first.windows(2).all(|p| p[1] > p[0]));
        if passes > 1 {
            assert!(along[per..2 * per].windows(2).all(|p| p[1] < p[0]));
        }
        let (ps, zs) = (
            num(&r, "result.photo_spacing.value"),
            num(&r, "result.pass_spacing.value"),
        );
        if per > 1 {
            assert!(
                (first[0] - w / 2.0).abs() < TOL && (first[per - 1] - (len - w / 2.0)).abs() < TOL
            );
            assert!(first.windows(2).all(|p| (p[1] - p[0] - ps).abs() < TOL));
            assert!(ps <= w * (1.0 - oh / 100.0) + 1e-9, "{ps}");
        } else {
            assert!((first[0] - len / 2.0).abs() < TOL && ps == 0.0);
        }
        let heights: Vec<f64> = (0..passes).map(|k| f(&wps[k * per], "height")).collect();
        if passes > 1 {
            assert!((heights[0] - (bottom + h / 2.0)).abs() < 1e-9);
            assert!((heights[passes - 1] - (top - h / 2.0)).abs() < 1e-9);
            assert!(heights.windows(2).all(|p| (p[1] - p[0] - zs).abs() < 1e-9));
            assert!(zs <= h * (1.0 - ov / 100.0) + 1e-9, "{zs}");
        } else {
            assert!((heights[0] - (bottom + top) / 2.0).abs() < 1e-9 && zs == 0.0);
        }
        // The other side: the same stations, twice the standoff away.
        let l = run("left");
        for (p, q) in wps.iter().zip(l["result"]["waypoints"].as_array().unwrap()) {
            let d = dist((f(p, "lat"), f(p, "lon")), (f(q, "lat"), f(q, "lon")));
            assert!((d - 2.0 * standoff).abs() < 1e-5, "{d}");
            assert_eq!(f(p, "height"), f(q, "height"));
        }
    }
}
