//! How two shapes relate: the properties every answer must have, and
//! agreement with the neighbouring tools that answer part of the question
//! their own way.

use gp_geometry::REGISTRY;
use serde_json::{Value, json};

fn call(id: &str, input: Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).expect("JSON")
}

fn relate(a: &Value, ka: &str, b: &Value, kb: &str, edges: &str) -> Value {
    call(
        "geometry.predicate.relate",
        json!({"geometry_a": a, "kind_a": ka, "geometry_b": b, "kind_b": kb, "edges": edges}),
    )
}

fn transpose(m: &str) -> String {
    let c: Vec<char> = m.chars().collect();
    [0, 3, 6, 1, 4, 7, 2, 5, 8].iter().map(|&i| c[i]).collect()
}

fn answer(r: &Value, test: &str) -> bool {
    r["result"]["tests"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["test"] == test)
        .unwrap_or_else(|| panic!("no {test} in {r}"))["answer"]
        == "yes"
}

fn corners(pts: &[(f64, f64)]) -> Value {
    Value::Array(
        pts.iter()
            .map(|&(la, lo)| json!({"lat": la, "lon": lo}))
            .collect(),
    )
}

#[test]
fn relate_invariants() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/relate_geos.json");
    let mut cases: Vec<Value> =
        serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("JSON");
    cases.truncate(400);
    // The same pairs on the ground, moved to Colorado with a grid step of about
    // a metre, so the geodesic mode meets the grid's degeneracies too.
    let moved = |g: &Value| -> Value {
        Value::Array(
            g.as_array()
                .unwrap()
                .iter()
                .map(|v| {
                    let mut v = v.clone();
                    v["lat"] = json!(40.0 + v["lat"].as_f64().unwrap() * 1e-5);
                    v["lon"] = json!(-105.0 + v["lon"].as_f64().unwrap() * 1e-5);
                    v
                })
                .collect(),
        )
    };
    for (i, c) in cases.iter().enumerate() {
        let (ka, kb) = (c["kind_a"].as_str().unwrap(), c["kind_b"].as_str().unwrap());
        for (edges, a, b) in [
            ("planar", c["geometry_a"].clone(), c["geometry_b"].clone()),
            ("geodesic", moved(&c["geometry_a"]), moved(&c["geometry_b"])),
        ] {
            let ab = relate(&a, ka, &b, kb, edges);
            let ba = relate(&b, kb, &a, ka, edges);
            let (m, n) = (
                ab["result"]["matrix"]
                    .as_str()
                    .unwrap_or_else(|| panic!("{ab}")),
                ba["result"]["matrix"].as_str().unwrap(),
            );
            // Swapping the shapes transposes the matrix.
            assert_eq!(transpose(m), n, "case {i} {edges}: {m} against {n}");
            // Each named test and its converse.
            assert_eq!(
                answer(&ab, "within"),
                answer(&ba, "contains"),
                "case {i} {edges}"
            );
            assert_eq!(
                answer(&ab, "intersects"),
                !answer(&ab, "disjoint"),
                "case {i} {edges}"
            );
            for t in ["intersects", "equals", "touches", "crosses", "overlaps"] {
                assert_eq!(answer(&ab, t), answer(&ba, t), "case {i} {edges}: {t}");
            }
            // The exteriors always meet in an area.
            assert_eq!(&m[8..], "2", "case {i} {edges}");
        }
        // At this size a geodesic and a straight line in latitude and longitude
        // part by microns, far inside the 1 mm tolerance, while grid places not
        // on an edge stay centimetres from it, so the topology is the same.
        // (At 0.001° steps they part by about 12 mm, and a grid corner on a
        // straight edge is rightly off the geodesic.)
        let geo = relate(
            &moved(&c["geometry_a"]),
            ka,
            &moved(&c["geometry_b"]),
            kb,
            "geodesic",
        );
        assert_eq!(
            geo["result"]["matrix"], c["matrix"],
            "case {i} on the ground: {c}"
        );
    }

    // Against point in polygon, which decides by winding number on the ellipsoid.
    let field = corners(&[
        (40.0, -105.0),
        (40.0, -104.9),
        (40.08, -104.9),
        (40.08, -105.0),
    ]);
    let pts = [
        (40.04, -104.95),
        (40.0, -104.95),
        (40.08, -104.9),
        (39.99, -104.95),
        (40.0000107, -104.95),
        (40.1, -105.2),
    ];
    let pip = call(
        "geometry.predicate.point-in-polygon",
        json!({"polygon": field, "points": corners(&pts)}),
    );
    for (i, &p) in pts.iter().enumerate() {
        let r = relate(&corners(&[p]), "points", &field, "polygon", "geodesic");
        let verdict = pip["result"]["results"][i]["nonzero"].as_str().unwrap();
        let expect = match verdict {
            "inside" => "within",
            "on-boundary" => "touches",
            _ => "disjoint",
        };
        assert!(
            answer(&r, expect),
            "point {p:?}: point in polygon says {verdict}, relate says {r}"
        );
    }

    // Against the boolean overlay: interiors meet exactly when the overlap has area.
    let others = [
        [
            (40.04, -104.95),
            (40.04, -104.85),
            (40.12, -104.85),
            (40.12, -104.95),
        ],
        [
            (40.08, -104.95),
            (40.08, -104.85),
            (40.12, -104.85),
            (40.12, -104.95),
        ],
        [
            (41.0, -105.0),
            (41.0, -104.9),
            (41.08, -104.9),
            (41.08, -105.0),
        ],
        [
            (40.02, -104.98),
            (40.02, -104.92),
            (40.06, -104.92),
            (40.06, -104.98),
        ],
    ];
    for o in others {
        let o = corners(&o);
        let r = relate(&field, "polygon", &o, "polygon", "geodesic");
        let b = call(
            "geometry.overlay.boolean",
            json!({"polygon_a": field, "polygon_b": o, "operation": "intersection"}),
        );
        let overlap = b["result"]["parts"].as_f64().unwrap() > 0.0;
        assert_eq!(
            &r["result"]["matrix"].as_str().unwrap()[..1] == "2",
            overlap,
            "{o}: relate {r}, boolean {b}"
        );
    }
}
