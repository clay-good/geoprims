//! cellsToMultiPolygon against H3 C 4.4.1 (via h3-py 4.4.2) on 400 cell sets:
//! solid patches, patches with a hole punched out, several separate patches,
//! sets around a pentagon, and sets across the antimeridian, at resolutions
//! 3-11 (tools/vectors/gen_h3_outline_diff.py).

use std::str::FromStr;

use gp_indexing::h3outline::{Ring, outline};
use h3o::CellIndex;
use serde_json::Value;

/// A ring as it compares: the closing point dropped, turned to start at its
/// lowest point, and run in whichever direction reads lower, so that two
/// tracings of one ring match however each happened to start or wind.
fn canon(ring: &Ring) -> Vec<(f64, f64)> {
    let mut pts = ring.clone();
    if pts.len() > 1 && near(pts[0], pts[pts.len() - 1]) {
        pts.pop();
    }
    let low = |v: &Vec<(f64, f64)>| {
        (0..v.len())
            .min_by(|&a, &b| v[a].0.total_cmp(&v[b].0).then(v[a].1.total_cmp(&v[b].1)))
            .unwrap_or(0)
    };
    let turn = |v: &Vec<(f64, f64)>| {
        let k = low(v);
        let mut out = v[k..].to_vec();
        out.extend_from_slice(&v[..k]);
        out
    };
    let forward = turn(&pts);
    pts.reverse();
    let backward = turn(&pts);
    let less = forward
        .iter()
        .zip(&backward)
        .find(|(a, b)| !near(**a, **b))
        .is_none_or(|(a, b)| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)).is_lt());
    if less { forward } else { backward }
}

fn near(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).abs() < 1e-7 && (a.1 - b.1).abs() < 1e-7
}

fn same_ring(a: &[(f64, f64)], b: &[(f64, f64)]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| near(*x, *y))
}

/// A polygon as it compares: its outer ring, then its holes in a fixed order.
fn shape(poly: &[Ring]) -> Vec<Vec<(f64, f64)>> {
    let mut rings: Vec<Vec<(f64, f64)>> = poly.iter().map(canon).collect();
    let outer = rings.remove(0);
    rings.sort_by(|a, b| a[0].0.total_cmp(&b[0].0).then(a[0].1.total_cmp(&b[0].1)));
    let mut out = vec![outer];
    out.extend(rings);
    out
}

fn sorted_shapes(polys: &[Vec<Ring>]) -> Vec<Vec<Vec<(f64, f64)>>> {
    let mut s: Vec<_> = polys.iter().map(|p| shape(p)).collect();
    s.sort_by(|a, b| {
        a[0][0]
            .0
            .total_cmp(&b[0][0].0)
            .then(a[0][0].1.total_cmp(&b[0][0].1))
    });
    s
}

#[test]
fn outlines_match_h3_c() {
    let text = include_str!("data/h3_outline_diff.jsonl");
    let mut checked = 0;
    for line in text.lines().skip(1).filter(|l| !l.trim().is_empty()) {
        let row: Value = serde_json::from_str(line).expect("fixture is JSON");
        let kind = row["kind"].as_str().expect("kind");
        let cells: Vec<CellIndex> = row["cells"]
            .as_array()
            .expect("cells")
            .iter()
            .map(|c| CellIndex::from_str(c.as_str().expect("token")).expect("an H3 cell"))
            .collect();
        let want: Vec<Vec<Ring>> = row["polys"]
            .as_array()
            .expect("polys")
            .iter()
            .map(|p| {
                p.as_array()
                    .expect("rings")
                    .iter()
                    .map(|r| {
                        r.as_array()
                            .expect("ring")
                            .iter()
                            .map(|pt| {
                                let v = pt.as_array().expect("point");
                                (v[0].as_f64().expect("lat"), v[1].as_f64().expect("lon"))
                            })
                            .collect::<Ring>()
                    })
                    .collect()
            })
            .collect();
        let got = outline(&cells).expect("one resolution");
        let (g, w) = (sorted_shapes(&got), sorted_shapes(&want));
        assert_eq!(
            g.len(),
            w.len(),
            "{kind} at res {}: {} polygons, H3 C gives {}",
            row["res"],
            g.len(),
            w.len()
        );
        for (gp, wp) in g.iter().zip(&w) {
            assert_eq!(
                gp.len(),
                wp.len(),
                "{kind} at res {}: {} rings, H3 C gives {}",
                row["res"],
                gp.len(),
                wp.len()
            );
            for (i, (gr, wr)) in gp.iter().zip(wp).enumerate() {
                assert!(
                    same_ring(gr, wr),
                    "{kind} at res {}: ring {i} differs ({} points, H3 C gives {})",
                    row["res"],
                    gr.len(),
                    wr.len()
                );
            }
        }
        checked += 1;
    }
    assert_eq!(checked, 400, "every fixture row is checked");
}
