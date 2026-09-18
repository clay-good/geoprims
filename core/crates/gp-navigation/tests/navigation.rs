//! Geodesic tools: catalog lint, examples, golden vectors, spec scenarios, and
//! a Karney-vs-Vincenty differential over random point pairs.

use std::path::Path;

use gp_base::{manifest, template, vectors};
use gp_navigation::{REGISTRY, TOOLS, vincenty};
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
    r["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap().to_owned())
        .collect()
}

const JFK_LHR: &str = r#"{"lat1":40.6413,"lon1":-73.7781,"lat2":51.47,"lon2":-0.4543,"options":{"outputUnits":{"distance":"m"}}}"#;

#[test]
fn catalog_lint() {
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
    let errs = manifest::lint(TOOLS, &taxonomy, &[]);
    assert!(errs.is_empty(), "{}", errs.join("\n"));
}

#[test]
fn examples_and_vectors() {
    let mut failures = Vec::new();
    for t in TOOLS {
        for ex in t.examples {
            let r = call(t.id, ex.input);
            let s = r["summary"].as_str().unwrap_or_default();
            if r["ok"] != true || template::grade(s) > 8.0 || s.len() > template::MAX_CHARS {
                failures.push(format!("{} example: {r}", t.id));
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
fn jfk_to_lhr() {
    let r = call("navigation.geodesic.inverse", JFK_LHR);
    assert!((num(&r, "result.distance.value") - 5_554_908.791).abs() < 1e-3);
    assert!((num(&r, "result.azimuth1.value") - 51.381_647_9).abs() < 1e-7);
    assert!((num(&r, "result.azimuth2.value") - 107.982_829_1).abs() < 1e-7);
    let nm = call(
        "navigation.geodesic.inverse",
        r#"{"lat1":40.6413,"lon1":-73.7781,"lat2":51.47,"lon2":-0.4543,"options":{"profile":"aviation"}}"#,
    );
    assert_eq!(nm["result"]["distance"]["unit"], "NM");
    assert!((num(&nm, "result.distance.value") - 2_999.411).abs() < 1e-3);
}

#[test]
fn nearly_and_exactly_antipodal() {
    let r = call(
        "navigation.geodesic.inverse",
        r#"{"lat1":0,"lon1":0,"lat2":0.5,"lon2":179.7,"options":{"outputUnits":{"distance":"m"}}}"#,
    );
    assert!((num(&r, "result.distance.value") - 19_944_127.421).abs() < 1e-3);
    assert!((num(&r, "result.azimuth1.value") - 15.556_883).abs() < 1e-6);
    assert!(!codes(&r).iter().any(|c| c.contains("CONVERGE")));
    let r = call(
        "navigation.geodesic.inverse",
        r#"{"lat1":0,"lon1":0,"lat2":0,"lon2":180}"#,
    );
    assert!(codes(&r).contains(&"AZIMUTH_NOT_UNIQUE".to_owned()));
}

#[test]
fn coincident_points() {
    let r = call(
        "navigation.geodesic.inverse",
        r#"{"lat1":10,"lon1":10,"lat2":10,"lon2":10}"#,
    );
    assert_eq!(num(&r, "result.distance.value"), 0.0);
    assert!(codes(&r).contains(&"AZIMUTH_UNDEFINED".to_owned()));
}

#[test]
fn direct_1000_km() {
    let r = call(
        "navigation.geodesic.direct",
        r#"{"lat1":40.6413,"lon1":-73.7781,"azimuth":51,"distance":"1000000 m"}"#,
    );
    assert!((num(&r, "result.lat2.value") - 45.892_080_8).abs() < 1e-7);
    assert!((num(&r, "result.lon2.value") + 63.754_956_3).abs() < 1e-7);
    assert!((num(&r, "result.azimuth2.value") - 57.886_373).abs() < 1e-6);
}

#[test]
fn vincenty_honesty() {
    let r = call(
        "navigation.geodesic.vincenty-inverse",
        r#"{"lat1":0,"lon1":0,"lat2":0.5,"lon2":179.7}"#,
    );
    assert_eq!(r["error"]["code"], "DID_NOT_CONVERGE");
    assert!(
        r["error"]["hint"]
            .as_str()
            .unwrap()
            .contains("navigation.geodesic.inverse")
    );
    let r = call("navigation.geodesic.vincenty-inverse", JFK_LHR);
    let diff_mm = num(&r, "result.karney_difference.value");
    assert!(diff_mm.abs() < 1.0, "{diff_mm} mm");
}

#[test]
fn haversine_error_shown() {
    let r = call("navigation.geodesic.haversine", JFK_LHR);
    assert!((num(&r, "result.distance.value") - 5_540_019.0).abs() < 1.0);
    assert!((num(&r, "result.difference.value") - 14_890.0).abs() < 1.0);
    assert!((num(&r, "result.difference_percent") - 0.27).abs() < 0.005);
    assert!(r["summary"].as_str().unwrap().contains("shorter"));
}

#[test]
fn mars_and_high_flattening() {
    let r = call(
        "navigation.geodesic.inverse",
        r#"{"lat1":0,"lon1":0,"lat2":10,"lon2":10,"a":"3396190 m","inverse_flattening":169.894}"#,
    );
    let model = r["meta"]["model"].as_str().unwrap();
    assert!(
        model.contains("3396190") && model.contains("169.894"),
        "{model}"
    );
    let r = call(
        "navigation.geodesic.inverse",
        r#"{"lat1":0,"lon1":0,"lat2":10,"lon2":10,"a":"6378137 m","inverse_flattening":10}"#,
    );
    assert_eq!(r["error"]["code"], "UNSUPPORTED");
    let r = call(
        "navigation.geodesic.inverse",
        r#"{"lat1":0,"lon1":0,"lat2":10,"lon2":10,"a":"6378137 m"}"#,
    );
    assert_eq!(r["error"]["code"], "INVALID_INPUT");
}

#[test]
fn input_rules() {
    let r = call(
        "navigation.geodesic.inverse",
        r#"{"lat1":90.0000001,"lon1":0,"lat2":0,"lon2":0}"#,
    );
    assert_eq!(r["error"]["field"], "/lat1");
    let r = call(
        "navigation.geodesic.inverse",
        r#"{"lat1":0,"lon1":540.25,"lat2":0,"lon2":0}"#,
    );
    assert!(codes(&r).contains(&"INPUT_NORMALIZED".to_owned()));
}

/// Differential: Karney and Vincenty agree within 1 mm away from antipodes.
#[test]
fn karney_vs_vincenty_differential() {
    let mut s: u64 = 0x1234_5678_9ABC_DEF1;
    let mut rnd = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        (s >> 11) as f64 / (1u64 << 53) as f64
    };
    let g = geographiclib_rs::Geodesic::wgs84();
    let (a, f) = (6_378_137.0, 1.0 / 298.257_223_563);
    let mut worst: f64 = 0.0;
    for _ in 0..2000 {
        let (la1, lo1) = (rnd() * 178.0 - 89.0, rnd() * 360.0 - 180.0);
        let (la2, lo2) = (rnd() * 178.0 - 89.0, rnd() * 360.0 - 180.0);
        use geographiclib_rs::InverseGeodesic;
        let (k, _, _, _): (f64, f64, f64, f64) = g.inverse(la1, lo1, la2, lo2);
        if k > 19_000_000.0 {
            continue; // near-antipodal: Vincenty is not expected to converge
        }
        if let Some((v, _, _)) = vincenty::inverse(a, f, la1, lo1, la2, lo2) {
            worst = worst.max((v - k).abs());
        }
    }
    assert!(worst < 1e-3, "worst Karney-Vincenty difference {worst} m");
}
