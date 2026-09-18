//! Indexing tools: catalog lint, examples, golden vectors, spec scenarios,
//! and round trips.

use std::path::Path;

use gp_base::{manifest, template, vectors};
use gp_indexing::{REGISTRY, TOOLS};
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
fn scenarios() {
    let g = call(
        "indexing.geohash.encode",
        r#"{"lat":40.446111,"lon":-79.982222,"precision":9}"#,
    );
    assert_eq!(g["result"]["geohash"], "dppn5fyxx");
    let bad = call("indexing.geohash.decode", r#"{"geohash":"dpan"}"#);
    assert!(
        bad["error"]["message"]
            .as_str()
            .unwrap()
            .contains("without a, i, l, and o")
    );
    let t = call(
        "indexing.tile.from-point",
        r#"{"lat":40.446111,"lon":-79.982222,"zoom":12}"#,
    );
    assert_eq!(t["result"]["tile"], "12/1137/1544");
    assert_eq!(num(&t, "result.tms_y"), 2551.0);
    assert_eq!(t["result"]["quadkey"], "032001112001");
    assert_eq!(t["display"]["ground_resolution"], "29.08 m");
    let clamped = call("indexing.tile.from-point", r#"{"lat":89,"lon":0,"zoom":3}"#);
    assert!(codes(&clamped).contains(&"WEB_MERCATOR_CLAMPED".to_owned()));
    let d = call(
        "indexing.tile.bounds",
        r#"{"tile":"12/1137/2551","convention":"detect"}"#,
    );
    assert!(
        d["result"]["other_reading"]
            .as_str()
            .unwrap()
            .contains("12/1137/1544")
    );
    let short = call("indexing.plus-code.decode", r#"{"code":"9G8F+6X"}"#);
    assert_eq!(short["error"]["field"], "/ref_lat");
    assert!(
        short["error"]["message"]
            .as_str()
            .unwrap()
            .contains("reference point")
    );
}

#[test]
fn geohash_antimeridian_and_poles() {
    let e = call(
        "indexing.geohash.encode",
        r#"{"lat":0.1,"lon":179.99,"precision":5}"#,
    );
    let n = call(
        "indexing.geohash.neighbors",
        &format!(
            r#"{{"geohash":"{}"}}"#,
            e["result"]["geohash"].as_str().unwrap()
        ),
    );
    let east = call(
        "indexing.geohash.decode",
        &format!(r#"{{"geohash":"{}"}}"#, n["result"]["e"].as_str().unwrap()),
    );
    assert!(num(&east, "result.lon.value") < -179.9, "{east}");
    let pole = call("indexing.geohash.neighbors", r#"{"geohash":"upb"}"#);
    assert_eq!(pole["result"]["n"], "none (past the pole)");
}

#[test]
fn round_trips() {
    for lat in (-80..=80).step_by(9) {
        for lon in (-179..180).step_by(23) {
            let (lat, lon) = (lat as f64 + 0.123_456, lon as f64 + 0.654_321);
            let g = call(
                "indexing.geohash.encode",
                &format!(r#"{{"lat":{lat},"lon":{lon},"precision":10}}"#),
            );
            assert!(num(&g, "result.south.value") <= lat && lat <= num(&g, "result.north.value"));
            let t = call(
                "indexing.tile.from-point",
                &format!(r#"{{"lat":{lat},"lon":{lon},"zoom":18}}"#),
            );
            let b = call(
                "indexing.tile.bounds",
                &format!(
                    r#"{{"tile":"{}"}}"#,
                    t["result"]["quadkey"].as_str().unwrap()
                ),
            );
            assert!(
                num(&b, "result.south.value") <= lat && lat <= num(&b, "result.north.value"),
                "{t} {b}"
            );
            assert!(num(&b, "result.west.value") <= lon && lon <= num(&b, "result.east.value"));
            let p = call(
                "indexing.plus-code.encode",
                &format!(r#"{{"lat":{lat},"lon":{lon},"length":11}}"#),
            );
            let code = p["result"]["code"].as_str().unwrap();
            let d = call(
                "indexing.plus-code.decode",
                &format!(r#"{{"code":"{code}"}}"#),
            );
            assert!(
                num(&d, "result.south.value") <= lat
                    && lat <= num(&d, "result.north.value") + 1e-12,
                "{code}"
            );
            let s = call(
                "indexing.plus-code.shorten",
                &format!(
                    r#"{{"code":"{code}","ref_lat":{},"ref_lon":{}}}"#,
                    lat + 0.01,
                    lon - 0.01
                ),
            );
            let r = call(
                "indexing.plus-code.decode",
                &format!(
                    r#"{{"code":"{}","ref_lat":{},"ref_lon":{}}}"#,
                    s["result"]["short_code"].as_str().unwrap(),
                    lat + 0.01,
                    lon - 0.01
                ),
            );
            assert_eq!(r["result"]["full_code"], code);
        }
    }
}

#[test]
fn hardening() {
    for (id, input) in [
        ("indexing.tile.from-point", r#"{"lat":0,"lon":0,"zoom":31}"#),
        (
            "indexing.tile.from-point",
            r#"{"lat":0,"lon":0,"zoom":1.5}"#,
        ),
        ("indexing.tile.bounds", r#"{"tile":"12/1137"}"#),
        ("indexing.tile.bounds", r#"{"tile":"0124"}"#),
        (
            "indexing.plus-code.encode",
            r#"{"lat":0,"lon":0,"length":7}"#,
        ),
        ("indexing.plus-code.decode", r#"{"code":"8FVC9G8F6X"}"#),
        ("indexing.plus-code.decode", r#"{"code":"8FVC9G8F+6"}"#),
        ("indexing.plus-code.decode", r#"{"code":"8FVA9G8F+6X"}"#),
        ("indexing.geohash.decode", r#"{"geohash":""}"#),
    ] {
        assert_eq!(
            call(id, input)["error"]["code"],
            "INVALID_INPUT",
            "{id} {input}"
        );
    }
}
