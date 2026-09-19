//! Time-zone parity with Python zoneinfo on the same IANA release: offset and
//! abbreviation for every zone at random instants from 1970 to 2100. The
//! committed fixture has 4 instants per zone; the full run (400 per zone) is
//! `TZ_DIFF=/path/full.csv cargo test -- --ignored` with a file from
//! tools/vectors/gen_tz_diff.py.

fn check(text: &str) -> (usize, Vec<String>) {
    let mut bad = Vec::new();
    let mut n = 0;
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = line.split(',').collect();
        let Some(z) = gp_time::tz::zone(f[0]) else {
            bad.push(format!("{} missing", f[0]));
            continue;
        };
        let t: i64 = f[1].parse().unwrap();
        let lt = z.at(t);
        if lt.utoff.to_string() != f[2] || lt.abbr != f[3] {
            bad.push(format!("{line} -> {} {}", lt.utoff, lt.abbr));
        }
        n += 1;
    }
    (n, bad)
}

#[test]
fn committed_fixture() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/tz_diff.csv"
    ))
    .unwrap();
    let (n, bad) = check(&text);
    assert!(n > 2000);
    assert!(
        bad.is_empty(),
        "{} mismatches:\n{}",
        bad.len(),
        bad[..bad.len().min(30)].join("\n")
    );
}

#[test]
#[ignore = "needs a file from tools/vectors/gen_tz_diff.py in TZ_DIFF"]
fn full_differential() {
    let (n, bad) =
        check(&std::fs::read_to_string(std::env::var("TZ_DIFF").expect("set TZ_DIFF")).unwrap());
    println!("{n} comparisons, {} mismatches", bad.len());
    assert!(
        bad.is_empty(),
        "{} mismatches:\n{}",
        bad.len(),
        bad[..bad.len().min(30)].join("\n")
    );
}

/// "YYYY-MM-DDTHH:MMZ" for Unix seconds, floored to the minute (transitions
/// fall on whole minutes, so flooring never crosses one).
fn utc_minute(t: i64) -> String {
    let (days, secs) = (t.div_euclid(86_400), t.rem_euclid(86_400));
    // Civil from days (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}Z", secs / 3600, secs % 3600 / 60)
}

#[test]
fn zulu_tool_matches_zoneinfo() {
    // The public Zulu tool (utc-to-local) on every committed instant: the
    // local offset and abbreviation must be zoneinfo's.
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/tz_diff.csv")).unwrap();
    let mut bad = Vec::new();
    let mut n = 0;
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = line.split(',').collect();
        let input = serde_json::json!({"time": utc_minute(f[1].parse().unwrap()), "offset": f[0], "direction": "utc-to-local"});
        let r: serde_json::Value = serde_json::from_str(&gp_time::REGISTRY.invoke("time.scale.utc-offset", &input.to_string())).unwrap();
        let local = r["result"]["local"].as_str().unwrap_or("");
        let off = &local[local.len().saturating_sub(6)..];
        let secs = off
            .get(1..3)
            .zip(off.get(4..6))
            .and_then(|(h, m)| Some(h.parse::<i64>().ok()? * 3600 + m.parse::<i64>().ok()? * 60))
            .map(|s| if off.starts_with('-') { -s } else { s });
        // GMT, GMT+0, GMT-0, and UTC read as the fixed offset +00:00, which has no abbreviation.
        let fixed_zero = r["result"]["abbr"].is_null() && f[2] == "0" && f[0].starts_with(['G', 'U']);
        if secs != f[2].parse().ok() || (r["result"]["abbr"] != f[3] && !fixed_zero) {
            bad.push(format!("{line} -> {local} {}", r["result"]["abbr"]));
        }
        n += 1;
    }
    assert_eq!(n, 2388);
    assert!(bad.is_empty(), "{} mismatches:\n{}", bad.len(), bad[..bad.len().min(10)].join("\n"));
}
