//! CRS search: the properties every answer must have.

use gp_geo::crs::{CrsKind, REGISTRY};
use gp_geo::spcs;
use gp_geodesy::REGISTRY as TOOLS;
use serde_json::{Value, json};

fn load_spcs2022() {
    gp_base::assets::put(
        "spcs2022-beta@2026-06-01/spcs2022-beta.json",
        include_bytes!("../../../../assets/data/spcs2022-beta/2026-06-01/spcs2022-beta.json"),
    );
}

fn search(input: Value) -> Value {
    load_spcs2022();
    let r: Value =
        serde_json::from_str(&TOOLS.invoke("geodesy.crs.search", &input.to_string())).unwrap();
    assert_eq!(r["ok"], true, "{r}");
    r
}

fn codes(r: &Value) -> Vec<String> {
    r["result"]["matches"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["code"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn every_system_is_found_by_its_code_and_its_name() {
    for c in REGISTRY {
        let by_code = search(json!({"query": format!("EPSG:{}", c.code)}));
        assert_eq!(codes(&by_code), [format!("EPSG:{}", c.code)]);
        assert_eq!(by_code["result"]["matches"][0]["name"], c.name);
        let by_name = search(json!({"query": c.name}));
        assert!(
            codes(&by_name).contains(&format!("EPSG:{}", c.code)),
            "{} not found by its name",
            c.name
        );
    }
}

#[test]
fn a_code_lookup_does_not_read_the_zone_file() {
    let r = search(json!({"query": "2232"}));
    assert!(
        r["meta"]["assets"].as_array().is_none_or(|a| a.is_empty()),
        "{}",
        r["meta"]
    );
    let named = search(json!({"query": "Colorado"}));
    assert_eq!(named["meta"]["assets"][0]["id"], "spcs2022-beta");
}

#[test]
fn every_system_covers_the_middle_of_its_area() {
    for c in REGISTRY {
        let [w, s, e, n] = c.bbox;
        let lon = if w <= e {
            (w + e) / 2.0
        } else {
            ((w + e + 360.0) / 2.0 + 180.0).rem_euclid(360.0) - 180.0
        };
        let lat = (s + n) / 2.0;
        let r = search(json!({"lat": lat, "lon": lon}));
        assert!(
            codes(&r).contains(&format!("EPSG:{}", c.code)),
            "{} not found at ({lat}, {lon})",
            c.name
        );
    }
}

#[test]
fn the_state_plane_answers_agree_with_the_zone_lookup() {
    // On a grid over the conterminous US, Alaska, and Hawaii, the SPCS83 zones
    // the search lists are exactly the zones the zone lookup gives.
    let mut checked = 0;
    for lat10 in (190..=700).step_by(15) {
        for lon10 in (-1790..=-660).step_by(25) {
            let (lat, lon) = (f64::from(lat10) / 10.0, f64::from(lon10) / 10.0);
            let want: std::collections::BTreeSet<&str> =
                spcs::candidates(lat, lon).iter().map(|z| z.fips).collect();
            let r = search(json!({"query": "NAD83 state plane", "lat": lat, "lon": lon}));
            let got: std::collections::BTreeSet<&str> = REGISTRY
                .iter()
                .filter(|c| codes(&r).contains(&format!("EPSG:{}", c.code)))
                .filter_map(|c| match c.kind {
                    CrsKind::Spcs83 { fips, .. } => Some(fips),
                    _ => None,
                })
                .collect();
            assert_eq!(got, want, "at ({lat}, {lon})");
            checked += usize::from(!want.is_empty());
        }
    }
    assert!(checked > 200, "{checked} points inside a zone");
}

#[test]
fn every_point_on_land_or_sea_has_its_wgs84_utm_zone() {
    for lat10 in (-790..=830).step_by(70) {
        for lon10 in (-1795..=1795).step_by(50) {
            let (lat, lon) = (f64::from(lat10) / 10.0, f64::from(lon10) / 10.0);
            let zone = ((lon + 180.0) / 6.0).floor() as u32 + 1;
            let want = if lat >= 0.0 {
                32600 + zone
            } else {
                32700 + zone
            };
            let r = search(json!({"query": "WGS 84 UTM", "lat": lat, "lon": lon}));
            assert!(
                codes(&r).contains(&format!("EPSG:{want}")),
                "({lat}, {lon}): {:?}",
                codes(&r)
            );
        }
    }
}

#[test]
fn nothing_is_left_unsaid() {
    let r: Value = serde_json::from_str(&TOOLS.invoke("geodesy.crs.search", "{}")).unwrap();
    assert_eq!(r["ok"], false);
    assert_eq!(r["error"]["field"], "/query");
    let none = search(json!({"query": "27700"}));
    assert_eq!(none["result"]["count"], 0.0);
    assert!(
        none["result"]["note"]
            .as_str()
            .unwrap()
            .contains("Nothing matched")
    );
    // Every row says how to get there, or that no conversion is needed.
    let denver = search(json!({"lat": 39.74, "lon": -104.99}));
    for m in denver["result"]["matches"].as_array().unwrap() {
        let convert = m["convert_with"].as_str().unwrap();
        assert!(convert.is_empty() == (m["code"] == "EPSG:4326"), "{m}");
        assert!(["current", "legacy", "beta"].contains(&m["status"].as_str().unwrap()));
    }
}

// ---------------------------------------------------------------- transform

fn transform(input: Value) -> Value {
    let r: Value =
        serde_json::from_str(&TOOLS.invoke("geodesy.crs.transform", &input.to_string())).unwrap();
    assert_eq!(r["ok"], true, "{r}");
    r["result"].clone()
}

fn tool(id: &str, input: Value) -> Value {
    let r: Value = serde_json::from_str(&TOOLS.invoke(id, &input.to_string())).unwrap();
    assert_eq!(r["ok"], true, "{id}: {r}");
    r["result"].clone()
}

fn v(r: &Value, k: &str) -> f64 {
    r[k]["value"].as_f64().unwrap()
}

#[test]
fn a_frame_change_is_the_three_tools_in_turn() {
    // The composite-path scenario: Colorado Central on NAD 83 (2011) to UTM
    // 13N on WGS 84 (G2296), against the State Plane inverse, the datum
    // transformation, and UTM forward run one after another.
    for (e, n, epoch) in [
        (953_000.0, 515_000.0, 2026.0),
        (900_000.0, 400_000.0, 2010.0),
        (1_050_000.0, 560_000.0, 2035.5),
    ] {
        let r = transform(
            json!({"from": "6427", "to": "32613", "easting": e, "northing": n, "epoch": epoch.to_string(), "height": "1600 m"}),
        );
        let geo = tool(
            "geodesy.spcs.spcs83-inverse",
            json!({"zone": "0502", "easting": format!("{e} m"), "northing": format!("{n} m"), "unit": "m"}),
        );
        let moved = tool(
            "geodesy.datum.transform",
            json!({"lat": v(&geo, "lat"), "lon": v(&geo, "lon"), "height": "1600 m", "from": "NAD83(2011)", "to": "WGS84(G2296)", "epoch": epoch.to_string()}),
        );
        let utm = tool(
            "geodesy.utm.forward",
            json!({"lat": v(&moved, "lat"), "lon": v(&moved, "lon"), "zone": 13}),
        );
        // The datum tool converts on GRS 80 at both ends; this one leaves on
        // WGS 84's ellipsoid, which moves the answer by a tenth of a millimeter.
        assert!(
            (v(&r, "easting") - v(&utm, "easting")).abs() < 0.001,
            "{r} vs {utm}"
        );
        assert!(
            (v(&r, "northing") - v(&utm, "northing")).abs() < 0.001,
            "{r} vs {utm}"
        );
        assert!((v(&r, "accuracy") - v(&moved, "accuracy")).abs() < 1e-12);
        // Steps in order: the projection out, each frame step, the projection in.
        let steps: Vec<&str> = r["steps"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["step"].as_str().unwrap())
            .collect();
        assert_eq!(steps.first(), Some(&"Inverse projection"));
        assert_eq!(steps.last(), Some(&"Forward projection"));
        assert!(
            steps[1..steps.len() - 1]
                .iter()
                .all(|s| *s == "Frame transformation")
        );
        assert_eq!(steps.len() - 2, moved["steps"].as_array().unwrap().len());
    }
}

#[test]
fn there_and_back_returns_the_start() {
    for (from, to, a, b) in [
        ("6427", "32613", 953_000.0, 515_000.0),
        ("6428", "3857", 3_126_000.0, 1_690_000.0),
        ("32613", "6405", 300_000.0, 3_700_000.0),
        ("6342", "6430", 480_000.0, 4_450_000.0),
    ] {
        let there = transform(
            json!({"from": from, "to": to, "easting": a, "northing": b, "epoch": "2026.0"}),
        );
        let back = transform(
            json!({"from": to, "to": from, "easting": v(&there, "easting"), "northing": v(&there, "northing"), "epoch": "2026.0"}),
        );
        assert!(
            (v(&back, "easting") - a).abs() < 1e-5,
            "{from} → {to}: {back}"
        );
        assert!(
            (v(&back, "northing") - b).abs() < 1e-5,
            "{from} → {to}: {back}"
        );
    }
}

#[test]
fn the_same_frame_needs_no_epoch_and_carries_no_datum_error() {
    let r =
        transform(json!({"from": "6427", "to": "6342", "easting": 953_000, "northing": 515_000}));
    assert_eq!(v(&r, "accuracy"), 0.0);
    assert!(
        r["steps"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["step"] != "Frame transformation")
    );
    // Feet twins differ from their metric zone only in their false origin:
    // Colorado Central in US survey feet is the metric answer converted,
    // less EPSG's rounding of the origin in feet.
    let m = transform(
        json!({"from": "4326", "to": "6427", "lat": 39.74, "lon": -104.99, "epoch": "2026.0"}),
    );
    let ft = transform(
        json!({"from": "4326", "to": "6428", "lat": 39.74, "lon": -104.99, "epoch": "2026.0"}),
    );
    let us_ft = 1200.0 / 3937.0;
    assert!(
        (v(&ft, "easting") * us_ft - (v(&m, "easting") - 914_401.828_9 + 3_000_000.0 * us_ft))
            .abs()
            < 1e-6
    );
}
