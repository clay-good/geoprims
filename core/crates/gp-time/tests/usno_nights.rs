//! The four aviation nights against USNO (tools/vectors/gen_usno_sun.py
//! --nights): 100 places, mostly in the US and Alaska, with USNO's sunset
//! and end of civil twilight on the date and sunrise and beginning of civil
//! twilight the next morning. Each window must follow its regulation from
//! those times, to within a minute:
//!   position lights (§91.209)       sunset to sunrise
//!   logging night (§1.1)            end to beginning of civil twilight
//!   passenger currency (§61.57(b))  sunset + 1 h to sunrise − 1 h
//!   Part 107 (§107.29(c))           30 minutes after sunset and before sunrise

use gp_time::REGISTRY;
use serde_json::{Value, json};

fn call(id: &str, input: &Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).expect("envelope is JSON")
}

fn hm(s: &str) -> Option<i64> {
    let (h, m) = s.split_once(':')?;
    Some(h.parse::<i64>().ok()? * 60 + m.parse::<i64>().ok()?)
}

/// Local clock minutes of both ends of "A local (...) to B local (...)".
fn window(s: &str) -> Option<(i64, i64)> {
    let (a, b) = s.split_once(" to ")?;
    let clock = |x: &str| hm(x.split(' ').nth(1)?);
    Some((clock(a)?, clock(b)?))
}

#[test]
fn nights_follow_usno() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/usno_nights.csv"
    ))
    .unwrap();
    let (mut checked, mut bad) = (0, Vec::new());
    let near = |a: i64, b: i64| (a - b).rem_euclid(1440).min((b - a).rem_euclid(1440)) <= 1;
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let c: Vec<&str> = line.split(',').collect();
        let tz: i32 = c[3].parse().unwrap();
        let offset = format!("{}{:02}:00", if tz < 0 { '-' } else { '+' }, tz.abs());
        let r = call(
            "time.sun.aviation-nights",
            &json!({"lat": c[0].parse::<f64>().unwrap(), "lon": c[1].parse::<f64>().unwrap(), "date": c[2], "offset": offset}),
        );
        assert_eq!(r["ok"], true, "{line}: {r}");
        let (set, dusk, rise, dawn) = (hm(c[4]), hm(c[5]), hm(c[6]), hm(c[7]));
        let mut expect = |key: &str, want: Option<(i64, i64)>| {
            let Some((a, b)) = want else { return };
            match window(r["result"][key].as_str().unwrap_or("")) {
                Some((x, y)) if near(x, a) && near(y, b) => checked += 1,
                got => bad.push(format!(
                    "{line}: {key} {got:?} vs USNO ({a}, {b}) from {}",
                    r["result"][key]
                )),
            }
        };
        expect("position_lights", set.zip(rise));
        expect("logging_night", dusk.zip(dawn));
        expect(
            "passenger_currency",
            set.zip(rise).map(|(s, r)| (s + 60, r - 60)),
        );
        expect("part107_evening", set.map(|s| (s, s + 30)));
        expect("part107_morning", rise.map(|r| (r - 30, r)));
    }
    assert!(
        bad.is_empty(),
        "{} disagreements:\n{}",
        bad.len(),
        bad.join("\n")
    );
    assert!(checked >= 450, "only {checked} windows checked");
    println!("{checked} windows within a minute of USNO");
}

/// add-practitioner-essentials "Part 107 lighting window": the drone view
/// states the anti-collision lighting rule with both paragraphs cited.
#[test]
fn part_107_lighting_is_stated() {
    let r = call(
        "time.sun.aviation-nights",
        &json!({"lat": 39.7392, "lon": -104.9903, "date": "2026-09-22", "offset": "-06:00"}),
    );
    let text = r["result"]["part107_lighting"].as_str().expect("stated");
    assert!(text.contains("3 statute miles"), "{text}");
    assert!(
        text.contains("§107.29(b)") && text.contains("§107.29(a)(2)"),
        "{text}"
    );
    // The evening period starts at sunset, where the position-lights window starts, and lasts 30 minutes.
    let (a, b) = window(r["result"]["part107_evening"].as_str().unwrap()).unwrap();
    let (s, _) = window(r["result"]["position_lights"].as_str().unwrap()).unwrap();
    assert_eq!((a, b - a), (s, 30));
}
