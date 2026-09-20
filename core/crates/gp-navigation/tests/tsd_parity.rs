//! Time, speed, and distance against Bowditch's printed Table 11 (NGA Pub. 9,
//! Volume II, 2024 edition): every one of its 4,800 cells, through the public
//! tool. Fixture from tools/vectors/gen_tsd_diff.py.

use gp_navigation::REGISTRY;
use serde_json::{Value, json};

fn call(id: &str, input: &Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).expect("envelope is JSON")
}

/// (minutes, knots, printed miles, whether the table misprints this cell).
fn table() -> Vec<(f64, f64, f64, bool)> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/tsd_table11.csv");
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("read {path}: {e}"))
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| {
            let c: Vec<&str> = l.split(',').collect();
            (
                c[0].parse().unwrap(),
                c[1].parse().unwrap(),
                c[2].parse().unwrap(),
                c.len() > 3,
            )
        })
        .collect()
}

#[test]
fn distance_matches_bowditch_table_11() {
    // The table prints tenths of a mile, so a cell agrees when it rounds the same.
    let rows = table();
    assert_eq!(rows.len(), 4800);
    let (mut worst, mut checked, mut misprints) = (0f64, 0, 0);
    for (minutes, knots, miles, misprint) in rows {
        let r = call(
            "navigation.route.time-speed-distance",
            &json!({"speed": format!("{knots} kt"), "time": format!("{minutes} min"),
                    "options": {"outputUnits": {"distance": "NM"}}}),
        );
        let got = r["result"]["distance"]["value"]
            .as_f64()
            .unwrap_or_else(|| panic!("{r}"));
        let gap = (got - miles).abs();
        if misprint {
            // Pinned: Table 11 prints 6.8 nm for 38 minutes at 10.5 knots, but
            // 10.5 x 38 / 60 is 6.65, and every neighbor in that row follows
            // speed x time. The core does not repeat the misprint.
            assert_eq!((minutes, knots, miles), (38.0, 10.5, 6.8));
            assert!((got - 6.65).abs() < 5e-9, "{got}");
            misprints += 1;
            continue;
        }
        worst = worst.max(gap);
        checked += 1;
        assert!(
            gap <= 0.05 + 1e-9,
            "{minutes} min at {knots} kt: {got} vs the table's {miles}"
        );
    }
    eprintln!("Table 11: {checked} cells within {worst:.4} nm, {misprints} misprint");
    assert_eq!(checked, 4799);
    assert_eq!(misprints, 1);
}

#[test]
fn every_unknown_is_solved_the_same_way() {
    // Give any two of distance, speed, and time; the third comes back so that
    // distance = speed x time, whichever one was left out.
    for (minutes, knots, _, misprint) in table().into_iter().filter(|r| r.0 as u32 % 7 == 3) {
        if misprint {
            continue;
        }
        let exact = knots * minutes / 60.0;
        let by = |input: Value| call("navigation.route.time-speed-distance", &input);
        let d = by(
            json!({"speed": format!("{knots} kt"), "time": format!("{minutes} min"),
                          "options": {"outputUnits": {"distance": "NM"}}}),
        );
        assert!((d["result"]["distance"]["value"].as_f64().unwrap() - exact).abs() < 1e-9);
        let s = by(
            json!({"distance": format!("{exact} NM"), "time": format!("{minutes} min"),
                          "options": {"outputUnits": {"speed": "kt"}}}),
        );
        assert!(
            (s["result"]["speed"]["value"].as_f64().unwrap() - knots).abs() < 1e-9,
            "{s}"
        );
        let t = by(
            json!({"distance": format!("{exact} NM"), "speed": format!("{knots} kt"),
                          "options": {"outputUnits": {"time": "min"}}}),
        );
        assert!(
            (t["result"]["time"]["value"].as_f64().unwrap() - minutes).abs() < 1e-9,
            "{t}"
        );
    }
}
