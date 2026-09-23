//! `indexing.s2.covering` against s2sphere (tools/vectors/gen_s2_cover.py):
//! 16 regions over every face of the cube, both poles and the antimeridian,
//! with 128 points s2sphere places inside them.
//!
//! A covering is not unique, so this does not compare cell lists. S2's own
//! RegionCoverer uses a priority-queue heuristic and this core refines from
//! the six faces; both are valid coverings of the same region and they are
//! different sets. Pinning one against the other would pin a choice between
//! equals.
//!
//! What a covering *has to be* is not a choice, and that is what this checks:
//! every cell within the level range, no more cells than the budget, and the
//! union containing the region. The last one is the reason s2sphere is here.
//! For each point it places inside a region it supplies the token of that
//! point's own cell at every level the covering may use; a covering contains
//! the point exactly when it holds one of those. That is a set intersection,
//! so the check never consults the core's own idea of where a cell is.

use serde_json::Value;

fn call(input: &str) -> Value {
    serde_json::from_str(&gp_indexing::REGISTRY.invoke("indexing.s2.covering", input))
        .expect("envelope is JSON")
}

#[test]
fn every_covering_contains_its_region() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/s2_cover.jsonl"
    ))
    .unwrap();
    let mut regions = 0;
    let mut points = 0;
    let mut bad = Vec::new();
    for line in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
    {
        let row: Value = serde_json::from_str(line).expect("fixture row is JSON");
        let p: Vec<f64> = row["params"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        let (lo, hi, budget) = (
            row["min_level"].as_u64().unwrap(),
            row["max_level"].as_u64().unwrap(),
            row["max_cells"].as_u64().unwrap(),
        );
        let area = if row["kind"] == "rect" {
            format!(
                r#""south":"{} deg","north":"{} deg","west":"{} deg","east":"{} deg""#,
                p[0], p[1], p[2], p[3]
            )
        } else {
            format!(r#""lat":{},"lon":{},"radius":"{} m""#, p[0], p[1], p[2])
        };
        let r = call(&format!(
            r#"{{{area},"min_level":{lo},"max_level":{hi},"max_cells":{budget}}}"#
        ));
        if r["ok"] != true {
            bad.push(format!("{}: {r}", row["kind"]));
            continue;
        }
        regions += 1;
        let cells: Vec<&str> = r["result"]["cells"]
            .as_array()
            .expect("cells")
            .iter()
            .map(|c| c["cell"].as_str().expect("cell"))
            .collect();
        // The budget holds unless the lowest level forces more, and when it
        // does the result says so. Silently exceeding it is the failure; the
        // warning is what makes exceeding it an answer rather than a bug.
        let over_budget = r["meta"]["warnings"]
            .as_array()
            .map(|w| w.iter().any(|x| x["code"] == "COVERING_OVER_BUDGET"))
            .unwrap_or(false);
        if cells.len() as u64 > budget && !over_budget {
            bad.push(format!(
                "{area}: {} cells over a budget of {budget}, with no warning",
                cells.len()
            ));
        }
        let (fine, coarse) = (
            r["result"]["finest_level"].as_u64().unwrap_or(0),
            r["result"]["coarsest_level"].as_u64().unwrap_or(0),
        );
        if coarse < lo || fine > hi {
            bad.push(format!(
                "{area}: levels {coarse}..{fine} outside {lo}..{hi}"
            ));
        }
        // Containment, judged by s2sphere: each of its interior points must
        // have one of its own ancestors in the covering.
        for pt in row["points"].as_array().expect("points") {
            points += 1;
            let ancestors: Vec<&str> = pt["ancestors"]
                .as_array()
                .unwrap()
                .iter()
                .map(|t| t.as_str().unwrap())
                .collect();
            if !ancestors.iter().any(|t| cells.contains(t)) {
                bad.push(format!(
                    "{area}: ({}, {}) is inside the region and in no cell of the covering",
                    pt["lat"], pt["lon"]
                ));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "{} problems:\n{}",
        bad.len(),
        bad[..bad.len().min(20)].join("\n")
    );
    assert_eq!(regions, 16, "{regions} regions covered");
    assert_eq!(points, 128, "{points} points checked");
}
