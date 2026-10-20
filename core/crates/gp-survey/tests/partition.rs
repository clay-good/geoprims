//! Area partition (add-survey-partition-and-adjustment, "Area partition by a
//! cutoff line").

use gp_survey::REGISTRY;
use serde_json::{Value, json};

fn invoke(input: &Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke("survey.cogo.area-partition", &input.to_string()))
        .unwrap()
}

fn num(r: &Value, f: &str) -> f64 {
    r["result"][f]["value"].as_f64().unwrap()
}

/// Distance from q to the nearest side of the figure.
fn to_boundary(pts: &[(f64, f64)], q: (f64, f64)) -> f64 {
    (0..pts.len())
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let t = (((q.0 - a.0) * dx + (q.1 - a.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
            (q.0 - a.0 - t * dx).hypot(q.1 - a.1 - t * dy)
        })
        .fold(f64::INFINITY, f64::min)
}

#[test]
fn area_partition_invariants() {
    // A small deterministic generator, so the test needs no crate.
    let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    let mut solved = 0;
    for case in 0..1_000 {
        // A star-shaped figure: simple, often not convex.
        let n = 3 + (rnd() * 10.0) as usize;
        let rad = 50.0 + rnd() * 950.0;
        let (cx, cy) = (1_000.0 + rnd() * 9_000.0, 1_000.0 + rnd() * 9_000.0);
        let mut angles: Vec<f64> = (0..n).map(|_| rnd() * std::f64::consts::TAU).collect();
        angles.sort_by(f64::total_cmp);
        let pts: Vec<(f64, f64)> = angles
            .iter()
            .map(|a| {
                let rr = rad * (0.6 + 0.4 * rnd());
                (cx + rr * a.cos(), cy + rr * a.sin())
            })
            .collect();
        let side = 1 + (rnd() * n as f64) as usize;
        let frac = 0.05 + 0.9 * rnd();
        let total = {
            let mut s = 0.0;
            for i in 0..n {
                let (a, b) = (pts[i], pts[(i + 1) % n]);
                s += (a.0 - pts[0].0) * (b.1 - pts[0].1) - (b.0 - pts[0].0) * (a.1 - pts[0].1);
            }
            s.abs() / 2.0
        };
        let want = total * frac;
        let mut input = json!({
            "points": pts.iter().map(|(x, y)| json!({"northing": y, "easting": x})).collect::<Vec<_>>(),
            "area": format!("{want} ft2"),
            "side": side,
        });
        match case % 3 {
            0 => input["method"] = json!("parallel"),
            1 => {
                input["method"] = json!("through-point");
                let (a, b) = (pts[side - 1], pts[side % n]);
                let l = (b.0 - a.0).hypot(b.1 - a.1);
                input["distance"] = json!(format!("{} ft", rnd() * l * 0.999_999));
            }
            _ => {
                input["method"] = json!("bearing");
                input["bearing"] = json!(format!("{:.4}", rnd() * 359.0));
            }
        }
        let r = invoke(&input);
        if r["ok"] != true {
            // Only a line that would make more than two pieces, or leave the
            // figure, may be refused.
            assert_eq!(
                r["error"]["code"], "DEGENERATE_GEOMETRY",
                "case {case}: {r}"
            );
            continue;
        }
        solved += 1;
        let (cut, rest, whole) = (
            num(&r, "cut_area"),
            num(&r, "remainder_area"),
            num(&r, "total_area"),
        );
        assert!(
            (cut + rest - whole).abs() <= 1e-9 * whole.max(1.0),
            "case {case}: {cut} + {rest} ≠ {whole}"
        );
        assert!(
            (whole - total).abs() <= 1e-9 * total,
            "case {case}: total {whole} ≠ {total}"
        );
        let len = num(&r, "length");
        assert!(
            (cut - want).abs() <= 1e-6 * len.max(1.0),
            "case {case}: cut {cut} ≠ {want}"
        );
        for end in ["start", "end"] {
            let q = (
                num(&r, &format!("{end}_easting")),
                num(&r, &format!("{end}_northing")),
            );
            let d = to_boundary(&pts, q);
            assert!(
                d <= 1e-9 * rad,
                "case {case}: {end} is {d} off the boundary"
            );
        }
    }
    assert!(solved >= 700, "only {solved} of 1,000 figures solved");
}

#[test]
fn trapezoid_matches_the_quadratic() {
    // Base 200 ft, sides at 45° and 90°: A(d) = 200 d − d²/2.
    let r = invoke(&json!({
        "points": [{"northing": 0, "easting": 0}, {"northing": 0, "easting": 200},
                   {"northing": 100, "easting": 200}, {"northing": 100, "easting": 100}],
        "area": "7500 ft2", "method": "parallel", "side": 1}));
    let d = 200.0 - (200.0f64 * 200.0 - 2.0 * 7_500.0).sqrt();
    assert!((num(&r, "offset") - d).abs() < 1e-7, "{r}");
    assert!(
        r["result"]["check"].as_str().unwrap().contains("0.5 d²"),
        "{r}"
    );
    assert!((num(&r, "length") - (200.0 - d)).abs() < 1e-7, "{r}");
}
