//! The focused geodesy operations: what each must do, as their derivation
//! notes state.

use gp_geodesy::REGISTRY;
use serde_json::{Value, json};

fn call(id: &str, input: Value) -> Value {
    let r: Value = serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).unwrap();
    assert_eq!(r["ok"], true, "{id} {input}: {r}");
    r["result"].clone()
}

fn num(r: &Value, k: &str) -> f64 {
    r[k]["value"]
        .as_f64()
        .or(r[k].as_f64())
        .unwrap_or_else(|| panic!("{k}: {r}"))
}

fn text(r: &Value, k: &str) -> String {
    r[k].as_str()
        .unwrap_or_else(|| panic!("{k}: {r}"))
        .to_owned()
}

#[test]
fn ellipsoid_views_invariants() {
    let deg = |lat: f64| call("geodesy.ellipsoid.degree-lengths", json!({ "lat": lat }));
    let (pole, equator) = (deg(90.0), deg(0.0));
    assert!(num(&pole, "degree_lon").abs() < 1e-6);
    let mut last_lon = f64::INFINITY;
    let mut last_lat = 0.0;
    for lat in [0.0, 15.0, 30.0, 45.0, 60.0, 75.0, 90.0] {
        let d = deg(lat);
        // Longitude degrees shrink toward the pole; latitude degrees grow.
        assert!(
            num(&d, "degree_lon") <= last_lon
                && num(&d, "degree_lon") <= num(&equator, "degree_lon")
        );
        assert!(num(&d, "degree_lat") >= last_lat);
        (last_lon, last_lat) = (num(&d, "degree_lon"), num(&d, "degree_lat"));
        // 3,600 arcseconds and 60 arcminutes make the degree.
        for axis in ["lat", "lon"] {
            let whole = num(&d, &format!("degree_{axis}"));
            assert!(
                (num(&d, &format!("arcsecond_{axis}")) * 3600.0 - whole).abs()
                    <= 1e-9 * whole.max(1.0)
            );
            assert!(
                (num(&d, &format!("arcminute_{axis}")) * 60.0 - whole).abs()
                    <= 1e-9 * whole.max(1.0)
            );
        }
    }
    let arc = |lat: f64| {
        num(
            &call("geodesy.ellipsoid.meridian-arc", json!({ "lat": lat })),
            "meridian_arc",
        )
    };
    let mut last = f64::NEG_INFINITY;
    for lat in [-80.0, -45.0, -10.0, 0.0, 10.0, 45.0, 80.0] {
        // M(−φ) = −M(φ), and the arc grows with latitude.
        assert!((arc(-lat) + arc(lat)).abs() < 1e-6);
        assert!(arc(lat) > last);
        last = arc(lat);
    }
    // The arc between two latitudes is the difference of their arcs.
    let between = call(
        "geodesy.ellipsoid.meridian-arc",
        json!({"lat": 12.5, "lat2": 47.25}),
    );
    assert!((num(&between, "arc_between") - (arc(47.25) - arc(12.5))).abs() < 1e-6);
}

#[test]
fn mgrs_views_invariants() {
    for (lat, lon) in [
        (40.446111, -79.982222),
        (78.2232, 15.6267),
        (-33.8688, 151.2093),
        (85.5, 30.0),
        (-82.0, -45.0),
    ] {
        let z = call(
            "geodesy.grid-ref.grid-zone-lookup",
            json!({"lat": lat, "lon": lon}),
        );
        let square = text(&z, "square_100km");
        let zone = text(&z, "grid_zone");
        // Above 84° N the designator is Y or Z; below 80° S, A or B.
        if lat > 84.0 {
            assert!(zone == "Y" || zone == "Z", "{zone}");
        } else if lat < -80.0 {
            assert!(zone == "A" || zone == "B", "{zone}");
        }
        // The designator and square begin every reference in the square.
        for precision in [1, 3, 5] {
            let f = call(
                "geodesy.grid-ref.mgrs-forward",
                json!({"lat": lat, "lon": lon, "precision": precision}),
            );
            assert!(text(&f, "mgrs").starts_with(&square), "{square}");
        }
        // Points in one 100 km square share the answer.
        let near = call(
            "geodesy.grid-ref.grid-zone-lookup",
            json!({"lat": lat + 1e-5, "lon": lon + 1e-5}),
        );
        let f = text(
            &call(
                "geodesy.grid-ref.mgrs-forward",
                json!({"lat": lat + 1e-5, "lon": lon + 1e-5, "precision": 0}),
            ),
            "mgrs",
        );
        if f == square {
            assert_eq!(text(&near, "square_100km"), square);
        }

        // Precision: each coarser square is ten times larger and contains the finer.
        let full = text(
            &call(
                "geodesy.grid-ref.mgrs-forward",
                json!({"lat": lat, "lon": lon, "precision": 5}),
            ),
            "mgrs",
        );
        let p = call("geodesy.grid-ref.mgrs-precision", json!({ "mgrs": full }));
        let mut size = num(&p, "square_size");
        let coarser = p["coarser"].as_array().unwrap();
        assert_eq!(coarser.len(), 5, "{full}");
        for row in coarser {
            let reference = row["reference"].as_str().unwrap();
            let q = call(
                "geodesy.grid-ref.mgrs-precision",
                json!({ "mgrs": reference }),
            );
            assert!((num(&q, "square_size") - 10.0 * size).abs() < 1e-9 * size);
            size = num(&q, "square_size");
            // The coarser square contains the finer: its reference is the
            // finer one's easting and northing cut to fewer digits.
            let digits = num(&q, "digits") as usize;
            let (e, n) = full[square.len()..].split_at((full.len() - square.len()) / 2);
            assert_eq!(
                reference,
                format!("{square}{}{}", &e[..digits], &n[..digits])
            );
        }
        // A 100 km reference has no coarser form.
        let top = call("geodesy.grid-ref.mgrs-precision", json!({ "mgrs": square }));
        assert!(top["coarser"].as_array().is_none_or(Vec::is_empty), "{top}");
    }
}

#[test]
fn height_views_invariants() {
    gp_base::assets::put(
        "egm96-15@2009-08-29/egm96-15.pgm",
        include_bytes!("../../../../assets/data/egm96-15/2009-08-29/egm96-15.pgm"),
    );
    for (lat, lon, h) in [
        (46.452, 6.583, "448.2 m"),
        (40.015, -105.2705, "5430 ft"),
        (-33.8688, 151.2093, "12 m"),
        (27.988, 86.925, "8848.86 m"),
    ] {
        let down = call(
            "geodesy.height.ellipsoidal-to-orthometric",
            json!({"lat": lat, "lon": lon, "height": h}),
        );
        // h − H = N, and the height keeps the unit it was entered in.
        let unit = h.split(' ').nth(1).unwrap();
        assert_eq!(down["orthometric"]["unit"], unit, "{down}");
        let n = num(&down, "geoid_height");
        let to_m = if unit == "ft" { 0.3048 } else { 1.0 };
        assert!(
            ((num(&down, "ellipsoidal") - num(&down, "orthometric")) * to_m - n).abs() < 1e-6,
            "{down}"
        );
        // One way and back returns the height.
        let back = call(
            "geodesy.height.orthometric-to-ellipsoidal",
            json!({"lat": lat, "lon": lon, "height": format!("{} {unit}", num(&down, "orthometric"))}),
        );
        let start: f64 = h.split(' ').next().unwrap().parse().unwrap();
        assert!((num(&back, "ellipsoidal") - start).abs() < 1e-6, "{back}");
        assert_eq!(back["ellipsoidal"]["unit"], unit);
    }
}

#[test]
fn magnetic_views_invariants() {
    for (lat, lon, date) in [
        (61.2181, -149.9003, "2027-01-01"),
        (74.697, -94.83, "2026-07-01"),
        (-77.85, 166.67, "2029-11-30"),
        (0.0, 0.0, "2025.5"),
        (86.5, 140.0, "2029-12-31"),
    ] {
        let place = json!({"lat": lat, "lon": lon, "date": date});
        // Model comparison: each declination is the declination tool's, the
        // difference lies within ±180°, and it is WMM less IGRF.
        let both = call("geodesy.magnetic.model-compare", place.clone());
        for (model, key) in [
            ("wmm2025", "wmm_declination"),
            ("igrf14", "igrf_declination"),
        ] {
            let mut input = place.clone();
            input["model"] = model.into();
            let one = call("geodesy.magnetic.declination", input);
            assert_eq!(both[key], one["declination"], "{model}");
        }
        let dd = num(&both, "declination_difference");
        assert!((-180.0..=180.0).contains(&dd));
        let raw = num(&both, "wmm_declination") - num(&both, "igrf_declination");
        assert!(((raw - dd + 540.0).rem_euclid(360.0) - 180.0).abs() < 1e-9);

        // Compass zone: uncertainty never below 0.26°, and the zone follows H.
        let z = call("geodesy.magnetic.uncertainty-zone", place.clone());
        let h = num(&z, "horizontal_intensity");
        let u = num(&z, "declination_uncertainty");
        assert!(u >= 0.26);
        let zone = text(&z, "compass_zone");
        let want = if h < 2000.0 {
            "blackout"
        } else if h < 6000.0 {
            "caution"
        } else {
            "normal"
        };
        assert_eq!(zone, want, "{lat}, {lon}: H {h}");
    }
    // A weaker horizontal field never gives a smaller uncertainty.
    let mut pairs: Vec<(f64, f64)> = [
        (0.0, 0.0),
        (45.0, -100.0),
        (70.0, -95.0),
        (80.0, -75.0),
        (86.0, 150.0),
    ]
    .iter()
    .map(|(lat, lon)| {
        let z = call(
            "geodesy.magnetic.uncertainty-zone",
            json!({"lat": lat, "lon": lon, "date": "2027-01-01"}),
        );
        (
            num(&z, "horizontal_intensity"),
            num(&z, "declination_uncertainty"),
        )
    })
    .collect();
    pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
    for w in pairs.windows(2) {
        assert!(w[0].1 >= w[1].1, "{pairs:?}");
    }
}

#[test]
fn projection_views_invariants() {
    for tool in [
        "geodesy.projection.convergence",
        "geodesy.projection.scale-factor",
    ] {
        // On the central meridian: no convergence, scale 0.9996.
        let cm = call(tool, json!({"lat": 45.0, "lon": -93.0}));
        assert!(num(&cm, "convergence").abs() < 1e-9);
        assert!((num(&cm, "scale") - 0.9996).abs() < 1e-12);
        // Convergence changes sign across the central meridian and the equator;
        // scale is the same at equal distances either side.
        let east = call(tool, json!({"lat": 45.0, "lon": -91.5}));
        let west = call(tool, json!({"lat": 45.0, "lon": -94.5}));
        let south = call(tool, json!({"lat": -45.0, "lon": -91.5}));
        assert!(num(&east, "convergence") > 0.0 && num(&west, "convergence") < 0.0);
        assert!((num(&east, "convergence") + num(&west, "convergence")).abs() < 1e-9);
        assert!((num(&south, "convergence") + num(&east, "convergence")).abs() < 1e-9);
        assert!((num(&east, "scale") - num(&west, "scale")).abs() < 1e-12);
        assert!(num(&east, "scale") > 0.9996);
    }
}
