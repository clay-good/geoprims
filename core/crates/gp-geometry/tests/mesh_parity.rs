//! Delaunay triangulation against GEOS, triangle for triangle.
//!
//! A triangulation of points in general position is unique, so this is one of
//! the few checks in the catalog that can demand exact agreement rather than
//! closeness. The fixture holds GEOS's own answer for each point set; see
//! tools/vectors/gen_delaunay_geos.py.

use std::collections::{BTreeMap, BTreeSet};

use gp_geometry::REGISTRY;
use serde_json::{Value, json};

fn fixture() -> Vec<Value> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/delaunay_geos.json");
    serde_json::from_str(
        &std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path}: {e}")),
    )
    .expect("JSON")
}

fn triangulate(points: &Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(
        "geometry.mesh.delaunay",
        &json!({"points": points, "surface": "planar"}).to_string(),
    ))
    .expect("JSON")
}

/// The triangulation as a set of sorted index triples, so neither the order of
/// the triangles nor the order within one can hide a difference.
fn as_set(triangles: &Value) -> BTreeSet<[u64; 3]> {
    triangles
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            let mut v = [
                t["a"].as_u64().unwrap(),
                t["b"].as_u64().unwrap(),
                t["c"].as_u64().unwrap(),
            ];
            v.sort_unstable();
            v
        })
        .collect()
}

#[test]
fn delaunay_matches_geos_triangle_for_triangle() {
    let cases = fixture();
    assert!(
        cases.len() >= 15,
        "only {} cases in the fixture",
        cases.len()
    );
    for case in &cases {
        let name = case["name"].as_str().unwrap();
        let r = triangulate(&case["points"]);
        assert!(r["ok"].as_bool().unwrap_or(false), "{name}: {r}");

        let want: BTreeSet<[u64; 3]> = case["triangles"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| {
                let mut v = [
                    t[0].as_u64().unwrap(),
                    t[1].as_u64().unwrap(),
                    t[2].as_u64().unwrap(),
                ];
                v.sort_unstable();
                v
            })
            .collect();
        let got = as_set(&r["result"]["triangles"]);

        assert_eq!(
            got.len(),
            r["result"]["triangle_count"].as_u64().unwrap() as usize,
            "{name}: triangle_count disagrees with the triangles returned, or two are identical"
        );
        assert_eq!(
            got,
            want,
            "{name}: the triangulation differs from GEOS's.\n  only ours: {:?}\n  only GEOS: {:?}",
            got.difference(&want).collect::<Vec<_>>(),
            want.difference(&got).collect::<Vec<_>>()
        );

        // Euler's relation for a triangulation of n points with h on the hull:
        // 2n - 2 - h triangles. Every vertex must also appear somewhere, or a
        // point was silently dropped.
        let n = case["points"].as_array().unwrap().len() as u64;
        let used: BTreeSet<u64> = got.iter().flatten().copied().collect();
        assert_eq!(
            used.len() as u64,
            n,
            "{name}: {} of {n} points appear in the triangulation",
            used.len()
        );
        assert!(
            used.iter().all(|&i| (1..=n).contains(&i)),
            "{name}: a triangle refers to a point that does not exist"
        );
    }
    eprintln!("Delaunay: {} point sets match GEOS exactly", cases.len());
}

fn voronoi(points: &Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(
        "geometry.mesh.voronoi",
        &json!({"points": points, "surface": "planar"}).to_string(),
    ))
    .expect("JSON")
}

fn cell_area(ring: &[Value]) -> f64 {
    let r: Value = serde_json::from_str(&REGISTRY.invoke(
        "geometry.area.polygon",
        &json!({"polygon": ring, "options": {"outputUnits": {"area": "m2"}}}).to_string(),
    ))
    .expect("JSON");
    r["result"]["area"]["value"].as_f64().unwrap_or(0.0)
}

#[test]
fn voronoi_matches_geos_cell_for_cell() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/voronoi_geos.json");
    let cases: Vec<Value> =
        serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("JSON");
    assert!(
        cases.len() >= 15,
        "only {} cases in the fixture",
        cases.len()
    );

    let mut worst = 0.0f64;
    for case in &cases {
        let name = case["name"].as_str().unwrap();
        let r = voronoi(&case["points"]);
        assert!(r["ok"].as_bool().unwrap_or(false), "{name}: {r}");

        // Gather the returned corners by the generator they belong to.
        let mut rings: BTreeMap<u64, Vec<Value>> = BTreeMap::new();
        for p in r["result"]["cells"].as_array().unwrap() {
            rings
                .entry(p["point"].as_u64().unwrap())
                .or_default()
                .push(json!({"lat": p["lat"]["value"], "lon": p["lon"]["value"]}));
        }
        assert_eq!(
            rings.len(),
            r["result"]["cell_count"].as_u64().unwrap() as usize,
            "{name}: cell_count is not the number of cells actually returned"
        );

        for want in case["cells"].as_array().unwrap() {
            let who = want["point"].as_u64().unwrap();
            let ring = rings
                .get(&who)
                .unwrap_or_else(|| panic!("{name}: no cell for point {who}"));
            assert_eq!(
                ring.len(),
                want["corners"].as_u64().unwrap() as usize,
                "{name}: cell {who} has {} corners, GEOS gives {}",
                ring.len(),
                want["corners"]
            );
            let got = cell_area(ring);
            let expected = want["area"].as_f64().unwrap();
            let rel = (got - expected).abs() / expected;
            assert!(
                rel < 1e-5,
                "{name}: cell {who} covers {got} m2, GEOS gives {expected} m2"
            );
            worst = worst.max(rel);
        }
    }
    eprintln!(
        "Voronoi: {} diagrams match GEOS, worst cell area {worst:.2e} relative",
        cases.len()
    );
}
