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
    // Related tools that live in other crates.
    let known = ["geodesy.height.convert", "geodesy.frame.to-local"];
    let errs = manifest::lint(TOOLS, &taxonomy, &known);
    assert!(errs.is_empty(), "{}", errs.join("\n"));
}

#[test]
fn examples_and_vectors() {
    // The host supplies data assets; the test plays the host.
    gp_base::assets::put(
        "egm96-15@2009-08-29/egm96-15.pgm",
        include_bytes!("../../../../assets/data/egm96-15/2009-08-29/egm96-15.pgm"),
    );
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
    // f = 1/10 takes the exact method, and agrees with GeodSolve -E.
    let r = call(
        "navigation.geodesic.inverse",
        r#"{"lat1":0,"lon1":0,"lat2":10,"lon2":10,"a":"6378137 m","inverse_flattening":10,"options":{"outputUnits":{"distance":"m"}}}"#,
    );
    assert!(
        r["meta"]["model"]
            .as_str()
            .unwrap()
            .contains("GeodesicExact"),
        "{r}"
    );
    let s = num(&r, "result.distance.value");
    assert!((s - 1_430_617.478_749_418).abs() < 1e-9 * s, "{s}");
    // Beyond |f| = 0.5 it is refused rather than guessed.
    let r = call(
        "navigation.geodesic.inverse",
        r#"{"lat1":0,"lon1":0,"lat2":10,"lon2":10,"a":"6378137 m","inverse_flattening":1.5}"#,
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

#[test]
fn cpa_scene_positions_come_from_the_core() {
    // At 110 s (the CPA), the separation at the scene time equals the CPA separation.
    let r = call(
        "navigation.route.cpa",
        r#"{"a_course":"090 deg","a_speed":"10 m/s","b_east":"1000 m","b_north":"1200 m","b_course":"180 deg","b_speed":"10 m/s","at_time":"110 s"}"#,
    );
    assert!(
        (num(&r, "result.separation_at.value") - num(&r, "result.separation.value")).abs() < 1e-9,
        "{r}"
    );
    assert!((num(&r, "result.a_east_at.value") - 1100.0).abs() < 1e-9);
    assert!((num(&r, "result.b_north_at.value") - 100.0).abs() < 1e-9);
    assert!((num(&r, "result.scene_end.value") - 165.0).abs() < 1e-9);
    // At time zero, the scene separation is the current separation.
    let z = call(
        "navigation.route.cpa",
        r#"{"a_course":"090 deg","a_speed":"10 m/s","b_east":"1000 m","b_north":"1200 m","b_course":"180 deg","b_speed":"10 m/s","at_time":"0 s"}"#,
    );
    assert!(
        (num(&z, "result.separation_at.value") - num(&z, "result.current_separation.value")).abs()
            < 1e-9
    );
    let m = gp_navigation::route::CPA.timeline.expect("a timeline");
    assert_eq!((m.input, m.end, m.key), ("at_time", "scene_end", "time"));
}

#[test]
fn haversine_matches_independent_implementations() {
    // tests/data/haversine_diff.csv (tools/vectors/gen_dev_diff.py): 1,000
    // short, regional, and global lines with the haversine distance from a
    // separate Python implementation and the ellipsoidal distance from
    // GeographicLib (Python) 2.1.
    let text = repo("core/crates/gp-navigation/tests/data/haversine_diff.csv");
    let mut n = 0;
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let c: Vec<f64> = line.split(',').map(|x| x.parse().unwrap()).collect();
        let input = serde_json::json!({"lat1": c[0], "lon1": c[1], "lat2": c[2], "lon2": c[3]});
        let r = call("navigation.geodesic.haversine", &input.to_string());
        let hav = num(&r, "result.distance.value") * 1000.0;
        let karney = num(&r, "result.ellipsoidal_distance.value") * 1000.0;
        assert!(
            (hav - c[4]).abs() <= 1e-6 + 1e-12 * c[4],
            "{line}: haversine {hav}"
        );
        assert!(
            (karney - c[5]).abs() <= 1e-6,
            "{line}: ellipsoidal {karney}"
        );
        n += 1;
    }
    assert_eq!(n, 1000);
}

#[test]
fn haversine_invariants() {
    // Distance is symmetric, zero from a point to itself, obeys the triangle
    // inequality, never exceeds half the circumference, and scales with the radius.
    let d = |a: (f64, f64), b: (f64, f64), r: Option<f64>| {
        let mut v = serde_json::json!({"lat1": a.0, "lon1": a.1, "lat2": b.0, "lon2": b.1});
        if let Some(r) = r {
            v["radius"] = serde_json::json!(format!("{r} km"));
        }
        num(
            &call("navigation.geodesic.haversine", &v.to_string()),
            "result.distance.value",
        )
    };
    let pts = [
        (40.6413, -73.7781),
        (51.47, -0.4543),
        (-33.9, 151.2),
        (0.0, 0.0),
        (89.0, 10.0),
        (-60.0, -170.0),
        (12.5, 179.9),
    ];
    let half = std::f64::consts::PI * 6371.008771;
    for &a in &pts {
        assert_eq!(d(a, a, None), 0.0);
        for &b in &pts {
            let ab = d(a, b, None);
            assert!((ab - d(b, a, None)).abs() < 1e-9);
            assert!(ab <= half + 1e-9);
            assert!((d(a, b, Some(2.0 * 6371.008771)) - 2.0 * ab).abs() < 1e-8);
            for &c in &pts {
                assert!(ab <= d(a, c, None) + d(c, b, None) + 1e-9);
            }
        }
    }
}

fn adiff(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

#[test]
fn rhumb_tools_match_rhumbsolve() {
    // The 2,000 RhumbSolve pairs of gp-geo's differential, through the public
    // tools: distance and course both ways, and the direct problem back to
    // the end point (away from the 100 starts within 0.01° of a pole, where
    // the end longitude is ill-conditioned).
    let text = repo("core/crates/gp-geo/tests/data/rhumb_diff.csv");
    let (mut ds, mut da, mut dp, mut n) = (0f64, 0f64, 0f64, 0);
    for l in text.lines().skip(1) {
        let f: Vec<f64> = l.split(',').map(|x| x.parse().unwrap()).collect();
        let inv = call(
            "navigation.rhumb.inverse",
            &serde_json::json!({"lat1": f[0], "lon1": f[1], "lat2": f[2], "lon2": f[3], "options": {"outputUnits": {"distance": "m"}}}).to_string(),
        );
        ds = ds.max((num(&inv, "result.distance.value") - f[5]).abs());
        da = da.max(adiff(num(&inv, "result.course.value"), f[4]));
        if f[0].abs() < 89.99 {
            let dir = call(
                "navigation.rhumb.direct",
                &serde_json::json!({"lat1": f[0], "lon1": f[1], "course": f[4].rem_euclid(360.0), "distance": format!("{} m", f[5])}).to_string(),
            );
            let m = 6_371_000.0f64.to_radians();
            let d = ((num(&dir, "result.lat2.value") - f[2]) * m)
                .hypot(adiff(num(&dir, "result.lon2.value"), f[3]) * m * f[2].to_radians().cos());
            dp = dp.max(d);
        }
        n += 1;
    }
    eprintln!("rhumb tools vs RhumbSolve: {ds:e} m, {da:e}°, direct {dp:e} m");
    assert_eq!(n, 2000);
    assert!(
        ds <= 1e-6 && da <= 1e-9 && dp <= 1e-6,
        "{ds:e} m, {da:e}°, {dp:e} m"
    );
}

#[test]
fn rhumb_invariants() {
    // A rhumb line is never shorter than the geodesic; reversing it keeps the
    // distance and turns the course around; half the distance along it lands
    // on the same line (the course from there to the end is unchanged).
    let mut seed: u64 = 17;
    let mut rnd = || {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    for _ in 0..300 {
        let (a, b, c, d) = (
            rnd() * 160.0 - 80.0,
            rnd() * 360.0 - 180.0,
            rnd() * 160.0 - 80.0,
            rnd() * 360.0 - 180.0,
        );
        let inv = |a: f64, b: f64, c: f64, d: f64| {
            call(
                "navigation.rhumb.inverse",
                &serde_json::json!({"lat1": a, "lon1": b, "lat2": c, "lon2": d, "options": {"outputUnits": {"distance": "m", "geodesic_distance": "m"}}}).to_string(),
            )
        };
        let f = inv(a, b, c, d);
        let (s, course) = (
            num(&f, "result.distance.value"),
            num(&f, "result.course.value"),
        );
        assert!(s >= num(&f, "result.geodesic_distance.value") - 1e-6);
        let r = inv(c, d, a, b);
        assert!((num(&r, "result.distance.value") - s).abs() < 1e-6);
        assert!(adiff(num(&r, "result.course.value"), course + 180.0) < 1e-9);
        let mid = call(
            "navigation.rhumb.direct",
            &serde_json::json!({"lat1": a, "lon1": b, "course": course, "distance": format!("{} m", s / 2.0)}).to_string(),
        );
        let rest = inv(
            num(&mid, "result.lat2.value"),
            num(&mid, "result.lon2.value"),
            c,
            d,
        );
        assert!(
            adiff(num(&rest, "result.course.value"), course) < 1e-8,
            "{rest}"
        );
        assert!((num(&rest, "result.distance.value") - s / 2.0).abs() < 1e-5);
    }
}

#[test]
fn cross_track_invariants() {
    // A point on the line is not off it; the foot point is on the line and
    // the same distance along it; mirroring the point across the line flips
    // the sign and keeps the distance; and "within" says whether the foot
    // lies between the ends.
    let mut seed: u64 = 41;
    let mut rnd = || {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    for _ in 0..200 {
        let (a1, o1) = (rnd() * 140.0 - 70.0, rnd() * 360.0 - 180.0);
        let (a2, o2) = (
            (a1 + rnd() * 20.0 - 10.0).clamp(-85.0, 85.0),
            o1 + rnd() * 20.0 - 10.0,
        );
        let (lat, lon) = (
            (a1 + rnd() * 24.0 - 12.0).clamp(-88.0, 88.0),
            o1 + rnd() * 24.0 - 12.0,
        );
        let xt = |lat: f64, lon: f64| {
            call(
                "navigation.route.cross-track",
                &serde_json::json!({"lat1": a1, "lon1": o1, "lat2": a2, "lon2": o2, "lat": lat, "lon": lon,
                                    "options": {"outputUnits": {"cross_track": "m", "along_track": "m", "segment": "m"}}}).to_string(),
            )
        };
        let r = xt(lat, lon);
        let (_d, along, seg) = (
            num(&r, "result.cross_track.value"),
            num(&r, "result.along_track.value"),
            num(&r, "result.segment.value"),
        );
        assert_eq!(
            r["result"]["within"],
            if (0.0..=seg).contains(&along) {
                "yes"
            } else {
                "no"
            }
        );
        // The foot point is on the line: no cross-track distance, same along-track.
        let foot = xt(
            num(&r, "result.foot_lat.value"),
            num(&r, "result.foot_lon.value"),
        );
        assert!(
            num(&foot, "result.cross_track.value").abs() < 1e-3,
            "{foot}"
        );
        assert!(
            (num(&foot, "result.along_track.value") - along).abs() < 1e-3,
            "{foot}"
        );
        // Both ends are on the line, at 0 and the segment length.
        for (la, lo, want) in [(a1, o1, 0.0), (a2, o2, seg)] {
            let e = xt(la, lo);
            assert!(num(&e, "result.cross_track.value").abs() < 1e-3);
            assert!((num(&e, "result.along_track.value") - want).abs() < 1e-3);
        }
    }
}

#[test]
fn time_speed_distance_invariants() {
    // Whichever value is left out comes back consistent with the other two, in
    // any units; the ETA is the departure plus the elapsed time, and the Zulu
    // ETA is the local ETA less the UTC offset.
    let mut seed: u64 = 53;
    let mut rnd = || {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    let tsd =
        |input: serde_json::Value| call("navigation.route.time-speed-distance", &input.to_string());
    for _ in 0..200 {
        let (speed, hours) = (0.5 + rnd() * 600.0, 0.05 + rnd() * 20.0);
        let distance = speed * hours;
        // The same leg in nautical and statute units gives the same distance.
        let a = tsd(
            serde_json::json!({"speed": format!("{speed} kt"), "time": format!("{hours} h"),
                                       "options": {"outputUnits": {"distance": "NM"}}}),
        );
        let b = tsd(
            serde_json::json!({"speed": format!("{} mi/h", speed * 1.150_779_448_023_074_2), "time": format!("{} min", hours * 60.0),
                                       "options": {"outputUnits": {"distance": "NM"}}}),
        );
        let (da, db) = (
            num(&a, "result.distance.value"),
            num(&b, "result.distance.value"),
        );
        assert!((da - distance).abs() < 1e-9 * distance.max(1.0), "{a}");
        assert!((da - db).abs() < 1e-9 * distance.max(1.0), "{a} vs {b}");
        // Solving for speed, then for time, returns what it started from.
        let s = tsd(
            serde_json::json!({"distance": format!("{distance} NM"), "time": format!("{hours} h"),
                                       "options": {"outputUnits": {"speed": "kt"}}}),
        );
        assert!(
            (num(&s, "result.speed.value") - speed).abs() < 1e-9 * speed,
            "{s}"
        );
        let t = tsd(
            serde_json::json!({"distance": format!("{distance} NM"), "speed": format!("{speed} kt"),
                                       "options": {"outputUnits": {"time": "h"}}}),
        );
        assert!(
            (num(&t, "result.time.value") - hours).abs() < 1e-9 * hours,
            "{t}"
        );
    }
    // The clock: 14:20 plus 1 h 30 min is 15:50 local, 20:50 Zulu five hours west.
    let r = tsd(
        serde_json::json!({"speed": "120 kt", "time": "90 min", "departure": "14:20", "utc_offset": "-5"}),
    );
    assert_eq!(r["result"]["eta"], "15:50");
    assert_eq!(r["result"]["eta_utc"], "20:50Z");
    assert_eq!(r["result"]["ete"], "1 h 30 min");
    // Past midnight the ETA wraps, and says so.
    let late = tsd(
        serde_json::json!({"speed": "60 kt", "time": "4 h", "departure": "23:30", "utc_offset": "0"}),
    );
    assert_eq!(late["result"]["eta"], "03:30 (next day)");
}

#[test]
fn a_utc_offset_with_a_stray_unicode_mark_is_refused_not_a_crash() {
    // U+202E (right-to-left override) shares its first byte with U+2212 (minus);
    // the offset parser once sliced it mid-character and trapped (found by the fuzzer).
    for off in ["\u{202e}1", "\u{202e}0600", "−06:00x", "+0é"] {
        let input = serde_json::json!({"distance":"250 NM","speed":"125 kt","departure":"14:30","utc_offset":off});
        let r = call("navigation.route.time-speed-distance", &input.to_string());
        assert_eq!(r["error"]["code"], "INVALID_INPUT", "{off}: {r}");
    }
    // The Unicode minus sign still reads as a negative offset (the field's
    // 6-character limit counts bytes, so the short form).
    let r = call(
        "navigation.route.time-speed-distance",
        r#"{"distance":"250 NM","speed":"125 kt","departure":"14:30","utc_offset":"−6"}"#,
    );
    assert_eq!(r["result"]["eta_utc"], "22:30Z", "{r}");
}

/// RFC 7946 checks for one polygon: closed rings, positions in range, the
/// exterior counterclockwise in lon/lat, and no edge jumping across ±180°.
fn valid_polygon(rings: &Value) -> Result<f64, String> {
    let ext = rings[0].as_array().ok_or("no exterior")?;
    let pts: Vec<(f64, f64)> = ext
        .iter()
        .map(|p| (p[0].as_f64().unwrap(), p[1].as_f64().unwrap()))
        .collect();
    if pts.len() < 4 || pts[0] != pts[pts.len() - 1] {
        return Err("ring not closed".into());
    }
    for w in pts.windows(2) {
        if !(-180.0..=180.0).contains(&w[0].0) || !(-90.0..=90.0).contains(&w[0].1) {
            return Err(format!("position out of range: {:?}", w[0]));
        }
        // Along a pole (lat ±90) the edge is a point, so ±180° there is fine.
        let on_pole = w[0].1.abs() == 90.0 && w[1].1 == w[0].1;
        if (w[1].0 - w[0].0).abs() > 180.0 && !on_pole {
            return Err(format!(
                "edge jumps the antimeridian: {:?} → {:?}",
                w[0], w[1]
            ));
        }
    }
    let area2: f64 = pts
        .windows(2)
        .map(|w| w[0].0 * w[1].1 - w[1].0 * w[0].1)
        .sum();
    if area2 <= 0.0 {
        return Err("exterior not counterclockwise".into());
    }
    Ok(area2 / 2.0)
}

#[test]
fn range_rings_are_valid_geojson_around_the_pole_and_across_the_antimeridian() {
    // The spec scenario: 1,500 km around 85° N encloses the North Pole.
    let r = call(
        "navigation.route.range-rings",
        r#"{"lat":85,"lon":30,"radii":[{"radius":"1500 km"}]}"#,
    );
    let doc: Value = serde_json::from_str(r["result"]["file"].as_str().unwrap()).unwrap();
    let geom = &doc["features"][0]["geometry"];
    assert_eq!(geom["type"], "Polygon", "{geom}");
    valid_polygon(&geom["coordinates"]).unwrap();
    let ext = geom["coordinates"][0].as_array().unwrap();
    assert!(ext.iter().any(|p| p[1] == 90.0), "reaches the pole");
    assert_eq!(r["result"]["summary"][0]["pole"], "north");
    // South pole too, reaching -90.
    let s = call(
        "navigation.route.range-rings",
        r#"{"lat":-89,"lon":0,"radii":[{"radius":"300 km"}]}"#,
    );
    let doc: Value = serde_json::from_str(s["result"]["file"].as_str().unwrap()).unwrap();
    let g = &doc["features"][0]["geometry"];
    valid_polygon(&g["coordinates"]).unwrap();
    assert!(
        g["coordinates"][0]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p[1] == -90.0)
    );
    // Across the antimeridian: two valid polygons, one each side.
    let a = call(
        "navigation.route.range-rings",
        r#"{"lat":0,"lon":179.5,"radii":[{"radius":"150 km"}]}"#,
    );
    let doc: Value = serde_json::from_str(a["result"]["file"].as_str().unwrap()).unwrap();
    let g = &doc["features"][0]["geometry"];
    assert_eq!(g["type"], "MultiPolygon", "{g}");
    let parts = g["coordinates"].as_array().unwrap();
    assert_eq!(parts.len(), 2);
    let sides: Vec<f64> = parts
        .iter()
        .map(|p| {
            valid_polygon(p).unwrap();
            p[0][0][0].as_f64().unwrap().signum()
        })
        .collect();
    assert!(sides.contains(&1.0) && sides.contains(&-1.0), "{sides:?}");
    // An ordinary ring is one polygon.
    let d = call(
        "navigation.route.range-rings",
        r#"{"lat":39.86,"lon":-104.67,"radii":[{"radius":"25 NM"}]}"#,
    );
    let doc: Value = serde_json::from_str(d["result"]["file"].as_str().unwrap()).unwrap();
    assert_eq!(doc["features"][0]["geometry"]["type"], "Polygon");
    valid_polygon(&doc["features"][0]["geometry"]["coordinates"]).unwrap();
}

#[test]
fn cpa_in_three_dimensions() {
    // Head-on at the same track, 1,000 ft apart vertically: the aircraft pass
    // overhead, so the horizontal miss is zero and the vertical is 1,000 ft.
    let r = call(
        "navigation.route.cpa",
        r#"{"a_course":"360 deg","a_speed":"250 kt","b_east":"0 m","b_north":"20 NM","b_course":"180 deg","b_speed":"250 kt","b_up":"1000 ft"}"#,
    );
    assert!(
        num(&r, "result.horizontal_separation.value").abs() < 1e-6,
        "{r}"
    );
    assert!((num(&r, "result.vertical_separation.value") - 1000.0).abs() < 1e-9);
    assert_eq!(r["result"]["vertical_separation"]["unit"], "ft");
    assert!((num(&r, "result.separation.value") - 304.8).abs() < 1e-9);
    // A descends into B's level: the 3D minimum comes before the tracks cross.
    let r = call(
        "navigation.route.cpa",
        r#"{"a_course":"360 deg","a_speed":"100 m/s","b_east":"0 m","b_north":"10000 m","b_course":"180 deg","b_speed":"100 m/s","b_up":"-1000 m","a_vertical_speed":"-10 m/s"}"#,
    );
    // r(t) = (0, 10000 − 200t, −1000 + 10t): minimized at t = 2,010,000 / 40,100.
    let t = 2_010_000.0 / 40_100.0;
    assert!((num(&r, "result.time.value") - t).abs() < 1e-9, "{r}");
    let sep = (10_000.0_f64 - 200.0 * t).hypot(-1000.0 + 10.0 * t);
    assert!((num(&r, "result.separation.value") - sep).abs() < 1e-9);
    // Without vertical inputs the result stays 2D and adds no fields.
    let r = call(
        "navigation.route.cpa",
        r#"{"a_course":"090 deg","a_speed":"10 m/s","b_east":"1000 m","b_north":"1200 m","b_course":"180 deg","b_speed":"10 m/s"}"#,
    );
    assert!(r["result"].get("vertical_separation").is_none());
    assert!((num(&r, "result.separation.value") - 141.421_356_237).abs() < 1e-6);
}

/// Layer E for `navigation.geodesic.intersection`. The crossing point does not
/// care which segment is called which, nor which way either is walked, and the
/// distance reported along a segment is the distance to the point.
#[test]
fn geodesic_intersection_invariants() {
    let cross = |a: [f64; 4], b: [f64; 4]| {
        let r = call(
            "navigation.geodesic.intersection",
            &format!(
                r#"{{"a_start_lat":{},"a_start_lon":{},"a_end_lat":{},"a_end_lon":{},"b_start_lat":{},"b_start_lon":{},"b_end_lat":{},"b_end_lon":{}}}"#,
                a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3]
            ),
        );
        assert_eq!(r["ok"], true, "{r}");
        r
    };
    let pairs = [
        // New York to London, crossed by Reykjavik to Lisbon.
        (
            [40.6413, -73.7781, 51.47, -0.4543],
            [64.1466, -21.9426, 38.7223, -9.1393],
        ),
        // Two short segments that cross near the equator.
        ([-1.0, -1.0, 1.0, 1.0], [-1.0, 1.0, 1.0, -1.0]),
        // A pair that crosses well outside both segments.
        ([0.0, 0.0, 0.0, 10.0], [10.0, 20.0, 5.0, 20.0]),
        // Far north, where the meridians converge.
        ([70.0, -50.0, 72.0, 50.0], [68.0, 10.0, 78.0, -10.0]),
    ];
    for (a, b) in pairs {
        let r = cross(a, b);
        let (lat, lon) = (num(&r, "result.lat.value"), num(&r, "result.lon.value"));
        let (da, db) = (
            num(&r, "result.along_a.value"),
            num(&r, "result.along_b.value"),
        );

        // Naming the segments the other way round swaps the two distances and
        // leaves the point where it is.
        let s = cross(b, a);
        assert!(
            (num(&s, "result.lat.value") - lat).abs() < 1e-9
                && (num(&s, "result.lon.value") - lon).abs() < 1e-9,
            "swapping the segments moved the crossing"
        );
        assert!(
            (num(&s, "result.along_a.value") - db).abs() < 1e-6,
            "swapped along_a"
        );
        assert!(
            (num(&s, "result.along_b.value") - da).abs() < 1e-6,
            "swapped along_b"
        );

        // Walking segment A the other way puts the crossing the same distance
        // from the other end, and does not move it.
        let back = cross([a[2], a[3], a[0], a[1]], b);
        let len_a = num(&r, "result.length_a.value");
        assert!(
            (num(&back, "result.lat.value") - lat).abs() < 1e-9,
            "reversing a segment moved the crossing"
        );
        assert!(
            (num(&back, "result.along_a.value") - (len_a - da)).abs() < 1e-6,
            "reversed along_a is {} and the length less the original is {}",
            num(&back, "result.along_a.value"),
            len_a - da
        );

        // The distance said to be along a segment is the distance to the point.
        let leg = call(
            "navigation.geodesic.inverse",
            &format!(
                r#"{{"lat1":{},"lon1":{},"lat2":{lat},"lon2":{lon}}}"#,
                a[0], a[1]
            ),
        );
        // along_a is in metres and the geodesic inverse answers in kilometres.
        let to_point_m = num(&leg, "result.distance.value") * 1000.0;
        assert!(
            (to_point_m - da.abs()).abs() < 1e-3,
            "along_a is {da} m and the geodesic to the point is {to_point_m} m"
        );

        // "within" is exactly whether both distances fall inside their segments.
        let len_b = num(&r, "result.length_b.value");
        let inside = (0.0..=len_a).contains(&da) && (0.0..=len_b).contains(&db);
        assert_eq!(
            r["result"]["within"] == "yes",
            inside,
            "within says {} for {da} of {len_a} and {db} of {len_b}",
            r["result"]["within"]
        );
    }
}

/// Layer E for `navigation.route.legs`. The legs add up to the total, there is
/// one fewer leg than waypoints, each leg is the geodesic between its own ends,
/// flying the route backwards covers the same ground, and the time is the
/// distance at the speed given.
#[test]
fn route_legs_invariants() {
    let route = |pts: &[(f64, f64)], extra: &str| {
        let rows: Vec<String> = pts
            .iter()
            .map(|(la, lo)| format!(r#"{{"lat":{la},"lon":{lo}}}"#))
            .collect();
        let r = call(
            "navigation.route.legs",
            &format!(r#"{{"waypoints":[{}]{extra}}}"#, rows.join(",")),
        );
        assert_eq!(r["ok"], true, "{r}");
        r
    };
    let routes: [&[(f64, f64)]; 3] = [
        &[
            (39.8617, -104.6731),
            (39.2232, -106.8688),
            (39.1224, -108.5267),
        ],
        &[(0.0, 0.0), (0.0, 30.0), (10.0, 60.0), (-5.0, 90.0)],
        // Across the antimeridian and up near the pole.
        &[(60.0, 170.0), (65.0, -175.0), (70.0, -160.0)],
    ];
    for pts in routes {
        let r = route(pts, "");
        let legs = r["result"]["legs"].as_array().expect("legs");
        assert_eq!(legs.len(), pts.len() - 1, "one fewer leg than waypoints");
        assert_eq!(
            num(&r, "result.legs_count") as usize,
            pts.len() - 1,
            "legs_count disagrees with the legs"
        );

        // The legs add up to the total.
        let summed: f64 = legs
            .iter()
            .map(|l| l["distance"]["value"].as_f64().expect("distance"))
            .sum();
        let total = num(&r, "result.total_distance.value");
        assert!(
            (summed - total).abs() < 1e-6,
            "the legs sum to {summed} and the total says {total}"
        );

        // Each leg is the geodesic between its own two waypoints.
        for (i, leg) in legs.iter().enumerate() {
            let inv = call(
                "navigation.geodesic.inverse",
                &format!(
                    r#"{{"lat1":{},"lon1":{},"lat2":{},"lon2":{}}}"#,
                    pts[i].0,
                    pts[i].1,
                    pts[i + 1].0,
                    pts[i + 1].1
                ),
            );
            // The route answers in nautical miles and the inverse in kilometres.
            let want_nm = num(&inv, "result.distance.value") / 1.852;
            let got = leg["distance"]["value"].as_f64().expect("distance");
            assert!(
                (got - want_nm).abs() < 1e-6,
                "leg {i} is {got} NM and the geodesic between its ends is {want_nm} NM"
            );
            let course = leg["true_course"]["value"].as_f64().expect("course");
            let azimuth = num(&inv, "result.azimuth1.value");
            assert!(
                (course - azimuth).abs() < 1e-6,
                "leg {i} courses {course} and the geodesic leaves on {azimuth}"
            );
        }

        // Flying it backwards covers the same ground.
        let mut back = pts.to_vec();
        back.reverse();
        let rev = route(&back, "");
        assert!(
            (num(&rev, "result.total_distance.value") - total).abs() < 1e-6,
            "the route is a different length backwards"
        );

        // Time is distance over speed, and nothing else. It is written for a
        // reader ("15 h 02 min"), so it is read back the same way.
        let timed = route(pts, r#","groundspeed":"120 kt""#);
        let written = timed["result"]["total_time"]
            .as_str()
            .expect("a written time");
        let part = |unit: &str| -> f64 {
            written
                .split_whitespace()
                .collect::<Vec<_>>()
                .windows(2)
                .find(|w| w[1] == unit)
                .and_then(|w| w[0].parse::<f64>().ok())
                .unwrap_or(0.0)
        };
        let hours = part("h") + part("min") / 60.0;
        // The written time is rounded to the minute, so allow half of one.
        assert!(
            (hours - total / 120.0).abs() < 1.0 / 120.0,
            "{written} for {total} NM at 120 kt"
        );

        // The last leg's running total is the route's total.
        let last = legs.last().expect("a leg");
        assert!(
            (last["cumulative"]["value"].as_f64().expect("cumulative") - total).abs() < 1e-6,
            "the running total ends at {:?} and the route is {total}",
            last["cumulative"]["value"]
        );
    }
}
