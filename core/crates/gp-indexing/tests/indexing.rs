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

#[test]
fn h3_scenarios() {
    let c = call(
        "indexing.h3.lat-lng-to-cell",
        r#"{"lat":40.446111,"lon":-79.982222,"resolution":9}"#,
    );
    assert_eq!(c["result"]["cell"], "892a8471487ffff");
    assert!((num(&c, "result.center_lat.value") - 40.444_866).abs() < 5e-7);
    assert!((num(&c, "result.center_lon.value") + 79.981_847).abs() < 5e-7);
    let p = call(
        "indexing.h3.parent",
        r#"{"cell":"892a8471487ffff","resolution":5}"#,
    );
    assert_eq!(p["result"]["parent"], "852a8473fffffff");
    let k = call(
        "indexing.h3.children",
        r#"{"cell":"892a8471487ffff","resolution":10}"#,
    );
    assert_eq!(num(&k, "result.count"), 7.0);
    let d = call(
        "indexing.h3.grid-disk",
        r#"{"cell":"85080003fffffff","k":1}"#,
    );
    assert_eq!(num(&d, "result.count"), 6.0);
    assert!(codes(&d).contains(&"PENTAGON_DISTORTION".to_owned()));
    let r = call(
        "indexing.h3.resolution-chooser",
        r#"{"target_area":"1 km2"}"#,
    );
    assert_eq!(num(&r, "result.resolution"), 8.0);
    assert_eq!(r["display"]["coarser_area"], "5.161 km²");
    let squeeze = call(
        "indexing.h3.compact",
        r#"{"cells":[{"cell":"8a2a84714847fff"},{"cell":"8a2a8471484ffff"},{"cell":"8a2a84714857fff"},{"cell":"8a2a8471485ffff"},{"cell":"8a2a84714867fff"},{"cell":"8a2a8471486ffff"},{"cell":"8a2a84714877fff"}]}"#,
    );
    assert_eq!(squeeze["result"]["cells"][0]["cell"], "892a8471487ffff");
}

#[test]
fn h3_input_guards() {
    let n = call("indexing.h3.cell-info", r#"{"cell":617741122143780863}"#);
    assert_eq!(n["error"]["code"], "INVALID_INPUT");
    assert!(n["error"]["message"].as_str().unwrap().contains("string"));
    for ok in ["892a8471487ffff", "0x892A8471487FFFF", "617741122143780863"] {
        let r = call("indexing.h3.cell-info", &format!(r#"{{"cell":"{ok}"}}"#));
        assert_eq!(r["result"]["decimal"], "617741122143780863", "{ok}");
    }
    for bad in ["892a8471487fff", "zz", "0x0", ""] {
        let r = call("indexing.h3.cell-info", &format!(r#"{{"cell":"{bad}"}}"#));
        assert_eq!(r["error"]["code"], "INVALID_INPUT", "{bad}");
    }
    let mixed = call(
        "indexing.h3.grid-path",
        r#"{"from":"892a8471487ffff","to":"852a8473fffffff"}"#,
    );
    assert_eq!(mixed["error"]["code"], "INVALID_INPUT");
    // Opposite sides of the world: H3's local grid cannot reach.
    let far = call(
        "indexing.h3.grid-path",
        r#"{"from":"8009fffffffffff","to":"80f3fffffffffff"}"#,
    );
    assert_eq!(far["error"]["code"], "DEGENERATE_GEOMETRY", "{far}");
    let up = call(
        "indexing.h3.parent",
        r#"{"cell":"892a8471487ffff","resolution":10}"#,
    );
    assert_eq!(up["error"]["field"], "/resolution");
    let big = call(
        "indexing.h3.uncompact",
        r#"{"cells":[{"cell":"8009fffffffffff"}],"resolution":15}"#,
    );
    assert!(big["result"].get("cells").is_none() && num(&big, "result.count") > 1e10);
}

#[test]
fn polygon_fill_scenarios() {
    let square = r#""points":[{"lat":51.40,"lon":-0.30},{"lat":51.60,"lon":-0.30},{"lat":51.60,"lon":0.10},{"lat":51.40,"lon":0.10}],"resolution":8"#;
    let cells = |r: &Value| -> Vec<String> {
        r["result"]["cells"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["cell"].as_str().unwrap().to_owned())
            .collect()
    };
    let center = call("indexing.h3.polygon-to-cells", &format!("{{{square}}}"));
    assert_eq!(
        center["result"]["containment"], "center",
        "the default is echoed"
    );
    let over = call(
        "indexing.h3.polygon-to-cells",
        &format!(r#"{{{square},"containment":"overlapping"}}"#),
    );
    let full = call(
        "indexing.h3.polygon-to-cells",
        &format!(r#"{{{square},"containment":"full"}}"#),
    );
    let (c, o, f) = (cells(&center), cells(&over), cells(&full));
    assert!(
        c.iter().all(|x| o.contains(x)) && o.len() > c.len(),
        "overlapping is a superset of center"
    );
    assert!(
        f.iter().all(|x| c.contains(x)) && f.len() < c.len(),
        "full is a subset of center"
    );
    // A continent at resolution 12 is refused from the estimate, quickly.
    let t = std::time::Instant::now();
    let big = call(
        "indexing.h3.polygon-to-cells",
        r#"{"points":[{"lat":-35,"lon":110},{"lat":-35,"lon":155},{"lat":-10,"lon":155},{"lat":-10,"lon":110}],"resolution":12}"#,
    );
    assert_eq!(big["error"]["code"], "LIMIT_EXCEEDED");
    assert!(big["error"]["hint"].as_str().unwrap().contains("coarser"));
    assert!(t.elapsed().as_millis() < 50, "{:?}", t.elapsed());
    let both = call(
        "indexing.h3.polygon-to-cells",
        r#"{"points":[{"lat":0,"lon":0},{"lat":1,"lon":0},{"lat":1,"lon":1}],"geojson":"{}","resolution":3}"#,
    );
    assert_eq!(both["error"]["code"], "INVALID_INPUT");
}
