//! Solar-position parity with pvlib's NREL SPA (tools/vectors/gen_spa_diff.py):
//! places uniform over the sphere at instants uniform over 1990-2060, through
//! the public tool. The committed fixture has 250; the full run is
//! `SPA_DIFF=/path/full.csv cargo test -- --ignored` with a larger file.
//!
//! This checks the implementation, not the model. Both sides implement the
//! same published algorithm, so their agreeing says nothing about whether SPA
//! is right -- that is what tests/usno_sun.rs is for, against USNO's own
//! published times. What it does check is the part a transcription error lives
//! in: the periodic terms, the nutation series, the refraction correction.

use serde_json::Value;

/// Elevation is compared directly. Azimuth is not: near the zenith or the
/// nadir a whole degree of azimuth is a vanishing angle on the sky, so the
/// comparison is the separation the two directions actually describe,
/// d_azimuth x cos(elevation), which is the error a camera or a shadow would
/// see. Over 2,000 points the worst separation was 1.95 arcseconds.
const LIMIT_ARCSEC: f64 = 5.0;

fn check(text: &str) -> (usize, Vec<String>) {
    let mut bad = Vec::new();
    let mut n = 0;
    for line in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
    {
        let f: Vec<f64> = line
            .split(',')
            .map(|x| x.parse().expect("number"))
            .collect();
        let (lat, lon, t, el_true, el_app, az) = (f[0], f[1], f[2] as i64, f[3], f[4], f[5]);
        let input = format!(r#"{{"lat":{lat},"lon":{lon},"time":"{}"}}"#, utc_second(t));
        let r: Value = serde_json::from_str(&gp_time::REGISTRY.invoke("time.sun.position", &input))
            .expect("envelope is JSON");
        if r["ok"] != true {
            bad.push(format!("{line} -> {r}"));
            continue;
        }
        let got = |k: &str| r["result"][k]["value"].as_f64().expect(k);
        let d_true = (got("elevation_true") - el_true).abs() * 3600.0;
        let d_app = (got("elevation") - el_app).abs() * 3600.0;
        let mut d_az = (got("azimuth") - az).abs();
        if d_az > 180.0 {
            d_az = 360.0 - d_az;
        }
        let sep = d_az * el_true.to_radians().cos().abs() * 3600.0;
        if d_true > LIMIT_ARCSEC || d_app > LIMIT_ARCSEC || sep > LIMIT_ARCSEC {
            bad.push(format!(
                "{line} -> true {d_true:.2}\" apparent {d_app:.2}\" separation {sep:.2}\""
            ));
        }
        n += 1;
    }
    (n, bad)
}

#[test]
fn committed_fixture() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/spa_diff.csv"
    ))
    .unwrap();
    let (n, bad) = check(&text);
    assert!(n >= 250, "{n} comparisons");
    assert!(
        bad.is_empty(),
        "{} beyond {LIMIT_ARCSEC}\":\n{}",
        bad.len(),
        bad[..bad.len().min(20)].join("\n")
    );
}

#[test]
#[ignore = "needs a file from tools/vectors/gen_spa_diff.py in SPA_DIFF"]
fn full_differential() {
    let (n, bad) =
        check(&std::fs::read_to_string(std::env::var("SPA_DIFF").expect("set SPA_DIFF")).unwrap());
    println!("{n} comparisons, {} beyond {LIMIT_ARCSEC}\"", bad.len());
    assert!(
        bad.is_empty(),
        "{} beyond {LIMIT_ARCSEC}\":\n{}",
        bad.len(),
        bad[..bad.len().min(20)].join("\n")
    );
}

/// "YYYY-MM-DDTHH:MM:SSZ" for Unix seconds (Howard Hinnant's civil-from-days).
fn utc_second(t: i64) -> String {
    let (days, secs) = (t.div_euclid(86_400), t.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        secs / 3600,
        secs % 3600 / 60,
        secs % 60
    )
}
