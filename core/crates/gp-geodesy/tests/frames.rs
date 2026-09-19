//! Ellipsoids and frames: the reference-frames spec scenarios and round-trip
//! properties over many points.

use gp_geo::ellipsoid::CATALOG;
use gp_geo::frames::{self as fr, Aux};
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

fn codes(r: &Value) -> Vec<String> {
    r["meta"]["warnings"].as_array().map_or(vec![], |a| {
        a.iter()
            .map(|w| w["code"].as_str().unwrap().to_owned())
            .collect()
    })
}

/// A small deterministic generator (xorshift64*), uniform in [0, 1).
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
    }
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next()
    }
}

#[test]
fn wgs84_parameters() {
    // reference-frames "WGS 84 parameters".
    let r = call("geodesy.ellipsoid.parameters", r#"{"ellipsoid":"wgs84"}"#);
    assert_eq!(num(&r, "result.a.value"), 6_378_137.0);
    assert_eq!(num(&r, "result.inverse_flattening"), 298.257_223_563);
    assert!(
        (num(&r, "result.b.value") - 6_356_752.314_245).abs() < 1e-6,
        "{r}"
    );
    // A sphere has no inverse flattening.
    let s = call(
        "geodesy.ellipsoid.parameters",
        r#"{"a":6371000,"inverse_flattening":0}"#,
    );
    assert!(s["result"]["inverse_flattening"].is_null());
    assert_eq!(num(&s, "result.authalic_radius.value"), 6_371_000.0);
}

#[test]
fn degree_lengths_at_45() {
    // reference-frames "Degree lengths at 45°".
    let r = call("geodesy.ellipsoid.radii", r#"{"lat":45}"#);
    assert!(
        (num(&r, "result.degree_lat.value") - 111_131.78).abs() < 0.01,
        "{r}"
    );
    assert!(
        (num(&r, "result.degree_lon.value") - 78_846.84).abs() < 0.01,
        "{r}"
    );
    assert_eq!(
        num(
            &call("geodesy.ellipsoid.radii", r#"{"lat":90}"#),
            "result.degree_lon.value"
        ),
        0.0
    );
}

#[test]
fn geocentric_latitude_and_round_trips() {
    // reference-frames "Geocentric latitude".
    let r = call("geodesy.ellipsoid.auxiliary-latitude", r#"{"latitude":45}"#);
    let g = num(&r, "result.geocentric.value");
    assert!((g - 44.8076).abs() < 5e-5, "{r}");
    let back = call(
        "geodesy.ellipsoid.auxiliary-latitude",
        &format!(r#"{{"latitude":{g},"from":"geocentric"}}"#),
    );
    assert!(
        (num(&back, "result.geodetic.value") - 45.0).abs() < 1e-12,
        "{back}"
    );
    // Every kind, every catalog ellipsoid, pole to pole: round trips within 1e-12°.
    let kinds = [
        Aux::Geocentric,
        Aux::Parametric,
        Aux::Rectifying,
        Aux::Conformal,
        Aux::Authalic,
        Aux::Isometric,
    ];
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut worst = 0f64;
    for ell in CATALOG {
        for k in 0..2_000 {
            let lat = if k < 8 {
                [0.0, 1e-9, 45.0, 89.0, 89.9999, -89.9999, -45.0, 60.0][k]
            } else {
                rng.range(-89.99999, 89.99999)
            };
            let phi = lat.to_radians();
            let a = ell.auxiliary(phi);
            for (kind, v) in kinds.into_iter().zip([
                a.geocentric,
                a.parametric,
                a.rectifying,
                a.conformal,
                a.authalic,
                a.isometric,
            ]) {
                let err = (ell.geodetic_from(kind, v) - phi).to_degrees().abs();
                worst = worst.max(err);
                assert!(err < 1e-12, "{} {kind:?} at {lat}: {err:e}", ell.id);
            }
        }
    }
    eprintln!("worst auxiliary round trip {worst:e}°");
}

#[test]
fn ecef_scenarios() {
    // reference-frames "Forward conversion".
    let r = call(
        "geodesy.frame.geodetic-to-ecef",
        r#"{"lat":40.446111,"lon":-79.982222,"height":300}"#,
    );
    for (k, want) in [
        ("x", 845_580.010),
        ("y", -4_786_836.717),
        ("z", 4_116_002.385),
    ] {
        assert!(
            (num(&r, &format!("result.{k}.value")) - want).abs() < 1e-3,
            "{k}: {r}"
        );
    }
    // "Pole": the exact semi-minor axis gives latitude 90, height 0, longitude 0 with a warning.
    let b = CATALOG[0].b();
    let p = call(
        "geodesy.frame.ecef-to-geodetic",
        &format!(r#"{{"x":0,"y":0,"z":{b}}}"#),
    );
    assert_eq!(num(&p, "result.lat.value"), 90.0);
    assert!(num(&p, "result.height.value").abs() < 1e-9, "{p}");
    assert_eq!(num(&p, "result.lon.value"), 0.0);
    assert!(codes(&p).contains(&"LONGITUDE_UNDEFINED".to_owned()));
    // "Earth's center is degenerate".
    let o = call("geodesy.frame.ecef-to-geodetic", r#"{"x":0,"y":0,"z":0}"#);
    assert_eq!(o["error"]["code"], "DEGENERATE_GEOMETRY");
    assert_eq!(
        call(
            "geodesy.frame.ecef-to-geodetic",
            r#"{"x":0.5,"y":0,"z":0.5}"#
        )["error"]["code"],
        "DEGENERATE_GEOMETRY"
    );
}

#[test]
fn ecef_round_trips_at_every_height() {
    // Geodetic → ECEF → geodetic → ECEF: the position closes within a few
    // units in the last place of the coordinates, from −10 km to 100,000 km.
    let mut rng = Rng(42);
    let mut worst_near = 0f64;
    let mut worst_rel = 0f64;
    for ell in CATALOG {
        for k in 0..20_000 {
            let lat = rng.range(-90.0, 90.0);
            let lon = rng.range(-180.0, 180.0);
            let h = match k % 4 {
                0 => rng.range(-10_000.0, 10_000.0),
                1 => rng.range(0.0, 1e6),
                2 => rng.range(0.0, 1e8),
                _ => 0.0,
            };
            let p = fr::to_ecef(&ell, lat.to_radians(), lon.to_radians(), h);
            let (phi, lam, hh) = fr::from_ecef(&ell, p.0, p.1, p.2).unwrap();
            let q = fr::to_ecef(&ell, phi, lam, hh);
            let err = ((p.0 - q.0).powi(2) + (p.1 - q.1).powi(2) + (p.2 - q.2).powi(2)).sqrt();
            let size = (p.0 * p.0 + p.1 * p.1 + p.2 * p.2).sqrt();
            if h.abs() <= 10_000.0 {
                worst_near = worst_near.max(err);
            }
            worst_rel = worst_rel.max(err / size);
        }
    }
    eprintln!("ECEF round trip: {worst_near:e} m near the surface, {worst_rel:e} relative");
    // One unit in the last place of a coordinate at the Earth's radius is 0.93 nm.
    assert!(worst_near < 6e-9, "{worst_near:e} m");
    assert!(worst_rel < 1e-15, "{worst_rel:e}");
}

#[test]
fn local_frames() {
    // reference-frames "AER of an overhead target".
    let r = call(
        "geodesy.frame.to-local",
        r#"{"lat0":40,"lon0":-105,"h0":1600,"lat":40,"lon":-105,"height":2600}"#,
    );
    assert_eq!(num(&r, "result.elevation.value"), 90.0);
    assert!((num(&r, "result.range.value") - 1000.0).abs() < 1e-9, "{r}");
    assert_eq!(num(&r, "result.azimuth.value"), 0.0);
    assert!(codes(&r).contains(&"AZIMUTH_UNDEFINED".to_owned()));
    // The rotation matrix on request is orthonormal.
    let m = call(
        "geodesy.frame.to-local",
        r#"{"lat0":40,"lon0":-105,"lat":41,"lon":-104,"matrix":"yes"}"#,
    );
    let rows: Vec<[f64; 3]> = m["result"]["rotation"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            [
                r["x"].as_f64().unwrap(),
                r["y"].as_f64().unwrap(),
                r["z"].as_f64().unwrap(),
            ]
        })
        .collect();
    for i in 0..3 {
        for j in 0..3 {
            let dot: f64 = (0..3).map(|k| rows[i][k] * rows[j][k]).sum();
            assert!((dot - if i == j { 1.0 } else { 0.0 }).abs() < 1e-15);
        }
    }
    // "ENU round trip": 10,000 points within 1,000 km of random origins.
    let ell = CATALOG[0];
    let mut rng = Rng(7);
    let mut worst = 0f64;
    for _ in 0..10_000 {
        let (lat0, lon0, h0) = (
            rng.range(-89.0, 89.0),
            rng.range(-180.0, 180.0),
            rng.range(-100.0, 5000.0),
        );
        let (phi0, lam0) = (lat0.to_radians(), lon0.to_radians());
        let o = fr::to_ecef(&ell, phi0, lam0, h0);
        let enu = [
            rng.range(-7e5, 7e5),
            rng.range(-7e5, 7e5),
            rng.range(-1e4, 1e5),
        ];
        let p = fr::enu_to_ecef(o, phi0, lam0, enu);
        let (phi, lam, h) = fr::from_ecef(&ell, p.0, p.1, p.2).unwrap();
        let back = fr::ecef_to_enu(o, phi0, lam0, fr::to_ecef(&ell, phi, lam, h));
        let err = (0..3)
            .map(|i| (back[i] - enu[i]).powi(2))
            .sum::<f64>()
            .sqrt();
        worst = worst.max(err);
    }
    eprintln!("ENU round trip {worst:e} m");
    assert!(worst < 1e-8, "{worst:e} m");
    // AER ↔ ENU through the tools.
    let f = call(
        "geodesy.frame.from-local",
        r#"{"lat0":40,"lon0":-105,"h0":1600,"frame":"aer","azimuth":45,"elevation":5,"range":10000}"#,
    );
    let back = call(
        "geodesy.frame.to-local",
        &format!(
            r#"{{"lat0":40,"lon0":-105,"h0":1600,"lat":{},"lon":{},"height":{}}}"#,
            num(&f, "result.lat.value"),
            num(&f, "result.lon.value"),
            num(&f, "result.height.value")
        ),
    );
    assert!(
        (num(&back, "result.azimuth.value") - 45.0).abs() < 1e-9,
        "{back}"
    );
    assert!((num(&back, "result.elevation.value") - 5.0).abs() < 1e-9);
    assert!((num(&back, "result.range.value") - 10_000.0).abs() < 1e-6);
}
