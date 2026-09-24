//! Polygon overlay against GEOS on shapes built to be degenerate.
//!
//! 400 pairs on a small integer grid, where shared edges, corners on edges,
//! and pieces touching at a point are common; GEOS's answers are in the
//! fixture (tools/vectors/gen_overlay_grid.py). Three checks:
//!
//! 1. the planar overlay on the grid itself agrees exactly, in area and in the
//!    number of parts;
//! 2. with every corner moved by up to 0.2 µm, as a corner on an edge is once
//!    it is projected, the answers do not change;
//! 3. the geodesic tool, with the grid placed at 40° N at a step of about a
//!    meter, returns GEOS's parts, with areas within 2 cm².
//!
//! Before 1.1.0 the second and third failed on 261 of the 1,600 overlays: a
//! corner a fraction of a micron off an edge was nudged across it by the side
//! test, which lost or kept the wrong pieces (an empty union, a difference a
//! third too small), and pieces touching at a point came back as one ring
//! touching itself.

use gp_geo::buffer::{Op, boolean};
use gp_geometry::REGISTRY;
use serde_json::{Value, json};

const OPS: [(&str, Op); 4] = [
    ("intersection", Op::Intersection),
    ("union", Op::Union),
    ("difference", Op::Difference),
    ("symmetric-difference", Op::SymmetricDifference),
];

fn fixture() -> Vec<Value> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/overlay_geos.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("JSON")
}

/// The shape's rings as (x, y) = (column, row), each corner moved by `jitter`.
fn rings(shape: &Value, jitter: &mut dyn FnMut() -> f64) -> Vec<Vec<(f64, f64)>> {
    let ring = |r: &Value, j: &mut dyn FnMut() -> f64| -> Vec<(f64, f64)> {
        r.as_array()
            .unwrap()
            .iter()
            .map(|p| (p[1].as_f64().unwrap() + j(), p[0].as_f64().unwrap() + j()))
            .collect()
    };
    let mut out = vec![ring(&shape[0], jitter)];
    for h in shape[1].as_array().unwrap() {
        out.push(ring(h, jitter));
    }
    out
}

fn signed(ring: &[(f64, f64)]) -> f64 {
    (0..ring.len())
        .map(|j| {
            let (p, q) = (ring[j], ring[(j + 1) % ring.len()]);
            p.0 * q.1 - q.0 * p.1
        })
        .sum::<f64>()
        / 2.0
}

fn summary(r: &[Vec<(f64, f64)>]) -> (f64, usize) {
    (
        r.iter().map(|x| signed(x)).sum(),
        r.iter().filter(|x| signed(x) > 0.0).count(),
    )
}

fn lat_lon(shape: &Value) -> Value {
    let mut out = Vec::new();
    for (k, ring) in std::iter::once(&shape[0])
        .chain(shape[1].as_array().unwrap())
        .enumerate()
    {
        for p in ring.as_array().unwrap() {
            let mut v = json!({"lat": 40.0 + p[0].as_f64().unwrap() * 1e-5,
                               "lon": -105.0 + p[1].as_f64().unwrap() * 1e-5});
            if k > 0 {
                v["ring"] = json!(k);
            }
            out.push(v);
        }
    }
    Value::Array(out)
}

#[test]
fn overlay_matches_geos_on_degenerate_shapes() {
    let cases = fixture();
    assert_eq!(cases.len(), 400);
    let mut seed: u64 = 7;
    let mut jitter = move || {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((seed >> 11) as f64 / (1u64 << 53) as f64 - 0.5) * 4e-7
    };
    let mut wrong = Vec::new();
    for (i, c) in cases.iter().enumerate() {
        let (a, b) = (rings(&c["a"], &mut || 0.0), rings(&c["b"], &mut || 0.0));
        let (na, nb) = (rings(&c["a"], &mut jitter), rings(&c["b"], &mut jitter));
        let (ga, gb) = (lat_lon(&c["a"]), lat_lon(&c["b"]));
        for (name, op) in OPS {
            let want = &c["results"][name];
            let (area, parts) = (
                want["grid_area"].as_f64().unwrap(),
                want["parts"].as_u64().unwrap() as usize,
            );
            let exact = summary(&boolean(&a, &b, op));
            if (exact.0 - area).abs() > 1e-9 || exact.1 != parts {
                wrong.push(format!(
                    "pair {i} {name} on the grid: {exact:?}, GEOS ({area}, {parts})"
                ));
            }
            let moved = summary(&boolean(&na, &nb, op));
            if (moved.0 - area).abs() > 1e-4 || moved.1 != parts {
                wrong.push(format!(
                    "pair {i} {name} moved 0.2 µm: {moved:?}, GEOS ({area}, {parts})"
                ));
            }
            let r: Value = serde_json::from_str(&REGISTRY.invoke(
                "geometry.overlay.boolean",
                &json!({"polygon_a": ga, "polygon_b": gb, "operation": name}).to_string(),
            ))
            .expect("JSON");
            let got_parts = r["result"]["parts"].as_u64().unwrap_or(u64::MAX) as usize;
            let got_m2 = r["result"]["area"]["value"].as_f64().unwrap_or(-1.0) * 1e6;
            let m2 = want["m2"].as_f64().unwrap();
            if got_parts != parts || (got_m2 - m2).abs() > 2e-4 {
                wrong.push(format!("pair {i} {name} on the ground: {got_parts} parts, {got_m2} m²; GEOS {parts}, {m2} m²"));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of 4,800 checks differ:\n{}",
        wrong.len(),
        wrong[..wrong.len().min(10)].join("\n")
    );
}
