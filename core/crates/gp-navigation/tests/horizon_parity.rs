//! Distance of the horizon against Bowditch's printed Table 12 (NGA Pub. 9,
//! Volume II, 2024 edition): every printed height, through the public tool.
//! Fixture from tools/vectors/gen_horizon_diff.py.

use gp_navigation::REGISTRY;
use serde_json::{Value, json};

fn call(id: &str, input: &Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).expect("envelope is JSON")
}

/// (feet, nautical miles, statute miles, meters, whether the table misprints it).
fn table() -> Vec<(f64, f64, f64, f64, bool)> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/horizon_table12.csv"
    );
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
                c[3].parse().unwrap(),
                c.len() > 4,
            )
        })
        .collect()
}

fn horizon(height: &str, unit: &str) -> Value {
    call(
        "navigation.los.horizon",
        &json!({"height": height, "options": {"outputUnits": {"rule_visual": unit, "optical": unit, "geometric": unit, "radio": unit}}}),
    )
}

#[test]
fn the_rule_matches_bowditch_table_12() {
    // Table 12 is the navigator's rule, 1.17 sqrt(h), which the tool reports as
    // rule_visual beside its own refracted model. Both columns are printed to a
    // tenth, so a row agrees when it rounds the same.
    let rows = table();
    assert_eq!(rows.len(), 126);
    let (mut worst, mut checked, mut misprints) = (0f64, 0, 0);
    for (feet, nm, statute, meters, misprint) in rows {
        let r = horizon(&format!("{feet} ft"), "NM");
        let rule = r["result"]["rule_visual"]["value"]
            .as_f64()
            .unwrap_or_else(|| panic!("{r}"));
        // The height in meters the table prints beside it is the same height.
        assert!(
            (meters - feet * 0.3048).abs() <= 0.005,
            "{feet} ft is {meters} m"
        );
        if misprint {
            // Pinned: Table 12 prints 29.5 nm at 640 ft, where 1.17 sqrt(640) is
            // 29.599 and the row's own statute cell (34.1) follows the rule.
            assert_eq!((feet, nm), (640.0, 29.5));
            assert!((rule - 29.5989).abs() < 1e-3, "{rule}");
            misprints += 1;
            continue;
        }
        worst = worst.max((rule - nm).abs());
        assert!(
            (rule - nm).abs() <= 0.05 + 1e-9,
            "{feet} ft: rule {rule} vs the table's {nm} nm"
        );
        // The statute column is the same rule in statute miles, from the unrounded value.
        let sm = horizon(&format!("{feet} ft"), "mi")["result"]["rule_visual"]["value"]
            .as_f64()
            .unwrap();
        assert!(
            (sm - statute).abs() <= 0.05 + 1e-9,
            "{feet} ft: {sm} vs the table's {statute} statute miles"
        );
        checked += 1;
    }
    eprintln!("Table 12: {checked} heights within {worst:.4} nm, {misprints} misprint");
    assert_eq!(checked, 125);
    assert_eq!(misprints, 1);
}

#[test]
fn horizon_invariants() {
    // Refraction only ever pushes the horizon further than the geometric one,
    // and the radio horizon further still; the slant range is a shade longer
    // than the distance along the surface; and the horizon always grows with
    // height. The rule is exactly 1.17 sqrt(h), so doubling the height
    // multiplies it by sqrt(2); the modelled horizon is an arc, not a square
    // root, so it only approaches that scaling for heights small against the
    // Earth's radius (within 0.05% at 100 m, but 0.7% low by 1,500 m).
    let mut seed: u64 = 67;
    let mut rnd = || {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    let value = |r: &Value, k: &str| {
        r["result"][k]["value"]
            .as_f64()
            .unwrap_or_else(|| panic!("{k} in {r}"))
    };
    for _ in 0..200 {
        let h = 0.5 + rnd() * 8000.0;
        let r = horizon(&format!("{h} m"), "km");
        let (geometric, optical, radio) = (
            value(&r, "geometric"),
            value(&r, "optical"),
            value(&r, "radio"),
        );
        assert!(geometric < optical && optical < radio, "{r}");
        assert!(value(&r, "optical_slant") >= optical - 1e-9, "{r}");
        let twice = horizon(&format!("{} m", h * 2.0), "km");
        assert!(value(&twice, "geometric") > geometric, "{h} m");
        assert!(
            (value(&twice, "rule_visual") / value(&r, "rule_visual") - 2f64.sqrt()).abs() < 1e-9
        );
        // The arc falls short of the square-root scaling, never past it.
        let ratio = value(&twice, "geometric") / geometric;
        assert!(
            ratio <= 2f64.sqrt() + 1e-12 && ratio > 1.40,
            "{h} m scaled by {ratio}"
        );
        // The stated rule error is the difference the tool reports between them.
        let gap = value(&r, "rule_visual") - optical;
        assert!((value(&r, "rule_visual_error") - gap).abs() < 1e-6, "{r}");
    }
    // Small against the Earth's radius, the arc and the square root agree.
    let small = horizon("10 m", "km");
    let doubled = horizon("20 m", "km");
    let ratio = value(&doubled, "geometric") / value(&small, "geometric");
    assert!((ratio - 2f64.sqrt()).abs() < 5e-4, "{ratio}");
}
