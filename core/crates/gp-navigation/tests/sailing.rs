//! The sailings (add-marine-navigation, "sailings"), against Bowditch 2024
//! Volume I chapter 13's worked examples.

use gp_navigation::REGISTRY;
use serde_json::{Value, json};

fn call(id: &str, input: Value) -> Value {
    let r: Value = serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).unwrap();
    assert_eq!(r["ok"], true, "{id}: {r}");
    r["result"].clone()
}

fn v(r: &Value, f: &str) -> f64 {
    r[f]["value"]
        .as_f64()
        .or_else(|| r[f].as_f64())
        .unwrap_or_else(|| panic!("{f} in {r}"))
}

fn near(got: f64, want: f64, tol: f64, what: &str) {
    assert!(
        (got - want).abs() <= tol,
        "{what}: {got} vs {want} (±{tol})"
    );
}

#[test]
fn bowditch_chapter_13_examples() {
    // §1302: 188.0 NM on 005°: l = 187.3' N, p = 16.4 NM E.
    let r = call(
        "navigation.sailing.plane",
        json!({"course": "5 deg", "distance": "188 NM"}),
    );
    near(v(&r, "lat_difference"), 187.3, 0.05, "plane l");
    near(v(&r, "departure"), 16.4, 0.05, "plane p");
    // And back again.
    let r = call(
        "navigation.sailing.plane",
        json!({"lat_difference": "187.28372 NM", "departure": "16.38608 NM"}),
    );
    near(v(&r, "course"), 5.0, 1e-3, "plane course");
    near(v(&r, "distance"), 188.0, 1e-3, "plane distance");

    // §1303: the five-leg traverse: 192.3° T, 67.3 NM.
    let r = call(
        "navigation.sailing.traverse",
        json!({"legs": [
        {"course": "158 deg", "distance": "15.5 NM"}, {"course": "135 deg", "distance": "33.7 NM"},
        {"course": "259 deg", "distance": "16.1 NM"}, {"course": "293 deg", "distance": "39.0 NM"},
        {"course": "169 deg", "distance": "40.4 NM"}]}),
    );
    // Bowditch sums traverse-table values rounded to 0.1 NM per leg (65.8 S,
    // 14.4 W); the exact sums give 192.3° and 67.24 NM.
    near(v(&r, "course"), 192.3, 0.05, "traverse course");
    near(v(&r, "distance"), 67.3, 0.1, "traverse distance");

    // §1304: DLo 3°30' on 49°30'N: p = 136.4 NM E.
    let r = call(
        "navigation.sailing.parallel",
        json!({"latitude": "49°30'N", "dlo": "3.5 deg"}),
    );
    near(v(&r, "departure"), 136.4, 0.05, "parallel p");
    let back = call(
        "navigation.sailing.parallel",
        json!({"latitude": "49°30'N", "departure": "136.3845 NM"}),
    );
    near(
        v(&back, "dlo"),
        3.5,
        1e-4,
        "parallel DLo (Bowditch rounds cos L to 0.64945)",
    );

    // §1305: 1,253 NM on 070° from 15°17.0'N 151°37.0'E: 22°25.6'N.
    // Bowditch prints 172°21.1'E, from DLo rounded to 1,244.1'; at full
    // precision DLo is 1,244.196' and the longitude 172°21.2'E.
    let r = call(
        "navigation.sailing.mid-latitude",
        json!({"lat1": "15°17.0'N", "lon1": "151°37.0'E", "course": "70 deg", "distance": "1253 NM"}),
    );
    near(
        v(&r, "lat2"),
        22.0 + 25.6 / 60.0,
        0.05 / 60.0,
        "mid-latitude arrival latitude",
    );
    near(
        v(&r, "lon2"),
        172.0 + 21.2 / 60.0,
        0.05 / 60.0,
        "mid-latitude arrival longitude",
    );

    // §1306: 32°14.7'N 66°28.9'W to 36°58.7'N 75°42.2'W: M₁ 2033.4, M₂
    // 2377.1, m 343.7, DLo 553.3' W, 301.8° T. Bowditch prints 538.7 NM
    // from 58.18521°, but atan(553.3 / 343.7) is 58.1526°, which gives 538.2.
    let r = call(
        "navigation.sailing.mercator",
        json!({"lat1": "32°14.7'N", "lon1": "66°28.9'W", "lat2": "36°58.7'N", "lon2": "75°42.2'W"}),
    );
    near(v(&r, "m1"), 2033.4, 0.06, "M1");
    near(v(&r, "m2"), 2377.1, 0.06, "M2");
    near(v(&r, "meridional_difference"), 343.7, 0.05, "m");
    near(v(&r, "course"), 301.8, 0.05, "Mercator course");
    near(v(&r, "distance"), 538.2, 0.05, "Mercator distance");
    assert_eq!(r["dlo_label"], "553.3' W");
    // The exact rhumb line beside it: the course agrees to 0.01°, and the
    // sphere's distance runs long of the ellipsoid's at this latitude.
    assert!(
        v(&r, "course_difference").abs() < 0.01,
        "{}",
        r["course_difference"]
    );
    let dd = v(&r, "distance_difference");
    assert!(dd > 0.0 && dd < 0.005 * 538.2, "distance difference {dd}");

    // §1307 example 1: 22°S 116°E to 20°S 031°E: 4,693.5 NM, 253° T; example
    // 3: final course 289.4° T.
    let r = call(
        "navigation.sailing.great-circle",
        json!({"lat1": "22 S", "lon1": "116 E", "lat2": "20 S", "lon2": "31 E"}),
    );
    near(v(&r, "distance"), 4693.5, 0.05, "great-circle distance");
    near(v(&r, "course"), 253.0, 0.05, "great-circle initial course");
    near(
        v(&r, "final_course"),
        289.4,
        0.05,
        "great-circle final course",
    );
    // Example 2, across the equator: 28°N 122°W to 24°S 151°E: 5,913.1 NM, 247.3° T.
    let r = call(
        "navigation.sailing.great-circle",
        json!({"lat1": "28 N", "lon1": "122 W", "lat2": "24 S", "lon2": "151 E"}),
    );
    // Bowditch rounds D to 98.55° before multiplying by 60; the full arc is
    // 98.5537°, 5,913.2 NM.
    near(v(&r, "distance"), 5913.1, 0.15, "equator-crossing distance");
    near(v(&r, "course"), 247.3, 0.05, "equator-crossing course");

    // §1308: composite under 47°N from 36°57.7'N 75°42.2'W to 45°39.1'N
    // 001°29.8'W: the limit is reached at 030°16.2'W and left at 018°57.5'W.
    let r = call(
        "navigation.sailing.great-circle",
        json!({"lat1": "36°57.7'N", "lon1": "75°42.2'W", "lat2": "45°39.1'N", "lon2": "1°29.8'W", "limit": "47 deg"}),
    );
    let legs = r["legs"].as_array().unwrap();
    assert_eq!(legs.len(), 3, "{r}");
    // The longitude in a leg's label, like "Great circle to 47°00.0'N 30°16.1'W".
    let west = |t: &str| -> f64 {
        let lon = t.rsplit(' ').next().unwrap().trim_end_matches('W');
        let (d, m) = lon.split_once('°').unwrap();
        d.parse::<f64>().unwrap() + m.trim_end_matches('\'').parse::<f64>().unwrap() / 60.0
    };
    // Bowditch's tangents are rounded (0.75251 / 1.07237), so within 0.15'.
    near(
        west(legs[0]["leg"].as_str().unwrap()),
        30.0 + 16.2 / 60.0,
        0.15 / 60.0,
        "limit reached",
    );
    // Bowditch prints 018°57.5'W, but acos(0.95397) is 17.4515° (17°27.1'),
    // not its 17.4651° (17°27.9'), and its own sum 1°29.8' + 17°27.9' is
    // 18°57.7'. The exact value is 18°56.9'W.
    near(
        west(legs[1]["leg"].as_str().unwrap()),
        18.0 + 56.9 / 60.0,
        0.05 / 60.0,
        "limit left",
    );
    assert!(v(&r, "composite_distance") > v(&r, "distance"));
}

#[test]
fn sailing_invariants() {
    // The spherical great circle equals the spherical inverse tool on a
    // sphere of 10800/π NM (one nautical mile per minute of arc).
    let r_m = 1852.0 * 10800.0 / std::f64::consts::PI;
    for (a, b, c, d) in [
        (40.6, -73.8, 51.5, -0.5),
        (-33.9, 151.2, 35.7, 139.8),
        (10.0, 20.0, -10.0, -150.0),
    ] {
        let gc = call(
            "navigation.sailing.great-circle",
            json!({"lat1": a, "lon1": b, "lat2": c, "lon2": d}),
        );
        let si = call(
            "navigation.geodesic.spherical-inverse",
            json!({"lat1": a, "lon1": b, "lat2": c, "lon2": d, "radius": format!("{r_m} m")}),
        );
        let km = si["distance"]["value"].as_f64().unwrap();
        assert_eq!(si["distance"]["unit"], "km");
        near(
            v(&gc, "distance"),
            km / 1.852,
            1e-9 * km,
            "great-circle distance vs spherical inverse",
        );
        near(
            v(&gc, "course"),
            v(&si, "initial_course"),
            1e-9,
            "initial course vs spherical inverse",
        );
        near(
            v(&gc, "final_course"),
            v(&si, "final_course"),
            1e-9,
            "final course vs spherical inverse",
        );
    }
    // Antipodes are refused.
    let r: Value = serde_json::from_str(&REGISTRY.invoke(
        "navigation.sailing.great-circle",
        &json!({"lat1": 10, "lon1": 20, "lat2": -10, "lon2": -160}).to_string(),
    ))
    .unwrap();
    assert_eq!(r["error"]["code"], "DEGENERATE_GEOMETRY", "{r}");
    // A composite under 45°N on a track whose vertex is above it: three legs,
    // the middle on 45°N, longer than the great circle.
    let r = call(
        "navigation.sailing.great-circle",
        json!({"lat1": 40.0, "lon1": -70.0, "lat2": 43.0, "lon2": -10.0, "limit": "45 deg"}),
    );
    assert!(v(&r, "vertex_lat") > 45.0, "{r}");
    assert_eq!(r["legs"].as_array().unwrap().len(), 3);
    assert!(r["legs"][1]["leg"].as_str().unwrap().contains("45°00.0'N"));
    assert!(v(&r, "composite_distance") > v(&r, "distance"));
    // Mercator there and back: the arrival from the course and distance is the place given.
    let m = call(
        "navigation.sailing.mercator",
        json!({"lat1": 20.0, "lon1": -40.0, "lat2": 35.0, "lon2": -10.0}),
    );
    let back = call(
        "navigation.sailing.mercator",
        json!({"lat1": 20.0, "lon1": -40.0, "course": format!("{} deg", v(&m, "course")), "distance": format!("{} NM", v(&m, "distance"))}),
    );
    near(v(&back, "lat2"), 35.0, 1e-9, "Mercator round trip latitude");
    near(
        v(&back, "lon2"),
        -10.0,
        1e-9,
        "Mercator round trip longitude",
    );
}
