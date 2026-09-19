//! Sunrise, sunset, and civil twilight against the USNO Astronomical
//! Applications API (tools/vectors/gen_usno_sun.py): 300 seeded places and
//! dates, through the public tool. USNO prints local times to the minute;
//! ours are the NOAA equations rounded the same way.

use gp_time::REGISTRY;
use serde_json::{Value, json};

fn call(id: &str, input: &Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).expect("envelope is JSON")
}

/// Minutes after local midnight of `date` for "YYYY-MM-DD HH:MM local (...)".
fn local_minutes(s: &str, date: &str) -> Option<i64> {
    let (d, rest) = s.split_once(' ')?;
    let hm = rest.get(..5)?;
    if !rest[5..].starts_with(" local") || d != date {
        return None;
    }
    let (h, m) = hm.split_once(':')?;
    Some(h.parse::<i64>().ok()? * 60 + m.parse::<i64>().ok()?)
}

#[test]
fn sun_events_match_usno() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/usno_sun.csv")).unwrap();
    let (mut n, mut worst, mut exact) = (0, 0i64, 0);
    let mut bad = Vec::new();
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let c: Vec<&str> = line.split(',').collect();
        let tz: i32 = c[3].parse().unwrap();
        let offset = format!("{}{:02}:00", if tz < 0 { '-' } else { '+' }, tz.abs());
        let r = call(
            "time.sun.events",
            &json!({"lat": c[0].parse::<f64>().unwrap(), "lon": c[1].parse::<f64>().unwrap(), "date": c[2], "offset": offset}),
        );
        assert_eq!(r["ok"], true, "{line}: {r}");
        for (key, usno) in [("sunrise", c[4]), ("sunset", c[5]), ("civil_dawn", c[6]), ("civil_dusk", c[7])] {
            let ours = r["result"][key].as_str().unwrap_or("");
            let got = local_minutes(ours, c[2]);
            match (usno, got) {
                ("-", None) => {}
                ("-", Some(_)) => bad.push(format!("{line}: {key} is {ours}, USNO lists none")),
                (hm, Some(m)) => {
                    let (h, mm) = hm.split_once(':').unwrap();
                    let want = h.parse::<i64>().unwrap() * 60 + mm.parse::<i64>().unwrap();
                    worst = worst.max((m - want).abs());
                    exact += i32::from(m == want);
                    if (m - want).abs() > 1 {
                        bad.push(format!("{line}: {key} {ours} vs USNO {hm}"));
                    }
                }
                (hm, None) => bad.push(format!("{line}: {key} is {ours}, USNO {hm}")),
            }
            n += 1;
        }
    }
    assert_eq!(n, 1200);
    assert!(bad.is_empty(), "{} disagreements (worst {worst} min):\n{}", bad.len(), bad.join("\n"));
    assert!(worst <= 1);
    println!("{n} events, {exact} to the same minute as USNO, worst {worst} min");
}
