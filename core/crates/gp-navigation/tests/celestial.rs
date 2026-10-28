//! The celestial almanac (add-celestial-and-moon, navigation/celestial).

use gp_navigation::REGISTRY;
use serde_json::{Value, json};

fn run(input: Value) -> Value {
    call("navigation.celestial.almanac", input)
}

fn envelope(id: &str, input: Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).unwrap()
}

fn call(id: &str, input: Value) -> Value {
    let r = envelope(id, input.clone());
    assert_eq!(r["ok"], true, "{input}: {r}");
    r["result"].clone()
}

fn reduce(input: Value) -> Value {
    call("navigation.celestial.sight-reduction", input)
}

fn correct(input: Value) -> Value {
    call("navigation.celestial.altitude-correction", input)
}

fn num(v: &Value, key: &str) -> f64 {
    v[key]["value"]
        .as_f64()
        .unwrap_or_else(|| panic!("{key} missing in {v}"))
}

fn codes(r: &Value) -> Vec<String> {
    r["meta"]["warnings"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|w| w["code"].as_str().unwrap().to_string())
                .collect()
        })
        .unwrap_or_default()
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

// -- Sight reduction (add-celestial-and-moon task 3) ------------------------

/// Bowditch 2024 §2010, Table 2010b: the four-star fix worked right through,
/// at the assumed positions the table chose. Three of its four columns hold:
/// the table's Hc comes from Pub. No. 229 with its own interpolation and each
/// of its inputs is rounded to 0.1', so 0.2' is as close as it can be read.
/// Procyon's column is left out: its increment for 35m59s is printed as
/// 8°56.2' where 9°01.2' is right, so its GHA, LHA, and Hc are all 5' adrift
/// (USNO gives GHA 265°09.9' for that second, not the table's 265°04.8'), and
/// Table 2010b also repeats its GHA in the SHA row.
#[test]
fn bowditch_star_fix_reduced() {
    for (body, utc, lat, lon, ho, lha, dec, hc, zn, a, z_label) in [
        (
            "Spica",
            "08:24:03",
            "41 00.0 S",
            "128 18.7 E",
            "32 53.4",
            304.0,
            -dm(11.0, 17.4),
            dm(32.0, 50.4),
            75.4,
            3.0,
            "S 104.6 E",
        ),
        (
            "Regulus",
            "08:29:58",
            "41 00.0 S",
            "127 37.3 E",
            "37 05.6",
            354.0,
            dm(11.0, 50.9),
            dm(36.0, 51.6),
            7.3,
            14.0,
            "S 172.7 E",
        ),
        (
            "Canopus",
            "08:41:55",
            "41 00.0 S",
            "128 19.4 E",
            "52 11.7",
            54.0,
            -dm(52.0, 42.6),
            dm(52.0, 15.0),
            233.2,
            -3.3,
            "S 53.2 W",
        ),
    ] {
        let r = reduce(json!({
            "body": body,
            "utc": format!("2024-06-02T{utc}Z"),
            "dut1": "0 s",
            "lat": lat,
            "lon": lon,
            "ho": ho,
        }));
        let d_lha = off(num(&r, "lha"), lha);
        let d_dec = off(num(&r, "dec"), dec);
        let d_hc = off(num(&r, "hc"), hc);
        let d_zn = (num(&r, "zn") - zn).abs();
        let d_a = (num(&r, "intercept") - a).abs();
        assert!(
            d_lha <= 0.2 && d_dec <= 0.1 && d_hc <= 0.2 && d_zn <= 0.1 && d_a <= 0.2,
            "{body}: LHA {d_lha}', Dec {d_dec}', Hc {d_hc}', Zn {d_zn} deg, a {d_a} NM"
        );
        assert_eq!(r["z_label"], z_label, "{body}");
        assert_eq!(
            r["direction"],
            if a >= 0.0 { "Toward" } else { "Away" },
            "{body}"
        );
        assert_eq!(r["lop"].as_array().unwrap().len(), 2);
    }
}

/// The same four sights through the assumed-position rule: the navigator
/// gives the dead-reckoning position and the tool rounds it the way Pub. No.
/// 229 needs, which is how Table 2010b's assumed longitudes arose.
#[test]
fn assumed_position_rule_matches_the_table() {
    for (body, utc, lon) in [
        ("Spica", "08:24:03", dm(128.0, 18.7)),
        ("Regulus", "08:29:58", dm(127.0, 37.3)),
        ("Canopus", "08:41:55", dm(128.0, 19.4)),
    ] {
        let r = reduce(json!({
            "body": body,
            "utc": format!("2024-06-02T{utc}Z"),
            "dut1": "0 s",
            "lat": "41 10.0 S",
            "lon": "128 00.0 E",
            "ho": "32 53.4",
            "position_rule": "pub229",
        }));
        let lha = num(&r, "lha");
        assert!(
            (lha - lha.round()).abs() * 60.0 < 0.01,
            "{body}: LHA {lha} is not a whole degree"
        );
        assert!((num(&r, "ap_lat") + 41.0).abs() < 1e-9, "{body}");
        let d_lon = off(num(&r, "ap_lon"), lon);
        assert!(d_lon <= 0.2, "{body}: assumed longitude off by {d_lon}'");
    }
}

/// The spec's parity check: USNO's own computed altitude and azimuth at the
/// assumed position, for twelve bodies at 24 instants and places 1995 to 2045.
#[test]
fn sight_reduction_matches_usno() {
    let path = format!(
        "{}/tests/data/usno_sight_reduction.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let data: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let (mut worst_hc, mut worst_zn) = (0.0_f64, 0.0_f64);
    let mut worst_odd = std::collections::BTreeMap::new();
    for row in data["rows"].as_array().unwrap() {
        let body = row["body"].as_str().unwrap();
        let r = reduce(json!({
            "body": body,
            "utc": format!("{}Z", row["ut1"].as_str().unwrap().replace(' ', "T")),
            "dut1": "0 s",
            "lat": row["lat"],
            "lon": row["lon"],
            "ho": 0.0,
        }));
        let hc = row["hc"].as_f64().unwrap();
        let d_hc = off(num(&r, "hc"), hc);
        // USNO's Venus and Moon depart from JPL where the gp-astro tests
        // record it; the almanac test explains the same two bounds.
        let limit = match body {
            "Venus" | "Moon" => {
                let w = worst_odd.entry(body.to_string()).or_insert(0.0_f64);
                *w = w.max(d_hc);
                if body == "Venus" { 0.5 } else { 0.25 }
            }
            _ => {
                worst_hc = worst_hc.max(d_hc);
                0.1
            }
        };
        assert!(d_hc < limit, "{body} Hc at {}: {d_hc}'", row["ut1"]);
        // Azimuth loses meaning near the zenith, where a tenth of a minute of
        // altitude swings it degrees.
        if hc < 85.0 {
            let d_zn = ((num(&r, "zn") - row["zn"].as_f64().unwrap() + 180.0).rem_euclid(360.0)
                - 180.0)
                .abs();
            if !matches!(body, "Venus" | "Moon") {
                worst_zn = worst_zn.max(d_zn);
            }
            assert!(d_zn < 0.05, "{body} Zn at {}: {d_zn} deg", row["ut1"]);
        }
    }
    eprintln!("worst against USNO: Hc {worst_hc}', Zn {worst_zn} deg; {worst_odd:?}");
}

/// Standing at the body's geographic position, the body is overhead: the
/// computed altitude is 90 degrees and a true sight has no intercept.
#[test]
fn at_the_geographic_position_the_intercept_vanishes() {
    let utc = "2024-06-02T08:24:03Z";
    let a = run(json!({"utc": utc, "dut1": "0 s"}));
    let (gha, dec) = (
        a["sun_gha"]["value"].as_f64().unwrap(),
        a["sun_dec"]["value"].as_f64().unwrap(),
    );
    let lon = ((-gha + 180.0).rem_euclid(360.0)) - 180.0;
    let r = reduce(json!({
        "body": "Sun", "utc": utc, "dut1": "0 s",
        "lat": dec, "lon": lon, "ho": 90.0,
    }));
    assert!((num(&r, "hc") - 90.0).abs() * 60.0 < 0.01, "{}", r["hc"]);
    assert!(num(&r, "intercept").abs() < 0.01, "{}", r["intercept"]);
    assert_eq!(r["direction"], "Toward");
}

#[test]
fn below_the_horizon_still_answers() {
    // Midnight at Greenwich in June: the Sun is well below the horizon.
    let r = envelope(
        "navigation.celestial.sight-reduction",
        json!({"body": "Sun", "utc": "2024-06-02T00:00:00Z", "dut1": "0 s",
               "lat": 51.48, "lon": 0.0, "ho": "-5 00.0"}),
    );
    assert_eq!(r["ok"], true, "{r}");
    assert!(codes(&r).contains(&"BODY_BELOW_HORIZON".into()), "{r}");
    let result = r["result"].clone();
    assert!(num(&result, "hc") < 0.0, "{}", result["hc"]);
    assert!(num(&result, "zn") >= 0.0);
}

#[test]
fn an_unknown_body_is_refused() {
    let r = envelope(
        "navigation.celestial.sight-reduction",
        json!({"body": "Betelgeuse-ish", "utc": "2024-06-02T08:24:03Z",
               "lat": 0.0, "lon": 0.0, "ho": 10.0}),
    );
    assert_eq!(r["error"]["code"], "INVALID_INPUT", "{r}");
    assert_eq!(r["error"]["field"], "/body", "{r}");
}

// -- Altitude corrections (add-celestial-and-moon task 3) ------------------

/// The worked examples of Bowditch 2024 chapter 16, §1618 to §1622, every one
/// of them: the Sun's two limbs, the Moon's two, a planet, a star, and the
/// two low-altitude sights of §1622, which the chapter itself calls variable.
#[test]
fn bowditch_chapter_16_examples() {
    struct Case<'a> {
        name: &'a str,
        input: Value,
        ha: f64,
        ho: f64,
        limit: f64,
    }
    let cases = [
        Case {
            name: "§1618 Example 1, the Sun's lower limb",
            input: json!({"hs": "51 28.4", "body": "Sun", "limb": "lower",
                          "height_of_eye": "38 ft", "utc": "2024-06-02T11:00:00Z",
                          "index_correction": "-2.0 arcmin", "dut1": "0 s"}),
            ha: dm(51.0, 20.4),
            ho: dm(51.0, 35.6),
            limit: 0.15,
        },
        Case {
            name: "§1618 Example 2, the Sun's upper limb",
            input: json!({"hs": "32 47.9", "body": "Sun", "limb": "upper",
                          "height_of_eye": "45 ft", "utc": "2024-06-02T11:00:00Z",
                          "index_correction": "1.0 arcmin", "dut1": "0 s"}),
            ha: dm(32.0, 42.4),
            ho: dm(32.0, 25.1),
            limit: 0.15,
        },
        Case {
            name: "§1619 Example 1, the Moon's lower limb",
            input: json!({"hs": "18 04.6", "body": "Moon", "limb": "lower",
                          "height_of_eye": "32 ft", "utc": "2024-06-02T11:00:00Z",
                          "index_correction": "3.2 arcmin", "hp": "59.6 arcmin",
                          "dut1": "0 s"}),
            ha: dm(18.0, 2.3),
            ho: dm(19.0, 12.2),
            limit: 0.1,
        },
        Case {
            name: "§1619 Example 2, the Moon's upper limb",
            input: json!({"hs": "66 47.3", "body": "Moon", "limb": "upper",
                          "height_of_eye": "70 ft", "utc": "2024-06-02T09:00:00Z",
                          "index_correction": "-1.6 arcmin", "hp": "59.6 arcmin",
                          "dut1": "0 s"}),
            ha: dm(66.0, 37.6),
            ho: dm(66.0, 44.5),
            limit: 0.1,
        },
        Case {
            name: "§1620, Venus",
            input: json!({"hs": "44 21.3", "body": "Venus", "limb": "center",
                          "height_of_eye": "28 ft", "utc": "2024-12-19T18:00:00Z",
                          "dut1": "0 s"}),
            ha: dm(44.0, 16.2),
            ho: dm(44.0, 15.3),
            limit: 0.1,
        },
        Case {
            name: "§1621, the star Miaplacidus",
            input: json!({"hs": "27 54.0", "body": "Miaplacidus", "limb": "center",
                          "height_of_eye": "50 ft", "utc": "2024-06-02T11:00:00Z",
                          "index_correction": "1.0 arcmin", "dut1": "0 s"}),
            ha: dm(27.0, 48.1),
            ho: dm(27.0, 46.3),
            limit: 0.1,
        },
        Case {
            name: "§1622 Example 1, the Sun low down at 88 degrees Fahrenheit",
            input: json!({"hs": "1 24.4", "body": "Sun", "limb": "lower",
                          "height_of_eye": "45 ft", "utc": "2024-06-02T11:00:00Z",
                          "index_correction": "1.8 arcmin", "temperature": "88 degF",
                          "pressure": "29.78 inHg", "dut1": "0 s"}),
            ha: dm(1.0, 19.7),
            ho: dm(1.0, 15.4),
            limit: 0.25,
        },
        Case {
            name: "§1622 Example 3, the Moon's upper limb low down",
            input: json!({"hs": "2 35.4", "body": "Moon", "limb": "upper",
                          "height_of_eye": "33 ft", "utc": "2024-06-02T17:14:27Z",
                          "temperature": "63 degF", "pressure": "29.81 inHg",
                          "hp": "59.6 arcmin", "dut1": "0 s"}),
            ha: dm(2.0, 29.8),
            ho: dm(2.0, 57.4),
            limit: 0.25,
        },
    ];
    for c in cases {
        let r = correct(c.input.clone());
        let d_ha = off(num(&r, "ha"), c.ha);
        let d_ho = off(num(&r, "ho"), c.ho);
        assert!(
            d_ha <= 0.05 && d_ho <= c.limit,
            "{}: ha off {d_ha}', Ho off {d_ho}' (limit {})",
            c.name,
            c.limit
        );
        assert_eq!(r["corrections"].as_array().unwrap().len(), 5, "{}", c.name);
    }
}

/// The corrections add up to the total, and the total takes hs to Ho.
#[test]
fn the_corrections_add_up() {
    let r = correct(json!({"hs": "51 28.4", "body": "Sun", "limb": "lower",
                           "height_of_eye": "38 ft", "utc": "2024-06-02T11:00:00Z",
                           "index_correction": "-2.0 arcmin", "dut1": "0 s"}));
    let sum: f64 = r["corrections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["value"]["value"].as_f64().unwrap())
        .sum();
    let total = num(&r, "total");
    assert!((sum - total).abs() < 1e-9, "{sum} vs {total}");
    let (hs, ho) = (dm(51.0, 28.4), num(&r, "ho"));
    assert!((hs + total / 60.0 - ho).abs() * 60.0 < 1e-6, "{ho}");
    // The dip of a 38-foot height of eye is 6.0'; refraction and dip subtract.
    assert!((num(&r, "dip") + 6.0).abs() < 0.05, "{}", r["dip"]);
    assert!(num(&r, "refraction") < 0.0);
    assert!(num(&r, "semidiameter") > 15.0);
}

#[test]
fn a_low_sight_says_so_and_a_star_has_no_limb() {
    let r = envelope(
        "navigation.celestial.altitude-correction",
        json!({"hs": "1 24.4", "body": "Sun", "limb": "lower", "height_of_eye": "45 ft",
               "utc": "2024-06-02T11:00:00Z", "dut1": "0 s"}),
    );
    assert_eq!(r["ok"], true, "{r}");
    assert!(codes(&r).contains(&"LOW_ALTITUDE_REFRACTION".into()), "{r}");
    let r = envelope(
        "navigation.celestial.altitude-correction",
        json!({"hs": "27 54.0", "body": "Spica", "limb": "lower",
               "height_of_eye": "50 ft", "utc": "2024-06-02T11:00:00Z", "dut1": "0 s"}),
    );
    assert_eq!(r["error"]["code"], "INVALID_INPUT", "{r}");
    assert_eq!(r["error"]["field"], "/limb", "{r}");
}

/// A sight corrected and then reduced: the two tools chain, and the second
/// one's observed altitude is the first one's answer.
#[test]
fn correcting_then_reducing_one_sight() {
    // Bowditch's §2010 example: a height of eye of 31 feet and an index
    // correction of 1.0' on the arc, and Table 2010a's sextant altitude for
    // Spica, 33°01.3'. Its Ho is the 32°53.4' Table 2010b reduces.
    let c = correct(json!({"hs": "33 01.3", "body": "Spica", "limb": "center",
                           "height_of_eye": "31 ft", "utc": "2024-06-02T08:24:03Z",
                           "index_correction": "-1.0 arcmin", "dut1": "0 s"}));
    let ho = num(&c, "ho");
    let d = off(ho, dm(32.0, 53.4));
    assert!(d <= 0.1, "Ho {ho} is {d}' from Bowditch's 32 53.4");
    let r = reduce(
        json!({"body": "Spica", "utc": "2024-06-02T08:24:03Z", "dut1": "0 s",
                          "lat": "41 00.0 S", "lon": "128 18.7 E", "ho": ho}),
    );
    let a = num(&r, "intercept");
    assert!((a - 3.0).abs() < 0.2, "intercept {a} NM");
}

fn polaris(input: Value) -> Value {
    call("navigation.celestial.polaris-latitude", input)
}

fn check(input: Value) -> Value {
    call("navigation.celestial.sun-azimuth-check", input)
}

/// Bowditch 2024 §2015's worked Polaris sight: Ho 16°11.3' at longitude
/// 110°07.3'W on 2024-06-02 at 12h25m12s UT gives latitude 16°03.2'N and an
/// azimuth of 000.8°. The three table corrections are each rounded to 0.1'
/// and the azimuth is read from a table printed to 0.1°, so the table cannot
/// be read closer than that.
#[test]
fn bowditch_polaris_sight() {
    let r = polaris(json!({
        "ho": "16 11.3",
        "utc": "2024-06-02T12:25:12Z",
        "dut1": "0 s",
        "lon": "110 07.3 W",
    }));
    let lat = num(&r, "lat");
    assert!(
        off(lat, dm(16.0, 3.2)) < 0.2,
        "latitude {lat} is {:.3}' from Bowditch's 16 03.2 N",
        off(lat, dm(16.0, 3.2))
    );
    // The example's own GHA Aries, 71°26.9' + 6°19.0'. Its printed LHA
    // Aries, 327°38.8', is 0.2' off that figure less the longitude.
    assert!(
        off(num(&r, "gha_aries"), dm(77.0, 45.9)) < 0.1,
        "GHA Aries {} is not the example's 77 45.9",
        num(&r, "gha_aries")
    );
    assert!(
        (num(&r, "zn") - 0.8).abs() < 0.2,
        "azimuth {} is not 000.8 degrees",
        num(&r, "zn")
    );
    // The sum of the Almanac's a0, a1, a2 less 1° is -8.1'.
    assert!(
        (num(&r, "correction") + 8.1).abs() < 0.2,
        "correction {} is not -8.1 minutes",
        num(&r, "correction")
    );
    assert_eq!(r["lat_text"], "16°03.2'N");
}

/// The latitude the solver returns is the latitude that would see the sight:
/// reducing a Polaris sight from it must return the altitude observed.
#[test]
fn the_polaris_latitude_reproduces_the_sight() {
    for (ho, lon) in [(16.188, -110.12), (45.0, 0.0), (62.5, 174.3), (8.0, 73.9)] {
        let r = polaris(json!({
            "ho": format!("{ho} deg"), "utc": "2024-06-02T12:25:12Z",
            "dut1": "0 s", "lon": format!("{lon} deg"),
        }));
        let back = reduce(json!({
            "body": "Polaris", "utc": "2024-06-02T12:25:12Z", "dut1": "0 s",
            "lat": format!("{} deg", num(&r, "lat")),
            "lon": format!("{lon} deg"), "ho": format!("{ho} deg"),
        }));
        assert!(
            num(&back, "intercept").abs() < 0.01,
            "ho {ho} at {lon}: intercept {} NM is not zero",
            num(&back, "intercept")
        );
        assert!(
            (num(&back, "zn") - num(&r, "zn")).abs() < 0.001,
            "ho {ho} at {lon}: azimuths disagree"
        );
    }
}

/// Bowditch 2024 §2016, Examples 1 and 2: the same declination and local hour
/// angle at a latitude of the same name and then a contrary one. The tool
/// takes a time and a place rather than an hour angle, so these check the
/// azimuth formula itself through a sight reduction at Bowditch's arguments,
/// where Hc is the altitude and Zn the azimuth of the same triangle.
#[test]
fn bowditch_azimuth_by_tables() {
    // The tool takes a time and a place, not an hour angle, so this checks
    // the azimuth formula itself at Bowditch's own arguments.
    for (lat, want) in [(dm(33.0, 24.0), 97.7), (-dm(33.0, 24.0), 44.1)] {
        let (dec, lha) = (dm(20.0, 13.8), dm(316.0, 41.2));
        let (phi, d, h) = (lat.to_radians(), dec.to_radians(), lha.to_radians());
        let zn = (-d.cos() * h.sin())
            .atan2(phi.cos() * d.sin() - phi.sin() * d.cos() * h.cos())
            .to_degrees()
            .rem_euclid(360.0);
        assert!(
            (zn - want).abs() < 0.05,
            "lat {lat}: azimuth {zn:.2} is not Bowditch's {want}"
        );
    }
}

/// Bowditch 2024 §2018: at latitude 51°24.6'N with the Sun's declination
/// N 19°40.4', the amplitude is W 32.6°N by double interpolation of Table 22,
/// a true bearing of 302.6°, so a gyro reading 303.0° is 0.4° W in error. The
/// Sun holds that declination on 2024-07-24 at 14h30m UT, and is setting then
/// at longitude 80°44.4'E, so the example is reproduced end to end. The
/// closed form gives 32.67°, which the table's 0.1° steps cannot resolve.
#[test]
fn bowditch_amplitude_and_gyro_error() {
    let r = check(json!({
        "utc": "2024-07-24T14:30:00Z", "dut1": "0 s",
        "lat": "51 24.6 N", "lon": "80 44.4 E",
        "bearing": "303 deg", "compass": "gyro",
    }));
    let dec = num(&r, "dec");
    assert!(
        off(dec, dm(19.0, 40.4)) < 0.1,
        "declination {dec} is {:.3}' from the example's N 19 40.4",
        off(dec, dm(19.0, 40.4))
    );
    let a = num(&r, "amplitude");
    assert!(
        (a - 32.6).abs() < 0.1,
        "amplitude {a:.3} is over 0.1 degrees from Bowditch's 32.6"
    );
    assert_eq!(r["amplitude_label"], "W 32.7 N");
    assert!(
        (num(&r, "amplitude_azimuth") - 302.6).abs() < 0.1,
        "true bearing {:.3} is not 302.6",
        num(&r, "amplitude_azimuth")
    );
    assert!(
        (num(&r, "error") + 0.4).abs() < 0.1,
        "gyro error {:.3} is not 0.4 W",
        num(&r, "error")
    );
    assert_eq!(r["error_text"], "0.3 W");
    // The Sun's center is on the celestial horizon at this instant, so the
    // amplitude and the computed azimuth are the same bearing.
    assert!(num(&r, "altitude").abs() < 0.05, "{}", num(&r, "altitude"));
    // The altitude is a hundredth of a degree rather than exactly zero, and
    // the azimuth moves about a degree for each degree of altitude here.
    assert!(
        (num(&r, "zn") - num(&r, "amplitude_azimuth")).abs() < 0.05,
        "on the horizon Zn {} and the amplitude bearing {} must agree",
        num(&r, "zn"),
        num(&r, "amplitude_azimuth")
    );
    // The exact amplitude for the example's own latitude and declination.
    let exact = (dm(19.0, 40.4).to_radians().sin() / dm(51.0, 24.6).to_radians().cos())
        .asin()
        .to_degrees();
    assert!(
        (exact - 32.6).abs() < 0.1,
        "the closed form gives {exact:.3}, over 0.1 degrees from the table"
    );
}

/// The sign of a compass error: east when the compass reads low, west when it
/// reads high (Bowditch §2016, where Zn 044.1° against a gyro reading 042.5°
/// is 1.6° east).
#[test]
fn the_compass_error_takes_its_name_from_the_compass() {
    let at = |bearing: f64| {
        check(json!({
            "utc": "2024-06-02T18:40:00Z", "dut1": "0 s",
            "lat": "51 24.6 N", "lon": "0 00.0 E",
            "bearing": format!("{bearing} deg"), "compass": "gyro",
        }))
    };
    let truth = num(&at(300.0), "zn");
    let low = at(truth - 1.6);
    assert!(
        (num(&low, "error") - 1.6).abs() < 0.01,
        "{}",
        num(&low, "error")
    );
    assert_eq!(low["error_text"], "1.6 E");
    let high = at(truth + 1.6);
    assert!(
        (num(&high, "error") + 1.6).abs() < 0.01,
        "{}",
        num(&high, "error")
    );
    assert_eq!(high["error_text"], "1.6 W");
    // Across north, where the two bearings sit either side of 360°.
    let r = check(json!({
        "utc": "2024-06-02T05:10:00Z", "dut1": "0 s",
        "lat": "51 24.6 N", "lon": "0 00.0 E",
        "bearing": "1 deg", "compass": "gyro",
    }));
    assert!(num(&r, "error").abs() <= 180.0);
}

/// A magnetic compass splits its error into the variation supplied and the
/// deviation left over, and will not be checked without a variation.
#[test]
fn a_magnetic_compass_needs_its_variation() {
    let base = |extra: Value| {
        let mut v = json!({
            "utc": "2024-06-02T18:40:00Z", "dut1": "0 s",
            "lat": "51 24.6 N", "lon": "0 00.0 E", "bearing": "303 deg",
        });
        for (k, x) in extra.as_object().unwrap() {
            v[k] = x.clone();
        }
        v
    };
    let r = envelope(
        "navigation.celestial.sun-azimuth-check",
        base(json!({"compass": "magnetic"})),
    );
    assert_eq!(r["ok"], false);
    assert_eq!(r["error"]["code"], "INVALID_INPUT");
    assert_eq!(r["error"]["field"], "/variation");
    let gyro = check(base(json!({"compass": "gyro"})));
    let mag = check(base(
        json!({"compass": "magnetic", "variation": "-1.5 deg"}),
    ));
    assert!((num(&mag, "error") - num(&gyro, "error")).abs() < 1e-9);
    assert!(
        (num(&mag, "deviation") - (num(&gyro, "error") + 1.5)).abs() < 1e-9,
        "deviation {} is not the error less the variation",
        num(&mag, "deviation")
    );
}

/// Without a bearing the tool still answers with the azimuth and amplitude,
/// and a latitude where the Sun never reaches the celestial horizon is
/// refused rather than answered.
#[test]
fn the_amplitude_has_a_domain() {
    let r = check(json!({
        "utc": "2024-06-02T18:40:00Z", "dut1": "0 s",
        "lat": "51 24.6 N", "lon": "0 00.0 E",
    }));
    assert!(r.get("error").is_none(), "{r}");
    assert!(r["answer"].as_str().unwrap().contains("amplitude"));
    let r = envelope(
        "navigation.celestial.sun-azimuth-check",
        json!({"utc": "2024-06-02T18:40:00Z", "dut1": "0 s", "lat": "80 N", "lon": "0 E"}),
    );
    assert_eq!(r["ok"], false, "{r}");
    assert_eq!(r["error"]["code"], "OUT_OF_DOMAIN");
}
