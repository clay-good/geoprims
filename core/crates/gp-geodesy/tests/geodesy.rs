//! Geodesy tools: catalog lint, examples, golden vectors, spec scenarios, and
//! round-trip properties over many points.

use std::path::Path;

use gp_base::{manifest, template, vectors};
use gp_geodesy::{REGISTRY, TOOLS};
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
fn catalog_lint_examples_vectors() {
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
    let mut failures = manifest::lint(TOOLS, &taxonomy, &["aviation.wind.runway-components"]);
    let codes_reg: Value = serde_json::from_str(&repo("data/codes.json")).unwrap();
    for t in TOOLS {
        for w in t.warnings {
            if codes_reg["warnings"].get(*w).is_none() {
                failures.push(format!("{} declares unregistered {w}", t.id));
            }
        }
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
fn pittsburgh_everywhere() {
    let u = call(
        "geodesy.utm.forward",
        r#"{"lat":40.446111,"lon":-79.982222}"#,
    );
    assert_eq!(u["result"]["formatted"], "17N 586309.953 4477770.428");
    let d = call(
        "geodesy.utm.forward",
        r#"{"lat":"40°26'46\"N","lon":"79°58'56\"W"}"#,
    );
    assert!(
        (num(&d, "result.easting.value") - 586_309.95).abs() < 0.05,
        "DMS input: {d}"
    );
    let m = call(
        "geodesy.grid-ref.mgrs-forward",
        r#"{"lat":40.446111,"lon":-79.982222}"#,
    );
    assert_eq!(m["result"]["mgrs_spaced"], "17T NE 86309 77770");
    let back = call(
        "geodesy.grid-ref.mgrs-inverse",
        r#"{"mgrs":"17TNE8630977770"}"#,
    );
    let corner_e = num(
        &call(
            "geodesy.utm.forward",
            &format!(
                r#"{{"lat":{},"lon":{}}}"#,
                num(&back, "result.corner_lat.value"),
                num(&back, "result.corner_lon.value")
            ),
        ),
        "result.easting.value",
    );
    assert!((corner_e - 586_309.0).abs() < 1e-6, "{corner_e}");
    let p = call("geodesy.parse.coordinates", r#"{"text":"17TNE8630977770"}"#);
    assert_eq!(p["result"]["notation"], "MGRS");
    let p = call(
        "geodesy.parse.coordinates",
        r#"{"text":"17N 586309.953 4477770.428"}"#,
    );
    assert!((num(&p, "result.lat.value") - 40.446_111).abs() < 1e-8);
}

#[test]
fn forced_zone_scale_is_larger() {
    let std = call(
        "geodesy.utm.forward",
        r#"{"lat":40.446111,"lon":-79.982222}"#,
    );
    let forced = call(
        "geodesy.utm.forward",
        r#"{"lat":40.446111,"lon":-79.982222,"zone":18}"#,
    );
    assert!(codes(&forced).contains(&"NONSTANDARD_ZONE".to_owned()));
    assert!(num(&forced, "result.scale") > num(&std, "result.scale"));
    let far = call("geodesy.utm.forward", r#"{"lat":40,"lon":-80,"zone":22}"#);
    assert_eq!(far["error"]["code"], "INVALID_INPUT");
}

#[test]
fn domain_limits() {
    let r = call("geodesy.utm.forward", r#"{"lat":84.5,"lon":0}"#);
    assert_eq!(r["error"]["code"], "OUT_OF_DOMAIN");
    assert!(
        r["error"]["hint"]
            .as_str()
            .unwrap()
            .contains("geodesy.ups.forward")
    );
}

#[test]
fn bearing_difference_across_north() {
    let r = call(
        "geodesy.parse.bearing-difference",
        r#"{"from":350,"to":10}"#,
    );
    assert_eq!(num(&r, "result.difference.value"), 20.0);
    assert_eq!(r["summary"], "Turn 20° right.");
}

#[test]
fn format_rounding_carry_and_resolution() {
    let r = call(
        "geodesy.parse.format",
        r#"{"lat":10.9999999,"lon":0,"style":"dms","decimals":0}"#,
    );
    assert_eq!(r["result"]["formatted"], "11°00'00\"N 0°00'00\"E");
    let r = call(
        "geodesy.parse.format",
        r#"{"lat":40,"lon":0,"style":"dd","decimals":4}"#,
    );
    assert!((num(&r, "result.latitude_resolution.value") - 11.1).abs() < 0.05);
}

/// UTM and MGRS round trips over a grid of points.
#[test]
fn round_trip_properties() {
    let mut worst = 0.0f64;
    for lat in (-79..=83).step_by(7) {
        for lon in (-179..180).step_by(13) {
            let (lat, lon) = (lat as f64 + 0.37, lon as f64 + 0.61);
            let f = call(
                "geodesy.utm.forward",
                &format!(r#"{{"lat":{lat},"lon":{lon}}}"#),
            );
            let inv = call(
                "geodesy.utm.inverse",
                &format!(
                    r#"{{"zone":{},"hemisphere":"{}","easting":{},"northing":{}}}"#,
                    num(&f, "result.zone"),
                    f["result"]["hemisphere"].as_str().unwrap(),
                    num(&f, "result.easting.value"),
                    num(&f, "result.northing.value")
                ),
            );
            worst = worst
                .max((num(&inv, "result.lat.value") - lat).abs())
                .max((num(&inv, "result.lon.value") - lon).abs());
            let m = call(
                "geodesy.grid-ref.mgrs-forward",
                &format!(r#"{{"lat":{lat},"lon":{lon},"precision":"0.001m"}}"#),
            );
            let back = call(
                "geodesy.grid-ref.mgrs-inverse",
                &format!(r#"{{"mgrs":"{}"}}"#, m["result"]["mgrs"].as_str().unwrap()),
            );
            assert_eq!(back["ok"], true, "{m} -> {back}");
            assert!((num(&back, "result.lat.value") - lat).abs() < 1e-7, "{m}");
        }
    }
    assert!(worst < 1e-10, "worst UTM round trip {worst}°");
}
