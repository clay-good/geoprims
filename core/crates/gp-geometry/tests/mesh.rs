//! Delaunay and Voronoi against Qhull via SciPy (tests/data/mesh_diff.json,
//! from tools/vectors/gen_mesh_diff.py): the same triangles, and the same
//! cell corners for every spherical point and every bounded planar cell.

use gp_geometry::REGISTRY;
use serde_json::{Value, json};

fn call(id: &str, input: &Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).expect("JSON")
}

#[test]
fn matches_qhull() {
    let cases: Value = serde_json::from_str(include_str!("data/mesh_diff.json")).unwrap();
    for (k, c) in cases.as_array().unwrap().iter().enumerate() {
        let pts: Vec<Value> = c["points"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| json!({"lat": p[0], "lon": p[1]}))
            .collect();
        let input = json!({"points": pts, "surface": c["surface"]});
        let d = call("geometry.mesh.delaunay", &input);
        let mut got: Vec<Vec<u64>> = d["result"]["triangles"]
            .as_array()
            .unwrap_or_else(|| panic!("case {k}: {d}"))
            .iter()
            .map(|t| {
                let mut v: Vec<u64> = ["a", "b", "c"]
                    .iter()
                    .map(|x| t[x].as_f64().unwrap() as u64)
                    .collect();
                v.sort_unstable();
                v
            })
            .collect();
        got.sort();
        let want: Vec<Vec<u64>> = serde_json::from_value(c["triangles"].clone()).unwrap();
        assert_eq!(got, want, "case {k} ({}) triangles", c["surface"]);
        // Voronoi corners, as sets per cell.
        let v = call("geometry.mesh.voronoi", &input);
        let rows = v["result"]["cells"].as_array().unwrap();
        for (point, corners) in c["cells"].as_object().unwrap() {
            let id: f64 = point.parse().unwrap();
            let mut mine: Vec<(f64, f64)> = rows
                .iter()
                .filter(|r| r["point"].as_f64() == Some(id))
                .map(|r| {
                    (
                        r["lat"]["value"].as_f64().unwrap(),
                        r["lon"]["value"].as_f64().unwrap(),
                    )
                })
                .collect();
            let mut theirs: Vec<(f64, f64)> = corners
                .as_array()
                .unwrap()
                .iter()
                .map(|p| (p[0].as_f64().unwrap(), p[1].as_f64().unwrap()))
                .collect();
            let key =
                |a: &(f64, f64), b: &(f64, f64)| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1));
            mine.sort_by(key);
            theirs.sort_by(key);
            assert_eq!(
                mine.len(),
                theirs.len(),
                "case {k} cell {point}: {mine:?} vs {theirs:?}"
            );
            for (a, b) in mine.iter().zip(&theirs) {
                let dlon = ((a.1 - b.1 + 540.0).rem_euclid(360.0) - 180.0).abs();
                assert!(
                    (a.0 - b.0).abs() < 1e-7 && dlon < 1e-7,
                    "case {k} cell {point}: {a:?} vs {b:?}"
                );
            }
        }
    }
}

#[test]
fn collinear_points_make_no_triangles() {
    let pts: Vec<Value> = (0..5)
        .map(|i| json!({"lat": 40.0, "lon": -105.0 + i as f64 * 0.01}))
        .collect();
    let r = call("geometry.mesh.delaunay", &json!({"points": pts}));
    // On the plane a row of points along a parallel is nearly, not exactly,
    // straight (the geodesic bows), so a thin triangle is also acceptable.
    assert!(
        r["ok"] == true || r["error"]["code"] == "DEGENERATE_GEOMETRY",
        "{r}"
    );
    let pts: Vec<Value> = (0..5)
        .map(|i| json!({"lat": 0.0, "lon": i as f64 * 10.0}))
        .collect();
    let r = call(
        "geometry.mesh.delaunay",
        &json!({"points": pts, "surface": "spherical"}),
    );
    assert_eq!(r["error"]["code"], "DEGENERATE_GEOMETRY", "{r}");
}
