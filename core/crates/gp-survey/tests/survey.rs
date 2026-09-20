//! Survey tools: catalog lint, examples, golden vectors, spec scenarios, and hardening.

use std::path::Path;

use gp_base::{manifest, template, vectors};
use gp_survey::{REGISTRY, TOOLS};
use serde_json::Value;

fn repo(path: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(path);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

fn call(id: &str, input: &str) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, input)).expect("envelope is JSON")
}

fn num(r: &Value, path: &str) -> f64 {
    path.split('.')
        .fold(r, |v, k| &v[k])
        .as_f64()
        .unwrap_or_else(|| panic!("{path} missing in {r}"))
}

fn codes(r: &Value) -> Vec<String> {
    r["meta"]["warnings"].as_array().map_or(vec![], |a| {
        a.iter()
            .map(|w| w["code"].as_str().unwrap().to_owned())
            .collect()
    })
}

#[test]
fn catalog_examples_vectors() {
    let tax: Value = serde_json::from_str(&repo("data/taxonomy.json")).unwrap();
    let owned: Vec<(String, Vec<String>)> = tax["domains"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(d, v)| {
            (
                d.clone(),
                v["groups"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|g| g.as_str().unwrap().to_owned())
                    .collect(),
            )
        })
        .collect();
    let taxonomy: Vec<(&str, Vec<&str>)> = owned
        .iter()
        .map(|(d, g)| (d.as_str(), g.iter().map(String::as_str).collect()))
        .collect();
    // Tools in other crates that survey tools point at; this crate cannot see
    // them, so they are named here.
    let mut failures = manifest::lint(TOOLS, &taxonomy, &["geodesy.spcs.spcs83-forward"]);
    let reg: Value = serde_json::from_str(&repo("data/codes.json")).unwrap();
    for t in TOOLS {
        failures.extend(
            t.warnings
                .iter()
                .filter(|w| reg["warnings"].get(**w).is_none())
                .map(|w| format!("{} unregistered {w}", t.id)),
        );
        for ex in t.examples {
            let r = call(t.id, ex.input);
            let s = r["summary"].as_str().unwrap_or_default();
            if r["ok"] != true || template::grade(s) > 8.0 || s.len() > template::MAX_CHARS {
                failures.push(format!(
                    "{} example (grade {:.1}): {r}",
                    t.id,
                    template::grade(s)
                ));
            }
        }
        let text = repo(&format!("core/vectors/{}.jsonl", t.id));
        failures.extend(vectors::lint(t.id, &text));
        failures.extend(vectors::run(&REGISTRY, t.id, &text));
        if vectors::count(&text) < 5 {
            failures.push(format!("{} has fewer than 5 vectors", t.id));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn inverse_and_bearings() {
    let r = call(
        "survey.cogo.inverse",
        r#"{"northing1":1000,"easting1":1000,"northing2":1100,"easting2":1100}"#,
    );
    assert_eq!(r["result"]["bearing"], "N 45°00'00\" E");
    assert_eq!(r["display"]["distance"], "141.421 ft");
    let f = call(
        "survey.cogo.forward",
        r#"{"northing":0,"easting":0,"direction":"S 44°30'00\" W","distance":100}"#,
    );
    let back = call(
        "survey.cogo.inverse",
        &format!(
            r#"{{"northing1":0,"easting1":0,"northing2":{},"easting2":{}}}"#,
            num(&f, "result.northing.value"),
            num(&f, "result.easting.value")
        ),
    );
    assert!(
        (num(&back, "result.azimuth.value") - 224.5).abs() < 1e-9,
        "{back}"
    );
    let same = call(
        "survey.cogo.inverse",
        r#"{"northing1":1,"easting1":1,"northing2":1,"easting2":1}"#,
    );
    assert!(codes(&same).contains(&"AZIMUTH_UNDEFINED".to_owned()));
    let bad = call(
        "survey.cogo.forward",
        r#"{"northing":0,"easting":0,"direction":"N 95 E","distance":1}"#,
    );
    assert_eq!(bad["error"]["code"], "INVALID_INPUT");
}

#[test]
fn mixed_feet_are_refused() {
    let r = call(
        "survey.cogo.forward",
        r#"{"northing":"0 ft","easting":"0 ftUS","direction":"90","distance":"1 ft"}"#,
    );
    assert_eq!(r["error"]["code"], "UNIT_MISMATCH");
    let us = call(
        "survey.cogo.forward",
        r#"{"northing":"0 ftUS","easting":"0 ftUS","direction":"90","distance":"1 ftUS"}"#,
    );
    assert!(codes(&us).contains(&"LEGACY_UNIT".to_owned()));
    assert_eq!(us["result"]["easting"]["unit"], "ftUS");
}

#[test]
fn traverse_closure_scenario() {
    let r = call(
        "survey.cogo.traverse-closure",
        r#"{"courses":[{"direction":"0","distance":300.00},{"direction":"90","distance":400.02},{"direction":"180","distance":299.95},{"direction":"270.01","distance":400.00}]}"#,
    );
    assert!((num(&r, "result.misclosure.value") - 0.1215).abs() < 5e-5);
    assert_eq!(r["display"]["total_length"], "1,399.97 ft");
    assert_eq!(r["result"]["precision"], "1:11,525");
    // The compass-adjusted traverse closes exactly and every course is reported.
    let adj = r["result"]["adjusted"].as_array().unwrap();
    assert_eq!(adj.len(), 5);
    let (mut sl, mut sd) = (0.0, 0.0);
    for w in adj.windows(2) {
        sl += num(&w[1], "northing.value") - num(&w[0], "northing.value");
        sd += num(&w[1], "easting.value") - num(&w[0], "easting.value");
    }
    assert!(sl.abs() < 1e-9 && sd.abs() < 1e-9);
    let t = call(
        "survey.cogo.traverse-closure",
        r#"{"courses":[{"direction":"0","distance":300},{"direction":"90","distance":400.02},{"direction":"180","distance":299.95},{"direction":"270.01","distance":400}],"adjustment":"transit"}"#,
    );
    assert!(
        t["result"]["note"]
            .as_str()
            .unwrap()
            .contains("orientation")
    );
    let none = call(
        "survey.cogo.traverse-closure",
        r#"{"courses":[{"direction":"0","distance":1},{"direction":"180","distance":1}],"adjustment":"none"}"#,
    );
    assert!(none["result"].get("adjusted").is_none());
    assert!(none["result"].get("precision_ratio").is_none());
}

#[test]
fn traverse_row_errors_point_at_the_row() {
    let r = call(
        "survey.cogo.traverse-closure",
        r#"{"courses":[{"direction":"0","distance":1},{"direction":"X","distance":1}]}"#,
    );
    assert_eq!(r["error"]["code"], "INVALID_INPUT");
    assert_eq!(r["error"]["field"], "/courses/1/direction", "{r}");
    let r = call(
        "survey.cogo.traverse-closure",
        r#"{"courses":[{"direction":"0","distance":1},{"direction":"90","distance":"1 ftUS"}]}"#,
    );
    assert_eq!(r["error"]["code"], "UNIT_MISMATCH", "{r}");
    let r = call(
        "survey.cogo.traverse-closure",
        r#"{"courses":[{"direction":"0","distance":1}]}"#,
    );
    assert_eq!(r["error"]["code"], "INVALID_INPUT");
    let r = call(
        "survey.cogo.traverse-closure",
        r#"{"courses":[{"direction":"0","distance":1},{"direction":"0","distance":1,"extra":1}]}"#,
    );
    assert_eq!(r["ok"], false);
}

#[test]
fn area_scenarios() {
    let r = call(
        "survey.cogo.area-by-coordinates",
        r#"{"points":[{"northing":0,"easting":0},{"northing":0,"easting":100},{"northing":50,"easting":100},{"northing":50,"easting":0}]}"#,
    );
    assert_eq!(r["display"]["area"], "5,000 ft²");
    assert_eq!(r["display"]["acres"], "0.1148 ac");
    // Large state plane coordinates keep their digits.
    let big = call(
        "survey.cogo.area-by-coordinates",
        r#"{"points":[{"northing":712345678.0,"easting":2345678.0},{"northing":712345678.0,"easting":2345778.0},{"northing":712345728.0,"easting":2345778.0},{"northing":712345728.0,"easting":2345678.0}]}"#,
    );
    assert_eq!(num(&big, "result.area.value"), 5000.0);
    let line = call(
        "survey.cogo.area-by-coordinates",
        r#"{"points":[{"northing":0,"easting":0},{"northing":1,"easting":1},{"northing":2,"easting":2}]}"#,
    );
    assert_eq!(line["error"]["code"], "DEGENERATE_GEOMETRY");
}

#[test]
fn circular_curve_any_two() {
    let base = call(
        "survey.curves.circular-curve",
        r#"{"radius":500,"delta":"30 deg"}"#,
    );
    for (k, want) in [
        ("tangent", 133.975),
        ("length", 261.799),
        ("chord", 258.819),
        ("external", 17.638),
        ("middle_ordinate", 17.037),
    ] {
        assert!(
            (num(&base, &format!("result.{k}.value")) - want).abs() < 5e-4,
            "{k}"
        );
    }
    for pair in [
        ("tangent", "chord"),
        ("length", "external"),
        ("chord", "middle_ordinate"),
        ("radius", "tangent"),
        ("delta", "external"),
    ] {
        let v = |k: &str| {
            if k == "delta" {
                "\"30 deg\"".to_owned()
            } else {
                num(&base, &format!("result.{k}.value")).to_string()
            }
        };
        let r = call(
            "survey.curves.circular-curve",
            &format!(
                r#"{{"{}":{},"{}":{}}}"#,
                pair.0,
                v(pair.0),
                pair.1,
                v(pair.1)
            ),
        );
        assert!(
            (num(&r, "result.radius.value") - 500.0).abs() < 1e-7,
            "{pair:?}: {r}"
        );
    }
    let three = call(
        "survey.curves.circular-curve",
        r#"{"radius":500,"delta":"30 deg","tangent":1}"#,
    );
    assert_eq!(three["error"]["code"], "INVALID_INPUT");
    let impossible = call(
        "survey.curves.circular-curve",
        r#"{"radius":100,"chord":300}"#,
    );
    assert_eq!(impossible["error"]["code"], "INVALID_INPUT");
    let d = call(
        "survey.curves.circular-curve",
        r#"{"degree":"4 deg","delta":"20 deg"}"#,
    );
    assert!((num(&d, "result.radius.value") - 1432.39).abs() < 0.01);
}

#[test]
fn vertical_curve_scenario() {
    let r = call(
        "survey.curves.vertical-curve",
        r#"{"g1":2,"g2":-3,"length":"600 ft","pvi_station":"10+00","pvi_elevation":"100 ft"}"#,
    );
    assert_eq!(r["result"]["pvc_station"], "7+00.00");
    assert_eq!(r["display"]["pvc_elevation"], "94 ft");
    assert_eq!(r["result"]["turning_station"], "9+40.00");
    assert_eq!(r["display"]["turning_elevation"], "96.4 ft");
    assert_eq!(num(&r, "result.k"), 120.0);
    let by_k = call(
        "survey.curves.vertical-curve",
        r#"{"g1":2,"g2":-3,"k":120,"pvi_station":"10+00","pvi_elevation":"100 ft"}"#,
    );
    assert_eq!(by_k["result"]["pvt_station"], "13+00.00");
    let same_sign = call(
        "survey.curves.vertical-curve",
        r#"{"g1":3,"g2":1,"length":"200 ft","pvi_station":"15+00","pvi_elevation":"75 ft"}"#,
    );
    assert!(same_sign["result"].get("turning_station").is_none());
    assert!(same_sign["result"]["turning_note"].is_string());
    let s = same_sign["summary"].as_str().unwrap();
    assert!(!s.contains("point is at"), "{s}");
    let metric = call(
        "survey.curves.vertical-curve",
        r#"{"g1":-1,"g2":1,"length":"200 m","pvi_station":"1+500","pvi_elevation":"50 m"}"#,
    );
    assert_eq!(metric["result"]["pvc_station"], "1+400.000", "{metric}");
}

#[test]
fn earthwork_scenarios() {
    let r = call(
        "survey.earthwork.average-end-area",
        r#"{"area1":"120 ft2","area2":"180 ft2","length":"100 ft"}"#,
    );
    assert_eq!(r["display"]["volume"], "555.56 yd³");
    assert_eq!(r["display"]["volume_ft3"], "15,000 ft³");
    let p = call(
        "survey.earthwork.prismoidal",
        r#"{"area1":"120 ft2","area2":"180 ft2","area_middle":"148 ft2","length":"100 ft"}"#,
    );
    assert_eq!(p["display"]["volume_ft3"], "14,866.7 ft³");
    let s = call(
        "survey.earthwork.shrink-swell",
        r#"{"bank_volume":"1000 yd3","swell":25,"truck_capacity":"12 yd3"}"#,
    );
    assert_eq!(num(&s, "result.loads"), 105.0);
    let exact = call(
        "survey.earthwork.shrink-swell",
        r#"{"bank_volume":"960 yd3","swell":25,"truck_capacity":"12 yd3"}"#,
    );
    assert_eq!(
        num(&exact, "result.loads"),
        100.0,
        "exact multiples do not round up"
    );
}

#[test]
fn combined_factor() {
    let r = call(
        "survey.reduction.combined-factor",
        r#"{"grid_scale":0.99991,"elevation":"1530 ft","geoid_height":"-30 ft","grid_distance":"999.838 ft"}"#,
    );
    let ef = 6_372_000.0 / (6_372_000.0 + 1500.0 * 0.3048);
    assert!((num(&r, "result.elevation_factor") - ef).abs() < 1e-15);
    assert!((num(&r, "result.ground_distance.value") - 999.838 / (0.99991 * ef)).abs() < 1e-9);
    assert!(codes(&r).iter().all(|c| c != "ORTHOMETRIC_AS_ELLIPSOIDAL"));
    let both = call(
        "survey.reduction.combined-factor",
        r#"{"grid_scale":1,"ellipsoid_height":0,"grid_distance":1,"ground_distance":1}"#,
    );
    assert_eq!(both["error"]["code"], "INVALID_INPUT");
}

#[test]
fn traverse_closure_invariants() {
    // Misclosure is the length of (Σlat, Σdep); rotating every course by the
    // same angle leaves misclosure and precision unchanged; reversing the
    // loop negates the sums; and after a compass or transit adjustment the
    // loop returns exactly to its start.
    let loops: [&[(f64, f64)]; 3] = [
        &[
            (0.0, 300.0),
            (90.0, 400.02),
            (180.0, 299.95),
            (270.01, 400.0),
        ],
        &[
            (12.25, 250.0),
            (95.5, 310.2),
            (170.75, 260.4),
            (281.0, 290.1),
        ],
        &[
            (60.0, 45.5),
            (140.0, 60.2),
            (230.0, 70.1),
            (320.0, 50.3),
            (355.0, 20.0),
        ],
    ];
    let run = |courses: &[(f64, f64)], method: &str| {
        let c: Vec<Value> = courses
            .iter()
            .map(|(a, d)| serde_json::json!({"direction": a.rem_euclid(360.0).to_string(), "distance": d}))
            .collect();
        call(
            "survey.cogo.traverse-closure",
            &serde_json::json!({"courses": c, "adjustment": method}).to_string(),
        )
    };
    for courses in loops {
        let base = run(courses, "compass");
        let (sl, sd) = (
            num(&base, "result.sum_latitudes.value"),
            num(&base, "result.sum_departures.value"),
        );
        let mis = num(&base, "result.misclosure.value");
        assert!((mis - sl.hypot(sd)).abs() < 1e-12);
        for rot in [17.0, 123.5, 271.25] {
            let turned: Vec<(f64, f64)> = courses.iter().map(|(a, d)| (a + rot, *d)).collect();
            let r = run(&turned, "compass");
            assert!((num(&r, "result.misclosure.value") - mis).abs() < 1e-9);
            assert!(
                (num(&r, "result.precision_ratio") - num(&base, "result.precision_ratio")).abs()
                    < 1e-6
            );
        }
        let back: Vec<(f64, f64)> = courses.iter().rev().map(|(a, d)| (a + 180.0, *d)).collect();
        let r = run(&back, "compass");
        assert!((num(&r, "result.sum_latitudes.value") + sl).abs() < 1e-9);
        assert!((num(&r, "result.sum_departures.value") + sd).abs() < 1e-9);
        for method in ["compass", "transit"] {
            let r = run(courses, method);
            let pts = r["result"]["adjusted"].as_array().unwrap();
            let (first, last) = (&pts[0], &pts[pts.len() - 1]);
            for k in ["northing", "easting"] {
                let gap = first[k]["value"].as_f64().unwrap() - last[k]["value"].as_f64().unwrap();
                assert!(gap.abs() < 1e-9, "{method} {k} gap {gap}");
            }
        }
    }
}

#[test]
fn combined_factor_invariants() {
    // Combined = grid scale × elevation factor; grid and ground distances
    // invert each other; h = H + N gives the same factor as the ellipsoid
    // height; and the elevation factor falls as the height rises.
    let mut prev = f64::INFINITY;
    for h in [-100.0, 0.0, 250.0, 1000.0, 3000.0, 4500.0] {
        let e = call(
            "survey.reduction.combined-factor",
            &format!(
                r#"{{"grid_scale":0.99993,"ellipsoid_height":"{h} m","ground_distance":"1000 m"}}"#
            ),
        );
        let ef = num(&e, "result.elevation_factor");
        assert!(ef < prev);
        prev = ef;
        let cf = num(&e, "result.combined_factor");
        assert!((cf - 0.99993 * ef).abs() < 1e-15);
        let grid = num(&e, "result.grid_distance.value");
        let back = call(
            "survey.reduction.combined-factor",
            &format!(
                r#"{{"grid_scale":0.99993,"ellipsoid_height":"{h} m","grid_distance":"{grid} m"}}"#
            ),
        );
        assert!((num(&back, "result.ground_distance.value") - 1000.0).abs() < 1e-9);
        let split = call(
            "survey.reduction.combined-factor",
            &format!(
                r#"{{"grid_scale":0.99993,"elevation":"{} m","geoid_height":"-30 m","ground_distance":"1000 m"}}"#,
                h + 30.0
            ),
        );
        assert!((num(&split, "result.combined_factor") - cf).abs() < 1e-15);
    }
}

#[test]
fn circular_curve_invariants() {
    // Any two independent elements give back the same curve, and the arc and
    // chord degrees of curve fix R = 5,729.578/D and R = 50/sin(D/2).
    let base = call(
        "survey.curves.circular-curve",
        r#"{"radius":"850 ft","delta":"37.5 deg"}"#,
    );
    let names = ["tangent", "length", "chord", "external", "middle_ordinate"];
    for (i, a) in names.iter().enumerate() {
        for b in &names[i + 1..] {
            let r = call(
                "survey.curves.circular-curve",
                &serde_json::json!({*a: format!("{} ft", num(&base, &format!("result.{a}.value"))),
                                    *b: format!("{} ft", num(&base, &format!("result.{b}.value")))})
                .to_string(),
            );
            assert!(
                (num(&r, "result.radius.value") - 850.0).abs() < 1e-6,
                "{a}+{b}: {r}"
            );
            assert!(
                (num(&r, "result.delta.value") - 37.5).abs() < 1e-9,
                "{a}+{b}"
            );
            // T and M fit a second, sharper curve too (their ratio has a minimum near Δ = 104°).
            let ambiguous = codes(&r).contains(&"AMBIGUOUS_INPUT".to_owned());
            assert_eq!(
                ambiguous,
                (*a, *b) == ("tangent", "middle_ordinate"),
                "{a}+{b}"
            );
        }
    }
    for d in [1.0, 4.5, 15.0, 30.0] {
        let arc = call(
            "survey.curves.circular-curve",
            &format!(r#"{{"degree":"{d} deg","delta":"20 deg"}}"#),
        );
        assert!(
            (num(&arc, "result.radius.value") - 18_000.0 / std::f64::consts::PI / d).abs() < 1e-9
        );
        let chord = call(
            "survey.curves.circular-curve",
            &format!(r#"{{"degree_chord":"{d} deg","delta":"20 deg"}}"#),
        );
        let r = num(&chord, "result.radius.value");
        assert!((r - 50.0 / (d / 2.0).to_radians().sin()).abs() < 1e-9);
        assert!((num(&chord, "result.degree_chord.value") - d).abs() < 1e-9);
    }
}

#[test]
fn vertical_curve_invariants() {
    // The PVC and PVT lie on the tangents, K = L/|g2 − g1|, and the turning
    // point is the extreme: the curve is no lower (sag) or higher (crest) at
    // points 1 ft either side.
    for (g1, g2, l) in [
        (-1.75, 2.25, 500.0),
        (3.0, -2.0, 800.0),
        (-4.0, 1.0, 350.0),
        (0.5, -3.5, 1000.0),
    ] {
        let r = call(
            "survey.curves.vertical-curve",
            &format!(
                r#"{{"g1":{g1},"g2":{g2},"length":"{l} ft","pvi_station":"50+00","pvi_elevation":"1000 ft"}}"#
            ),
        );
        let pvc = num(&r, "result.pvc_elevation.value");
        assert!((pvc - (1000.0 - g1 / 100.0 * l / 2.0)).abs() < 1e-9);
        assert!(
            (num(&r, "result.pvt_elevation.value") - (1000.0 + g2 / 100.0 * l / 2.0)).abs() < 1e-9
        );
        assert!((num(&r, "result.k") - l / (g2 - g1).abs()).abs() < 1e-9);
        let y = |x: f64| pvc + g1 / 100.0 * x + (g2 - g1) / 100.0 * x * x / (2.0 * l);
        let x = -g1 / 100.0 * l / ((g2 - g1) / 100.0);
        let turn = num(&r, "result.turning_elevation.value");
        assert!((turn - y(x)).abs() < 1e-9);
        let sag = g2 > g1;
        for dx in [-1.0, 1.0] {
            assert!(if sag {
                y(x + dx) >= turn
            } else {
                y(x + dx) <= turn
            });
        }
    }
}

#[test]
fn area_by_coordinates_invariants() {
    // Area does not change when the parcel is shifted, rotated, or started
    // at another corner; reversing the order flips only the orientation.
    let p = [
        (100.0, 100.0),
        (340.0, 180.0),
        (520.0, 90.0),
        (610.0, 400.0),
        (300.0, 520.0),
        (80.0, 330.0),
    ];
    let area = |pts: &[(f64, f64)]| {
        let v: Vec<Value> = pts
            .iter()
            .map(|(n, e)| serde_json::json!({"northing": n, "easting": e}))
            .collect();
        let r = call(
            "survey.cogo.area-by-coordinates",
            &serde_json::json!({"points": v}).to_string(),
        );
        (
            num(&r, "result.area.value"),
            r["result"]["orientation"].as_str().unwrap().to_owned(),
        )
    };
    let (a, o) = area(&p);
    let shifted: Vec<_> = p
        .iter()
        .map(|(n, e)| (n + 1_234_567.0, e - 765_432.0))
        .collect();
    assert!((area(&shifted).0 - a).abs() < 1e-6 * a);
    let (s, c) = (0.6_f64.sin(), 0.6_f64.cos());
    let turned: Vec<_> = p
        .iter()
        .map(|(n, e)| (n * c - e * s, n * s + e * c))
        .collect();
    assert!((area(&turned).0 - a).abs() < 1e-9 * a);
    let mut rolled = p.to_vec();
    rolled.rotate_left(2);
    assert_eq!(area(&rolled), (a, o.clone()));
    let mut rev = p.to_vec();
    rev.reverse();
    let (ra, ro) = area(&rev);
    assert!((ra - a).abs() < 1e-9 && ro != o);
}

#[test]
fn circular_curve_extreme_angles() {
    // Pairs other than R and Δ still solve at very flat and very sharp curves.
    for delta in [0.1, 0.2, 179.8, 179.9] {
        let base = call(
            "survey.curves.circular-curve",
            &format!(r#"{{"radius":"1000 ft","delta":"{delta} deg"}}"#),
        );
        let (t, c) = (
            num(&base, "result.tangent.value"),
            num(&base, "result.chord.value"),
        );
        let r = call(
            "survey.curves.circular-curve",
            &format!(r#"{{"tangent":"{t} ft","chord":"{c} ft"}}"#),
        );
        assert!(
            (num(&r, "result.delta.value") - delta).abs() < 1e-6,
            "{delta}: {r}"
        );
    }
}
