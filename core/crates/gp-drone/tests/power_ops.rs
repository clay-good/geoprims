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
