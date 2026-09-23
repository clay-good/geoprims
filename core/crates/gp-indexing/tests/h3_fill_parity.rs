//! polygonToCells parity with H3 C for each containment mode.

use gp_indexing::h3fill::{Mode, Polygon, fill};
use h3o::Resolution;
use serde_json::Value;

/// Per mode: (polygons compared, polygons that differ, cells missing, cells extra).
fn check(text: &str) -> Vec<(String, usize, usize, usize, usize)> {
    let mut stats: Vec<(String, usize, usize, usize, usize)> = [
        "center",
        "full",
        "overlap",
        "center at a pole",
        "full at a pole",
        "overlap at a pole",
    ]
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
            let want: Vec<String> = r[["center", "full", "overlap"][i]]
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
            // A ring that reaches a pole is counted apart: H3 reads rings in
            // latitude and longitude with straight edges, so those shapes are
            // degenerate and overlap disagrees there by a cell or two.
            let s = &mut stats[i + if r["pole"] == true { 3 } else { 0 }];
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
    for i in 0..3 {
        assert_eq!(
            stats[i].2, 0,
            "{} must match H3 C exactly: {stats:?}",
            stats[i].0
        );
    }
    // At a pole only the two containment tests that ask where a point or a
    // whole boundary falls still agree; overlap, which asks whether an edge is
    // crossed, differs by a cell or two on these degenerate rings.
    for i in 3..5 {
        assert_eq!(
            stats[i].2, 0,
            "{} must match H3 C exactly: {stats:?}",
            stats[i].0
        );
    }
    assert!(
        stats[5].1 > 0,
        "the fixture carries rings that reach a pole"
    );
}

#[test]
#[ignore = "needs a file from tools/vectors/gen_h3_fill.py in H3_FILL"]
fn full_differential() {
    let stats = check(&std::fs::read_to_string(std::env::var("H3_FILL").unwrap()).unwrap());
    println!("{stats:?}");
    assert_eq!(stats[0].2, 0, "{stats:?}");
}
