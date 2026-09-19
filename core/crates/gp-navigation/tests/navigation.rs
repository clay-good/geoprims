//! Geodesic tools: catalog lint, examples, golden vectors, spec scenarios, and
//! a Karney-vs-Vincenty differential over random point pairs.

use std::path::Path;

use gp_base::{manifest, template, vectors};
use gp_navigation::{REGISTRY, TOOLS, vincenty};
use serde_json::Value;

fn repo(path: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(path);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

fn call(id: &str, input: &str) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, input)).expect("envelope is JSON")
}

fn num(r: &Value, path: &str) -> f64 {
    path.split('.')
        .fold(r, |v, k| &v[k])
        .as_f64()
        .unwrap_or_else(|| panic!("{path} missing in {r}"))
}

fn codes(r: &Value) -> Vec<String> {
    r["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap().to_owned())
        .collect()
}

const JFK_LHR: &str = r#"{"lat1":40.6413,"lon1":-73.7781,"lat2":51.47,"lon2":-0.4543,"options":{"outputUnits":{"distance":"m"}}}"#;

#[test]
fn catalog_lint() {
    let tax: Value = serde_json::from_str(&repo("data/taxonomy.json")).unwrap();
    let owned: Vec<(String, Vec<String>)> = tax["domains"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(d, v)| {
            (
                d.clone(),
                v["groups"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|g| g.as_str().unwrap().to_owned())
                    .collect(),
            )
        })
        .collect();
    let taxonomy: Vec<(&str, Vec<&str>)> = owned
        .iter()
        .map(|(d, g)| (d.as_str(), g.iter().map(String::as_str).collect()))
        .collect();
    let errs = manifest::lint(TOOLS, &taxonomy, &[]);
    assert!(errs.is_empty(), "{}", errs.join("\n"));
}

#[test]
fn examples_and_vectors() {
    let mut failures = Vec::new();
    for t in TOOLS {
        for ex in t.examples {
            let r = call(t.id, ex.input);
            let s = r["summary"].as_str().unwrap_or_default();
            if r["ok"] != true || template::grade(s) > 8.0 || s.len() > template::MAX_CHARS {
                failures.push(format!("{} example: {r}", t.id));
            }
        }
        let text = repo(&format!("core/vectors/{}.jsonl", t.id));
        failures.extend(vectors::lint(t.id, &text));
        failures.extend(vectors::run(&REGISTRY, t.id, &text));
        if vectors::count(&text) < 5 {
            failures.push(format!("{} has fewer than 5 vectors", t.id));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn jfk_to_lhr() {
    let r = call("navigation.geodesic.inverse", JFK_LHR);
    assert!((num(&r, "result.distance.value") - 5_554_908.791).abs() < 1e-3);
    assert!((num(&r, "result.azimuth1.value") - 51.381_647_9).abs() < 1e-7);
    assert!((num(&r, "result.azimuth2.value") - 107.982_829_1).abs() < 1e-7);
    let nm = call(
        "navigation.geodesic.inverse",
        r#"{"lat1":40.6413,"lon1":-73.7781,"lat2":51.47,"lon2":-0.4543,"options":{"profile":"aviation"}}"#,
    );
    assert_eq!(nm["result"]["distance"]["unit"], "NM");
    assert!((num(&nm, "result.distance.value") - 2_999.411).abs() < 1e-3);
}

#[test]
fn nearly_and_exactly_antipodal() {
    let r = call(
        "navigation.geodesic.inverse",
        r#"{"lat1":0,"lon1":0,"lat2":0.5,"lon2":179.7,"options":{"outputUnits":{"distance":"m"}}}"#,
    );
    assert!((num(&r, "result.distance.value") - 19_944_127.421).abs() < 1e-3);
    assert!((num(&r, "result.azimuth1.value") - 15.556_883).abs() < 1e-6);
    assert!(!codes(&r).iter().any(|c| c.contains("CONVERGE")));
    let r = call(
        "navigation.geodesic.inverse",
        r#"{"lat1":0,"lon1":0,"lat2":0,"lon2":180}"#,
    );
    assert!(codes(&r).contains(&"AZIMUTH_NOT_UNIQUE".to_owned()));
}

#[test]
fn coincident_points() {
    let r = call(
        "navigation.geodesic.inverse",
        r#"{"lat1":10,"lon1":10,"lat2":10,"lon2":10}"#,
    );
    assert_eq!(num(&r, "result.distance.value"), 0.0);
    assert!(codes(&r).contains(&"AZIMUTH_UNDEFINED".to_owned()));
}

#[test]
fn direct_1000_km() {
    let r = call(
        "navigation.geodesic.direct",
        r#"{"lat1":40.6413,"lon1":-73.7781,"azimuth":51,"distance":"1000000 m"}"#,
    );
    assert!((num(&r, "result.lat2.value") - 45.892_080_8).abs() < 1e-7);
    assert!((num(&r, "result.lon2.value") + 63.754_956_3).abs() < 1e-7);
    assert!((num(&r, "result.azimuth2.value") - 57.886_373).abs() < 1e-6);
}

#[test]
fn vincenty_honesty() {
    let r = call(
        "navigation.geodesic.vincenty-inverse",
        r#"{"lat1":0,"lon1":0,"lat2":0.5,"lon2":179.7}"#,
    );
    assert_eq!(r["error"]["code"], "DID_NOT_CONVERGE");
    assert!(
        r["error"]["hint"]
            .as_str()
            .unwrap()
            .contains("navigation.geodesic.inverse")
    );
    let r = call("navigation.geodesic.vincenty-inverse", JFK_LHR);
    let diff_mm = num(&r, "result.karney_difference.value");
    assert!(diff_mm.abs() < 1.0, "{diff_mm} mm");
}

#[test]
fn haversine_error_shown() {
    let r = call("navigation.geodesic.haversine", JFK_LHR);
    assert!((num(&r, "result.distance.value") - 5_540_019.0).abs() < 1.0);
    assert!((num(&r, "result.difference.value") - 14_890.0).abs() < 1.0);
    assert!((num(&r, "result.difference_percent") - 0.27).abs() < 0.005);
    assert!(r["summary"].as_str().unwrap().contains("shorter"));
}

#[test]
fn mars_and_high_flattening() {
    let r = call(
        "navigation.geodesic.inverse",
        r#"{"lat1":0,"lon1":0,"lat2":10,"lon2":10,"a":"3396190 m","inverse_flattening":169.894}"#,
    );
    let model = r["meta"]["model"].as_str().unwrap();
    assert!(
        model.contains("3396190") && model.contains("169.894"),
        "{model}"
    );
    let r = call(
        "navigation.geodesic.inverse",
        r#"{"lat1":0,"lon1":0,"lat2":10,"lon2":10,"a":"6378137 m","inverse_flattening":10}"#,
    );
    assert_eq!(r["error"]["code"], "UNSUPPORTED");
    let r = call(
        "navigation.geodesic.inverse",
        r#"{"lat1":0,"lon1":0,"lat2":10,"lon2":10,"a":"6378137 m"}"#,
    );
    assert_eq!(r["error"]["code"], "INVALID_INPUT");
}

#[test]
fn input_rules() {
    let r = call(
        "navigation.geodesic.inverse",
        r#"{"lat1":90.0000001,"lon1":0,"lat2":0,"lon2":0}"#,
    );
    assert_eq!(r["error"]["field"], "/lat1");
    let r = call(
        "navigation.geodesic.inverse",
        r#"{"lat1":0,"lon1":540.25,"lat2":0,"lon2":0}"#,
    );
    assert!(codes(&r).contains(&"INPUT_NORMALIZED".to_owned()));
}

/// Differential: Karney and Vincenty agree within 1 mm away from antipodes.
#[test]
fn karney_vs_vincenty_differential() {
    let mut s: u64 = 0x1234_5678_9ABC_DEF1;
    let mut rnd = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        (s >> 11) as f64 / (1u64 << 53) as f64
    };
    let g = geographiclib_rs::Geodesic::wgs84();
    let (a, f) = (6_378_137.0, 1.0 / 298.257_223_563);
    let mut worst: f64 = 0.0;
    for _ in 0..2000 {
        let (la1, lo1) = (rnd() * 178.0 - 89.0, rnd() * 360.0 - 180.0);
        let (la2, lo2) = (rnd() * 178.0 - 89.0, rnd() * 360.0 - 180.0);
        use geographiclib_rs::InverseGeodesic;
        let (k, _, _, _): (f64, f64, f64, f64) = g.inverse(la1, lo1, la2, lo2);
        if k > 19_000_000.0 {
            continue; // near-antipodal: Vincenty is not expected to converge
        }
        if let Some((v, _, _)) = vincenty::inverse(a, f, la1, lo1, la2, lo2) {
            worst = worst.max((v - k).abs());
        }
    }
    assert!(worst < 1e-3, "worst Karney-Vincenty difference {worst} m");
}

#[test]
fn geodtest_sample_matches_karney() {
    // 1,000 lines of GeographicLib's GeodTest-short.dat (high-precision
    // reference, 0.1 nm). Distances and direct positions must be within the
    // stated 15 nm everywhere. Azimuths, reduced length, and area are checked
    // away from nearly antipodal lines, where they are ill-conditioned in the
    // 12-decimal endpoints (GeographicLib's own C library differs there too).
    let text = include_str!("data/GeodTest-sample.dat");
    let adiff = |a: f64, b: f64| {
        let d = (a - b).rem_euclid(360.0);
        d.min(360.0 - d)
    };
    let (mut ds, mut dpos, mut daz, mut dm12, mut ds12) = (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for l in text.lines() {
        let f: Vec<f64> = l.split_whitespace().map(|x| x.parse().unwrap()).collect();
        let (lat1, lon1, azi1, lat2, lon2, azi2, s12, m12, area) =
            (f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[8], f[9]);
        let r = call(
            "navigation.geodesic.inverse",
            &format!(r#"{{"lat1":{lat1},"lon1":{lon1},"lat2":{lat2},"lon2":{lon2}}}"#),
        );
        ds = ds.max((num(&r, "result.distance.value") * 1000.0 - s12).abs());
        if s12 < 1.9e7 {
            daz = daz
                .max(adiff(num(&r, "result.azimuth1.value"), azi1))
                .max(adiff(num(&r, "result.azimuth2.value"), azi2));
            dm12 = dm12.max((num(&r, "result.reduced_length.value") - m12).abs());
            ds12 = ds12.max((num(&r, "result.area.value") - area).abs());
        }
        let d = call(
            "navigation.geodesic.direct",
            &format!(r#"{{"lat1":{lat1},"lon1":{lon1},"azimuth":{azi1},"distance":"{s12} m"}}"#),
        );
        let m_per_deg = 6_371_000.0f64.to_radians();
        let pos = ((num(&d, "result.lat2.value") - lat2) * m_per_deg)
            .hypot(adiff(num(&d, "result.lon2.value"), lon2) * m_per_deg * lat2.to_radians().cos());
        dpos = dpos.max(pos);
    }
    assert!(ds <= 15e-9, "distance {ds} m");
    assert!(dpos <= 15e-9, "direct position {dpos} m");
    assert!(daz <= 1e-8, "azimuth {daz}°");
    assert!(dm12 <= 1e-8, "reduced length {dm12} m");
    assert!(ds12 <= 0.1, "area {ds12} m²");
}

#[test]
fn geodesic_invariants() {
    // Symmetry, the direct problem inverting the inverse, and the triangle
    // inequality, over a spread of pairs including poles and the antimeridian.
    let pts: Vec<(f64, f64)> = (-90..=90)
        .step_by(30)
        .flat_map(|lat| {
            (-180..180)
                .step_by(75)
                .map(move |lon| (lat as f64 * 0.99 + 0.1, lon as f64 + 0.3))
        })
        .collect();
    let inv = |a: (f64, f64), b: (f64, f64)| {
        call(
            "navigation.geodesic.inverse",
            &format!(
                r#"{{"lat1":{},"lon1":{},"lat2":{},"lon2":{}}}"#,
                a.0, a.1, b.0, b.1
            ),
        )
    };
    for (i, &a) in pts.iter().enumerate() {
        for &b in &pts[i + 1..] {
            let ab = inv(a, b);
            let s = num(&ab, "result.distance.value");
            assert!(
                (s - num(&inv(b, a), "result.distance.value")).abs() < 1e-12,
                "{a:?} {b:?}"
            );
            let d = call(
                "navigation.geodesic.direct",
                &format!(
                    r#"{{"lat1":{},"lon1":{},"azimuth":{},"distance":"{} km"}}"#,
                    a.0,
                    a.1,
                    num(&ab, "result.azimuth1.value"),
                    s
                ),
            );
            let back = inv(
                b,
                (num(&d, "result.lat2.value"), num(&d, "result.lon2.value")),
            );
            assert!(
                num(&back, "result.distance.value") < 1e-9,
                "{a:?} {b:?} {back}"
            );
            let c = pts[(i * 7 + 3) % pts.len()];
            let via =
                num(&inv(a, c), "result.distance.value") + num(&inv(c, b), "result.distance.value");
            assert!(s <= via + 1e-9, "triangle {a:?} {b:?} {c:?}");
        }
    }
}

#[test]
fn cross_track_matches_a_brute_force_search() {
    // Independent reference: golden-section search along the geodesic from A
    // (via the direct problem) for the point nearest P.
    use geographiclib_rs::{DirectGeodesic, Geodesic, InverseGeodesic};
    let g = Geodesic::wgs84();
    let mut seed = 12345u64;
    let mut rnd = || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    let (mut worst_foot, mut worst_xt, mut worst_at) = (0.0f64, 0.0f64, 0.0f64);
    for _ in 0..200 {
        let (la1, lo1) = (rnd() * 140.0 - 70.0, rnd() * 360.0 - 180.0);
        let (az, len) = (rnd() * 360.0, 50e3 + rnd() * 2_000e3);
        let (la2, lo2): (f64, f64) = g.direct(la1, lo1, az, len);
        // P: somewhere along (or past) the line, then off to one side.
        let (fa, fo, faz): (f64, f64, f64) = g.direct(la1, lo1, az, len * (rnd() * 1.6 - 0.3));
        let (lat, lon): (f64, f64) = g.direct(
            fa,
            fo,
            faz + if rnd() < 0.5 { 90.0 } else { -90.0 },
            rnd() * 300e3,
        );
        let dist = |s: f64| -> f64 {
            let (x, y): (f64, f64) = g.direct(la1, lo1, az, s);
            g.inverse(x, y, lat, lon)
        };
        let (mut lo, mut hi) = (-0.5 * len, 1.5 * len);
        // A coarse bracket by golden-section search on distance...
        let phi = (5f64.sqrt() - 1.0) / 2.0;
        for _ in 0..200 {
            let (m1, m2) = (hi - phi * (hi - lo), lo + phi * (hi - lo));
            if dist(m1) < dist(m2) {
                hi = m2
            } else {
                lo = m1
            }
        }
        // Distance is flat at the minimum, so refine by bisecting on the
        // perpendicularity condition: the course to P is 90° off the line there.
        let ahead = |s: f64| -> f64 {
            let (x, y, line_az): (f64, f64, f64) = g.direct(la1, lo1, az, s);
            let (to_p, _, _): (f64, f64, f64) = g.inverse(x, y, lat, lon); // (azi1, azi2, a12)
            (to_p - line_az).to_radians().cos()
        };
        let mid = (lo + hi) / 2.0;
        let (mut a, mut b) = (mid - 1000.0, mid + 1000.0);
        for _ in 0..100 {
            let c = (a + b) / 2.0;
            if ahead(c) > 0.0 { a = c } else { b = c }
        }
        let s_best = (a + b) / 2.0;
        let (bx, by): (f64, f64) = g.direct(la1, lo1, az, s_best);
        let r = call(
            "navigation.route.cross-track",
            &format!(
                r#"{{"lat1":{la1},"lon1":{lo1},"lat2":{la2},"lon2":{lo2},"lat":{lat},"lon":{lon}}}"#
            ),
        );
        let (fx, fy) = (
            num(&r, "result.foot_lat.value"),
            num(&r, "result.foot_lon.value"),
        );
        let d_foot: f64 = g.inverse(fx, fy, bx, by);
        worst_foot = worst_foot.max(d_foot);
        worst_xt =
            worst_xt.max((num(&r, "result.cross_track.value").abs() * 1000.0 - dist(s_best)).abs());
        worst_at = worst_at.max((num(&r, "result.along_track.value") * 1000.0 - s_best).abs());
    }
    assert!(worst_foot < 1e-3, "foot {worst_foot} m");
    assert!(worst_xt < 1e-3, "cross-track {worst_xt} m");
    assert!(worst_at < 1e-3, "along-track {worst_at} m");
}

#[test]
fn cross_track_spec_scenarios() {
    // South-east of JFK-LHR near the start: right of course, within the segment.
    let r = call(
        "navigation.route.cross-track",
        r#"{"lat1":40.6413,"lon1":-73.7781,"lat2":51.47,"lon2":-0.4543,"lat":44,"lon":-60}"#,
    );
    assert!(num(&r, "result.cross_track.value") > 0.0, "{r}");
    assert_eq!(r["result"]["within"], "yes");
    // Past the end: flagged, with the distance to B.
    let past = call(
        "navigation.route.cross-track",
        r#"{"lat1":40.6413,"lon1":-73.7781,"lat2":51.47,"lon2":-0.4543,"lat":52,"lon":5}"#,
    );
    assert!(
        codes(&past).contains(&"FOOT_OUTSIDE_SEGMENT".to_owned()),
        "{past}"
    );
    assert!(num(&past, "result.along_track.value") > num(&past, "result.segment.value"));
    assert!(num(&past, "result.end_distance.value") > 0.0);
    // Left of course is negative; the spherical method stays within 1% here.
    let left = call(
        "navigation.route.cross-track",
        r#"{"lat1":40.6413,"lon1":-73.7781,"lat2":51.47,"lon2":-0.4543,"lat":50,"lon":-60}"#,
    );
    let sph = call(
        "navigation.route.cross-track",
        r#"{"lat1":40.6413,"lon1":-73.7781,"lat2":51.47,"lon2":-0.4543,"lat":50,"lon":-60,"method":"spherical"}"#,
    );
    let (e, s) = (
        num(&left, "result.cross_track.value"),
        num(&sph, "result.cross_track.value"),
    );
    assert!(e < 0.0 && s < 0.0, "{left} {sph}");
    assert!(((e - s) / e).abs() < 0.01, "{e} {s}");
    let same = call(
        "navigation.route.cross-track",
        r#"{"lat1":10,"lon1":10,"lat2":10,"lon2":10,"lat":11,"lon":11}"#,
    );
    assert_eq!(same["error"]["code"], "INVALID_INPUT");
}

#[test]
fn fly_by_tsd_and_cpa_scenarios() {
    // 90° fly-by at 120 kt and 25° of bank: radius and lead 833.4 m (±0.1 m).
    let r = call(
        "navigation.route.fly-by",
        r#"{"inbound":"360 deg","outbound":"090 deg","speed":"120 kt","bank":"25 deg"}"#,
    );
    assert!((num(&r, "result.radius.value") - 833.4).abs() < 0.1, "{r}");
    assert!((num(&r, "result.lead_distance.value") - 833.4).abs() < 0.1);
    assert_eq!(r["result"]["direction"], "right");
    // Standard rate by default: 18.24° of bank at 120 kt.
    let s = call(
        "navigation.route.fly-by",
        r#"{"inbound":"090 deg","outbound":"045 deg","speed":"120 kt"}"#,
    );
    assert!(
        (num(&s, "result.bank_used.value") - 18.24).abs() < 0.01,
        "{s}"
    );
    assert_eq!(s["result"]["direction"], "left");
    let sharp = call(
        "navigation.route.fly-by",
        r#"{"inbound":"000 deg","outbound":"150 deg","speed":"120 kt"}"#,
    );
    assert!(codes(&sharp).contains(&"FLY_OVER_RECOMMENDED".to_owned()));
    // 250 NM at 125 kt is 2 h 00 min; departing 14:30 at UTC-6 arrives 16:30, 22:30Z.
    let t = call(
        "navigation.route.time-speed-distance",
        r#"{"distance":"250 NM","speed":"125 kt","departure":"14:30","utc_offset":"-06:00"}"#,
    );
    assert_eq!(t["result"]["ete"], "2 h 00 min", "{t}");
    assert_eq!(t["result"]["eta"], "16:30");
    assert_eq!(t["result"]["eta_utc"], "22:30Z");
    let late = call(
        "navigation.route.time-speed-distance",
        r#"{"distance":"250 NM","speed":"125 kt","departure":"23:30","utc_offset":"+05:30"}"#,
    );
    assert_eq!(late["result"]["eta"], "01:30 (next day)", "{late}");
    assert_eq!(late["result"]["eta_utc"], "20:00Z");
    let speed = call(
        "navigation.route.time-speed-distance",
        r#"{"distance":"300 NM","time":"2.5 h"}"#,
    );
    assert!(
        (num(&speed, "result.speed.value") - 120.0).abs() < 1e-9,
        "{speed}"
    );
    let one = call(
        "navigation.route.time-speed-distance",
        r#"{"distance":"300 NM"}"#,
    );
    assert_eq!(one["error"]["code"], "INVALID_INPUT");
    // CPA: t = 110 s, separation 141.42 m.
    let c = call(
        "navigation.route.cpa",
        r#"{"a_course":"090 deg","a_speed":"10 m/s","b_east":"1000 m","b_north":"1200 m","b_course":"180 deg","b_speed":"10 m/s"}"#,
    );
    assert!((num(&c, "result.time.value") - 110.0).abs() < 1e-9, "{c}");
    assert!((num(&c, "result.separation.value") - 141.421_356).abs() < 1e-5);
    let away = call(
        "navigation.route.cpa",
        r#"{"a_course":"270 deg","a_speed":"10 m/s","b_east":"1000 m","b_north":"0 m","b_course":"090 deg","b_speed":"10 m/s"}"#,
    );
    assert!(codes(&away).contains(&"DIVERGING".to_owned()), "{away}");
    assert!((num(&away, "result.separation.value") - 1000.0).abs() < 1e-9);
}

#[test]
fn route_legs_scenario() {
    // Four waypoints with a date: every leg has true and magnetic courses and
    // the declination; totals and times add up.
    let r = call(
        "navigation.route.legs",
        r#"{"waypoints":[{"name":"KDEN","lat":39.8617,"lon":-104.6731},{"name":"KASE","lat":39.2232,"lon":-106.8688},{"name":"KGJT","lat":39.1224,"lon":-108.5267},{"name":"KDEN","lat":39.8617,"lon":-104.6731}],"date":"2026-09-18","groundspeed":"120 kt","departure":"09:00","utc_offset":"-06:00"}"#,
    );
    let legs = r["result"]["legs"]
        .as_array()
        .unwrap_or_else(|| panic!("{r}"));
    assert_eq!(legs.len(), 3);
    let mut sum = 0.0;
    for leg in legs {
        let tc = leg["true_course"]["value"].as_f64().unwrap();
        let d = leg["declination"]["value"].as_f64().unwrap();
        let mc = leg["magnetic_course"]["value"].as_f64().unwrap();
        assert!(
            ((tc - d - mc).rem_euclid(360.0)).min((mc - tc + d).rem_euclid(360.0)) < 1e-9,
            "{leg}"
        );
        assert!(
            (5.0..10.0).contains(&d),
            "Colorado declination is about 7-8° east: {d}"
        );
        sum += leg["distance"]["value"].as_f64().unwrap();
        assert_eq!(leg["distance"]["unit"], "NM");
    }
    assert!((num(&r, "result.total_distance.value") - sum).abs() < 1e-9);
    assert_eq!(r["meta"]["assets"][0]["id"], "wmm2025");
    assert!(r["result"]["arrival_utc"].as_str().unwrap().ends_with('Z'));
    // The first leg matches the geodesic tool.
    let g = call(
        "navigation.geodesic.inverse",
        r#"{"lat1":39.8617,"lon1":-104.6731,"lat2":39.2232,"lon2":-106.8688}"#,
    );
    assert!(
        (legs[0]["distance"]["value"].as_f64().unwrap() * 1.852 - num(&g, "result.distance.value"))
            .abs()
            < 1e-9
    );
    // Without a date: true courses only.
    let t = call(
        "navigation.route.legs",
        r#"{"waypoints":[{"lat":0,"lon":0},{"lat":0,"lon":1}],"path":"rhumb"}"#,
    );
    assert!(
        t["result"]["legs"][0].get("magnetic_course").is_none(),
        "{t}"
    );
    assert_eq!(t["result"]["legs"][0]["from"], "WP1");
    assert!(
        (t["result"]["legs"][0]["true_course"]["value"]
            .as_f64()
            .unwrap()
            - 90.0)
            .abs()
            < 1e-9
    );
}

#[test]
fn line_of_sight_scenarios() {
    // 100 m: geometric 35.70 km, optical (k = 0.13) 38.27 km, radio 41.22 km (±0.01 km).
    let h = call("navigation.los.horizon", r#"{"height":"100 m"}"#);
    for (k, want) in [("geometric", 35.70), ("optical", 38.27), ("radio", 41.22)] {
        assert!(
            (num(&h, &format!("result.{k}.value")) - want).abs() < 0.01,
            "{k} {h}"
        );
    }
    assert!(codes(&h).contains(&"TERRAIN_NOT_CONSIDERED".to_owned()));
    // From 2 m, 30 km away: horizon 5.41 km, 41.3 m of the target hidden (±0.1 m).
    let v = call(
        "navigation.los.visibility",
        r#"{"observer_height":"2 m","target_height":"50 m","distance":"30 km"}"#,
    );
    assert!(
        (num(&v, "result.observer_horizon.value") - 5.41).abs() < 0.01,
        "{v}"
    );
    assert!(
        (num(&v, "result.hidden_height.value") - 41.3).abs() < 0.1,
        "{v}"
    );
    assert_eq!(v["result"]["visible"], "yes", "a 50 m target peeks over");
    let far = call(
        "navigation.los.visibility",
        r#"{"observer_height":"2 m","target_height":"10 m","distance":"30 km"}"#,
    );
    assert_eq!(far["result"]["visible"], "no");
    assert!(num(&far, "result.midpoint_clearance.value") < 0.0, "{far}");
    // Dip at 10 m in arcminutes, with the 1.76′√h rule beside it.
    let d = call("navigation.los.dip", r#"{"height":"10 m"}"#);
    assert_eq!(d["result"]["dip"]["unit"], "arcmin");
    assert!((num(&d, "result.dip.value") - 5.68).abs() < 0.01, "{d}");
    assert!((num(&d, "result.rule.value") - 1.76 * 10f64.sqrt()).abs() < 1e-9);
    // 5.8 GHz over 10 km at the midpoint: Fresnel 11.37 m, bulge 1.47 m (K = 4/3).
    let f = call(
        "navigation.los.fresnel",
        r#"{"frequency":"5.8 GHz","distance":"10 km"}"#,
    );
    assert!(
        (num(&f, "result.fresnel_radius.value") - 11.37).abs() < 0.01,
        "{f}"
    );
    assert!((num(&f, "result.earth_bulge.value") - 1.47).abs() < 0.01);
    let sum = 0.6 * num(&f, "result.fresnel_radius.value") + num(&f, "result.earth_bulge.value");
    assert!((num(&f, "result.required_clearance.value") - sum).abs() < 1e-9);
}

#[test]
fn waypoints_and_closest_point_scenarios() {
    use geographiclib_rs::{Geodesic, InverseGeodesic};
    let g = Geodesic::wgs84();
    // JFK-LHR in 10 intervals: 11 points, each 555,490.879 m apart along the geodesic.
    let r = call(
        "navigation.geodesic.waypoints",
        r#"{"lat1":40.6413,"lon1":-73.7781,"lat2":51.47,"lon2":-0.4543,"intervals":10}"#,
    );
    let pts = r["result"]["points"]
        .as_array()
        .unwrap_or_else(|| panic!("{r}"));
    assert_eq!(pts.len(), 11);
    for w in pts.windows(2) {
        let p = |x: &Value| {
            (
                x["lat"]["value"].as_f64().unwrap(),
                x["lon"]["value"].as_f64().unwrap(),
            )
        };
        let (a, b) = (p(&w[0]), p(&w[1]));
        let d: f64 = g.inverse(a.0, a.1, b.0, b.1);
        assert!((d - 555_490.879).abs() < 1e-3, "{d}");
    }
    let geo: Value = serde_json::from_str(r["result"]["geojson"].as_str().unwrap()).unwrap();
    assert_eq!(geo["geometry"]["coordinates"].as_array().unwrap().len(), 11);
    assert!(
        r["result"]["gpx"]
            .as_str()
            .unwrap()
            .matches("<rtept")
            .count()
            == 11
    );
    // Spacing: the last interval is shorter; fractions: exactly those points.
    let s = call(
        "navigation.geodesic.waypoints",
        r#"{"lat1":0,"lon1":0,"lat2":0,"lon2":10,"spacing":"400 km"}"#,
    );
    assert_eq!(s["result"]["count"], 4, "{s}");
    let f = call(
        "navigation.geodesic.waypoints",
        r#"{"lat1":0,"lon1":0,"lat2":0,"lon2":10,"fractions":"0.5"}"#,
    );
    assert!(
        (f["result"]["points"][0]["lon"]["value"].as_f64().unwrap() - 5.0).abs() < 1e-9,
        "{f}"
    );
    let two = call(
        "navigation.geodesic.waypoints",
        r#"{"lat1":0,"lon1":0,"lat2":0,"lon2":10,"intervals":5,"spacing":"1 km"}"#,
    );
    assert_eq!(two["error"]["code"], "INVALID_INPUT");
    let huge = call(
        "navigation.geodesic.waypoints",
        r#"{"lat1":0,"lon1":0,"lat2":0,"lon2":10,"spacing":"1 m"}"#,
    );
    assert_eq!(huge["error"]["code"], "LIMIT_EXCEEDED");
    // Beside leg 3 of a five-leg route: leg 3, along-route past the first two legs.
    let route = r#"[{"lat":40,"lon":-105},{"lat":40,"lon":-104},{"lat":41,"lon":-104},{"lat":41,"lon":-103},{"lat":40,"lon":-103},{"lat":40,"lon":-102}]"#;
    let c = call(
        "navigation.route.closest-point",
        &format!(r#"{{"route":{route},"lat":41.1,"lon":-103.5}}"#),
    );
    assert_eq!(c["result"]["leg"], 3, "{c}");
    let first_two: f64 = {
        let a: f64 = g.inverse(40.0, -105.0, 40.0, -104.0);
        let b: f64 = g.inverse(40.0, -104.0, 41.0, -104.0);
        a + b
    };
    let along = num(&c, "result.along_route.value") * 1000.0;
    assert!(
        along > first_two && along < first_two + 90_000.0,
        "{along} {first_two}"
    );
    assert!(
        num(&c, "result.cross_track.value") < 0.0,
        "north of an eastbound leg is left: {c}"
    );
    // Past the end of the route, the last waypoint is closest.
    let end = call(
        "navigation.route.closest-point",
        &format!(r#"{{"route":{route},"lat":40,"lon":-101}}"#),
    );
    assert_eq!(end["result"]["leg"], 5);
    assert!(
        (num(&end, "result.along_route.value") - num(&end, "result.route_length.value")).abs()
            < 1e-9
    );
}
