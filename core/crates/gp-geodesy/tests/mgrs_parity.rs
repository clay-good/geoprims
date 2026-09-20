//! MGRS against NGA GEOTRANS (via the mgrs package, tools/vectors/gen_mgrs_diff.py),
//! through the public tools. References must be identical. GEOTRANS corners
//! carry its own projection-series error, up to 1.5 cm near the UPS edge and
//! far out in Svalbard's widened zones; PROJ puts our corners exactly on the
//! grid there (pinned in the golden vectors), so corners are compared within 2 cm.

use gp_geodesy::REGISTRY;
use serde_json::{Value, json};

fn call(id: &str, input: &Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).expect("envelope is JSON")
}

const PRECISION: [&str; 6] = ["100km", "10km", "1km", "100m", "10m", "1m"];

#[test]
fn mgrs_matches_geotrans() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/mgrs_diff.csv"
    ))
    .unwrap();
    let (mut n, mut worst) = (0, 0.0f64);
    let mut bad = Vec::new();
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let c: Vec<&str> = line.split(',').collect();
        let f = |i: usize| c[i].parse::<f64>().unwrap();
        let d: usize = c[2].parse().unwrap();
        let r = call(
            "geodesy.grid-ref.mgrs-forward",
            &json!({"lat": f(0), "lon": f(1), "precision": PRECISION[d]}),
        );
        if r["result"]["mgrs"] != c[3] {
            bad.push(format!("{line} -> {}", r["result"]["mgrs"]));
            continue;
        }
        let b = call("geodesy.grid-ref.mgrs-inverse", &json!({"mgrs": c[3]}));
        let (la, lo) = (
            b["result"]["corner_lat"]["value"].as_f64().unwrap(),
            b["result"]["corner_lon"]["value"].as_f64().unwrap(),
        );
        let dlon = (lo - f(5) + 540.0).rem_euclid(360.0) - 180.0;
        let gap_m = (la - f(4)).hypot(dlon * f(4).to_radians().cos()) * 111_320.0;
        worst = worst.max(gap_m);
        n += 1;
    }
    assert!(
        bad.is_empty(),
        "{} mismatches:\n{}",
        bad.len(),
        bad[..bad.len().min(10)].join("\n")
    );
    assert_eq!(n, 2000);
    assert!(worst < 0.02, "worst corner gap {worst} m");
}

#[test]
fn mgrs_invariants() {
    // Truncation nests (a coarser reference keeps the leading digits of each
    // axis), and every square's corner lies within 1.5 square sizes of the
    // point (the slack covers grid convergence and scale). A square's center
    // can fall outside its zone when the square is cut by the zone edge, as
    // 60NZF is at the antimeridian, so centers are not re-encoded.
    let pts = [
        (40.446111, -79.982222),
        (60.5, 4.5),
        (78.2, 15.0),
        (-33.8688, 151.2093),
        (84.3, 50.0),
        (-89.9, -135.0),
        (0.0001, 179.9999),
    ];
    for (lat, lon) in pts {
        let full = call(
            "geodesy.grid-ref.mgrs-forward",
            &json!({"lat": lat, "lon": lon, "precision": "1m"}),
        );
        let g = full["result"]["mgrs"].as_str().unwrap().to_owned();
        let head = &g[..g.len() - 10];
        let (e, n) = (&g[g.len() - 10..g.len() - 5], &g[g.len() - 5..]);
        for (k, p) in PRECISION.iter().enumerate() {
            let r = call(
                "geodesy.grid-ref.mgrs-forward",
                &json!({"lat": lat, "lon": lon, "precision": p}),
            );
            let want = format!("{head}{}{}", &e[..k], &n[..k]);
            assert_eq!(r["result"]["mgrs"], want.as_str(), "{lat},{lon} at {p}");
            let b = call("geodesy.grid-ref.mgrs-inverse", &json!({"mgrs": want}));
            let size = b["result"]["square_size"]["value"].as_f64().unwrap();
            let (la, lo) = (
                b["result"]["corner_lat"]["value"].as_f64().unwrap(),
                b["result"]["corner_lon"]["value"].as_f64().unwrap(),
            );
            let dlon = (lo - lon + 540.0).rem_euclid(360.0) - 180.0;
            let gap = (la - lat).hypot(dlon * lat.to_radians().cos()) * 111_320.0;
            assert!(
                gap <= 1.5 * size,
                "{want}: corner {gap} m from the point, square {size} m"
            );
        }
    }
}
