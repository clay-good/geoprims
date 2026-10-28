//! Astronomic azimuth (add-celestial-and-moon, survey/astro), against USNO.

use gp_survey::REGISTRY;
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

/// Degrees apart, the shorter way round the circle.
fn apart(a: f64, b: f64) -> f64 {
    ((a - b + 180.0).rem_euclid(360.0) - 180.0).abs()
}

const USNO: &str = include_str!("data/usno_azimuth.json");

/// The sun's semidiameter near the June solstice, when it is near aphelion.
const SUN_SD_JUNE_ARCSEC: f64 = 945.5;

/// The body's azimuth by the hour-angle method against USNO's own, at 61
/// places and instants from 1995 to 2045. A single pointing with a horizontal
/// angle of zero makes the line's azimuth the body's.
#[test]
fn azimuth_matches_usno() {
    let doc: Value = serde_json::from_str(USNO).unwrap();
    let mut worst = (0.0_f64, String::new());
    let mut n = (0, 0);
    for row in doc["rows"].as_array().unwrap() {
        let utc = format!("{}Z", row["ut1"].as_str().unwrap().replace(' ', "T"));
        let (lat, lon) = (row["lat"].as_f64().unwrap(), row["lon"].as_f64().unwrap());
        let polaris = row["body"] == "POLARIS";
        // Polaris is not observable from the southern hemisphere, and the
        // hour-angle method needs the body above the horizon to be pointed at.
        if row["hc"].as_f64().unwrap() < 1.0 {
            continue;
        }
        let id = if polaris {
            "survey.astro.polaris-azimuth"
        } else {
            "survey.astro.solar-azimuth"
        };
        let r = call(
            id,
            json!({"lat": lat, "lon": lon, "dut1": "0 s",
                   "pointings": [{"time": utc, "angle": "0 00 00"}]}),
        );
        let got = num(&r, "azimuth");
        let want = row["zn"].as_f64().unwrap();
        let d = apart(got, want);
        if d > worst.0 {
            worst = (d, format!("{utc} at {lat}, {lon} ({})", row["body"]));
        }
        assert!(
            d < 0.001,
            "{utc} at {lat}, {lon}: azimuth {got} is {d} degrees from USNO's {want}"
        );
        if polaris {
            n.0 += 1;
        } else {
            n.1 += 1;
        }
    }
    println!(
        "survey azimuth vs USNO: {} Polaris and {} sun pointings, worst {:.6} degrees ({})",
        n.0, n.1, worst.0, worst.1
    );
    assert!(n.0 >= 25 && n.1 >= 15, "only {n:?} rows");
}

/// The spec's worked Polaris observation: 2016-02-23 04:21:15 UT at 29°31.0'N
/// 074°30.0'W, with a horizontal angle of 45°00'00" from the mark. USNO puts
/// Polaris at 359.250°, so the line is at 314.250°.
#[test]
fn the_spec_polaris_observation() {
    let r = call(
        "survey.astro.polaris-azimuth",
        json!({"lat": "29 31.0 N", "lon": "74 30.0 W", "dut1": "0 s",
               "pointings": [{"time": "2016-02-23T04:21:15Z", "angle": "45 00 00"}]}),
    );
    let body = num(&r["pointings"][0], "body_azimuth");
    assert!(
        apart(body, 359.2) < 0.1,
        "Polaris's azimuth {body} is not 359.2 degrees"
    );
    assert!(
        apart(num(&r, "azimuth"), 314.2) < 0.1,
        "the line's azimuth {} is not 314.2 degrees",
        num(&r, "azimuth")
    );
    // Tighter, against USNO's own figure for that instant and place.
    assert!(apart(body, 359.250169) < 0.001, "{body}");
    assert!(r["notice"].as_str().unwrap().contains("licensed surveyor"));
    assert!(
        codes(&envelope(
            "survey.astro.polaris-azimuth",
            json!({"lat": "29 31.0 N", "lon": "74 30.0 W", "dut1": "0 s",
               "pointings": [{"time": "2016-02-23T04:21:15Z", "angle": "45 00 00"}]}),
        ))
        .contains(&"ASTRONOMIC_AZIMUTH".to_string())
    );
}

/// The limb correction is the semidiameter over the cosine of the altitude,
/// and the two limbs move the line's azimuth opposite ways by that much.
#[test]
fn the_limb_correction_is_the_semidiameter_over_cos_altitude() {
    // The spec's arithmetic: 16.1' at 30 degrees of altitude is 18.59'.
    let want = 16.1 / 30.0_f64.to_radians().cos();
    assert!((want - 18.59).abs() < 0.01, "{want}");
    let at = |limb: &str| {
        call(
            "survey.astro.solar-azimuth",
            json!({"lat": "39 44 21.1 N", "lon": "104 59 25.1 W", "dut1": "0 s", "limb": limb,
                   "pointings": [{"time": "2026-06-21T15:40:00Z", "angle": "112 18 40"}]}),
        )
    };
    let (centre, leading, trailing) = (at("center"), at("leading"), at("trailing"));
    let correction = num(&leading, "limb_correction");
    assert!(
        correction > 900.0 && correction < 1_400.0,
        "the correction is {correction} arcseconds, not a semidiameter over a cosine"
    );
    assert!(
        (num(&trailing, "limb_correction") - correction).abs() < 1e-9,
        "the two limbs must share the magnitude"
    );
    // Leading sits at the greater azimuth, so the angle to the centre is less
    // and the line's azimuth is greater by the correction.
    let c = num(&centre, "azimuth");
    assert!(
        ((num(&leading, "azimuth") - c) * 3600.0 - correction).abs() < 0.01,
        "leading moved the line by {} arcseconds, not {correction}",
        (num(&leading, "azimuth") - c) * 3600.0
    );
    assert!(
        ((num(&trailing, "azimuth") - c) * 3600.0 + correction).abs() < 0.01,
        "trailing must move it the other way"
    );
    // The correction over the sun's semidiameter is the secant of the
    // altitude, which is over 1 and, in June from Denver at that hour, under 2.
    let secant = correction / (SUN_SD_JUNE_ARCSEC);
    assert!(
        secant > 1.0 && secant < 2.0,
        "the secant of the altitude is {secant}"
    );
}

/// A Laplace correction turns the astronomic azimuth into a geodetic one, and
/// the warning says so.
#[test]
fn a_laplace_correction_gives_the_geodetic_azimuth() {
    let base = json!({"lat": "29 31.0 N", "lon": "74 30.0 W", "dut1": "0 s",
                      "pointings": [{"time": "2016-02-23T04:21:15Z", "angle": "45 00 00"}]});
    let plain = call("survey.astro.polaris-azimuth", base.clone());
    assert!(plain.get("geodetic_azimuth").is_none(), "{plain}");
    let mut with = base.clone();
    with["laplace"] = json!("-2.4 arcsec");
    let r = envelope("survey.astro.polaris-azimuth", with);
    assert_eq!(r["ok"], true, "{r}");
    let g = &r["result"];
    assert!(
        ((num(g, "geodetic_azimuth") - num(g, "azimuth")) * 3600.0 + 2.4).abs() < 1e-6,
        "geodetic {} is not astronomic {} less 2.4 seconds",
        num(g, "geodetic_azimuth"),
        num(g, "azimuth")
    );
    let msg = r["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["code"] == "ASTRONOMIC_AZIMUTH")
        .map(|w| w["message"].as_str().unwrap().to_string())
        .expect("the warning is there");
    assert!(msg.contains("-2.40") || msg.contains("-2.4"), "{msg}");
}

/// Six pointings with one blunder: the odd one is flagged and the mean
/// without it is reported.
#[test]
fn a_blunder_in_a_set_is_flagged() {
    let mut pointings = Vec::new();
    // Polaris moves so little that six pointings two minutes apart on the same
    // mark give nearly the same azimuth, which is what makes a blunder show.
    for (i, angle) in [
        "45 00 00", "45 00 02", "44 59 58", "45 00 01", "45 02 00", "44 59 59",
    ]
    .iter()
    .enumerate()
    {
        pointings.push(json!({
            "time": format!("2016-02-23T04:2{}:15Z", i),
            "angle": angle,
        }));
    }
    let r = envelope(
        "survey.astro.polaris-azimuth",
        json!({"lat": "29 31.0 N", "lon": "74 30.0 W", "dut1": "0 s", "pointings": pointings}),
    );
    assert_eq!(r["ok"], true, "{r}");
    assert!(
        codes(&r).contains(&"SUSPECT_VALUE".to_string()),
        "{:?}",
        codes(&r)
    );
    let g = &r["result"];
    let flagged: Vec<usize> = g["pointings"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .filter(|(_, p)| p["suspect"] == "yes")
        .map(|(i, _)| i)
        .collect();
    assert_eq!(flagged, vec![4], "the fifth pointing is the blunder");
    assert!(g.get("mean_without_suspect").is_some(), "{g}");
    // Dropping a pointing 2 minutes out of six moves the mean by about 20
    // seconds of arc.
    let moved = (num(g, "mean_without_suspect") - num(g, "azimuth")).abs() * 3600.0;
    assert!(
        moved > 10.0 && moved < 30.0,
        "the mean moved {moved} arcseconds"
    );
}

/// Without DUT1 the result says UT1 was approximated, and a bad angle or time
/// is refused at its own row.
#[test]
fn the_inputs_are_checked_row_by_row() {
    let r = envelope(
        "survey.astro.polaris-azimuth",
        json!({"lat": "29 31.0 N", "lon": "74 30.0 W",
               "pointings": [{"time": "2016-02-23T04:21:15Z", "angle": "45 00 00"}]}),
    );
    assert!(codes(&r).contains(&"UT1_APPROXIMATED".to_string()));
    let bad = envelope(
        "survey.astro.polaris-azimuth",
        json!({"lat": 29.5, "lon": -74.5, "dut1": "0 s",
               "pointings": [{"time": "2016-02-23T04:21:15Z", "angle": "45 00 00"},
                             {"time": "not a time", "angle": "45 00 00"}]}),
    );
    assert_eq!(bad["ok"], false);
    assert_eq!(bad["error"]["field"], "/pointings/1/time");
    let bad = envelope(
        "survey.astro.polaris-azimuth",
        json!({"lat": 29.5, "lon": -74.5, "dut1": "0 s",
               "pointings": [{"time": "2016-02-23T04:21:15Z", "angle": "400 00 00"}]}),
    );
    assert_eq!(bad["ok"], false);
    assert_eq!(bad["error"]["field"], "/pointings/0/angle");
    let old = envelope(
        "survey.astro.polaris-azimuth",
        json!({"lat": 29.5, "lon": -74.5, "dut1": "0 s",
               "pointings": [{"time": "1750-01-01T00:00:00Z", "angle": "0 00 00"}]}),
    );
    assert_eq!(old["ok"], false);
    assert_eq!(old["error"]["code"], "OUT_OF_DOMAIN");
}
