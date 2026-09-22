//! EGM96 geoid: GeographicLib GeoidEval parity (cubic and bilinear, 2,010
//! points including the poles), the asset flow, and height conversion.

use gp_geodesy::REGISTRY;
use serde_json::Value;

const GRID: &[u8] = include_bytes!("../../../../assets/data/egm96-15/2009-08-29/egm96-15.pgm");

fn call(id: &str, input: &str) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, input)).expect("envelope is JSON")
}

fn supply() {
    gp_base::assets::put("egm96-15@2009-08-29/egm96-15.pgm", GRID);
}

#[test]
fn matches_geoideval_cubic_and_bilinear() {
    let g = gp_geo::geoid::Grid::parse(GRID).unwrap();
    let (mut wc, mut wl) = (0.0f64, 0.0f64);
    let mut n = 0;
    for l in include_str!("data/egm96_diff.csv").lines().skip(1) {
        let v: Vec<f64> = l.split(',').map(|x| x.parse().unwrap()).collect();
        wc = wc.max((g.height(v[0], v[1], true) - v[2]).abs());
        wl = wl.max((g.height(v[0], v[1], false) - v[3]).abs());
        n += 1;
    }
    assert_eq!(n, 2010);
    // GeoidEval prints 4 decimals: agreement to half a unit in the last place.
    assert!(wc <= 0.5e-4 + 1e-9, "cubic {wc}");
    assert!(wl <= 0.5e-4 + 1e-9, "bilinear {wl}");
}

#[test]
fn asset_unavailable_names_the_file() {
    gp_base::assets::clear();
    let r = call(
        "geodesy.geoid.geoid-height",
        r#"{"lat":16.776,"lon":-3.009}"#,
    );
    assert_eq!(r["error"]["code"], "ASSET_UNAVAILABLE", "{r}");
    assert_eq!(r["error"]["asset"]["id"], "egm96-15");
    assert_eq!(r["error"]["asset"]["version"], "2009-08-29");
    assert_eq!(r["error"]["asset"]["key"], "egm96-15.pgm");
}

#[test]
fn corrupt_grid_is_an_integrity_error() {
    gp_base::assets::put("egm96-15@2009-08-29/egm96-15.pgm", b"P5\nnot a grid");
    let r = call(
        "geodesy.geoid.geoid-height",
        r#"{"lat":16.776,"lon":-3.009}"#,
    );
    assert_eq!(r["error"]["code"], "ASSET_INTEGRITY", "{r}");
}

#[test]
fn geoid_height_and_conversion() {
    supply();
    let r = call(
        "geodesy.geoid.geoid-height",
        r#"{"lat":16.776,"lon":-3.009}"#,
    );
    assert!(
        (r["result"]["geoid_height"]["value"].as_f64().unwrap() - 28.7079).abs() < 1e-4,
        "{r}"
    );
    assert_eq!(r["meta"]["assets"][0]["id"], "egm96-15");
    let r = call(
        "geodesy.height.convert",
        r#"{"lat":16.776,"lon":-3.009,"height":"100 m"}"#,
    );
    let h = r["result"]["orthometric"]["value"].as_f64().unwrap();
    assert!((h - (100.0 - 28.7079)).abs() < 1e-4, "{r}");
    let back = call(
        "geodesy.height.convert",
        &format!(r#"{{"lat":16.776,"lon":-3.009,"height":"{h} m","from":"orthometric"}}"#),
    );
    assert!((back["result"]["converted"]["value"].as_f64().unwrap() - 100.0).abs() < 1e-9);
    // Feet in, feet out.
    let r = call(
        "geodesy.height.convert",
        r#"{"lat":16.776,"lon":-3.009,"height":"328.084 ft"}"#,
    );
    assert_eq!(r["result"]["converted"]["unit"], "ft");
}

#[test]
fn geoid_height_invariants() {
    // One value at each pole whatever the longitude, the ±180° meridian
    // agrees, heights stay in EGM96's range, and cubic and bilinear agree
    // within the grid header's stated interpolation errors (0.17 m + 1.15 m).
    supply();
    let at = |lat: f64, lon: f64, how: &str| {
        let r = call(
            "geodesy.geoid.geoid-height",
            &format!(r#"{{"lat":{lat},"lon":{lon},"interpolation":"{how}"}}"#),
        );
        r["result"]["geoid_height"]["value"]
            .as_f64()
            .unwrap_or_else(|| panic!("{r}"))
    };
    for pole in [90.0, -90.0] {
        let n0 = at(pole, 0.0, "cubic");
        for lon in (-180..=180).step_by(30) {
            assert!(
                (at(pole, lon as f64, "cubic") - n0).abs() < 1e-9,
                "{pole} {lon}"
            );
        }
    }
    for lat in (-89..=89).step_by(7) {
        let lat = lat as f64 + 0.37;
        assert!((at(lat, -180.0, "cubic") - at(lat, 180.0, "cubic")).abs() < 1e-9);
        for lon in (-180..180).step_by(11) {
            let lon = lon as f64 + 0.61;
            let (c, b) = (at(lat, lon, "cubic"), at(lat, lon, "bilinear"));
            assert!((-107.0..=86.0).contains(&c), "{lat} {lon} {c}");
            assert!((c - b).abs() < 1.32, "{lat} {lon} {c} {b}");
        }
    }
}

#[test]
fn height_above_ground_needs_the_ground_and_flags_being_under_it() {
    // add-geodesy-suite heights-and-geoid: a drone's height above the
    // ellipsoid over terrain given as mean sea level.
    supply();
    let at = |extra: &str| {
        call(
            "geodesy.height.convert",
            &format!(r#"{{"lat":-33.8688,"lon":151.2093,"height":"120 m"{extra}}}"#),
        )
    };
    let r = at(r#","terrain":"250 m""#);
    let n = r["result"]["geoid_height"]["value"].as_f64().unwrap();
    let orth = r["result"]["orthometric"]["value"].as_f64().unwrap();
    assert!((orth - (120.0 - n)).abs() < 1e-9, "{r}");
    // 120 m above the ellipsoid is well under 250 m of ground.
    let agl = r["result"]["agl"]["value"].as_f64().unwrap();
    assert!((agl - (orth - 250.0)).abs() < 1e-9, "{r}");
    assert!(agl < 0.0);
    assert!(
        r["meta"]["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["code"] == "BELOW_TERRAIN"),
        "{r}"
    );
    // The other way: 120 m above that ground is 370 m above sea level.
    let up = at(r#","from":"agl","terrain":"250 m""#);
    assert!(
        (up["result"]["orthometric"]["value"].as_f64().unwrap() - 370.0).abs() < 1e-9,
        "{up}"
    );
    assert!((up["result"]["ellipsoidal"]["value"].as_f64().unwrap() - (370.0 + n)).abs() < 1e-9);
    assert!((up["result"]["agl"]["value"].as_f64().unwrap() - 120.0).abs() < 1e-9);
    // A height above ground says nothing without the ground.
    let bad = at(r#","from":"agl""#);
    assert_eq!(bad["ok"], false, "{bad}");
    assert_eq!(bad["error"]["field"], "/terrain");
    // Without terrain the tool says nothing about height above ground.
    assert!(at("")["result"].get("agl").is_none());
}
