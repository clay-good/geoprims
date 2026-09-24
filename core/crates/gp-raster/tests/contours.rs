//! Contours: the properties every answer must have.

use gp_raster::REGISTRY;
use serde_json::{Value, json};

fn run(grid: &[Vec<f64>], interval: f64, cell: f64, base: f64) -> Value {
    let rows: Vec<Value> = grid
        .iter()
        .map(|r| json!({"row": r.iter().map(|v| format!("{v}")).collect::<Vec<_>>().join(", ")}))
        .collect();
    let r: Value = serde_json::from_str(
        &REGISTRY.invoke(
            "raster.terrain.contours",
            &json!({"elevations": rows, "interval": format!("{interval} m"),
                "cell_size": format!("{cell} m"), "base": format!("{base} m")})
            .to_string(),
        ),
    )
    .expect("JSON");
    assert_eq!(r["ok"], true, "{r}");
    r["result"].clone()
}

fn val(v: &Value) -> f64 {
    v["value"].as_f64().unwrap()
}

/// A bumpy surface with no grid value on a whole-meter level.
fn surface(rows: usize, cols: usize, seed: f64) -> Vec<Vec<f64>> {
    (0..rows)
        .map(|i| {
            (0..cols)
                .map(|j| {
                    let (x, y) = (j as f64, i as f64);
                    let z = 100.0
                        + 12.0 * (0.45 * x + seed).sin() * (0.35 * y - seed).cos()
                        + 0.8 * x
                        + 7.0 * (-((x - 6.0).powi(2) + (y - 5.0).powi(2)) / 9.0).exp();
                    (z * 1000.0).round() / 1000.0 + 0.0005
                })
                .collect()
        })
        .collect()
}

#[test]
fn contour_invariants() {
    for seed in [0.0, 0.7, 1.9, 3.1] {
        let z = surface(14, 17, seed);
        let cell = 10.0;
        let r = run(&z, 2.0, cell, 0.0);
        let rows = r["contours"].as_array().unwrap();
        assert!(!rows.is_empty());

        // Every vertex sits on a grid line, where the elevation interpolated
        // linearly along that line is the contour's level; and the ground to
        // the left of every step is higher.
        let at = |i: usize, j: usize| z[i][j];
        for (k, v) in rows.iter().enumerate() {
            let (e, s, level) = (
                val(&v["east"]) / cell,
                val(&v["south"]) / cell,
                val(&v["level"]),
            );
            let (fe, fs) = (e - e.floor(), s - s.floor());
            let z_here = if fe.abs() < 1e-12 {
                let (j, i) = (e.round() as usize, s.floor() as usize);
                at(i, j) + fs * (at((i + 1).min(z.len() - 1), j) - at(i, j))
            } else {
                assert!(
                    fs.abs() < 1e-12,
                    "seed {seed} vertex {k} is off the grid lines: {v}"
                );
                let (j, i) = (e.floor() as usize, s.round() as usize);
                at(i, j) + fe * (at(i, (j + 1).min(z[0].len() - 1)) - at(i, j))
            };
            assert!(
                (z_here - level).abs() < 1e-9,
                "seed {seed} vertex {k}: {z_here} on a {level} contour"
            );
        }
        for w in rows.windows(2) {
            if w[0]["line"] != w[1]["line"] {
                continue;
            }
            // At both ends of a step, the higher grid point of the grid line
            // it crosses there lies on its left (on the ground; in east-south
            // coordinates the sign of the cross product flips).
            let p = (val(&w[0]["east"]) / cell, val(&w[0]["south"]) / cell);
            let q = (val(&w[1]["east"]) / cell, val(&w[1]["south"]) / cell);
            for v in [p, q] {
                let (a, b) = if (v.0 - v.0.round()).abs() < 1e-12 {
                    let j = v.0.round() as usize;
                    let i = v.1.floor() as usize;
                    ((i, j), (i + 1, j))
                } else {
                    let i = v.1.round() as usize;
                    let j = v.0.floor() as usize;
                    ((i, j), (i, j + 1))
                };
                let hi = if at(a.0, a.1) > at(b.0, b.1) { a } else { b };
                let (he, hs) = (hi.1 as f64, hi.0 as f64);
                let cross = (q.0 - p.0) * (hs - p.1) - (q.1 - p.1) * (he - p.0);
                assert!(
                    -cross > -1e-9,
                    "seed {seed}: the higher grid point {hi:?} is right of the step {} -> {}",
                    w[0],
                    w[1]
                );
            }
        }

        // Raising everything and the base together moves nothing.
        let up: Vec<Vec<f64>> = z
            .iter()
            .map(|r| r.iter().map(|v| v + 37.0).collect())
            .collect();
        let r2 = run(&up, 2.0, cell, 37.0);
        assert_eq!(r2["line_count"], r["line_count"], "seed {seed}");
        assert!(
            (val(&r2["length"]) - val(&r["length"])).abs() < 1e-6,
            "seed {seed}"
        );

        // Twice the cell size is twice the length, with the same lines.
        let r3 = run(&z, 2.0, 2.0 * cell, 0.0);
        assert_eq!(r3["line_count"], r["line_count"], "seed {seed}");
        assert!(
            (val(&r3["length"]) - 2.0 * val(&r["length"])).abs() < 1e-6,
            "seed {seed}"
        );

        // Turning the ground upside down keeps every line and its length:
        // the level set is the same, only uphill has changed sides.
        let flipped: Vec<Vec<f64>> = z.iter().map(|r| r.iter().map(|v| -v).collect()).collect();
        let r4 = run(&flipped, 2.0, cell, 0.0);
        assert!(
            (val(&r4["length"]) - val(&r["length"])).abs() < 1e-6,
            "seed {seed}"
        );
        assert_eq!(r4["line_count"], r["line_count"], "seed {seed}");
    }

    // A single hill: one closed loop per level, nested and shrinking.
    let hill: Vec<Vec<f64>> = (0..21)
        .map(|i| {
            (0..21)
                .map(|j| {
                    let d2 = ((i as f64 - 10.0).powi(2) + (j as f64 - 10.0).powi(2)) / 16.0;
                    100.0 + 50.0 * (-d2).exp() + 0.001
                })
                .collect()
        })
        .collect();
    let r = run(&hill, 10.0, 1.0, 0.0);
    let levels = r["levels"].as_array().unwrap();
    assert_eq!(levels.len(), 5, "110 to 150 m: {r}");
    let mut last = f64::INFINITY;
    for l in levels {
        assert_eq!(l["lines"], 1, "{l}");
        assert_eq!(l["closed"], 1, "{l}");
        let len = val(&l["length"]);
        assert!(len < last, "loops do not shrink uphill: {l}");
        last = len;
    }
}
