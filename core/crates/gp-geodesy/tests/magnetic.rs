//! Geomagnetism: every spec scenario the WMM2025 and IGRF-14 tools cover.

use gp_geodesy::REGISTRY;
use serde_json::Value;

fn call(id: &str, input: &str) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, input)).expect("envelope is JSON")
}

fn num(r: &Value, path: &str) -> f64 {
    path.split('.')
        .fold(r, |v, k| &v[k])
        .as_f64()
        .unwrap_or_else(|| panic!("{path} missing in {r}"))
}

fn warns(r: &Value, code: &str) -> bool {
    r["meta"]["warnings"]
        .as_array()
        .is_some_and(|w| w.iter().any(|w| w["code"] == code))
}

const D: &str = "geodesy.magnetic.declination";

#[test]
fn every_official_wmm2025_test_value() {
    let text = include_str!("data/WMM2025_TestValues.txt");
    let mut n = 0;
    for l in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
    {
        let f: Vec<&str> = l.split_whitespace().collect();
        let v: Vec<f64> = f.iter().map(|t| t.parse().unwrap()).collect();
        let r = call(
            D,
            &format!(
                r#"{{"lat":{},"lon":{},"height":"{} km","date":"{}"}}"#,
                f[2], f[3], f[1], f[0]
            ),
        );
        assert_eq!(r["ok"], true, "{r}");
        for (path, want, tol) in [
            ("result.declination.value", v[4], 0.005),
            ("result.inclination.value", v[5], 0.005),
            ("result.horizontal_intensity", v[6], 1e-3),
            ("result.north", v[7], 1e-3),
            ("result.east", v[8], 1e-3),
            ("result.down", v[9], 1e-3),
            ("result.total_intensity", v[10], 1e-3),
            ("result.annual_change.value", v[11], 1e-5),
            ("result.inclination_rate", v[12], 1e-5),
        ] {
            assert!(
                (num(&r, path) - want).abs() <= tol,
                "{l}\n{path}: {} vs {want}",
                num(&r, path)
            );
        }
        assert_eq!(r["meta"]["assets"][0]["id"], "wmm2025");
        n += 1;
    }
    assert_eq!(n, 100);
}

#[test]
fn uncertainty_formula() {
    let r = call(D, r#"{"lat":40,"lon":-105,"date":"2026-01-01"}"#);
    let h = num(&r, "result.horizontal_intensity");
    let want = (0.26f64.powi(2) + (5417.0 / h).powi(2)).sqrt();
    assert!((num(&r, "result.declination_uncertainty.value") - want).abs() < 1e-12);
    assert_eq!(r["result"]["compass_zone"], "normal");
    assert!(r["meta"]["model"].as_str().unwrap().starts_with("WMM2025"));
}

#[test]
fn blackout_zone_near_the_magnetic_pole() {
    let r = call(D, r#"{"lat":86,"lon":140,"date":"2026-01-01"}"#);
    assert!(num(&r, "result.horizontal_intensity") < 2000.0, "{r}");
    assert!(warns(&r, "COMPASS_BLACKOUT_ZONE"));
    assert!(num(&r, "result.declination_uncertainty.value") > 2.0);
}

#[test]
fn caution_zone() {
    // Find a point with 2,000 ≤ H < 6,000 nT along 80° N.
    let mut hit = false;
    for lon in (-180..180).step_by(10) {
        let r = call(
            D,
            &format!(r#"{{"lat":80,"lon":{lon},"date":"2026-01-01"}}"#),
        );
        let h = num(&r, "result.horizontal_intensity");
        if (2000.0..6000.0).contains(&h) {
            assert!(warns(&r, "COMPASS_CAUTION_ZONE"), "{r}");
            assert_eq!(r["result"]["compass_zone"], "caution");
            hit = true;
        }
    }
    assert!(hit);
}

#[test]
fn historical_date_suggests_igrf() {
    let r = call(D, r#"{"lat":40,"lon":-105,"date":"1985-06-01"}"#);
    assert_eq!(r["error"]["code"], "OUT_OF_DOMAIN", "{r}");
    assert!(r["error"]["hint"].as_str().unwrap().contains("IGRF-14"));
    let r = call(
        D,
        r#"{"lat":40,"lon":-105,"date":"1985-06-01","model":"igrf14"}"#,
    );
    assert_eq!(r["ok"], true, "{r}");
    assert!(r["meta"]["model"].as_str().unwrap().contains("definitive"));
    assert_eq!(r["meta"]["assets"][0]["id"], "igrf14");
    assert!(r["result"].get("declination_uncertainty").is_none());
}

#[test]
fn after_the_validity_window() {
    let r = call(D, r#"{"lat":40,"lon":-105,"date":"2030.5"}"#);
    assert_eq!(r["error"]["code"], "OUT_OF_DOMAIN");
    let m = r["error"]["message"].as_str().unwrap();
    assert!(m.contains("2025") && m.contains("2030"), "{m}");
}

#[test]
fn geographic_pole_convention() {
    let r = call(D, r#"{"lat":90,"lon":0,"date":"2026-01-01"}"#);
    assert!(warns(&r, "DECLINATION_POLE_CONVENTION"), "{r}");
    // Longitude is irrelevant at the pole except through the convention: the east
    // component comes from the pole-safe sum, and stays finite.
    assert!(num(&r, "result.east").is_finite());
}

#[test]
fn igrf_and_wmm_agree_in_2026() {
    let w = call(D, r#"{"lat":40,"lon":-105,"date":"2026-06-01"}"#);
    let i = call(
        D,
        r#"{"lat":40,"lon":-105,"date":"2026-06-01","model":"igrf14"}"#,
    );
    // Two independent models: within a few tenths of a degree.
    assert!(
        (num(&w, "result.declination.value") - num(&i, "result.declination.value")).abs() < 0.3
    );
}

#[test]
fn bad_dates() {
    for d in ["2026-02-30", "18/09/2026", "soon"] {
        let r = call(D, &format!(r#"{{"lat":40,"lon":-105,"date":"{d}"}}"#));
        assert_eq!(r["error"]["code"], "INVALID_INPUT", "{d}: {r}");
    }
}

const T: &str = "geodesy.magnetic.true-to-magnetic";

#[test]
fn chart_variation() {
    let r = call(T, r#"{"bearing":90,"variation":"12°W"}"#);
    assert!(
        (num(&r, "result.result.value") - 102.0).abs() < 1e-12,
        "{r}"
    );
    assert_eq!(r["result"]["variation_text"], "12° W");
}

#[test]
fn chart_versus_model() {
    let r = call(
        T,
        r#"{"bearing":90,"variation":"8°E","lat":40.015,"lon":-105.27,"date":"2026-09-18"}"#,
    );
    assert_eq!(r["result"]["variation_source"], "chart");
    let m = num(&r, "result.model_declination.value");
    assert!((num(&r, "result.difference.value") - (8.0 - m)).abs() < 1e-12);
    assert!(
        r["summary"]
            .as_str()
            .unwrap()
            .contains("differs from the chart")
    );
}

#[test]
fn model_only_and_nothing() {
    let r = call(
        T,
        r#"{"bearing":0,"lat":40.015,"lon":-105.27,"date":"2026-09-18"}"#,
    );
    assert_eq!(r["result"]["variation_source"], "model");
    let r = call(T, r#"{"bearing":0}"#);
    assert_eq!(r["error"]["code"], "INVALID_INPUT");
}

#[test]
fn field_element_invariants() {
    // H² = X² + Y², F² = H² + Z², D = atan2(Y, X), I = atan2(Z, H), for both
    // models, at the surface and aloft, and the ±180° meridian agrees.
    for (model, date) in [("wmm2025", "2027.5"), ("igrf14", "1965.0")] {
        for lat in (-85..=85).step_by(17) {
            for lon in (-180..=180).step_by(45) {
                for h in [0, 100_000] {
                    let r = call(
                        D,
                        &format!(
                            r#"{{"lat":{lat},"lon":{lon},"height":{h},"date":"{date}","model":"{model}"}}"#
                        ),
                    );
                    let (x, y, z) = (
                        num(&r, "result.north"),
                        num(&r, "result.east"),
                        num(&r, "result.down"),
                    );
                    let hh = num(&r, "result.horizontal_intensity");
                    let f = num(&r, "result.total_intensity");
                    assert!((hh - x.hypot(y)).abs() < 1e-9 * f, "{r}");
                    assert!((f - hh.hypot(z)).abs() < 1e-9 * f, "{r}");
                    let d = num(&r, "result.declination.value");
                    assert!((d - y.atan2(x).to_degrees()).abs() < 1e-9, "{r}");
                    let i = num(&r, "result.inclination.value");
                    assert!((i - z.atan2(hh).to_degrees()).abs() < 1e-9, "{r}");
                    assert!(d > -180.0 && d <= 180.0);
                }
            }
            let west = call(
                D,
                &format!(r#"{{"lat":{lat},"lon":-180,"date":"{date}","model":"{model}"}}"#),
            );
            let east = call(
                D,
                &format!(r#"{{"lat":{lat},"lon":180,"date":"{date}","model":"{model}"}}"#),
            );
            assert!(
                (num(&west, "result.declination.value") - num(&east, "result.declination.value"))
                    .abs()
                    < 1e-9
            );
        }
    }
}

#[test]
fn meta_carries_the_magnetic_caveats() {
    // mcp "Magnetic caveat": model, epoch, validity window, uncertainty, and
    // the compass zone, plus the not-for-navigation notice, all in meta.
    let r = call(D, r#"{"lat":40,"lon":-105,"date":"2026-09-19"}"#);
    let c = &r["meta"]["context"];
    assert_eq!(c["model"], "wmm2025", "{r}");
    assert!((c["epoch"].as_f64().unwrap() - 2026.716).abs() < 0.01);
    assert_eq!(
        (c["validFrom"].as_f64(), c["validTo"].as_f64()),
        (Some(2025.0), Some(2030.0))
    );
    assert!(c["declinationUncertaintyDeg"].as_f64().unwrap() > 0.2);
    assert_eq!(c["compassZone"], "normal");
    assert_eq!(
        r["meta"]["notice"],
        "Planning and education aid. Not for primary navigation."
    );
    let polar = call(D, r#"{"lat":86,"lon":150,"date":"2026-09-19"}"#);
    assert_ne!(polar["meta"]["context"]["compassZone"], "normal", "{polar}");
}
