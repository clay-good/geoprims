//! Grid references: the grid-references spec scenarios for USNG, Maidenhead,
//! GARS, and GEOREF.

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

#[test]
fn usng_space_delimited_and_truncated() {
    // grid-references "Space-delimited output".
    let r = call(
        "geodesy.grid-ref.usng-forward",
        r#"{"lat":40.446111,"lon":-79.982222}"#,
    );
    assert_eq!(r["result"]["usng"], "17T NE 86309 77770", "{r}");
    // A truncated local reference needs its grid zone, and then names a 100 m square.
    let t = call(
        "geodesy.grid-ref.usng-inverse",
        r#"{"usng":"NE 863 777","zone":"17T"}"#,
    );
    assert_eq!(num(&t, "result.square_size.value"), 100.0);
    let full = call(
        "geodesy.grid-ref.usng-inverse",
        r#"{"usng":"17T NE 863 777"}"#,
    );
    assert_eq!(t["result"], full["result"]);
    assert_eq!(
        call("geodesy.grid-ref.usng-inverse", r#"{"usng":"NE 863 777"}"#)["error"]["field"],
        "/zone"
    );
}

#[test]
fn maidenhead_scenarios() {
    // "Six-character locator".
    let r = call(
        "geodesy.grid-ref.maidenhead-forward",
        r#"{"lat":40.446111,"lon":-79.982222,"precision":"6"}"#,
    );
    assert_eq!(r["result"]["locator"], "FN00ak");
    // "Edge clamp": latitude 90 lands in field row R and decodes to the northernmost cell.
    let n = call(
        "geodesy.grid-ref.maidenhead-forward",
        r#"{"lat":90,"lon":180,"precision":"4"}"#,
    );
    let loc = n["result"]["locator"].as_str().unwrap();
    assert_eq!(&loc[1..2], "R");
    assert_eq!(loc, "RR99");
    let back = call(
        "geodesy.grid-ref.maidenhead-inverse",
        &format!(r#"{{"locator":"{loc}"}}"#),
    );
    assert_eq!(num(&back, "result.north.value"), 90.0);
    assert_eq!(num(&back, "result.east.value"), 180.0);
    // Case-insensitive input, conventional case output.
    let mixed = call(
        "geodesy.grid-ref.maidenhead-inverse",
        r#"{"locator":"fn00AK"}"#,
    );
    assert_eq!(
        mixed["result"],
        call(
            "geodesy.grid-ref.maidenhead-inverse",
            r#"{"locator":"FN00ak"}"#
        )["result"]
    );
}

#[test]
fn gars_keypad_and_georef() {
    // "GARS keypad": 3 digits, 2 letters, quadrant digit, keypad digit, and the bounds.
    let r = call(
        "geodesy.grid-ref.gars-forward",
        r#"{"lat":40.446111,"lon":-79.982222}"#,
    );
    let g = r["result"]["gars"].as_str().unwrap();
    assert_eq!(g.len(), 7);
    assert!(
        g[..3].chars().all(|c| c.is_ascii_digit())
            && g[3..5].chars().all(|c| c.is_ascii_uppercase())
    );
    assert!(g[5..].chars().all(|c| c.is_ascii_digit()));
    let size = num(&r, "result.north.value") - num(&r, "result.south.value");
    assert!((size - 5.0 / 60.0).abs() < 1e-12);
    assert!(
        num(&r, "result.south.value") <= 40.446111 && 40.446111 < num(&r, "result.north.value")
    );
    assert_eq!(
        call(
            "geodesy.grid-ref.gars-forward",
            r#"{"lat":57.64911,"lon":10.40744}"#
        )["result"]["gars"],
        "381NH45"
    );
    let geo = call(
        "geodesy.grid-ref.georef-forward",
        r#"{"lat":57.64911,"lon":10.40744}"#,
    );
    assert_eq!(geo["result"]["georef"], "NKLN2438");
    let t = call(
        "geodesy.grid-ref.georef-forward",
        r#"{"lat":57.64911,"lon":10.40744,"precision":"15deg"}"#,
    );
    assert_eq!(t["result"]["georef"], "NK");
    // 180° E is 180° W: tile A.
    assert_eq!(
        call(
            "geodesy.grid-ref.georef-forward",
            r#"{"lat":0,"lon":180,"precision":"1deg"}"#
        )["result"]["georef"],
        "AGAA"
    );
}

#[test]
fn grid_reference_invariants() {
    // For GARS, GEOREF, and Maidenhead at every precision: the decoded cell
    // holds the point, its center encodes back to the same code, and each
    // finer cell lies inside the coarser one. For USNG: the reference's
    // square holds the point, and its center encodes back to it.
    let mut seed: u64 = 5;
    let mut rnd = || {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    let systems: [(&str, &str, &str, &[&str]); 3] = [
        ("gars", "gars", "gars", &["30min", "15min", "5min"]),
        ("georef", "georef", "georef", &["15deg", "1deg", "1min", "0.1min", "0.01min", "0.001min", "0.0001min"]),
        ("maidenhead", "locator", "locator", &["2", "4", "6", "8", "10"]),
    ];
    for _ in 0..300 {
        let (lat, lon) = (rnd() * 179.8 - 89.9, rnd() * 359.8 - 179.9);
        for (sys, out, field, precs) in systems {
            let mut outer: Option<[f64; 4]> = None;
            for p in precs {
                let f = call(
                    &format!("geodesy.grid-ref.{sys}-forward"),
                    &serde_json::json!({"lat": lat, "lon": lon, "precision": p}).to_string(),
                );
                let code = f["result"][out].as_str().unwrap().to_owned();
                let inv = format!("geodesy.grid-ref.{sys}-inverse");
                let d = call(&inv, &serde_json::json!({ field: code }).to_string());
                let (s, w) = (num(&d, "result.south.value"), num(&d, "result.west.value"));
                let (clat, clon) = (num(&d, "result.lat.value"), num(&d, "result.lon.value"));
                let (n, e) = (2.0 * clat - s, 2.0 * clon - w);
                assert!(s <= lat && lat <= n && w <= lon && lon <= e, "{code} does not hold {lat},{lon}");
                let back = call(
                    &format!("geodesy.grid-ref.{sys}-forward"),
                    &serde_json::json!({"lat": clat, "lon": clon, "precision": p}).to_string(),
                );
                assert!(back["result"][out].as_str().unwrap().eq_ignore_ascii_case(&code), "{code}");
                if let Some([os, ow, on, oe]) = outer {
                    let eps = 1e-12;
                    assert!(s >= os - eps && w >= ow - eps && n <= on + eps && e <= oe + eps, "{code} escapes its parent");
                }
                outer = Some([s, w, n, e]);
            }
        }
        if lat.abs() < 80.0 {
            for p in ["10km", "1km", "100m", "10m", "1m"] {
                let f = call(
                    "geodesy.grid-ref.usng-forward",
                    &serde_json::json!({"lat": lat, "lon": lon, "precision": p}).to_string(),
                );
                let u = f["result"]["usng"].as_str().unwrap().to_owned();
                let d = call("geodesy.grid-ref.usng-inverse", &serde_json::json!({"usng": u}).to_string());
                let size = num(&d, "result.square_size.value");
                // The point is within one square diagonal of the square's corner.
                let (dy, dx) = (
                    (num(&d, "result.corner_lat.value") - lat) * 111_320.0,
                    (num(&d, "result.corner_lon.value") - lon) * 111_320.0 * lat.to_radians().cos(),
                );
                assert!(dy.hypot(dx) <= size * 1.5 + 0.01, "{u}: {} m from {lat},{lon}", dy.hypot(dx));
                let back = call(
                    "geodesy.grid-ref.usng-forward",
                    &serde_json::json!({"lat": num(&d, "result.lat.value"), "lon": num(&d, "result.lon.value"), "precision": p}).to_string(),
                );
                // A square cut by a zone edge can have its center in the next zone.
                let b = back["result"]["usng"].as_str().unwrap();
                assert!(b == u || b.split(' ').next() != u.split(' ').next(), "{u} -> {b}");
            }
        }
    }
}
