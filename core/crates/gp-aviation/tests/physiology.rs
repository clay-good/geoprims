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
