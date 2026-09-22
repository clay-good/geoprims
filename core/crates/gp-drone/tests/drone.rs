//! Drone photogrammetry: catalog lint, examples, vectors, and spec scenarios.

use std::path::Path;

use gp_base::{manifest, template, vectors};
use gp_drone::{REGISTRY, TOOLS};
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

const CAM: &str = r#""sensor_width":"13.2 mm","sensor_height":"8.8 mm","focal_length":"8.8 mm","image_width":5472,"image_height":3648"#;

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
    let mut failures = manifest::lint(TOOLS, &taxonomy, &[]);
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
fn one_inch_at_100_m() {
    let r = call(
        "drone.photogrammetry.gsd",
        &format!(r#"{{"height":"100 m",{CAM}}}"#),
    );
    assert!((num(&r, "result.gsd.value") - 2.741).abs() < 5e-4);
    assert!((num(&r, "result.footprint_across.value") - 150.0).abs() < 1e-9);
    assert!(codes(&r).iter().all(|c| c != "EQUIVALENT_FOCAL_LENGTH"));
}

#[test]
fn equivalent_focal_length_suspected() {
    let r = call(
        "drone.photogrammetry.gsd",
        r#"{"height":"100 m","sensor_width":"13.2 mm","sensor_height":"8.8 mm","focal_length":"24 mm","image_width":5472}"#,
    );
    assert!(codes(&r).contains(&"EQUIVALENT_FOCAL_LENGTH".to_owned()));
    let fixed = call(
        "drone.photogrammetry.gsd",
        r#"{"height":"100 m","sensor_width":"13.2 mm","sensor_height":"8.8 mm","focal_length":"24 mm","focal_length_type":"equivalent-35mm","image_width":5472}"#,
    );
    assert!(
        (num(&fixed, "result.focal_length_used.value") - 24.0 * 15.864_425_6 / 43.266_615_3).abs()
            < 1e-3
    );
}

#[test]
fn two_cm_target() {
    let r = call(
        "drone.photogrammetry.altitude-for-gsd",
        &format!(r#"{{"target_gsd":"2 cm",{CAM}}}"#),
    );
    assert!((num(&r, "result.height.value") - 72.96).abs() < 1e-9);
    let high = call(
        "drone.photogrammetry.altitude-for-gsd",
        &format!(r#"{{"target_gsd":"8 cm",{CAM}}}"#),
    );
    assert!(codes(&high).contains(&"ABOVE_ALTITUDE_CEILING".to_owned()));
}

#[test]
fn overlap_75_65_and_camera_too_slow() {
    let base = r#""height":"100 m","sensor_width":"13.2 mm","sensor_height":"8.8 mm","focal_length":"8.8 mm","image_width":5472,"groundspeed":"10 m/s","front_overlap":75,"side_overlap":65"#;
    let r = call("drone.photogrammetry.trigger", &format!("{{{base}}}"));
    assert!((num(&r, "result.trigger_distance.value") - 25.0).abs() < 1e-9);
    assert!((num(&r, "result.trigger_interval.value") - 2.5).abs() < 1e-9);
    assert!((num(&r, "result.line_spacing.value") - 52.5).abs() < 1e-9);
    let slow = call(
        "drone.photogrammetry.trigger",
        &format!(r#"{{{base},"min_interval":"3 s"}}"#),
    );
    assert!(codes(&slow).contains(&"TRIGGER_TOO_FAST".to_owned()));
    assert!((num(&slow, "result.max_groundspeed.value") - 8.333_333).abs() < 1e-5);
}

#[test]
fn forest_preset() {
    let r = call(
        "drone.photogrammetry.trigger",
        r#"{"height":"100 m","sensor_width":"13.2 mm","sensor_height":"8.8 mm","focal_length":"8.8 mm","image_width":5472,"groundspeed":"10 m/s","preset":"forest"}"#,
    );
    assert!((num(&r, "result.trigger_distance.value") - 15.0).abs() < 1e-9);
    assert!((num(&r, "result.line_spacing.value") - 45.0).abs() < 1e-9);
}

#[test]
fn blur_at_one_thousandth() {
    let r = call(
        "drone.photogrammetry.motion-blur",
        r#"{"groundspeed":"10 m/s","exposure":"0.001 s","gsd":"2.741 cm"}"#,
    );
    assert!((num(&r, "result.blur") - 0.365).abs() < 5e-4);
    assert_eq!(r["result"]["max_exposure_fraction"], "1/730 s");
}

#[test]
fn asprs_scenarios() {
    let r = call(
        "drone.photogrammetry.asprs-accuracy",
        r#"{"rmse_z":"1.00 cm","checkpoint_rmse":"2.0 cm","checkpoints":30}"#,
    );
    assert!((num(&r, "result.vertical.value") - 2.24).abs() < 0.005);
    let few = call(
        "drone.photogrammetry.asprs-accuracy",
        r#"{"rmse_z":"1 cm","checkpoint_rmse":"1 cm","checkpoints":20}"#,
    );
    assert!(codes(&few).contains(&"INSUFFICIENT_CHECKPOINTS".to_owned()));
    assert!(
        few["result"]["checkpoint_status"]
            .as_str()
            .unwrap()
            .contains("30")
    );
}

#[test]
fn gsd_invariants() {
    // GSD is linear in height, the footprint is GSD × pixels, and the height
    // for a target GSD inverts the GSD tool exactly.
    let cams = [
        (13.2, 8.8, 8.8, 5472, 3648),
        (17.3, 13.0, 12.29, 5280, 3956),
        (6.17, 4.55, 4.5, 4000, 3000),
        (35.9, 24.0, 35.0, 8192, 5460),
    ];
    let v = |r: &Value, k: &str| num(r, &format!("result.{k}.value"));
    for (w, h, f, iw, ih) in cams {
        let cam = serde_json::json!({"sensor_width": format!("{w} mm"), "sensor_height": format!("{h} mm"),
            "focal_length": format!("{f} mm"), "image_width": iw, "image_height": ih});
        let gsd = |m: f64| {
            let mut i = cam.clone();
            i["height"] = serde_json::json!(format!("{m} m"));
            call("drone.photogrammetry.gsd", &i.to_string())
        };
        for m in [10.0, 45.0, 100.0, 120.0, 400.0] {
            let r = gsd(m);
            let g = v(&r, "gsd");
            assert!((v(&gsd(2.0 * m), "gsd") - 2.0 * g).abs() < 1e-12 * g.max(1.0));
            assert!((v(&r, "footprint_across") - g / 100.0 * f64::from(iw)).abs() < 1e-9);
            assert!(
                (v(&r, "footprint_along") - v(&r, "gsd_along") / 100.0 * f64::from(ih)).abs()
                    < 1e-9
            );
            let mut i = cam.clone();
            i["target_gsd"] = serde_json::json!(format!("{g} cm"));
            let back = call("drone.photogrammetry.altitude-for-gsd", &i.to_string());
            assert!((v(&back, "height") - m).abs() < 1e-9 * m, "{back}");
        }
    }
}

#[test]
fn oblique_45_degrees_is_coarser_than_nadir_and_a_trapezoid() {
    let r = call(
        "drone.photogrammetry.oblique-gsd",
        &format!(r#"{{{CAM},"height":"100 m","pitch":"45 deg"}}"#),
    );
    let (c, n) = (
        num(&r, "result.gsd_center.value"),
        num(&r, "result.gsd_nadir.value"),
    );
    assert!(c > n, "center {c} should be coarser than nadir {n}");
    // Across at the center is nadir ÷ cos θ; along, ÷ cos² θ.
    assert!((c - n * 2f64.sqrt()).abs() < 1e-9);
    assert!(num(&r, "result.gsd_center_along.value") > c);
    let fp = &r["result"]["footprint"];
    let x = |i: usize| fp[i]["right"]["value"].as_f64().unwrap();
    let w = |a: usize, b: usize| x(b) - x(a);
    assert!(w(3, 2) > w(0, 1), "the far edge is wider: a trapezoid");
    assert!(codes(&r).iter().all(|c| c != "BEYOND_HORIZON"));
}

#[test]
fn hill_reduces_overlap() {
    let r = call(
        "drone.photogrammetry.terrain-overlap",
        r#"{"height":"100 m","highest_terrain":"40 m","front_overlap":75}"#,
    );
    assert_eq!(num(&r, "result.effective_height.value"), 60.0);
    let f = num(&r, "result.front_overlap_worst");
    assert!((f - 58.333_333_333).abs() < 1e-6, "{f}");
    assert!(codes(&r).iter().any(|c| c == "OVERLAP_BELOW_TARGET"));
    let msg = r["meta"]["warnings"][1]["message"].as_str().unwrap();
    assert!(msg.starts_with("Front overlap falls to 58.3%"), "{msg}");
}
