//! The focused COGO operations: what each must do, as their derivation notes
//! state.

use gp_survey::REGISTRY;
use serde_json::{Value, json};

fn call(id: &str, input: Value) -> Value {
    let r: Value = serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).unwrap();
    assert_eq!(r["ok"], true, "{id} {input}: {r}");
    r["result"].clone()
}

fn num(r: &Value, k: &str) -> f64 {
    r[k]["value"]
        .as_f64()
        .or(r[k].as_f64())
        .unwrap_or_else(|| panic!("{k}: {r}"))
}

fn azimuth(direction: &str, reference: &str) -> f64 {
    num(
        &call(
            "survey.cogo.azimuth-bearing",
            json!({"direction": direction, "reference": reference}),
        ),
        "azimuth",
    )
}

#[test]
fn azimuth_bearing_invariants() {
    // Quadrant boundaries name the same direction from either side.
    for (a, b, want) in [
        ("N 0°00'00\" E", "N 0°00'00\" W", 0.0),
        ("N 90°00'00\" E", "S 90°00'00\" E", 90.0),
        ("S 0°00'00\" E", "S 0°00'00\" W", 180.0),
        ("S 90°00'00\" W", "N 90°00'00\" W", 270.0),
    ] {
        assert!((azimuth(a, "assumed") - want).abs() < 1e-9, "{a}");
        assert!((azimuth(b, "assumed") - want).abs() < 1e-9, "{b}");
    }
    // 0 gon and 400 gon are both north.
    assert_eq!(azimuth("0 gon", "assumed"), 0.0);
    assert_eq!(azimuth("400 gon", "assumed"), 0.0);
    // The reference is a label; the number does not move.
    for d in ["N 10°15'30\" W", "S 45°00'00\" E", "123.456 deg"] {
        let a = azimuth(d, "assumed");
        for r in ["grid", "geodetic"] {
            assert_eq!(azimuth(d, r), a, "{d} {r}");
        }
    }
}

/// A line from (5000, 5000) at each azimuth, as a direction or a second point.
fn lines() -> Vec<(f64, Value, Value)> {
    [0.0, 37.5, 90.0, 200.0, 315.25]
        .iter()
        .map(|&az: &f64| {
            let base = json!({"start_northing": "5000 ft", "start_easting": "5000 ft", "start_station": "10+00"});
            let mut by_dir = base.clone();
            by_dir["direction"] = json!(az.to_string());
            let mut by_end = base;
            let r = az.to_radians();
            by_end["end_northing"] = json!(format!("{} ft", 5000.0 + 800.0 * r.cos()));
            by_end["end_easting"] = json!(format!("{} ft", 5000.0 + 800.0 * r.sin()));
            (az, by_dir, by_end)
        })
        .collect()
}

#[test]
fn station_offset_views_invariants() {
    for (az, by_dir, by_end) in lines() {
        let (d_e, d_n) = (az.to_radians().sin(), az.to_radians().cos());
        for (station, offset) in [("10+00", 0.0), ("13+50", -12.5), ("17+25.5", 40.0)] {
            // Point from station and offset: the point less its foot is the
            // offset along the right-hand normal.
            let mut input = by_dir.clone();
            input["station"] = json!(station);
            input["offset"] = json!(format!("{offset} ft"));
            let p = call("survey.cogo.point-from-station-offset", input);
            let (pe, pn) = (num(&p, "easting"), num(&p, "northing"));
            let (fe, fn_) = (num(&p, "foot_easting"), num(&p, "foot_northing"));
            assert!((pe - fe - offset * d_n).abs() < 1e-9, "{az} {station}");
            assert!((pn - fn_ + offset * d_e).abs() < 1e-9, "{az} {station}");
            // A second point on the ray gives the same answer.
            let mut by_point = by_end.clone();
            by_point["station"] = json!(station);
            by_point["offset"] = json!(format!("{offset} ft"));
            let q = call("survey.cogo.point-from-station-offset", by_point);
            assert!(
                (num(&q, "easting") - pe).abs() < 1e-6 && (num(&q, "northing") - pn).abs() < 1e-6
            );

            // Perpendicular foot: back to the same station and offset, with
            // the foot-to-point vector perpendicular to the line.
            for line in [&by_dir, &by_end] {
                let mut back = line.clone();
                back["northing"] = json!(format!("{pn} ft"));
                back["easting"] = json!(format!("{pe} ft"));
                let f = call("survey.cogo.perpendicular-foot", back);
                assert!((num(&f, "offset") - offset).abs() < 1e-6, "{az} {station}");
                let s = f["station"].as_str().unwrap().replace('+', "");
                let want = station.replace('+', "");
                assert!(
                    (s.parse::<f64>().unwrap() - want.parse::<f64>().unwrap()).abs() < 1e-6,
                    "{s} vs {want}"
                );
                let (ve, vn) = (pe - num(&f, "foot_easting"), pn - num(&f, "foot_northing"));
                assert!(
                    (ve * d_e + vn * d_n).abs() < 1e-6,
                    "{az}: not perpendicular"
                );
            }
        }
    }
}
