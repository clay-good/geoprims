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
