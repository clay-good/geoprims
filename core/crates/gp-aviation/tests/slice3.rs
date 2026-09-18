//! Aviation slice 3: weather decoding and holding, every spec scenario these
//! tools cover, plus decoder cases hand-checked against FAA JO 7900.5E.

use gp_aviation::REGISTRY;
use serde_json::Value;

fn call(id: &str, input: &str) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, input)).expect("envelope is JSON")
}

fn num(r: &Value, path: &str) -> f64 {
    path.split('.')
        .fold(r, |v, k| {
            if let Ok(i) = k.parse::<usize>() {
                &v[i]
            } else {
                &v[k]
            }
        })
        .as_f64()
        .unwrap_or_else(|| panic!("{path} missing in {r}"))
}

fn warns(r: &Value, code: &str) -> bool {
    r["meta"]["warnings"]
        .as_array()
        .is_some_and(|w| w.iter().any(|w| w["code"] == code))
}

const M: &str = "aviation.weather.metar-decode";

fn metar(report: &str) -> Value {
    let r = call(M, &serde_json::json!({ "report": report }).to_string());
    assert_eq!(r["ok"], true, "{r}");
    r
}

#[test]
fn full_us_metar() {
    let r =
        metar("KDEN 181753Z 30015G25KT 10SM FEW080 SCT200 30/08 A2980 RMK AO2 SLP052 T03000083");
    assert_eq!(num(&r, "result.wind_direction.value"), 300.0);
    assert_eq!(num(&r, "result.wind_speed.value"), 15.0);
    assert_eq!(num(&r, "result.wind_gust.value"), 25.0);
    assert!(r["result"]["wind"].as_str().unwrap().contains("true"));
    assert_eq!(num(&r, "result.visibility.value"), 10.0);
    assert_eq!(r["result"]["clouds"][0]["cover"], "few");
    assert_eq!(num(&r, "result.clouds.0.base.value"), 8000.0);
    assert_eq!(r["result"]["clouds"][1]["cover"], "scattered");
    assert_eq!(num(&r, "result.clouds.1.base.value"), 20000.0);
    assert_eq!(num(&r, "result.temperature.value"), 30.0);
    assert!((num(&r, "result.dew_point.value") - 8.3).abs() < 1e-12);
    assert_eq!(num(&r, "result.altimeter.value"), 29.80);
    assert!((num(&r, "result.sea_level_pressure.value") - 1005.2).abs() < 1e-9);
    assert_eq!(r["result"]["flight_category"], "VFR");
    assert!(
        r["result"]["not_decoded"].as_array().unwrap().is_empty(),
        "{r}"
    );
    assert!(r["summary"].as_str().unwrap().contains("official briefing"));
    assert_eq!(
        r["result"]["notice"],
        "Decoded from the text you pasted. Get a current official briefing before flight."
    );
}

#[test]
fn undecoded_group_listed_with_position() {
    let r = metar("KDEN 181753Z 30015KT 10SM CLR 30/08 A2980 RMK AO2 XYZZY SLP052");
    let nd = r["result"]["not_decoded"].as_array().unwrap();
    assert_eq!(nd.len(), 1, "{r}");
    assert_eq!(nd[0]["group"], "XYZZY");
    assert_eq!(nd[0]["position"], 10.0);
}

#[test]
fn low_ifr_with_fractions_weather_and_vv() {
    let r = metar(
        "KSFO 010856Z AUTO 00000KT 1 1/2SM -RA BR VV004 12/12 A3001 RMK AO2 SLP163 T01170117 $",
    );
    assert_eq!(r["result"]["wind"], "calm");
    assert_eq!(num(&r, "result.visibility.value"), 1.5);
    assert_eq!(r["result"]["weather"], "light rain; mist");
    assert_eq!(num(&r, "result.ceiling.value"), 400.0);
    assert_eq!(r["result"]["flight_category"], "LIFR");
    assert_eq!(r["result"]["modifier"], "AUTO");
    assert!((num(&r, "result.sea_level_pressure.value") - 1016.3).abs() < 1e-9);
    assert!((num(&r, "result.temperature.value") - 11.7).abs() < 1e-12);
    let g = r["result"]["groups"].as_array().unwrap();
    assert!(
        g.iter()
            .any(|x| x["group"] == "$" && x["meaning"].as_str().unwrap().contains("maintenance"))
    );
}

#[test]
fn negative_temperatures_variable_wind_and_remarks() {
    let r = metar(
        "SPECI PAFA 152253Z 22012G22KT 180V250 M1/4SM +FZRA BKN008 OVC015 M05/M07 A2992 RMK AO2 PK WND 23032/2240 WSHFT 2235 FROPA PRESFR SLP987 T10501072",
    );
    assert_eq!(r["result"]["report_type"], "SPECI");
    assert!(
        r["result"]["wind"]
            .as_str()
            .unwrap()
            .contains("varying from 180° to 250°")
    );
    assert!((num(&r, "result.visibility.value") - 0.25).abs() < 1e-12);
    assert_eq!(r["result"]["visibility_text"], "less than 1/4 SM");
    assert_eq!(r["result"]["weather"], "heavy freezing rain");
    assert_eq!(num(&r, "result.ceiling.value"), 800.0);
    assert!((num(&r, "result.temperature.value") + 5.0).abs() < 1e-12);
    assert!((num(&r, "result.dew_point.value") + 7.2).abs() < 1e-12);
    // SLP987 is 998.7 hPa (the value nearest 1000).
    assert!((num(&r, "result.sea_level_pressure.value") - 998.7).abs() < 1e-9);
    assert!(
        r["result"]["not_decoded"].as_array().unwrap().is_empty(),
        "{r}"
    );
    let g = serde_json::to_string(&r["result"]["groups"]).unwrap();
    assert!(g.contains("peak wind 230° true at 32 kt at 2240Z"));
    assert!(g.contains("frontal passage"));
    assert!(g.contains("falling rapidly"));
}

#[test]
fn international_metar() {
    let r = metar("EGLL 181750Z 24012KT 9999 FEW035 17/09 Q1013 NOSIG");
    assert_eq!(num(&r, "result.altimeter.value"), 1013.0);
    assert_eq!(r["result"]["altimeter"]["unit"], "hPa");
    assert_eq!(r["result"]["visibility_text"], "10 km or more");
    let r = metar("LFPG 181800Z 18005KT CAVOK 22/12 Q1018");
    assert_eq!(r["result"]["flight_category"], "VFR");
    let r = metar("KJFK 181751Z 04008KT 3SM TSRA BKN015CB 22/20 A2990 RMK R04R/2400FT");
    assert_eq!(r["result"]["flight_category"], "MVFR");
    assert_eq!(r["result"]["clouds"][0]["type"], "cumulonimbus");
    assert_eq!(r["result"]["weather"], "thunderstorm rain");
}

#[test]
fn rvr_group() {
    let r = metar("KORD 181751Z 27010KT 1/2SM R28L/2400V4000FT FG OVC002 10/10 A2995");
    let g = serde_json::to_string(&r["result"]["groups"]).unwrap();
    assert!(
        g.contains("runway 28L visual range 2400 ft to 4000 ft"),
        "{g}"
    );
}

#[test]
fn old_observation() {
    let r = call(
        M,
        r#"{"report":"KDEN 181753Z 30015KT 10SM CLR 30/08 A2980","current_time":"182053Z"}"#,
    );
    assert!(warns(&r, "OBSERVATION_OLD"), "{r}");
    let r = call(
        M,
        r#"{"report":"KDEN 181753Z 30015KT 10SM CLR 30/08 A2980","current_time":"181853Z"}"#,
    );
    assert!(!warns(&r, "OBSERVATION_OLD"));
    // Month wrap: a report from the 31st checked on the 1st.
    let r = call(
        M,
        r#"{"report":"KDEN 312330Z 30015KT 10SM CLR 30/08 A2980","current_time":"2026-11-01T00:30Z"}"#,
    );
    assert!(!warns(&r, "OBSERVATION_OLD"), "{r}");
    let r = call(
        M,
        r#"{"report":"KDEN 312330Z 30015KT 10SM CLR 30/08 A2980","current_time":"2026-11-01T03:30Z"}"#,
    );
    assert!(warns(&r, "OBSERVATION_OLD"), "{r}");
}

#[test]
fn not_a_metar() {
    let r = call(M, r#"{"report":"hello world"}"#);
    assert_eq!(r["error"]["code"], "INVALID_INPUT");
}

const FB: &str = "aviation.weather.fb-winds-decode";

#[test]
fn fb_high_speed() {
    let r = call(FB, r#"{"report":"731960","level":"34000 ft"}"#);
    assert_eq!(num(&r, "result.winds.0.direction.value"), 230.0, "{r}");
    assert_eq!(num(&r, "result.winds.0.speed.value"), 119.0);
    assert_eq!(num(&r, "result.winds.0.temperature.value"), -60.0);
}

#[test]
fn fb_light_and_variable() {
    let r = call(FB, r#"{"report":"9900+05","level":"9000 ft"}"#);
    assert_eq!(
        r["result"]["winds"][0]["text"], "light and variable (less than 5 kt), 5 °C",
        "{r}"
    );
    assert!(r["result"]["winds"][0].get("direction").is_none());
}

#[test]
fn fb_station_line_aligns_right() {
    // Denver: no 3,000 or 6,000 ft winds (within 1,500 ft of the station).
    let r = call(
        FB,
        r#"{"report":"DEN 2321-04 2532-14 2540-26 2447-38 245152 245657 245358"}"#,
    );
    let w = r["result"]["winds"].as_array().unwrap();
    assert_eq!(w.len(), 7, "{r}");
    assert_eq!(num(&r, "result.winds.0.level.value"), 9000.0);
    assert_eq!(num(&r, "result.winds.4.temperature.value"), -52.0);
    assert_eq!(num(&r, "result.winds.4.level.value"), 30000.0);
    assert_eq!(num(&r, "result.winds.6.speed.value"), 53.0);
    assert_eq!(r["result"]["station"], "DEN");
    // 3,000 ft has no temperature.
    let r = call(
        FB,
        r#"{"report":"MKC 9900 2415+11 2420+06 2425+01 2535-11 2545-23 255038 255548 256058"}"#,
    );
    assert!(r["result"]["winds"][0].get("temperature").is_none());
    assert_eq!(num(&r, "result.winds.8.temperature.value"), -58.0);
}

#[test]
fn fb_rejects_bad_groups() {
    for g in ["4020", "2x14", "3620+5"] {
        let r = call(FB, &format!(r#"{{"report":"{g}","level":"9000 ft"}}"#));
        assert_eq!(r["error"]["code"], "INVALID_INPUT", "{g}: {r}");
    }
}

const H: &str = "aviation.ifr.hold-entry";

fn entry(ic: f64, hdg: f64, turns: &str) -> Value {
    call(
        H,
        &format!(r#"{{"inbound_course":{ic},"heading":{hdg},"turns":"{turns}"}}"#),
    )
}

#[test]
fn arriving_along_the_inbound_course_is_direct() {
    for ic in [0.0, 90.0, 237.0, 359.0] {
        assert_eq!(entry(ic, ic, "right")["result"]["entry"], "direct");
        assert_eq!(entry(ic, ic, "left")["result"]["entry"], "direct");
    }
}

#[test]
fn right_turn_sectors() {
    for (h, want) in [
        (90.0, "direct"),
        (150.0, "teardrop"),
        (240.0, "parallel"),
        (300.0, "direct"),
        (200.0, "parallel"),
    ] {
        let r = entry(360.0, h, "right");
        assert_eq!(r["result"]["entry"], want, "heading {h}: {r}");
    }
    let r = entry(360.0, 150.0, "right");
    assert_eq!(num(&r, "result.teardrop_heading.value"), 150.0);
}

#[test]
fn left_turns_mirror() {
    for rel in [
        -170.0, -150.0, -120.0, -60.0, 0.0, 45.0, 100.0, 150.0, 175.0,
    ] {
        let right = entry(180.0, (180.0f64 + rel).rem_euclid(360.0), "right");
        let left = entry(180.0, (180.0f64 - rel).rem_euclid(360.0), "left");
        assert_eq!(
            right["result"]["entry"], left["result"]["entry"],
            "rel {rel}"
        );
    }
    let r = entry(360.0, 210.0, "left");
    assert_eq!(r["result"]["entry"], "teardrop");
    assert_eq!(num(&r, "result.teardrop_heading.value"), 210.0);
}

#[test]
fn boundary_either_entry() {
    let r = entry(360.0, 112.0, "right");
    assert_eq!(r["result"]["entry"], "teardrop");
    assert_eq!(r["result"]["alternative"], "direct");
    assert!(r["summary"].as_str().unwrap().contains("also acceptable"));
    let r = entry(360.0, 292.0, "right");
    assert_eq!(r["result"]["alternative"], "parallel", "{r}");
    let r = entry(360.0, 90.0, "right");
    assert!(r["result"].get("alternative").is_none());
}

#[test]
fn holding_speed_limit() {
    let r = call(
        "aviation.ifr.hold-speed-limit",
        r#"{"altitude":"8000 ft","planned_ias":"240 kt"}"#,
    );
    assert_eq!(num(&r, "result.max_ias.value"), 230.0);
    assert!(warns(&r, "ABOVE_MAX_HOLDING_SPEED"), "{r}");
    for (alt, v) in [
        (3000.0, 200.0),
        (6000.0, 200.0),
        (6001.0, 230.0),
        (14000.0, 230.0),
        (14001.0, 265.0),
        (41000.0, 265.0),
    ] {
        let r = call(
            "aviation.ifr.hold-speed-limit",
            &format!(r#"{{"altitude":"{alt} ft"}}"#),
        );
        assert_eq!(num(&r, "result.max_ias.value"), v, "{alt}");
    }
    let r = call(
        "aviation.ifr.hold-speed-limit",
        r#"{"altitude":"5000 ft","planned_ias":"190 kt","published_limit":"175 kt"}"#,
    );
    assert!(warns(&r, "ABOVE_MAX_HOLDING_SPEED"));
}

#[test]
fn holding_wind_timing() {
    // Direct headwind inbound: no drift, shorter outbound time.
    let r = call(
        "aviation.ifr.hold-wind-timing",
        r#"{"inbound_course":360,"tas":"120 kt","wind_direction":360,"wind_speed":"20 kt"}"#,
    );
    assert!(num(&r, "result.inbound_wca.value").abs() < 1e-12);
    assert!((num(&r, "result.outbound_time.value") - 60.0 * 100.0 / 140.0).abs() < 1e-9);
    // Crosswind from the left: correct left inbound, triple it to the right outbound.
    let r = call(
        "aviation.ifr.hold-wind-timing",
        r#"{"inbound_course":360,"tas":"120 kt","wind_direction":270,"wind_speed":"20 kt"}"#,
    );
    let w = num(&r, "result.inbound_wca.value");
    assert!(w < 0.0);
    assert!((num(&r, "result.outbound_heading.value") - (180.0 - 3.0 * w)).abs() < 1e-9);
}

#[test]
fn vdp_rules() {
    let r = call(
        "aviation.performance.vdp",
        r#"{"height_above_touchdown":"400 ft"}"#,
    );
    assert!((num(&r, "result.distance.value") - 1.26).abs() < 0.005);
    assert!((num(&r, "result.rule_hat_300.value") - 1.33).abs() < 0.005);
    assert!((num(&r, "result.rule_hat_318.value") - 400.0 / 318.0).abs() < 1e-12);
}

#[test]
fn glidepath_vertical_speed() {
    // θ = 3°, 120 kt: about 637 fpm.
    let r = call(
        "aviation.performance.climb-gradient",
        r#"{"angle":"3 deg","groundspeed":"120 kt"}"#,
    );
    assert!(
        (num(&r, "result.vertical_speed.value") - 637.0).abs() < 0.5,
        "{r}"
    );
}
