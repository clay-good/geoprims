//! The focused H3 operations: what each must do, as their derivation notes
//! state, checked across hexagons and pentagons at several resolutions.

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
    r[k].as_str()
        .unwrap_or_else(|| panic!("{k}: {r}"))
        .to_owned()
}

fn yes(r: &Value, k: &str) -> bool {
    r[k] == true || r[k] == "yes"
}

fn list(r: &Value, key: &str, field: &str) -> Vec<String> {
    r[key]
        .as_array()
        .unwrap_or_else(|| panic!("{key}: {r}"))
        .iter()
        .map(|v| v[field].as_str().unwrap().to_owned())
        .collect()
}

/// Hexagons at several resolutions and places, and pentagons.
fn cells() -> Vec<String> {
    let mut out = Vec::new();
    for (lat, lon, res) in [
        (40.446111, -79.982222, 9),
        (-33.8688, 151.2093, 5),
        (64.1466, -21.9426, 0),
        (51.4779, -0.0015, 12),
        (-77.85, 166.67, 3),
    ] {
        out.push(text(
            &call(
                "indexing.h3.lat-lng-to-cell",
                json!({"lat": lat, "lon": lon, "resolution": res}),
            ),
            "cell",
        ));
    }
    out.extend(["830800fffffffff", "8009fffffffffff", "85080003fffffff"].map(String::from));
    out
}

fn resolution(cell: &str) -> u8 {
    num(
        &call("indexing.h3.resolution", json!({ "cell": cell })),
        "resolution",
    ) as u8
}

fn pentagon(cell: &str) -> bool {
    yes(
        &call("indexing.h3.is-pentagon", json!({ "cell": cell })),
        "pentagon",
    )
}

fn ring1(cell: &str) -> Vec<String> {
    list(
        &call("indexing.h3.grid-ring", json!({"cell": cell, "k": 1})),
        "cells",
        "cell",
    )
}

#[test]
fn h3_views_invariants() {
    let all = cells();
    assert!(all.iter().any(|c| pentagon(c)) && all.iter().any(|c| !pentagon(c)));
    for cell in &all {
        let res = resolution(cell);
        let pent = pentagon(cell);
        let sides = if pent { 5 } else { 6 };

        // cell-to-latlng: the center's cell at the cell's resolution is the cell.
        let c = call("indexing.h3.cell-to-latlng", json!({ "cell": cell }));
        let back = call(
            "indexing.h3.lat-lng-to-cell",
            json!({"lat": num(&c, "lat"), "lon": num(&c, "lon"), "resolution": res}),
        );
        assert_eq!(&text(&back, "cell"), cell, "center of {cell}");
        assert_eq!(num(&c, "resolution") as u8, res);

        // cell-boundary: at least 5 or 6 vertices, more where a face is crossed.
        let b = call("indexing.h3.cell-boundary", json!({ "cell": cell }));
        let count = num(&b, "vertex_count") as usize;
        assert!(count >= sides, "{cell}: {count} boundary vertices");
        assert_eq!(b["boundary"].as_array().unwrap().len(), count);

        // directed-edges and vertexes: 6 for a hexagon, 5 for a pentagon.
        let e = call("indexing.h3.directed-edges", json!({ "cell": cell }));
        assert_eq!(num(&e, "edge_count") as usize, sides, "{cell} edges");
        let v = call("indexing.h3.vertexes", json!({ "cell": cell }));
        assert_eq!(num(&v, "vertex_count") as usize, sides, "{cell} vertexes");
        let mine = list(&v, "vertexes", "vertex");

        // Each edge leads to a neighbor in ring 1; the reverse edge differs
        // and has the same length; neighbors share exactly two vertexes.
        let neighbors = ring1(cell);
        for (edge, to) in list(&e, "edges", "edge")
            .iter()
            .zip(list(&e, "edges", "to"))
        {
            assert!(neighbors.contains(&to), "{cell} -> {to}");
            let theirs = call("indexing.h3.directed-edges", json!({ "cell": to }));
            let reverse = list(&theirs, "edges", "edge")
                .into_iter()
                .zip(list(&theirs, "edges", "to"))
                .find(|(_, t)| t == cell)
                .map(|(e, _)| e)
                .expect("the neighbor has an edge back");
            assert_ne!(&reverse, edge);
            let (l1, l2) = (
                num(
                    &call("indexing.h3.edge-length", json!({ "edge": edge })),
                    "length",
                ),
                num(
                    &call("indexing.h3.edge-length", json!({ "edge": reverse })),
                    "length",
                ),
            );
            assert!((l1 - l2).abs() <= 1e-9 * l1, "{edge} {l1} vs {l2}");
            let shared = list(
                &call("indexing.h3.vertexes", json!({ "cell": to })),
                "vertexes",
                "vertex",
            )
            .into_iter()
            .filter(|x| mine.contains(x))
            .count();
            assert_eq!(shared, 2, "{cell} and {to} share {shared} vertexes");
        }

        // grid-distance: zero to itself, symmetric, k for ring k.
        let d = |a: &str, b: &str| {
            num(
                &call("indexing.h3.grid-distance", json!({"from": a, "to": b})),
                "distance",
            )
        };
        assert_eq!(d(cell, cell), 0.0);
        // Coarse cells far apart can fall off H3's local grid, which H3 C
        // also refuses; the rings are checked where the grid reaches.
        for k in if res >= 2 { &[1, 2][..] } else { &[][..] } {
            for other in list(
                &call("indexing.h3.grid-ring", json!({"cell": cell, "k": *k})),
                "cells",
                "cell",
            )
            .iter()
            .take(4)
            {
                assert_eq!(d(cell, other), f64::from(*k));
                assert_eq!(d(other, cell), f64::from(*k));
            }
        }

        // is-class-iii: odd resolutions are Class III, resolution 0 is not.
        let class3 = yes(
            &call("indexing.h3.is-class-iii", json!({ "cell": cell })),
            "class3",
        );
        assert_eq!(class3, res % 2 == 1, "{cell}");

        // validate: every cell another tool returns is valid; an edge is not.
        let valid = |x: &str| yes(&call("indexing.h3.validate", json!({ "cell": x })), "valid");
        assert!(valid(cell));
        assert!(!valid(&list(&e, "edges", "edge")[0]));

        // base-cell: the base cell of a pentagon is a pentagon, between 0 and 121.
        let base = call("indexing.h3.base-cell", json!({ "cell": cell }));
        let n = num(&base, "base_cell");
        assert!((0.0..=121.0).contains(&n));
        if pent {
            assert!(yes(&base, "base_pentagon"));
        }

        // cell-area: positive, below the sphere's, and near the level average
        // away from the pentagons.
        let a = call("indexing.h3.cell-area", json!({ "cell": cell }));
        let area = num(&a, "area");
        assert!(area > 0.0 && area < 5.11e14, "{cell}: {area}");
        if !pent {
            let ratio = num(&a, "ratio");
            assert!((0.5..2.0).contains(&ratio), "{cell}: ratio {ratio}");
        }

        if res < 15 {
            let fine = res + 1;
            // center-child: its parent is the cell; a pentagon's is a pentagon;
            // at the cell's own resolution it is the cell.
            let child = text(
                &call(
                    "indexing.h3.center-child",
                    json!({"cell": cell, "resolution": fine}),
                ),
                "center_child",
            );
            let up = call(
                "indexing.h3.parent",
                json!({"cell": child, "resolution": res}),
            );
            assert_eq!(&text(&up, "parent"), cell);
            assert_eq!(pentagon(&child), pent);
            let same = call(
                "indexing.h3.center-child",
                json!({"cell": cell, "resolution": res}),
            );
            assert_eq!(&text(&same, "center_child"), cell);
            // A cell and its descendants share a base cell.
            let child_base = call("indexing.h3.base-cell", json!({ "cell": child }));
            assert_eq!(num(&child_base, "base_cell"), n);
            // A child is of the other class.
            let child_class = yes(
                &call("indexing.h3.is-class-iii", json!({ "cell": child })),
                "class3",
            );
            assert_ne!(child_class, class3);
            // resolution: each finer average is about a seventh of the coarser.
            let (avg, avg_fine) = (
                num(
                    &call("indexing.h3.resolution", json!({ "cell": cell })),
                    "average_area",
                ),
                num(
                    &call("indexing.h3.resolution", json!({ "cell": child })),
                    "average_area",
                ),
            );
            assert!((6.5..7.5).contains(&(avg / avg_fine)), "{}", avg / avg_fine);
            // child-position: the center child is position 0, and each child's
            // position indexes it in the parent's list of children.
            let kids = list(
                &call(
                    "indexing.h3.children",
                    json!({"cell": cell, "resolution": fine}),
                ),
                "cells",
                "cell",
            );
            for (i, kid) in kids.iter().enumerate() {
                let p = call(
                    "indexing.h3.child-position",
                    json!({"cell": kid, "resolution": res}),
                );
                let pos = num(&p, "position") as usize;
                assert_eq!(pos, i, "{kid}");
                assert!(pos < kids.len());
                if kid == &child {
                    assert_eq!(pos, 0);
                }
            }
            // edge-length: edges shrink by about √7 per resolution.
            let (coarse, finer) = (
                num(
                    &call(
                        "indexing.h3.edge-length",
                        json!({ "edge": list(&e, "edges", "edge")[0] }),
                    ),
                    "length",
                ),
                num(
                    &call(
                        "indexing.h3.edge-length",
                        json!({ "edge": list(&call("indexing.h3.directed-edges", json!({ "cell": child })), "edges", "edge")[0] }),
                    ),
                    "length",
                ),
            );
            assert!((2.0..3.3).contains(&(coarse / finer)), "{}", coarse / finer);
        }
    }
    // Exactly 12 of the 122 base cells are pentagons. A resolution-0 index
    // is mode 1 with base cell b in bits 45 to 51 and every digit 7.
    let base_pentagons = (0..122u64)
        .filter(|b| pentagon(&format!("{:015x}", 0x8001fffffffffff_u64 + (b << 45))))
        .count();
    assert_eq!(base_pentagons, 12);
}
