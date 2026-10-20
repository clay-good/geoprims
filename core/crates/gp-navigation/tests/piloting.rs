//! Piloting (add-marine-navigation, "piloting"), against Bowditch 2024.

use gp_navigation::REGISTRY;
use serde_json::{Value, json};

fn invoke(id: &str, input: Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).unwrap()
}
fn call(id: &str, input: Value) -> Value {
    let r = invoke(id, input);
    assert_eq!(r["ok"], true, "{id}: {r}");
    r["result"].clone()
}
fn v(r: &Value, f: &str) -> f64 {
    r[f]["value"]
        .as_f64()
        .unwrap_or_else(|| panic!("{f} in {r}"))
}
fn near(got: f64, want: f64, tol: f64, what: &str) {
    assert!(
        (got - want).abs() <= tol,
        "{what}: {got} vs {want} (±{tol})"
    );
}

#[test]
fn current_triangle_bowditch_1008() {
    // Example 1: course 080° at 10 kn, set 140°, drift 2 kn: track 089°, 11.2 kn.
    let r = call(
        "navigation.piloting.set-and-drift",
        json!({"problem": "track", "course": "80 deg", "speed": "10 kt", "set": "140 deg", "drift": "2 kt"}),
    );
    near(v(&r, "track"), 89.0, 0.5, "track");
    near(v(&r, "speed_made_good"), 11.2, 0.1, "speed made good");
    // Example 2: make good 095° at 12 kn through set 170°, drift 2.5 kn: steer 083.5°, 12.4 kn.
    let r = call(
        "navigation.piloting.set-and-drift",
        json!({"problem": "course-to-steer", "course": "95 deg", "speed": "12 kt", "set": "170 deg", "drift": "2.5 kt"}),
    );
    near(v(&r, "course_to_steer"), 83.5, 0.5, "course to steer");
    near(v(&r, "speed_made_good"), 12.4, 0.1, "speed made good");
    // Example 3: make good 265° at 15 kn through set 185°, drift 3 kn: steer 276°, 14.8 kn.
    let r = call(
        "navigation.piloting.set-and-drift",
        json!({"problem": "course-and-speed", "course": "265 deg", "speed": "15 kt", "set": "185 deg", "drift": "3 kt"}),
    );
    // Bowditch plots this one and prints the course to the degree; the exact
    // triangle gives 276.53°.
    near(v(&r, "course_to_steer"), 276.0, 1.0, "course to steer");
    near(v(&r, "speed_to_use"), 14.8, 0.1, "speed to use");
    // Too strong a current: at 3 kn the ship cannot make good a course
    // straight into 5 kn of current, nor hold one across 5 kn.
    let r = invoke(
        "navigation.piloting.set-and-drift",
        json!({"problem": "course-to-steer", "course": "0 deg", "speed": "3 kt", "set": "180 deg", "drift": "5 kt"}),
    );
    assert_eq!(r["error"]["code"], "NO_SOLUTION", "{r}");
    let r = invoke(
        "navigation.piloting.set-and-drift",
        json!({"problem": "course-to-steer", "course": "0 deg", "speed": "3 kt", "set": "90 deg", "drift": "5 kt"}),
    );
    assert_eq!(r["error"]["code"], "NO_SOLUTION", "{r}");
}

#[test]
fn marine_and_aviation_triangles_agree() {
    // Course to steer: the marine current triangle and the aviation wind
    // triangle run one function, so a current of set s is a wind from s + 180.
    let avi = gp_geo::wind::heading_groundspeed(95.0, 12.0, 350.0, 2.5).unwrap();
    let r = call(
        "navigation.piloting.set-and-drift",
        json!({"problem": "course-to-steer", "course": 95.0, "speed": "12 kt", "set": 170.0, "drift": "2.5 kt"}),
    );
    assert_eq!(
        v(&r, "course_to_steer"),
        gp_base::angle::wrap_azimuth(95.0 + avi.0)
    );
    assert_eq!(v(&r, "speed_made_good"), avi.1);
}

#[test]
fn bearings_and_fixes() {
    // Doubling the angle: 30° and 60° on the bow, 4.0 NM apart.
    let r = call(
        "navigation.piloting.two-bearing-fix",
        json!({"preset": "doubling", "first": "30 deg", "run": "4 NM"}),
    );
    near(v(&r, "distance_second"), 4.0, 1e-9, "doubling distance");
    near(v(&r, "distance_abeam"), 3.46, 0.01, "abeam");
    // Bow and beam: the distance abeam is the run.
    let r = call(
        "navigation.piloting.two-bearing-fix",
        json!({"preset": "bow-and-beam", "run": "3 NM"}),
    );
    near(v(&r, "distance_abeam"), 3.0, 1e-9, "bow and beam");
    // §1121: on 050° at 15 kn a light bears 024° at 1130 and 359° at 1140:
    // 2.6 NM at 1140 and 2.0 NM abeam (Table 18 factors 1.04 and 0.81).
    let r = call(
        "navigation.piloting.running-fix",
        json!({"course": "50 deg", "bearing1": "24 deg", "bearing2": "359 deg", "speed": "15 kt", "time": "10 min", "object_lat": "36°55'N", "object_lon": "76°00'W"}),
    );
    near(v(&r, "run"), 2.5, 1e-9, "run");
    near(v(&r, "distance"), 2.6, 0.05, "distance at 1140");
    near(v(&r, "distance_abeam"), 2.0, 0.05, "distance abeam");
    assert_eq!(r["lines"].as_array().unwrap().len(), 4);
    // The fix lies on the second bearing from the object at that distance.
    let (lat, lon) = (v(&r, "lat"), v(&r, "lon"));
    use geographiclib_rs::{Geodesic, InverseGeodesic};
    let (s, az1, _, _): (f64, f64, f64, f64) =
        Geodesic::wgs84().inverse(lat, lon, 36.0 + 55.0 / 60.0, -76.0);
    near(
        s / 1852.0,
        v(&r, "distance"),
        1e-9,
        "fix to object distance",
    );
    // Within the meridians' convergence over 2.6 NM.
    near(
        gp_base::angle::wrap_azimuth(az1),
        359.0,
        1e-3,
        "bearing from the fix",
    );
}

#[test]
fn vertical_angle_table_15() {
    // H − h = 100 ft at 0°30': Table 15, 1.8 NM; and other entries.
    for (a, hh, want) in [
        ("0°30'", 100.0, 1.8),
        ("0°00'", 25.0, 5.8),
        ("0°10'", 50.0, 2.6),
        ("0°05'", 100.0, 7.1),
        ("1°00'", 140.0, 1.3),
    ] {
        let r = call(
            "navigation.piloting.distance-by-vertical-angle",
            json!({"angle": a, "object_height": format!("{} ft", hh + 20.0), "eye_height": "20 ft"}),
        );
        near(
            v(&r, "distance"),
            want,
            0.05,
            &format!("Table 15 at {a}, {hh} ft"),
        );
    }
    // From the waterline, an object far beyond the horizon warns.
    let r = invoke(
        "navigation.piloting.distance-by-vertical-angle",
        json!({"angle": "0°05'", "object_height": "200 ft", "eye_height": "9 ft", "method": "waterline"}),
    );
    assert!(
        r["meta"]["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["code"] == "BEYOND_HORIZON"),
        "{r}"
    );
}

#[test]
fn compass_bowditch_402() {
    // TH 215°, V 7° W, D 1.5° W: MH 222°, CH 223.5°, CE 8.5° W; a light at
    // 306.5° by compass: MB 305°, TB 298°, relative 083°.
    let r = call(
        "navigation.piloting.compass-conversion",
        json!({"heading": "215 deg", "variation": "7 W", "deviation": "1.5 W", "bearing": "306.5 deg", "bearing_reference": "compass"}),
    );
    near(v(&r, "magnetic_heading"), 222.0, 1e-9, "MH");
    near(v(&r, "compass_heading"), 223.5, 1e-9, "CH");
    assert_eq!(r["compass_error"], "8.5° W");
    near(v(&r, "magnetic_bearing"), 305.0, 1e-9, "MB");
    near(v(&r, "true_bearing"), 298.0, 1e-9, "TB");
    near(v(&r, "relative_bearing"), 83.0, 1e-9, "RB");
    // Problem 1 rows of §402: (1) TC 105, V 15 E, D 5 W: MC 090, CC 095, CE 10 E.
    let r = call(
        "navigation.piloting.compass-conversion",
        json!({"heading": "105 deg", "variation": "15 E", "deviation": "5 W"}),
    );
    near(v(&r, "magnetic_heading"), 90.0, 1e-9, "row 1 MC");
    near(v(&r, "compass_heading"), 95.0, 1e-9, "row 1 CC");
    assert_eq!(r["compass_error"], "10.0° E");
    // (3) V 12 W, CC 067, CE 7 W, so D 5 E: TC 060, MC 072.
    let r = call(
        "navigation.piloting.compass-conversion",
        json!({"heading": "67 deg", "heading_reference": "compass", "variation": "12 W", "deviation": "5 E"}),
    );
    near(v(&r, "true_heading"), 60.0, 1e-9, "row 3 TC");
    near(v(&r, "magnetic_heading"), 72.0, 1e-9, "row 3 MC");
    // Problem 2: CC 150°, V 19° E, D 1° E: CE 20° E, MH 151°, TH 170°.
    let r = call(
        "navigation.piloting.compass-conversion",
        json!({"heading": "150 deg", "heading_reference": "compass", "variation": "19 E", "deviation": "1 E"}),
    );
    near(v(&r, "magnetic_heading"), 151.0, 1e-9, "problem 2 MH");
    near(v(&r, "true_heading"), 170.0, 1e-9, "problem 2 TH");
    // A deviation table, interpolated at the magnetic heading, matches the
    // aviation heading chain's card on the same heading.
    let table = json!([{"heading": "0 deg", "deviation": "2 deg"}, {"heading": "90 deg", "deviation": "-2 deg"}, {"heading": "180 deg", "deviation": "1 deg"}, {"heading": "270 deg", "deviation": "0 deg"}]);
    let r = call(
        "navigation.piloting.compass-conversion",
        json!({"heading": "120 deg", "variation": "10 E", "deviation_table": table}),
    );
    let card = [(0.0, 2.0), (90.0, -2.0), (180.0, 1.0), (270.0, 0.0)];
    let (m, d, c) = gp_geo::compass::true_to_compass(120.0, 10.0, &card);
    assert_eq!(
        (v(&r, "magnetic_heading"), v(&r, "compass_heading")),
        (m, c)
    );
    let _ = d;
    // And back from compass to true.
    let back = call(
        "navigation.piloting.compass-conversion",
        json!({"heading": format!("{c} deg"), "heading_reference": "compass", "variation": "10 E", "deviation_table": table}),
    );
    near(
        v(&back, "true_heading"),
        120.0,
        1e-9,
        "compass back to true",
    );
}

#[test]
fn dead_reckoning_runs() {
    let r = call(
        "navigation.piloting.dead-reckoning",
        json!({"lat": "36°55'N", "lon": "76°00'W", "course": "50 deg", "speed": "15 kt", "time": "2 h"}),
    );
    near(v(&r, "distance"), 30.0, 1e-9, "run");
    assert!(v(&r, "method_difference") < 0.05, "{r}");
    let r = call(
        "navigation.piloting.dead-reckoning",
        json!({"lat": 30, "lon": -40, "course": "90 deg", "distance": "300 NM"}),
    );
    assert!(r["method"].as_str().unwrap().starts_with("rhumb"));
    let r = invoke(
        "navigation.piloting.dead-reckoning",
        json!({"lat": 30, "lon": -40, "course": "90 deg", "distance": "700 NM"}),
    );
    assert_eq!(r["error"]["code"], "LIMIT_EXCEEDED");
}
