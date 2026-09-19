//! Aviation slice 1: catalog lint, examples, golden vectors, and every spec
//! scenario from the atmosphere, altimetry, and wind specs.

use std::path::Path;

use gp_aviation::{REGISTRY, TOOLS};
use gp_base::{manifest, template, vectors};
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

#[test]
fn catalog_lint_and_registry() {
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
    let known = ["units.pressure.inhg-to-hpa"];
    let errs = manifest::lint(TOOLS, &taxonomy, &known);
    assert!(errs.is_empty(), "{}", errs.join("\n"));
    let codes: Value = serde_json::from_str(&repo("data/codes.json")).unwrap();
    for t in TOOLS {
        for w in t.warnings {
            assert!(
                codes["warnings"].get(*w).is_some(),
                "{} declares unregistered {w}",
                t.id
            );
        }
    }
}

#[test]
fn examples_run_with_readable_summaries() {
    for t in TOOLS {
        for ex in t.examples {
            let r = call(t.id, ex.input);
            assert_eq!(r["ok"], true, "{}: {r}", t.id);
            let s = r["summary"].as_str().unwrap();
            assert!(
                s.len() <= template::MAX_CHARS && template::grade(s) <= 8.0,
                "{}: {s} (grade {:.1})",
                t.id,
                template::grade(s)
            );
        }
    }
}

#[test]
fn golden_vectors() {
    let mut failures = Vec::new();
    for t in TOOLS {
        let text = repo(&format!("core/vectors/{}.jsonl", t.id));
        failures.extend(vectors::lint(t.id, &text));
        failures.extend(vectors::run(&REGISTRY, t.id, &text));
        if vectors::count(&text) < 5 {
            failures.push(format!("{} has fewer than 5 vectors", t.id));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// atmosphere

#[test]
fn isa_10000_ft() {
    let r = call(
        "aviation.atmosphere.isa",
        r#"{"altitude":"10000 ft","options":{"outputUnits":{"temperature":"K","pressure":"Pa"}}}"#,
    );
    assert!((num(&r, "result.temperature.value") - 268.338).abs() < 1e-3);
    assert!((num(&r, "result.pressure.value") - 69_681.6).abs() < 0.1);
    assert!((num(&r, "result.density.value") / 0.904_637 - 1.0).abs() < 1e-4);
}

#[test]
fn isa_tropopause_names_layer() {
    let r = call(
        "aviation.atmosphere.isa",
        r#"{"altitude":"11000 m","options":{"outputUnits":{"temperature":"K","pressure":"Pa"}}}"#,
    );
    assert_eq!(num(&r, "result.temperature.value"), 216.65);
    assert!((num(&r, "result.pressure.value") - 22_632.0).abs() < 0.5);
    assert_eq!(r["result"]["layer"], "tropopause (isothermal)");
}

#[test]
fn isa_geometric_input_shows_both() {
    let r = call(
        "aviation.atmosphere.isa",
        r#"{"altitude":"11000 m","altitude_type":"geometric","options":{"outputUnits":{"geopotential_altitude":"m","geometric_altitude":"m"}}}"#,
    );
    assert!((num(&r, "result.geopotential_altitude.value") - 10_981.0).abs() < 0.5);
    assert_eq!(num(&r, "result.geometric_altitude.value"), 11_000.0);
}

#[test]
fn isa_above_model_hints_us76() {
    let r = call("aviation.atmosphere.isa", r#"{"altitude":"90 km"}"#);
    assert_eq!(r["error"]["code"], "OUT_OF_DOMAIN");
    assert!(r["error"]["hint"].as_str().unwrap().contains("86 km"));
    let r = call(
        "aviation.atmosphere.isa",
        r#"{"altitude":"86 km","altitude_type":"geometric","model":"us76","options":{"outputUnits":{"temperature":"K"}}}"#,
    );
    assert!(
        (num(&r, "result.temperature.value") - 186.87).abs() < 0.01,
        "{r}"
    );
}

#[test]
fn isa_plus_20() {
    let std = call(
        "aviation.atmosphere.isa",
        r#"{"altitude":"5000 ft","options":{"outputUnits":{"temperature":"K"}}}"#,
    );
    let hot = call(
        "aviation.atmosphere.isa",
        r#"{"altitude":"5000 ft","temperature_deviation":"+20 degC","options":{"outputUnits":{"temperature":"K"}}}"#,
    );
    assert_eq!(
        num(&hot, "result.pressure.value"),
        num(&std, "result.pressure.value")
    );
    assert!(
        (num(&hot, "result.temperature.value") - num(&std, "result.temperature.value") - 20.0)
            .abs()
            < 1e-9
    );
    assert!(num(&hot, "result.density.value") < num(&std, "result.density.value"));
}

// altimetry

#[test]
fn pressure_altitude_5000_ft() {
    let r = call(
        "aviation.altimetry.pressure-altitude",
        r#"{"elevation":"5000 ft","altimeter":"29.80 inHg"}"#,
    );
    assert!((num(&r, "result.pressure_altitude.value") - 5108.0).abs() <= 1.0);
    assert!((num(&r, "result.rule_of_thumb.value") - 5120.0).abs() < 1e-6);
}

#[test]
fn pressure_altitude_standard_setting() {
    let r = call(
        "aviation.altimetry.pressure-altitude",
        r#"{"elevation":"0 ft","altimeter":"1013.25 hPa"}"#,
    );
    assert!(num(&r, "result.pressure_altitude.value").abs() <= 0.1);
}

#[test]
fn metar_altimeter_groups() {
    let a = call(
        "aviation.altimetry.pressure-altitude",
        r#"{"elevation":"5000 ft","altimeter":"A2980"}"#,
    );
    let b = call(
        "aviation.altimetry.pressure-altitude",
        r#"{"elevation":"5000 ft","altimeter":"29.80 inHg"}"#,
    );
    assert_eq!(a["result"], b["result"]);
    let q = call(
        "aviation.altimetry.pressure-altitude",
        r#"{"elevation":"0 ft","altimeter":"Q1009"}"#,
    );
    let h = call(
        "aviation.altimetry.pressure-altitude",
        r#"{"elevation":"0 ft","altimeter":"1009 hPa"}"#,
    );
    assert_eq!(q["result"], h["result"]);
    let odd = call(
        "aviation.altimetry.pressure-altitude",
        r#"{"elevation":"0 ft","altimeter":"25.10 inHg"}"#,
    );
    assert!(
        odd["meta"]["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["code"] == "SUSPECT_VALUE")
    );
}

#[test]
fn density_altitude_hot_high_and_sentence() {
    let r = call(
        "aviation.altimetry.density-altitude",
        r#"{"elevation":"5000 ft","altimeter":"29.80 inHg","temperature":"30 degC"}"#,
    );
    assert!((num(&r, "result.pressure_altitude.value") - 5108.0).abs() <= 1.0);
    assert!((num(&r, "result.isa_temperature.value") - 4.88).abs() < 0.01);
    assert!((num(&r, "result.density_altitude.value") - 7932.0).abs() <= 5.0);
    assert!((num(&r, "result.rule_118_8.value") - 8093.0).abs() <= 1.0);
    assert!((num(&r, "result.rule_118_8_error.value") - 161.0).abs() <= 1.0);
    assert!((num(&r, "result.rule_120.value") - 8123.0).abs() <= 1.0);
    assert_eq!(
        r["summary"],
        "Density altitude is 7,932 ft, about 2,900 ft higher than the field. Expect a longer takeoff roll and weaker climb. Assumes dry air."
    );
    assert_eq!(r["display"]["density_altitude"], "7,932 ft");
}

#[test]
fn humidity_raises_density_altitude() {
    let dry = call(
        "aviation.altimetry.density-altitude",
        r#"{"elevation":"5000 ft","altimeter":"29.80 inHg","temperature":"30 degC"}"#,
    );
    let wet = call(
        "aviation.altimetry.density-altitude",
        r#"{"elevation":"5000 ft","altimeter":"29.80 inHg","temperature":"30 degC","dew_point":"20 degC"}"#,
    );
    assert!(
        num(&wet, "result.density_altitude.value") > num(&dry, "result.density_altitude.value")
    );
    assert!(!wet["summary"].as_str().unwrap().contains("dry air"));
    let bad = call(
        "aviation.altimetry.density-altitude",
        r#"{"elevation":"0 ft","altimeter":"29.92","temperature":"10 degC","dew_point":"12 degC"}"#,
    );
    assert_eq!(bad["error"]["field"], "/dew_point");
}

#[test]
fn isa_temperature_fl410() {
    let r = call(
        "aviation.altimetry.isa-temperature",
        r#"{"pressure_altitude":"41000 ft"}"#,
    );
    assert_eq!(num(&r, "result.isa_temperature.value"), -56.5);
}

// wind

#[test]
fn runway_27_right_crosswind_and_gust() {
    let r = call(
        "aviation.wind.runway-components",
        r#"{"runway":"27","wind_direction":300,"wind_speed":15,"gust":25}"#,
    );
    assert!((num(&r, "result.headwind.value") - 12.990_381).abs() < 1e-5);
    assert!((num(&r, "result.crosswind.value") - 7.5).abs() < 1e-9);
    assert_eq!(r["result"]["crosswind_from"], "right");
    assert!((num(&r, "result.gust_crosswind.value") - 12.5).abs() < 1e-9);
    assert!(
        r["meta"]["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["code"] == "RUNWAY_HEADING_APPROXIMATE")
    );
}

#[test]
fn crosswind_beyond_limit() {
    let r = call(
        "aviation.wind.runway-components",
        r#"{"runway":"27","wind_direction":360,"wind_speed":20,"max_crosswind":"15 kt"}"#,
    );
    assert_eq!(
        r["result"]["crosswind_status"],
        "Beyond your 15 kt crosswind limit"
    );
    let status = r["result"]["crosswind_status"]
        .as_str()
        .unwrap()
        .to_lowercase();
    for banned in ["safe", "unsafe", "legal", "approved"] {
        assert!(!status.contains(banned));
    }
}

#[test]
fn variable_wind_worst_case() {
    let r = call(
        "aviation.wind.runway-components",
        r#"{"runway":"27","wind":"VRB08KT","variation":0}"#,
    );
    assert_eq!(num(&r, "result.crosswind.value"), 8.0);
    assert_eq!(num(&r, "result.headwind.value"), -8.0);
    assert!(r["summary"].as_str().unwrap().contains("worst case"));
}

#[test]
fn metar_wind_needs_variation_on_a_magnetic_runway() {
    let r = call(
        "aviation.wind.runway-components",
        r#"{"runway":"27","wind":"30015KT"}"#,
    );
    assert_eq!(r["error"]["code"], "INVALID_INPUT");
    assert!(r["error"]["hint"].as_str().unwrap().contains("METAR"));
    let r = call(
        "aviation.wind.runway-components",
        r#"{"runway":"27","wind":"31015KT","variation":"10 deg"}"#,
    );
    assert!(
        (num(&r, "result.crosswind.value") - 7.5).abs() < 1e-9,
        "{r}"
    );
}

#[test]
fn calm_wind() {
    let r = call(
        "aviation.wind.runway-components",
        r#"{"runway":"09L","wind":"00000KT","variation":0}"#,
    );
    assert_eq!(num(&r, "result.crosswind.value"), 0.0);
    assert!(r["summary"].as_str().unwrap().starts_with("No crosswind"));
    let t = call(
        "aviation.wind.heading-groundspeed",
        r#"{"course":90,"tas":100,"wind_direction":0,"wind_speed":0}"#,
    );
    assert_eq!(num(&t, "result.wind_correction_angle.value"), 0.0);
    assert_eq!(num(&t, "result.groundspeed.value"), 100.0);
}

#[test]
fn wind_triangle_forms() {
    let r = call(
        "aviation.wind.heading-groundspeed",
        r#"{"course":90,"tas":120,"wind_direction":30,"wind_speed":20}"#,
    );
    assert!((num(&r, "result.wind_correction_angle.value") + 8.30).abs() < 0.005);
    assert!((num(&r, "result.heading.value") - 81.7).abs() < 0.005);
    assert!((num(&r, "result.groundspeed.value") - 108.7).abs() < 0.05);
    let w = call(
        "aviation.wind.find-wind",
        r#"{"heading":81.7,"tas":120,"track":90,"groundspeed":108.7}"#,
    );
    // The scenario's inputs are rounded to 0.1, so the wind is "about" 030° at 20 kt.
    assert!((num(&w, "result.wind_direction.value") - 30.0).abs() < 0.2);
    assert!((num(&w, "result.wind_speed.value") - 20.0).abs() < 0.1);
}

#[test]
fn wind_too_strong_and_bad_tas() {
    let r = call(
        "aviation.wind.heading-groundspeed",
        r#"{"course":90,"tas":30,"wind_direction":0,"wind_speed":40}"#,
    );
    assert_eq!(r["error"]["code"], "NO_SOLUTION");
    let r = call(
        "aviation.wind.heading-groundspeed",
        r#"{"course":90,"tas":0,"wind_direction":0,"wind_speed":10}"#,
    );
    assert_eq!(r["error"]["code"], "INVALID_INPUT");
}

#[test]
fn mixed_references_rejected() {
    let r = call(
        "aviation.wind.heading-groundspeed",
        r#"{"course":90,"tas":120,"wind_direction":30,"wind_speed":20,"wind_reference":"magnetic"}"#,
    );
    assert_eq!(r["error"]["code"], "INVALID_INPUT");
    assert!(r["error"]["hint"].as_str().unwrap().contains("ATIS"));
}

#[test]
fn isa_invariants() {
    // The ideal gas law p = ρRT at every level, hydrostatic balance
    // dp/dH = -ρg0 in geopotential altitude, and pressure and density falling
    // monotonically across the ICAO range (-5 km to 80 km geopotential).
    const R: f64 = 287.052_87;
    const G0: f64 = 9.806_65;
    let at = |h_m: f64| {
        call(
            "aviation.atmosphere.isa",
            &format!(
                r#"{{"altitude":"{h_m} m","options":{{"outputUnits":{{"temperature":"K","pressure":"Pa"}}}}}}"#
            ),
        )
    };
    let (mut prev_p, mut prev_rho) = (f64::INFINITY, f64::INFINITY);
    let mut h = -4999.0;
    while h <= 79_000.0 {
        let r = at(h);
        let (t, p, rho) = (
            num(&r, "result.temperature.value"),
            num(&r, "result.pressure.value"),
            num(&r, "result.density.value"),
        );
        assert!((p - rho * R * t).abs() <= 1e-12 * p, "gas law at {h} m");
        assert!(p < prev_p && rho < prev_rho, "monotonic at {h} m");
        (prev_p, prev_rho) = (p, rho);
        let d = 0.5;
        let dpdh = (num(&at(h + d), "result.pressure.value")
            - num(&at(h - d), "result.pressure.value"))
            / (2.0 * d);
        assert!(
            (dpdh + rho * G0).abs() <= 1e-6 * rho * G0,
            "hydrostatic at {h} m: {dpdh} vs {}",
            -rho * G0
        );
        h += 1_237.0;
    }
}
