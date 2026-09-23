//! Drone slice 2: every spec scenario from endurance-and-power, operations-
//! reference, and the VLOS requirement.

use gp_drone::REGISTRY;
use serde_json::Value;

fn call(id: &str, input: &str) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, input)).expect("envelope is JSON")
}

fn num(r: &Value, path: &str) -> f64 {
    path.split('.')
        .fold(r, |v, k| &v[k])
        .as_f64()
        .unwrap_or_else(|| panic!("{path} missing in {r}"))
}

fn near(r: &Value, path: &str, want: f64, tol: f64) {
    let got = num(r, path);
    assert!(
        (got - want).abs() <= tol,
        "{path}: {got} vs {want} ± {tol}\n{r}"
    );
}

fn warns(r: &Value, code: &str) -> bool {
    r["meta"]["warnings"]
        .as_array()
        .is_some_and(|w| w.iter().any(|w| w["code"] == code))
}

#[test]
fn battery_energy() {
    let r = call(
        "drone.power.battery-energy",
        r#"{"capacity":"5870 mAh","voltage":"15.4 V"}"#,
    );
    near(&r, "result.energy.value", 90.4, 0.05);
    let r = call(
        "drone.power.battery-energy",
        r#"{"capacity":"5000 mAh","cells":4}"#,
    );
    near(&r, "result.voltage.value", 14.8, 1e-12);
    assert!(warns(&r, "NOMINAL_VALUE_USED"));
}

#[test]
fn c_rate_warning() {
    let r = call(
        "drone.power.battery-energy",
        r#"{"capacity":"2200 mAh","voltage":"11.1 V","power":"800 W","max_c_rate":25}"#,
    );
    near(&r, "result.c_rate", 800.0 / 11.1 / 2.2, 1e-9);
    assert!(warns(&r, "C_RATE_EXCEEDED"), "{r}");
    let r = call(
        "drone.power.battery-energy",
        r#"{"capacity":"2200 mAh","voltage":"11.1 V","power":"200 W","max_c_rate":25}"#,
    );
    assert!(!warns(&r, "C_RATE_EXCEEDED"));
}

#[test]
fn hover_power_at_sea_level() {
    let r = call(
        "drone.power.hover-power",
        r#"{"mass":"1.4 kg","rotors":4,"rotor_diameter":"9.4 in"}"#,
    );
    near(&r, "result.disk_area.value", 0.1791, 0.00005);
    near(&r, "result.ideal_power.value", 76.8, 0.05);
    near(&r, "result.electrical_power.value", 150.6, 0.05);
}

#[test]
fn density_altitude_increases_power() {
    let sl = call(
        "drone.power.hover-power",
        r#"{"mass":"1.4 kg","rotors":4,"rotor_diameter":"9.4 in"}"#,
    );
    let hi = call(
        "drone.power.hover-power",
        r#"{"mass":"1.4 kg","rotors":4,"rotor_diameter":"9.4 in","density_altitude":"8000 ft"}"#,
    );
    let ratio =
        num(&hi, "result.electrical_power.value") / num(&sl, "result.electrical_power.value");
    assert!((ratio - 1.128).abs() < 0.0005, "{ratio}");
    near(&hi, "result.density_factor", ratio, 1e-12);
}

#[test]
fn endurance_with_reserve() {
    let r = call(
        "drone.power.endurance",
        r#"{"energy":"90.4 Wh","usable":80,"power":"150.6 W"}"#,
    );
    near(&r, "result.hover_time.value", 28.8, 0.05);
    near(&r, "result.reserve_energy.value", 0.0, 1e-12);
    let r = call(
        "drone.power.endurance",
        r#"{"energy":"90.4 Wh","usable":80,"power":"150.6 W","reserve":20}"#,
    );
    near(&r, "result.hover_time.value", 28.8 * 0.8, 0.05);
    near(&r, "result.hover_time_no_reserve.value", 28.8, 0.05);
}

#[test]
fn cold_battery() {
    let r = call(
        "drone.power.endurance",
        r#"{"energy":"100 Wh","power":"100 W","battery_temperature":"0 degC"}"#,
    );
    near(&r, "result.derating_applied", 20.0, 1e-9);
    near(&r, "result.usable_energy.value", 80.0, 1e-9);
    assert!(warns(&r, "HEURISTIC_DERATING"));
    let r = call(
        "drone.power.endurance",
        r#"{"energy":"100 Wh","power":"100 W","battery_temperature":"25 degC"}"#,
    );
    assert!(!warns(&r, "HEURISTIC_DERATING"));
}

#[test]
fn maximum_payload() {
    let r = call(
        "drone.power.max-payload",
        r#"{"mass":"1.4 kg","rotors":4,"rotor_diameter":"9.4 in","usable_energy":"72 Wh","target_time":"20 min"}"#,
    );
    let extra = num(&r, "result.max_payload.value");
    assert!(extra > 0.0, "{r}");
    // Hovering at the maximum mass takes exactly the target time.
    let m = 1.4 + extra;
    let h = call(
        "drone.power.hover-power",
        &format!(r#"{{"mass":"{m} kg","rotors":4,"rotor_diameter":"9.4 in"}}"#),
    );
    near(
        &h,
        "result.electrical_power.value",
        72.0 * 60.0 / 20.0,
        1e-6,
    );
    let r = call(
        "drone.power.max-payload",
        r#"{"mass":"1.4 kg","rotors":4,"rotor_diameter":"9.4 in","usable_energy":"20 Wh","target_time":"20 min"}"#,
    );
    assert_eq!(r["error"]["code"], "NO_SOLUTION", "{r}");
}

#[test]
fn headwind_return_and_cannot_return() {
    let r = call(
        "drone.power.rth-budget",
        r#"{"distance":"1.5 km","airspeed":"15 m/s","headwind":"10 m/s","power":"180 W","remaining_energy":"40 Wh","reserve_energy":"10 Wh"}"#,
    );
    near(&r, "result.return_groundspeed.value", 5.0, 1e-12);
    near(
        &r,
        "result.return_energy.value",
        180.0 * 300.0 / 3600.0,
        1e-9,
    );
    near(&r, "result.margin.value", 40.0 - 10.0 - 15.0, 1e-9);
    let r = call(
        "drone.power.rth-budget",
        r#"{"distance":"1.5 km","airspeed":"15 m/s","headwind":"15 m/s","power":"180 W","remaining_energy":"40 Wh"}"#,
    );
    assert_eq!(r["error"]["code"], "NO_SOLUTION", "{r}");
}

#[test]
fn near_a_structure() {
    let r = call(
        "drone.ops.part107-altitude",
        r#"{"structure_height":"300 ft","structure_distance":"200 ft"}"#,
    );
    near(&r, "result.max_agl.value", 700.0, 1e-9);
    assert!(
        r["result"]["basis"]
            .as_str()
            .unwrap()
            .contains("Within 400 ft")
    );
    let r = call(
        "drone.ops.part107-altitude",
        r#"{"structure_height":"300 ft","structure_distance":"450 ft"}"#,
    );
    near(&r, "result.max_agl.value", 400.0, 1e-9);
}

#[test]
fn msl_and_hae() {
    let r = call(
        "drone.ops.part107-altitude",
        r#"{"ground_elevation":"5280 ft","geoid_height":"-17 m"}"#,
    );
    near(&r, "result.max_msl.value", 5680.0, 1e-9);
    near(&r, "result.max_hae.value", 5680.0 - 17.0 / 0.3048, 1e-9);
}

#[test]
fn disclaimer_everywhere() {
    for (id, input) in [
        ("drone.ops.part107-altitude", r#"{}"#),
        ("drone.ops.speed-check", r#"{"airspeed":"40 mph"}"#),
        (
            "drone.ops.kinetic-energy",
            r#"{"mass":"1 kg","speed":"10 m/s"}"#,
        ),
        (
            "drone.ops.easa-subcategory",
            r#"{"mass":"0.2 kg","class_mark":"none"}"#,
        ),
    ] {
        let r = call(id, input);
        let n = r["result"]["notice"]
            .as_str()
            .unwrap_or_else(|| panic!("{id}: {r}"));
        assert!(
            n.starts_with("Summary of rules as of 2026-09-18. Not legal advice."),
            "{id}: {n}"
        );
        assert!(
            r["summary"].as_str().unwrap().contains("Not legal advice"),
            "{id}"
        );
    }
}

#[test]
fn tailwind_pushes_over_the_limit() {
    let r = call(
        "drone.ops.speed-check",
        r#"{"airspeed":"90 mph","tailwind":"15 mph"}"#,
    );
    assert_eq!(r["result"]["status"], "over the limit");
    near(&r, "result.limit.value", 87.0 * 1852.0 / 1609.344, 1e-9);
}

#[test]
fn easa_c1_energy() {
    let r = call(
        "drone.ops.kinetic-energy",
        r#"{"mass":"0.9 kg","speed":"19 m/s"}"#,
    );
    near(&r, "result.energy.value", 162.5, 0.1);
    assert!(
        r["result"]["easa_c1"]
            .as_str()
            .unwrap()
            .contains("cannot meet C1")
    );
    near(
        &r,
        "result.energy_ft_lbf.value",
        162.45 / 1.3558179483314004,
        1e-9,
    );
}

#[test]
fn legacy_2_kg_drone() {
    let r = call(
        "drone.ops.easa-subcategory",
        r#"{"mass":"2 kg","class_mark":"none"}"#,
    );
    assert_eq!(r["result"]["available"], "A3");
    assert!(
        r["result"]["rules"][0]["rule"]
            .as_str()
            .unwrap()
            .contains("150 m")
    );
    let r = call(
        "drone.ops.easa-subcategory",
        r#"{"mass":"3 kg","class_mark":"c2"}"#,
    );
    assert_eq!(r["result"]["available"], "A2 and A3");
    let r = call(
        "drone.ops.easa-subcategory",
        r#"{"mass":"10 kg","class_mark":"c5"}"#,
    );
    assert_eq!(r["error"]["code"], "OUT_OF_DOMAIN");
}

#[test]
fn vlos_small_multirotor_and_mission() {
    let r = call(
        "drone.sensors.vlos",
        r#"{"characteristic_dimension":"0.35 m","farthest_distance":"400 m"}"#,
    );
    near(&r, "result.alos.value", 134.45, 1e-9);
    assert_eq!(
        r["result"]["mission_status"],
        "Beyond VLOS guidance by 266 m"
    );
    assert!(
        r["result"]["label"]
            .as_str()
            .unwrap()
            .contains("Part 107 sets no numeric VLOS")
    );
    let r = call(
        "drone.sensors.vlos",
        r#"{"characteristic_dimension":"1 m","ground_visibility":"1 km"}"#,
    );
    near(&r, "result.vlos.value", 300.0, 1e-9);
}

#[test]
fn battery_energy_invariants() {
    // Energy is linear in capacity and voltage, Ah and mAh agree, a cell
    // count gives the same answer as its nominal voltage, and usable energy
    // is the energy between the depth-of-discharge limit and the reserve.
    let e = |inp: &str| {
        num(
            &call("drone.power.battery-energy", inp),
            "result.energy.value",
        )
    };
    for (c, v) in [(5870.0, 15.4), (2200.0, 11.1), (16000.0, 51.8)] {
        let base = e(&format!(r#"{{"capacity":"{c} mAh","voltage":"{v} V"}}"#));
        assert!((base - c / 1000.0 * v).abs() < 1e-9);
        assert!(
            (e(&format!(
                r#"{{"capacity":"{} mAh","voltage":"{v} V"}}"#,
                2.0 * c
            )) - 2.0 * base)
                .abs()
                < 1e-9
        );
        assert!(
            (e(&format!(
                r#"{{"capacity":"{c} mAh","voltage":"{} V"}}"#,
                3.0 * v
            )) - 3.0 * base)
                .abs()
                < 1e-9
        );
        assert!(
            (e(&format!(
                r#"{{"capacity":"{} Ah","voltage":"{v} V"}}"#,
                c / 1000.0
            )) - base)
                .abs()
                < 1e-9
        );
    }
    for (n, chem, per) in [(4, "lipo", 3.7), (6, "li-ion", 3.6)] {
        let by_cells = e(&format!(
            r#"{{"capacity":"5000 mAh","cells":{n},"chemistry":"{chem}"}}"#
        ));
        let by_volts = e(&format!(
            r#"{{"capacity":"5000 mAh","voltage":"{} V"}}"#,
            f64::from(n) * per
        ));
        assert!((by_cells - by_volts).abs() < 1e-9);
    }
    for (dod, res) in [(100.0, 0.0), (80.0, 20.0), (90.0, 45.0)] {
        let r = call(
            "drone.power.battery-energy",
            &format!(
                r#"{{"capacity":"5000 mAh","voltage":"22.2 V","depth_of_discharge":{dod},"reserve":{res}}}"#
            ),
        );
        let full = num(&r, "result.energy.value");
        assert!((num(&r, "result.usable_energy.value") - full * (dod - res) / 100.0).abs() < 1e-9);
    }
}

#[test]
fn vlos_invariants() {
    // VLOS is the smaller of ALOS and DLOS; ALOS grows linearly with size at
    // 327 (multirotor) or 490 (fixed wing) per meter; DLOS is 0.3 × ground
    // visibility; and the margin is VLOS minus the farthest planned point.
    let v = |cd: f64, kind: &str, gv: f64, far: Option<f64>| {
        let mut inp = format!(
            r#"{{"characteristic_dimension":"{cd} m","aircraft_type":"{kind}","ground_visibility":"{gv} km""#
        );
        if let Some(f) = far {
            inp += &format!(r#","farthest_distance":"{f} m""#);
        }
        call("drone.sensors.vlos", &(inp + "}"))
    };
    for (kind, slope) in [("multirotor", 327.0), ("fixed-wing", 490.0)] {
        for gv in [0.5, 2.0, 5.0] {
            let (a, b) = (v(1.0, kind, gv, None), v(2.5, kind, gv, None));
            let (a1, a2) = (num(&a, "result.alos.value"), num(&b, "result.alos.value"));
            assert!((a2 - a1 - 1.5 * slope).abs() < 1e-9);
            for r in [&a, &b] {
                let (al, dl, vl) = (
                    num(r, "result.alos.value"),
                    num(r, "result.dlos.value"),
                    num(r, "result.vlos.value"),
                );
                assert!((dl - 300.0 * gv).abs() < 1e-9);
                assert_eq!(vl, al.min(dl));
            }
        }
        let r = v(1.2, kind, 5.0, Some(250.0));
        assert!(
            (num(&r, "result.margin.value") - (num(&r, "result.vlos.value") - 250.0)).abs() < 1e-9
        );
        // Visibility counts up to 5 km, so VLOS stops at 1,500 m however large the aircraft.
        assert_eq!(num(&v(20.0, kind, 12.0, None), "result.vlos.value"), 1500.0);
        let none = call(
            "drone.sensors.vlos",
            &format!(r#"{{"characteristic_dimension":"20 m","aircraft_type":"{kind}"}}"#),
        );
        assert_eq!(num(&none, "result.vlos.value"), 1500.0);
    }
}

#[test]
fn peukert_off_by_default() {
    let r = call(
        "drone.power.endurance",
        r#"{"energy":"100 Wh","power":"150 W"}"#,
    );
    near(&r, "result.hover_time.value", 40.0, 1e-9);
    assert!(r["result"].get("peukert_factor").is_none(), "{r}");
    assert!(!warns(&r, "HEURISTIC_PEUKERT"));
    // Entered, it is applied and labeled.
    let r = call(
        "drone.power.endurance",
        r#"{"energy":"100 Wh","power":"150 W","peukert":1.05}"#,
    );
    near(
        &r,
        "result.peukert_factor",
        (100.0_f64 / 150.0).powf(0.05),
        1e-12,
    );
    assert!(warns(&r, "HEURISTIC_PEUKERT"));
    // A rated time without an exponent is refused, not silently ignored.
    let r = call(
        "drone.power.endurance",
        r#"{"energy":"100 Wh","power":"150 W","rated_time":"1 h"}"#,
    );
    assert_eq!(r["error"]["field"], "/rated_time", "{r}");
}

#[test]
fn calibration_round_trips_through_hover_power() {
    let r = call(
        "drone.power.calibrate-hover",
        r#"{"mass":"1.4 kg","rotors":4,"rotor_diameter":"9.4 in","energy_used":"40 Wh","hover_time":"15 min"}"#,
    );
    near(&r, "result.measured_power.value", 160.0, 1e-9);
    let fm = num(&r, "result.figure_of_merit");
    // Fed back into the hover tool at the same efficiency, it predicts the measured power.
    let h = call(
        "drone.power.hover-power",
        &format!(
            r#"{{"mass":"1.4 kg","rotors":4,"rotor_diameter":"9.4 in","figure_of_merit":{fm}}}"#
        ),
    );
    near(&h, "result.electrical_power.value", 160.0, 1e-9);
}

#[test]
fn endurance_invariants() {
    // Time is energy over power: linear in energy, inverse in power, and the
    // usable share, derating, and reserve each scale it by their own factor.
    // The reserve is a share of the usable energy, and the two times differ
    // by exactly that share. Range is time × groundspeed, and fed the usable
    // energy from the battery tool, the time is that energy over the power.
    let t = |inp: &str| {
        num(
            &call("drone.power.endurance", inp),
            "result.hover_time.value",
        )
    };
    for (e, p) in [(90.4, 150.6), (45.0, 60.0), (500.0, 1200.0)] {
        let base = t(&format!(r#"{{"energy":"{e} Wh","power":"{p} W"}}"#));
        assert!((base - e / p * 60.0).abs() < 1e-9);
        assert!(
            (t(&format!(r#"{{"energy":"{} Wh","power":"{p} W"}}"#, 2.0 * e)) - 2.0 * base).abs()
                < 1e-9
        );
        assert!(
            (t(&format!(r#"{{"energy":"{e} Wh","power":"{} W"}}"#, 4.0 * p)) - base / 4.0).abs()
                < 1e-9
        );
        for (u, d, r) in [(80.0, 0.0, 20.0), (90.0, 15.0, 30.0), (100.0, 50.0, 0.0)] {
            let res = call(
                "drone.power.endurance",
                &format!(
                    r#"{{"energy":"{e} Wh","power":"{p} W","usable":{u},"derating":{d},"reserve":{r},"groundspeed":"10 m/s"}}"#
                ),
            );
            let f = u / 100.0 * (1.0 - d / 100.0);
            let h = num(&res, "result.hover_time.value");
            assert!((h - base * f * (1.0 - r / 100.0)).abs() < 1e-9);
            assert!(
                (num(&res, "result.hover_time_no_reserve.value") * (1.0 - r / 100.0) - h).abs()
                    < 1e-9
            );
            assert!(
                (num(&res, "result.usable_energy.value") * r / 100.0
                    - num(&res, "result.reserve_energy.value"))
                .abs()
                    < 1e-9
            );
            assert!((num(&res, "result.range.value") - h * 60.0 * 10.0 / 1000.0).abs() < 1e-9);
        }
    }
    // Colder never flies longer; the heuristic is flat from 20 °C up, warns
    // only below it, and is capped at 50%.
    let cold = |c: f64| {
        call(
            "drone.power.endurance",
            &format!(r#"{{"energy":"100 Wh","power":"100 W","battery_temperature":"{c} degC"}}"#),
        )
    };
    let mut prev = f64::INFINITY;
    for c in [40.0, 20.0, 15.0, 0.0, -10.0, -30.0, -60.0] {
        let r = cold(c);
        let h = num(&r, "result.hover_time.value");
        assert!(h <= prev, "{c}: {h} > {prev}");
        prev = h;
        assert_eq!(warns(&r, "HEURISTIC_DERATING"), c < 20.0, "{c}\n{r}");
    }
    near(&cold(-60.0), "result.derating_applied", 50.0, 1e-9);
    // Chained from the battery tool, the time is its usable energy over the power.
    let b = call(
        "drone.power.battery-energy",
        r#"{"capacity":"5870 mAh","voltage":"15.4 V","depth_of_discharge":80,"reserve":20}"#,
    );
    let usable = num(&b, "result.usable_energy.value");
    near(
        &call(
            "drone.power.endurance",
            &format!(r#"{{"energy":"{usable} Wh","power":"150.6 W"}}"#),
        ),
        "result.hover_time.value",
        usable / 150.6 * 60.0,
        1e-9,
    );
}
