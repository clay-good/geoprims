//! Polygon repair against GEOS on rings built to be degenerate.
//!
//! 600 invalid rings on a small grid (crossing, touching, and running back
//! over themselves), each with its even-odd repair built from GEOS's noding
//! and polygonizing (tools/vectors/gen_make_valid_grid.py). Three checks:
//!
//! 1. the planar repair on the grid itself agrees exactly, in area and in the
//!    number of parts;
//! 2. with every corner moved by up to 0.2 µm, the answers do not change;
//! 3. the geodesic tool, with the grid placed at 40° N at a step of about a
//!    meter, returns the same parts and area.
//!
//! Before 1.1.0, 238 of the rings came back with too few parts on the grid
//! (pieces touching at a point returned as one ring touching itself), and on
//! the ground 104 were wrong, some repaired to nothing: a corner a hair off
//! its own ring's edge after projection was nudged across it.

use gp_geo::buffer::even_odd;
use gp_geometry::REGISTRY;
use serde_json::{Value, json};

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

#[test]
fn make_valid_matches_geos_on_degenerate_rings() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/make_valid_geos.json"
    );
    let fx: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("JSON");
    let cell = fx["cell_m2"].as_f64().unwrap();
    let cases = fx["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 600);
    let mut seed: u64 = 11;
    let mut jitter = move || {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((seed >> 11) as f64 / (1u64 << 53) as f64 - 0.5) * 4e-7
    };
    let mut wrong = Vec::new();
    for (i, c) in cases.iter().enumerate() {
        let (area, parts) = (
            c["area"].as_f64().unwrap(),
            c["parts"].as_u64().unwrap() as usize,
        );
        let corners: Vec<(f64, f64)> = c["ring"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| (p[1].as_f64().unwrap(), p[0].as_f64().unwrap()))
            .collect();
        let exact = summary(&even_odd(std::slice::from_ref(&corners)));
        if (exact.0 - area).abs() > 1e-9 || exact.1 != parts {
            wrong.push(format!(
                "ring {i} on the grid: {exact:?}, reference ({area}, {parts})"
            ));
        }
        let moved: Vec<(f64, f64)> = corners
            .iter()
            .map(|p| (p.0 + jitter(), p.1 + jitter()))
            .collect();
        let m = summary(&even_odd(&[moved]));
        if (m.0 - area).abs() > 1e-4 || m.1 != parts {
            wrong.push(format!(
                "ring {i} moved 0.2 µm: {m:?}, reference ({area}, {parts})"
            ));
        }
        let poly: Vec<Value> = corners
            .iter()
            .map(|p| json!({"lat": 40.0 + p.1 * 1e-5, "lon": -105.0 + p.0 * 1e-5}))
            .collect();
        let r: Value = serde_json::from_str(&REGISTRY.invoke(
            "geometry.validity.make-valid",
            &json!({"polygon": poly}).to_string(),
        ))
        .expect("JSON");
        let got_parts = r["result"]["parts"].as_u64().unwrap_or(u64::MAX) as usize;
        let got = r["result"]["area"]["value"].as_f64().unwrap_or(-1.0) * 1e6 / cell;
        if got_parts != parts || (got - area).abs() > 1e-3 * area.max(1.0) {
            wrong.push(format!("ring {i} on the ground: {got_parts} parts, {got} squares; reference {parts}, {area}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of 1,800 checks differ:\n{}",
        wrong.len(),
        wrong[..wrong.len().min(10)].join("\n")
    );
}
