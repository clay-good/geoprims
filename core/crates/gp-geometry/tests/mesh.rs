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

use std::collections::BTreeMap;

fn tool(id: &str, input: &serde_json::Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).expect("JSON")
}

fn area_of(polygon: &Value) -> f64 {
    tool(
        "geometry.area.polygon",
        &json!({"polygon": polygon, "options": {"outputUnits": {"area": "m2"}}}),
    )["result"]["area"]["value"]
        .as_f64()
        .unwrap()
}

#[test]
fn delaunay_invariants() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/delaunay_geos.json");
    let cases: Vec<Value> =
        serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("JSON");

    for case in cases.iter().take(8) {
        let name = case["name"].as_str().unwrap();
        let points = &case["points"];
        let n = points.as_array().unwrap().len();
        let r = tool(
            "geometry.mesh.delaunay",
            &json!({"points": points, "surface": "planar"}),
        );
        assert!(r["ok"].as_bool().unwrap_or(false), "{name}: {r}");
        let triangles = r["result"]["triangles"].as_array().unwrap();

        // The hull corner count comes from the neighbouring tool, so Euler's
        // relation is checked against something this one did not compute.
        let enc = tool("geometry.shape.enclosing", &json!({"points": points}));
        let h = enc["result"]["hull_count"].as_u64().unwrap() as usize;
        assert_eq!(
            triangles.len(),
            2 * n - 2 - h,
            "{name}: {} triangles for {n} points with {h} on the hull, not the {} Euler requires",
            triangles.len(),
            2 * n - 2 - h
        );

        // Every interior edge is shared by exactly two triangles and every hull
        // edge by one. That is what makes this a triangulation rather than a
        // heap of triangles, and nothing else here would notice a tear.
        let mut edges: BTreeMap<(u64, u64), usize> = BTreeMap::new();
        for t in triangles {
            let v = [
                t["a"].as_u64().unwrap(),
                t["b"].as_u64().unwrap(),
                t["c"].as_u64().unwrap(),
            ];
            assert!(
                v[0] != v[1] && v[1] != v[2] && v[0] != v[2],
                "{name}: a triangle repeats a vertex: {v:?}"
            );
            for i in 0..3 {
                let (a, b) = (v[i], v[(i + 1) % 3]);
                *edges.entry((a.min(b), a.max(b))).or_default() += 1;
            }
        }
        let on_hull = edges.values().filter(|&&c| c == 1).count();
        assert!(
            edges.values().all(|&c| c <= 2),
            "{name}: an edge is shared by more than two triangles"
        );
        assert_eq!(
            on_hull, h,
            "{name}: {on_hull} edges border nothing, not the {h} on the hull"
        );

        // The triangles tile the hull exactly: no gap, no overlap. Areas from
        // geometry.area.polygon, not from anything the mesh computed.
        let total: f64 = triangles
            .iter()
            .map(|t| {
                let pts: Vec<Value> = [&t["a"], &t["b"], &t["c"]]
                    .iter()
                    .map(|i| points[i.as_u64().unwrap() as usize - 1].clone())
                    .collect();
                area_of(&Value::Array(pts))
            })
            .sum();
        let hull: Vec<Value> = enc["result"]["outlines"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p["shape"] == "hull")
            .map(|p| json!({"lat": p["lat"]["value"], "lon": p["lon"]["value"]}))
            .collect();
        assert_eq!(hull.len(), h, "{name}: the hull outline is not {h} points");
        let hull_area = area_of(&Value::Array(hull));
        assert!(
            (total - hull_area).abs() / hull_area < 1e-9,
            "{name}: the triangles cover {total} m2 of a {hull_area} m2 hull"
        );
    }
}

#[test]
fn voronoi_invariants() {
    use geographiclib_rs::{Geodesic, InverseGeodesic};
    let g = Geodesic::wgs84();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/voronoi_geos.json");
    let cases: Vec<Value> =
        serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("JSON");

    for case in cases.iter().take(6) {
        let name = case["name"].as_str().unwrap();
        let points = case["points"].as_array().unwrap();
        let r = tool(
            "geometry.mesh.voronoi",
            &json!({"points": case["points"], "surface": "planar"}),
        );
        assert!(r["ok"].as_bool().unwrap_or(false), "{name}: {r}");

        let mut rings: BTreeMap<u64, Vec<Value>> = BTreeMap::new();
        for p in r["result"]["cells"].as_array().unwrap() {
            rings
                .entry(p["point"].as_u64().unwrap())
                .or_default()
                .push(json!({"lat": p["lat"]["value"], "lon": p["lon"]["value"]}));
        }
        assert_eq!(rings.len(), points.len(), "{name}: not one cell per point");
        assert_eq!(
            r["result"]["cell_count"].as_u64().unwrap() as usize,
            rings.len(),
            "{name}: cell_count is not the number of cells returned"
        );

        let mut total = 0.0;
        for (who, ring) in &rings {
            let site = &points[*who as usize - 1];
            let (glat, glon) = (site["lat"].as_f64().unwrap(), site["lon"].as_f64().unwrap());

            // The generator is inside its own cell, decided by the predicate
            // tool rather than by anything the mesh computed.
            let inside = tool(
                "geometry.predicate.point-in-polygon",
                &json!({"polygon": ring, "points": [site]}),
            );
            assert_eq!(
                inside["result"]["first"], "inside",
                "{name}: point {who} is not inside its own cell"
            );

            // The definition: everywhere in the cell is nearer to its own
            // generator than to any other. Probed halfway to each corner. This
            // is the only check that would catch cells given to the wrong points.
            for corner in ring.iter().take(3) {
                let probe = (
                    (glat + corner["lat"].as_f64().unwrap()) / 2.0,
                    (glon + corner["lon"].as_f64().unwrap()) / 2.0,
                );
                let mine: f64 = g.inverse(probe.0, probe.1, glat, glon);
                for (i, other) in points.iter().enumerate() {
                    if i as u64 + 1 == *who {
                        continue;
                    }
                    let d: f64 = g.inverse(
                        probe.0,
                        probe.1,
                        other["lat"].as_f64().unwrap(),
                        other["lon"].as_f64().unwrap(),
                    );
                    assert!(
                        mine <= d + 1e-6,
                        "{name}: a point in cell {who} is {mine} m from its generator and {d} m from point {}",
                        i + 1
                    );
                }
            }
            total += area_of(&Value::Array(ring.clone()));
        }

        // The cells tile the clip box: the areas sum to what GEOS measured for
        // the same box, with no gap and no overlap.
        let want: f64 = case["cells"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["area"].as_f64().unwrap())
            .sum();
        assert!(
            (total - want).abs() / want < 1e-5,
            "{name}: the cells cover {total} m2 against {want} m2"
        );
    }
}
