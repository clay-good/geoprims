//! Under-keel clearance (add-marine-navigation, navigation/under-keel-clearance):
//! the spec's scenarios, and the invariants of the stack and of Barrass's squat.

use gp_navigation::REGISTRY;
use serde_json::{Value, json};

fn run(input: Value) -> Value {
    serde_json::from_str(
        &REGISTRY.invoke("navigation.vessel.under-keel-clearance", &input.to_string()),
    )
    .unwrap()
}

fn value(r: &Value, field: &str) -> f64 {
    r["result"][field]["value"].as_f64().unwrap()
}

fn warnings(r: &Value) -> Vec<String> {
    r["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn clearance_stack_scenarios() {
    // Stack: depth 12.0, tide +1.5, draft 10.8, squat 0.6, margin 1.0.
    let r = run(
        json!({"charted_depth": "12.0 m", "tide": "1.5 m", "draft": "10.8 m",
                       "squat": "0.6 m", "margin": "1.0 m"}),
    );
    assert!((value(&r, "clearance") - 1.1).abs() < 1e-9, "{r}");
    assert!((value(&r, "under_keel") - 2.1).abs() < 1e-9, "{r}");
    assert!((value(&r, "water_depth") - 13.5).abs() < 1e-9, "{r}");
    assert!((value(&r, "keel_depth") - 11.4).abs() < 1e-9, "{r}");
    assert!(!warnings(&r).contains(&"BELOW_MARGIN".to_string()), "{r}");
    // Negative clearance says the shortfall and carries BELOW_MARGIN.
    let short = run(
        json!({"charted_depth": "9.0 m", "tide": "0.4 m", "draft": "8.9 m",
                           "squat": "0.5 m", "margin": "1.0 m"}),
    );
    assert!(value(&short, "clearance") < 0.0, "{short}");
    assert!(
        warnings(&short).contains(&"BELOW_MARGIN".to_string()),
        "{short}"
    );
    // The sentence reports numbers; it never calls a clearance safe.
    let sentence = short["summary"].as_str().unwrap().to_ascii_lowercase();
    for word in ["safe", "adequate", "sufficient", "enough"] {
        assert!(!sentence.contains(word), "{sentence}");
    }
}

#[test]
fn under_keel_clearance_invariants() {
    let base = |tide: f64, margin: f64| {
        run(
            json!({"charted_depth": "12.0 m", "tide": format!("{tide} m"), "draft": "10.0 m",
                   "squat": "0.5 m", "margin": format!("{margin} m")}),
        )
    };
    // Linear in the tide and in the margin, with slope +1 and -1.
    let (a, b) = (base(0.0, 1.0), base(1.0, 1.0));
    assert!((value(&b, "clearance") - value(&a, "clearance") - 1.0).abs() < 1e-9);
    let c = base(0.0, 1.5);
    assert!((value(&a, "clearance") - value(&c, "clearance") - 0.5).abs() < 1e-9);
    // Clearance is the water under the keel less the margin, always.
    for (tide, margin) in [(0.0, 1.0), (1.0, 1.0), (0.0, 1.5), (2.5, 0.0)] {
        let r = base(tide, margin);
        assert!(
            (value(&r, "under_keel") - value(&r, "clearance") - margin).abs() < 1e-9,
            "{r}"
        );
    }

    // Barrass's squat grows with the square of the speed: doubling it
    // quadruples the squat, whatever the block coefficient or the channel.
    for channel in ["open-water", "confined-channel"] {
        let squat_at = |v: f64| {
            let r = run(
                json!({"charted_depth": "14.4 m", "tide": "0 m", "draft": "12.0 m",
                               "margin": "0.5 m", "speed": format!("{v} kt"),
                               "block_coefficient": 0.8, "channel": channel}),
            );
            value(&r, "squat_used")
        };
        let (one, two) = (squat_at(5.0), squat_at(10.0));
        assert!((two / one - 4.0).abs() < 1e-9, "{channel}: {one} {two}");
    }
    // A confined channel squats about twice as deep as open water (K 2 vs 1).
    let at = |channel: &str| {
        let r = run(
            json!({"charted_depth": "14.4 m", "tide": "0 m", "draft": "12.0 m",
                           "margin": "0.5 m", "speed": "10 kt", "block_coefficient": 0.8,
                           "channel": channel}),
        );
        value(&r, "squat_used")
    };
    let ratio = at("confined-channel") / at("open-water");
    assert!((ratio - 2.0).abs() < 0.01, "{ratio}");
    // Open water is K = 1, so the squat is C_B V^2 / 100 exactly.
    assert!(
        (at("open-water") - 0.8 * 100.0 / 100.0).abs() < 0.005,
        "{}",
        at("open-water")
    );

    // The other end is never deeper than the maximum, and the ends agree at
    // a block coefficient of 0.7.
    for cb in [0.5_f64, 0.6, 0.7, 0.8, 0.85] {
        let r = run(
            json!({"charted_depth": "14.4 m", "tide": "0 m", "draft": "12.0 m",
                           "margin": "0.5 m", "speed": "9 kt", "block_coefficient": cb}),
        );
        let (max, other) = (value(&r, "squat_used"), value(&r, "squat_other_end"));
        assert!(other <= max + 1e-12, "{r}");
        if (cb - 0.7).abs() < 1e-12 {
            assert!((other - max).abs() < 1e-12, "{r}");
        }
    }
}

#[test]
fn published_ranges_are_refused_outside() {
    // h/T = 2.0 is outside Barrass's published 1.1 to 1.4.
    let r = run(
        json!({"charted_depth": "20.0 m", "tide": "0 m", "draft": "10.0 m",
                       "margin": "1 m", "speed": "10 kt", "block_coefficient": 0.8}),
    );
    assert_eq!(r["error"]["code"], "OUT_OF_DOMAIN", "{r}");
    assert!(
        r["error"]["message"].as_str().unwrap().contains("1.1"),
        "{r}"
    );
    // ... but the same water with the squat given is arithmetic, and answers.
    let given = run(
        json!({"charted_depth": "20.0 m", "tide": "0 m", "draft": "10.0 m",
                           "margin": "1 m", "squat": "0.2 m"}),
    );
    assert!((value(&given, "clearance") - 8.8).abs() < 1e-9, "{given}");
    // Block coefficient and blockage factor have published ranges too.
    for bad in [
        json!({"charted_depth": "12.0 m", "tide": "0 m", "draft": "10.0 m", "margin": "1 m",
               "speed": "10 kt", "block_coefficient": 0.95}),
        json!({"charted_depth": "12.0 m", "tide": "0 m", "draft": "10.0 m", "margin": "1 m",
               "speed": "10 kt", "block_coefficient": 0.8, "channel": "blockage-factor",
               "blockage_factor": 0.05}),
    ] {
        let r = run(bad);
        assert_eq!(r["error"]["code"], "OUT_OF_DOMAIN", "{r}");
    }
}
