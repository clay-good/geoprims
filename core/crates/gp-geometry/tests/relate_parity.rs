//! The DE-9IM against GEOS, matrix for matrix.
//!
//! Corners on a small integer grid make the cases that break inexact code
//! common (shared edges, a corner on an edge, collinear overlaps, a line
//! ending on a boundary), and with planar edges both sides decide on the same
//! doubles with exact orientation tests, so every character must agree. The
//! fixture holds GEOS's own matrix for each pair; see
//! tools/vectors/gen_relate_geos.py.

use gp_geometry::REGISTRY;
use serde_json::{Value, json};

#[test]
fn relate_matches_geos_on_the_grid() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/relate_geos.json");
    let cases: Vec<Value> =
        serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("JSON");
    assert!(cases.len() >= 1_000, "{} cases", cases.len());
    let mut wrong = Vec::new();
    for (i, c) in cases.iter().enumerate() {
        let r: Value = serde_json::from_str(
            &REGISTRY.invoke(
                "geometry.predicate.relate",
                &json!({
                    "geometry_a": c["geometry_a"], "kind_a": c["kind_a"],
                    "geometry_b": c["geometry_b"], "kind_b": c["kind_b"],
                    "edges": "planar",
                })
                .to_string(),
            ),
        )
        .expect("JSON");
        let got = r["result"]["matrix"].as_str().unwrap_or("error");
        if got != c["matrix"] {
            wrong.push(format!("case {i}: GEOS {} here {got}: {c}", c["matrix"]));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {} differ:\n{}",
        wrong.len(),
        cases.len(),
        wrong[..wrong.len().min(10)].join("\n")
    );
}
