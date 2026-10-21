//! Tides (add-marine-navigation, navigation/tides): prediction parity with
//! NOAA's own predictions, from the same constituents and datums, and the
//! datum conversion scenarios.

use gp_navigation::REGISTRY;
use serde_json::{Value, json};

fn run(id: &str, input: Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).unwrap()
}

/// Minutes since 1970 of a NOAA "YYYY-MM-DD HH:MM" or our ISO stamp.
fn minutes(s: &str) -> f64 {
    let (date, time) = (&s[..10], &s[11..16]);
    let p: Vec<i64> = date.split('-').map(|x| x.parse().unwrap()).collect();
    let (y, m, d) = (p[0], p[1], p[2]);
    // Days from civil (Howard Hinnant's algorithm).
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let days = era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468;
    let h: f64 = time[..2].parse().unwrap();
    let mi: f64 = time[3..5].parse().unwrap();
    days as f64 * 1440.0 + h * 60.0 + mi
}

#[test]
fn predictions_match_noaa_within_0_02_ft_and_2_minutes() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/tides");
    for st in ["8443970", "9447130", "9414290", "8724580", "1612340"] {
        let f: Value =
            serde_json::from_str(&std::fs::read_to_string(format!("{dir}/{st}.json")).unwrap())
                .unwrap();
        let r = run(
            "navigation.tide.predict",
            json!({
                "constituents": f["harcon"].to_string(),
                "datum_sheet": f["datums"].to_string(),
                "start": "2026-10-01T00:00Z",
                "days": 31,
                "interval": "60 min",
            }),
        );
        assert_eq!(r["ok"], true, "{st}: {}", r["error"]);
        let series = r["result"]["series"].as_array().unwrap();
        let hourly = f["hourly"].as_array().unwrap();
        let mut worst: f64 = 0.0;
        for (k, want) in hourly.iter().enumerate() {
            let got = &series[k];
            assert_eq!(got["height"]["unit"], "ft");
            assert_eq!(
                minutes(got["time"].as_str().unwrap()),
                minutes(want[0].as_str().unwrap()),
                "{st} row {k}"
            );
            worst = worst
                .max((got["height"]["value"].as_f64().unwrap() - want[1].as_f64().unwrap()).abs());
        }
        assert!(
            worst <= 0.02,
            "{st}: hourly heights differ by up to {worst:.4} ft"
        );
        let (mut wt, mut wh): (f64, f64) = (0.0, 0.0);
        let ext = r["result"]["extremes"].as_array().unwrap();
        let hilo = f["hilo"].as_array().unwrap();
        assert_eq!(
            ext.len(),
            hilo.len(),
            "{st}: {} extremes against NOAA's {}",
            ext.len(),
            hilo.len()
        );
        for (got, want) in ext.iter().zip(hilo) {
            assert_eq!(got["type"], want[2], "{st} {want}");
            let dt =
                (minutes(got["time"].as_str().unwrap()) - minutes(want[0].as_str().unwrap())).abs();
            let dh = (got["height"]["value"].as_f64().unwrap() - want[1].as_f64().unwrap()).abs();
            assert!(dt <= 2.0 && dh <= 0.02, "{st}: {got} against {want}");
            (wt, wh) = (wt.max(dt), wh.max(dh));
        }
        eprintln!(
            "{st}: hourly worst {worst:.4} ft; extremes worst {wt} min, {wh:.4} ft over {} events",
            ext.len()
        );
    }
}

#[test]
fn tide_refusals_and_options() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/tides");
    let f: Value =
        serde_json::from_str(&std::fs::read_to_string(format!("{dir}/8443970.json")).unwrap())
            .unwrap();
    let base = |extra: Value| {
        let mut v = json!({"constituents": f["harcon"].to_string(), "datum_sheet": f["datums"].to_string(), "start": "2026-10-21"});
        v.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        run("navigation.tide.predict", v)
    };
    // The spec's window limit.
    let long = base(json!({"days": 90}));
    assert_eq!(long["error"]["code"], "LIMIT_EXCEEDED");
    assert!(long["error"]["message"].as_str().unwrap().contains("31"));
    // Twenty of the 37: the missing 17 are named, and the run needs the acknowledgment.
    let mut few = f["harcon"].clone();
    few["HarmonicConstituents"]
        .as_array_mut()
        .unwrap()
        .truncate(20);
    let partial = run(
        "navigation.tide.predict",
        json!({"constituents": few.to_string(), "start": "2026-10-21"}),
    );
    assert_eq!(partial["ok"], false);
    let msg = partial["error"]["message"].as_str().unwrap();
    assert!(
        msg.contains("17 of NOAA's 37") && msg.contains("MS4"),
        "{msg}"
    );
    let ack = run(
        "navigation.tide.predict",
        json!({"constituents": few.to_string(), "start": "2026-10-21", "partial": "acknowledge"}),
    );
    assert_eq!(ack["ok"], true, "{ack}");
    assert!(
        ack["meta"]["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["code"] == "PARTIAL_CONSTITUENTS")
    );
    // Without a datum sheet, heights are above mean sea level and say so.
    let msl = run(
        "navigation.tide.predict",
        json!({"constituents": f["harcon"].to_string(), "start": "2026-10-21"}),
    );
    assert!(
        msl["result"]["datum_used"]
            .as_str()
            .unwrap()
            .starts_with("MSL")
    );
    // The instant option stays within a few hundredths of the mid-year one.
    let mid = base(json!({}));
    let inst = base(json!({"node_factors": "instant"}));
    let d = (mid["result"]["high"]["value"].as_f64().unwrap()
        - inst["result"]["high"]["value"].as_f64().unwrap())
    .abs();
    assert!(d < 0.1, "{d}");
    // A local-time paste with its offset gives the same tide as the GMT one.
    let table: String = f["harcon"]["HarmonicConstituents"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(k, c)| {
            let gmt = c["phase_GMT"].as_f64().unwrap();
            let sp = c["speed"].as_f64().unwrap();
            let local = (gmt + sp * -5.0).rem_euclid(360.0);
            format!(
                "{} {} {} {:.4} {} desc\n",
                k + 1,
                c["name"].as_str().unwrap(),
                c["amplitude"],
                local,
                sp
            )
        })
        .collect();
    let loc = run(
        "navigation.tide.predict",
        json!({"constituents": table, "units": "ft", "phases": "local", "utc_offset": "-05:00", "start": "2026-10-21T00:00Z", "datum_sheet": f["datums"].to_string()}),
    );
    assert_eq!(loc["ok"], true, "{loc}");
    assert!(
        (loc["result"]["high"]["value"].as_f64().unwrap()
            - mid["result"]["high"]["value"].as_f64().unwrap())
        .abs()
            < 1e-3
    );
}

#[test]
fn datum_conversion() {
    let sheet = json!({"units": "meters", "datums": [
        {"name": "STND", "value": 0.0}, {"name": "MHHW", "value": 4.205}, {"name": "MSL", "value": 2.66},
        {"name": "MLLW", "value": 1.074}, {"name": "GT", "value": 3.131}, {"name": "NAVD88", "value": 2.752}]})
    .to_string();
    // The spec's Boston scenario: 3.000 m MLLW is 1.322 m NAVD88.
    let r = run(
        "navigation.tide.datums",
        json!({"sheet": sheet, "height": "3.000 m", "from": "MLLW", "to": "NAVD88"}),
    );
    assert!(
        (r["result"]["converted"]["value"].as_f64().unwrap() - 1.322).abs() < 1e-9,
        "{r}"
    );
    // A datum the sheet lacks names the ones it has; a range row is not a datum.
    let lat = run(
        "navigation.tide.datums",
        json!({"sheet": sheet, "height": "1 m", "from": "MLLW", "to": "LAT"}),
    );
    assert_eq!(lat["error"]["field"], "/to");
    assert!(lat["error"]["message"].as_str().unwrap().contains("MHHW"));
    let gt = run(
        "navigation.tide.datums",
        json!({"sheet": sheet, "height": "1 m", "from": "MLLW", "to": "GT"}),
    );
    assert_eq!(gt["ok"], false);
    // A pasted table in feet, and a round trip.
    let table = "Datum Value Description\nMHHW 13.80 Mean Higher-High Water\nMSL 8.73 Mean Sea Level\nMLLW 3.53 Mean Lower-Low Water\nNAVD88 9.03 North American Vertical Datum of 1988\n";
    let a = run(
        "navigation.tide.datums",
        json!({"sheet": table, "sheet_units": "ft", "height": "10 ft", "from": "MLLW", "to": "MSL"}),
    );
    let back = run(
        "navigation.tide.datums",
        json!({"sheet": table, "sheet_units": "ft", "height": format!("{} m", a["result"]["converted"]["value"]), "from": "MSL", "to": "MLLW"}),
    );
    assert!(
        (back["result"]["converted"]["value"].as_f64().unwrap() - 3.048).abs() < 1e-9,
        "{back}"
    );
    // A table without units is refused rather than guessed.
    let nounits = run(
        "navigation.tide.datums",
        json!({"sheet": table, "height": "1 m", "from": "MLLW", "to": "MSL"}),
    );
    assert_eq!(nounits["error"]["field"], "/sheet");
}

#[test]
fn node_factors_match_pytides_1990_to_2040() {
    // The spec's check: f and V0 + u for M2, S2, N2, K1, and O1, every year
    // 1990 to 2040, within 1e-6 and 0.001 degree of the reference
    // implementation (pytides2 0.0.5, fixture generated from its source).
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/tides");
    let f: Value =
        serde_json::from_str(&std::fs::read_to_string(format!("{dir}/pytides_fu.json")).unwrap())
            .unwrap();
    let rows = f["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 51 * 5);
    for r in rows {
        let (year, name) = (r[0].as_i64().unwrap(), r[1].as_str().unwrap());
        let (want_f, want_vu) = (r[2].as_f64().unwrap(), r[3].as_f64().unwrap());
        let (got_f, got_vu) = gp_navigation::tide::year_factors(name, year).unwrap();
        assert!(
            (got_f - want_f).abs() < 1e-6,
            "{year} {name} f {got_f} against {want_f}"
        );
        let d = (got_vu - want_vu + 540.0).rem_euclid(360.0) - 180.0;
        assert!(
            d.abs() < 0.001,
            "{year} {name} V0+u {got_vu} against {want_vu}"
        );
    }
}
