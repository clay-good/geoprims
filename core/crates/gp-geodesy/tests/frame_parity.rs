//! The ECEF and local-frame tools against GeographicLib's CartConvert (C++),
//! through the public tool interface. Fixtures from
//! tools/vectors/gen_frame_diff.py.

use gp_geodesy::REGISTRY;
use serde_json::{Value, json};

fn call(id: &str, input: &Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).expect("envelope is JSON")
}

fn rows(name: &str) -> Vec<Vec<f64>> {
    let path = format!("{}/tests/data/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {path}: {e}"))
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.split(',').map(|v| v.parse().unwrap()).collect())
        .collect()
}

fn v(r: &Value, k: &str) -> f64 {
    r["result"][k]["value"].as_f64().unwrap_or_else(|| panic!("{k} missing in {r}"))
}

/// Longitude difference in degrees, across the antimeridian.
fn dlon(a: f64, b: f64) -> f64 {
    ((a - b + 540.0).rem_euclid(360.0) - 180.0).abs()
}

#[test]
fn ecef_matches_cartconvert() {
    let rows = rows("ecef_diff.csv");
    assert_eq!(rows.len(), 1000);
    let (mut worst_xyz, mut worst_h, mut worst_deg) = (0f64, 0f64, 0f64);
    for c in &rows {
        let f = call("geodesy.frame.geodetic-to-ecef", &json!({"lat": c[0], "lon": c[1], "height": c[2]}));
        for (k, want) in ["x", "y", "z"].iter().zip(&c[3..6]) {
            // CartConvert prints 9 decimals; allow that plus 1e-15 of the radius.
            let err = (v(&f, k) - want).abs();
            worst_xyz = worst_xyz.max(err);
            assert!(err < 1e-8 + 1e-15 * want.abs().max(6.4e6), "{c:?} {k}: {err:e} m");
        }
        let r = call("geodesy.frame.ecef-to-geodetic", &json!({"x": c[3], "y": c[4], "z": c[5]}));
        let dlat = (v(&r, "lat") - c[6]).abs();
        let dl = if c[6].abs() == 90.0 { 0.0 } else { dlon(v(&r, "lon"), c[7]) };
        let dh = (v(&r, "height") - c[8]).abs();
        worst_deg = worst_deg.max(dlat.max(dl));
        worst_h = worst_h.max(dh);
        assert!(dlat < 1e-11 && dl < 1e-11, "{c:?}: {dlat:e}°, {dl:e}°");
        assert!(dh < 1e-8 + 1e-15 * c[8].abs(), "{c:?}: {dh:e} m");
    }
    eprintln!("ECEF vs CartConvert: {worst_xyz:e} m, {worst_deg:e}°, {worst_h:e} m");
}

#[test]
fn local_frames_match_cartconvert() {
    let rows = rows("enu_diff.csv");
    assert_eq!(rows.len(), 500);
    let mut worst = 0f64;
    for c in &rows {
        let (o, t) = (&c[0..3], &c[3..6]);
        let r = call(
            "geodesy.frame.to-local",
            &json!({"lat0": o[0], "lon0": o[1], "h0": o[2], "lat": t[0], "lon": t[1], "height": t[2]}),
        );
        for (k, want) in ["east", "north", "up"].iter().zip(&c[6..9]) {
            let err = (v(&r, k) - want).abs();
            worst = worst.max(err);
            assert!(err < 1e-8, "{c:?} {k}: {err:e} m");
        }
        let b = call(
            "geodesy.frame.from-local",
            &json!({"lat0": o[0], "lon0": o[1], "h0": o[2], "frame": "enu", "east": c[6], "north": c[7], "up": c[8]}),
        );
        assert!((v(&b, "lat") - t[0]).abs() < 1e-11, "{c:?}: {b}");
        // At a pole the longitude is arbitrary.
        assert!(t[0].abs() == 90.0 || dlon(v(&b, "lon"), t[1]) < 1e-11, "{c:?}: {b}");
        assert!((v(&b, "height") - t[2]).abs() < 1e-8, "{c:?}: {b}");
    }
    eprintln!("ENU vs CartConvert: {worst:e} m");
}
