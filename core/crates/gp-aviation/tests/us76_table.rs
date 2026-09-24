//! The US 1976 Standard Atmosphere at every tabulated kilometer from 0 to 81
//! km against the ambiance package, an independent implementation of the
//! standard (tools/vectors/gen_us76_ambiance.py), within the table's printed
//! precision: temperature to 0.001 K, pressure and density to five
//! significant figures. Each value must lie within half a unit of the last
//! printed digit. ambiance stops at 81.02 km; the rows from 82 to 86 km are
//! not covered here, and it reports the molecular-scale temperature, so
//! temperature is compared to 80 km.

use gp_aviation::REGISTRY;
use serde_json::{Value, json};

/// Half a unit in the fifth significant figure of `v`.
fn half_unit(v: f64) -> f64 {
    0.5 * 10f64.powi(v.abs().log10().floor() as i32 - 4)
}

#[test]
fn us76_matches_the_table_to_its_printed_precision() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/us76_ambiance.csv"
    ))
    .unwrap();
    let mut wrong = Vec::new();
    let mut n = 0;
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let c: Vec<f64> = line.split(',').map(|x| x.parse().unwrap()).collect();
        let r: Value = serde_json::from_str(&REGISTRY.invoke(
            "aviation.atmosphere.isa",
            &json!({"altitude": format!("{} km", c[0]), "altitude_type": "geometric", "model": "us76",
                    "options": {"outputUnits": {"temperature": "K", "pressure": "Pa", "density": "kg/m3"}}})
            .to_string(),
        ))
        .unwrap();
        let v = |k: &str| r["result"][k]["value"].as_f64().unwrap_or(f64::NAN);
        n += 1;
        let (t, p, rho) = (v("temperature"), v("pressure"), v("density"));
        // Above 80 km the table's temperature is kinetic, the molecular-scale
        // temperature times M/M0, which the tool applies and ambiance does
        // not (at 81 km they differ by 0.0022 K, the factor 0.999989);
        // pressure and density do not depend on it.
        if c[0] <= 80.0 && !((t - c[1]).abs() <= 0.0005) {
            wrong.push(format!("{} km: T {t} against {}", c[0], c[1]));
        }
        if !((p - c[2]).abs() <= half_unit(c[2])) {
            wrong.push(format!("{} km: p {p} against {}", c[0], c[2]));
        }
        if !((rho - c[3]).abs() <= half_unit(c[3])) {
            wrong.push(format!("{} km: rho {rho} against {}", c[0], c[3]));
        }
    }
    assert_eq!(n, 82);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
