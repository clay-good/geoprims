//! The flight-planning tools (add-flight-and-drone-planning-tools): the nav
//! log, the climb plan, and the equal time point and point of no return,
//! against the spec scenarios and the tools they compose.

use gp_aviation::REGISTRY;
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

const LOG: &str = "aviation.flight-plan.nav-log";
const HGS: &str = "aviation.wind.heading-groundspeed";

/// Every leg's heading, groundspeed, and wind correction are
/// heading-groundspeed's, to the last bit, from courses as given and as the
/// geodesic makes them.
#[test]
fn nav_log_legs_match_heading_groundspeed_bit_for_bit() {
    let winds = [
        (0.0, 0.0),
        (30.0, 20.0),
        (275.5, 33.3),
        (181.0, 47.0),
        (359.9, 12.0),
    ];
    let mut checked = 0;
    for (k, &(wd, ws)) in winds.iter().enumerate() {
        let tas = 95.0 + 17.0 * k as f64;
        let courses: Vec<String> = (0..12)
            .map(|i| {
                format!(
                    r#"{{"course":"{} deg","distance":"10 NM"}}"#,
                    i as f64 * 30.7 + 0.3
                )
            })
            .collect();
        let by_course = format!(
            r#"{{"legs":[{}],"tas":"{tas} kt","wind_direction":"{wd} deg","wind_speed":"{ws} kt"}}"#,
            courses.join(",")
        );
        let by_waypoints = format!(
            r#"{{"waypoints":[{{"lat":35,"lon":-98}},{{"lat":36.2,"lon":-97.1}},{{"lat":35.4,"lon":-95}},{{"lat":34,"lon":-96.5}},{{"lat":50,"lon":179.5}},{{"lat":50.3,"lon":-179}}],"tas":"{tas} kt","wind_direction":"{wd} deg","wind_speed":"{ws} kt"}}"#
        );
        for input in [by_course, by_waypoints] {
            let log = call(LOG, &input);
            assert_eq!(log["ok"], true, "{log}");
            for leg in log["result"]["legs"].as_array().unwrap() {
                let course = leg["true_course"]["value"].as_f64().unwrap();
                let one = call(
                    HGS,
                    &format!(
                        r#"{{"course":{},"tas":"{tas} kt","wind_direction":"{wd} deg","wind_speed":"{ws} kt"}}"#,
                        serde_json::to_string(&course).unwrap()
                    ),
                );
                for (a, b) in [
                    ("true_heading", "heading"),
                    ("groundspeed", "groundspeed"),
                    ("wind_correction_angle", "wind_correction_angle"),
                ] {
                    let x = leg[a]["value"].as_f64().unwrap();
                    let y = one["result"][b]["value"].as_f64().unwrap();
                    assert_eq!(
                        x.to_bits(),
                        y.to_bits(),
                        "{a}: {x} vs {y} at course {course}"
                    );
                }
                checked += 1;
            }
        }
    }
    assert_eq!(checked, winds.len() * (12 + 5));
}

#[test]
fn nav_log_spec_leg_heading() {
    let r = call(
        LOG,
        r#"{"legs":[{"course":"090 deg","distance":"50 NM"}],"tas":"120 kt","wind_direction":"030 deg","wind_speed":"20 kt"}"#,
    );
    assert!((num(&r, "result.legs.0.true_heading.value") - 81.7).abs() < 0.05);
    assert!((num(&r, "result.legs.0.groundspeed.value") - 108.7).abs() < 0.05);
}

#[test]
fn nav_log_wind_stronger_than_tas_drops_the_totals() {
    let r = call(
        LOG,
        r#"{"legs":[{"course":"000 deg","distance":"20 NM"},{"course":"090 deg","distance":"20 NM"}],"tas":"30 kt","wind_direction":"000 deg","wind_speed":"40 kt"}"#,
    );
    assert_eq!(r["ok"], true, "{r}");
    let w = r["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|w| w["code"] == "WIND_EXCEEDS_TAS")
        .collect::<Vec<_>>();
    // Leg 1 is a headwind above the airspeed, leg 2 a crosswind above it.
    assert_eq!(w.len(), 2, "{r}");
    assert_eq!(w[0]["field"], "/legs/0");
    assert!(w[1]["message"].as_str().unwrap().starts_with("Leg 2"));
    assert!(r["result"].get("total_time").is_none());
    assert!(r["result"]["legs"][0].get("true_heading").is_none());
    assert_eq!(num(&r, "result.total_distance.value"), 40.0);
}

#[test]
fn nav_log_crosses_the_antimeridian_the_short_way() {
    let r = call(
        LOG,
        r#"{"waypoints":[{"lat":50,"lon":170},{"lat":50,"lon":-170}],"tas":"120 kt"}"#,
    );
    let d = num(&r, "result.legs.0.distance.value");
    let tc = num(&r, "result.legs.0.true_course.value");
    // The short way is about 770 NM eastbound; the long way would be over 12,000.
    assert!((700.0..800.0).contains(&d), "{d}");
    assert!((0.0..90.0).contains(&tc), "{tc}");
    assert_eq!(r["result"]["legs_count"], 1);
    assert_eq!(r["result"]["path"].as_array().unwrap().len(), 2);
}

#[test]
fn nav_log_single_leg_and_wmm_variation() {
    let r = call(
        LOG,
        r#"{"waypoints":[{"lat":39.8617,"lon":-104.6731},{"lat":39.2232,"lon":-106.8688}],"tas":"120 kt","date":"2026-09-18","fuel_burn":"10 gal/h"}"#,
    );
    assert_eq!(r["ok"], true, "{r}");
    let var = num(&r, "result.legs.0.variation.value");
    // Denver's declination is about 7.5° E in 2026.
    assert!((6.5..8.5).contains(&var), "{var}");
    let th = num(&r, "result.legs.0.true_heading.value");
    let mh = num(&r, "result.legs.0.magnetic_heading.value");
    assert!(((th - var).rem_euclid(360.0) - mh).abs() < 1e-12);
    assert!(
        (num(&r, "result.total_fuel.value") - num(&r, "result.total_time.value") / 6.0).abs()
            < 1e-12
    );
    assert_eq!(r["meta"]["assets"][0]["id"], "wmm2025");
}

const CLIMB: &str = "aviation.performance.climb-plan";

#[test]
fn climb_plan_field_at_cruise_needs_no_climb() {
    let r = call(
        CLIMB,
        r#"{"field_elevation":"5500 ft","cruise_altitude":"5500 ft","climb_rate":"500 fpm","climb_tas":"90 kt","climb_burn":"11 gal/h"}"#,
    );
    assert_eq!(num(&r, "result.time.value"), 0.0);
    assert_eq!(num(&r, "result.top_of_climb.value"), 0.0);
    assert_eq!(num(&r, "result.fuel.value"), 0.0);
    assert!(r["result"]["note"].as_str().unwrap().contains("No climb"));
    assert!(
        r["summary"]
            .as_str()
            .unwrap()
            .starts_with("No climb is needed")
    );
}

#[test]
fn climb_plan_groundspeed_is_the_wind_triangles() {
    let r = call(
        CLIMB,
        r#"{"field_elevation":"0 ft","cruise_altitude":"4000 ft","climb_rate":"500 fpm","climb_tas":"90 kt","course":"031 deg","wind_direction":"360 deg","wind_speed":"10 kt"}"#,
    );
    let one = call(
        HGS,
        r#"{"course":"031 deg","tas":"90 kt","wind_direction":"360 deg","wind_speed":"10 kt"}"#,
    );
    let gs = num(&r, "result.groundspeed.value");
    assert_eq!(
        gs.to_bits(),
        num(&one, "result.groundspeed.value").to_bits()
    );
    assert_eq!(num(&r, "result.time.value"), 8.0);
    assert!((num(&r, "result.top_of_climb.value") - gs * 8.0 / 60.0).abs() < 1e-12);
}

const ETP: &str = "aviation.performance.etp-pnr";

#[test]
fn etp_with_no_wind_is_exactly_halfway() {
    for d in [1.0, 37.3, 906.0, 2_417.9] {
        let r = call(ETP, &format!(r#"{{"distance":"{d} NM","tas":"137 kt"}}"#));
        assert_eq!(num(&r, "result.etp_distance.value"), d / 2.0, "{r}");
    }
}

#[test]
fn etp_moves_toward_destination_in_a_headwind_out() {
    let r = call(
        ETP,
        r#"{"distance":"500 NM","tas":"150 kt","course":"270 deg","wind_direction":"270 deg","wind_speed":"30 kt"}"#,
    );
    assert!(num(&r, "result.etp_distance.value") > 250.0);
    let tail = call(
        ETP,
        r#"{"distance":"500 NM","tas":"150 kt","course":"090 deg","wind_direction":"270 deg","wind_speed":"30 kt"}"#,
    );
    assert!(num(&tail, "result.etp_distance.value") < 250.0);
}

#[test]
fn pnr_leaves_the_reserve_on_return() {
    // 50 gal with 10 gal reserve at 10 gal/h: 4 h to fly. Out at 100 kt and
    // back at 140 kt, the time out and back must use exactly those 4 h.
    let r = call(
        ETP,
        r#"{"distance":"200 NM","groundspeed_out":"100 kt","groundspeed_back":"140 kt","usable_fuel":"50 gal","reserve_fuel":"10 gal","fuel_burn":"10 gal/h"}"#,
    );
    let t = num(&r, "result.pnr_time.value") / 60.0;
    let d = num(&r, "result.pnr_distance.value");
    assert!((t + d / 140.0 - 4.0).abs() < 1e-12, "{r}");
    assert!((d - 100.0 * t).abs() < 1e-12);
}
