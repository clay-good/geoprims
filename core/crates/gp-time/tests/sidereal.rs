//! Sidereal time (add-celestial-and-moon, time/sidereal-and-calendar).

use gp_time::REGISTRY;
use serde_json::{Value, json};
use std::f64::consts::{PI, TAU};

fn run(input: Value) -> Value {
    let r: Value =
        serde_json::from_str(&REGISTRY.invoke("time.scale.sidereal", &input.to_string())).unwrap();
    assert_eq!(r["ok"], true, "{input}: {r}");
    r
}

fn wrap(a: f64) -> f64 {
    (a + PI).rem_euclid(TAU) - PI
}

#[test]
fn matches_erfa_within_1e9_rad() {
    // The spec's check: ERFA era00, gmst06, and gst06a within 1e-9 rad, at
    // 240 random instants 1972 to 2050 with random DUT1 (fixture from pyerfa,
    // which converts UTC to TT with its own leap-second table).
    let f: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/data/erfa_sidereal.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let rows = f["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 240);
    let mut worst: [f64; 3] = [0.0; 3];
    for row in rows {
        let r = run(json!({"utc": row["utc"], "dut1": format!("{} s", row["dut1"])}));
        let era = r["result"]["era"]["value"].as_f64().unwrap().to_radians();
        let gmst = r["result"]["gmst_decimal"].as_f64().unwrap() / 24.0 * TAU;
        let gast = r["result"]["gast_decimal"].as_f64().unwrap() / 24.0 * TAU;
        for (k, (got, key)) in [(era, "era"), (gmst, "gmst"), (gast, "gast")]
            .into_iter()
            .enumerate()
        {
            let d = wrap(got - row[key].as_f64().unwrap()).abs();
            worst[k] = worst[k].max(d);
            assert!(d < 1e-9, "{} {key}: off by {d:e} rad", row["utc"]);
        }
    }
    eprintln!(
        "worst against ERFA: era {:e}, gmst {:e}, gast {:e} rad",
        worst[0], worst[1], worst[2]
    );
}

#[test]
fn j2000_and_local_sidereal_time() {
    // The spec's J2000.0 scenario: GMST 18h 41m 50.548s, ERA 280.4606°.
    let r = run(json!({"utc": "2000-01-01T12:00:00Z", "dut1": "0 s"}));
    // IAU 2006 (ERFA gmst06) gives 50.5494 s at J2000.0; the scenario's
    // 50.548 s is the IAU 1982 value, 1.4 ms less, shown beside as Meeus's.
    let secs = (r["result"]["gmst_decimal"].as_f64().unwrap() - 18.0) * 3600.0 - 41.0 * 60.0;
    assert!((secs - 50.5494).abs() <= 0.001, "{secs}");
    assert_eq!(r["result"]["gmst_meeus"], "18h 41m 50.548s");
    assert!((r["result"]["era"]["value"].as_f64().unwrap() - 280.4606).abs() < 0.0001);
    // Local sidereal time at 74°30'W is GMST less 4h 58m 00s exactly.
    let l = run(json!({"utc": "2026-10-21T06:30:00Z", "dut1": "0.05 s", "longitude": "-74.5 deg"}));
    let g = l["result"]["gmst_decimal"].as_f64().unwrap();
    let parse = |s: &str| {
        let v: Vec<f64> = s
            .split(['h', 'm', 's', ' '])
            .filter(|x| !x.is_empty())
            .map(|x| x.parse().unwrap())
            .collect();
        v[0] + v[1] / 60.0 + v[2] / 3600.0
    };
    let lmst = parse(l["result"]["lmst"].as_str().unwrap());
    let want = (g - (4.0 + 58.0 / 60.0)).rem_euclid(24.0);
    assert!(
        (lmst - want).abs() < 0.001 / 3600.0,
        "{lmst} against {want}"
    );
    // Without DUT1 it says UT1 was approximated; with it, it does not.
    let w = |r: &Value| {
        r["meta"]["warnings"]
            .as_array()
            .map(|a| a.iter().any(|x| x["code"] == "UT1_APPROXIMATED"))
            .unwrap_or(false)
    };
    assert!(w(&run(json!({"utc": "2026-10-21T06:30:00Z"}))));
    assert!(!w(&l));
    // Meeus's IAU 1982 value stays within a few milliseconds of IAU 2006 near J2000.
    let d = r["result"]["meeus_difference"]["value"].as_f64().unwrap();
    assert!(d.abs() < 0.01, "{d}");
    // DUT1 beyond ±0.9 s is refused.
    let bad: Value = serde_json::from_str(&REGISTRY.invoke(
        "time.scale.sidereal",
        &json!({"utc": "2026-10-21T06:30:00Z", "dut1": "1.2 s"}).to_string(),
    ))
    .unwrap();
    assert_eq!(bad["error"]["code"], "INVALID_INPUT");
}

#[test]
fn matches_usno_within_a_millisecond() {
    // The spec's second oracle: USNO's sidereal-time API (GMST, GAST, LMST,
    // LAST, and the equation of the equinoxes, to 0.0001 s) at 40 random UT1
    // instants and places, 1990 to 2049, within 0.001 s.
    let f: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/data/usno_sidereal.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let secs = |s: &str| {
        let p: Vec<f64> = s.split(':').map(|x| x.parse().unwrap()).collect();
        p[0] * 3600.0 + p[1] * 60.0 + p[2]
    };
    let ours = |s: &str| {
        let v: Vec<f64> = s
            .split(['h', 'm', 's', ' '])
            .filter(|x| !x.is_empty())
            .map(|x| x.parse().unwrap())
            .collect();
        v[0] * 3600.0 + v[1] * 60.0 + v[2]
    };
    let diff = |a: f64, b: f64| ((a - b + 43_200.0).rem_euclid(86_400.0) - 43_200.0).abs();
    let mut worst: f64 = 0.0;
    for row in f["rows"].as_array().unwrap() {
        // UT1 is the input: DUT1 0 makes the tool's UT1 the same instant.
        let r = run(
            json!({"utc": row["ut1"], "dut1": "0 s", "longitude": format!("{} deg", row["lon"])}),
        );
        let res = &r["result"];
        let gmst = res["gmst_decimal"].as_f64().unwrap() * 3600.0;
        let gast = res["gast_decimal"].as_f64().unwrap() * 3600.0;
        for (got, key) in [
            (gmst, "gmst"),
            (gast, "gast"),
            (ours(res["lmst"].as_str().unwrap()), "lmst"),
            (ours(res["last"].as_str().unwrap()), "last"),
        ] {
            let d = diff(got, secs(row[key].as_str().unwrap()));
            worst = worst.max(d);
            assert!(d <= 0.001, "{} {key}: {d} s", row["ut1"]);
        }
        let ee = res["ee"]["value"].as_f64().unwrap();
        assert!(
            (ee - row["eqofeq"].as_str().unwrap().parse::<f64>().unwrap()).abs() <= 0.0002,
            "{} ee",
            row["ut1"]
        );
    }
    eprintln!("worst against USNO: {worst} s");
}
