//! The US 1976 Standard Atmosphere at every tabulated kilometer from 0 to 81
//! km against the ambiance package, an independent implementation of the
//! standard (tools/vectors/gen_us76_ambiance.py), plus every printed 0.5 km
//! row from 82 to 86 km against NOAA-S/T 76-1562 Table I. The official rows
//! must be within two units of the last printed digit (some do not round from
//! the defining equations to the nearest digit). ambiance reports the
//! molecular-scale temperature above 80 km, so its temperature is compared
//! only through 80 km.

use gp_aviation::REGISTRY;
use serde_json::{Value, json};

/// Half a unit in the fifth significant figure of `v`.
fn half_unit(v: f64) -> f64 {
    0.5 * 10f64.powi(v.abs().log10().floor() as i32 - 4)
}

/// Farther apart than `tol`, or not a number at all.
fn off(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() > tol || a.is_nan()
}

#[test]
fn us76_matches_ambiance_through_81_km() {
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
        if c[0] <= 80.0 && off(t, c[1], 0.0005) {
            wrong.push(format!("{} km: T {t} against {}", c[0], c[1]));
        }
        if off(p, c[2], half_unit(c[2]).max(c[2].abs() * 1e-5)) {
            wrong.push(format!("{} km: p {p} against {}", c[0], c[2]));
        }
        if off(rho, c[3], half_unit(c[3]).max(c[3].abs() * 1e-5)) {
            wrong.push(format!("{} km: rho {rho} against {}", c[0], c[3]));
        }
    }
    assert_eq!(n, 82);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn us76_upper_atmosphere_matches_noaa_table_i() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/us76_upper_table.csv"
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
        for (name, got, want, tolerance) in [
            ("T", v("temperature"), c[1], c[2]),
            ("p", v("pressure"), c[3], c[4]),
            ("rho", v("density"), c[5], c[6]),
        ] {
            if off(got, want, tolerance) {
                wrong.push(format!(
                    "{} km: {name} {got} against {want} (tolerance {tolerance})",
                    c[0]
                ));
            }
        }
    }
    assert_eq!(n, 9);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
