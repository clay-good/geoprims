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

/// Layer E for `geodesy.datum.helmert`. The reverse is the exact inverse, the
/// two conventions differ only in the sign of the rotations, and no parameters
/// means no movement.
#[test]
fn helmert_invariants() {
    let at = |r: &Value, k: &str| r["result"][k]["value"].as_f64().expect("a number");
    // Positions spread over the globe, and parameter sets from the small ones
    // a modern frame tie uses to rotations far larger than any real datum,
    // where negating the parameters instead of inverting would show.
    let places = [
        (3657660.66, 255768.55, 5201382.11),
        (-2694045.0, -4293642.0, 3857878.0),
        (6378137.0, 0.0, 0.0),
        (1113194.9, 1113194.9, 6259542.0),
    ];
    let sets = [
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
        (4.5, -1.2, 0.3, 0.0, 0.0, 0.554, 0.219),
        (-146.0, 507.0, 685.0, 0.0, 0.0, 0.554, -2.4),
        (10.0, -20.0, 30.0, 12.0, -30.0, 45.0, 100.0),
    ];
    for (x, y, z) in places {
        for (tx, ty, tz, rx, ry, rz, scale) in sets {
            for convention in ["position-vector", "coordinate-frame"] {
                let args = format!(
                    r#"{{"x":{x},"y":{y},"z":{z},"tx":{tx},"ty":{ty},"tz":{tz},"rx":{rx},"ry":{ry},"rz":{rz},"scale":{scale},"convention":"{convention}"}}"#
                );
                let f = call("geodesy.datum.helmert", &args);
                assert_eq!(f["ok"], true, "{f}");
                let (fx, fy, fz) = (at(&f, "x"), at(&f, "y"), at(&f, "z"));
                let back = call(
                    "geodesy.datum.helmert",
                    &format!(
                        r#"{{"x":{fx},"y":{fy},"z":{fz},"tx":{tx},"ty":{ty},"tz":{tz},"rx":{rx},"ry":{ry},"rz":{rz},"scale":{scale},"convention":"{convention}","direction":"reverse"}}"#
                    ),
                );
                assert_eq!(back["ok"], true, "{back}");
                for (got, want, axis) in [
                    (at(&back, "x"), x, "x"),
                    (at(&back, "y"), y, "y"),
                    (at(&back, "z"), z, "z"),
                ] {
                    assert!(
                        (got - want).abs() < 1e-6,
                        "{convention} {axis}: {got} came back from {want}"
                    );
                }
                // The conventions differ in the sign of the rotations alone.
                let mirrored = call(
                    "geodesy.datum.helmert",
                    &format!(
                        r#"{{"x":{x},"y":{y},"z":{z},"tx":{tx},"ty":{ty},"tz":{tz},"rx":{},"ry":{},"rz":{},"scale":{scale},"convention":"{}"}}"#,
                        -rx,
                        -ry,
                        -rz,
                        if convention == "position-vector" {
                            "coordinate-frame"
                        } else {
                            "position-vector"
                        }
                    ),
                );
                for (a, b) in [
                    (fx, at(&mirrored, "x")),
                    (fy, at(&mirrored, "y")),
                    (fz, at(&mirrored, "z")),
                ] {
                    assert!((a - b).abs() < 1e-9, "conventions disagree: {a} vs {b}");
                }
            }
        }
    }
}
