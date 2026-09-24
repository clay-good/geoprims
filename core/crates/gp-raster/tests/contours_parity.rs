//! Contours against contourpy, level by level.
//!
//! contourpy (the engine behind Matplotlib) is a separate C++ implementation
//! that follows the conventions this tool documents, so on every level the two
//! must find the same lines: the same count, the same closed loops, the same
//! number of vertices, and the same length. The fixture holds contourpy's own
//! answer for 300 random grids; see tools/vectors/gen_contours.py.

use gp_raster::REGISTRY;
use serde_json::{Value, json};

#[test]
fn contours_match_contourpy() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/contours_contourpy.json"
    );
    let cases: Vec<Value> =
        serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("JSON");
    assert!(cases.len() >= 200, "{} grids", cases.len());
    let mut wrong = Vec::new();
    for (i, c) in cases.iter().enumerate() {
        let r: Value = serde_json::from_str(
            &REGISTRY.invoke(
                "raster.terrain.contours",
                &json!({
                    "elevations": c["elevations"],
                    "interval": format!("{} m", c["interval"]),
                    "cell_size": format!("{} m", c["cell_size"]),
                    "no_data": c["no_data"],
                })
                .to_string(),
            ),
        )
        .expect("JSON");
        let got = r["result"]["levels"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let want = c["levels"].as_array().unwrap();
        if got.len() != want.len() {
            wrong.push(format!(
                "grid {i}: {} levels here, {} in contourpy",
                got.len(),
                want.len()
            ));
            continue;
        }
        let mut per_level: std::collections::HashMap<u64, u64> = std::collections::HashMap::new();
        for v in r["result"]["contours"].as_array().unwrap() {
            *per_level
                .entry(v["level"]["value"].as_f64().unwrap().to_bits())
                .or_default() += 1;
        }
        for (g, w) in got.iter().zip(want) {
            let level = w["level"].as_f64().unwrap();
            let vertices = per_level
                .get(&g["level"]["value"].as_f64().unwrap().to_bits())
                .copied()
                .unwrap_or(0);
            let (len_g, len_w) = (
                g["length"]["value"].as_f64().unwrap(),
                w["length"].as_f64().unwrap(),
            );
            if (g["level"]["value"].as_f64().unwrap() - level).abs() > 1e-9
                || g["lines"] != w["lines"]
                || g["closed"] != w["closed"]
                || vertices != w["vertices"].as_u64().unwrap()
                || (len_g - len_w).abs() > 1e-9 * len_w.max(1.0)
            {
                wrong.push(format!(
                    "grid {i} level {level}: here {g} with {vertices} vertices, contourpy {w}"
                ));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} differences:\n{}",
        wrong.len(),
        wrong[..wrong.len().min(10)].join("\n")
    );
}
