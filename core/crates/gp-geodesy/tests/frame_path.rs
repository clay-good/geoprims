//! The frame graph behind geodesy.datum.transform (add-geodesy-suite task
//! 3.3): for every pair of frames, the path's stated accuracy is the root sum
//! of squares of its steps, the path back is as good as the path there, the
//! round trip returns the point, and a path of one step gives what the
//! dedicated tool gives.

use gp_geodesy::REGISTRY;
use serde_json::{Value, json};

const FRAMES: &[&str] = &[
    "ITRF2020",
    "ITRF2014",
    "ITRF2008",
    "ITRF2005",
    "ITRF2000",
    "ITRF97",
    "ITRF96",
    "ITRF94",
    "ITRF93",
    "ITRF92",
    "ITRF91",
    "ITRF90",
    "ITRF89",
    "ITRF88",
    "WGS84(G2296)",
    "WGS84(G2139)",
    "WGS84(G1762)",
    "WGS84(G1674)",
    "WGS84(G1150)",
    "NAD83(2011)",
    "NAD83(PA11)",
    "NAD83(MA11)",
];

fn run(tool: &str, input: Value) -> Value {
    let r: Value = serde_json::from_str(&REGISTRY.invoke(tool, &input.to_string())).unwrap();
    assert_eq!(r["ok"], true, "{tool} {input}: {r}");
    r["result"].clone()
}

fn v(r: &Value, k: &str) -> f64 {
    r[k]["value"].as_f64().unwrap()
}

#[test]
fn frame_path_invariants() {
    let p = json!({"lat": 21.3, "lon": -157.8, "height": "30 m", "epoch": "2024.25"});
    for &a in FRAMES {
        for &b in FRAMES {
            let there = run(
                "geodesy.datum.transform",
                json!({"lat": p["lat"], "lon": p["lon"], "height": p["height"], "epoch": p["epoch"], "from": a, "to": b}),
            );
            let steps = there["steps"].as_array().unwrap();
            let rss: f64 = steps
                .iter()
                .map(|s| v(s, "accuracy").powi(2))
                .sum::<f64>()
                .sqrt();
            assert!((v(&there, "accuracy") - rss).abs() < 1e-15, "{a} -> {b}");
            // The steps chain from the source to the target.
            if a != b {
                assert_eq!(steps[0]["from"], a);
                assert_eq!(steps[steps.len() - 1]["to"], b);
                for w in steps.windows(2) {
                    assert_eq!(w[0]["to"], w[1]["from"], "{a} -> {b}");
                }
            } else {
                assert!(steps.is_empty());
            }
            let back = run(
                "geodesy.datum.transform",
                json!({"lat": v(&there, "lat"), "lon": v(&there, "lon"), "height": format!("{} m", v(&there, "height")), "epoch": p["epoch"], "from": b, "to": a}),
            );
            assert!(
                (v(&back, "accuracy") - v(&there, "accuracy")).abs() < 1e-15,
                "{a} <-> {b}"
            );
            // HTDP's reverse negates its parameters rather than inverting them,
            // which leaves up to 1.5 µm over a round trip; IERS inverts exactly.
            let dlat = (v(&back, "lat") - 21.3) * 111_000.0;
            let dlon = (v(&back, "lon") + 157.8) * 111_000.0 * 21.3f64.to_radians().cos();
            let dh = v(&back, "height") - 30.0;
            assert!(
                dlat.hypot(dlon).hypot(dh) < 1e-5,
                "{a} -> {b} -> {a}: {dlat} {dlon} {dh}"
            );
            // One step gives what the dedicated tool gives.
            if steps.len() == 1 {
                let method = steps[0]["method"].as_str().unwrap();
                let tool = if method.starts_with("IERS") {
                    "geodesy.datum.itrf"
                } else if method.starts_with("NGS HTDP") {
                    "geodesy.datum.nad83"
                } else {
                    // Aligned: the coordinates do not change.
                    assert!((v(&there, "lat") - 21.3).abs() < 1e-12 && v(&there, "shift") == 0.0);
                    continue;
                };
                let d = run(
                    tool,
                    json!({"lat": 21.3, "lon": -157.8, "height": "30 m", "epoch": "2024.25", "from": a, "to": b}),
                );
                for k in ["lat", "lon", "height"] {
                    assert!(
                        (v(&d, k) - v(&there, k)).abs() < 1e-12,
                        "{a} -> {b} {k} against {tool}"
                    );
                }
            }
        }
    }
}
