//! GPS visibility and DOP: the properties every answer must have.

use gp_survey::REGISTRY;
use gp_survey::dop::{EXAMPLE_INPUT, parse_almanac, parse_sem, parse_yuma};
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

/// The header and first two records of the Coast Guard's SEM almanac for the
/// same week 392, verbatim but for the record count, set to 2.
const NAVCEN_SEM: &str = "2  CURRENT.ALM
 392 61440

1
80
0
 2.05898284912109E-03  4.34875488281250E-03 -2.60843080468476E-09
 5.15368847656250E+03 -2.78079032897949E-01  5.07532358169556E-02
 7.67284631729126E-01  1.51634216308594E-04 -7.27595761418343E-12
0
12

2
61
0
 1.71442031860352E-02  5.81550598144531E-03 -2.61570676229894E-09
 5.15364355468750E+03 -3.29516768455505E-01 -2.21939921379089E-01
-8.24917197227478E-01  1.91688537597656E-04  7.27595761418343E-12
0
9

";

#[test]
fn a_sem_almanac_reads_as_the_same_orbits() {
    let (sem, yuma) = (parse_sem(NAVCEN_SEM).unwrap(), parse_yuma(NAVCEN).unwrap());
    assert_eq!(sem.len(), 2);
    for (s, y) in sem.iter().zip(&yuma) {
        assert_eq!(
            (s.prn, s.week, s.health, s.toa),
            (y.prn, y.week, y.health, y.toa)
        );
        // YUMA prints ten significant digits (nine decimals for the perigee);
        // SEM prints fifteen, in semicircles.
        for (a, b) in [
            (s.e, y.e),
            (s.i0, y.i0),
            (s.omega_dot, y.omega_dot),
            (s.sqrt_a, y.sqrt_a),
            (s.omega0, y.omega0),
            (s.w, y.w),
            (s.m0, y.m0),
        ] {
            assert!(
                (a - b).abs() <= 2e-8 * b.abs().max(1e-3),
                "PRN {}: {a} vs {b}",
                s.prn
            );
        }
    }
    // The format is told apart by its labels, and a short file is refused.
    assert_eq!(parse_almanac(NAVCEN_SEM).unwrap().len(), 2);
    assert_eq!(parse_almanac(NAVCEN).unwrap().len(), 2);
    let short = NAVCEN_SEM.replacen("2  CURRENT.ALM", "3  CURRENT.ALM", 1);
    assert!(parse_sem(&short).unwrap_err().contains("3 satellites"));
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
