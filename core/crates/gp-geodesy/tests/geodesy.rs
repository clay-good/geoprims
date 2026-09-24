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
            "geometry.area.polygon",
            "indexing.tile.from-point",
            "navigation.geodesic.direct",
            "navigation.geodesic.inverse",
            "survey.reduction.combined-factor",
            "units.length.convert",
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

#[test]
fn bearing_difference_invariants() {
    const B: &str = "geodesy.parse.bearing-difference";
    let diff = |a: f64, b: f64| {
        num(
            &call(B, &format!(r#"{{"from":{a},"to":{b}}}"#)),
            "result.difference.value",
        )
    };
    // The short way round, signed: the whole reason the tool exists.
    assert_eq!(diff(350.0, 10.0), 20.0);
    assert_eq!(diff(10.0, 350.0), -20.0);
    assert_eq!(diff(359.0, 1.0), 2.0);
    assert_eq!(diff(1.0, 359.0), -2.0);
    for (a, b) in [
        (0.0, 90.0),
        (90.0, 0.0),
        (45.0, 200.0),
        (123.456, 234.567),
        (359.5, 179.5),
    ] {
        let d = diff(a, b);
        // The range is half-open the other way than one might guess. The
        // definition is ((to - from + 180) mod 360) - 180, and a difference of
        // exactly 180 gives (360 mod 360) - 180 = -180, so a reversal reads as
        // a left turn. That falls out of the formula rather than being chosen,
        // and the tool states the formula.
        assert!((-180.0..180.0).contains(&d), "{a}->{b} gave {d}");
        // Reversing the arguments negates the answer, except at the reversal
        // where both directions are the same turn.
        let back = diff(b, a);
        assert!(
            (d + back).abs() < 1e-9 || (d + 180.0).abs() < 1e-9,
            "{a}->{b} is {d} but {b}->{a} is {back}"
        );
        // The difference added to `from` gives `to` back, which is what makes
        // it a turn rather than a number.
        assert!(
            ((a + d - b).rem_euclid(360.0)).min((b - a - d).rem_euclid(360.0)) < 1e-9,
            "{a} + {d} is not {b}"
        );
    }
    // A whole turn on either side changes nothing.
    assert_eq!(diff(10.0, 10.0), 0.0);
    assert_eq!(diff(10.0, 370.0), 0.0);
    assert_eq!(diff(370.0, 10.0), 0.0);
    assert_eq!(diff(0.0, 180.0), -180.0, "a reversal reads as a left turn");
    assert_eq!(diff(180.0, 0.0), -180.0, "and so does the other direction");
}

#[test]
fn angle_arithmetic_invariants() {
    const A: &str = "geodesy.parse.angle-arithmetic";
    let sum = |terms: &str, norm: &str| {
        let n = if norm.is_empty() {
            String::new()
        } else {
            format!(r#","normalize":"{norm}""#)
        };
        call(A, &format!(r#"{{"terms":[{terms}]{n}}}"#))
    };
    let one = |a: &str| format!(r#"{{"angle":"{a}"}}"#);
    let minus = |a: &str| format!(r#"{{"angle":"{a}","operation":"subtract"}}"#);
    // The three outputs describe one angle.
    for terms in [
        format!("{},{}", one("45-30-15"), one("12-45-50")),
        format!("{},{}", one("1-00-00"), minus("0-59-59.5")),
        format!("{},{},{}", one("10"), one("20"), one("30")),
    ] {
        let r = sum(&terms, "");
        let (secs, degs) = (
            num(&r, "result.seconds.value"),
            num(&r, "result.degrees.value"),
        );
        assert!(
            (secs - degs * 3600.0).abs() < 1e-6,
            "{terms}: {secs} arcseconds is not {degs} degrees"
        );
    }
    // Adding and taking away the same angle returns the original exactly.
    let base = sum(&one("45-30-15"), "");
    let there_and_back = sum(
        &format!(
            "{},{},{}",
            one("45-30-15"),
            one("7-08-09"),
            minus("7-08-09")
        ),
        "",
    );
    assert_eq!(
        base["result"]["dms"], there_and_back["result"]["dms"],
        "a round trip changed the angle"
    );
    // The carry goes all the way: 59'59.995" is a whole degree, not 60 minutes.
    let carried = sum(&format!("{},{}", one("0-59-59.99"), one("0-00-00.01")), "");
    assert_eq!(carried["result"]["dms"], "1\u{b0}00'00.00\"");
    // And across the turn, under 0-360, it is zero rather than 360.
    let wrapped = sum(
        &format!("{},{}", one("359-59-59.995"), one("0-00-00.005")),
        "0-360",
    );
    assert_eq!(wrapped["result"]["dms"], "0\u{b0}00'00.00\"");
    // Each normalization lands in its own range.
    for (terms, norm, lo, hi) in [
        (
            format!("{},{}", one("300"), one("100")),
            "0-360",
            0.0,
            360.0,
        ),
        (
            format!("{},{}", one("270"), one("180")),
            "plus-minus-180",
            -180.0,
            180.0,
        ),
    ] {
        let d = num(&sum(&terms, norm), "result.degrees.value");
        assert!(d >= lo && d <= hi, "{terms} under {norm} gave {d}");
    }
    // The sum does not depend on the order the terms are given in.
    let a = sum(
        &format!(
            "{},{},{}",
            one("12-34-56"),
            one("1-02-03"),
            minus("0-30-00")
        ),
        "",
    );
    let b = sum(
        &format!(
            "{},{},{}",
            minus("0-30-00"),
            one("1-02-03"),
            one("12-34-56")
        ),
        "",
    );
    assert_eq!(a["result"]["dms"], b["result"]["dms"]);
}

#[test]
fn format_invariants() {
    const F: &str = "geodesy.parse.format";
    let fmt = |lat: f64, lon: f64, style: &str, d: u32| {
        call(
            F,
            &format!(r#"{{"lat":{lat},"lon":{lon},"style":"{style}","decimals":{d}}}"#),
        )
    };
    // The carry reaches the degrees rather than stopping at 60 minutes.
    assert_eq!(
        fmt(10.9999999, 0.0, "dms", 0)["result"]["formatted"],
        "11\u{b0}00'00\"N 0\u{b0}00'00\"E"
    );
    // One more decimal divides the resolution by ten, and the styles differ by
    // the 60 and 3600 their units say.
    let res = |lat: f64, style: &str, d: u32| {
        num(&fmt(lat, 0.0, style, d), "result.latitude_resolution.value")
    };
    assert!((res(45.0, "dd", 4) / res(45.0, "dd", 5) - 10.0).abs() < 1e-9);
    assert!((res(45.0, "dd", 3) / res(45.0, "ddm", 3) - 60.0).abs() < 1e-9);
    assert!((res(45.0, "ddm", 3) / res(45.0, "dms", 3) - 60.0).abs() < 1e-9);
    // Longitude shrinks with latitude; latitude barely moves.
    let lon_res = |lat: f64| num(&fmt(lat, 0.0, "dd", 6), "result.longitude_resolution.value");
    assert!(
        lon_res(60.0) < 0.55 * lon_res(0.0),
        "longitude did not follow cos"
    );
    assert!(
        (res(60.0, "dd", 6) / res(0.0, "dd", 6) - 1.0).abs() < 0.01,
        "latitude resolution moved too much"
    );
    // Six decimals of longitude at the equator is the 11 cm this is always
    // quoted as.
    assert!(
        (lon_res(0.0) - 0.1113).abs() < 0.001,
        "{} m is not about 11 cm",
        lon_res(0.0)
    );
    // Every style round trips through the coordinate parser to within the
    // resolution it claims.
    for (style, d) in [("dd", 6u32), ("ddm", 3), ("dms", 2)] {
        let r = fmt(-33.8688, 151.2093, style, d);
        let text = r["result"]["formatted"].as_str().expect("formatted");
        // The formatted string carries a double quote for seconds, so it has
        // to be escaped rather than pasted into JSON.
        let escaped = text.replace('"', "\\\"");
        let back = call(
            "geodesy.parse.coordinates",
            &format!(r#"{{"text":"{escaped}"}}"#),
        );
        assert_eq!(back["ok"], true, "{style}: {text} did not parse: {back}");
        let slack = num(&r, "result.latitude_resolution.value") / 111_000.0 + 1e-9;
        assert!(
            (num(&back, "result.lat.value") + 33.8688).abs() < slack * 2.0,
            "{style}: {text} came back as {}",
            num(&back, "result.lat.value")
        );
    }
}

#[test]
fn utm_zone_invariants() {
    const Z: &str = "geodesy.utm.zone";
    let at = |lat: f64, lon: f64| call(Z, &format!(r#"{{"lat":{lat},"lon":{lon}}}"#));
    let zone = |lat: f64, lon: f64| num(&at(lat, lon), "result.zone") as i64;
    let band = |lat: f64, lon: f64| {
        at(lat, lon)["result"]["grid_zone"]
            .as_str()
            .expect("grid_zone")
            .to_owned()
    };
    // Away from the exceptions the zone is the formula and nothing else.
    let mut lon: f64 = -179.0;
    while lon < 180.0 {
        let want = ((lon + 180.0) / 6.0).floor() as i64 + 1;
        assert_eq!(zone(0.0, lon), want, "the equator at {lon}");
        lon += 2.0;
    }
    // Norway: zone 32 is widened west to 3 deg between 56 and 64 north, and
    // not a degree outside the box on any of the four sides.
    for (lat, lon) in [(56.0, 3.0), (56.0, 5.0), (60.0, 4.0), (63.9, 11.9)] {
        assert_eq!(zone(lat, lon), 32, "inside the Norway box at {lat},{lon}");
    }
    assert_eq!(zone(55.9, 5.0), 31, "below the Norway box");
    assert_eq!(zone(64.1, 5.0), 31, "above the Norway box");
    assert_eq!(zone(60.0, 2.9), 31, "west of the Norway box");
    assert_eq!(zone(60.0, 12.1), 33, "east of the Norway box");
    // Svalbard: the even zones vanish between 72 and 84 north and come back
    // below it.
    for lon in [0.0, 7.0, 10.0, 20.0, 22.0, 34.0, 40.0] {
        let z = zone(78.0, lon);
        assert!(
            [31, 33, 35, 37].contains(&z),
            "Svalbard at {lon} gave zone {z}"
        );
    }
    assert_eq!(zone(71.9, 10.0), 32, "below Svalbard the even zone returns");
    assert_eq!(zone(71.9, 22.0), 34);
    // The central meridian belongs to the zone and is near the point.
    for (lat, lon) in [(0.0, 0.0), (-35.0, -70.0), (60.0, 5.0), (78.0, 20.0)] {
        let z = zone(lat, lon);
        let cm = num(&at(lat, lon), "result.central_meridian.value");
        assert_eq!(
            cm,
            (6 * z - 183) as f64,
            "{lat},{lon}: meridian of zone {z}"
        );
    }
    // The band letters skip I and O, and the poles are zone 0.
    let mut lat: f64 = -79.0;
    while lat < 84.0 {
        let g = band(lat, 20.0);
        let letter = g.chars().last().expect("a band letter");
        assert!(
            letter != 'I' && letter != 'O',
            "{lat} gave band {letter}, which reads as a digit"
        );
        lat += 2.0;
    }
    for (lat, want) in [(-80.1, "B"), (84.1, "Z")] {
        assert_eq!(zone(lat, 20.0), 0, "{lat} is polar");
        assert_eq!(band(lat, 20.0), want);
    }
}

#[test]
fn true_to_magnetic_invariants() {
    const T: &str = "geodesy.magnetic.true-to-magnetic";
    let go = |bearing: f64, dir: &str, extra: &str| {
        call(
            T,
            &format!(r#"{{"bearing":{bearing},"direction":"{dir}"{extra}}}"#),
        )
    };
    const PIT: &str = r#","lat":40.446111,"lon":-79.982222,"date":"2026-07-02""#;
    // Sydney's variation is easterly and Pittsburgh's westerly, which is what
    // the "east is least" check needs on both sides.
    const SYD: &str = r#","lat":-33.8688,"lon":151.2093,"date":"2026-08-08""#;
    // The two directions undo each other.
    for (b, place) in [(0.0, PIT), (90.0, PIT), (270.0, SYD), (359.0, SYD)] {
        let m = num(&go(b, "true-to-magnetic", place), "result.result.value");
        let back = num(&go(m, "magnetic-to-true", place), "result.result.value");
        assert!(
            ((back - b + 180.0).rem_euclid(360.0) - 180.0).abs() < 1e-9,
            "{b} went to {m} and came back {back}"
        );
        // Every answer is a bearing.
        assert!((0.0..360.0).contains(&m), "{m} is not in [0, 360)");
    }
    // East is least, west is best. Pittsburgh's variation is westerly and
    // Sydney's easterly, so the magnetic bearing is larger at one and
    // smaller at the other.
    let west = go(90.0, "true-to-magnetic", PIT);
    assert!(
        num(&west, "result.variation_used.value") < 0.0,
        "not westerly"
    );
    assert!(
        num(&west, "result.result.value") > 90.0,
        "west is best: the magnetic bearing should be larger"
    );
    let east = go(90.0, "true-to-magnetic", SYD);
    assert!(
        num(&east, "result.variation_used.value") > 0.0,
        "not easterly"
    );
    assert!(
        num(&east, "result.result.value") < 90.0,
        "east is least: the magnetic bearing should be smaller"
    );
    // The source is named, and a chart figure is used in preference to the
    // model, with the model reported beside it.
    assert_eq!(west["result"]["variation_source"], "model");
    assert!(west["result"]["model_declination"].is_null());
    let chart = go(
        0.0,
        "true-to-magnetic",
        &format!(r#","variation":"9.2W"{PIT}"#),
    );
    assert_eq!(chart["result"]["variation_source"], "chart");
    assert_eq!(num(&chart, "result.variation_used.value"), -9.2);
    // difference is chart minus model, signed.
    let d = num(&chart, "result.model_declination.value");
    assert!(
        (num(&chart, "result.difference.value") - (-9.2 - d)).abs() < 1e-9,
        "difference is not the chart figure less the model's"
    );
}

#[test]
fn grivation_invariants() {
    const G: &str = "geodesy.magnetic.grivation";
    let at = |lat: f64, lon: f64, date: &str| {
        call(
            G,
            &format!(r#"{{"lat":{lat},"lon":{lon},"date":"{date}"}}"#),
        )
    };
    // Grivation is the declination less the convergence, everywhere.
    for (lat, lon) in [
        (40.446111, -79.982222),
        (51.5074, -0.1278),
        (-33.8688, 151.2093),
        (78.0, 15.0),
        (-45.0, 170.0),
    ] {
        let r = at(lat, lon, "2026-07-02");
        let (g, d, c) = (
            num(&r, "result.grivation.value"),
            num(&r, "result.declination.value"),
            num(&r, "result.convergence.value"),
        );
        assert!(
            (g - (d - c)).abs() < 1e-9,
            "{lat},{lon}: {g} is not {d} - {c}"
        );
        // And the declination is the one the declination tool gives, not a
        // second copy of the model.
        let own = num(
            &call(
                "geodesy.magnetic.declination",
                &format!(r#"{{"lat":{lat},"lon":{lon},"date":"2026-07-02"}}"#),
            ),
            "result.declination.value",
        );
        assert!((d - own).abs() < 1e-9, "{lat},{lon}: two different models");
    }
    // On a zone's central meridian the convergence is zero, so the grivation
    // is the declination itself. Zone 17's meridian is -81.
    let on = at(40.0, -81.0, "2026-07-02");
    assert!(
        num(&on, "result.convergence.value").abs() < 1e-9,
        "the central meridian is not straight"
    );
    assert!(
        (num(&on, "result.grivation.value") - num(&on, "result.declination.value")).abs() < 1e-9
    );
    // It changes sign either side, positive to the east.
    assert!(num(&at(40.0, -80.0, "2026-07-02"), "result.convergence.value") > 0.0);
    assert!(num(&at(40.0, -82.0, "2026-07-02"), "result.convergence.value") < 0.0);
    // And grows with latitude for the same offset, since it goes as sin(lat).
    let near = num(&at(20.0, -80.0, "2026-07-02"), "result.convergence.value");
    let far = num(&at(60.0, -80.0, "2026-07-02"), "result.convergence.value");
    assert!(
        far > near,
        "convergence did not grow with latitude: {near} then {far}"
    );
}

#[test]
fn height_convert_invariants() {
    const H: &str = "geodesy.height.convert";
    // The geoid grid is an asset, not compiled in, so it has to be supplied
    // before anything that reads it will answer.
    gp_base::assets::put(
        "egm96-15@2009-08-29/egm96-15.pgm",
        include_bytes!("../../../../assets/data/egm96-15/2009-08-29/egm96-15.pgm"),
    );
    let go = |lat: f64, lon: f64, h: f64, from: &str| {
        call(
            H,
            &format!(r#"{{"lat":{lat},"lon":{lon},"height":"{h} m","from":"{from}"}}"#),
        )
    };
    for (lat, lon) in [
        (40.446111, -79.982222),
        (-33.8688, 151.2093),
        (90.0, 0.0),
        (-90.0, 0.0),
        (0.0, 179.9),
        (27.9881, 86.9250),
    ] {
        let r = go(lat, lon, 100.0, "ellipsoidal");
        let (h, ortho, n) = (
            num(&r, "result.ellipsoidal.value"),
            num(&r, "result.orthometric.value"),
            num(&r, "result.geoid_height.value"),
        );
        // h = H + N, stated as an identity rather than assumed.
        assert!(
            (h - ortho - n).abs() < 1e-9,
            "{lat},{lon}: {h} != {ortho} + {n}"
        );
        // Where the geoid is above the ellipsoid, the sea-level height is the
        // smaller of the two. That is the sign, and it is easy to flip.
        if n > 0.0 {
            assert!(ortho < h, "{lat},{lon}: N is +{n} but H is not below h");
        } else if n < 0.0 {
            assert!(ortho > h, "{lat},{lon}: N is {n} but H is not above h");
        }
        // The two directions are exact inverses.
        let back = go(lat, lon, ortho, "orthometric");
        assert!(
            (num(&back, "result.ellipsoidal.value") - h).abs() < 1e-9,
            "{lat},{lon}: round trip lost {h}"
        );
        // The geoid height is a property of the place, not of the height.
        let higher = go(lat, lon, 3000.0, "ellipsoidal");
        assert!(
            (num(&higher, "result.geoid_height.value") - n).abs() < 1e-12,
            "{lat},{lon}: the geoid moved when the height changed"
        );
        // And it is the same model the geoid tool uses.
        let own = num(
            &call(
                "geodesy.geoid.geoid-height",
                &format!(r#"{{"lat":{lat},"lon":{lon}}}"#),
            ),
            "result.geoid_height.value",
        );
        assert!((n - own).abs() < 1e-9, "{lat},{lon}: two different geoids");
    }
    // The two interpolation schemes differ by centimetres, not metres.
    let cubic = num(
        &call(
            H,
            r#"{"lat":40.446111,"lon":-79.982222,"height":"100 m","interpolation":"cubic"}"#,
        ),
        "result.geoid_height.value",
    );
    let bilinear = num(
        &call(
            H,
            r#"{"lat":40.446111,"lon":-79.982222,"height":"100 m","interpolation":"bilinear"}"#,
        ),
        "result.geoid_height.value",
    );
    assert!(
        (cubic - bilinear).abs() < 0.5,
        "the two schemes differ by {} m",
        cubic - bilinear
    );
}

#[test]
fn arc_to_chord_invariants() {
    const A: &str = "geodesy.projection.arc-to-chord";
    let line = |la1: f64, lo1: f64, la2: f64, lo2: f64, z: &str| {
        call(
            A,
            &format!(
                r#"{{"lat1":{la1},"lon1":{lo1},"lat2":{la2},"lon2":{lo2},"grid":"utm","zone":"{z}"}}"#
            ),
        )
    };
    let ends = |r: &Value| {
        (
            num(r, "result.t_minus_t_from.value"),
            num(r, "result.t_minus_t_to.value"),
        )
    };
    // Along a central meridian there is no correction at either end. Zone 17's
    // meridian is -81.
    let (a, b) = ends(&line(40.0, -81.0, 41.0, -81.0, "17"));
    assert!(
        a.abs() < 1e-3 && b.abs() < 1e-3,
        "on the central meridian the correction is {a} and {b}"
    );
    // Reversing the line swaps the ends: one curve read two ways.
    let there = ends(&line(40.0, -80.0, 40.5, -79.5, "17"));
    let back = ends(&line(40.5, -79.5, 40.0, -80.0, "17"));
    assert!(
        (there.0 - back.1).abs() < 1e-6 && (there.1 - back.0).abs() < 1e-6,
        "reversing gave {back:?} against {there:?}"
    );
    // Away from the central meridian the two ends take opposite signs, which
    // is the classical near-equal-and-opposite result. A line that STRADDLES
    // the meridian is the exception and both ends take the same sign, the
    // curve bending the same way on either half. I had this the wrong way
    // round until the test said so.
    for (la1, lo1, la2, lo2) in [
        (40.44, -79.99, 40.52, -79.91),
        (40.0, -80.5, 40.0, -79.5),
        (39.0, -80.0, 41.0, -79.0),
    ] {
        let (a, b) = ends(&line(la1, lo1, la2, lo2, "17"));
        assert!(a * b < 0.0, "{la1},{lo1}..{la2},{lo2} gave {a} and {b}");
    }
    let (s1, s2) = ends(&line(40.0, -81.4, 40.3, -80.6, "17"));
    assert!(
        s1 * s2 > 0.0,
        "straddling the meridian gave {s1} and {s2}, opposite signs"
    );
    // Longer lines and lines further from the meridian have larger corrections.
    let short = ends(&line(40.0, -80.5, 40.0, -80.3, "17")).0.abs();
    let long = ends(&line(40.0, -80.5, 40.0, -79.5, "17")).0.abs();
    assert!(long > short, "a longer line gave a smaller correction");
    let near = ends(&line(40.0, -81.1, 40.0, -80.9, "17")).0.abs();
    let far = ends(&line(40.0, -78.6, 40.0, -78.4, "17")).0.abs();
    assert!(
        far > near,
        "further from the meridian gave {far} against {near}"
    );
    // Grid and ellipsoid distance differ only by the scale factor.
    let r = line(40.0, -80.0, 40.5, -79.5, "17");
    let ratio = num(&r, "result.grid_distance.value") / num(&r, "result.ellipsoid_distance.value");
    assert!(
        (0.999..1.001).contains(&ratio),
        "the scale factor came out as {ratio}"
    );
}

#[test]
fn spcs_zone_lookup_invariants() {
    const S: &str = "geodesy.spcs.zone-lookup";
    let by_name = |q: &str| call(S, &format!(r#"{{"query":"{q}"}}"#));
    let at = |lat: f64, lon: f64| call(S, &format!(r#"{{"lat":{lat},"lon":{lon}}}"#));
    let codes = |r: &Value| -> Vec<i64> {
        r["result"]["zones"]
            .as_array()
            .expect("zones")
            .iter()
            .map(|z| z["epsg"].as_i64().expect("epsg"))
            .collect()
    };
    // Every row is filled in, and the count matches the list.
    for q in ["Colorado", "Texas", "Alaska", "Hawaii"] {
        let r = by_name(q);
        let zones = r["result"]["zones"].as_array().expect("zones");
        assert_eq!(num(&r, "result.count") as usize, zones.len(), "{q}");
        for z in zones {
            for field in ["zone_code", "zone_name", "projection", "feet"] {
                assert!(
                    z[field].as_str().is_some_and(|s| !s.is_empty()),
                    "{q}: a zone with no {field}"
                );
            }
            assert!(
                z["epsg"].as_i64().is_some(),
                "{q}: a zone with no EPSG code"
            );
        }
    }
    // A more specific query returns a subset of a less specific one.
    let all = codes(&by_name("Colorado"));
    let one = codes(&by_name("Colorado Central"));
    assert_eq!(one.len(), 1, "Colorado Central is one zone");
    assert!(
        all.contains(&one[0]),
        "the specific zone is not among the state's"
    );
    // Whole words, not a prefix: zone 1 is not zone 10.
    assert_eq!(codes(&by_name("Alaska zone 1")).len(), 1);
    assert_eq!(codes(&by_name("Alaska")).len(), 10);
    // A zone found by name is found again by a point inside it, and every
    // code the lookup gives is one the projection tool accepts.
    let denver = at(39.7392, -104.9903);
    let found = codes(&denver);
    assert!(!found.is_empty(), "Denver matched no zone");
    for epsg in &found {
        let p = call(
            "geodesy.spcs.spcs83-forward",
            &format!(r#"{{"lat":39.7392,"lon":-104.9903,"zone":"{epsg}"}}"#),
        );
        assert_eq!(
            p["ok"], true,
            "EPSG {epsg} is not a zone the projector takes: {p}"
        );
    }
    // The middle of the Pacific belongs to no zone, and says so.
    assert_eq!(num(&at(0.0, -150.0), "result.count"), 0.0);
}
