//! Approach category (add-aviation-practical-additions, "Approach category").

use gp_aviation::REGISTRY;
use serde_json::{Value, json};

fn category(input: Value) -> String {
    let r: Value = serde_json::from_str(
        &REGISTRY.invoke("aviation.ifr.approach-category", &input.to_string()),
    )
    .unwrap();
    assert_eq!(r["ok"], true, "{input}: {r}");
    r["result"]["category"].as_str().unwrap().to_owned()
}

#[test]
fn approach_category_invariants() {
    // The boundary scenario, and every band edge of 97.3.
    for (v, want) in [
        (90.9, "A"),
        (91.0, "B"),
        (120.9, "B"),
        (121.0, "C"),
        (140.9, "C"),
        (141.0, "D"),
        (165.9, "D"),
        (166.0, "E"),
    ] {
        assert_eq!(category(json!({"vref": format!("{v} kt")})), want, "{v}");
    }
    // The category never goes down as the speed rises.
    let order = ["A", "B", "C", "D", "E"];
    let mut last = 0;
    for v in (40..260).map(f64::from) {
        let i = order
            .iter()
            .position(|c| *c == category(json!({"vref": format!("{v} kt")})))
            .unwrap();
        assert!(i >= last, "{v}");
        last = i;
    }
    // Without VREF, 1.3 VS0 decides; with both, VREF wins.
    assert_eq!(
        category(json!({"vs0": "70 kt"})),
        category(json!({"vref": "91 kt"}))
    );
    assert_eq!(category(json!({"vref": "121 kt", "vs0": "50 kt"})), "C");
}
