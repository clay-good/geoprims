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
