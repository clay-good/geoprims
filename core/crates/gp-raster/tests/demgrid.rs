//! Whole-grid DEM measures: what each must do under moves that change the
//! grid in known ways, as their derivation notes state.

use gp_raster::REGISTRY;
use serde_json::{Value, json};

fn rows(z: &[Vec<f64>]) -> Value {
    z.iter()
        .map(|r| {
            let s: Vec<String> = r.iter().map(f64::to_string).collect();
            json!({"row": s.join(", ")})
        })
        .collect()
}

fn run(tool: &str, input: Value) -> Value {
    let r: Value = serde_json::from_str(&REGISTRY.invoke(tool, &input.to_string())).unwrap();
    assert_eq!(r["ok"], true, "{tool}: {r}");
    r["result"].clone()
}

fn num(r: &Value, k: &str) -> f64 {
    r[k]["value"].as_f64().or(r[k].as_f64()).unwrap()
}

fn grids() -> Vec<Vec<Vec<f64>>> {
    vec![
        vec![
            vec![212.4, 213.1, 214.0, 214.8, 215.2],
            vec![211.9, 212.6, 213.7, 214.5, 215.0],
            vec![211.2, 212.0, 213.1, 214.1, 214.6],
        ],
        vec![
            vec![48.2, 48.9, 49.1, 48.7],
            vec![48.8, 50.3, 50.6, 49.0],
            vec![48.5, 50.1, 50.6, 48.9],
        ],
        vec![vec![-3.5, 0.0], vec![2.25, 7.0]],
    ]
}

#[test]
fn dem_stats_invariants() {
    for z in grids() {
        let s = run("raster.terrain.dem-stats", json!({"elevations": rows(&z)}));
        // Adding a constant moves the levels by it and leaves the spread.
        let up: Vec<Vec<f64>> = z
            .iter()
            .map(|r| r.iter().map(|v| v + 100.0).collect())
            .collect();
        let u = run("raster.terrain.dem-stats", json!({"elevations": rows(&up)}));
        for k in ["mean", "median", "lowest", "highest"] {
            assert!((num(&u, k) - num(&s, k) - 100.0).abs() < 1e-9, "{k}");
        }
        for k in ["relief", "std_dev"] {
            assert!((num(&u, k) - num(&s, k)).abs() < 1e-9, "{k}");
        }
        // The mean and median lie between the lowest and highest.
        for k in ["mean", "median"] {
            assert!(num(&s, k) >= num(&s, "lowest") && num(&s, k) <= num(&s, "highest"));
        }
        // A row of no-data cells never changes the answer.
        let mut gap = z.clone();
        gap.push(vec![-9999.0; z[0].len()]);
        let g = run(
            "raster.terrain.dem-stats",
            json!({"elevations": rows(&gap), "no_data": -9999}),
        );
        for k in ["mean", "median", "lowest", "highest", "std_dev"] {
            assert_eq!(num(&g, k), num(&s, k), "{k}");
        }
        assert_eq!(num(&g, "no_data_cells"), z[0].len() as f64);
    }
}

#[test]
fn highest_point_invariants() {
    for z in grids() {
        let h = run(
            "raster.terrain.highest-point",
            json!({"elevations": rows(&z)}),
        );
        let top = num(&h, "highest");
        assert!(z.iter().flatten().all(|v| *v <= top));
        // Raising one cell above all others moves the answer there, untied.
        let mut raised = z.clone();
        raised[z.len() - 1][0] = top + 1.0;
        let r = run(
            "raster.terrain.highest-point",
            json!({"elevations": rows(&raised)}),
        );
        assert_eq!(num(&r, "row"), z.len() as f64);
        assert_eq!(num(&r, "column"), 1.0);
        assert_eq!(num(&r, "ties"), 1.0);
        // A no-data cell is never reported, even where the data was highest.
        let (row, col) = (num(&h, "row") as usize - 1, num(&h, "column") as usize - 1);
        let mut gap = z.clone();
        gap[row][col] = -9999.0;
        let g = run(
            "raster.terrain.highest-point",
            json!({"elevations": rows(&gap), "no_data": -9999}),
        );
        assert!(num(&g, "highest") <= top);
        assert!((num(&g, "row"), num(&g, "column")) != ((row + 1) as f64, (col + 1) as f64));
    }
}

#[test]
fn dem_difference_invariants() {
    let z = grids();
    let (before, after) = (&z[0], {
        let mut a = z[0].clone();
        a[0][1] += 0.8;
        a[1][2] -= 0.3;
        a[2][4] += 0.02;
        a[1][0] -= 1.5;
        a
    });
    let input = |b: &Vec<Vec<f64>>, a: &Vec<Vec<f64>>, floor: f64| json!({"before": rows(b), "after": rows(a), "cell_size": "2 m", "min_change": format!("{floor} m")});
    let d = run(
        "raster.terrain.dem-difference-volume",
        input(before, &after, 0.0),
    );
    // Net is gained less lost.
    assert!(
        (num(&d, "net_volume") - (num(&d, "gained_volume") - num(&d, "lost_volume"))).abs() < 1e-9
    );
    // Swapping the surveys swaps gained and lost and negates the net.
    let s = run(
        "raster.terrain.dem-difference-volume",
        input(&after, before, 0.0),
    );
    assert!((num(&s, "gained_volume") - num(&d, "lost_volume")).abs() < 1e-9);
    assert!((num(&s, "lost_volume") - num(&d, "gained_volume")).abs() < 1e-9);
    assert!((num(&s, "net_volume") + num(&d, "net_volume")).abs() < 1e-9);
    // A larger minimum change never counts more area as changed.
    let mut last = f64::INFINITY;
    for floor in [0.0, 0.01, 0.05, 0.5, 1.0, 2.0] {
        let r = run(
            "raster.terrain.dem-difference-volume",
            input(before, &after, floor),
        );
        assert!(num(&r, "changed_area") <= last, "{floor}");
        last = num(&r, "changed_area");
    }
    assert_eq!(last, 0.0);
}
