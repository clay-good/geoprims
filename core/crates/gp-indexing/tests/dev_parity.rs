//! Developer hero tools against separately written reference libraries,
//! through the public tool interface (what the web page and MCP both call).
//! Fixtures come from tools/vectors/gen_dev_diff.py: pygeohash 3.3.2,
//! mercantile 1.2.1, and H3 C 4.4.1 via h3-py 4.4.2.

use gp_indexing::REGISTRY;
use serde_json::{Value, json};

fn call(id: &str, input: &Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).expect("envelope is JSON")
}

fn rows(name: &str) -> Vec<Vec<String>> {
    let path = format!("{}/tests/data/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {path}: {e}"))
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.split(',').map(str::to_owned).collect())
        .collect()
}

fn f(s: &str) -> f64 {
    s.parse().unwrap()
}

fn deg(r: &Value, k: &str) -> f64 {
    r["result"][k]["value"].as_f64().unwrap_or_else(|| panic!("{k} missing in {r}"))
}

#[test]
fn geohash_matches_pygeohash() {
    let rows = rows("geohash_diff.csv");
    assert_eq!(rows.len(), 1000);
    let mut bad = Vec::new();
    for c in &rows {
        let r = call(
            "indexing.geohash.encode",
            &json!({"lat": f(&c[0]), "lon": f(&c[1]), "precision": f(&c[2])}),
        );
        let same_box = ["south", "west", "north", "east"]
            .iter()
            .zip(&c[4..8])
            .all(|(k, v)| (deg(&r, k) - f(v)).abs() < 1e-12);
        if r["result"]["geohash"] != c[3].as_str() || !same_box {
            bad.push(format!("{} -> {}", c.join(","), r["result"]));
        }
    }
    assert!(bad.is_empty(), "{} mismatches:\n{}", bad.len(), bad[..bad.len().min(10)].join("\n"));
}

#[test]
fn tiles_match_mercantile() {
    let rows = rows("tile_diff.csv");
    assert_eq!(rows.len(), 1000);
    let mut bad = Vec::new();
    for c in &rows {
        let xyz = format!("{}/{}/{}", c[2], c[3], c[4]);
        let p = call(
            "indexing.tile.from-point",
            &json!({"lat": f(&c[0]), "lon": f(&c[1]), "zoom": f(&c[2])}),
        );
        if p["result"]["tile"] != xyz.as_str() || p["result"]["quadkey"] != c[5].as_str() {
            bad.push(format!("from-point {} -> {}", c.join(","), p["result"]));
            continue;
        }
        let b = call("indexing.tile.bounds", &json!({"tile": xyz}));
        let same = ["west", "south", "east", "north"]
            .iter()
            .zip(&c[6..10])
            .all(|(k, v)| (deg(&b, k) - f(v)).abs() < 1e-11);
        if !same || b["result"]["quadkey"] != c[5].as_str() {
            bad.push(format!("bounds {} -> {}", c.join(","), b["result"]));
        }
    }
    assert!(bad.is_empty(), "{} mismatches:\n{}", bad.len(), bad[..bad.len().min(10)].join("\n"));
}

#[test]
fn grid_disk_matches_h3_c() {
    let rows = rows("h3_disk_diff.csv");
    assert_eq!(rows.len(), 250);
    let mut bad = Vec::new();
    for c in &rows {
        let r = call("indexing.h3.grid-disk", &json!({"cell": c[0], "k": f(&c[1])}));
        let got: Vec<&str> = r["result"]["cells"]
            .as_array()
            .unwrap_or_else(|| panic!("{r}"))
            .iter()
            .map(|x| x["cell"].as_str().unwrap())
            .collect();
        let mut want: Vec<&str> = c[2].split(' ').collect();
        let mut sorted = got.clone();
        sorted.sort_unstable();
        want.sort_unstable();
        if sorted != want || got[0] != c[0] || r["result"]["count"].as_f64() != Some(want.len() as f64) {
            bad.push(format!("{} k={}: {} cells, want {}", c[0], c[1], got.len(), want.len()));
        }
    }
    assert!(bad.is_empty(), "{} mismatches:\n{}", bad.len(), bad[..bad.len().min(10)].join("\n"));
}
