//! The H3 family against H3 C 4.4.1 (via h3-py 4.4.2) through the public
//! tools, on 300 seeded cells at every resolution, one in eight a pentagon
//! (tools/vectors/gen_h3_family_diff.py): cell details and boundary, grid
//! ring, grid path, parent and child position, children, directed edges and
//! vertexes, and compact / uncompact.

use gp_indexing::REGISTRY;
use serde_json::{Value, json};

fn call(id: &str, input: &Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).expect("envelope is JSON")
}

fn cells(r: &Value, key: &str, field: &str) -> Vec<String> {
    let mut v: Vec<String> = r["result"][key]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(|x| {
            x.get(field)
                .and_then(Value::as_str)
                .or_else(|| x.as_str())
                .unwrap_or("")
                .to_owned()
        })
        .collect();
    v.sort();
    v
}

fn strs(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn h3_family_matches_h3_c() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/h3_family_diff.jsonl"
    ))
    .unwrap();
    let mut bad = Vec::new();
    let mut n = 0;
    for line in text.lines().skip(1) {
        let w: Value = serde_json::from_str(line).unwrap();
        let c = w["cell"].as_str().unwrap();
        let mut miss = |what: &str, detail: String| bad.push(format!("{c} {what}: {detail}"));

        let info = call("indexing.h3.cell-info", &json!({"cell": c}));
        let i = &info["result"];
        let near = |a: &Value, b: &Value, tol: f64| {
            (a.as_f64().unwrap_or(f64::NAN) - b.as_f64().unwrap()).abs() <= tol
        };
        if !near(&i["lat"]["value"], &w["lat"], 5e-11)
            || !near(
                &i["lon"]["value"],
                &w["lon"],
                5e-11 / w["lat"].as_f64().unwrap().to_radians().cos().max(1e-3),
            )
        {
            miss("center", format!("{} {}", i["lat"], i["lon"]));
        }
        if i["resolution"] != w["resolution"] || i["base_cell"] != w["base_cell"] {
            miss(
                "resolution or base cell",
                format!("{} {}", i["resolution"], i["base_cell"]),
            );
        }
        if (i["pentagon"] == "yes") != w["pentagon"].as_bool().unwrap()
            || (i["class3"] == "yes") != w["class3"].as_bool().unwrap()
        {
            miss(
                "pentagon or class",
                format!("{} {}", i["pentagon"], i["class3"]),
            );
        }
        // Areas agree to 6e-15 at resolution 0, loosening about 7x per resolution to 2e-8 at
        // resolution 15, where the spherical excess of a 1 m² cell runs out of digits in both.
        if (i["area"]["value"].as_f64().unwrap() / w["area_km2"].as_f64().unwrap() - 1.0).abs()
            > 5e-8
        {
            miss(
                "area",
                format!("{} vs {}", i["area"]["value"], w["area_km2"]),
            );
        }
        let b = i["boundary"].as_array().unwrap();
        let wb = w["boundary"].as_array().unwrap();
        if b.len() != wb.len()
            || b.iter()
                .zip(wb)
                .any(|(x, y)| (x["lat"].as_f64().unwrap() - y[0].as_f64().unwrap()).abs() > 5e-11)
        {
            miss("boundary", format!("{} vertices vs {}", b.len(), wb.len()));
        }

        if let Some(ring) = w["ring"].as_array() {
            let r = call(
                "indexing.h3.grid-ring",
                &json!({"cell": c, "k": w["ring_k"]}),
            );
            let want: Vec<String> = ring
                .iter()
                .map(|x| x.as_str().unwrap().to_owned())
                .collect();
            if cells(&r, "cells", "cell") != want {
                miss(
                    "ring",
                    format!("{} cells vs {}", r["result"]["count"], want.len()),
                );
            }
        }

        let p = call(
            "indexing.h3.grid-path",
            &json!({"from": c, "to": w["path_to"]}),
        );
        match w["path"].as_array() {
            Some(path) => {
                let got: Vec<String> = p["result"]["cells"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .map(|x| x["cell"].as_str().unwrap().to_owned())
                            .collect()
                    })
                    .unwrap_or_default();
                if got != strs(&w["path"]) {
                    miss("path", format!("{} cells vs {}", got.len(), path.len()));
                }
            }
            None => {
                if p["ok"] == true {
                    miss("path", "H3 C refuses this path, the tool gave one".into());
                }
            }
        }

        let par = call(
            "indexing.h3.parent",
            &json!({"cell": c, "resolution": w["parent_res"]}),
        );
        if par["result"]["parent"] != w["parent"]
            || par["result"]["child_position"] != w["child_pos"]
        {
            miss(
                "parent",
                format!(
                    "{} {}",
                    par["result"]["parent"], par["result"]["child_position"]
                ),
            );
        }

        let ch = call(
            "indexing.h3.children",
            &json!({"cell": c, "resolution": w["children_res"]}),
        );
        if ch["result"]["count"] != w["children_count"]
            || ch["result"]["center_child"] != w["center_child"]
        {
            miss(
                "children",
                format!("{} {}", ch["result"]["count"], ch["result"]["center_child"]),
            );
        }

        let e = call("indexing.h3.edges", &json!({"cell": c}));
        if cells(&e, "edges", "edge") != strs(&w["edges"])
            || cells(&e, "vertexes", "vertex") != strs(&w["vertexes"])
        {
            miss(
                "edges or vertexes",
                format!("{} edges", e["result"]["edge_count"]),
            );
        }

        let rows = |v: &Value| {
            v.as_array()
                .unwrap()
                .iter()
                .map(|x| json!({"cell": x}))
                .collect::<Vec<_>>()
        };
        let cp = call(
            "indexing.h3.compact",
            &json!({"cells": rows(&w["compact_input"])}),
        );
        if cells(&cp, "cells", "cell") != strs(&w["compacted"]) {
            miss(
                "compact",
                format!(
                    "{} cells vs {}",
                    cp["result"]["count"],
                    w["compacted"].as_array().unwrap().len()
                ),
            );
        }
        let uc = call(
            "indexing.h3.uncompact",
            &json!({"cells": rows(&w["compacted"]), "resolution": w["resolution"]}),
        );
        if cells(&uc, "cells", "cell") != strs(&w["compact_input"]) {
            miss("uncompact", format!("{} cells", uc["result"]["count"]));
        }
        n += 1;
    }
    assert_eq!(n, 300);
    assert!(
        bad.is_empty(),
        "{} disagreements:\n{}",
        bad.len(),
        bad[..bad.len().min(30)].join("\n")
    );
}
