//! Oxygen requirement (add-aviation-practical-additions, "Oxygen requirement").

use gp_aviation::REGISTRY;
use serde_json::{Value, json};

fn run(input: Value) -> Value {
    let r: Value = serde_json::from_str(
        &REGISTRY.invoke("aviation.physiology.oxygen-requirement", &input.to_string()),
    )
    .unwrap();
    assert_eq!(r["ok"], true, "{input}: {r}");
    r["result"].clone()
}

#[test]
fn oxygen_requirement_invariants() {
    // The spec's two scenarios.
    let a = run(json!({"cabin_altitude": "13500 ft", "duration": "45 min"}));
    assert_eq!(a["paragraph"], "14 CFR 91.211(a)(1)");
    assert_eq!(a["oxygen_minutes"]["value"], 15.0);
    let b = run(json!({"cabin_altitude": "11000 ft", "duration": "40 min", "part": "135"}));
    assert_eq!(b["paragraph"], "14 CFR 135.89(a)(1)");
    assert_eq!(b["oxygen_minutes"]["value"], 10.0);
    // Minutes on oxygen never fall as the cabin climbs, for a fixed time.
    for part in ["91", "135", "121"] {
        let mut last = -1.0;
        for h in (8000..20000).step_by(250) {
            let r = run(
                json!({"cabin_altitude": format!("{h} ft"), "duration": "45 min", "part": part}),
            );
            let m = r["oxygen_minutes"]["value"].as_f64().unwrap();
            assert!(m >= last, "{part} {h}");
            last = m;
        }
    }
    // Every result carries the notice and the dated citation.
    let r = run(json!({"cabin_altitude": "16000 ft", "flight_altitude": "36000 ft"}));
    assert!(
        r["notice"]
            .as_str()
            .unwrap()
            .starts_with("Dated reference, not medical or legal advice")
    );
    assert!(
        r["rules"]
            .as_str()
            .unwrap()
            .contains("rules as of 2026-10-19")
    );
    assert!(r["pressurized"].as_str().unwrap().contains("FL350"));
}

fn invoke(id: &str, input: Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).unwrap()
}

#[test]
fn va_for_weight_invariants() {
    let va = |w: f64| {
        invoke(
            "aviation.performance.va-for-weight",
            json!({"va_max": "105 kt", "max_weight": "2550 lb", "weight": format!("{w} lb")}),
        )["result"]["va"]["value"]
            .as_f64()
            .unwrap()
    };
    // At maximum weight VA is the published VA; the design example; and VA falls with weight.
    assert!((va(2550.0) - 105.0).abs() < 1e-12);
    assert!((va(2000.0) - 92.99).abs() < 0.01);
    assert!(va(1800.0) < va(2200.0));
    // Quartering the weight halves VA.
    assert!((va(2550.0 / 4.0) - 52.5).abs() < 1e-9);
}

#[test]
fn hydroplaning_invariants() {
    let v = |p: f64| {
        invoke(
            "aviation.performance.hydroplaning-speed",
            json!({"pressure": format!("{p} psi")}),
        )["result"]["speed"]["value"]
            .as_f64()
            .unwrap()
    };
    assert_eq!(v(36.0), 54.0);
    // Four times the pressure, twice the speed.
    assert!((v(100.0) - 2.0 * v(25.0)).abs() < 1e-12);
    assert_eq!(
        invoke(
            "aviation.performance.hydroplaning-speed",
            json!({"pressure": "200 psi"})
        )["error"]["code"],
        "OUT_OF_DOMAIN"
    );
}

#[test]
fn climb_requirement_invariants() {
    let run = |v: Value| invoke("aviation.performance.climb-requirement", v);
    // The design examples: 297 ft/NM at 180 kt is 891 fpm; 1,779 ft in 5 NM is 355.8 ft/NM.
    assert_eq!(
        run(json!({"gradient": "297 ft/NM", "groundspeed": "180 kt"}))["result"]["required_rate"]["value"],
        891.0
    );
    let g = run(
        json!({"altitude": "3000 ft", "start_altitude": "1221 ft", "distance": "5 NM", "groundspeed": "120 kt"}),
    );
    assert!((g["result"]["gradient"]["value"].as_f64().unwrap() - 355.8).abs() < 1e-9);
    // A tailwind raises the rate needed, and a shortfall carries the caution.
    let r = run(
        json!({"gradient": "250 ft/NM", "airspeed": "100 kt", "headwind": "-20 kt", "rate_of_climb": "450 ft/min"}),
    );
    let codes: Vec<&str> = r["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap())
        .collect();
    assert!(codes.contains(&"CLIMB_GRADIENT_SHORT"), "{codes:?}");
    // The gradient you can make, back through the rate needed, is your rate.
    let m = run(
        json!({"gradient": "300 ft/NM", "groundspeed": "120 kt", "rate_of_climb": "700 ft/min"}),
    );
    let made = m["result"]["available_gradient"]["value"].as_f64().unwrap();
    assert!((made * 120.0 / 60.0 - 700.0).abs() < 1e-9);
}
