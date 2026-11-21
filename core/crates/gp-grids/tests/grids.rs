//! The national grids: the invariants each country's transformation has to
//! hold, beside the golden vectors. The issuers' own published examples, with
//! every intermediate they print, are in `gp-geo`'s tests, next to the math.

use gp_grids::REGISTRY;
use serde_json::Value;

#[test]
fn the_nzgd2000_grid_round_trips_and_moves_new_zealand_north_east() {
    // add-international-grids, geodesy/national-grids: NZGD1949 to NZGD2000
    // by LINZ's distortion grid, read from the committed package.
    gp_base::assets::put(
        "nzgd2000-grid@0005/nzgd2kgrid0005.grid",
        &std::fs::read(format!(
            "{}/../../../assets/data/nzgd2000-grid/0005/nzgd2kgrid0005.grid",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap(),
    );
    let call = |input: serde_json::Value| -> Value {
        serde_json::from_str(&REGISTRY.invoke("geodesy.datum.nzgd2000", &input.to_string()))
            .unwrap()
    };
    for (lat, lon) in [
        (-36.85, 174.76),
        (-41.28, 174.78),
        (-43.53, 172.64),
        (-45.87, 170.50),
        (-46.41, 168.35),
    ] {
        let f = call(serde_json::json!({"lat": lat, "lon": lon}));
        assert_eq!(f["ok"], true, "{f}");
        let (a, b) = (
            f["result"]["lat"]["value"].as_f64().unwrap(),
            f["result"]["lon"]["value"].as_f64().unwrap(),
        );
        // NZGD2000 sits north and east of NZGD1949; at the twenty vector
        // places from Cape Reinga to Stewart Island the move is 174 to 203 m.
        let shift = f["result"]["shift"]["value"].as_f64().unwrap();
        assert!((165.0..210.0).contains(&shift), "{shift} m at {lat}, {lon}");
        assert!(a > lat && b > lon, "north and east: {f}");
        // And the reverse lands back on the NZGD1949 point.
        let r = call(serde_json::json!({"lat": a, "lon": b, "direction": "to-nzgd1949"}));
        assert_eq!(r["ok"], true, "{r}");
        assert!(
            (r["result"]["lat"]["value"].as_f64().unwrap() - lat).abs() < 1e-11,
            "{r}"
        );
        assert!(
            (r["result"]["lon"]["value"].as_f64().unwrap() - lon).abs() < 1e-11,
            "{r}"
        );
    }
    // Outside the grid there is no answer.
    let r = call(serde_json::json!({"lat": -44.0, "lon": -176.5}));
    assert_eq!(r["error"]["code"], "OUT_OF_DOMAIN", "{r}");
    let r = call(serde_json::json!({"lat": -33.0, "lon": 172.0}));
    assert_eq!(r["error"]["code"], "OUT_OF_DOMAIN", "{r}");
}

#[test]
fn rdnaptrans_round_trips_and_its_factors_behave() {
    // add-international-grids, geodesy/national-grids: RDNAPTRANS2018, read
    // from the committed packages.
    for g in ["rdtrans2018", "nlgeo2018"] {
        gp_base::assets::put(
            &format!("rdnaptrans@2018/{g}.grid"),
            &std::fs::read(format!(
                "{}/../../../assets/data/rdnaptrans/2018/{g}.grid",
                env!("CARGO_MANIFEST_DIR")
            ))
            .unwrap(),
        );
    }
    let fwd = |input: serde_json::Value| -> Value {
        serde_json::from_str(&REGISTRY.invoke("geodesy.datum.rdnaptrans", &input.to_string()))
            .unwrap()
    };
    let inv = |input: serde_json::Value| -> Value {
        serde_json::from_str(
            &REGISTRY.invoke("geodesy.datum.rdnaptrans-inverse", &input.to_string()),
        )
        .unwrap()
    };
    let num = |v: &Value, path: [&str; 2]| v["result"][path[0]][path[1]].as_f64().unwrap();
    // The central point of the projection, Amersfoort, is the false origin.
    let c = fwd(serde_json::json!({
        "lat": 52.0 + 9.0 / 60.0 + 22.178 / 3600.0,
        "lon": 5.0 + 23.0 / 60.0 + 15.5 / 3600.0,
        "height": "43 m",
    }));
    assert_eq!(c["ok"], true, "{c}");
    // The correction grid moves the point about 100 m before it is projected,
    // so the answer is near the false origin, not on it.
    assert!(
        (num(&c, ["easting", "value"]) - 155_000.0).abs() < 200.0
            && (num(&c, ["northing", "value"]) - 463_000.0).abs() < 200.0,
        "{c}"
    );
    assert!(
        (c["result"]["scale_factor"].as_f64().unwrap() - 0.999_907_9).abs() < 1e-7,
        "the scale at the central point is the projection's own: {c}"
    );
    assert!(
        num(&c, ["convergence", "value"]).abs() < 0.01,
        "grid north is true north at the central point: {c}"
    );

    for (lat, lon, h) in [
        (52.373_056_f64, 4.892_778_f64, 45.0_f64),
        (51.909_400, 4.486_800, 43.5),
        (53.219_400, 6.568_100, 45.5),
        (50.848_900, 5.688_900, 92.0),
        (52.223_300, 6.890_000, 48.0),
        (51.498_900, 3.613_100, 41.0),
    ] {
        let f = fwd(serde_json::json!({"lat": lat, "lon": lon, "height": format!("{h} m")}));
        assert_eq!(f["ok"], true, "{f}");
        let (x, y) = (
            num(&f, ["easting", "value"]),
            num(&f, ["northing", "value"]),
        );
        // RD puts the whole country in the first quadrant, under 300 km east
        // and between 300 and 620 km north.
        assert!((0.0..300_000.0).contains(&x), "{x} at {lat}, {lon}");
        assert!((300_000.0..620_000.0).contains(&y), "{y} at {lat}, {lon}");
        // NLGEO2018 is 39 to 49 m over its whole grid, and H = h − N.
        let n = num(&f, ["geoid_height", "value"]);
        assert!((39.0..49.0).contains(&n), "quasi-geoid {n} at {lat}, {lon}");
        assert!(
            (num(&f, ["nap_height", "value"]) - (h - n)).abs() < 1e-9,
            "H = h − N: {f}"
        );
        // Grid north leans the way the meridian does: east of the central
        // meridian the convergence is positive, west of it negative, and it
        // is small, under 2.5° anywhere the grids reach. A convergence in the
        // wrong units would be 57 times this.
        let conv = num(&f, ["convergence", "value"]);
        let lon0 = 5.0_f64 + 23.0 / 60.0 + 15.5 / 3600.0;
        assert!(conv.abs() < 2.5, "convergence {conv} at {lat}, {lon}");
        assert!(
            conv.signum() == (lon - lon0).signum(),
            "convergence {conv} at {lon} against the central meridian {lon0}"
        );
        // Roughly (λ − λ₀) sin φ, the spherical value, within a tenth.
        let about = (lon - lon0) * lat.to_radians().sin();
        assert!(
            (conv - about).abs() < 0.1,
            "convergence {conv} against {about}"
        );
        // The scale factor stays near unity over the country.
        let k = f["result"]["scale_factor"].as_f64().unwrap();
        assert!((0.9999..1.0004).contains(&k), "scale {k} at {lat}, {lon}");

        // And back: the same point, to a nanodegree and a tenth of a millimetre.
        let r = inv(serde_json::json!({
            "easting": format!("{x} m"),
            "northing": format!("{y} m"),
            "nap_height": format!("{} m", h - n),
        }));
        assert_eq!(r["ok"], true, "{r}");
        assert!((num(&r, ["lat", "value"]) - lat).abs() < 1e-9, "{r}");
        assert!((num(&r, ["lon", "value"]) - lon).abs() < 1e-9, "{r}");
        assert!((num(&r, ["height", "value"]) - h).abs() < 1e-4, "{r}");
        // The two tools report the same factors at the same point.
        assert!(
            (num(&r, ["convergence", "value"]) - conv).abs() < 1e-9,
            "{r}"
        );
    }

    // Outside the grids there is no answer, on any side.
    for (lat, lon) in [
        (48.858_200, 2.294_500),
        (53.550_000, 9.993_000),
        (51.034_800, 1.900_000),
        (49.950_000, 5.000_000),
        (56.100_000, 5.000_000),
    ] {
        let f = fwd(serde_json::json!({"lat": lat, "lon": lon, "height": "43 m"}));
        assert_eq!(f["error"]["code"], "OUT_OF_DOMAIN", "{lat}, {lon}: {f}");
    }
    let r = inv(serde_json::json!({
        "easting": "1500000 m", "northing": "463000 m", "nap_height": "0 m",
    }));
    assert_eq!(r["error"]["code"], "OUT_OF_DOMAIN", "{r}");
}

#[test]
fn chenyx06_removes_the_swiss_distortion_and_round_trips() {
    // add-international-grids, geodesy/national-grids: LV03 to LV95 by
    // swisstopo's CHENyx06, read from the committed package.
    gp_base::assets::put(
        "chenyx06@2009/chenyx06a.grid",
        &std::fs::read(format!(
            "{}/../../../assets/data/chenyx06/2009/chenyx06a.grid",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap(),
    );
    let call = |input: serde_json::Value| -> Value {
        serde_json::from_str(&REGISTRY.invoke("geodesy.datum.chenyx06", &input.to_string()))
            .unwrap()
    };
    let num = |v: &Value, path: [&str; 2]| v["result"][path[0]][path[1]].as_f64().unwrap();
    for (e03, n03) in [
        (600_421.0_f64, 199_498.0_f64),
        (683_206.0, 248_234.0),
        (500_675.0, 117_769.0),
        (717_252.0, 96_025.0),
        (745_509.0, 254_309.0),
        (624_222.0, 97_277.0),
    ] {
        let f = call(serde_json::json!({
            "easting": format!("{e03} m"), "northing": format!("{n03} m"),
        }));
        assert_eq!(f["ok"], true, "{f}");
        let (e, n) = (
            num(&f, ["easting", "value"]),
            num(&f, ["northing", "value"]),
        );
        // LV95 is LV03 plus a round 2,000 km and the distortion, which
        // swisstopo puts at up to 1.6 m and is never zero.
        let (de, dn) = (
            num(&f, ["east_shift", "value"]),
            num(&f, ["north_shift", "value"]),
        );
        assert!(
            (e - (e03 + 2_000_000.0) - de).abs() < 1e-9,
            "the shift is the difference: {f}"
        );
        assert!(
            (n - (n03 + 1_000_000.0) - dn).abs() < 1e-9,
            "the shift is the difference: {f}"
        );
        let shift = num(&f, ["shift", "value"]);
        assert!((shift - de.hypot(dn)).abs() < 1e-9, "{f}");
        assert!(
            (0.01..1.7).contains(&shift),
            "{shift} m of distortion at {e03}, {n03}"
        );
        // And the reverse lands back on the LV03 point, to a tenth of a
        // millimetre.
        let r = call(serde_json::json!({
            "easting": format!("{e} m"), "northing": format!("{n} m"), "direction": "to-lv03",
        }));
        assert_eq!(r["ok"], true, "{r}");
        assert!((num(&r, ["easting", "value"]) - e03).abs() < 1e-4, "{r}");
        assert!((num(&r, ["northing", "value"]) - n03).abs() < 1e-4, "{r}");
        // Both directions report the same distortion, LV03 to LV95.
        assert!((num(&r, ["east_shift", "value"]) - de).abs() < 1e-6, "{r}");
        assert!((num(&r, ["north_shift", "value"]) - dn).abs() < 1e-6, "{r}");
    }
    // Outside the grid there is no answer.
    for (e, n) in [(530_000.0, -40_000.0), (1_000_000.0, 300_000.0)] {
        let f =
            call(serde_json::json!({"easting": format!("{e} m"), "northing": format!("{n} m")}));
        assert_eq!(f["error"]["code"], "OUT_OF_DOMAIN", "{e}, {n}: {f}");
    }
}

#[test]
fn beta2007_crosses_the_german_grids_and_picks_its_zones() {
    // add-international-grids, geodesy/national-grids: DHDN Gauss-Krüger to
    // ETRS89 UTM by AdV's BeTA2007, read from the committed package.
    gp_base::assets::put(
        "beta2007@2009/beta2007.grid",
        &std::fs::read(format!(
            "{}/../../../assets/data/beta2007/2009/beta2007.grid",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap(),
    );
    let fwd = |input: serde_json::Value| -> Value {
        serde_json::from_str(&REGISTRY.invoke("geodesy.datum.beta2007", &input.to_string()))
            .unwrap()
    };
    let inv = |input: serde_json::Value| -> Value {
        serde_json::from_str(&REGISTRY.invoke("geodesy.datum.beta2007-inverse", &input.to_string()))
            .unwrap()
    };
    let num = |v: &Value, path: [&str; 2]| v["result"][path[0]][path[1]].as_f64().unwrap();

    // AdV's first point, given on Gauss-Krüger zones 2 and 3: the same ground
    // point, so the same UTM answer, which is the zone-prefixed easting
    // working as intended.
    let a = fwd(serde_json::json!({"easting": "2598417.333192 m", "northing": "5930677.980308 m"}));
    let b = fwd(serde_json::json!({"easting": "3399371.190396 m", "northing": "5930724.531323 m"}));
    assert_eq!(a["ok"], true, "{a}");
    assert_eq!(b["ok"], true, "{b}");
    for f in ["easting", "northing"] {
        assert!(
            (num(&a, [f, "value"]) - num(&b, [f, "value"])).abs() < 1e-5,
            "zones 2 and 3 disagree on {f}: {a} {b}"
        );
    }
    assert_eq!(a["result"]["zone"], 32, "{a}");

    for (gk, east, north) in [
        (4u8, 4_599_289.0_f64, 5_820_787.0_f64),
        (3u8, 3_392_332.0, 5_644_501.0),
        (2u8, 2_567_468.0, 5_644_890.0),
        (4u8, 4_468_390.0, 5_333_776.0),
        (4u8, 4_641_000.0, 6_002_000.0),
    ] {
        let f = fwd(serde_json::json!({
            "easting": format!("{east} m"), "northing": format!("{north} m"),
        }));
        assert_eq!(f["ok"], true, "zone {gk}: {f}");
        let zone = f["result"]["zone"].as_u64().unwrap();
        assert!((32..=33).contains(&zone), "UTM zone {zone} in Germany: {f}");
        // The datum move between DHDN90 and ETRS89 is metres across Germany,
        // never zero and never the hundreds of metres a wrong ellipsoid gives.
        let shift = num(&f, ["shift", "value"]);
        assert!(
            (10.0..1000.0).contains(&shift),
            "{shift} m at {east}, {north}"
        );
        // UTM's scale stays near 0.9996 and grid north leans under 3° from
        // true north anywhere in Germany. A convergence in radians or in the
        // wrong units would be far outside this.
        let k = f["result"]["scale_factor"].as_f64().unwrap();
        assert!(
            (0.9996..1.0006).contains(&k),
            "scale {k} at {east}, {north}"
        );
        assert!(num(&f, ["convergence", "value"]).abs() < 3.0, "{f}");

        // And back onto the same Gauss-Krüger zone, to a tenth of a millimetre.
        let r = inv(serde_json::json!({
            "easting": format!("{} m", num(&f, ["easting", "value"])),
            "northing": format!("{} m", num(&f, ["northing", "value"])),
            "utm_zone": zone,
            "gk_zone": gk,
        }));
        assert_eq!(r["ok"], true, "{r}");
        assert_eq!(r["result"]["zone"], gk, "{r}");
        assert!((num(&r, ["easting", "value"]) - east).abs() < 1e-4, "{r}");
        assert!((num(&r, ["northing", "value"]) - north).abs() < 1e-4, "{r}");
        // Both directions report the same datum shift at the same point.
        assert!((num(&r, ["shift", "value"]) - shift).abs() < 1e-3, "{r}");
    }

    // A forced zone says so, and still answers.
    let f = fwd(serde_json::json!({
        "easting": "3399371.190396 m", "northing": "5930724.531323 m", "utm_zone": 33,
    }));
    assert_eq!(f["ok"], true, "{f}");
    assert_eq!(f["result"]["zone"], 33, "{f}");
    assert!(
        f["meta"]["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["code"] == "NONSTANDARD_ZONE"),
        "{f}"
    );

    // An easting with no zone in front of it is refused, and so is a point
    // off the grid.
    let f = fwd(serde_json::json!({"easting": "399340.602 m", "northing": "5928794.178 m"}));
    assert_eq!(f["error"]["code"], "INVALID_INPUT", "{f}");
    let f = fwd(serde_json::json!({"easting": "2300000 m", "northing": "5100000 m"}));
    assert_eq!(f["error"]["code"], "OUT_OF_DOMAIN", "{f}");
    let r = inv(serde_json::json!({
        "easting": "500000 m", "northing": "4000000 m", "utm_zone": 32,
    }));
    assert_eq!(r["error"]["code"], "OUT_OF_DOMAIN", "{r}");
}

#[test]
fn ch1903plus_moves_switzerland_south_west_and_up_and_round_trips() {
    // add-international-grids, geodesy/national-grids: the CH1903+ to ETRS89
    // datum step by swisstopo's three parameters.
    let call = |input: serde_json::Value| -> Value {
        serde_json::from_str(&REGISTRY.invoke("geodesy.datum.ch1903plus", &input.to_string()))
            .unwrap()
    };
    let num = |v: &Value, path: [&str; 2]| v["result"][path[0]][path[1]].as_f64().unwrap();
    for (lat, lon, h) in [
        (46.947975_f64, 7.447447_f64, 590.0_f64),
        (47.370080, 8.544200, 455.0),
        (46.207400, 6.155800, 420.0),
        (46.497800, 9.838300, 1870.0),
        (46.004400, 8.951600, 320.0),
    ] {
        let f = call(serde_json::json!({
            "lat": lat, "lon": lon, "height": format!("{h} m"),
        }));
        assert_eq!(f["ok"], true, "{f}");
        let (a, b) = (num(&f, ["lat", "value"]), num(&f, ["lon", "value"]));
        // The translation carries a Swiss position south, west and up: about
        // 145 m of latitude, 70 m of longitude, and 50 m of height.
        assert!(a < lat && b < lon, "south and west: {f}");
        let shift = num(&f, ["shift", "value"]);
        assert!((140.0..190.0).contains(&shift), "{shift} m at {lat}, {lon}");
        let up = num(&f, ["height_change", "value"]);
        assert!(
            (45.0..55.0).contains(&up),
            "{up} m of height at {lat}, {lon}"
        );
        // The azimuth is into the third quadrant, south-west.
        let az = num(&f, ["azimuth", "value"]);
        assert!((180.0..270.0).contains(&az), "azimuth {az} at {lat}, {lon}");
        // The reported shift is the horizontal move the answer implies.
        assert!(
            (num(&f, ["height", "value"]) - (h + up)).abs() < 1e-9,
            "the height change is the difference: {f}"
        );

        // And back, to a nanodegree and a tenth of a millimetre.
        let r = call(serde_json::json!({
            "lat": a, "lon": b, "height": format!("{} m", num(&f, ["height", "value"])),
            "direction": "to-ch1903-plus",
        }));
        assert_eq!(r["ok"], true, "{r}");
        assert!((num(&r, ["lat", "value"]) - lat).abs() < 1e-11, "{r}");
        assert!((num(&r, ["lon", "value"]) - lon).abs() < 1e-11, "{r}");
        assert!((num(&r, ["height", "value"]) - h).abs() < 1e-4, "{r}");
        // Going back moves it the other way, by the same amount. The two
        // directions measure the move on different ellipsoids and from
        // different latitudes, so they agree to centimetres rather than
        // exactly.
        assert!((num(&r, ["shift", "value"]) - shift).abs() < 0.05, "{r}");
        assert!(
            (num(&r, ["height_change", "value"]) + up).abs() < 1e-6,
            "{r}"
        );

        // Every answer carries swisstopo's warning about CH1903 against
        // CH1903+, which is the 1.6 m trap this tool cannot detect for you.
        let note = f["meta"]["context"]["ch1903_note"]
            .as_str()
            .or_else(|| f["context"]["ch1903_note"].as_str())
            .unwrap_or("");
        assert!(note.contains("1.6 m"), "the CH1903 warning is missing: {f}");
    }
    // The height is part of the step: 100 m of height error moves the
    // latitude about 2 mm, which is the figure the tool's limitations give.
    let a = call(serde_json::json!({"lat": 46.95, "lon": 7.45, "height": "500 m"}));
    let b = call(serde_json::json!({"lat": 46.95, "lon": 7.45, "height": "600 m"}));
    let d = (num(&a, ["lat", "value"]) - num(&b, ["lat", "value"])).abs() * 111_320.0;
    assert!(
        (0.001..0.004).contains(&d),
        "100 m of height moved the latitude {d} m"
    );
}
