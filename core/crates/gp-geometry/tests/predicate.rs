//! Point in polygon: the properties the answer must have, and the places where
//! it is known without computing anything.

use gp_geometry::REGISTRY;
use serde_json::{Value, json};

fn pip(polygon: &Value, points: &Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(
        "geometry.predicate.point-in-polygon",
        &json!({"polygon": polygon, "points": points}).to_string(),
    ))
    .expect("JSON")
}

fn centroid(polygon: &Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(
        "geometry.shape.centroid",
        &json!({"polygon": polygon}).to_string(),
    ))
    .expect("JSON")
}

fn at(r: &Value, i: usize, key: &str) -> Value {
    r["result"]["results"][i][key].clone()
}

#[test]
fn point_in_polygon_invariants() {
    let square = json!([{"lat":40.0,"lon":-105.0},{"lat":40.0,"lon":-104.9},
                        {"lat":40.08,"lon":-104.9},{"lat":40.08,"lon":-105.0}]);
    let l_shape = json!([{"lat":40.0,"lon":-105.0},{"lat":40.0,"lon":-104.9},
                         {"lat":40.04,"lon":-104.9},{"lat":40.04,"lon":-104.95},
                         {"lat":40.08,"lon":-104.95},{"lat":40.08,"lon":-105.0}]);

    for (name, poly) in [("a square", &square), ("an L", &l_shape)] {
        // The neighbouring tool's own points, tested here. Nothing in this
        // crate computed both sides of that.
        let c = centroid(poly);
        let picked = json!([
            {"lat": c["result"]["interior_lat"]["value"], "lon": c["result"]["interior_lon"]["value"]},
        ]);
        let r = pip(poly, &picked);
        assert_eq!(
            at(&r, 0, "nonzero"),
            "inside",
            "{name}: the interior point from geometry.shape.centroid is not inside\n{r}"
        );
        if c["result"]["centroid_inside"] == "yes" {
            let cp = json!([
                {"lat": c["result"]["centroid_lat"]["value"], "lon": c["result"]["centroid_lon"]["value"]},
            ]);
            assert_eq!(
                at(&pip(poly, &cp), 0, "nonzero"),
                "inside",
                "{name}: the centroid is reported inside by one tool and not the other"
            );
        }

        // Every vertex is on the boundary, and at no distance from it. This is
        // the one place the answer is known without computing anything.
        let verts = pip(poly, poly);
        for (i, _) in poly.as_array().unwrap().iter().enumerate() {
            assert_eq!(
                at(&verts, i, "nonzero"),
                "on-boundary",
                "{name}: vertex {i} is not on its own boundary\n{verts}"
            );
            let d = at(&verts, i, "distance")["value"].as_f64().unwrap();
            assert!(d < 1e-6, "{name}: vertex {i} is {d} m from the boundary");
        }

        // The two rules agree on a simple outline, and inside_count counts
        // exactly the points the winding rule called inside.
        let scattered = json!([
            {"lat":40.02,"lon":-104.98},{"lat":40.06,"lon":-104.92},
            {"lat":40.2,"lon":-104.95},{"lat":39.9,"lon":-104.95},
            {"lat":40.04,"lon":-105.3}
        ]);
        let s = pip(poly, &scattered);
        let mut inside = 0;
        for i in 0..scattered.as_array().unwrap().len() {
            assert_eq!(
                at(&s, i, "nonzero"),
                at(&s, i, "even_odd"),
                "{name}: the two rules disagree on point {i} of a simple outline\n{s}"
            );
            if at(&s, i, "nonzero") == "inside" {
                inside += 1;
            }
        }
        assert_eq!(
            s["result"]["inside_count"].as_i64().unwrap(),
            inside,
            "{name}: inside_count is not the number reported inside\n{s}"
        );

        // Inside does not depend on which way the outline was drawn.
        let reversed: Vec<Value> = poly.as_array().unwrap().iter().rev().cloned().collect();
        let rev = pip(&Value::Array(reversed), &scattered);
        for i in 0..scattered.as_array().unwrap().len() {
            assert_eq!(
                at(&rev, i, "nonzero"),
                at(&s, i, "nonzero"),
                "{name}: reversing the winding changed point {i}"
            );
        }

        // Far outside is exactly zero, not nearly zero.
        let far = pip(poly, &json!([{"lat":10.0,"lon":10.0}]));
        assert_eq!(at(&far, 0, "winding").as_i64().unwrap(), 0, "{far}");
        assert_eq!(at(&far, 0, "nonzero"), "outside", "{far}");
    }

    // The distance grows as a point is walked away from a vertex.
    let corner = 40.0;
    let mut last = -1.0;
    for step in [0.0, 0.001, 0.01, 0.1] {
        let r = pip(
            &square,
            &json!([{"lat": corner - step, "lon": -105.0 - step}]),
        );
        let d = at(&r, 0, "distance")["value"].as_f64().unwrap();
        assert!(
            d >= last,
            "the distance fell at step {step}: {d} after {last}"
        );
        last = d;
    }
}

/// Planar edges against GEOS, exactly: 300 grid polygons (some with a hole)
/// and 3,600 points on the grid and on half steps, 146 of them on an edge or
/// a corner (tools/vectors/gen_pip_planar.py).
#[test]
fn planar_point_in_polygon_matches_geos() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/pip_planar_geos.json"
    );
    let cases: Vec<Value> =
        serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("JSON");
    let mut wrong = Vec::new();
    for (i, c) in cases.iter().enumerate() {
        let mut polygon: Vec<Value> = c["outline"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| json!({"lat": p[0], "lon": p[1]}))
            .collect();
        for (k, h) in c["holes"].as_array().unwrap().iter().enumerate() {
            for p in h.as_array().unwrap() {
                polygon.push(json!({"lat": p[0], "lon": p[1], "ring": k + 1}));
            }
        }
        let points: Vec<Value> = c["points"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| json!({"lat": p[0], "lon": p[1]}))
            .collect();
        let r: Value = serde_json::from_str(&REGISTRY.invoke(
            "geometry.predicate.point-in-polygon",
            &json!({"polygon": polygon, "points": points, "edges": "planar"}).to_string(),
        ))
        .expect("JSON");
        for (j, want) in c["expect"].as_array().unwrap().iter().enumerate() {
            let got = &r["result"]["results"][j];
            if &got["nonzero"] != want || &got["even_odd"] != want {
                wrong.push(format!("polygon {i} point {j}: {got}, GEOS {want}"));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} differ:\n{}",
        wrong.len(),
        wrong[..wrong.len().min(10)].join("\n")
    );
    // A pentagram's middle is wound twice: inside by the winding rule, outside
    // by even-odd, with planar edges as with geodesic ones.
    let star = json!([{"lat":0.0,"lon":0.0},{"lat":0.0,"lon":10.0},{"lat":-6.0,"lon":2.0},
                      {"lat":4.0,"lon":5.0},{"lat":-6.0,"lon":8.0}]);
    let r = pip_planar(&star, &json!([{"lat": -1.5, "lon": 5.0}]));
    assert_eq!(at(&r, 0, "winding").as_i64().map(i64::abs), Some(2), "{r}");
    assert_eq!(at(&r, 0, "nonzero"), "inside");
    assert_eq!(at(&r, 0, "even_odd"), "outside");
}

fn pip_planar(polygon: &Value, points: &Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(
        "geometry.predicate.point-in-polygon",
        &json!({"polygon": polygon, "points": points, "edges": "planar"}).to_string(),
    ))
    .expect("JSON")
}
