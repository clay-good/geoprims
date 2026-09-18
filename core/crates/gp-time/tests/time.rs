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
    assert_eq!(named["error"]["code"], "UNSUPPORTED");
    let clash = call(
        "time.scale.utc-offset",
        r#"{"time":"2026-07-01T14:05-04:00","offset":"-05:00"}"#,
    );
    assert_eq!(clash["error"]["code"], "INVALID_INPUT");
}
