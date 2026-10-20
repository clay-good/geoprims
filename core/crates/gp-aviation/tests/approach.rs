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

#[test]
fn rvr_visibility_invariants() {
    let vis = |rvr: &str| -> Value {
        serde_json::from_str(&REGISTRY.invoke(
            "aviation.ifr.rvr-visibility",
            &json!({"rvr": rvr}).to_string(),
        ))
        .unwrap()
    };
    // Every row of the regulation's table, exactly, and visibility rises with RVR.
    let mut last = 0.0;
    for (rvr, want) in [
        ("1600 ft", "1/4"),
        ("2400 ft", "1/2"),
        ("3200 ft", "5/8"),
        ("4000 ft", "3/4"),
        ("4500 ft", "7/8"),
        ("5000 ft", "1"),
        ("6000 ft", "1 1/4"),
    ] {
        let r = vis(rvr);
        assert_eq!(r["result"]["visibility"], want, "{rvr}");
        let sm = r["result"]["visibility_sm"]["value"].as_f64().unwrap();
        assert!(sm > last);
        last = sm;
    }
    // No interpolation between rows.
    assert_eq!(vis("2000 ft")["error"]["code"], "INVALID_INPUT");
}
