//! Aviation slice 2: every spec scenario from the airspeed, flight-performance,
//! and fuel-and-loading specs that these tools cover.

use gp_aviation::REGISTRY;
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

// airspeed

#[test]
fn cas_250_at_fl100() {
    let r = call(
        "aviation.airspeed.cas-to-tas",
        r#"{"airspeed":"250 kt","pressure_altitude":"10000 ft","temperature":"-5 degC"}"#,
    );
    near(&r, "result.mach", 0.4523, 0.00005);
    near(&r, "result.tas.value", 288.6, 0.1);
    near(&r, "result.eas.value", 248.1, 0.1);
    // The 2% rule shows 300 kt beside the exact value, with its error.
    near(&r, "result.rule_of_thumb.value", 300.0, 1e-9);
    near(
        &r,
        "result.rule_error.value",
        300.0 - num(&r, "result.tas.value"),
        1e-9,
    );
    assert_eq!(r["result"]["flow"], "subsonic (isentropic)");
    assert!(!warns(&r, "CALIBRATION_ASSUMED"));
}

#[test]
fn cas_300_at_fl350() {
    let r = call(
        "aviation.airspeed.cas-to-tas",
        r#"{"airspeed":"300 kt","pressure_altitude":"35000 ft","temperature":"-54.3 degC"}"#,
    );
    near(&r, "result.mach", 0.8736, 0.0005);
    near(&r, "result.tas.value", 503.6, 0.1);
}

#[test]
fn supersonic_branch_round_trips() {
    // 600 KCAS at FL400 is well above Mach 1.
    let r = call(
        "aviation.airspeed.cas-to-tas",
        r#"{"airspeed":"600 kt","pressure_altitude":"40000 ft","temperature":"-56.5 degC"}"#,
    );
    assert!(num(&r, "result.mach") > 1.0, "{r}");
    assert_eq!(r["result"]["flow"], "supersonic (Rayleigh pitot)");
    let mach = num(&r, "result.mach");
    let back = call(
        "aviation.airspeed.tas-to-cas",
        &format!(r#"{{"mach":{mach},"pressure_altitude":"40000 ft"}}"#),
    );
    near(&back, "result.cas.value", 600.0, 1e-7);
}

#[test]
fn round_trip_and_compressibility_sign() {
    for cas in [40.0, 90.0, 150.0, 250.0, 350.0, 450.0, 661.0, 700.0, 900.0] {
        for ft in [-1000.0, 0.0, 5000.0, 18000.0, 36089.0, 45000.0, 60000.0] {
            let r = call(
                "aviation.airspeed.cas-to-tas",
                &format!(
                    r#"{{"airspeed":"{cas} kt","pressure_altitude":"{ft} ft","temperature_source":"isa"}}"#
                ),
            );
            let tas = num(&r, "result.tas.value");
            // Below sea level static pressure exceeds P0, so EAS > CAS there.
            if num(&r, "result.mach") <= 1.0 && ft >= 0.0 {
                assert!(
                    num(&r, "result.compressibility_correction.value") >= -1e-9,
                    "{r}"
                );
            }
            let back = call(
                "aviation.airspeed.tas-to-cas",
                &format!(
                    r#"{{"tas":"{tas} kt","pressure_altitude":"{ft} ft","temperature_source":"isa"}}"#
                ),
            );
            near(&back, "result.cas.value", cas, 1e-8 * cas);
        }
    }
}

#[test]
fn isa_temperature_must_be_chosen() {
    let r = call(
        "aviation.airspeed.cas-to-tas",
        r#"{"airspeed":"120 kt","pressure_altitude":"8000 ft"}"#,
    );
    assert_eq!(r["error"]["code"], "INVALID_INPUT", "{r}");
    assert_eq!(r["error"]["field"], "/temperature");
    let r = call(
        "aviation.airspeed.cas-to-tas",
        r#"{"airspeed":"120 kt","pressure_altitude":"8000 ft","temperature_source":"isa"}"#,
    );
    assert!(warns(&r, "ISA_TEMPERATURE_ASSUMED"), "{r}");
    assert!(r["summary"].as_str().unwrap().contains("ISA"));
}

#[test]
fn calibration_table() {
    let table = r#"[{"indicated":"60 kt","calibrated":"64 kt"},{"indicated":"100 kt","calibrated":"101 kt"},{"indicated":"160 kt","calibrated":"158 kt"}]"#;
    let r = call(
        "aviation.airspeed.cas-to-tas",
        r#"{"airspeed":"100 kt","airspeed_type":"indicated","pressure_altitude":"0 ft","temperature":"15 degC"}"#,
    );
    assert!(warns(&r, "CALIBRATION_ASSUMED"), "{r}");
    near(&r, "result.cas.value", 100.0, 1e-9);
    let r = call(
        "aviation.airspeed.cas-to-tas",
        &format!(
            r#"{{"airspeed":"130 kt","airspeed_type":"indicated","calibration":{table},"pressure_altitude":"0 ft","temperature":"15 degC"}}"#
        ),
    );
    near(&r, "result.cas.value", 129.5, 1e-9);
    assert!(!warns(&r, "CALIBRATION_ASSUMED"));
    let r = call(
        "aviation.airspeed.cas-to-tas",
        &format!(
            r#"{{"airspeed":"180 kt","airspeed_type":"indicated","calibration":{table},"pressure_altitude":"0 ft","temperature":"15 degC"}}"#
        ),
    );
    assert_eq!(r["error"]["code"], "OUT_OF_DOMAIN", "{r}");
    let msg = r["error"]["message"].as_str().unwrap();
    assert!(msg.contains("60 kt") && msg.contains("160 kt"), "{msg}");
}

#[test]
fn tat_to_sat_ram_rise() {
    let r = call(
        "aviation.airspeed.tat-sat",
        r#"{"temperature":"-20 degC","mach":0.8}"#,
    );
    near(&r, "result.sat.value", -48.73, 0.005);
    near(&r, "result.ram_rise.value", 28.73, 0.005);
    let back = call(
        "aviation.airspeed.tat-sat",
        &format!(
            r#"{{"temperature":"{} degC","temperature_kind":"static","mach":0.8}}"#,
            num(&r, "result.sat.value")
        ),
    );
    near(&back, "result.tat.value", -20.0, 1e-9);
}

// performance

#[test]
fn standard_rate_at_100_kt() {
    let r = call("aviation.performance.turn", r#"{"tas":"100 kt"}"#);
    near(&r, "result.bank.value", 15.36, 0.005);
    near(&r, "result.turn_rate.value", 3.0, 1e-12);
    near(&r, "result.rule_of_thumb_bank.value", 17.0, 1e-9);
    near(&r, "result.turn_time.value", 120.0, 1e-9);
}

#[test]
fn sixty_degree_bank() {
    let r = call(
        "aviation.performance.turn",
        r#"{"tas":"100 kt","bank":"60 deg","stall_speed":"50 kt"}"#,
    );
    near(&r, "result.load_factor", 2.0, 1e-12);
    near(&r, "result.stall_speed_in_turn.value", 70.7, 0.05);
    assert!(r["result"].get("rule_of_thumb_bank").is_none());
    // 11.294 is the feet-and-knots constant derived from g0.
    near(
        &r,
        "result.radius.value",
        100.0 * 100.0 / (11.294 * 3f64.sqrt()),
        0.05,
    );
}

#[test]
fn load_limit_exceeded() {
    let r = call(
        "aviation.performance.turn",
        r#"{"tas":"120 kt","bank":"77 deg","load_limit":3.8}"#,
    );
    assert!(warns(&r, "LOAD_LIMIT_EXCEEDED"), "{r}");
    near(
        &r,
        "result.max_bank.value",
        (1.0f64 / 3.8).acos().to_degrees(),
        1e-9,
    );
    let r = call(
        "aviation.performance.turn",
        r#"{"tas":"120 kt","bank":"45 deg","load_limit":3.8}"#,
    );
    assert!(!warns(&r, "LOAD_LIMIT_EXCEEDED"));
}

#[test]
fn turn_solves_for_tas() {
    let r = call(
        "aviation.performance.turn",
        r#"{"bank":"15.358846754747 deg","turn_rate":"3 deg/s"}"#,
    );
    near(&r, "result.tas.value", 100.0, 1e-6);
}

#[test]
fn descent_from_fl350() {
    let r = call(
        "aviation.performance.top-of-descent",
        r#"{"from_altitude":"35000 ft","to_altitude":"3000 ft","groundspeed":"420 kt","descent_angle":"3 deg"}"#,
    );
    near(&r, "result.distance.value", 100.5, 0.05);
    near(&r, "result.rule_3_to_1.value", 96.0, 1e-9);
    near(&r, "result.vertical_speed.value", 2229.0, 0.5);
    near(&r, "result.rule_5_gs.value", 2100.0, 1e-9);
    // A vertical speed instead of an angle, and no 3° rules on a steep path.
    let r = call(
        "aviation.performance.top-of-descent",
        r#"{"from_altitude":"9500 ft","to_altitude":"1500 ft","groundspeed":"150 kt","vertical_speed":"1500 fpm"}"#,
    );
    assert!(r["result"].get("rule_3_to_1").is_none(), "{r}");
    near(&r, "result.time.value", 8000.0 / 1500.0, 1e-9);
}

#[test]
fn climb_gradient_to_fpm() {
    let r = call(
        "aviation.performance.climb-gradient",
        r#"{"gradient":"200 ft/NM","groundspeed":"120 kt"}"#,
    );
    near(&r, "result.vertical_speed.value", 400.0, 1e-9);
    let r = call(
        "aviation.performance.climb-gradient",
        r#"{"vertical_speed":"400 fpm","groundspeed":"120 kt"}"#,
    );
    near(&r, "result.gradient.value", 200.0, 1e-9);
}

#[test]
fn vdp_hat_300() {
    let r = call(
        "aviation.performance.vdp",
        r#"{"height_above_touchdown":"400 ft","groundspeed":"90 kt"}"#,
    );
    near(&r, "result.rule_hat_300.value", 400.0 / 300.0, 1e-12);
    near(
        &r,
        "result.distance.value",
        400.0 * 0.3048 / 3f64.to_radians().tan() / 1852.0,
        1e-9,
    );
}

#[test]
fn glide_into_headwind() {
    let r = call(
        "aviation.performance.glide",
        r#"{"height":"5000 ft","glide_ratio":9,"tas":"70 kt","headwind":"20 kt"}"#,
    );
    near(&r, "result.still_air_range.value", 7.41, 0.005);
    near(&r, "result.wind_range.value", 5.29, 0.005);
}

#[test]
fn pivotal_altitude_100_kt() {
    let r = call(
        "aviation.performance.pivotal-altitude",
        r#"{"groundspeed":"100 kt"}"#,
    );
    near(&r, "result.pivotal_altitude.value", 885.0, 0.5);
    near(&r, "result.rule_of_thumb.value", 10000.0 / 11.3, 1e-9);
}

// loading

#[test]
fn fuel_weight_nominal() {
    let r = call(
        "aviation.loading.fuel-weight",
        r#"{"volume":"40 gal","fuel":"100ll"}"#,
    );
    near(&r, "result.weight.value", 240.0, 1e-9);
    assert_eq!(r["result"]["density_basis"], "nominal");
    assert!(warns(&r, "NOMINAL_VALUE_USED"));
    assert!(r["summary"].as_str().unwrap().contains("nominal"));
}

const STATIONS: &str = r#"[{"name":"Empty","weight":"1500 lb","arm":"85 in"},{"name":"Front seats","weight":"340 lb","arm":"90 in"},{"name":"Rear seats","weight":"170 lb","arm":"118 in"},{"name":"Fuel","weight":"240 lb","arm":"48 in"}]"#;
const ENVELOPE: &str = r#"[{"arm":"82 in","weight":"1500 lb"},{"arm":"93 in","weight":"1500 lb"},{"arm":"93 in","weight":"2300 lb"},{"arm":"84 in","weight":"2300 lb"},{"arm":"82 in","weight":"1950 lb"}]"#;

#[test]
fn cg_computation() {
    let r = call(
        "aviation.loading.weight-balance",
        &format!(r#"{{"stations":{STATIONS}}}"#),
    );
    near(&r, "result.total_weight.value", 2250.0, 1e-9);
    near(&r, "result.cg.value", 84.30, 0.005);
    near(&r, "result.total_moment", 189_680.0, 1e-6);
    assert_eq!(r["result"]["moment_unit"], "lb*in");
}

#[test]
fn landing_cg_shift() {
    let r = call(
        "aviation.loading.weight-balance",
        &format!(r#"{{"stations":{STATIONS},"fuel_burn":"60 lb","envelope":{ENVELOPE}}}"#),
    );
    near(&r, "result.landing_weight.value", 2190.0, 1e-9);
    near(
        &r,
        "result.landing_cg.value",
        (189_680.0 - 60.0 * 48.0) / 2190.0,
        1e-9,
    );
    assert_eq!(r["result"]["takeoff_status"], "inside");
    assert_eq!(r["result"]["landing_status"], "inside");
    assert!(!warns(&r, "OUTSIDE_CG_ENVELOPE"));
}

#[test]
fn out_of_envelope() {
    // Heavy rear load pushes the CG aft of 93 in.
    let stations = r#"[{"name":"Empty","weight":"1500 lb","arm":"85 in"},{"name":"Rear seats","weight":"400 lb","arm":"118 in"},{"name":"Baggage","weight":"100 lb","arm":"142 in"}]"#;
    let r = call(
        "aviation.loading.weight-balance",
        &format!(r#"{{"stations":{stations},"envelope":{ENVELOPE}}}"#),
    );
    assert_eq!(r["result"]["takeoff_status"], "outside", "{r}");
    assert!(warns(&r, "OUTSIDE_CG_ENVELOPE"));
    let cg = num(&r, "result.cg.value");
    near(&r, "result.takeoff_cg_shift.value", 93.0 - cg, 1e-9);
    // No vertical crossing at a CG aft of the whole envelope.
    assert!(r["result"].get("takeoff_weight_change").is_none());
    let msg = &r["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["code"] == "OUTSIDE_CG_ENVELOPE")
        .unwrap()["message"];
    assert!(msg.as_str().unwrap().contains("forward"), "{msg}");
}

#[test]
fn percent_mac() {
    let r = call(
        "aviation.loading.weight-balance",
        &format!(r#"{{"stations":{STATIONS},"lemac":"60 in","mac":"58 in"}}"#),
    );
    near(
        &r,
        "result.cg_mac",
        (189_680.0 / 2250.0 - 60.0) / 58.0 * 100.0,
        1e-9,
    );
}
