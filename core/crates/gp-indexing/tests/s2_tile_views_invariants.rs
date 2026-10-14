//! The focused S2 and tile operations: what each must do, as their
//! derivation notes state.

use gp_indexing::REGISTRY;
use serde_json::{Value, json};

fn call(id: &str, input: Value) -> Value {
    let r: Value = serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).unwrap();
    assert_eq!(r["ok"], true, "{id} {input}: {r}");
    r["result"].clone()
}

fn num(r: &Value, k: &str) -> f64 {
    r[k]["value"]
        .as_f64()
        .or(r[k].as_f64())
        .unwrap_or_else(|| panic!("{k}: {r}"))
}

fn text(r: &Value, k: &str) -> String {
    match &r[k] {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        v => panic!("{k}: {v}"),
    }
}

fn cells(r: &Value, k: &str) -> Vec<String> {
    r[k].as_array()
        .map(|a| a.iter().map(|c| text(c, "cell")).collect())
        .unwrap_or_default()
}

fn corners(cell: &str) -> Vec<[f64; 3]> {
    let r = call("indexing.s2.vertices", json!({ "cell": cell }));
    r["boundary"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            let (lat, lon) = (num(p, "lat").to_radians(), num(p, "lon").to_radians());
            [lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin()]
        })
        .collect()
}

fn sample() -> Vec<String> {
    let mut out: Vec<String> = [
        (40.446111, -79.982222, 12),
        (-33.8688, 151.2093, 5),
        (64.1466, -21.9426, 20),
        (0.0, 0.0, 1),
        (89.9, 45.0, 9),
    ]
    .iter()
    .map(|(lat, lon, level)| {
        text(
            &call(
                "indexing.s2.lat-lng-to-cell",
                json!({"lat": lat, "lon": lon, "level": level}),
            ),
            "cell",
        )
    })
    .collect();
    out.push("3".into()); // a face cell, level 0
    out
}

#[test]
fn s2_views_invariants() {
    // The six faces' areas add up to the sphere, six times the level-0 average.
    let faces = ["1", "3", "5", "7", "9", "b"];
    let total: f64 = faces
        .iter()
        .map(|f| num(&call("indexing.s2.area", json!({ "cell": f })), "area"))
        .sum();
    let avg = num(
        &call("indexing.s2.area", json!({ "cell": "1" })),
        "average_area",
    );
    assert!(
        (total - 6.0 * avg).abs() <= 1e-9 * total,
        "{total} vs {}",
        6.0 * avg
    );

    for cell in sample() {
        let token = call("indexing.s2.token", json!({ "cell": cell }));
        let level = num(&token, "level") as u32;
        // Token to id and back returns the token; a long decimal is read as an id.
        let id = text(&token, "cell_id");
        assert!(id.len() >= 17, "{id}");
        assert_eq!(
            text(&call("indexing.s2.token", json!({ "cell": id })), "cell"),
            cell
        );

        let h = call("indexing.s2.hierarchy", json!({ "cell": cell }));
        let kids = cells(&h, "children");
        // Each child's parent is the cell, and one level finer.
        for kid in &kids {
            let up = call("indexing.s2.hierarchy", json!({ "cell": kid }));
            assert_eq!(text(&up, "parent"), cell);
            assert_eq!(num(&up, "level") as u32, level + 1);
        }
        // Level 0 has no parent; a parent is one level coarser.
        if level == 0 {
            assert!(h.get("parent").is_none_or(Value::is_null), "{h}");
        } else {
            let parent = text(&h, "parent");
            let p = call("indexing.s2.token", json!({ "cell": parent }));
            assert_eq!(num(&p, "level") as u32, level - 1);
        }
        // Stepping up parent by parent reaches the ancestor at any level.
        if level >= 6 {
            let target = call("indexing.s2.hierarchy", json!({"cell": cell, "level": 6}));
            let mut up = cell.clone();
            for _ in 6..level {
                up = text(
                    &call("indexing.s2.hierarchy", json!({ "cell": up })),
                    "parent",
                );
            }
            assert_eq!(text(&target, "ancestor"), up);
        }

        // Areas: positive, near the level average, and the children's sum to it.
        let a = call("indexing.s2.area", json!({ "cell": cell }));
        let ratio = num(&a, "ratio");
        assert!((0.5..1.5).contains(&ratio), "{cell}: {ratio}");
        let sum: f64 = kids
            .iter()
            .map(|k| num(&call("indexing.s2.area", json!({ "cell": k })), "area"))
            .sum();
        assert!((sum - num(&a, "area")).abs() <= 1e-9 * sum, "{cell}");

        // Corners: four, counterclockwise from outside, each a neighbor's corner too.
        let c = corners(&cell);
        assert_eq!(c.len(), 4);
        for i in 0..4 {
            let (p, q, r) = (c[i], c[(i + 1) % 4], c[(i + 2) % 4]);
            let (u, v) = (
                [q[0] - p[0], q[1] - p[1], q[2] - p[2]],
                [r[0] - q[0], r[1] - q[1], r[2] - q[2]],
            );
            let n = [
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            ];
            assert!(
                n[0] * q[0] + n[1] * q[1] + n[2] * q[2] > 0.0,
                "{cell} corner {i}"
            );
        }
        let near: Vec<[f64; 3]> = cells(
            &call("indexing.s2.neighbors", json!({ "cell": cell })),
            "neighbors",
        )
        .iter()
        .flat_map(|n| corners(n))
        .collect();
        for p in &c {
            assert!(
                near.iter()
                    .any(|q| (0..3).all(|k| (p[k] - q[k]).abs() < 1e-12)),
                "{cell}: a corner no neighbor shares"
            );
        }
    }
    // Level 30 has no children.
    let leaf = text(
        &call(
            "indexing.s2.lat-lng-to-cell",
            json!({"lat": 51.4779, "lon": -0.0015, "level": 30}),
        ),
        "cell",
    );
    assert!(
        cells(
            &call("indexing.s2.hierarchy", json!({ "cell": leaf })),
            "children"
        )
        .is_empty()
    );
}

#[test]
fn tile_views_invariants() {
    // At zoom 0 both schemes are 0/0/0.
    let z0 = call(
        "indexing.tile.xyz-tms",
        json!({"tile": "0/0/0", "convention": "xyz"}),
    );
    assert_eq!(
        (text(&z0, "xyz"), text(&z0, "tms")),
        ("0/0/0".into(), "0/0/0".into())
    );
    for (z, x, y) in [
        (1, 1, 0),
        (10, 300, 385),
        (14, 4551, 6176),
        (19, 140000, 210000),
    ] {
        let tile = format!("{z}/{x}/{y}");
        // Converting twice returns the tile; the column and zoom never change.
        let t = call(
            "indexing.tile.xyz-tms",
            json!({"tile": tile, "convention": "xyz"}),
        );
        let tms = text(&t, "tms");
        let parts: Vec<&str> = tms.split('/').collect();
        assert_eq!(
            (parts[0], parts[1]),
            (z.to_string().as_str(), x.to_string().as_str())
        );
        let back = call(
            "indexing.tile.xyz-tms",
            json!({"tile": tms, "convention": "tms"}),
        );
        assert_eq!(text(&back, "xyz"), tile);

        // Quadkeys: reading one back gives the tile, a tile's begins with its
        // parent's, and a TMS tile gives the same quadkey as its XYZ twin.
        let q = text(
            &call("indexing.tile.tile-quadkey", json!({ "tile": tile })),
            "quadkey",
        );
        assert_eq!(q.len(), z);
        assert_eq!(
            text(
                &call("indexing.tile.tile-quadkey", json!({ "tile": q })),
                "xyz"
            ),
            tile
        );
        let parent = format!("{}/{}/{}", z - 1, x / 2, y / 2);
        let pq = text(
            &call("indexing.tile.tile-quadkey", json!({ "tile": parent })),
            "quadkey",
        );
        assert!(q.starts_with(&pq), "{q} {pq}");
        let from_tms = call(
            "indexing.tile.tile-quadkey",
            json!({"tile": tms, "convention": "tms"}),
        );
        assert_eq!(text(&from_tms, "quadkey"), q);
    }
}
