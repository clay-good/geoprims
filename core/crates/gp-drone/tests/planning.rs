//! Mission planning (drone/mission-planning spec): sorties and battery swaps,
//! wind at flying height, visual line of sight over a mission, and ground
//! control with checkpoints. Scenarios, and checks that the composed tools
//! agree with the tools they are built from.

use gp_drone::REGISTRY;
use serde_json::Value;

fn call(id: &str, input: &str) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, input)).expect("envelope is JSON")
}

fn num(r: &Value, path: &str) -> f64 {
    path.split('.')
        .fold(r, |v, k| match v {
            Value::Array(a) => &a[k.parse::<usize>().unwrap()],
            _ => &v[k],
        })
        .as_f64()
        .unwrap_or_else(|| panic!("{path} missing in {r}"))
}

fn codes(r: &Value) -> Vec<String> {
    r["meta"]["warnings"].as_array().map_or(vec![], |a| {
        a.iter()
            .map(|w| w["code"].as_str().unwrap().to_owned())
            .collect()
    })
}

/// A serpentine of `lines` east-west lines 0.0018° apart and `dlon` long,
/// starting 0.0009° north of (40, -105), as JSON rows.
fn grid(lines: usize, dlon: f64) -> String {
    let mut pts = Vec::new();
    for k in 0..lines {
        let la = 40.0009 + k as f64 * 0.0018;
        let (a, b) = if k % 2 == 0 {
            (-105.0, -105.0 + dlon)
        } else {
            (-105.0 + dlon, -105.0)
        };
        pts.push(format!(r#"{{"lat":{la},"lon":{a}}}"#));
        pts.push(format!(r#"{{"lat":{la},"lon":{b}}}"#));
    }
    format!("[{}]", pts.join(","))
}

#[test]
fn sorties_hand_worked_three() {
    let r = call(
        "drone.mission.sorties",
        &format!(
            r#"{{"lat":40,"lon":-105,"waypoints":{},"flight_time":"7 min","groundspeed":"10 m/s","reserve":20}}"#,
            grid(4, 0.01408)
        ),
    );
    assert_eq!(r["ok"], true, "{r}");
    assert_eq!(num(&r, "result.batteries"), 3.0);
    let swaps: Vec<f64> = r["result"]["swap_points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["waypoint"].as_f64().unwrap())
        .collect();
    assert_eq!(swaps, [4.0, 7.0]);
    // Every sortie fits the battery less the reserve: 7 min × 0.8.
    for s in r["result"]["sorties"].as_array().unwrap() {
        let t = s["flying_time"]["value"].as_f64().unwrap()
            + s["return_time"]["value"].as_f64().unwrap();
        assert!(t <= 5.6 + 1e-9, "{s}");
    }
    assert_eq!(
        r["summary"],
        "The mission takes 3 batteries and about 16 min of flying."
    );
}

#[test]
fn sorties_one_battery_when_it_fits() {
    let r = call(
        "drone.mission.sorties",
        &format!(
            r#"{{"lat":40,"lon":-105,"waypoints":{},"flight_time":"30 min","groundspeed":"10 m/s","reserve":20}}"#,
            grid(4, 0.01408)
        ),
    );
    assert_eq!(num(&r, "result.batteries"), 1.0);
    assert_eq!(r["result"]["swap_points"].as_array().unwrap().len(), 0);
}

#[test]
fn sorties_far_ends_are_shorter() {
    // A grid whose far end is 2 km from home, on 20 min batteries: a sortie
    // that ends far out must save more for the trip home, so it flies less.
    let r = call(
        "drone.mission.sorties",
        &format!(
            r#"{{"lat":40,"lon":-105,"waypoints":{},"flight_time":"20 min","groundspeed":"6 m/s","transit_speed":"12 m/s","reserve":20}}"#,
            grid(12, 0.0235)
        ),
    );
    assert_eq!(r["ok"], true, "{r}");
    let s = r["result"]["sorties"].as_array().unwrap();
    assert!(s.len() >= 3, "{r}");
    let full: Vec<(f64, f64)> = s[..s.len() - 1]
        .iter()
        .map(|x| {
            (
                x["return_time"]["value"].as_f64().unwrap(),
                x["flying_time"]["value"].as_f64().unwrap(),
            )
        })
        .collect();
    let far = full
        .iter()
        .copied()
        .fold((f64::MIN, 0.0), |a, b| if b.0 > a.0 { b } else { a });
    let near = full
        .iter()
        .copied()
        .fold((f64::MAX, 0.0), |a, b| if b.0 < a.0 { b } else { a });
    assert!(
        far.0 > near.0 && far.1 < near.1,
        "far {far:?} near {near:?}"
    );
}

#[test]
fn sorties_waypoint_out_of_range() {
    let r = call(
        "drone.mission.sorties",
        r#"{"lat":40,"lon":-105,"waypoints":[{"lat":40.0009,"lon":-105},{"lat":40.0009,"lon":-104.8}],"flight_time":"10 min","groundspeed":"10 m/s","reserve":20}"#,
    );
    assert_eq!(r["ok"], false);
    assert_eq!(r["error"]["field"], "/waypoints/1");
    let msg = r["error"]["message"].as_str().unwrap();
    assert!(
        msg.contains("Waypoint 2") && msg.contains(" m from home"),
        "{msg}"
    );
    let first = call(
        "drone.mission.sorties",
        r#"{"lat":40,"lon":-105,"waypoints":[{"lat":40.1,"lon":-105}],"flight_time":"10 min","groundspeed":"10 m/s"}"#,
    );
    assert_eq!(first["error"]["field"], "/waypoints/0", "{first}");
}

#[test]
fn sorties_compose_endurance_and_rth() {
    // The battery time is the endurance tool's cruise time to the reserve,
    // and every trip home is the return-to-home tool's, bit for bit.
    let r = call(
        "drone.mission.sorties",
        &format!(
            r#"{{"lat":40,"lon":-105,"waypoints":{},"energy":"90 Wh","usable":80,"power":"250 W","groundspeed":"8 m/s","transit_speed":"14 m/s","headwind":"4 m/s","reserve":20}}"#,
            grid(6, 0.0235)
        ),
    );
    assert_eq!(r["ok"], true, "{r}");
    let e = call(
        "drone.power.endurance",
        r#"{"energy":"90 Wh","usable":80,"power":"250 W","reserve":20}"#,
    );
    assert_eq!(
        num(&r, "result.battery_time.value"),
        num(&e, "result.hover_time.value")
    );
    for s in r["result"]["sorties"].as_array().unwrap() {
        let last = s["last_waypoint"].as_f64().unwrap() as usize - 1;
        let wp = &r["result"]["path"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p["part"] == s["sortie"])
            .nth(1 + last - (s["first_waypoint"].as_f64().unwrap() as usize - 1))
            .unwrap()
            .clone();
        let (la, lo) = (
            wp["lat"]["value"].as_f64().unwrap(),
            wp["lon"]["value"].as_f64().unwrap(),
        );
        let dist = geodesic(40.0, -105.0, la, lo);
        let rth = call(
            "drone.power.rth-budget",
            &format!(
                r#"{{"distance":"{dist} m","airspeed":"14 m/s","headwind":"4 m/s","power":"250 W","remaining_energy":"90 Wh"}}"#
            ),
        );
        assert!(
            (num(&rth, "result.return_time.value") - s["return_time"]["value"].as_f64().unwrap())
                .abs()
                < 1e-9,
            "{s} {rth}"
        );
        assert!(
            (num(&rth, "result.return_energy.value")
                - s["return_energy"]["value"].as_f64().unwrap())
            .abs()
                < 1e-9
        );
    }
}

fn geodesic(a: f64, b: f64, c: f64, d: f64) -> f64 {
    use geographiclib_rs::{Geodesic, InverseGeodesic};
    Geodesic::wgs84().inverse(a, b, c, d)
}

#[test]
fn wind_unchanged_at_report_height() {
    for terrain in ["open", "water", "woodland"] {
        let r = call(
            "drone.ops.wind-limit",
            &format!(
                r#"{{"wind_speed":"8 m/s","gust":"11 m/s","flying_height":"10 m","terrain":"{terrain}","wind_rating":"12 m/s"}}"#
            ),
        );
        assert_eq!(num(&r, "result.wind_at_height.value"), 8.0);
        assert_eq!(num(&r, "result.gust_at_height.value"), 11.0);
    }
}

#[test]
fn wind_gust_over_rating() {
    let r = call(
        "drone.ops.wind-limit",
        r#"{"wind_speed":"15 kt","gust":"25 kt","flying_height":"120 m","wind_rating":"12 m/s"}"#,
    );
    assert!(codes(&r).contains(&"GUST_EXCEEDS_RATING".to_owned()));
    assert!(!codes(&r).contains(&"WIND_EXCEEDS_RATING".to_owned()));
    let w = r["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["code"] == "GUST_EXCEEDS_RATING")
        .unwrap()["message"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(
        w.contains("18.3 m/s") && w.contains("11 m/s") && w.contains("12 m/s"),
        "{w}"
    );
    let s = r["summary"].as_str().unwrap();
    assert!(s.contains("not modeled"), "{s}");
    assert!(r["result"]["note"].as_str().unwrap().contains("0.143"));
}

#[test]
fn wind_rises_with_height_and_roughness() {
    let at = |h: f64, t: &str| {
        num(
            &call(
                "drone.ops.wind-limit",
                &format!(
                    r#"{{"wind_speed":"6 m/s","flying_height":"{h} m","terrain":"{t}","wind_rating":"15 m/s"}}"#
                ),
            ),
            "result.wind_at_height.value",
        )
    };
    let mut n = 0;
    for w in ["water", "open", "crops", "hedges", "suburbs", "woodland"].windows(2) {
        for h in [20.0, 60.0, 120.0] {
            assert!(at(h, w[1]) > at(h, w[0]));
            assert!(at(h * 2.0, w[0]) > at(h, w[0]));
            n += 1;
        }
    }
    assert_eq!(n, 15);
}

#[test]
fn vlos_check_ring_and_dimension() {
    let r = call(
        "drone.ops.vlos-check",
        r#"{"lat":40,"lon":-105,"waypoints":[{"lat":40.0009,"lon":-105},{"lat":40.0009,"lon":-104.98592}],"characteristic_dimension":"0.35 m"}"#,
    );
    let v = call(
        "drone.sensors.vlos",
        r#"{"characteristic_dimension":"0.35 m"}"#,
    );
    assert_eq!(
        num(&r, "result.visual_range_used.value"),
        num(&v, "result.vlos.value")
    );
    assert_eq!(num(&r, "result.beyond_count"), 1.0);
    let notice = r["result"]["notice"].as_str().unwrap();
    assert!(
        notice.contains("Not legal advice") && notice.contains("section-107.31"),
        "{notice}"
    );
    assert!(
        r["summary"]
            .as_str()
            .unwrap()
            .contains("your call on the day")
    );
    // The ring is at the range: its first point is due north by that distance.
    let ring = r["result"]["range_ring"].as_array().unwrap();
    assert_eq!(ring.len(), 72);
    let d = geodesic(
        40.0,
        -105.0,
        ring[0]["lat"]["value"].as_f64().unwrap(),
        ring[0]["lon"]["value"].as_f64().unwrap(),
    );
    assert!((d - num(&v, "result.vlos.value")).abs() < 1e-6);
}

/// Even-odd ray test in latitude and longitude, written apart from the core's.
fn in_ring(ring: &[(f64, f64)], p: (f64, f64)) -> bool {
    let mut c = false;
    for i in 0..ring.len() {
        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
        if (a.0 > p.0) != (b.0 > p.0) && p.1 < a.1 + (p.0 - a.0) * (b.1 - a.1) / (b.0 - a.0) {
            c = !c;
        }
    }
    c
}

#[test]
fn gcp_points_inside_an_l_shape() {
    // An L: 300 m by 100 m along the bottom, 150 m by 200 m up the left.
    let l = [
        (40.0, -105.0),
        (40.0, -104.99648),
        (40.0009, -104.99648),
        (40.0009, -104.99824),
        (40.0027, -104.99824),
        (40.0027, -105.0),
    ];
    let rows: Vec<String> = l
        .iter()
        .map(|(a, b)| format!(r#"{{"lat":{a},"lon":{b}}}"#))
        .collect();
    let mut reached = 0;
    for spacing in ["", r#","spacing":"60 m""#, r#","spacing":"25 m""#] {
        let r = call(
            "drone.photogrammetry.gcp-plan",
            &format!(
                r#"{{"area":[{}],"horizontal_class":"5 cm"{spacing}}}"#,
                rows.join(",")
            ),
        );
        assert_eq!(r["ok"], true, "{r}");
        assert_eq!(num(&r, "result.checkpoints"), 30.0);
        let cps = r["result"]["checkpoint_points"].as_array().unwrap();
        assert_eq!(cps.len(), 30);
        let gcps = r["result"]["gcp_points"].as_array().unwrap();
        assert!(gcps.len() >= 6, "{r}");
        for p in gcps.iter().chain(cps) {
            let q = (
                p["lat"]["value"].as_f64().unwrap(),
                p["lon"]["value"].as_f64().unwrap(),
            );
            assert!(in_ring(&l, q), "{p} outside");
            // Not in the notch (the missing upper-right block).
            assert!(!(q.0 > 40.0009 && q.1 > -104.99824), "{p} in the notch");
            reached += 1;
        }
    }
    assert!(reached > 100, "{reached}");
}

#[test]
fn gcp_holes_are_outside() {
    let r = call(
        "drone.photogrammetry.gcp-plan",
        r#"{"area":[{"lat":40,"lon":-105},{"lat":40,"lon":-104.994},{"lat":40.005,"lon":-104.994},{"lat":40.005,"lon":-105},{"lat":40.002,"lon":-104.998,"ring":1},{"lat":40.002,"lon":-104.996,"ring":1},{"lat":40.003,"lon":-104.996,"ring":1},{"lat":40.003,"lon":-104.998,"ring":1}],"spacing":"40 m"}"#,
    );
    assert_eq!(r["ok"], true, "{r}");
    for p in r["result"]["gcp_points"]
        .as_array()
        .unwrap()
        .iter()
        .chain(r["result"]["checkpoint_points"].as_array().unwrap())
    {
        let (la, lo) = (
            p["lat"]["value"].as_f64().unwrap(),
            p["lon"]["value"].as_f64().unwrap(),
        );
        assert!(
            !(la > 40.002 && la < 40.003 && lo > -104.998 && lo < -104.996),
            "{p} in the hole"
        );
    }
}

#[test]
fn gcp_table_c1_rows() {
    // ASPRS Edition 2, Table C.1, row by row, in whole km².
    use gp_drone::gcp::table_c1;
    let rows = [
        (0.01, 30),
        (500.0, 30),
        (500.6, 35),
        (750.0, 35),
        (751.0, 40),
        (1000.0, 40),
        (1250.0, 45),
        (1251.0, 50),
        (1750.0, 55),
        (2000.0, 60),
        (2250.0, 65),
        (2500.0, 70),
    ];
    for (a, n) in rows {
        assert_eq!(table_c1(a), Some(n), "{a}");
    }
    assert_eq!(table_c1(2501.0), None);
}
