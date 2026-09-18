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
