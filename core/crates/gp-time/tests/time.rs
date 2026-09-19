//! Time tools: catalog lint, examples, golden vectors, and spec scenarios.

use std::path::Path;

use gp_base::{manifest, template, vectors};
use gp_time::{REGISTRY, TOOLS};
use serde_json::Value;

fn repo(path: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(path);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

fn call(id: &str, input: &str) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, input)).expect("envelope is JSON")
}

fn num(r: &Value, path: &str) -> f64 {
    path.split('.')
        .fold(r, |v, k| &v[k])
        .as_f64()
        .unwrap_or_else(|| panic!("{path} missing in {r}"))
}

fn codes(r: &Value) -> Vec<String> {
    r["meta"]["warnings"].as_array().map_or(vec![], |a| {
        a.iter()
            .map(|w| w["code"].as_str().unwrap().to_owned())
            .collect()
    })
}

#[test]
fn catalog_examples_vectors() {
    let tax: Value = serde_json::from_str(&repo("data/taxonomy.json")).unwrap();
    let owned: Vec<(String, Vec<String>)> = tax["domains"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(d, v)| {
            (
                d.clone(),
                v["groups"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|g| g.as_str().unwrap().to_owned())
                    .collect(),
            )
        })
        .collect();
    let taxonomy: Vec<(&str, Vec<&str>)> = owned
        .iter()
        .map(|(d, g)| (d.as_str(), g.iter().map(String::as_str).collect()))
        .collect();
    let mut failures = manifest::lint(TOOLS, &taxonomy, &[]);
    let reg: Value = serde_json::from_str(&repo("data/codes.json")).unwrap();
    for t in TOOLS {
        failures.extend(
            t.warnings
                .iter()
                .filter(|w| reg["warnings"].get(**w).is_none())
                .map(|w| format!("{} unregistered {w}", t.id)),
        );
        for ex in t.examples {
            let r = call(t.id, ex.input);
            let s = r["summary"].as_str().unwrap_or_default();
            if r["ok"] != true || template::grade(s) > 8.0 || s.len() > template::MAX_CHARS {
                failures.push(format!(
                    "{} example (grade {:.1}): {r}",
                    t.id,
                    template::grade(s)
                ));
            }
        }
        let text = repo(&format!("core/vectors/{}.jsonl", t.id));
        failures.extend(vectors::lint(t.id, &text));
        failures.extend(vectors::run(&REGISTRY, t.id, &text));
        if vectors::count(&text) < 5 {
            failures.push(format!("{} has fewer than 5 vectors", t.id));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn gps_week_scenarios() {
    let r = call("time.scale.gps-week", r#"{"utc":"2026-09-18T00:00:00Z"}"#);
    assert_eq!(num(&r, "result.gps_week"), 2436.0);
    assert_eq!(num(&r, "result.seconds_of_week"), 432_018.0);
    assert_eq!(num(&r, "result.gps_minus_utc.value"), 18.0);
    assert_eq!(
        r["display"]["gps_week"], "2436",
        "identifiers are not digit-grouped"
    );
    assert_eq!(r["meta"]["assets"][0]["id"], "leap-seconds");
    let stale = call("time.scale.gps-week", r#"{"utc":"2027-07-01T00:00:00Z"}"#);
    assert!(codes(&stale).contains(&"LEAP_SECOND_TABLE_EXPIRED".to_owned()));
    let fresh = call("time.scale.gps-week", r#"{"utc":"2027-06-30T23:59:59Z"}"#);
    assert!(!codes(&fresh).contains(&"LEAP_SECOND_TABLE_EXPIRED".to_owned()));
}

#[test]
fn gps_round_trip_across_leap_seconds() {
    // Every second around the 2016-12-31 leap second maps one to one.
    let mut last = -1.0;
    for t in [
        "2016-12-31T23:59:58Z",
        "2016-12-31T23:59:59Z",
        "2016-12-31T23:59:60Z",
        "2017-01-01T00:00:00Z",
        "2017-01-01T00:00:01Z",
    ] {
        let g = call("time.scale.gps-week", &format!(r#"{{"utc":"{t}"}}"#));
        let sow = num(&g, "result.seconds_of_week");
        assert!(sow > last, "{t}");
        last = sow;
        let back = call(
            "time.scale.gps-to-utc",
            &format!(
                r#"{{"week":{},"seconds_of_week":{sow}}}"#,
                num(&g, "result.gps_week")
            ),
        );
        assert_eq!(back["result"]["utc"], t);
    }
    let bad = call("time.scale.gps-week", r#"{"utc":"2026-12-31T23:59:60Z"}"#);
    assert_eq!(bad["error"]["code"], "INVALID_INPUT");
}

#[test]
fn ten_bit_week_needs_era() {
    let r = call(
        "time.scale.gps-to-utc",
        r#"{"week":388,"seconds_of_week":0}"#,
    );
    assert_eq!(r["error"]["code"], "INVALID_INPUT");
    assert_eq!(r["error"]["field"], "/era");
    let ok = call(
        "time.scale.gps-to-utc",
        r#"{"week":388,"seconds_of_week":432018,"era":2}"#,
    );
    assert_eq!(ok["result"]["utc"], "2026-09-18T00:00:00Z");
    let both = call(
        "time.scale.gps-to-utc",
        r#"{"week":2436,"seconds_of_week":0,"era":2}"#,
    );
    assert_eq!(both["error"]["code"], "INVALID_INPUT");
}

#[test]
fn julian_date_scenario() {
    let r = call(
        "time.scale.julian-date",
        r#"{"utc":"2026-09-18T00:00:00Z"}"#,
    );
    assert_eq!(num(&r, "result.jd"), 2_461_301.5);
    assert_eq!(num(&r, "result.mjd"), 61_301.0);
    assert_eq!(num(&r, "result.day_of_year"), 261.0);
    assert_eq!(r["display"]["jd"], "2461301.5");
    let leap_day = call("time.scale.julian-date", r#"{"utc":"2024-12-31"}"#);
    assert_eq!(num(&leap_day, "result.day_of_year"), 366.0);
    let two = call("time.scale.julian-date", r#"{"utc":"2026-09-18","jd":1}"#);
    assert_eq!(two["error"]["code"], "INVALID_INPUT");
}

#[test]
fn decimal_hours_and_block_time() {
    let r = call("time.scale.decimal-hours", r#"{"time":"1.3"}"#);
    assert_eq!(r["result"]["duration"], "1 h 18 min");
    assert_eq!(r["result"]["hm"], "1:18");
    for bad in ["1:60", "-1", "abc", "1:5"] {
        let e = call(
            "time.scale.decimal-hours",
            &format!(r#"{{"time":"{bad}"}}"#),
        );
        assert_eq!(e["error"]["code"], "INVALID_INPUT", "{bad}");
    }
    let b = call(
        "time.scale.block-time",
        r#"{"out_time":"2215Z","in_time":"0140Z"}"#,
    );
    assert_eq!(b["result"]["hm"], "3:25");
    assert!(b["result"]["note"].is_string());
    let back = call(
        "time.scale.block-time",
        r#"{"out_time":"2026-09-19T01:40Z","in_time":"2026-09-18T22:15Z"}"#,
    );
    assert_eq!(back["error"]["code"], "INVALID_INPUT");
}

#[test]
fn zulu_scenarios() {
    let r = call(
        "time.scale.utc-offset",
        r#"{"time":"2026-07-01T14:05","offset":"-05:00"}"#,
    );
    assert_eq!(r["result"]["zulu"], "1905Z");
    assert_eq!(r["result"]["utc"], "2026-07-01T19:05:00Z");
    let evening = call(
        "time.scale.utc-offset",
        r#"{"time":"2026-07-01T20:30","offset":"UTC-6"}"#,
    );
    assert_eq!(evening["result"]["utc"], "2026-07-02T02:30:00Z");
    assert!(evening["summary"].as_str().unwrap().contains("next day"));
    let named = call(
        "time.scale.utc-offset",
        r#"{"time":"2026-07-01T14:05","offset":"America/Chicago"}"#,
    );
    assert_eq!(named["result"]["zulu"], "1905Z");
    assert_eq!(named["result"]["abbr"], "CDT");
    let clash = call(
        "time.scale.utc-offset",
        r#"{"time":"2026-07-01T14:05-04:00","offset":"-05:00"}"#,
    );
    assert_eq!(clash["error"]["code"], "INVALID_INPUT");
}

/// USNO rise, set, and civil twilight (aa.usno.navy.mil, retrieved
/// 2026-09-18), local clock minutes; the core agrees within 1 minute.
#[test]
fn sun_events_agree_with_usno() {
    let cases = [
        (
            (39.7392, -104.9903, "2026-06-21", "-06:00"),
            ["05:00", "05:32", "20:31", "21:04"],
        ),
        (
            (-33.8688, 151.2093, "2026-12-21", "+11:00"),
            ["05:11", "05:41", "20:05", "20:35"],
        ),
        (
            (51.5074, -0.1278, "2026-01-15", "+00:00"),
            ["07:21", "07:59", "16:21", "16:59"],
        ),
        (
            (40.4406, -79.9959, "2026-09-18", "-04:00"),
            ["06:36", "07:04", "19:24", "19:51"],
        ),
    ];
    let minutes = |s: &str| -> i64 {
        let hm = s.split(' ').nth(1).unwrap();
        hm[..2].parse::<i64>().unwrap() * 60 + hm[3..5].parse::<i64>().unwrap()
    };
    for ((lat, lon, date, off), usno) in cases {
        let r = call(
            "time.sun.events",
            &format!(r#"{{"lat":{lat},"lon":{lon},"date":"{date}","offset":"{off}"}}"#),
        );
        for (k, want) in ["civil_dawn", "sunrise", "sunset", "civil_dusk"]
            .iter()
            .zip(usno)
        {
            let got = minutes(r["result"][k].as_str().unwrap());
            let want = minutes(&format!("x {want}"));
            assert!((got - want).abs() <= 1, "{lat} {k}: {r}");
        }
    }
}

#[test]
fn polar_night_and_evening_after_zulu_midnight() {
    let r = call(
        "time.sun.events",
        r#"{"lat":71.29,"lon":-156.79,"date":"2026-12-21","offset":"-09:00"}"#,
    );
    assert_eq!(r["result"]["state"], "polar-night");
    assert!(
        r["result"]["civil_dawn"]
            .as_str()
            .unwrap()
            .contains("local"),
        "civil twilight still occurs"
    );
    let d = call(
        "time.sun.events",
        r#"{"lat":39.7392,"lon":-104.9903,"date":"2026-06-21","offset":"-06:00"}"#,
    );
    let dusk = d["result"]["civil_dusk"].as_str().unwrap();
    assert!(
        dusk.starts_with("2026-06-21 21:04 local") && dusk.contains("2026-06-22 0304Z"),
        "{dusk}"
    );
}

#[test]
fn four_nights_are_labeled_and_distinct() {
    let r = call(
        "time.sun.aviation-nights",
        r#"{"lat":39.7392,"lon":-104.9903,"date":"2026-06-21","offset":"-06:00","landing_time":"21:20"}"#,
    );
    let res = &r["result"];
    for k in [
        "logging_night",
        "passenger_currency",
        "position_lights",
        "part107_evening",
        "part107_morning",
    ] {
        assert!(res[k].as_str().unwrap().contains(" to "), "{k}");
    }
    assert_eq!(res["landing_logs_night"], "yes");
    assert_eq!(res["landing_counts_currency"], "no");
    let s = r["summary"].as_str().unwrap();
    assert!(s.contains("21:31") && s.contains("does not count"), "{s}");
    let ak = call(
        "time.sun.aviation-nights",
        r#"{"lat":61.2181,"lon":-149.9003,"date":"2026-06-21","offset":"-08:00","alaska":"yes"}"#,
    );
    // Near the solstice in Anchorage civil twilight lasts all night.
    assert!(
        ak["result"]["part107_evening"]
            .as_str()
            .unwrap()
            .starts_with("none"),
        "{ak}"
    );
}

#[test]
fn sun_position_extras() {
    let r = call(
        "time.sun.position",
        r#"{"lat":39.7392,"lon":-104.9903,"time":"2026-06-21T13:02-06:00","slope":"30 deg","aspect":"180 deg"}"#,
    );
    // Near solar noon the sun is due south at about 90 − 39.74 + 23.44.
    assert!((num(&r, "result.azimuth.value") - 180.0).abs() < 1.0, "{r}");
    assert!((num(&r, "result.elevation.value") - 73.7).abs() < 0.1);
    assert!((num(&r, "result.incidence.value") - (90.0 - 73.7 - 30.0_f64).abs()).abs() < 0.5);
    let night = call(
        "time.sun.position",
        r#"{"lat":39.7392,"lon":-104.9903,"time":"2026-06-21T08:00Z"}"#,
    );
    assert!(codes(&night).contains(&"SUN_BELOW_HORIZON".to_owned()));
    let half = call(
        "time.sun.position",
        r#"{"lat":0,"lon":0,"time":"2026-03-20T12:00Z","slope":"10 deg"}"#,
    );
    assert_eq!(half["error"]["field"], "/aspect");
}

#[test]
fn dst_gap_and_overlap() {
    let gap = call(
        "time.scale.utc-offset",
        r#"{"time":"2026-03-08T02:30","offset":"America/Denver"}"#,
    );
    assert_eq!(gap["error"]["code"], "INVALID_INPUT");
    assert!(
        gap["error"]["message"]
            .as_str()
            .unwrap()
            .contains("does not exist"),
        "{gap}"
    );
    let twice = call(
        "time.scale.utc-offset",
        r#"{"time":"2026-11-01T01:30","offset":"America/Denver"}"#,
    );
    assert_eq!(twice["result"]["utc"], "2026-11-01T07:30:00Z");
    assert!(codes(&twice).contains(&"AMBIGUOUS_INPUT".to_owned()));
    let unknown = call(
        "time.scale.utc-offset",
        r#"{"time":"2026-07-01T14:05","offset":"Mars/Olympus"}"#,
    );
    assert_eq!(unknown["error"]["code"], "INVALID_INPUT");
    // Sun events on the spring-forward date use each event's own offset.
    let e = call(
        "time.sun.events",
        r#"{"lat":39.7392,"lon":-104.9903,"date":"2026-03-08","offset":"America/Denver"}"#,
    );
    let rise = e["result"]["sunrise"].as_str().unwrap();
    let set = e["result"]["sunset"].as_str().unwrap();
    assert!(
        rise.starts_with("2026-03-08 07:") && set.starts_with("2026-03-08 19:"),
        "{rise} / {set}"
    );
}

/// The embedded leap-second table matches IANA's leap-seconds.list
/// (tzdata 2026d, which expires 28 June 2027).
#[test]
fn leap_table_matches_iana_list() {
    let text = repo("core/crates/gp-time/tests/data/leap-seconds.list");
    for line in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
    {
        let mut f = line.split_whitespace();
        let ntp: i64 = f.next().unwrap().parse().unwrap();
        let tai: i32 = f.next().unwrap().parse().unwrap();
        // NTP epoch 1900-01-01 is 2,208,988,800 s before Unix time.
        let day = (ntp - 2_208_988_800) / 86_400;
        assert_eq!(gp_time::civil::tai_minus_utc(day), Some(tai), "{line}");
        assert_eq!(
            gp_time::civil::tai_minus_utc(day - 1),
            (tai > 10).then_some(tai - 1),
            "{line}"
        );
    }
}

#[test]
fn sun_position_invariants() {
    // Zenith and elevation are complements, refraction only raises the sun,
    // azimuth stays in [0, 360), and at one instant the equation of time is
    // the same everywhere while declination moves only by parallax (twice 8.8″, under 0.005°).
    for time in [
        "2026-03-20T15:00Z",
        "2026-06-21T03:30Z",
        "2026-12-21T22:10Z",
    ] {
        let mut eot = None;
        let mut dec = None;
        for lat in (-80..=80).step_by(20) {
            for lon in (-180..180).step_by(40) {
                let r = call(
                    "time.sun.position",
                    &format!(r#"{{"lat":{lat},"lon":{lon},"time":"{time}"}}"#),
                );
                let el = num(&r, "result.elevation.value");
                assert!(
                    (num(&r, "result.zenith.value") + el - 90.0).abs() < 1e-9,
                    "{r}"
                );
                assert!(el >= num(&r, "result.elevation_true.value") - 1e-12, "{r}");
                let az = num(&r, "result.azimuth.value");
                assert!((0.0..360.0).contains(&az), "{r}");
                let e = num(&r, "result.equation_of_time.value");
                assert!((e - *eot.get_or_insert(e)).abs() < 1e-9, "{r}");
                let d = num(&r, "result.declination.value");
                assert!((d - *dec.get_or_insert(d)).abs() < 0.005, "{r}");
            }
        }
    }
}

/// UT minutes since 1970 from the first "(YYYY-MM-DD HHMMZ)" in `s`, by the
/// days-from-civil algorithm.
fn ut_minutes(s: &str) -> i64 {
    let z = s.split('(').nth(1).unwrap();
    let n = |a: usize, b: usize| z[a..b].parse::<i64>().unwrap();
    let (y, m, d) = (n(0, 4), n(5, 7), n(8, 10));
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
    let days = era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468;
    days * 1440 + n(11, 13) * 60 + n(13, 15)
}

/// Both ends of "A local (...) to B local (...)" in UT minutes.
fn ut_window(s: &str) -> (i64, i64) {
    let (a, b) = s.split_once(" to ").unwrap();
    (ut_minutes(a), ut_minutes(b))
}

#[test]
fn sun_events_invariants() {
    // Events come in order (astronomical, nautical, civil dawn, sunrise,
    // noon, sunset, and back), solar noon sits midway between sunrise and
    // sunset to within a minute, and in June the day lengthens with latitude.
    let order = [
        "astronomical_dawn", "nautical_dawn", "civil_dawn", "sunrise", "solar_noon", "sunset", "civil_dusk", "nautical_dusk", "astronomical_dusk",
    ];
    for (lat, lon, date, off) in [
        (39.7392, -104.9903, "2026-06-21", "-06:00"),
        (-33.8688, 151.2093, "2026-12-21", "+11:00"),
        (0.0, 0.0, "2026-03-20", "+00:00"),
        (51.5074, -0.1278, "2026-01-15", "+00:00"),
        (35.6762, 139.6503, "2026-09-30", "+09:00"),
    ] {
        let r = call(
            "time.sun.events",
            &serde_json::json!({"lat": lat, "lon": lon, "date": date, "offset": off}).to_string(),
        );
        let t: Vec<i64> = order.iter().map(|k| ut_minutes(r["result"][k].as_str().unwrap())).collect();
        assert!(t.windows(2).all(|w| w[0] < w[1]), "{lat}: out of order {:?}", order.iter().zip(&t).collect::<Vec<_>>());
        assert!(((t[3] + t[5]) - 2 * t[4]).abs() <= 2, "{lat}: noon is not midway");
    }
    let mut prev = 0.0;
    for lat in [-60.0, -30.0, 0.0, 30.0, 50.0, 60.0, 65.0] {
        let r = call(
            "time.sun.events",
            &serde_json::json!({"lat": lat, "lon": 0.0, "date": "2026-06-21", "offset": "+00:00"}).to_string(),
        );
        let d = num(&r, "result.day_minutes");
        assert!(d > prev, "June day length must grow with latitude ({lat}: {d})");
        prev = d;
    }
}

#[test]
fn aviation_nights_invariants() {
    // Logging night and passenger currency lie inside sunset-to-sunrise
    // (position lights), currency is exactly 1 hour in from each end, and
    // the Part 107 windows are the 30 minutes after sunset and before sunrise.
    for (lat, lon, date, off) in [
        (39.7392, -104.9903, "2026-06-21", "-06:00"),
        (40.4406, -79.9959, "2026-09-18", "-04:00"),
        (61.2181, -149.9003, "2026-03-10", "-09:00"),
        (21.3069, -157.8583, "2026-12-01", "-10:00"),
        (47.6062, -122.3321, "2026-01-05", "-08:00"),
    ] {
        let r = call(
            "time.sun.aviation-nights",
            &serde_json::json!({"lat": lat, "lon": lon, "date": date, "offset": off}).to_string(),
        );
        let w = |k: &str| ut_window(r["result"][k].as_str().unwrap());
        let (set, rise) = w("position_lights");
        let (dusk, dawn) = w("logging_night");
        let (c0, c1) = w("passenger_currency");
        assert!(set < dusk && dawn < rise, "{lat}: logging night outside sunset to sunrise");
        assert!((c0 - (set + 60)).abs() <= 1 && (c1 - (rise - 60)).abs() <= 1, "{lat}: currency is not 1 h in");
        assert_eq!(w("part107_evening").0, set);
        assert!((w("part107_evening").1 - (set + 30)).abs() <= 1);
        assert_eq!(w("part107_morning").1, rise);
        assert!((w("part107_morning").0 - (rise - 30)).abs() <= 1);
    }
}

#[test]
fn utc_offset_invariants() {
    // UTC to local and back is the identity for fixed offsets and named zones
    // (away from DST gaps), and the local time minus the UTC time is the offset.
    for zone in ["-05:00", "+05:45", "+13:45", "America/Denver", "Europe/London", "Asia/Kathmandu", "Australia/Lord_Howe", "America/St_Johns"] {
        for utc in ["2026-01-15T03:07Z", "2026-07-01T23:59Z", "2026-03-08T09:30Z", "2026-11-01T08:30Z", "2026-12-31T23:30Z"] {
            let to_local = call(
                "time.scale.utc-offset",
                &serde_json::json!({"time": utc, "offset": zone, "direction": "utc-to-local"}).to_string(),
            );
            let local = to_local["result"]["local"].as_str().unwrap();
            let back = call(
                "time.scale.utc-offset",
                &serde_json::json!({"time": &local[..16], "offset": zone, "direction": "local-to-utc"}).to_string(),
            );
            // An hour repeated at a fall-back resolves to its first occurrence, so skip those.
            let ambiguous = back["meta"]["warnings"].as_array().is_some_and(|w| w.iter().any(|x| x["code"] == "AMBIGUOUS_INPUT"));
            if !ambiguous {
                assert_eq!(back["result"]["utc"].as_str().unwrap()[..16], utc[..16], "{zone} {utc} -> {local}");
            }
            let sign = if local.as_bytes()[19] == b'-' { -1 } else { 1 };
            let off = sign * (local[20..22].parse::<i64>().unwrap() * 60 + local[23..25].parse::<i64>().unwrap());
            let mins = |s: &str| s[11..13].parse::<i64>().unwrap() * 60 + s[14..16].parse::<i64>().unwrap();
            let day = num(&to_local, "result.day_shift") as i64;
            assert_eq!(mins(local) - mins(utc) + 1440 * -day, off, "{zone} {utc} -> {local}");
        }
    }
}
