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

#[test]
fn circling_radius_invariants() {
    let car = |input: Value| {
        let r: Value = serde_json::from_str(
            &REGISTRY.invoke("aviation.ifr.circling-radius", &input.to_string()),
        )
        .unwrap();
        r
    };
    let nm = |r: &Value, k: &str| r["result"][k]["value"].as_f64().unwrap();
    // An independent number: IPH Figure 4-2's text, Category C at a sea-level
    // airport with a 1,500 ft HAA, 2.86 NM.
    let iph =
        car(json!({"category": "C", "airport_elevation": "0 ft", "circling_altitude": "1500 ft"}));
    assert!((nm(&iph, "radius") - 2.86).abs() < 0.005, "{iph}");
    // The spec's category C scenario shows Table 2-7-1's inputs and is at least 1.30 NM.
    let c =
        car(json!({"category": "C", "airport_elevation": "0 ft", "circling_altitude": "1000 ft"}));
    assert_eq!(nm(&c, "ias"), 140.0);
    assert_eq!(nm(&c, "bank"), 20.0);
    assert_eq!(nm(&c, "straight_segment"), 0.5);
    assert!(nm(&c, "radius") >= 1.30);
    // Category A at sea level: the formula falls below 1.30 NM, and the minimum applies.
    let a = car(json!({"category": "A", "airport_elevation": "0 ft"}));
    assert!(nm(&a, "radius_formula") < 1.30);
    assert_eq!(nm(&a, "radius"), 1.30);
    assert!(
        a["summary"].as_str().unwrap().contains("1.30 NM minimum"),
        "{a}"
    );
    // The height used is never under 1,000 ft above the airport.
    let low = car(
        json!({"category": "B", "airport_elevation": "2000 ft", "circling_altitude": "2400 ft"}),
    );
    assert_eq!(nm(&low, "height_used"), 1000.0);
    assert_eq!(
        nm(&low, "radius"),
        nm(
            &car(json!({"category": "B", "airport_elevation": "2000 ft"})),
            "radius"
        )
    );
    // Higher airports and faster categories need more room; the legacy radii are shown.
    for (cat, legacy) in [("A", 1.3), ("B", 1.5), ("C", 1.7), ("D", 2.3), ("E", 4.5)] {
        let sea = car(json!({"category": cat, "airport_elevation": "0 ft"}));
        let high = car(json!({"category": cat, "airport_elevation": "5000 ft"}));
        assert!(
            nm(&high, "radius_formula") > nm(&sea, "radius_formula"),
            "{cat}"
        );
        assert_eq!(nm(&sea, "legacy_radius"), legacy, "{cat}");
    }
    let order: Vec<f64> = ["A", "B", "C", "D", "E"]
        .iter()
        .map(|c| {
            nm(
                &car(json!({"category": c, "airport_elevation": "2000 ft"})),
                "radius",
            )
        })
        .collect();
    assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?}");
    // A circling altitude below the airport is refused.
    let bad = car(
        json!({"category": "C", "airport_elevation": "1000 ft", "circling_altitude": "900 ft"}),
    );
    assert_eq!(bad["error"]["code"], "OUT_OF_DOMAIN");
}
