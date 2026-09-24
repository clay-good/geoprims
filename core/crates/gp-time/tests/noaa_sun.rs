//! Sunrise and sunset against the NOAA algorithm (astral's implementation,
//! tools/vectors/gen_noaa_sun.py): 1,000 random places below 60° latitude and
//! dates from 1950 to 2050, through the public tool. The spec asks for
//! agreement within 1 minute; the tool prints whole minutes, so each event
//! must lie within a minute of astral's time. astral evaluates NOAA's
//! equations for the UTC day, which misplaces an event near 00:00Z by the next
//! day's declination, so the fixture skips those; one is checked here against
//! NREL SPA instead.

use gp_time::REGISTRY;
use serde_json::{Value, json};

/// Minutes after local midnight of `date` for "YYYY-MM-DD HH:MM local (...)",
/// counting a later date as 1,440 minutes on.
fn local_minutes(s: &str, date: &str) -> Option<f64> {
    let (d, rest) = s.split_once(' ')?;
    let (h, m) = rest.get(..5)?.split_once(':')?;
    let day = if d == date {
        0.0
    } else if d > date {
        1440.0
    } else {
        -1440.0
    };
    Some(day + h.parse::<f64>().ok()? * 60.0 + m.parse::<f64>().ok()?)
}

#[test]
fn sunrise_and_sunset_match_noaa() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/noaa_sun.csv"
    ))
    .unwrap();
    let (mut n, mut worst) = (0, 0.0f64);
    let mut bad = Vec::new();
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let c: Vec<&str> = line.split(',').collect();
        let off: i32 = c[3].parse().unwrap();
        let offset = format!("{}{:02}:00", if off < 0 { '-' } else { '+' }, off.abs());
        let r: Value = serde_json::from_str(&REGISTRY.invoke(
            "time.sun.events",
            &json!({"lat": c[0].parse::<f64>().unwrap(), "lon": c[1].parse::<f64>().unwrap(), "date": c[2], "offset": offset}).to_string(),
        ))
        .unwrap();
        n += 1;
        for (k, i) in [("sunrise", 4), ("sunset", 5)] {
            let want: f64 = c[i].parse().unwrap();
            match r["result"][k].as_str().and_then(|s| local_minutes(s, c[2])) {
                Some(got) => {
                    worst = worst.max((got - want).abs());
                    if (got - want).abs() > 1.0 {
                        bad.push(format!("{line}: {k} {got} against {want}"));
                    }
                }
                None => bad.push(format!("{line}: no {k}: {}", r["result"])),
            }
        }
    }
    println!("{n} places, worst {worst:.3} min");
    assert_eq!(n, 1000);
    assert!(
        bad.is_empty(),
        "{} of 2,000 events off:\n{}",
        bad.len(),
        bad[..bad.len().min(8)].join("\n")
    );
}

/// At 54.8447° S, 99.2749° E on 2049-04-07 (UTC+7), sunrise falls at 00:00Z,
/// where astral puts it at 07:01 local. The sun's center, by NREL SPA through
/// time.sun.position, crosses -0.833° (refraction and the disc's radius)
/// between 23:58Z (-0.945°) and 23:59Z (-0.804°), at about 06:58:47 local;
/// the tool says 06:59.
#[test]
fn sunrise_at_a_utc_midnight_matches_spa() {
    let r: Value = serde_json::from_str(
        &REGISTRY.invoke(
            "time.sun.events",
            &json!({"lat": -54.8447, "lon": 99.2749, "date": "2049-04-07", "offset": "+07:00"})
                .to_string(),
        ),
    )
    .unwrap();
    assert!(
        r["result"]["sunrise"]
            .as_str()
            .unwrap()
            .starts_with("2049-04-07 06:59 local"),
        "{r}"
    );
    let elev = |t: &str| -> f64 {
        let p: Value = serde_json::from_str(&REGISTRY.invoke(
            "time.sun.position",
            &json!({"lat": -54.8447, "lon": 99.2749, "time": t}).to_string(),
        ))
        .unwrap();
        p["result"]["elevation_true"]["value"].as_f64().unwrap()
    };
    assert!(elev("2049-04-06T23:58:00Z") < -0.833 && elev("2049-04-06T23:59:00Z") > -0.833);
}
