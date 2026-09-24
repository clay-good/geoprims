//! S2's own RegionCoverer, ported in `s2exact`, against s2sphere on 400
//! random rectangles and caps (tools/vectors/gen_s2_coverer_parity.py): the
//! same cells, token for token.

use gp_indexing::s2exact::{Region, covering};
use serde_json::Value;

#[test]
fn coverings_match_s2sphere() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/s2_coverer_parity.jsonl"
    ))
    .unwrap();
    let mut wrong = Vec::new();
    let mut n = 0;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let r: Value = serde_json::from_str(line).unwrap();
        let f = |k: &str| r[k].as_f64().unwrap();
        let region = if r["kind"] == "rect" {
            Region::Rect {
                south: f("south"),
                north: f("north"),
                west: f("west"),
                east: f("east"),
            }
        } else {
            Region::Cap {
                lat: f("lat"),
                lon: f("lon"),
                radius: f("radius_m") / 6_371_008.8,
            }
        };
        let got: Vec<String> = covering(
            &region,
            f("min_level") as u8,
            f("max_level") as u8,
            f("max_cells") as usize,
        )
        .iter()
        .map(|c| c.token())
        .collect();
        let want: Vec<String> = r["cells"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t.as_str().unwrap().to_owned())
            .collect();
        n += 1;
        if got != want {
            wrong.push(format!("{line}\n  got  {got:?}"));
        }
    }
    assert!(n >= 400);
    assert!(
        wrong.is_empty(),
        "{} of {n} differ:\n{}",
        wrong.len(),
        wrong[..wrong.len().min(4)].join("\n")
    );
}
