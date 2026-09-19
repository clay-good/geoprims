//! Datum transformations: the datums-and-transformations spec scenarios for
//! Helmert conventions and ITRF/WGS 84 frames.

use gp_geodesy::REGISTRY;
use serde_json::Value;

fn call(id: &str, input: &str) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, input)).expect("envelope is JSON")
}

#[test]
fn convention_required() {
    // "Convention required": rotations without a convention are refused, with the reason.
    let r = call(
        "geodesy.datum.helmert",
        r#"{"x":3657660.66,"y":255768.55,"z":5201382.11,"rz":0.554}"#,
    );
    assert_eq!(r["error"]["code"], "INVALID_INPUT");
    assert_eq!(r["error"]["field"], "/convention");
    assert!(
        r["error"]["message"]
            .as_str()
            .unwrap()
            .contains("sign of the rotations"),
        "{r}"
    );
    // Translations and scale alone need no convention.
    assert_eq!(
        call(
            "geodesy.datum.helmert",
            r#"{"x":1,"y":2,"z":6378137,"tz":4.5,"scale":0.219}"#
        )["ok"],
        true
    );
}

#[test]
fn coincidence_and_realization() {
    // "Coincidence stated": WGS 84 (G2296) to ITRF2020 leaves the coordinates alone and says why.
    let r = call(
        "geodesy.datum.itrf",
        r#"{"from":"WGS84(G2296)","to":"ITRF2020","epoch":"2026.7","lat":38.5,"lon":-98}"#,
    );
    assert_eq!(r["result"]["shift"]["value"], 0.0);
    let acc = r["meta"]["accuracy"].as_str().unwrap();
    assert!(acc.contains("few-centimeter"), "{acc}");
    assert_eq!(r["meta"]["context"]["epoch"], 2026.7);
    // "WGS 84" alone is taken as the current realization, and says so.
    let w = call(
        "geodesy.datum.itrf",
        r#"{"from":"WGS84","to":"ITRF2014","epoch":"2026-09-19","lat":38.5,"lon":-98}"#,
    );
    let codes: Vec<&str> = w["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["code"].as_str().unwrap())
        .collect();
    assert!(codes.contains(&"REALIZATION_ASSUMED"), "{w}");
    // ITRF2020 to ITRF2014 moves a point by millimeters, not more.
    let s = w["result"]["shift"]["value"].as_f64().unwrap();
    assert!(s > 0.0005 && s < 0.01, "{s}");
}
