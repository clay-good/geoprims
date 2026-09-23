//! Densifying: the spacing promise, and the exact-multiple case that the
//! ceiling used to decide by the last bit of a double.

use gp_geometry::REGISTRY;
use serde_json::{Value, json};

fn densify(points: &Value, max: &str, shape: &str) -> Value {
    serde_json::from_str(
        &REGISTRY.invoke(
            "geometry.shape.densify",
            &json!({"points": points, "max_length": max, "shape": shape,
                "options": {"outputUnits": {"longest_piece": "m", "length": "m"}}})
            .to_string(),
        ),
    )
    .expect("JSON")
}

fn out_points(r: &Value) -> Vec<(f64, f64)> {
    r["result"]["densified"]
        .as_array()
        .unwrap_or_else(|| panic!("{r}"))
        .iter()
        .map(|p| {
            (
                p["lat"]["value"].as_f64().unwrap(),
                p["lon"]["value"].as_f64().unwrap(),
            )
        })
        .collect()
}

#[test]
fn densify_invariants() {
    use geographiclib_rs::{Geodesic, InverseGeodesic};
    let g = Geodesic::wgs84();

    let line =
        json!([{"lat":40.0,"lon":-105.0},{"lat":40.1,"lon":-104.8},{"lat":40.3,"lon":-104.6}]);
    let square = json!([{"lat":40.0,"lon":-105.0},{"lat":40.0,"lon":-104.9},
                        {"lat":40.08,"lon":-104.9},{"lat":40.08,"lon":-105.0}]);

    for (name, pts, shape) in [("a line", &line, "line"), ("a polygon", &square, "polygon")] {
        // Long enough to need no cutting: the shape comes back as it went in.
        let untouched = densify(pts, "100000 m", shape);
        let n_in = pts.as_array().unwrap().len();
        assert_eq!(
            untouched["result"]["vertices_out"].as_u64().unwrap() as usize,
            n_in,
            "{name}: a maximum longer than every edge still added vertices"
        );
        let total = untouched["result"]["length"]["value"].as_f64().unwrap();

        let mut previous = 0usize;
        for max in [5000.0, 2500.0, 1250.0] {
            let r = densify(pts, &format!("{max} m"), shape);
            let got = out_points(&r);

            // The promise: nothing further apart than the maximum. Measured
            // with geographiclib-rs, not read off the tool's own report.
            let ring = shape == "polygon";
            let pairs = if ring { got.len() } else { got.len() - 1 };
            for i in 0..pairs {
                let (a, b) = (got[i], got[(i + 1) % got.len()]);
                let d: f64 = g.inverse(a.0, a.1, b.0, b.1);
                assert!(
                    d <= max + 1e-6,
                    "{name} at {max} m: two vertices are {d} m apart"
                );
            }

            // Every input point survives, in order: this adds, never moves.
            let mut j = 0;
            for p in pts.as_array().unwrap() {
                let want = (p["lat"].as_f64().unwrap(), p["lon"].as_f64().unwrap());
                while j < got.len()
                    && ((got[j].0 - want.0).abs() > 1e-12 || (got[j].1 - want.1).abs() > 1e-12)
                {
                    j += 1;
                }
                assert!(j < got.len(), "{name} at {max} m: an input point was lost");
                j += 1;
            }

            // Adding points along a geodesic does not lengthen it.
            let len = r["result"]["length"]["value"].as_f64().unwrap();
            assert!(
                (len - total).abs() / total < 1e-9,
                "{name} at {max} m: the length changed from {total} to {len}"
            );

            // Halving the maximum cannot need fewer vertices.
            let kept = r["result"]["vertices_out"].as_u64().unwrap() as usize;
            assert!(
                kept >= previous,
                "{name}: {max} m gave fewer vertices than a looser one"
            );
            previous = kept;
        }
    }

    // The case the snapping exists for. This line is 2 km long to within a
    // femtometre; at a 1 km maximum it must be two pieces, not three, whichever
    // side of the last bit the inverse problem lands on.
    let exact = json!([{"lat":0.0,"lon":0.0},{"lat":0.0,"lon":0.0179663055841}]);
    let r = densify(&exact, "1000 m", "line");
    assert_eq!(
        r["result"]["vertices_out"].as_u64().unwrap(),
        3,
        "an edge of exactly two maximum lengths was not cut into two pieces\n{r}"
    );

    // A polygon closes and a line does not, so the same points give one more
    // edge and a longer total.
    let as_line = densify(&square, "2500 m", "line");
    let as_polygon = densify(&square, "2500 m", "polygon");
    assert!(
        as_polygon["result"]["length"]["value"].as_f64().unwrap()
            > as_line["result"]["length"]["value"].as_f64().unwrap(),
        "the polygon is not longer than the line over the same points"
    );
}
