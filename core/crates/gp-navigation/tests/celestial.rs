//! The celestial almanac (add-celestial-and-moon, navigation/celestial).

use gp_navigation::REGISTRY;
use serde_json::{Value, json};

fn run(input: Value) -> Value {
    let r: Value =
        serde_json::from_str(&REGISTRY.invoke("navigation.celestial.almanac", &input.to_string()))
            .unwrap();
    assert_eq!(r["ok"], true, "{input}: {r}");
    r["result"].clone()
}

fn dm(d: f64, m: f64) -> f64 {
    d.signum() * (d.abs() + m / 60.0)
}

/// Difference in arcminutes, wrapping at 360°.
fn off(got: f64, want: f64) -> f64 {
    ((got - want + 180.0).rem_euclid(360.0) - 180.0).abs() * 60.0
}

fn star<'a>(r: &'a Value, name: &str) -> &'a Value {
    r["stars"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == name)
        .unwrap()
}

#[test]
fn bowditch_four_star_fix() {
    // Bowditch 2024 §2010, Tables 2010a and 2010b (2024-06-02): GHA Aries
    // 11°17.0′ at 08h, and each star's SHA and declination. Table 2010b
    // repeats Procyon's GHA as its SHA; Table 2010a's 244°51.6′ is the SHA.
    let r = run(json!({"utc": "2024-06-02T08:00:00Z", "dut1": "0 s"}));
    assert!(off(r["aries_gha"]["value"].as_f64().unwrap(), dm(11.0, 17.0)) <= 0.1);
    for (name, sha, dec) in [
        ("Spica", dm(158.0, 22.6), dm(-11.0, 17.4)),
        ("Regulus", dm(207.0, 35.1), dm(11.0, 50.9)),
        ("Procyon", dm(244.0, 51.6), dm(5.0, 9.8)),
        ("Canopus", dm(263.0, 53.1), dm(-52.0, 42.6)),
    ] {
        let s = star(&r, name);
        let (d_sha, d_dec) = (
            off(s["sha"]["value"].as_f64().unwrap(), sha),
            off(s["dec"]["value"].as_f64().unwrap(), dec),
        );
        assert!(
            d_sha <= 0.1 && d_dec <= 0.1,
            "{name}: SHA {d_sha}′, Dec {d_dec}′"
        );
    }
    // At the Spica sight, 24m03s later: 11°17.0′ + 6°01.7′.
    let r = run(json!({"utc": "2024-06-02T08:24:03Z", "dut1": "0 s"}));
    assert!(off(r["aries_gha"]["value"].as_f64().unwrap(), dm(17.0, 18.7)) <= 0.1);
    assert!(r["notice"].as_str().unwrap().contains("Nautical Almanac"));
}

#[test]
fn bowditch_high_altitude_sun() {
    // Bowditch 2024 §2011, Table 2011 (2024-05-31): the Sun at declination
    // N 22°02.6′, GHA 51°36.5′ at 15h24m13s. Its GHA for 15h15m03s, 49°22.0′,
    // is a misprint: USNO's celnav API gives 49.315819° (49°18.9′) for that
    // second (asked 2026-10-21), 9m10s before the second sight's 51°36.4′.
    for (t, g) in [("15:15:03", 49.315_819), ("15:24:13", dm(51.0, 36.5))] {
        let r = run(json!({"utc": format!("2024-05-31T{t}Z"), "dut1": "0 s"}));
        let d_gha = off(r["sun_gha"]["value"].as_f64().unwrap(), g);
        let d_dec = off(r["sun_dec"]["value"].as_f64().unwrap(), dm(22.0, 2.6));
        assert!(
            d_gha <= 0.1 && d_dec <= 0.1,
            "{t}: GHA {d_gha}′, Dec {d_dec}′"
        );
    }
}

#[test]
fn matches_usno_through_the_tool() {
    // The spec's parity check run through the tool: USNO celnav at 40
    // instants 1995 to 2045 (the gp-astro fixtures), with UT1 given as the
    // time and DUT1 0. Sun, planets, Aries, and stars within 0.1′; Venus and
    // the Moon to the bounds the gp-astro tests explain (USNO departs from
    // JPL there).
    let read = |name: &str| -> Value {
        let path = format!(
            "{}/../gp-astro/tests/data/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    };
    let bodies = read("usno_celnav.json");
    let stars = read("usno_stars.json");
    let mut cache = std::collections::HashMap::new();
    let mut result = |ut1: &str| -> Value {
        cache
            .entry(ut1.to_string())
            .or_insert_with(|| {
                run(json!({"utc": format!("{}:00Z", ut1.replace(' ', "T")), "dut1": "0 s"}))
            })
            .clone()
    };
    let mut worst: f64 = 0.0;
    for row in bodies["rows"].as_array().unwrap() {
        let r = result(row["ut1"].as_str().unwrap());
        let name = row["body"].as_str().unwrap();
        let b = r["bodies"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["body"] == name)
            .unwrap();
        let d = off(
            b["gha"]["value"].as_f64().unwrap(),
            row["gha"].as_f64().unwrap(),
        )
        .max(off(
            b["dec"]["value"].as_f64().unwrap(),
            row["dec"].as_f64().unwrap(),
        ));
        let limit = match name {
            "Venus" => 0.5,
            "Moon" => 0.25,
            _ => {
                worst = worst.max(d);
                0.1
            }
        };
        assert!(d < limit, "{name} at {}: {d}′", row["ut1"]);
    }
    for row in stars["rows"].as_array().unwrap() {
        let r = result(row["ut1"].as_str().unwrap());
        let aries = r["aries_gha"]["value"].as_f64().unwrap();
        let d = if row["body"] == "Aries" {
            off(aries, row["gha"].as_f64().unwrap())
        } else {
            let hip = row["hip"].as_u64().unwrap();
            let name = gp_astro::stars::STARS
                .iter()
                .find(|s| u64::from(s.hip) == hip)
                .unwrap()
                .name;
            let s = star(&r, name);
            off(
                aries + s["sha"]["value"].as_f64().unwrap(),
                row["gha"].as_f64().unwrap(),
            )
            .max(off(
                s["dec"]["value"].as_f64().unwrap(),
                row["dec"].as_f64().unwrap(),
            ))
        };
        worst = worst.max(d);
        assert!(d < 0.1, "{} at {}: {d}′", row, row["ut1"]);
    }
    eprintln!("worst against USNO (all but Venus and the Moon): {worst}′");
}

#[test]
fn hp_sd_and_the_equation_of_time() {
    // Early November the Sun runs about 16.4 minutes ahead of the mean sun;
    // mid-February about 14.2 minutes behind. The Moon's HP lies between
    // 53.9′ and 61.5′ and its SD is about 0.2725 HP.
    let r = run(json!({"utc": "2024-11-03T12:00:00Z", "dut1": "0 s"}));
    let e = r["equation_of_time"]["value"].as_f64().unwrap();
    assert!((e - 16.4).abs() < 0.1, "{e}");
    let r = run(json!({"utc": "2024-02-12T12:00:00Z", "dut1": "0 s"}));
    let e = r["equation_of_time"]["value"].as_f64().unwrap();
    assert!((e + 14.2).abs() < 0.1, "{e}");
    let hp = r["moon_hp"]["value"].as_f64().unwrap();
    let sd = r["moon_sd"]["value"].as_f64().unwrap();
    assert!((53.9..61.5).contains(&hp), "{hp}");
    assert!((sd / hp - 0.2725).abs() < 0.001, "{sd} {hp}");
    let sun_sd = r["sun_sd"]["value"].as_f64().unwrap();
    assert!((15.7..16.3).contains(&sun_sd), "{sun_sd}");
    assert_eq!(r["stars"].as_array().unwrap().len(), 58);
    assert_eq!(r["bodies"].as_array().unwrap().len(), 6);
}

#[test]
fn out_of_range_and_warnings() {
    let err = |utc: &str| -> Value {
        serde_json::from_str(&REGISTRY.invoke(
            "navigation.celestial.almanac",
            &json!({"utc": utc}).to_string(),
        ))
        .unwrap()
    };
    for utc in ["1799-12-31T23:59:59Z", "2101-01-01T00:00:00Z"] {
        let r = err(utc);
        assert_eq!(r["error"]["code"], "OUT_OF_DOMAIN", "{r}");
        assert!(
            r["error"]["message"]
                .as_str()
                .unwrap()
                .contains("1800 to 2100")
        );
    }
    let codes = |r: &Value| -> Vec<String> {
        r["meta"]["warnings"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|w| w["code"].as_str().unwrap().to_string())
                    .collect()
            })
            .unwrap_or_default()
    };
    let r = err("2030-01-01T00:00:00Z");
    assert_eq!(r["ok"], true);
    let c = codes(&r);
    assert!(
        c.contains(&"UT1_APPROXIMATED".into()) && c.contains(&"LEAP_SECOND_TABLE_EXPIRED".into()),
        "{c:?}"
    );
    // 1850 has no leap seconds: ΔT is the dated estimate, about 7 s.
    let r = run(json!({"utc": "1850-06-01T00:00:00Z", "dut1": "0 s"}));
    let dt = r["delta_t"]["value"].as_f64().unwrap();
    assert!((5.0..10.0).contains(&dt), "{dt}");
}
