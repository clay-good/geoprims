//! Fuel density by temperature (add-aviation-practical-additions,
//! aviation/fuel-density) and its hand-off to the fuel weight tool.

use gp_aviation::REGISTRY;
use serde_json::{Value, json};

fn call(id: &str, input: Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).unwrap()
}

fn run(input: Value) -> Value {
    let r = call("aviation.loading.fuel-density-temperature", input.clone());
    assert_eq!(r["ok"], true, "{input}: {r}");
    r["result"].clone()
}

fn vcf(fuel: &str, rho: &str, t: f64) -> f64 {
    run(json!({"fuel": fuel, "density_15": rho, "temperature": format!("{t} degC")}))["vcf"]
        .as_f64()
        .unwrap()
}

#[test]
fn fuel_density_temperature_invariants() {
    // Measurement Canada's Jet A table (Table 54B, 800 kg/m³), the design's
    // seven points, within half the table's last digit.
    for (t, table) in [
        (-40.0, 1.0502),
        (-20.0, 1.0322),
        (0.0, 1.0139),
        (10.0, 1.0046),
        (20.0, 0.9953),
        (30.0, 0.9860),
        (40.0, 0.9766),
    ] {
        let v = vcf("jet-a", "800 kg/m3", t);
        assert!(
            (v - table).abs() < 6e-5,
            "Jet A at {t} °C: {v} against {table}"
        );
    }
    // The avgas table (54B, 710) and the Jet B table (54A, 760).
    assert!((vcf("100ll", "710 kg/m3", -20.0) - 1.0450).abs() < 6e-5);
    assert!((vcf("wide-cut", "760 kg/m3", 30.0) - 0.9840).abs() < 6e-5);

    // At 15 °C nothing changes, and warmer fuel is lighter.
    assert_eq!(vcf("jet-a", "800 kg/m3", 15.0), 1.0);
    let mut last = f64::INFINITY;
    for t in (-50..=60).step_by(5) {
        let d =
            run(json!({"fuel": "jet-a", "temperature": format!("{t} degC")}))["density"]["value"]
                .as_f64()
                .unwrap();
        assert!(d < last, "density rose at {t} °C");
        last = d;
    }

    // Chevron's example on its single coefficient: 1.000 gal at 0 °C is
    // 1.040 gal at 40 °C, the same mass.
    let linear = |t: f64| {
        run(
            json!({"fuel": "jet-a", "temperature": format!("{t} degC"), "method": "linear", "volume": "1 gal"}),
        )
    };
    let at15 = linear(0.0)["volume_15"]["value"].as_f64().unwrap();
    let warm = at15 / linear(40.0)["vcf"].as_f64().unwrap();
    assert!((warm - 1.040).abs() < 0.001, "{warm} gal");

    // The two methods are both shown, and they part at the cold end.
    let r = run(json!({"fuel": "jet-a", "density_15": "800 kg/m3", "temperature": "-20 degC"}));
    let gap = r["vcf_linear"].as_f64().unwrap() - r["vcf_table"].as_f64().unwrap();
    assert!(gap > 0.003 && gap < 0.004, "{gap}");

    // The density zones of Table 54B meet without a step.
    for edge in [770.5, 787.5, 838.5] {
        let below = vcf("jet-a", &format!("{} kg/m3", edge - 1e-6), -40.0);
        let above = vcf("jet-a", &format!("{edge} kg/m3"), -40.0);
        assert!(
            (below - above).abs() < 2e-4,
            "{edge}: {below} against {above}"
        );
    }

    // Weight follows volume × density at the temperature.
    let w = run(
        json!({"fuel": "jet-a", "density_15": "800 kg/m3", "temperature": "30 degC", "volume": "100 L"}),
    );
    let kg = 0.1 * 800.0 * w["vcf"].as_f64().unwrap();
    assert!((w["weight"]["value"].as_f64().unwrap() - kg / 0.453_592_37).abs() < 1e-9);

    // Outside -50 to 60 °C, and outside the tables' densities, it refuses.
    for t in ["-51 degC", "61 degC"] {
        let r = call(
            "aviation.loading.fuel-density-temperature",
            json!({"fuel": "jet-a", "temperature": t}),
        );
        assert_eq!(r["error"]["code"], "OUT_OF_DOMAIN", "{t}: {r}");
    }
    let r = call(
        "aviation.loading.fuel-density-temperature",
        json!({"density_15": "1100 kg/m3", "temperature": "0 degC"}),
    );
    assert_eq!(r["error"]["code"], "OUT_OF_DOMAIN");
    // Chevron's coefficient is for kerosine; avgas needs one entered.
    let r = call(
        "aviation.loading.fuel-density-temperature",
        json!({"fuel": "100ll", "temperature": "0 degC", "method": "linear"}),
    );
    assert_eq!(r["ok"], false);
}

#[test]
fn a_corrected_density_goes_to_fuel_weight_with_its_temperature() {
    // The spec's scenario: 0.822 kg/L at -10 °C sent to the fuel weight tool.
    let r = call(
        "aviation.loading.fuel-weight",
        json!({"volume": "50 gal", "density": "0.822 kg/L", "fuel_temperature": "-10 degC"}),
    );
    assert_eq!(r["ok"], true, "{r}");
    let summary = r["summary"].as_str().unwrap();
    assert!(
        summary.contains("-10 °C") || summary.contains("−10 °C"),
        "{summary}"
    );
    let lb = 50.0 * 3.785_411_784e-3 * 822.0 / 0.453_592_37;
    assert!((r["result"]["weight"]["value"].as_f64().unwrap() - lb).abs() < 1e-9);
    // Without the temperature, the sentence says nothing about one.
    let plain = call(
        "aviation.loading.fuel-weight",
        json!({"volume": "50 gal", "density": "0.822 kg/L"}),
    );
    assert!(!plain["summary"].as_str().unwrap().contains("°C"));
    // A temperature on a nominal density would claim a correction not made.
    let r = call(
        "aviation.loading.fuel-weight",
        json!({"volume": "50 gal", "fuel": "jet-a", "fuel_temperature": "-10 degC"}),
    );
    assert_eq!(r["ok"], false);
}
