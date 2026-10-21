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

#[test]
fn circling_legend_is_the_formula_at_each_band_floor() {
    // The TPP legend (cycle 2611) prints the radius by circling MDA band.
    // Each cell is Formula 2-7-2 at the band's lowest MDA, cut down to a
    // tenth and never under 1.3 NM; the tool's legend output must be that.
    let run = |cat: &str, elev: f64, mda: f64| {
        let r: Value = serde_json::from_str(&REGISTRY.invoke(
            "aviation.ifr.circling-radius",
            &json!({"category": cat, "airport_elevation": format!("{elev} ft"), "circling_altitude": format!("{mda} ft")})
                .to_string(),
        ))
        .unwrap();
        assert_eq!(r["ok"], true, "{r}");
        r["result"].clone()
    };
    for (floor, top) in [
        (0.0, 1000.0),
        (1001.0, 3000.0),
        (3001.0, 5000.0),
        (5001.0, 7000.0),
        (7001.0, 9000.0),
        (9001.0, 10_500.0),
    ] {
        for cat in ["A", "B", "C", "D", "E"] {
            // The tool's height is elevation + k with k at least 1,000 ft, so it
            // equals the band's floor from an airport 1,000 ft below it. The
            // lowest band's floor, 0 ft MSL, cannot be reached that way; its
            // cells are held by the golden vectors.
            let elev = (floor - 1000.0_f64).max(-2000.0);
            let at_floor = run(cat, elev, floor.max(elev + 1000.0));
            let formula = at_floor["radius_formula"]["value"].as_f64().unwrap();
            let expect = ((formula * 10.0).floor() / 10.0).max(1.3);
            if floor > 0.0 {
                assert!(
                    (at_floor["legend_radius"]["value"].as_f64().unwrap() - expect).abs() < 1e-9,
                    "{cat} {floor}: {at_floor}"
                );
            }
            // Anywhere in the band, the legend's number is the same.
            let top_elev = top - 1500.0;
            let at_top = run(cat, top_elev, top);
            assert_eq!(
                at_top["legend_radius"], at_floor["legend_radius"],
                "{cat} band {floor} to {top}"
            );
        }
    }
    // Without a circling altitude there is no band, and no legend radius.
    let r: Value = serde_json::from_str(&REGISTRY.invoke(
        "aviation.ifr.circling-radius",
        &json!({"category": "C", "airport_elevation": "0 ft"}).to_string(),
    ))
    .unwrap();
    assert!(r["result"].get("legend_radius").is_none());
}

#[test]
fn rvr_legend_table() {
    let run = |input: Value| -> Value {
        serde_json::from_str(&REGISTRY.invoke("aviation.ifr.rvr-visibility", &input.to_string()))
            .unwrap()
    };
    let codes = |r: &Value| -> Vec<String> {
        r["meta"]["warnings"]
            .as_array()
            .map(|w| {
                w.iter()
                    .map(|x| x["code"].as_str().unwrap().to_owned())
                    .collect()
            })
            .unwrap_or_default()
    };
    // The legend's own example: 4,800 takes the next higher row, 5,000 ft, 1 SM.
    let a = run(json!({"rvr": "4800 ft", "table": "legend"}));
    assert_eq!(a["result"]["visibility"], "1", "{a}");
    assert_eq!(a["result"]["rvr_used"]["value"], 5000.0);
    assert!(!codes(&a).contains(&"RVR_NOT_IN_REGULATION".to_owned()));
    // A row the regulation lacks carries the caution, and the sentence says so.
    let b = run(json!({"rvr": "2800 ft", "table": "legend"}));
    assert_eq!(b["result"]["visibility"], "5/8");
    assert_eq!(b["result"]["rvr_used"]["value"], 3000.0);
    assert!(
        codes(&b).contains(&"RVR_NOT_IN_REGULATION".to_owned()),
        "{b}"
    );
    assert!(
        b["summary"]
            .as_str()
            .unwrap()
            .contains("not a takeoff or landing minimum")
    );
    // Every printed row returns itself, and the seven regulation rows agree
    // with the regulation table, with no caution.
    for (rvr, vis, in_reg) in [
        (1200, "1/4", false),
        (1600, "1/4", true),
        (1800, "1/2", false),
        (2000, "1/2", false),
        (2200, "1/2", false),
        (2400, "1/2", true),
        (2600, "1/2", false),
        (3000, "5/8", false),
        (3200, "5/8", true),
        (3500, "5/8", false),
        (4000, "3/4", true),
        (4500, "7/8", true),
        (5000, "1", true),
        (5500, "1", false),
        (6000, "1 1/4", true),
    ] {
        let r = run(json!({"rvr": format!("{rvr} ft"), "table": "legend"}));
        assert_eq!(r["result"]["visibility"], vis, "{rvr}");
        assert_eq!(r["result"]["rvr_used"]["value"], f64::from(rvr));
        assert_eq!(
            codes(&r).contains(&"RVR_NOT_IN_REGULATION".to_owned()),
            !in_reg,
            "{rvr}"
        );
        if in_reg {
            let reg = run(json!({"rvr": format!("{rvr} ft")}));
            assert_eq!(reg["result"]["visibility"], vis, "{rvr}");
        }
    }
    // Outside the legend's rows it refuses; the regulation mode still refuses 2,800.
    for rvr in ["1100 ft", "6100 ft"] {
        assert_eq!(
            run(json!({"rvr": rvr, "table": "legend"}))["ok"],
            false,
            "{rvr}"
        );
    }
    assert_eq!(run(json!({"rvr": "2800 ft"}))["ok"], false);
}
