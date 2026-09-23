//! Track distances: the metric properties, and the relationships between the
//! three measures that no one of them could satisfy alone.

use gp_geometry::REGISTRY;
use serde_json::{Value, json};

fn tracks(a: &Value, b: &Value) -> Value {
    serde_json::from_str(
        &REGISTRY.invoke(
            "geometry.distance.tracks",
            &json!({"track_a": a, "track_b": b,
                "options": {"outputUnits": {"frechet": "m", "hausdorff": "m", "closest": "m"}}})
            .to_string(),
        ),
    )
    .expect("JSON")
}

fn d(r: &Value, key: &str) -> f64 {
    r["result"][key]["value"]
        .as_f64()
        .unwrap_or_else(|| panic!("{key} in {r}"))
}

fn shifted(track: &Value, dlat: f64) -> Value {
    Value::Array(
        track
            .as_array()
            .unwrap()
            .iter()
            .map(|p| json!({"lat": p["lat"].as_f64().unwrap() + dlat, "lon": p["lon"]}))
            .collect(),
    )
}

#[test]
fn track_distance_invariants() {
    let a = json!([{"lat":40.0,"lon":-105.0},{"lat":40.001,"lon":-104.999},
                   {"lat":40.002,"lon":-104.998},{"lat":40.003,"lon":-104.997},
                   {"lat":40.004,"lon":-104.996}]);
    let b = json!([{"lat":40.0005,"lon":-105.0005},{"lat":40.0015,"lon":-104.9995},
                   {"lat":40.0025,"lon":-104.9985},{"lat":40.0035,"lon":-104.9975},
                   {"lat":40.0045,"lon":-104.9965}]);
    let crossing = json!([{"lat":40.004,"lon":-105.0},{"lat":40.003,"lon":-104.999},
                          {"lat":40.002,"lon":-104.998},{"lat":40.001,"lon":-104.997},
                          {"lat":40.0,"lon":-104.996}]);

    // A track against itself. Not identically zero for the Hausdorff distance:
    // that one asks the geodesic solver for the distance to a segment the
    // point already lies on, and gets a nanometre back.
    let same = tracks(&a, &a);
    assert_eq!(d(&same, "frechet"), 0.0, "{same}");
    assert_eq!(d(&same, "closest"), 0.0, "{same}");
    assert!(d(&same, "hausdorff") < 1e-6, "{same}");

    // Symmetric in their arguments -- including the Frechet distance, whose
    // recurrence is not.
    let ab = tracks(&a, &b);
    let ba = tracks(&b, &a);
    for k in ["frechet", "hausdorff", "closest"] {
        assert_eq!(
            d(&ab, k),
            d(&ba, k),
            "{k} depends on the order of the arguments"
        );
    }

    // Forced by what each one measures, on every pair.
    for (name, x, y) in [
        ("beside", &a, &b),
        ("crossing", &a, &crossing),
        (
            "reversed",
            &a,
            &json!(
                a.as_array()
                    .unwrap()
                    .iter()
                    .rev()
                    .cloned()
                    .collect::<Vec<_>>()
            ),
        ),
    ] {
        let r = tracks(x, y);
        assert!(
            d(&r, "closest") <= d(&r, "hausdorff") + 1e-9,
            "{name}: closest {} above hausdorff {}",
            d(&r, "closest"),
            d(&r, "hausdorff")
        );
        assert!(
            d(&r, "hausdorff") <= d(&r, "frechet") + 1e-9,
            "{name}: hausdorff {} above frechet {}",
            d(&r, "hausdorff"),
            d(&r, "frechet")
        );
    }

    // The difference between an ordered measure and an unordered one, which is
    // the whole reason both are reported. Over identical ground, backwards:
    // Hausdorff sees nothing, Frechet sees the length of the track.
    let reversed = Value::Array(a.as_array().unwrap().iter().rev().cloned().collect());
    let rev = tracks(&a, &reversed);
    assert!(
        d(&rev, "hausdorff") < 1e-6,
        "reversed: hausdorff {}",
        d(&rev, "hausdorff")
    );
    assert_eq!(d(&rev, "closest"), 0.0, "{rev}");
    assert!(
        d(&rev, "frechet") > 500.0,
        "reversed: frechet {} -- the walkers start at opposite ends and cannot turn back",
        d(&rev, "frechet")
    );

    // Crossing tracks touch.
    assert_eq!(d(&tracks(&a, &crossing), "closest"), 0.0);

    // Moving one track bodily away moves all three out together.
    let far = tracks(&a, &shifted(&b, 0.01));
    for k in ["frechet", "hausdorff", "closest"] {
        assert!(
            d(&far, k) > d(&ab, k),
            "{k} did not grow when the tracks were moved apart"
        );
    }

    // The tightest pair is a real pair of points, and the distance between
    // them is the reported Frechet distance -- measured here by the geodesic
    // inverse rather than taken from the same computation.
    let (ia, ib) = (
        ab["result"]["frechet_a"].as_u64().unwrap() as usize,
        ab["result"]["frechet_b"].as_u64().unwrap() as usize,
    );
    assert!(
        ia >= 1
            && ia <= a.as_array().unwrap().len()
            && ib >= 1
            && ib <= b.as_array().unwrap().len(),
        "the tightest pair is ({ia}, {ib}), outside the tracks"
    );
    let (pa, pb) = (&a[ia - 1], &b[ib - 1]);
    let leash: Value = serde_json::from_str(
        &REGISTRY.invoke(
            "geometry.distance.tracks",
            &json!({"track_a": [pa, pa], "track_b": [pb, pb],
                "options": {"outputUnits": {"frechet": "m", "hausdorff": "m", "closest": "m"}}})
            .to_string(),
        ),
    )
    .expect("JSON");
    assert!(
        (d(&leash, "frechet") - d(&ab, "frechet")).abs() < 1e-6,
        "the tightest pair are {} m apart, not the reported {} m",
        d(&leash, "frechet"),
        d(&ab, "frechet")
    );
}
