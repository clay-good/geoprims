//! Polygon area against GeographicLib's Planimeter (tools/vectors/gen_area_diff.py):
//! 500 polygons from fields to continents, both orientations, across the
//! antimeridian, and around the poles; plus the catalog lint and vectors.

use std::path::Path;

use gp_base::{manifest, template, vectors};
use gp_geometry::{REGISTRY, TOOLS};
use serde_json::{Value, json};

fn repo(path: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(path);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

fn call(id: &str, input: &str) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, input)).expect("envelope is JSON")
}

#[test]
fn matches_planimeter() {
    let text = include_str!("data/planimeter_diff.txt");
    let mut lines = text.lines();
    let (mut worst_area, mut worst_per, mut n) = (0.0f64, 0.0f64, 0);
    while let Some(head) = lines.next() {
        let h: Vec<f64> = head
            .split_whitespace()
            .map(|x| x.parse().unwrap())
            .collect();
        let rows: Vec<Value> = (0..h[0] as usize)
            .map(|_| {
                let p: Vec<f64> = lines
                    .next()
                    .unwrap()
                    .split_whitespace()
                    .map(|x| x.parse().unwrap())
                    .collect();
                json!({"lat": p[0], "lon": p[1]})
            })
            .collect();
        let r = call(
            "geometry.area.polygon",
            &json!({ "polygon": rows }).to_string(),
        );
        let area = r["result"]["area"]["value"]
            .as_f64()
            .unwrap_or_else(|| panic!("{r}"))
            * 1e6;
        let per = r["result"]["perimeter"]["value"].as_f64().unwrap() * 1000.0;
        worst_area = worst_area.max((area - h[2].abs()).abs() / h[2].abs().max(1.0));
        worst_per = worst_per.max((per - h[1]).abs() / h[1]);
        let ccw = r["result"]["orientation"] == "counterclockwise";
        assert_eq!(ccw, h[2] > 0.0, "orientation {r}");
        n += 1;
    }
    println!("{n} polygons: area {worst_area:e} relative, perimeter {worst_per:e} relative");
    assert_eq!(n, 500);
    assert!(worst_area <= 1e-8, "area {worst_area}");
    assert!(worst_per <= 1e-10, "perimeter {worst_per}");
}

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
    // Related tools in other modules.
    let external = [
        "survey.cogo.area-by-coordinates",
        "navigation.geodesic.inverse",
    ];
    let errs = manifest::lint(TOOLS, &taxonomy, &external);
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
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn spec_scenarios() {
    // A ring at 80° N: the polar cap, not the rest of the Earth.
    let cap: Vec<Value> = (0..12)
        .map(|k| json!({"lat": 80, "lon": -180 + 30 * k}))
        .collect();
    let r = call(
        "geometry.area.polygon",
        &json!({ "polygon": cap }).to_string(),
    );
    assert_eq!(r["result"]["pole"], "north");
    assert!(r["result"]["area"]["value"].as_f64().unwrap() < 5e6, "{r}");
    // Holes that cover more than the outline are refused.
    let bad = json!({"polygon": [
        {"lat": 0, "lon": 0}, {"lat": 0, "lon": 1}, {"lat": 1, "lon": 1},
        {"lat": -1, "lon": -1, "ring": 1}, {"lat": -1, "lon": 2, "ring": 1}, {"lat": 2, "lon": 2, "ring": 1}, {"lat": 2, "lon": -1, "ring": 1}
    ]});
    assert_eq!(
        call("geometry.area.polygon", &bad.to_string())["error"]["code"],
        "INVALID_INPUT"
    );
    let two =
        json!({"polygon": [{"lat": 0, "lon": 0}, {"lat": 0, "lon": 1}, {"lat": 0, "lon": 0}]});
    assert_eq!(
        call("geometry.area.polygon", &two.to_string())["error"]["code"],
        "INVALID_INPUT"
    );
}
