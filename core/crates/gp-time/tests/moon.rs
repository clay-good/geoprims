//! The moon (add-celestial-and-moon, time/moon), against USNO.

use gp_time::REGISTRY;
use serde_json::{Value, json};

fn envelope(id: &str, input: Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).unwrap()
}

fn call(id: &str, input: Value) -> Value {
    let r = envelope(id, input.clone());
    assert_eq!(r["ok"], true, "{input}: {r}");
    r["result"].clone()
}

fn num(v: &Value, key: &str) -> f64 {
    v[key]["value"]
        .as_f64()
        .unwrap_or_else(|| panic!("{key} missing in {v}"))
}

const USNO: &str = include_str!("data/usno_moon.json");

/// The Zulu `HHMM` inside `2026-06-21 12:59 local (2026-06-21 1859Z)`, as
/// minutes of the day, or None when the tool reported no event.
fn zulu_minutes(s: &str) -> Option<i64> {
    let open = s.find('(')? + 1;
    let z = s[open..].split('Z').next()?;
    let hhmm = z.rsplit(' ').next()?;
    let (h, m) = (
        hhmm[0..2].parse::<i64>().ok()?,
        hhmm[2..4].parse::<i64>().ok()?,
    );
    Some(h * 60 + m)
}

/// Minutes between two times of day, the shorter way round midnight.
fn apart(a: i64, b: i64) -> i64 {
    let d = (a - b).rem_euclid(1440);
    d.min(1440 - d)
}

fn hm(s: &str) -> i64 {
    let (h, m) = s.split_once(':').expect("HH:MM");
    h.parse::<i64>().unwrap() * 60 + m.parse::<i64>().unwrap()
}

/// USNO's own rise, set, and transit for 500 places and UTC dates from 1995
/// to 2045, including polar latitudes and the dates the moon's retardation
/// skips an event. Every time within a minute, and every continuously-up or
/// continuously-down day reported as such.
#[test]
fn rise_set_matches_usno() {
    let doc: Value = serde_json::from_str(USNO).unwrap();
    let rows = doc["rstt"].as_array().unwrap();
    assert!(rows.len() >= 500, "only {} rows", rows.len());
    let mut worst = (0_i64, String::new());
    let mut checked = 0;
    let mut polar = 0;
    let mut skipped_event = 0;
    let mut boundary = 0;
    for row in rows {
        let (lat, lon) = (row["lat"].as_f64().unwrap(), row["lon"].as_f64().unwrap());
        let date = row["date"].as_str().unwrap();
        let r = call(
            "time.moon.rise-set",
            json!({"lat": lat, "lon": lon, "date": date, "offset": "+00:00"}),
        );
        let want = |phen: &str| -> Option<i64> {
            row["events"]
                .as_array()
                .unwrap()
                .iter()
                .find(|e| e["phen"] == phen)
                .and_then(|e| e["time"].as_str())
                .map(hm)
        };
        let always_up = row["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["phen"] == "Object continuously above the Horizon");
        let always_down = row["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["phen"] == "Object continuously below the Horizon");
        let state = r["state"].as_str().unwrap();
        let at = format!("{date} at {lat}, {lon}");
        if always_up || always_down {
            polar += 1;
            assert_eq!(
                state,
                if always_up {
                    "always-up"
                } else {
                    "always-down"
                },
                "{at}: USNO says the moon is continuously {}, the tool says {state}",
                if always_up { "up" } else { "down" }
            );
            continue;
        }
        for (phen, key) in [
            ("Rise", "moonrise"),
            ("Set", "moonset"),
            ("Upper Transit", "transit"),
        ] {
            let got = zulu_minutes(r[key].as_str().unwrap());
            match (want(phen), got) {
                (Some(w), Some(g)) => {
                    let d = apart(g, w);
                    if d > worst.0 {
                        worst = (d, format!("{at} {key}: USNO {w}, tool {g}"));
                    }
                    assert!(d <= 1, "{at} {key}: USNO {w} min, tool {g} min");
                    checked += 1;
                }
                (None, None) => {
                    if key != "transit" {
                        skipped_event += 1;
                        assert!(
                            state == "no-moonrise" || state == "no-moonset",
                            "{at}: no {key} either side, but the state is {state}"
                        );
                    }
                }
                // An event within a minute of midnight falls on either side
                // of the day boundary, and USNO dates it to the other day.
                (None, Some(g)) if g <= 1 || g >= 1439 => boundary += 1,
                (w, g) => panic!("{at} {key}: USNO {w:?}, tool {g:?} ({state})"),
            }
        }
    }
    println!(
        "moon rise/set vs USNO: {checked} times checked, worst {} min ({}); {polar} polar days, {skipped_event} skipped events, {boundary} at a day boundary",
        worst.0, worst.1
    );
    assert!(checked >= 1_300, "only {checked} times compared");
    assert!(polar >= 20, "only {polar} polar days in the sample");
    assert!(skipped_event >= 20, "only {skipped_event} skipped events");
}

/// USNO's own instants for the four phases, 208 of them from 1995 to 2045:
/// asked a day earlier, the tool's next instant of that phase is within two
/// minutes.
#[test]
fn phases_match_usno() {
    let doc: Value = serde_json::from_str(USNO).unwrap();
    let phases = doc["phases"].as_array().unwrap();
    assert!(phases.len() >= 200, "only {} phases", phases.len());
    let mut worst = (0.0_f64, String::new());
    for p in phases {
        let utc = p["utc"].as_str().unwrap();
        let key = match p["phase"].as_str().unwrap() {
            "New Moon" => "next_new_moon",
            "First Quarter" => "next_first_quarter",
            "Full Moon" => "next_full_moon",
            "Last Quarter" => "next_last_quarter",
            other => panic!("unknown phase {other}"),
        };
        // A day before the instant, so the next one of that phase is this one.
        let day = &utc[0..10];
        let asked = format!("{day}T00:00:00Z");
        let r = call("time.moon.phase", json!({"time": asked}));
        let got = r[key].as_str().unwrap();
        let mins = |s: &str| {
            let (d, t) = s.split_once('T').unwrap();
            let t = t.trim_end_matches('Z');
            let (y, rest) = d.split_once('-').unwrap();
            let (mo, da) = rest.split_once('-').unwrap();
            let (h, mi) = t.split_once(':').unwrap();
            // Days from a fixed epoch are enough for a difference.
            let (y, mo, da) = (
                y.parse::<i64>().unwrap(),
                mo.parse::<i64>().unwrap(),
                da.parse::<i64>().unwrap(),
            );
            let a = (y * 12 + mo - 3) / 12;
            let days = 365 * y + y / 4 - y / 100 + y / 400 + (153 * ((mo + 9) % 12) + 2) / 5 + da
                - 719_468
                - (a - a);
            days * 1440 + h.parse::<i64>().unwrap() * 60 + mi.parse::<i64>().unwrap()
        };
        let d = (mins(got) - mins(utc)).abs() as f64;
        if d > worst.0 {
            worst = (d, format!("{} {key}: USNO {utc}, tool {got}", p["phase"]));
        }
        assert!(d <= 2.0, "{key}: USNO {utc}, tool {got}");
    }
    println!(
        "moon phases vs USNO: {} instants, worst {} min ({})",
        phases.len(),
        worst.0,
        worst.1
    );
}

/// At a full moon the disc is all but entirely lit, and the phase the tool
/// names is the one USNO names.
#[test]
fn a_full_moon_is_full() {
    let doc: Value = serde_json::from_str(USNO).unwrap();
    for p in doc["phases"].as_array().unwrap() {
        if p["phase"] != "Full Moon" {
            continue;
        }
        let r = call("time.moon.phase", json!({"time": p["utc"]}));
        let k = r["illuminated"].as_f64().unwrap();
        assert!(k > 0.99, "{}: illuminated {k}", p["utc"]);
        assert_eq!(r["phase_name"], "full-moon", "{}", p["utc"]);
    }
}

/// The moon's geocentric place through these tools is the same place the
/// celestial almanac publishes, and the topocentric altitude matches USNO's
/// own geocentric altitude less its own parallax in altitude.
#[test]
fn position_matches_usno() {
    let doc: Value = serde_json::from_str(USNO).unwrap();
    let rows = doc["celnav"].as_array().unwrap();
    let mut worst = (0.0_f64, 0.0_f64, String::new());
    let mut worst_az = (0.0_f64, String::new());
    let mut n = 0;
    for row in rows {
        let utc = format!("{}Z", row["ut1"].as_str().unwrap().replace(' ', "T"));
        let (lat, lon) = (row["lat"].as_f64().unwrap(), row["lon"].as_f64().unwrap());
        let r = call(
            "time.moon.position",
            json!({"lat": lat, "lon": lon, "time": utc}),
        );
        // USNO's hc is the geocentric altitude; its pa is the parallax in
        // altitude that carries the topocentric altitude up to it.
        let topo = num(&r, "altitude_geometric");
        let want_topo = row["hc"].as_f64().unwrap() - row["pa"].as_f64().unwrap();
        let dalt = ((topo - want_topo) * 60.0).abs();
        let ddec = ((num(&r, "dec") - row["dec"].as_f64().unwrap()) * 60.0).abs();
        // USNO's azimuth is geocentric. Parallax moves the moon along the
        // vertical circle, so on a sphere it would not move the azimuth at
        // all; the ellipsoid tilts that circle a little, which is what this
        // difference measures.
        let daz = {
            let d = num(&r, "azimuth") - row["zn"].as_f64().unwrap();
            ((d + 180.0).rem_euclid(360.0) - 180.0).abs() * 60.0
        };
        if dalt > worst.0 {
            worst = (dalt, ddec, format!("{utc} at {lat}, {lon}"));
        }
        if daz > worst_az.0 && topo < 85.0 {
            worst_az = (daz, format!("{utc} at {lat}, {lon}"));
        }
        // Near the zenith a hundredth of a degree of altitude swings the
        // azimuth degrees, so it is not compared there, as the sight
        // reduction tests do not compare it above 85 degrees either.
        // This difference is the ellipsoid, not an error: parallax moves the
        // moon along the vertical circle through the geodetic zenith, while
        // USNO's azimuth is that of its geocentric place, and the two verticals
        // differ by up to 11.5'. The gap grows as 1/cos(altitude), so it is
        // measured below 85 degrees and left alone above, where a hundredth of
        // a degree of altitude swings the azimuth degrees.
        if topo < 85.0 {
            assert!(
                daz < 0.4,
                "{utc}: azimuth {daz:.3}' from USNO's geocentric one at {topo:.2} degrees"
            );
        }
        assert!(
            dalt < 0.25,
            "{utc}: topocentric altitude {topo} is {dalt:.3}' from USNO's {want_topo}"
        );
        assert!(ddec < 0.25, "{utc}: declination off by {ddec:.3}'");
        n += 1;
    }
    println!(
        "moon position vs USNO: {n} instants, worst altitude {:.3}', declination {:.3}' ({}); worst azimuth against its geocentric one {:.3}' ({})",
        worst.0, worst.1, worst.2, worst_az.0, worst_az.1
    );
    assert!(n >= 100, "only {n} moon rows");
}

/// A date that skips a moonset names the set either side of it, and the
/// retardation is the reason the result gives.
#[test]
fn a_skipped_moonset_says_when_the_neighbours_are() {
    let doc: Value = serde_json::from_str(USNO).unwrap();
    let mut seen = 0;
    for row in doc["rstt"].as_array().unwrap() {
        let events = row["events"].as_array().unwrap();
        let has = |p: &str| events.iter().any(|e| e["phen"] == p);
        if !(has("Rise") && !has("Set") && events.len() > 1) {
            continue;
        }
        let r = call(
            "time.moon.rise-set",
            json!({"lat": row["lat"], "lon": row["lon"], "date": row["date"], "offset": "+00:00"}),
        );
        assert_eq!(r["state"], "no-moonset", "{}", row["date"]);
        assert_eq!(r["moonset"], "none");
        let note = r["note"].as_str().unwrap();
        assert!(
            note.contains("no moonset") && note.contains("retardation"),
            "{}: note is {note:?}",
            row["date"]
        );
        assert!(
            note.contains("local ("),
            "{}: no times in {note:?}",
            row["date"]
        );
        seen += 1;
    }
    assert!(seen >= 10, "only {seen} skipped-moonset days in the sample");
    println!("skipped moonsets checked: {seen}");
}
