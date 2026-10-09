//! GPS visibility and DOP: the properties every answer must have.

use gp_survey::REGISTRY;
use gp_survey::dop::{EXAMPLE_INPUT, parse_yuma};
use serde_json::{Value, json};

fn example() -> Value {
    serde_json::from_str(EXAMPLE_INPUT).unwrap()
}

fn run(input: &Value) -> Value {
    let r: Value =
        serde_json::from_str(&REGISTRY.invoke("survey.gnss.dop", &input.to_string())).unwrap();
    assert_eq!(r["ok"], true, "{r}");
    r["result"].clone()
}

fn with(changes: Value) -> Value {
    let mut v = example();
    for (k, x) in changes.as_object().unwrap() {
        v[k] = x.clone();
    }
    v
}

fn records(text: &str) -> Vec<String> {
    text.split("********")
        .skip(1)
        .collect::<Vec<_>>()
        .chunks(2)
        .filter(|c| c.len() == 2)
        .map(|c| format!("********{}********{}", c[0], c[1]))
        .collect()
}

/// The first two records of the Coast Guard's YUMA almanac for week 392
/// (published October 2026), verbatim.
const NAVCEN: &str = "******** Week 392 almanac for PRN-01 ********
ID:                         01
Health:                     000
Eccentricity:               0.2058982849E-002
Time of Applicability(s):  61440.0000
Orbital Inclination(rad):   0.9561398125
Rate of Right Ascen(r/s):  -0.8194627053E-008
SQRT(A)  (m 1/2):           5153.688477
Right Ascen at Week(rad):  -0.8736110469E+000
Argument of Perigee(rad):   0.159445993
Mean Anom(rad):             0.2410495762E+001
Af0(s):                     0.1516342163E-003
Af1(s/s):                  -0.7275957614E-011
week:                        392

******** Week 392 almanac for PRN-02 ********
ID:                         02
Health:                     000
Eccentricity:               0.1714420319E-001
Time of Applicability(s):  61440.0000
Orbital Inclination(rad):   0.9607477469
Rate of Right Ascen(r/s):  -0.8217485148E-008
SQRT(A)  (m 1/2):           5153.643555
Right Ascen at Week(rad):  -0.1035207459E+001
Argument of Perigee(rad):  -0.697244827
Mean Anom(rad):            -0.2591553807E+001
Af0(s):                     0.1916885376E-003
Af1(s/s):                   0.7275957614E-011
week:                        392
";

#[test]
fn a_coast_guard_almanac_parses() {
    let a = parse_yuma(NAVCEN).unwrap();
    assert_eq!(a.len(), 2);
    assert_eq!((a[0].prn, a[0].week, a[0].health), (1, 392, 0));
    assert_eq!(a[0].e, 0.2058982849e-2);
    assert_eq!(a[1].omega0, -0.1035207459e1);
    assert_eq!(a[1].w, -0.697244827);
    assert_eq!(a[1].toa, 61440.0);
    assert!(parse_yuma("nothing here").is_err());
    let twice = format!("{NAVCEN}\n{}", &NAVCEN[..NAVCEN.find("\n\n").unwrap()]);
    assert!(
        parse_yuma(&twice)
            .unwrap_err()
            .contains("PRN 1 appears twice")
    );
    let broken = NAVCEN.replace("Mean Anom(rad):            -0.2591553807E+001\n", "");
    assert!(parse_yuma(&broken).unwrap_err().contains("PRN 2"));
}

#[test]
fn the_same_instant_in_any_offset_gives_the_same_sky() {
    let utc = run(&example());
    let local = run(&with(json!({"start": "2026-10-03T08:00-06:00"})));
    assert_eq!(utc["satellites"], local["satellites"]);
    assert_eq!(utc["timeline"], local["timeline"]);
}

#[test]
fn record_order_and_week_form_do_not_matter() {
    let base = run(&example());
    let text = example()["almanac"].as_str().unwrap().to_owned();
    let mut recs = records(&text);
    recs.reverse();
    let reversed = run(&with(json!({"almanac": recs.concat()})));
    assert_eq!(base["timeline"], reversed["timeline"]);
    // The ten-bit week 390 and the full week 2438 are the same week here.
    let full = text.replace(
        "week:                        390",
        "week:                        2438",
    );
    assert_ne!(full, text);
    assert_eq!(
        base["timeline"],
        run(&with(json!({"almanac": full})))["timeline"]
    );
}

#[test]
fn a_higher_mask_never_helps() {
    let mut last: Option<(f64, Option<f64>)> = None;
    for mask in [0, 5, 10, 15, 20, 30] {
        let r = run(&with(
            json!({"mask": format!("{mask} deg"), "duration": "0 h"}),
        ));
        let n = r["visible"].as_f64().unwrap();
        let p = r["pdop"].as_f64();
        if let Some((n0, p0)) = last {
            assert!(n <= n0, "mask {mask}: more satellites");
            if let (Some(p), Some(p0)) = (p, p0) {
                assert!(p >= p0 - 1e-12, "mask {mask}: PDOP improved");
            }
        }
        last = Some((n, p));
    }
}

#[test]
fn a_skyline_under_the_mask_changes_nothing() {
    let base = run(&example());
    let low = run(&with(json!({"horizon": [
        {"azimuth": "0 deg", "elevation": "5 deg"},
        {"azimuth": "180 deg", "elevation": "8 deg"}
    ]})));
    assert_eq!(base["timeline"], low["timeline"]);
    // A wall to the south leaves out exactly the satellites behind it.
    let wall = run(&with(json!({"horizon": [
        {"azimuth": "90 deg", "elevation": "0 deg"},
        {"azimuth": "91 deg", "elevation": "89 deg"},
        {"azimuth": "269 deg", "elevation": "89 deg"},
        {"azimuth": "270 deg", "elevation": "0 deg"}
    ]})));
    for s in wall["satellites"].as_array().unwrap() {
        let az = s["azimuth"]["value"].as_f64().unwrap();
        let el = s["elevation"]["value"].as_f64().unwrap();
        if (95.0..265.0).contains(&az) && el < 85.0 {
            assert_eq!(s["used"], "no", "{s}");
        }
    }
    assert!(wall["visible"].as_f64() < base["visible"].as_f64());
}

#[test]
fn unhealthy_satellites_are_left_out() {
    let base = run(&example());
    let text = example()["almanac"].as_str().unwrap().to_owned();
    let sick = text.replacen(
        "Health:                     000",
        "Health:                     063",
        1,
    );
    let r = run(&with(json!({"almanac": sick})));
    assert_eq!(
        r["healthy"].as_f64().unwrap(),
        base["healthy"].as_f64().unwrap() - 1.0
    );
}

#[test]
fn an_old_almanac_is_flagged() {
    let r: Value = serde_json::from_str(&REGISTRY.invoke(
        "survey.gnss.dop",
        &with(json!({"start": "2026-10-20T14:00Z", "duration": "0 h"})).to_string(),
    ))
    .unwrap();
    let codes: Vec<&str> = r["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap())
        .collect();
    assert!(codes.contains(&"ALMANAC_OLD"), "{codes:?}");
    let fresh = REGISTRY.invoke("survey.gnss.dop", &example().to_string());
    assert!(!fresh.contains("ALMANAC_OLD"));
}
