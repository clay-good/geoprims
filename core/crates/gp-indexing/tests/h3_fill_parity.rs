//! polygonToCells parity with H3 C for each containment mode.

use gp_indexing::h3fill::{Mode, Polygon, fill};
use h3o::Resolution;
use serde_json::Value;

/// Per mode: (polygons compared, polygons that differ, cells missing, cells extra).
fn check(text: &str) -> Vec<(String, usize, usize, usize, usize)> {
    let mut stats: Vec<(String, usize, usize, usize, usize)> = ["center", "full", "overlap"]
        .iter()
        .map(|m| (m.to_string(), 0, 0, 0, 0))
        .collect();
    for line in text.lines().skip(1) {
        let r: Value = serde_json::from_str(line).unwrap();
        let ring = |v: &Value| -> Vec<(f64, f64)> {
            v.as_array()
                .unwrap()
                .iter()
                .map(|p| (p[0].as_f64().unwrap(), p[1].as_f64().unwrap()))
                .collect()
        };
        let mut rings = vec![ring(&r["outer"])];
        rings.extend(r["holes"].as_array().unwrap().iter().map(ring));
        let poly = Polygon::new(&rings);
        let res = Resolution::try_from(r["res"].as_u64().unwrap() as u8).unwrap();
        for (i, mode) in [Mode::Center, Mode::Full, Mode::Overlap]
            .into_iter()
            .enumerate()
        {
            let want: Vec<String> = r[&stats[i].0]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| c.as_str().unwrap().to_owned())
                .collect();
            let got: Vec<String> = fill(&poly, res, mode, 10_000_000)
                .unwrap()
                .iter()
                .map(|c| c.to_string())
                .collect();
            let s = &mut stats[i];
            s.1 += 1;
            if got != want {
                s.2 += 1;
                s.3 += want.iter().filter(|c| !got.contains(c)).count();
                s.4 += got.iter().filter(|c| !want.contains(c)).count();
            }
        }
    }
    stats
}

#[test]
fn committed_fixture() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/h3_fill_diff.jsonl"
    ))
    .unwrap();
    let stats = check(&text);
    println!("{stats:?}");
    assert_eq!(
        stats[0].2, 0,
        "center mode must match H3 C exactly: {stats:?}"
    );
}

#[test]
#[ignore = "needs a file from tools/vectors/gen_h3_fill.py in H3_FILL"]
fn full_differential() {
    let stats = check(&std::fs::read_to_string(std::env::var("H3_FILL").unwrap()).unwrap());
    println!("{stats:?}");
    assert_eq!(stats[0].2, 0, "{stats:?}");
}
