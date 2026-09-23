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
            let p = fr::to_ecef(ell, lat.to_radians(), lon.to_radians(), h);
            let (phi, lam, hh) = fr::from_ecef(ell, p.0, p.1, p.2).unwrap();
            let q = fr::to_ecef(ell, phi, lam, hh);
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

#[test]
fn frame_tool_invariants() {
    // Through the four tools, on three ellipsoids: geodetic → ECEF → geodetic
    // returns the input; raising a point by h moves it exactly h along the
    // normal; the origin is at (0, 0, 0) in its own local frame; range is the
    // length of (east, north, up); and to-local then from-local returns the
    // target.
    let mut rng = Rng(99);
    for ell in ["wgs84", "grs80", "clarke1866"] {
        for _ in 0..200 {
            let (lat, lon, h) = (
                rng.range(-89.0, 89.0),
                rng.range(-180.0, 180.0),
                rng.range(-500.0, 2e5),
            );
            let ecef = |h: f64| {
                call(
                    "geodesy.frame.geodetic-to-ecef",
                    &serde_json::json!({"lat": lat, "lon": lon, "height": h, "ellipsoid": ell})
                        .to_string(),
                )
            };
            let p = ecef(h);
            let (x, y, z) = (
                num(&p, "result.x.value"),
                num(&p, "result.y.value"),
                num(&p, "result.z.value"),
            );
            let g = call(
                "geodesy.frame.ecef-to-geodetic",
                &serde_json::json!({"x": x, "y": y, "z": z, "ellipsoid": ell}).to_string(),
            );
            assert!((num(&g, "result.lat.value") - lat).abs() < 1e-11);
            assert!((num(&g, "result.lon.value") - lon).abs() < 1e-11);
            assert!((num(&g, "result.height.value") - h).abs() < 1e-8);
            let s = ecef(0.0);
            let d = ((x - num(&s, "result.x.value")).powi(2)
                + (y - num(&s, "result.y.value")).powi(2)
                + (z - num(&s, "result.z.value")).powi(2))
            .sqrt();
            assert!((d - h.abs()).abs() < 1e-8, "{d} vs {h}");

            let o = serde_json::json!({"lat0": lat, "lon0": lon, "h0": h, "ellipsoid": ell});
            let local = |t: (f64, f64, f64)| {
                let mut q = o.clone();
                q["lat"] = t.0.into();
                q["lon"] = t.1.into();
                q["height"] = t.2.into();
                call("geodesy.frame.to-local", &q.to_string())
            };
            let at = local((lat, lon, h));
            for k in ["east", "north", "up", "range"] {
                assert!(num(&at, &format!("result.{k}.value")).abs() < 1e-8, "{at}");
            }
            let t = (
                (lat + rng.range(-3.0, 3.0)).clamp(-89.9, 89.9),
                lon + rng.range(-3.0, 3.0),
                rng.range(-100.0, 3e4),
            );
            let r = local(t);
            let (e, n, u) = (
                num(&r, "result.east.value"),
                num(&r, "result.north.value"),
                num(&r, "result.up.value"),
            );
            assert!((num(&r, "result.range.value") - (e * e + n * n + u * u).sqrt()).abs() < 1e-8);
            let mut q = o.clone();
            q["frame"] = "enu".into();
            q["east"] = e.into();
            q["north"] = n.into();
            q["up"] = u.into();
            let b = call("geodesy.frame.from-local", &q.to_string());
            assert!((num(&b, "result.lat.value") - t.0).abs() < 1e-11, "{b}");
            let dl = (num(&b, "result.lon.value") - t.1 + 540.0).rem_euclid(360.0) - 180.0;
            assert!(dl.abs() < 1e-11, "{b}");
            assert!((num(&b, "result.height.value") - t.2).abs() < 1e-8, "{b}");
        }
    }
}

/// Layer E for `geodesy.ellipsoid.parameters`. The derived values are not
/// independent of one another: each follows exactly from the two that define
/// the ellipsoid, and the three radii sit in a known order.
#[test]
fn ellipsoid_parameter_invariants() {
    for name in ["wgs84", "grs80", "airy1830", "clarke1866", "intl1924"] {
        let r = call(
            "geodesy.ellipsoid.parameters",
            &format!(r#"{{"ellipsoid":"{name}"}}"#),
        );
        assert_eq!(r["ok"], true, "{r}");
        let g = |k: &str| num(&r, k);
        let (a, b) = (g("result.a.value"), g("result.b.value"));
        let (f, e2, ep2, n) = (
            g("result.flattening"),
            g("result.e2"),
            g("result.ep2"),
            g("result.n"),
        );
        let close = |got: f64, want: f64, what: &str| {
            assert!(
                (got - want).abs() <= 1e-12 * want.abs().max(1.0),
                "{name}: {what} is {got}, and the defining values give {want}"
            );
        };
        // Every derived value against the two that define the figure.
        close(b, a * (1.0 - f), "b = a(1 − f)");
        close(e2, f * (2.0 - f), "e² = f(2 − f)");
        close(ep2, e2 / (1.0 - e2), "e′² = e²/(1 − e²)");
        close(n, f / (2.0 - f), "n = f/(2 − f)");
        close(
            g("result.mean_radius.value"),
            (2.0 * a + b) / 3.0,
            "R₁ = (2a + b)/3",
        );
        // A sphere of equal area and one of equal volume both lie between the
        // axes, and below the mean radius, which weights the equator twice.
        for (radius, what) in [
            (g("result.authalic_radius.value"), "the authalic radius"),
            (g("result.volumetric_radius.value"), "the volumetric radius"),
        ] {
            assert!(
                b < radius && radius < a,
                "{name}: {what} is outside the axes"
            );
            assert!(
                radius < g("result.mean_radius.value"),
                "{name}: {what} is above the mean radius"
            );
        }
        assert!(b < a, "{name}: the ellipsoid is not oblate");
    }
}

/// Layer E for `geodesy.ellipsoid.radii`. The two radii of curvature bracket
/// their Gaussian mean and meet at the poles, Euler's formula gives the radius
/// in any azimuth from them, a degree of longitude shrinks as the cosine of
/// latitude, and the meridian arc grows without turning back.
#[test]
fn ellipsoid_radii_invariants() {
    let at = |lat: f64, extra: &str| {
        call(
            "geodesy.ellipsoid.radii",
            &format!(r#"{{"lat":{lat}{extra}}}"#),
        )
    };
    let mut last_arc = -1.0;
    for lat in [0.0, 15.0, 30.0, 45.0, 60.0, 75.0, 89.0] {
        let r = at(lat, "");
        assert_eq!(r["ok"], true, "{r}");
        let (m, n, g) = (
            num(&r, "result.meridional.value"),
            num(&r, "result.prime_vertical.value"),
            num(&r, "result.gaussian.value"),
        );
        // On an oblate ellipsoid the meridian bends more sharply than the
        // prime vertical everywhere but the poles, and the Gaussian mean is
        // the geometric mean of the two, so it sits between them.
        assert!(m < n, "at {lat}: the meridional radius is not the smaller");
        assert!(
            m < g && g < n,
            "at {lat}: the Gaussian mean is outside them"
        );
        assert!(
            (g - (m * n).sqrt()).abs() < 1e-6,
            "at {lat}: the Gaussian mean is not sqrt(MN)"
        );
        // Euler: 1/R(alpha) = cos^2(alpha)/M + sin^2(alpha)/N. Due north is M,
        // due east is N, and 45 degrees follows from both.
        for (azimuth, want) in [(0.0, m), (90.0, n)] {
            let e = num(
                &at(lat, &format!(r#","azimuth":{azimuth}"#)),
                "result.in_azimuth.value",
            );
            assert!(
                (e - want).abs() < 1e-6,
                "at {lat} azimuth {azimuth}: {e} not {want}"
            );
        }
        let diagonal = num(&at(lat, r#","azimuth":45"#), "result.in_azimuth.value");
        let euler = 1.0 / (0.5 / m + 0.5 / n);
        assert!(
            (diagonal - euler).abs() < 1e-6,
            "at {lat}: 45 degrees gives {diagonal}, Euler gives {euler}"
        );
        // A degree of longitude is a degree of the prime vertical circle,
        // which shrinks with the cosine of latitude; a degree of latitude
        // does not shrink at all, and at 45 degrees they cross.
        let (dlat, dlon) = (
            num(&r, "result.degree_lat.value"),
            num(&r, "result.degree_lon.value"),
        );
        let want_lon = n * lat.to_radians().cos() * std::f64::consts::PI / 180.0;
        assert!(
            (dlon - want_lon).abs() < 1e-6,
            "at {lat}: a degree of longitude is {dlon}"
        );
        assert!(dlat > 0.0 && dlon >= 0.0);
        if lat > 0.0 {
            assert!(
                dlon < dlat,
                "at {lat}: longitude has not fallen below latitude"
            );
        }
        // The meridian arc from the equator only grows going north.
        let arc = num(&r, "result.meridian_arc.value");
        assert!(arc > last_arc, "the meridian arc turned back at {lat}");
        last_arc = arc;
    }
}

#[test]
fn auxiliary_latitude_invariants() {
    const AUX: &str = "geodesy.ellipsoid.auxiliary-latitude";
    // The order they take through the northern hemisphere. Rectifying comes
    // before authalic, which is easy to write down the other way round.
    const ORDER: [&str; 6] = [
        "geocentric",
        "conformal",
        "rectifying",
        "authalic",
        "parametric",
        "geodetic",
    ];
    let aux = |lat: f64| call(AUX, &format!(r#"{{"latitude":{lat}}}"#));
    let (a, b) = (CATALOG[0].a, CATALOG[0].a * (1.0 - CATALOG[0].f));

    for i in 0..=90 {
        let phi = i as f64;
        let r = aux(phi);
        assert!(r["ok"].as_bool().unwrap_or(false), "{phi}: {r}");

        // Ordered, and odd in the latitude.
        let mut last = f64::NEG_INFINITY;
        for k in ORDER {
            let v = num(&r, &format!("result.{k}.value"));
            assert!(v >= last - 1e-12, "{phi}: {k} = {v} below {last}\n{r}");
            last = v;
        }
        let neg = aux(-phi);
        for k in ORDER.iter().chain(["isometric"].iter()) {
            let (p, m) = (
                num(&r, &format!("result.{k}.value")),
                num(&neg, &format!("result.{k}.value")),
            );
            assert!((p + m).abs() < 1e-12, "{phi}: {k} is not odd: {p} and {m}");
        }
        // The equator and the poles are fixed, except that the isometric
        // latitude has no pole to reach.
        if i == 0 || i == 90 {
            for k in ORDER {
                assert!(
                    (num(&r, &format!("result.{k}.value")) - phi).abs() < 1e-9,
                    "{phi}: {k}\n{r}"
                );
            }
        }

        // The conformal latitude is the Gudermannian of the isometric: two of
        // the outputs, tied to each other rather than each to the same source.
        if i < 90 {
            let psi: f64 = num(&r, "result.isometric.value").to_radians();
            let chi = psi.sinh().atan().to_degrees();
            assert!(
                (chi - num(&r, "result.conformal.value")).abs() < 1e-11,
                "{phi}: conformal from isometric {chi}\n{r}"
            );
        }

        // The geocentric and parametric latitudes are the ones the ECEF tool
        // implies from its own X and Z, which knows nothing of this tool.
        let e = call(
            "geodesy.frame.geodetic-to-ecef",
            &format!(r#"{{"lat":{phi},"lon":0,"height":0}}"#),
        );
        let (x, z) = (num(&e, "result.x.value"), num(&e, "result.z.value"));
        assert!(
            (z.atan2(x).to_degrees() - num(&r, "result.geocentric.value")).abs() < 1e-9,
            "{phi}: geocentric against ECEF\n{r}\n{e}"
        );
        assert!(
            ((z / b).atan2(x / a).to_degrees() - num(&r, "result.parametric.value")).abs() < 1e-9,
            "{phi}: parametric against ECEF\n{r}\n{e}"
        );

        // Every latitude reads back as the geodetic one it came from.
        for k in ORDER.iter().chain(["isometric"].iter()) {
            if *k == "geodetic" {
                continue;
            }
            let v = num(&r, &format!("result.{k}.value"));
            let back = call(AUX, &format!(r#"{{"latitude":{v},"from":"{k}"}}"#));
            assert!(
                (num(&back, "result.geodetic.value") - phi).abs() < 1e-12,
                "{phi}: {k} = {v} came back as {}\n{back}",
                num(&back, "result.geodetic.value")
            );
        }
    }
}
