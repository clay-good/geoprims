//! The frame of an ellipsoidal height (add-geodesy-suite 6.3,
//! geodesy/heights-and-geoid "Ellipsoidal ↔ orthometric heights"): EGM96 is
//! referred to WGS 84, and a US survey height is on NAD 83 (2011).

use gp_geodesy::REGISTRY;
use serde_json::{Value, json};

fn run(id: &str, input: Value) -> Value {
    // The tools read EGM96 from the asset store, which a bare test has not
    // filled; put the repository's grid there first, as tests/geoid.rs does.
    gp_base::assets::put(
        "egm96-15@2009-08-29/egm96-15.pgm",
        include_bytes!("../../../../assets/data/egm96-15/2009-08-29/egm96-15.pgm"),
    );
    serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).unwrap()
}

fn num(r: &Value, k: &str) -> f64 {
    r["result"][k]["value"]
        .as_f64()
        .unwrap_or_else(|| panic!("{k} in {r}"))
}

fn codes(r: &Value) -> Vec<String> {
    r["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn a_wgs84_height_takes_no_step_and_says_nothing_new() {
    // The default is unchanged: every height published before the frame
    // input existed still converts to the same number, with no step and no
    // frame warning.
    let plain = run(
        "geodesy.height.ellipsoidal-to-orthometric",
        json!({"lat": 38.5, "lon": -98.0, "height": "500 m"}),
    );
    let named = run(
        "geodesy.height.ellipsoidal-to-orthometric",
        json!({"lat": 38.5, "lon": -98.0, "height": "500 m", "frame": "WGS84"}),
    );
    assert_eq!(plain["result"], named["result"]);
    assert!(plain["result"].get("frame_step").is_none(), "{plain}");
    assert!(
        !codes(&plain).contains(&"FRAME_MISMATCH".to_owned()),
        "{plain}"
    );
}

#[test]
fn a_nad83_height_round_trips_through_orthometric() {
    // Out to H on NAD 83 (2011) at an epoch and back must land on the
    // starting height, and both directions must report the same step.
    for (lat, lon, h, t) in [
        (38.5, -98.0, 500.0, "2026.7"),
        (47.6062, -122.3321, 50.0, "2026.87"),
        (25.7617, -80.1918, 2.0, "2020.0"),
    ] {
        let f = run(
            "geodesy.height.ellipsoidal-to-orthometric",
            json!({"lat": lat, "lon": lon, "height": format!("{h} m"), "frame": "NAD83(2011)", "epoch": t}),
        );
        let big_h = num(&f, "orthometric");
        let b = run(
            "geodesy.height.orthometric-to-ellipsoidal",
            json!({"lat": lat, "lon": lon, "height": format!("{big_h} m"), "frame": "NAD83(2011)", "epoch": t}),
        );
        assert!((num(&b, "ellipsoidal") - h).abs() < 1e-6, "{f}\n{b}");
        assert!(
            (num(&f, "frame_step") - num(&b, "frame_step")).abs() < 1e-6,
            "{f}\n{b}"
        );
    }
}

#[test]
fn the_step_has_the_size_and_shape_htdp_gives_across_the_lower_48() {
    // Measured with this tool over 25-49 N, 124-67 W at 2026.0, whose values
    // the HTDP vectors pin to 1.5 mm at five places: the WGS 84 height sits
    // 0.24 m (the northwest) to 1.72 m (the southeast) below the NAD 83
    // (2011) one. The NAD 83 frame is tilted against ITRF, so the gap grows
    // from northwest to southeast; a sign flip, a unit slip, or a dropped
    // rotation would break the band or the order.
    let step = |lat: f64, lon: f64| {
        num(
            &run(
                "geodesy.height.ellipsoidal-to-orthometric",
                json!({"lat": lat, "lon": lon, "height": "100 m", "frame": "NAD83(2011)", "epoch": "2026.0"}),
            ),
            "frame_step",
        )
    };
    let places = [
        (47.61, -122.33),
        (39.74, -104.99),
        (38.5, -98.0),
        (40.45, -79.98),
        (25.76, -80.19),
    ];
    for (lat, lon) in places {
        let s = step(lat, lon);
        assert!((-1.8..-0.2).contains(&s), "step at {lat}, {lon} is {s} m");
    }
    assert!(
        step(47.61, -122.33) > step(25.76, -80.19),
        "Seattle's gap should be smaller than Miami's"
    );
}

#[test]
fn without_an_epoch_it_warns_with_the_size_and_does_not_guess() {
    let r = run(
        "geodesy.height.convert",
        json!({"lat": 38.5, "lon": -98.0, "height": "500 m", "frame": "NAD83(2011)"}),
    );
    let w = r["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["code"] == "FRAME_MISMATCH")
        .unwrap_or_else(|| panic!("no FRAME_MISMATCH: {r}"));
    // The spec asks for the expected frame and the approximate vertical
    // impact, so the message names WGS 84 and a size in metres.
    let msg = w["message"].as_str().unwrap();
    assert!(msg.contains("WGS 84") && msg.contains(" m "), "{msg}");
    assert!(
        r["result"].get("frame_step").is_none(),
        "no step is applied: {r}"
    );

    // An epoch with a WGS 84 height is ignored, and the answer says so.
    let r = run(
        "geodesy.height.convert",
        json!({"lat": 38.5, "lon": -98.0, "height": "500 m", "epoch": "2026.7"}),
    );
    assert!(codes(&r).contains(&"INPUT_NORMALIZED".to_owned()), "{r}");
}
