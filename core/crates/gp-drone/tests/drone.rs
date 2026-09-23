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
    let mut failures = manifest::lint(
        TOOLS,
        &taxonomy,
        &["navigation.los.fresnel", "geodesy.geoid.geoid-height"],
    );
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
    let msg = r["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["code"] == "OVERLAP_BELOW_TARGET")
        .and_then(|w| w["message"].as_str())
        .unwrap();
    assert!(msg.starts_with("Front overlap falls to 58.3%"), "{msg}");
}

#[test]
fn a_vanishing_focal_length_is_refused_not_a_crash() {
    // Found by the fuzzer: a 1e-12 mm focal length trapped the module.
    let r = call(
        "drone.photogrammetry.oblique-gsd",
        r#"{"focal_length":1e-12,"height":"100 m","image_height":3648,"image_width":5472,"pitch":"45 deg","sensor_width":"13.2 mm"}"#,
    );
    assert_eq!(r["ok"], false, "{r}");
    assert_ne!(r["error"]["code"], "INTERNAL", "{r}");
}

#[test]
fn trigger_invariants() {
    // Spacing is the footprint times (1 − overlap), the footprint is the GSD
    // tool's, everything scales linearly with height, the interval times the
    // groundspeed is the trigger distance, portrait swaps the two axes, and
    // the camera check fires exactly when the interval is under the minimum.
    let cams = [
        (13.2, 8.8, 8.8, 5472),
        (17.3, 13.0, 12.29, 5280),
        (6.17, 4.55, 4.5, 4000),
        (35.9, 24.0, 35.0, 8192),
    ];
    let v = |r: &Value, k: &str| num(r, &format!("result.{k}.value"));
    let close = |a: f64, b: f64| (a - b).abs() <= 1e-9 * b.abs().max(1.0);
    for (w, h, f, iw) in cams {
        let run = |m: f64, gs: f64, fo: f64, so: f64, extra: &str| {
            call(
                "drone.photogrammetry.trigger",
                &format!(
                    r#"{{"sensor_width":"{w} mm","sensor_height":"{h} mm","focal_length":"{f} mm","image_width":{iw},"height":"{m} m","groundspeed":"{gs} m/s","front_overlap":{fo},"side_overlap":{so}{extra}}}"#
                ),
            )
        };
        for m in [20.0, 60.0, 100.0, 120.0] {
            let gsd = call(
                "drone.photogrammetry.gsd",
                &format!(
                    r#"{{"sensor_width":"{w} mm","sensor_height":"{h} mm","focal_length":"{f} mm","image_width":{iw},"height":"{m} m"}}"#
                ),
            );
            for (fo, so) in [(0.0, 0.0), (60.0, 30.0), (75.0, 65.0), (85.0, 70.0)] {
                let r = run(m, 10.0, fo, so, "");
                let (along, across) = (v(&r, "footprint_along"), v(&r, "footprint_across"));
                assert!(close(across, v(&gsd, "footprint_across")), "{r}");
                assert!(close(along, v(&gsd, "footprint_along")), "{r}");
                assert!(close(v(&r, "trigger_distance"), along * (1.0 - fo / 100.0)));
                assert!(close(v(&r, "line_spacing"), across * (1.0 - so / 100.0)));
                assert!(close(
                    v(&r, "trigger_interval") * 10.0,
                    v(&r, "trigger_distance")
                ));
                // Linear in height, inverse in groundspeed.
                let hi = run(2.0 * m, 10.0, fo, so, "");
                assert!(close(
                    v(&hi, "trigger_distance"),
                    2.0 * v(&r, "trigger_distance")
                ));
                assert!(close(v(&hi, "line_spacing"), 2.0 * v(&r, "line_spacing")));
                let fast = run(m, 20.0, fo, so, "");
                assert!(close(
                    2.0 * v(&fast, "trigger_interval"),
                    v(&r, "trigger_interval")
                ));
                // Portrait: the long side runs along the track.
                let p = run(m, 10.0, fo, so, r#","orientation":"portrait""#);
                assert!(close(v(&p, "footprint_along"), across));
                assert!(close(v(&p, "footprint_across"), along));
                // The camera check: max speed × minimum interval = distance.
                let t = v(&r, "trigger_interval");
                for min in [0.5 * t, 2.0 * t] {
                    let c = run(m, 10.0, fo, so, &format!(r#","min_interval":"{min} s""#));
                    assert!(close(
                        v(&c, "max_groundspeed") * min,
                        v(&r, "trigger_distance")
                    ));
                    let warned = codes(&c).iter().any(|x| x == "TRIGGER_TOO_FAST");
                    assert_eq!(warned, t < min, "{c}");
                }
            }
            // More overlap never spreads photos or lines further apart.
            let mut last = (f64::INFINITY, f64::INFINITY);
            for o in [0.0, 30.0, 50.0, 60.0, 70.0, 80.0, 90.0, 99.0] {
                let r = run(m, 10.0, o, o, "");
                let now = (v(&r, "trigger_distance"), v(&r, "line_spacing"));
                assert!(now.0 < last.0 && now.1 < last.1);
                last = now;
            }
        }
    }
}

#[test]
fn terrain_overlap_invariants() {
    // Flat ground keeps the plan, rising ground only lowers overlap, the
    // result is the trigger tool's spacing over the footprint at h − t, it is
    // scale-free in (h, t), the GSD shrinks by (h − t)/h, and flying at the
    // reported height holds the accepted overlap exactly.
    let v = |r: &Value, k: &str| num(r, &format!("result.{k}.value"));
    let pct = |r: &Value, k: &str| num(r, &format!("result.{k}"));
    let close = |a: f64, b: f64| (a - b).abs() <= 1e-9 * b.abs().max(1.0);
    let cam = r#""sensor_width":"13.2 mm","focal_length":"8.8 mm","image_width":5472"#;
    let run = |h: f64, t: f64, fo: f64, so: f64, extra: &str| {
        call(
            "drone.photogrammetry.terrain-overlap",
            &format!(
                r#"{{"height":"{h} m","highest_terrain":"{t} m","front_overlap":{fo},"side_overlap":{so}{extra}}}"#
            ),
        )
    };
    for (fo, so) in [(60.0, 30.0), (75.0, 65.0), (80.0, 70.0), (85.0, 75.0)] {
        for h in [45.0, 100.0, 120.0] {
            let flat = run(h, 0.0, fo, so, "");
            assert!(close(pct(&flat, "front_overlap_worst"), fo));
            assert!(close(pct(&flat, "side_overlap_worst"), so));
            assert!(codes(&flat).iter().all(|c| c != "OVERLAP_BELOW_TARGET"));
            let plan = call(
                "drone.photogrammetry.trigger",
                &format!(
                    r#"{{{cam},"sensor_height":"8.8 mm","height":"{h} m","groundspeed":"10 m/s","front_overlap":{fo},"side_overlap":{so}}}"#
                ),
            );
            let mut last = (f64::INFINITY, f64::INFINITY);
            for k in [-0.4, -0.1, 0.1, 0.3, 0.5, 0.7, 0.9] {
                let t = k * h;
                let r = run(h, t, fo, so, "");
                let now = (
                    pct(&r, "front_overlap_worst"),
                    pct(&r, "side_overlap_worst"),
                );
                assert!(now.0 < last.0 && now.1 < last.1, "{r}");
                last = now;
                assert_eq!(
                    codes(&r).iter().any(|c| c == "OVERLAP_BELOW_TARGET"),
                    t > 0.0,
                    "{r}"
                );
                // The same answer from the trigger tool's spacing at h and the
                // GSD tool's footprint at h − t.
                let there = call(
                    "drone.photogrammetry.gsd",
                    &format!(
                        r#"{{{cam},"sensor_height":"8.8 mm","height":"{} m"}}"#,
                        h - t
                    ),
                );
                let o = |spacing: f64, fp: f64| 100.0 * (1.0 - spacing / fp);
                let front = o(v(&plan, "trigger_distance"), v(&there, "footprint_along"));
                let side = o(v(&plan, "line_spacing"), v(&there, "footprint_across"));
                assert!((now.0 - front).abs() < 1e-9, "{now:?} {front}");
                assert!((now.1 - side).abs() < 1e-9, "{now:?} {side}");
                // Scale-free: the same ratio of terrain to height, the same overlap.
                let big = run(3.0 * h, 3.0 * t, fo, so, "");
                assert!(close(pct(&big, "front_overlap_worst"), now.0));
                // The GSD over the high ground shrinks by (h − t)/h.
                let g = run(h, t, fo, so, &format!(",{cam}"));
                assert!(close(
                    v(&g, "gsd_worst"),
                    v(&g, "gsd_takeoff") * (h - t) / h
                ));
            }
            // Flying at the reported height holds the accepted overlap.
            let t = 0.3 * h;
            let target = so - 10.0;
            let r = run(h, t, fo, so, &format!(r#","target_overlap":{target}"#));
            let up = v(&r, "min_height");
            let again = run(up, t, fo, so, "");
            let worst = pct(&again, "front_overlap_worst").min(pct(&again, "side_overlap_worst"));
            assert!((worst - target).abs() < 1e-9, "{again}");
        }
    }
}
