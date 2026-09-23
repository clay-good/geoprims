//! Geodesy tools: catalog lint, examples, golden vectors, spec scenarios, and
//! round-trip properties over many points.

use std::path::Path;

use gp_base::{manifest, template, vectors};
use gp_geodesy::{REGISTRY, TOOLS};
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
    r["meta"]["warnings"].as_array().map_or(vec![], |a| {
        a.iter()
            .map(|w| w["code"].as_str().unwrap().to_owned())
            .collect()
    })
}

#[test]
fn catalog_lint_examples_vectors() {
    // The host supplies data assets; the test plays the host.
    gp_base::assets::put(
        "egm96-15@2009-08-29/egm96-15.pgm",
        include_bytes!("../../../../assets/data/egm96-15/2009-08-29/egm96-15.pgm"),
    );
    for g in [
        "nad27_nad83_1986_conus",
        "nad27_nad83_1986_alaska",
        "ohd_nad83_1986_hawaii",
        "pr40_nad83_1986_prvi",
        "sp1952_nad83_1986_stpaul",
        "as62_nad83_1993_as",
        "gu63_nad83_1993_guamcnmi",
    ] {
        let path = format!(
            "{}/../../../assets/data/nadcon5/20160901/{g}.grid",
            env!("CARGO_MANIFEST_DIR")
        );
        gp_base::assets::put(
            &format!("nadcon5@20160901/{g}.grid"),
            &std::fs::read(&path).unwrap(),
        );
    }
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
    // Tools in other crates that geodesy tools point at; this crate cannot see
    // them, so they are named here. tools/trust/related.mjs checks the links
    // themselves against the whole catalog.
    let mut failures = manifest::lint(
        TOOLS,
        &taxonomy,
        &[
            "aviation.wind.runway-components",
            "navigation.geodesic.inverse",
            "survey.reduction.combined-factor",
        ],
    );
    let codes_reg: Value = serde_json::from_str(&repo("data/codes.json")).unwrap();
    for t in TOOLS {
        for w in t.warnings {
            if codes_reg["warnings"].get(*w).is_none() {
                failures.push(format!("{} declares unregistered {w}", t.id));
            }
        }
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
fn pittsburgh_everywhere() {
    let u = call(
        "geodesy.utm.forward",
        r#"{"lat":40.446111,"lon":-79.982222}"#,
    );
    assert_eq!(u["result"]["formatted"], "17N 586309.953 4477770.428");
    let d = call(
        "geodesy.utm.forward",
        r#"{"lat":"40°26'46\"N","lon":"79°58'56\"W"}"#,
    );
    assert!(
        (num(&d, "result.easting.value") - 586_309.95).abs() < 0.05,
        "DMS input: {d}"
    );
    let m = call(
        "geodesy.grid-ref.mgrs-forward",
        r#"{"lat":40.446111,"lon":-79.982222}"#,
    );
    assert_eq!(m["result"]["mgrs_spaced"], "17T NE 86309 77770");
    let back = call(
        "geodesy.grid-ref.mgrs-inverse",
        r#"{"mgrs":"17TNE8630977770"}"#,
    );
    let corner_e = num(
        &call(
            "geodesy.utm.forward",
            &format!(
                r#"{{"lat":{},"lon":{}}}"#,
                num(&back, "result.corner_lat.value"),
                num(&back, "result.corner_lon.value")
            ),
        ),
        "result.easting.value",
    );
    assert!((corner_e - 586_309.0).abs() < 1e-6, "{corner_e}");
    let p = call("geodesy.parse.coordinates", r#"{"text":"17TNE8630977770"}"#);
    assert_eq!(p["result"]["notation"], "MGRS");
    let p = call(
        "geodesy.parse.coordinates",
        r#"{"text":"17N 586309.953 4477770.428"}"#,
    );
    assert!((num(&p, "result.lat.value") - 40.446_111).abs() < 1e-8);
}

#[test]
fn forced_zone_scale_is_larger() {
    let std = call(
        "geodesy.utm.forward",
        r#"{"lat":40.446111,"lon":-79.982222}"#,
    );
    let forced = call(
        "geodesy.utm.forward",
        r#"{"lat":40.446111,"lon":-79.982222,"zone":18}"#,
    );
    assert!(codes(&forced).contains(&"NONSTANDARD_ZONE".to_owned()));
    assert!(num(&forced, "result.scale") > num(&std, "result.scale"));
    let far = call("geodesy.utm.forward", r#"{"lat":40,"lon":-80,"zone":22}"#);
    assert_eq!(far["error"]["code"], "INVALID_INPUT");
}

#[test]
fn domain_limits() {
    let r = call("geodesy.utm.forward", r#"{"lat":84.5,"lon":0}"#);
    assert_eq!(r["error"]["code"], "OUT_OF_DOMAIN");
    assert!(
        r["error"]["hint"]
            .as_str()
            .unwrap()
            .contains("geodesy.ups.forward")
    );
}

#[test]
fn bearing_difference_across_north() {
    let r = call(
        "geodesy.parse.bearing-difference",
        r#"{"from":350,"to":10}"#,
    );
    assert_eq!(num(&r, "result.difference.value"), 20.0);
    assert_eq!(r["summary"], "Turn 20° right.");
}

#[test]
fn format_rounding_carry_and_resolution() {
    let r = call(
        "geodesy.parse.format",
        r#"{"lat":10.9999999,"lon":0,"style":"dms","decimals":0}"#,
    );
    assert_eq!(r["result"]["formatted"], "11°00'00\"N 0°00'00\"E");
    let r = call(
        "geodesy.parse.format",
        r#"{"lat":40,"lon":0,"style":"dd","decimals":4}"#,
    );
    assert!((num(&r, "result.latitude_resolution.value") - 11.1).abs() < 0.05);
}

/// Coordinates as a machine exports them, with an exponent. The E of an
/// exponent is not the E of East, and used to be refused outright.
#[test]
fn scientific_notation_is_read_as_a_number() {
    let cases: [(&str, f64, f64); 5] = [
        ("1.0E-3 2.0", 0.001, 2.0),
        ("-1.0523e2 4.001e1", 40.01, -105.23),
        ("4.05E1N 7.998E1W", 40.5, -79.98),
        ("1.5E2 40.1", 40.1, 150.0),
        ("4.0446111e1 -7.9982222e1", 40.446111, -79.982222),
    ];
    for (text, lat, lon) in cases {
        let r = call(
            "geodesy.parse.coordinates",
            &format!(r#"{{"text":"{text}"}}"#),
        );
        assert_eq!(r["ok"], true, "{text}: {r}");
        assert!(
            (num(&r, "result.lat.value") - lat).abs() < 1e-9
                && (num(&r, "result.lon.value") - lon).abs() < 1e-9,
            "{text} -> {} {}",
            num(&r, "result.lat.value"),
            num(&r, "result.lon.value")
        );
    }
    // A trailing E is still East, and a pair of them still splits the pair.
    let r = call("geodesy.parse.coordinates", r#"{"text":"1.5E 40.1"}"#);
    assert_eq!(num(&r, "result.lon.value"), 1.5);
    let r = call("geodesy.parse.coordinates", r#"{"text":"40.45N 79.98E"}"#);
    assert_eq!(num(&r, "result.lon.value"), 79.98);
    // An exponent can outrun a double; that is refused, not carried as infinity.
    let r = call("geodesy.parse.coordinates", r#"{"text":"1e400 2"}"#);
    assert_eq!(r["ok"], false);
    assert!(
        r["error"]["message"]
            .as_str()
            .expect("message")
            .contains("too large"),
        "{r}"
    );
}

/// A pair without hemisphere letters whose first value is malformed used to
/// have that value thrown away and the second one read twice, so an invalid
/// coordinate came back as a plausible one: `40d26'60" 79d58'56"` answered
/// latitude and longitude both 79.98 instead of saying the seconds were 60.
#[test]
fn a_malformed_first_value_is_reported_not_replaced() {
    let bad: [(&str, &str); 3] = [
        (
            "40\u{b0}26'60\" 79\u{b0}58'56\"",
            "seconds must be less than 60",
        ),
        ("12:70:00 30", "minutes must be less than 60"),
        ("1e400 2", "too large"),
    ];
    for (text, why) in bad {
        let r = call(
            "geodesy.parse.coordinates",
            &format!(r#"{{"text":"{}"}}"#, text.replace('"', "\\\"")),
        );
        assert_eq!(r["ok"], false, "{text} should not parse: {r}");
        assert!(
            r["error"]["message"]
                .as_str()
                .expect("message")
                .contains(why),
            "{text}: {r}"
        );
    }
    // The orders that are only unusual, not wrong, still read as before.
    let r = call("geodesy.parse.coordinates", r#"{"text":"-105.27 40.01"}"#);
    assert_eq!(num(&r, "result.lat.value"), 40.01);
    assert_eq!(num(&r, "result.lon.value"), -105.27);
    let r = call("geodesy.parse.coordinates", r#"{"text":"40.45 -79.98"}"#);
    assert_eq!(num(&r, "result.lat.value"), 40.45);
}

/// Every rejection the spec lists, written without the hemisphere letters its
/// scenarios happen to carry. Letters send a pair down another branch, so a
/// scenario can pass while the rule it names does nothing on a bare pair.
#[test]
fn the_rejections_bite_without_hemisphere_letters() {
    let bad: [(&str, &str); 9] = [
        (
            "40\u{b0}26\'60\" 79\u{b0}58\'56\"",
            "seconds must be less than 60",
        ),
        (
            "40\u{b0}26\'46\" 79\u{b0}58\'60\"",
            "seconds must be less than 60",
        ),
        (
            "40\u{b0}60\'00\" 79\u{b0}58\'56\"",
            "minutes must be less than 60",
        ),
        ("40\u{b0}-26\'46\" 79\u{b0}58\'56\"", "cannot be negative"),
        ("40\u{b0}26\'-46\" 79\u{b0}58\'56\"", "cannot be negative"),
        ("40:-26:46 79:58:56", "cannot be negative"),
        (
            "40\u{b0}26\'46\" 181\u{b0}58\'56\"",
            "longitude must be within 180",
        ),
        ("40.45 -79.98xyz", "unexpected character"),
        (
            "40\u{b0}26.5\'46\" 79.98",
            "only the last component may have decimals",
        ),
    ];
    for (text, why) in bad {
        let r = call(
            "geodesy.parse.coordinates",
            &format!(
                r#"{{"text":"{}"}}"#,
                text.replace('\\', "\\\\").replace('"', "\\\"")
            ),
        );
        assert_eq!(r["ok"], false, "{text} should not parse: {r}");
        assert!(
            r["error"]["message"]
                .as_str()
                .expect("message")
                .contains(why),
            "{text}: wanted {why}, got {r}"
        );
    }
    // A hyphen straight after a digit is still how 40-26-46 separates its
    // components, which is what stops the rule above from being a blunt ban.
    let r = call(
        "geodesy.parse.coordinates",
        r#"{"text":"40-26-46N 79-58-56W"}"#,
    );
    assert!(
        (num(&r, "result.lat.value") - 40.446111).abs() < 1e-6,
        "{r}"
    );
    assert!(
        (num(&r, "result.lon.value") + 79.982222).abs() < 1e-6,
        "{r}"
    );
    // A first value that cannot be a latitude still infers the order instead
    // of being refused: that one is unusual, not wrong.
    let r = call("geodesy.parse.coordinates", r#"{"text":"91.5 79.98"}"#);
    assert_eq!(num(&r, "result.lat.value"), 79.98);
    assert_eq!(num(&r, "result.lon.value"), 91.5);
}

/// UTM and MGRS round trips over a grid of points.
#[test]
fn round_trip_properties() {
    let mut worst = 0.0f64;
    for lat in (-79..=83).step_by(7) {
        for lon in (-179..180).step_by(13) {
            let (lat, lon) = (lat as f64 + 0.37, lon as f64 + 0.61);
            let f = call(
                "geodesy.utm.forward",
                &format!(r#"{{"lat":{lat},"lon":{lon}}}"#),
            );
            let inv = call(
                "geodesy.utm.inverse",
                &format!(
                    r#"{{"zone":{},"hemisphere":"{}","easting":{},"northing":{}}}"#,
                    num(&f, "result.zone"),
                    f["result"]["hemisphere"].as_str().unwrap(),
                    num(&f, "result.easting.value"),
                    num(&f, "result.northing.value")
                ),
            );
            worst = worst
                .max((num(&inv, "result.lat.value") - lat).abs())
                .max((num(&inv, "result.lon.value") - lon).abs());
            let m = call(
                "geodesy.grid-ref.mgrs-forward",
                &format!(r#"{{"lat":{lat},"lon":{lon},"precision":"0.001m"}}"#),
            );
            let back = call(
                "geodesy.grid-ref.mgrs-inverse",
                &format!(r#"{{"mgrs":"{}"}}"#, m["result"]["mgrs"].as_str().unwrap()),
            );
            assert_eq!(back["ok"], true, "{m} -> {back}");
            assert!((num(&back, "result.lat.value") - lat).abs() < 1e-7, "{m}");
        }
    }
    assert!(worst < 1e-10, "worst UTM round trip {worst}°");
}

#[test]
fn utm_matches_exact_transverse_mercator() {
    // 1,000 points of GeographicLib's TMcoords.dat (exact transverse Mercator
    // with 80-digit arithmetic), within 3.5° of the central meridian, placed
    // in zone 31. Forward and inverse must agree to 5 nm and 1e-12°.
    let text = include_str!("data/TMcoords-sample.dat");
    let (mut dxy, mut dll, mut dg, mut dk) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    for l in text.lines() {
        let f: Vec<f64> = l.split_whitespace().map(|x| x.parse().unwrap()).collect();
        let (lat, dlon, x, y, g, k) = (f[0], f[1], f[2], f[3], f[4], f[5]);
        let r = call(
            "geodesy.utm.forward",
            &format!(r#"{{"lat":{lat},"lon":{},"zone":31}}"#, 3.0 + dlon),
        );
        dxy = dxy.max(
            (num(&r, "result.easting.value") - 500_000.0 - x)
                .hypot(num(&r, "result.northing.value") - y),
        );
        dg = dg.max((num(&r, "result.convergence.value") - g).abs());
        dk = dk.max((num(&r, "result.scale") - k).abs());
        let i = call(
            "geodesy.utm.inverse",
            &format!(
                r#"{{"zone":31,"hemisphere":"N","easting":"{} m","northing":"{y} m"}}"#,
                500_000.0 + x
            ),
        );
        dll = dll.max(
            (num(&i, "result.lat.value") - lat)
                .abs()
                .max((num(&i, "result.lon.value") - 3.0 - dlon).abs()),
        );
        dg = dg.max((num(&i, "result.convergence.value") - g).abs());
        dk = dk.max((num(&i, "result.scale") - k).abs());
    }
    assert!(dxy <= 5e-9, "position {dxy} m");
    assert!(dll <= 1e-12, "lat/lon {dll}°");
    assert!(dg <= 1e-12, "convergence {dg}°");
    assert!(dk <= 1e-13, "scale {dk}");
}

#[test]
fn utm_invariants() {
    // Mirror symmetry about the central meridian and the equator, and
    // forward then inverse returning the point, in every zone.
    for zone in (1..=60).step_by(7) {
        let cm = 6.0 * zone as f64 - 183.0;
        for lat in [-79.5, -41.2, -0.3, 0.3, 23.4, 61.7, 79.4] {
            for d in [0.4, 1.7, 2.9] {
                let at = |lat: f64, lon: f64| {
                    call(
                        "geodesy.utm.forward",
                        &format!(r#"{{"lat":{lat},"lon":{lon},"zone":{zone}}}"#),
                    )
                };
                let (e, w) = (at(lat, cm + d), at(lat, cm - d));
                let de = num(&e, "result.easting.value") - 500_000.0;
                assert!(
                    (de + num(&w, "result.easting.value") - 500_000.0).abs() < 1e-8,
                    "E {zone} {lat} {d}"
                );
                assert!(
                    (num(&e, "result.northing.value") - num(&w, "result.northing.value")).abs()
                        < 1e-8
                );
                let s = at(-lat, cm + d);
                let (n_n, n_s) = if lat >= 0.0 {
                    (
                        num(&e, "result.northing.value"),
                        num(&s, "result.northing.value"),
                    )
                } else {
                    (
                        num(&s, "result.northing.value"),
                        num(&e, "result.northing.value"),
                    )
                };
                assert!(
                    (n_n + n_s - 10_000_000.0).abs() < 1e-8,
                    "N {zone} {lat} {d}"
                );
                let back = call(
                    "geodesy.utm.inverse",
                    &format!(
                        r#"{{"zone":{zone},"hemisphere":"{}","easting":"{} m","northing":"{} m"}}"#,
                        e["result"]["hemisphere"].as_str().unwrap(),
                        num(&e, "result.easting.value"),
                        num(&e, "result.northing.value")
                    ),
                );
                assert!(
                    (num(&back, "result.lat.value") - lat).abs() < 1e-11,
                    "{back}"
                );
                assert!(
                    (num(&back, "result.lon.value") - cm - d).abs() < 1e-11,
                    "{back}"
                );
            }
        }
    }
}

#[test]
fn parse_regressions_from_the_notation_fixture() {
    // Found by tests/parse_parity.rs (1,500 independently written strings).
    let parse = |t: &str| {
        call(
            "geodesy.parse.coordinates",
            &serde_json::json!({"text": t}).to_string(),
        )
    };
    let near = |r: &Value, lat: f64, lon: f64| {
        (num(r, "result.lat.value") - lat).abs() < 1e-9
            && (num(r, "result.lon.value") - lon).abs() < 1e-9
    };
    // Signed DMS with degree signs and no hemisphere letters: sliced inside the two-byte ° and panicked.
    let r = parse("21°4'34.13\" -139°35'55.91\"");
    assert!(
        near(
            &r,
            21.0 + 4.0 / 60.0 + 34.13 / 3600.0,
            -(139.0 + 35.0 / 60.0 + 55.91 / 3600.0)
        ),
        "{r}"
    );
    let r = parse("21°4′34.13″ -139°35′55.91″");
    assert!(
        near(
            &r,
            21.0 + 4.0 / 60.0 + 34.13 / 3600.0,
            -(139.0 + 35.0 / 60.0 + 55.91 / 3600.0)
        ),
        "{r}"
    );
    // Spaced DMS with a one- or two-digit longitude was read as packed notation (1°25' became 125°).
    let r = parse("30 34 14.3 N 1 25 23.9 E");
    assert!(
        near(
            &r,
            30.0 + 34.0 / 60.0 + 14.3 / 3600.0,
            1.0 + 25.0 / 60.0 + 23.9 / 3600.0
        ),
        "{r}"
    );
    let r = parse("3 41 7.5 S 151 22 45.1 W");
    assert!(
        near(
            &r,
            -(3.0 + 41.0 / 60.0 + 7.5 / 3600.0),
            -(151.0 + 22.0 / 60.0 + 45.1 / 3600.0)
        ),
        "{r}"
    );
    // One-digit decimal degrees with a hemisphere letter underflowed the packed check.
    let r = parse("5.12345N 7.54321E");
    assert!(near(&r, 5.12345, 7.54321), "{r}");
    // Packed notation itself still reads.
    let r = parse("402646N0795856W");
    assert!(
        near(
            &r,
            40.0 + 26.0 / 60.0 + 46.0 / 3600.0,
            -(79.0 + 58.0 / 60.0 + 56.0 / 3600.0)
        ),
        "{r}"
    );
}

#[test]
fn parse_coordinates_invariants() {
    // Whatever the formatter writes, the parser reads back to the point within
    // the printed rounding; hemisphere letters and signs agree; Unicode primes
    // and ASCII marks agree.
    let pts = [
        (40.446111, -79.982222),
        (-33.8688, 151.2093),
        (0.000_5, -0.000_5),
        (89.99, 179.99),
        (-89.99, -179.99),
        (7.5, 5.25),
    ];
    for (lat, lon) in pts {
        for (style, decimals, tol) in [
            ("dms", 3, 0.001 / 3600.0),
            ("ddm", 4, 0.0001 / 60.0),
            ("dd", 7, 1e-7),
        ] {
            for signs in ["letters", "signed"] {
                let f = call(
                    "geodesy.parse.format",
                    &serde_json::json!({"lat": lat, "lon": lon, "style": style, "decimals": decimals, "signs": signs}).to_string(),
                );
                let text = f["result"]["formatted"].as_str().unwrap().to_owned();
                let p = call(
                    "geodesy.parse.coordinates",
                    &serde_json::json!({"text": text}).to_string(),
                );
                assert!(
                    (num(&p, "result.lat.value") - lat).abs() <= tol * 0.51,
                    "{text}: {p}"
                );
                assert!(
                    (num(&p, "result.lon.value") - lon).abs() <= tol * 0.51,
                    "{text}: {p}"
                );
                let unicode = text.replace('\'', "′").replace('"', "″");
                let u = call(
                    "geodesy.parse.coordinates",
                    &serde_json::json!({"text": unicode}).to_string(),
                );
                assert_eq!(u["result"]["lat"], p["result"]["lat"], "{unicode}");
                assert_eq!(u["result"]["lon"], p["result"]["lon"], "{unicode}");
            }
        }
    }
}

#[test]
fn ups_invariants() {
    const F: &str = "geodesy.ups.forward";
    const I: &str = "geodesy.ups.inverse";
    let fwd = |lat: f64, lon: f64| call(F, &format!(r#"{{"lat":{lat},"lon":{lon}}}"#));
    let inv = |h: &str, e: f64, n: f64| {
        call(
            I,
            &format!(r#"{{"hemisphere":"{h}","easting":{e},"northing":{n}}}"#),
        )
    };
    let en = |r: &Value| {
        (
            num(r, "result.easting.value"),
            num(r, "result.northing.value"),
        )
    };
    // The pole is the false origin, in both hemispheres.
    for (lat, h) in [(90.0, "N"), (-90.0, "S")] {
        let (e, n) = en(&fwd(lat, 0.0));
        assert_eq!(
            (e, n),
            (2_000_000.0, 2_000_000.0),
            "the pole is not the origin"
        );
        let back = inv(h, 2_000_000.0, 2_000_000.0);
        assert!(
            (num(&back, "result.lat.value") - lat).abs() < 1e-9,
            "the origin does not invert to the pole"
        );
    }
    // Forward then inverse is the point it started from, to under a micrometre
    // of ground. Longitude is divided by cos(lat), since that is the factor
    // that turns a ground distance into an angle.
    for (lat, lon) in [
        (84.0, 0.0),
        (84.0, -135.0),
        (86.5, 30.0),
        (89.9, 170.0),
        (-80.0, 45.0),
        (-88.0, 100.0),
    ] {
        let h = if lat > 0.0 { "N" } else { "S" };
        let (e, n) = en(&fwd(lat, lon));
        let back = inv(h, e, n);
        let (blat, blon) = (
            num(&back, "result.lat.value"),
            num(&back, "result.lon.value"),
        );
        assert!((blat - lat).abs() < 1e-11, "{lat},{lon}: latitude {blat}");
        let slack = 1e-11 / lat.to_radians().cos().abs();
        assert!(
            (blon - lon).abs() < slack,
            "{lat},{lon}: longitude {blon}, outside {slack}"
        );
    }
    // Distance from the origin depends only on latitude, so walking the
    // longitude circle traces a circle.
    let radius = |lat: f64, lon: f64| {
        let (e, n) = en(&fwd(lat, lon));
        ((e - 2_000_000.0).powi(2) + (n - 2_000_000.0).powi(2)).sqrt()
    };
    let r0 = radius(84.0, 0.0);
    for lon in [45.0, 90.0, 135.0, -45.0, -90.0, -135.0, 179.0] {
        assert!(
            (radius(84.0, lon) - r0).abs() < 1e-6,
            "84 deg at {lon} is not on the same circle"
        );
    }
    // Further from the pole is a larger radius: the projection does not fold.
    assert!(radius(84.0, 0.0) > radius(86.0, 0.0));
    assert!(radius(86.0, 0.0) > radius(89.0, 0.0));
    // Convergence is the longitude, negated in the south.
    for lon in [0.0, 30.0, -120.0] {
        assert!(
            (num(&fwd(84.0, lon), "result.convergence.value") - lon).abs() < 1e-9,
            "north convergence at {lon}"
        );
        assert!(
            (num(&fwd(-84.0, lon), "result.convergence.value") + lon).abs() < 1e-9,
            "south convergence at {lon}"
        );
    }
    // The hemisphere is read, not guessed: the same numbers give latitudes of
    // opposite sign.
    let (e, n) = en(&fwd(85.0, 40.0));
    let north = num(&inv("N", e, n), "result.lat.value");
    let south = num(&inv("S", e, n), "result.lat.value");
    assert!(north > 0.0 && south < 0.0, "{north} and {south}");
    assert!((north + south).abs() < 1e-9, "the caps are not mirrored");
}
