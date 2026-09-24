//! Where two courses cross: agreement with GeographicLib's IntersectTool, and
//! the properties every answer must have.

use gp_navigation::REGISTRY;
use serde_json::{Value, json};

fn call(id: &str, input: Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).expect("JSON")
}

fn num(r: &Value, k: &str) -> f64 {
    r["result"][k]["value"]
        .as_f64()
        .unwrap_or_else(|| panic!("{k} in {r}"))
}

fn cross(c: &[f64], method: &str) -> Value {
    call(
        "navigation.route.course-intersection",
        json!({"lat1": c[0], "lon1": c[1], "course1": format!("{} deg", c[2]),
               "lat2": c[3], "lon2": c[4], "course2": format!("{} deg", c[5]), "method": method}),
    )
}

/// 500 random pairs up to about 3,000 km apart, against IntersectTool's
/// crossings (tools/vectors/gen_intersect_more.py): the same crossing, the
/// runs within 10 micrometers.
#[test]
fn course_intersection_matches_intersect_tool() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/course_intersection.json"
    );
    let fx: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("JSON");
    let cases = fx["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 500);
    let mut wrong = Vec::new();
    for (i, (c, want)) in cases.iter().zip(fx["runs"].as_array().unwrap()).enumerate() {
        let c: Vec<f64> = c
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        let r = cross(&c, "geodesic");
        let (s1, s2) = (num(&r, "distance1"), num(&r, "distance2"));
        let (w1, w2) = (want[0].as_f64().unwrap(), want[1].as_f64().unwrap());
        if (s1 - w1).abs() > 1e-5 || (s2 - w2).abs() > 1e-5 {
            wrong.push(format!("pair {i}: {s1} {s2}, IntersectTool {w1} {w2}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of 500 differ:\n{}",
        wrong.len(),
        wrong[..wrong.len().min(10)].join("\n")
    );
}

#[test]
fn course_intersection_invariants() {
    let pairs: [[f64; 6]; 6] = [
        [42.36, -71.0, 45.0, 43.66, -70.26, 135.0],
        [10.0, 0.0, 80.0, -20.0, 40.0, 10.0],
        [-33.9, 151.2, 120.0, -36.8, 174.8, 200.0],
        [60.0, 20.0, 0.0, 60.0, 30.0, 315.0],
        [0.0, 179.0, 60.0, 5.0, -175.0, 250.0],
        [70.0, -40.0, 100.0, 65.0, -10.0, 340.0],
    ];
    for c in pairs {
        for (method, direct, angle) in [
            ("geodesic", "navigation.geodesic.direct", "azimuth"),
            ("rhumb", "navigation.rhumb.direct", "course"),
        ] {
            let r = cross(&c, method);
            let (s1, s2, lat, lon) = (
                num(&r, "distance1"),
                num(&r, "distance2"),
                num(&r, "lat"),
                num(&r, "lon"),
            );
            // Swapping the two positions swaps the runs.
            let swapped = cross(&[c[3], c[4], c[5], c[0], c[1], c[2]], method);
            assert!(
                (num(&swapped, "distance1") - s2).abs() < 1e-6,
                "{method} {c:?}"
            );
            assert!(
                (num(&swapped, "distance2") - s1).abs() < 1e-6,
                "{method} {c:?}"
            );
            // Carrying each course its run, by the direct tool of the same
            // kind, lands on the reported crossing (a negative run goes the
            // other way along the course).
            for (la, lo, course, s) in [(c[0], c[1], c[2], s1), (c[3], c[4], c[5], s2)] {
                let (az, dist) = if s < 0.0 {
                    ((course + 180.0) % 360.0, -s)
                } else {
                    (course, s)
                };
                let d = call(
                    direct,
                    json!({"lat1": la, "lon1": lo, angle: format!("{az} deg"), "distance": format!("{dist} m")}),
                );
                let (la2, lo2) = (num(&d, "lat2"), num(&d, "lon2"));
                let dlon = ((lo2 - lon + 540.0) % 360.0) - 180.0;
                let off = ((la2 - lat).powi(2) + (dlon * lat.to_radians().cos()).powi(2)).sqrt()
                    * 111_195.0;
                assert!(
                    off < 1e-3,
                    "{method} {c:?}: the run from {la}, {lo} lands {off} m from the crossing\n{d}"
                );
            }
        }
    }
    // Parallel courses along one line have no crossing.
    let r = cross(&[0.0, 0.0, 90.0, 0.0, 10.0, 90.0], "geodesic");
    assert_eq!(r["error"]["code"], "NO_SOLUTION", "{r}");
}
