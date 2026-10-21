//! Unix time and ISO week dates (add-celestial-and-moon, time/sidereal-and-
//! calendar).

use gp_time::REGISTRY;
use serde_json::{Value, json};

fn run(id: &str, input: Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).unwrap()
}

#[test]
fn unix_time_scenarios() {
    // The epoch.
    let e = run("time.scale.unix-time", json!({"unix": "0"}));
    assert_eq!(e["result"]["utc"], "1970-01-01T00:00:00Z", "{e}");
    // Milliseconds read from the digit count, and said so.
    let ms = run("time.scale.unix-time", json!({"unix": "1760832000000"}));
    assert_eq!(ms["result"]["utc"], "2025-10-19T00:00:00Z");
    let w = ms["meta"]["warnings"].as_array().unwrap();
    assert!(
        w.iter().any(|x| x["code"] == "INPUT_NORMALIZED"
            && x["message"].as_str().unwrap().contains("milliseconds")),
        "{ms}"
    );
    // An explicit unit wins, without the warning.
    let s = run(
        "time.scale.unix-time",
        json!({"unix": "1760832000000", "unit": "s"}),
    );
    assert_eq!(
        s["ok"], false,
        "a 13-digit count of seconds is past year 9999: {s}"
    );
    let us = run(
        "time.scale.unix-time",
        json!({"unix": "1760832000000000", "unit": "us"}),
    );
    assert_eq!(us["result"]["utc"], "2025-10-19T00:00:00Z");
    // A leap second has no Unix time.
    let leap = run(
        "time.scale.unix-time",
        json!({"utc": "2016-12-31T23:59:60Z"}),
    );
    assert_eq!(leap["error"]["code"], "INVALID_INPUT");
    assert!(
        leap["error"]["hint"]
            .as_str()
            .unwrap()
            .contains("TAI or GPS")
    );
    // The seconds on either side of it are one Unix second apart.
    let a = run(
        "time.scale.unix-time",
        json!({"utc": "2016-12-31T23:59:59Z"}),
    );
    let b = run(
        "time.scale.unix-time",
        json!({"utc": "2017-01-01T00:00:00Z"}),
    );
    assert_eq!(
        b["result"]["unix"].as_f64().unwrap() - a["result"]["unix"].as_f64().unwrap(),
        1.0
    );
    // GPS: the GPS epoch is week 0, second 0; offsets and before 1980 have none.
    let g = run(
        "time.scale.unix-time",
        json!({"utc": "1980-01-06T00:00:00Z"}),
    );
    assert_eq!(g["result"]["gps_week"], 0.0);
    assert_eq!(g["result"]["gps_seconds"], 0.0);
    assert!(
        run("time.scale.unix-time", json!({"unix": "0"}))["result"]
            .get("gps_week")
            .is_none()
    );
    let off = run(
        "time.scale.unix-time",
        json!({"utc": "2026-10-19T01:00:00+01:00"}),
    );
    assert_eq!(off["result"]["unix"], 1_792_368_000.0);
}

#[test]
fn unix_time_round_trips_1970_to_2100() {
    // Every 97,003 seconds or so across 130 years, out and back.
    let mut t: i64 = 0;
    while t < 4_102_444_800 {
        let r = run("time.scale.unix-time", json!({"unix": t.to_string()}));
        let utc = r["result"]["utc"].as_str().unwrap().to_owned();
        let back = run("time.scale.unix-time", json!({"utc": utc}));
        assert_eq!(
            back["result"]["unix"].as_f64().unwrap(),
            t as f64,
            "{t} -> {utc}"
        );
        t += 97_003_177;
    }
}

#[test]
fn iso_week_scenarios_and_rule() {
    let wk = |d: &str| run("time.scale.iso-week", json!({"date": d}))["result"].clone();
    // The spec's anchors.
    assert_eq!(wk("2027-01-01")["week_date"], "2026-W53-5");
    assert_eq!(wk("2026-01-01")["week_date"], "2026-W01-4");
    // Monday and Sunday bracket the week.
    let r = wk("2026-10-21");
    assert_eq!(r["week_date"], "2026-W43-3");
    assert_eq!(r["monday"], "2026-10-19");
    assert_eq!(r["sunday"], "2026-10-25");
    // The reverse: a week date to its calendar date, and a week alone to its Monday.
    let back = run("time.scale.iso-week", json!({"week_date": "2026-W53-5"}));
    assert_eq!(back["result"]["date"], "2027-01-01");
    let monday = run("time.scale.iso-week", json!({"week_date": "2026-W01"}));
    assert_eq!(monday["result"]["date"], "2025-12-29");
    // Week 53 exists only in a 53-week year.
    assert_eq!(
        run("time.scale.iso-week", json!({"week_date": "2025-W53-1"}))["ok"],
        false
    );
    // The rule, year by year: 53 weeks when 1 January is a Thursday, or a
    // Wednesday in a leap year; 4 January is always in week 1.
    for y in 1970..2101 {
        let jan4 = wk(&format!("{y}-01-04"));
        assert_eq!(jan4["week"], 1.0, "{y}");
        assert_eq!(jan4["week_year"], f64::from(y));
        let dec28 = wk(&format!("{y}-12-28"));
        assert_eq!(
            dec28["week"], dec28["weeks_in_year"],
            "{y}: 28 December is in the last week"
        );
    }
}
