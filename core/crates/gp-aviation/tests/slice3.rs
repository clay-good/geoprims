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
    assert_eq!(r["result"]["weather"], "thunderstorm with rain");
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

/// The entry depends only on the arrival angle toward the holding side, a
/// left hold mirrors a right one (including exactly on a boundary), the three
/// sectors are 180°, 70°, and 110° wide, a second entry is offered exactly
/// within 5° of a boundary and is always the neighboring sector, and the
/// headings are the outbound course and 30° off it toward the holding side.
#[test]
fn hold_entry_invariants() {
    let bounds = [-70.0f64, 110.0, 180.0];
    let neighbors = |e: &str| match e {
        "direct" => vec!["teardrop", "parallel"],
        "teardrop" => vec!["direct", "parallel"],
        _ => vec!["direct", "teardrop"],
    };
    let mut reached = 0;
    for ic in (0..360).step_by(29).map(f64::from) {
        let mut width = std::collections::HashMap::new();
        for rel in (-179..=180).map(f64::from) {
            let right = entry(ic, (ic + rel).rem_euclid(360.0), "right");
            let left = entry(ic, (ic - rel).rem_euclid(360.0), "left");
            let e = right["result"]["entry"].as_str().unwrap().to_owned();
            assert_eq!(left["result"]["entry"], e, "mirror at ic {ic} rel {rel}");
            assert_eq!(
                right["result"]["alternative"], left["result"]["alternative"],
                "mirrored alternative at ic {ic} rel {rel}"
            );
            // Rotating the whole picture does not change the answer.
            let turned = entry(
                (ic + 100.0).rem_euclid(360.0),
                (ic + 100.0 + rel).rem_euclid(360.0),
                "right",
            );
            assert_eq!(
                turned["result"]["entry"], e,
                "rotation at ic {ic} rel {rel}"
            );
            *width.entry(e.clone()).or_insert(0) += 1;
            let d = bounds
                .iter()
                .map(|b| {
                    let x = (rel - b).abs();
                    x.min(360.0 - x)
                })
                .fold(f64::INFINITY, f64::min);
            match right["result"].get("alternative") {
                Some(a) => {
                    assert!(d <= 5.0, "alternative {a} {d}° from a boundary");
                    assert!(neighbors(&e).contains(&a.as_str().unwrap()), "{right}");
                }
                None => assert!(d > 5.0, "no alternative {d}° from a boundary: {right}"),
            }
            let oc = (ic + 180.0).rem_euclid(360.0);
            assert!((num(&right, "result.outbound_course.value") - oc).abs() < 1e-9);
            assert!((num(&right, "result.parallel_heading.value") - oc).abs() < 1e-9);
            let tr = num(&right, "result.teardrop_heading.value");
            let tl = num(&left, "result.teardrop_heading.value");
            assert!(((oc - 30.0).rem_euclid(360.0) - tr).abs() < 1e-9);
            assert!(((oc + 30.0).rem_euclid(360.0) - tl).abs() < 1e-9);
            reached += 1;
        }
        // Integer angles −179..180: direct −70..110 (181), teardrop 111..180
        // (70), parallel −179..−71 (109).
        assert_eq!(width["direct"], 181, "ic {ic}");
        assert_eq!(width["teardrop"], 70, "ic {ic}");
        assert_eq!(width["parallel"], 109, "ic {ic}");
    }
    assert_eq!(reached, 13 * 360);
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

const T: &str = "aviation.weather.taf-decode";

#[test]
fn taf_validity_across_midnight() {
    let r = call(
        T,
        r#"{"report":"TAF KDEN 181720Z 1818/1918 30012G22KT P6SM SCT080 BKN200 TEMPO 1820/1824 VRB25G35KT 3SM TSRA BKN060CB FM190200 32008KT P6SM FEW100 BECMG 1910/1912 18010KT PROB30 1914/1918 3SM -SHRA BKN030","utc_offset":"-06:00"}"#,
    );
    assert_eq!(r["result"]["valid_from"], "day 18 at 1800Z", "{r}");
    assert_eq!(r["result"]["valid_to"], "day 19 at 1800Z");
    assert_eq!(num(&r, "result.valid_hours"), 24.0);
    let p = r["result"]["periods"].as_array().unwrap();
    let kinds: Vec<&str> = p.iter().map(|x| x["change"].as_str().unwrap()).collect();
    assert_eq!(
        kinds,
        ["base", "temporary", "from", "becoming", "30% probability"]
    );
    // The base ends where FM starts; FM runs to the end.
    assert_eq!(p[0]["to"], "day 19 at 0200Z");
    assert_eq!(p[2]["from"], "day 19 at 0200Z");
    assert_eq!(p[2]["to"], "day 19 at 1800Z");
    let starts: Vec<f64> = p
        .iter()
        .map(|x| x["start_hour"].as_f64().unwrap())
        .collect();
    assert_eq!(starts, [0.0, 2.0, 8.0, 16.0, 20.0]);
    assert_eq!(p[0]["from_local"], "12:00 local");
    assert_eq!(p[2]["from_local"], "20:00 local, previous day");
    assert_eq!(p[1]["weather"], "thunderstorm with rain");
    assert_eq!(p[1]["flight_category"], "MVFR");
    assert_eq!(p[4]["ceiling"]["value"], 3000.0);
    assert!(p.iter().all(|x| x.get("not_decoded").is_none()), "{r}");
    assert!(r["summary"].as_str().unwrap().contains("official briefing"));
}

#[test]
fn taf_month_end_and_wind_shear() {
    let r = call(
        T,
        r#"{"report":"TAF AMD KORD 302330Z 3100/0106 27015KT P6SM BKN025 WS020/30045KT FM010300 VRB03KT 1/2SM FG VV002"}"#,
    );
    assert_eq!(r["result"]["amendment"], "AMD", "{r}");
    assert_eq!(r["result"]["valid_to"], "day 1 at 0600Z");
    assert_eq!(num(&r, "result.valid_hours"), 30.0);
    let p = r["result"]["periods"].as_array().unwrap();
    assert_eq!(p[1]["start_hour"], 27.0);
    assert!(
        p[0]["other"]
            .as_str()
            .unwrap()
            .contains("wind shear at 2,000 ft")
    );
    assert_eq!(p[1]["flight_category"], "LIFR");
}

#[test]
fn taf_bad_header() {
    let r = call(T, r#"{"report":"TAF hello"}"#);
    assert_eq!(r["error"]["code"], "INVALID_INPUT");
}

#[test]
fn present_weather_reads_like_the_handbook() {
    // FAA-H-8083-28B table 24-3 and its notes: intensity qualifies the
    // precipitation ("heavy rain shower(s) is coded as +SHRA"), VC places the
    // phenomenon near the station, and +FC is a tornado or waterspout.
    for (code, words) in [
        ("+SHRA", "heavy rain showers"),
        ("+TSRA", "thunderstorm with heavy rain"),
        ("-TSRA", "thunderstorm with light rain"),
        ("TSRA", "thunderstorm with rain"),
        ("+TSRAGR", "thunderstorm with heavy rain and hail"),
        ("TS", "thunderstorm"),
        ("VCTS", "thunderstorm in the vicinity"),
        ("VCSH", "showers in the vicinity"),
        ("VCFG", "fog in the vicinity"),
        ("-SHRASN", "light rain and snow showers"),
        ("-FZDZ", "light freezing drizzle"),
        ("FZFG", "freezing fog"),
        ("BLSN", "blowing snow"),
        ("MIFG", "shallow fog"),
        ("-DZ", "light drizzle"),
        ("BR", "mist"),
        ("+FC", "tornado or waterspout"),
    ] {
        assert_eq!(
            gp_aviation::weather::parse_weather(code).as_deref(),
            Some(words),
            "{code}"
        );
    }
    for bad in ["+", "-", "VC", "XX", "RAXX"] {
        assert_eq!(gp_aviation::weather::parse_weather(bad), None, "{bad}");
    }
}

#[test]
fn metar_regressions_from_live_reports() {
    // Found by the python-metar differential on live reports (tests/metar_parity.rs).
    let decode = |r: &str| {
        call(
            "aviation.weather.metar-decode",
            &serde_json::json!({"report": r}).to_string(),
        )
    };
    // Automated stations write /// when they cannot tell the cloud type.
    let r =
        decode("METAR EKBI 191850Z AUTO 24012KT 9999 FEW015/// SCT057/// BKN200/// 16/13 Q1009");
    assert_eq!(r["result"]["clouds"].as_array().unwrap().len(), 3, "{r}");
    assert_eq!(r["result"]["ceiling"]["value"], 20000.0);
    // A trend is a forecast: it must not replace the observed wind or add layers.
    let r = decode("METAR LFBL 191900Z AUTO 30005KT CAVOK 18/10 Q1025 BECMG 36010KT");
    assert_eq!(r["result"]["wind_direction"]["value"], 300.0, "{r}");
    assert_eq!(r["result"]["wind_speed"]["value"], 5.0);
    let r = decode("METAR EKCH 191850Z 23016KT 9999 BKN009 17/17 Q1010 TEMPO BKN012");
    assert_eq!(r["result"]["clouds"].as_array().unwrap().len(), 1, "{r}");
    // NDV: the sensor cannot report directional variation.
    let r = decode("METAR LSME 191850Z AUTO 00000KT 9999NDV NCD 17/13 Q1024 RMK");
    assert_eq!(r["result"]["visibility_text"], "10 km or more", "{r}");
    assert!(
        r["result"]["not_decoded"]
            .as_array()
            .is_none_or(|a| a.is_empty()),
        "{r}"
    );
}

#[test]
fn metar_invariants() {
    // On every live report of the differential fixture: each group is either
    // explained or listed as not decoded, the flight category follows the
    // FAA thresholds from the decoded ceiling and visibility, and cutting the
    // remarks leaves the body's values unchanged.
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/metar_diff.jsonl"
    ))
    .unwrap();
    for line in text.lines().skip(1) {
        let report = serde_json::from_str::<Value>(line).unwrap()["report"]
            .as_str()
            .unwrap()
            .to_owned();
        let r = call(
            "aviation.weather.metar-decode",
            &serde_json::json!({"report": report}).to_string(),
        );
        let res = &r["result"];
        // Groups can span tokens ("1 3/4SM"), so every token must sit inside one.
        let mut seen: Vec<String> = res["groups"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|g| g["group"].as_str().unwrap().to_owned())
            .collect();
        seen.extend(
            res["not_decoded"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|g| g["group"].as_str().unwrap_or("").to_owned()),
        );
        for t in report
            .split_whitespace()
            .filter(|t| *t != "METAR" && *t != "SPECI")
        {
            assert!(
                seen.iter().any(|g| g.split_whitespace().any(|w| w == t)),
                "{report}: {t} is not accounted for"
            );
        }
        if let (Some(vis), cat) = (
            res["visibility"]["value"].as_f64(),
            res["flight_category"].as_str().unwrap_or(""),
        ) {
            let ceil = res["ceiling"]["value"].as_f64().unwrap_or(f64::INFINITY);
            let want = if ceil < 500.0 || vis < 1.0 {
                "LIFR"
            } else if ceil < 1000.0 || vis < 3.0 {
                "IFR"
            } else if ceil <= 3000.0 || vis <= 5.0 {
                "MVFR"
            } else {
                "VFR"
            };
            assert_eq!(cat, want, "{report}");
        }
        if let Some((body, _)) = report.split_once(" RMK") {
            let b = call(
                "aviation.weather.metar-decode",
                &serde_json::json!({"report": body}).to_string(),
            );
            for k in [
                "wind_direction",
                "wind_speed",
                "wind_gust",
                "visibility",
                "clouds",
                "ceiling",
                "altimeter",
                "weather",
            ] {
                assert_eq!(
                    b["result"][k], res[k],
                    "{report}: {k} changed without remarks"
                );
            }
        }
    }
}

#[test]
fn fb_invariants() {
    // Every wind from 010° to 360° at 5 to 199 kt, with any temperature,
    // round-trips through its FB code at a level with and above 24,000 ft.
    for d in (10..=360).step_by(10) {
        for spd in [5, 37, 99, 100, 150, 199] {
            for (level, t) in [(18_000, -12_i32), (18_000, 7), (34_000, -48)] {
                let mut code = format!(
                    "{:02}{:02}",
                    (d / 10 + if spd >= 100 { 50 } else { 0 }) % 100,
                    spd % 100
                );
                code += &if level > 24_000 {
                    format!("{:02}", -t)
                } else {
                    format!("{}{:02}", if t < 0 { '-' } else { '+' }, t.abs())
                };
                let r = call(
                    "aviation.weather.fb-winds-decode",
                    &serde_json::json!({"report": code, "level": format!("{level} ft")})
                        .to_string(),
                );
                let w = &r["result"]["winds"][0];
                assert_eq!(w["direction"]["value"], d, "{code}");
                assert_eq!(w["speed"]["value"], spd, "{code}");
                assert_eq!(w["temperature"]["value"], t, "{code}");
            }
        }
    }
}

#[test]
fn taf_invariants() {
    // On every live TAF of the differential fixture: the prevailing periods
    // (base and FM) tile the validity window in order with no gap or overlap,
    // every TEMPO, BECMG, and PROB period starts inside it, and the period
    // count is one plus the change groups in the text.
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/taf_diff.jsonl"
    ))
    .unwrap();
    for line in text.lines().skip(1) {
        let report = serde_json::from_str::<Value>(line).unwrap()["report"]
            .as_str()
            .unwrap()
            .to_owned();
        let r = call(
            "aviation.weather.taf-decode",
            &serde_json::json!({"report": report}).to_string(),
        );
        let res = &r["result"];
        let periods = res["periods"].as_array().unwrap();
        let hours = res["valid_hours"].as_f64().unwrap();
        let prevailing: Vec<&Value> = periods
            .iter()
            .filter(|p| matches!(p["change"].as_str(), Some("base" | "from")))
            .collect();
        assert_eq!(prevailing[0]["change"], "base", "{report}");
        assert_eq!(prevailing[0]["from"], res["valid_from"], "{report}");
        for w in prevailing.windows(2) {
            assert_eq!(
                w[0]["to"], w[1]["from"],
                "{report}: a gap or overlap between prevailing periods"
            );
            assert!(
                w[0]["start_hour"].as_f64() <= w[1]["start_hour"].as_f64(),
                "{report}"
            );
        }
        assert_eq!(
            prevailing.last().unwrap()["to"],
            res["valid_to"],
            "{report}"
        );
        // A period outside the validity (a forecaster's slip) must be flagged, never passed silently.
        let flagged = r["meta"]["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["code"] == "SUSPECT_VALUE");
        for p in periods {
            let h = p["start_hour"].as_f64().unwrap();
            assert!(
                (0.0..=hours).contains(&h) || flagged,
                "{report}: a period starts outside the validity"
            );
        }
        let body = report.split(" RMK ").next().unwrap();
        let changes = body
            .split_whitespace()
            .enumerate()
            .filter(|(i, t)| {
                t.starts_with("FM") && t.len() == 8
                    || *t == "BECMG"
                    || (*t == "TEMPO"
                        && !body
                            .split_whitespace()
                            .nth(i - 1)
                            .unwrap_or("")
                            .starts_with("PROB"))
                    || t.starts_with("PROB")
            })
            .count();
        assert_eq!(periods.len(), changes + 1, "{report}");
    }
}

#[test]
fn taf_temperature_extremes_and_stray_periods() {
    // From a live TAF (LSGG, 2026-09-19): TX/TN groups decode, and a BECMG
    // group before the valid period is flagged.
    let r = call(
        "aviation.weather.taf-decode",
        r#"{"report":"TAF LSGG 191725Z 1918/2024 06007KT CAVOK TX25/2015Z TNM03/2005Z BECMG 1916/1918 VRB02KT"}"#,
    );
    let base = &r["result"]["periods"][0];
    assert!(base["not_decoded"].is_null(), "{r}");
    assert_eq!(
        base["other"],
        "maximum temperature 25 °C on day 20 at 1500Z; minimum temperature -3 °C on day 20 at 0500Z"
    );
    assert!(
        r["meta"]["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["code"] == "SUSPECT_VALUE"),
        "{r}"
    );
}

#[test]
fn radial_fix_invariants() {
    // The fix is the station's variation east of the radial, at the distance
    // entered: the geodesic back from the fix to the station has that length,
    // a 0 distance is the station, and the radial and variation only enter as
    // their sum.
    use geographiclib_rs::{Geodesic, InverseGeodesic};
    let fix = |lat: f64, lon: f64, radial: f64, nm: f64, var: f64| {
        let r = call(
            "aviation.ifr.radial-fix",
            &format!(
                r#"{{"lat":{lat},"lon":{lon},"radial":"{radial} deg","distance":"{nm} NM","variation":"{var} deg"}}"#
            ),
        );
        assert_eq!(r["ok"], true, "{r}");
        assert!(r["result"]["wmm_declination"].is_null(), "{r}");
        (
            num(&r, "result.fix_lat.value"),
            num(&r, "result.fix_lon.value"),
            num(&r, "result.true_course.value"),
        )
    };
    let g = Geodesic::wgs84();
    for (i, &(lat, lon)) in [
        (39.8, -104.7),
        (-45.0, 170.0),
        (0.0, 0.0),
        (70.0, -179.5),
        (-80.0, 30.0),
    ]
    .iter()
    .enumerate()
    {
        for (j, &nm) in [0.0, 1.0, 12.5, 100.0, 539.0].iter().enumerate() {
            let radial = (i * 73 + j * 41) as f64 % 360.0;
            let var = (i as f64 - 2.0) * 9.5;
            let (flat, flon, tc) = fix(lat, lon, radial, nm, var);
            assert!((tc - (radial + var).rem_euclid(360.0)).abs() < 1e-12);
            let (s, az, _, _): (f64, f64, f64, f64) = g.inverse(lat, lon, flat, flon);
            assert!((s - nm * 1852.0).abs() < 1e-6, "{lat} {lon} {nm}: {s}");
            if nm > 0.0 {
                let d = (az - tc + 540.0).rem_euclid(360.0) - 180.0;
                assert!(d.abs() < 1e-6, "{lat} {lon} {nm}: {az} vs {tc}");
            }
            // Moving 7° from the radial to the variation leaves the fix alone.
            let (lat2, lon2, tc2) = fix(lat, lon, (radial - 7.0).rem_euclid(360.0), nm, var + 7.0);
            assert!((tc2 - tc).abs() < 1e-9);
            assert!((lat2 - flat).abs() < 1e-9 && (lon2 - flon).abs() < 1e-9);
        }
    }
    // The warning turns on past 1° between the station's variation and WMM,
    // and the reported difference is the variation less the declination.
    for var in [-20.0, 0.0, 7.0, 8.0, 11.0, 30.0] {
        let r = call(
            "aviation.ifr.radial-fix",
            &format!(
                r#"{{"lat":39.8,"lon":-104.7,"radial":"98 deg","distance":"12.5 NM","variation":"{var} deg","date":"2026-09-22"}}"#
            ),
        );
        let d = num(&r, "result.wmm_declination.value");
        let diff = num(&r, "result.variation_difference.value");
        assert!((diff - (var - d)).abs() < 1e-12);
        assert_eq!(
            warns(&r, "STATION_VARIATION_DIFFERS"),
            diff.abs() > 1.0,
            "{r}"
        );
    }
}

#[test]
fn tfr_area_invariants() {
    use geographiclib_rs::{Geodesic, InverseGeodesic};
    let g = Geodesic::wgs84();
    let ring = |r: &Value| -> Vec<(f64, f64)> {
        r["result"]["rings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                (
                    p["lat"]["value"].as_f64().unwrap(),
                    p["lon"]["value"].as_f64().unwrap(),
                )
            })
            .collect()
    };
    // A circle: every point is the radius from the center, the area is that
    // of a 72-sided polygon inscribed in the circle (the ellipsoid moves it
    // under 0.1% at these sizes), and the bounds hold every point.
    let gon = 36.0 * 5f64.to_radians().sin();
    for (center, nm) in [
        ("393400N1224330W", 3.0),
        ("3356S15110E", 10.0),
        ("0000N00000E", 0.5),
        ("6500N02000W", 60.0),
        ("4500S07000W", 150.0),
    ] {
        let r = call(
            "aviation.airspace.tfr-area",
            &format!(r#"{{"center":"{center}","radius":"{nm} NM"}}"#),
        );
        assert_eq!(r["ok"], true, "{r}");
        let (clat, clon) = (
            num(&r, "result.center_lat.value"),
            num(&r, "result.center_lon.value"),
        );
        let pts = ring(&r);
        assert!(pts.len() == 72 || pts.len() == 73, "{}", pts.len());
        for &(la, lo) in &pts {
            let s: f64 = g.inverse(clat, clon, la, lo);
            assert!((s - nm * 1852.0).abs() < 1e-6, "{center}: {s}");
            assert!(la >= num(&r, "result.south.value") && la <= num(&r, "result.north.value"));
            assert!(lo >= num(&r, "result.west.value") && lo <= num(&r, "result.east.value"));
        }
        let area = num(&r, "result.area.value");
        assert!(
            (area / (gon * nm * nm) - 1.0).abs() < 1e-3,
            "{center}: {area}"
        );
        // The GeoJSON ring is closed and counterclockwise (RFC 7946).
        let f: Value = serde_json::from_str(r["result"]["file"].as_str().unwrap()).unwrap();
        let c = f["geometry"]["coordinates"][0].as_array().unwrap();
        assert_eq!(c.first(), c.last(), "{center}");
        let twice: f64 = c
            .windows(2)
            .map(|w| {
                let x = |v: &Value, i: usize| v[i].as_f64().unwrap();
                x(&w[0], 0) * x(&w[1], 1) - x(&w[1], 0) * x(&w[0], 1)
            })
            .sum();
        assert!(twice > 0.0, "{center}: clockwise");
    }
    // A point list has the same area and bounds entered either way round, or
    // closed on its first point.
    let corners = [
        "393000N1050000W",
        "393000N1043000W",
        "400000N1043000W",
        "394500N1044500W",
        "400000N1050000W",
    ];
    let list = |c: &[&str]| {
        let rows: Vec<String> = c.iter().map(|p| format!(r#"{{"point":"{p}"}}"#)).collect();
        let r = call(
            "aviation.airspace.tfr-area",
            &format!(r#"{{"points":[{}]}}"#, rows.join(",")),
        );
        assert_eq!(r["ok"], true, "{r}");
        ["area", "south", "north", "west", "east"].map(|k| num(&r, &format!("result.{k}.value")))
    };
    let a = list(&corners);
    let mut rev = corners;
    rev.reverse();
    let mut closed = corners.to_vec();
    closed.push(corners[0]);
    for other in [list(&rev), list(&closed)] {
        for (x, y) in a.iter().zip(other) {
            assert!((x - y).abs() <= 1e-9 * x.abs().max(1.0), "{x} vs {y}");
        }
    }
}

#[test]
fn humidity_invariants() {
    let run = |input: String| {
        let r = call("aviation.atmosphere.humidity", &input);
        assert_eq!(r["ok"], true, "{r}");
        r
    };
    for (t, p) in [
        (-30.0, 600.0),
        (0.0, 900.0),
        (15.0, 1013.25),
        (30.0, 1000.0),
        (45.0, 1020.0),
    ] {
        let mut last_e = 0.0;
        for spread in [20.0, 10.0, 5.0, 1.0, 0.0] {
            let td = t - spread;
            let r = run(format!(
                r#"{{"temperature":"{t} degC","dew_point":"{td} degC","pressure":"{p} hPa"}}"#
            ));
            let (e, es, rh, w, tv) = (
                num(&r, "result.vapor_pressure.value"),
                num(&r, "result.saturation_vapor_pressure.value"),
                num(&r, "result.relative_humidity"),
                num(&r, "result.mixing_ratio"),
                num(&r, "result.virtual_temperature.value"),
            );
            let (moist, dry) = (
                num(&r, "result.moist_density.value"),
                num(&r, "result.dry_density.value"),
            );
            // A higher dew point holds more vapor; saturated air is at 100%.
            assert!(e > last_e && e <= es * (1.0 + 1e-12), "{r}");
            last_e = e;
            assert!((rh - 100.0 * e / es).abs() < 1e-9);
            if spread == 0.0 {
                assert!((rh - 100.0).abs() < 1e-9);
            }
            // The stated relations hold among the outputs.
            assert!((w - 622.0 * e / (p - e)).abs() < 1e-9 * w.max(1.0));
            assert!((tv + 273.15 - (t + 273.15) / (1.0 - 0.378 * e / p)).abs() < 1e-9);
            assert!((dry - p * 100.0 / (287.052_87 * (t + 273.15))).abs() < 1e-12);
            assert!((moist - p * 100.0 / (287.052_87 * (tv + 273.15))).abs() < 1e-12);
            // Vapor is lighter than air: moist air is thinner and reads warmer.
            assert!(moist < dry && tv > t, "{r}");
            // The relative humidity given back returns the same dew point.
            let back = run(format!(
                r#"{{"temperature":"{t} degC","relative_humidity":{rh},"pressure":"{p} hPa"}}"#
            ));
            assert!(
                (num(&back, "result.dew_point.value") - td).abs() < 1e-9,
                "{back}"
            );
            assert!((num(&back, "result.moist_density.value") - moist).abs() < 1e-12);
        }
    }
}

#[test]
fn q_codes_invariants() {
    let run = |input: String| {
        let r = call("aviation.altimetry.q-codes", &input);
        assert_eq!(r["ok"], true, "{r}");
        r
    };
    for elev in [-1000.0, 0.0, 1500.0, 5000.0, 9000.0, 14000.0] {
        let mut last_qfe = 0.0;
        for qnh in [960.0, 995.0, 1013.25, 1030.0, 1050.0] {
            let r = run(format!(
                r#"{{"elevation":"{elev} ft","altimeter":"{qnh} hPa"}}"#
            ));
            let (qfe, qne) = (num(&r, "result.qfe.value"), num(&r, "result.qne.value"));
            // A higher setting means more pressure at the field.
            assert!(qfe > last_qfe, "{r}");
            last_qfe = qfe;
            // At the standard setting the field's pressure altitude is its
            // elevation; a higher setting puts it below, a lower one above.
            if qnh == 1013.25 {
                assert!((qne - elev).abs() < 1e-6, "{r}");
            } else {
                assert_eq!(qne < elev, qnh > 1013.25, "{r}");
            }
            // At sea level QFE is QNH; above it QFE is lower, below it higher.
            if elev == 0.0 {
                assert!((qfe - qnh).abs() < 1e-9, "{r}");
            } else {
                assert_eq!(qfe < qnh, elev > 0.0, "{r}");
            }
            // The station pressure given back returns the same setting.
            let back = run(format!(
                r#"{{"elevation":"{elev} ft","station_pressure":"{qfe} hPa"}}"#
            ));
            assert!(
                (num(&back, "result.qnh.value") * 33.863_89 - qnh).abs() < 1e-8,
                "{back}"
            );
            assert!(
                (num(&back, "result.qne.value") - qne).abs() < 1e-6,
                "{back}"
            );
            // The NWS setting reads a little lower, by its 0.3 hPa offset and
            // its own constants: within 0.02 inHg up to 9,000 ft.
            let gap = num(&back, "result.qnh.value") - num(&back, "result.qnh_nws.value");
            if elev <= 9000.0 {
                assert!(gap > 0.0 && gap < 0.02, "{elev} {qnh}: {gap}");
            }
        }
    }
}
