//! Terrain derivatives against GDAL's gdaldem: the spec's "GDAL agreement"
//! scenario. 1,452 windows from three projected DEMs (5, 10, and 30 m cells,
//! one with a flat patch), every interior cell's 3 x 3 window, against
//! gdaldem's slope, aspect, hillshade, TRI (Riley and Wilson), TPI, and
//! roughness at that cell (tools/vectors/gen_terrain_gdal.py).

use gp_raster::REGISTRY;
use serde_json::{Value, json};

#[test]
fn terrain_matches_gdaldem() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/terrain_gdal.json");
    let fx: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("JSON");
    let windows = fx["windows"].as_array().unwrap();
    assert!(windows.len() > 1_400);
    let (mut worst_slope, mut worst_aspect, mut worst_len, mut worst_shade) =
        (0.0f64, 0.0f64, 0.0f64, 0i64);
    let mut wrong = Vec::new();
    for (k, w) in windows.iter().enumerate() {
        let rows: Vec<Value> = w["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| json!({"row": r}))
            .collect();
        let cell = w["cell_size"].as_f64().unwrap();
        let input = json!({"elevations": rows, "cell_size": format!("{cell} m")}).to_string();
        let s: Value =
            serde_json::from_str(&REGISTRY.invoke("raster.terrain.slope-aspect", &input)).unwrap();
        let r: Value =
            serde_json::from_str(&REGISTRY.invoke("raster.terrain.ruggedness", &input)).unwrap();
        let slope = s["result"]["slope"]["value"].as_f64().unwrap();
        worst_slope = worst_slope.max((slope - w["slope"].as_f64().unwrap()).abs());
        match (
            w["aspect"].as_f64(),
            s["result"]["aspect"]["value"].as_f64(),
        ) {
            (Some(a), Some(b)) => {
                let d = (a - b).rem_euclid(360.0);
                let d = d.min(360.0 - d);
                // gdaldem holds elevations as 32-bit floats. On nearly flat
                // ground aspect is ill-conditioned: rounding the elevations
                // (a 32-bit epsilon of the largest, in four terms of each Horn
                // sum) moves the gradient by up to dg, and the aspect by about
                // dg / |g| radians. The spec's 0.01 degrees is held from half a
                // degree of slope up; below that, twice gdaldem's own bound.
                let zmax = w["rows"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .flat_map(|r| {
                        r.as_str()
                            .unwrap()
                            .split(',')
                            .map(|x| x.trim().parse::<f64>().unwrap().abs())
                    })
                    .fold(0.0, f64::max);
                let dg =
                    4.0 * zmax * f64::from(f32::EPSILON) * std::f64::consts::SQRT_2 / (8.0 * cell);
                let bound = if slope >= 0.5 {
                    0.01
                } else {
                    (2.0 * dg / slope.to_radians().tan()).to_degrees().max(0.01)
                };
                if d > bound {
                    wrong.push(format!(
                        "window {k}: aspect {b} against gdaldem {a} ({d} deg at slope {slope})"
                    ));
                }
                if slope >= 0.5 {
                    worst_aspect = worst_aspect.max(d);
                }
            }
            // gdaldem marks a flat cell's aspect as no-data; the tool says flat.
            (None, None) => assert_eq!(s["result"]["aspect_text"], "flat"),
            (a, b) => wrong.push(format!("window {k}: aspect {a:?} against {b:?}")),
        }
        let shade = s["result"]["hillshade"].as_f64().unwrap() as i64;
        worst_shade =
            worst_shade.max((shade - w["hillshade"].as_f64().unwrap().round() as i64).abs());
        for f in ["tri", "tri_mean", "tpi", "roughness"] {
            let got = r["result"][f]["value"].as_f64().unwrap();
            worst_len = worst_len.max((got - w[f].as_f64().unwrap()).abs());
        }
    }
    println!(
        "worst: slope {worst_slope} deg, aspect {worst_aspect} deg from half a degree of slope, {worst_len} m, hillshade {worst_shade}"
    );
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert!(worst_slope < 1e-3, "slope off by {worst_slope} deg");
    assert!(worst_aspect < 0.01, "aspect off by {worst_aspect} deg");
    assert!(
        worst_len < 1e-3,
        "TRI, TPI, or roughness off by {worst_len} m"
    );
    assert!(worst_shade == 0, "hillshade off by {worst_shade}");
}

fn window(z: &[f64; 9], cell: f64) -> String {
    let rows: Vec<Value> = (0..3)
        .map(|i| json!({"row": format!("{}, {}, {}", z[3 * i], z[3 * i + 1], z[3 * i + 2])}))
        .collect();
    json!({"elevations": rows, "cell_size": format!("{cell} m")}).to_string()
}

fn get(tool: &str, z: &[f64; 9], cell: f64, k: &str) -> f64 {
    let r: Value = serde_json::from_str(&REGISTRY.invoke(tool, &window(z, cell))).unwrap();
    r["result"][k]["value"]
        .as_f64()
        .unwrap_or_else(|| panic!("{k}: {r}"))
}

/// What the terrain measures must do under moves that change the ground in
/// known ways.
#[test]
fn terrain_invariants() {
    let windows: [[f64; 9]; 4] = [
        [101.2, 100.6, 100.2, 100.4, 99.8, 99.2, 99.6, 99.0, 98.4],
        [
            250.0, 252.5, 255.1, 248.2, 251.0, 254.9, 246.0, 249.3, 253.3,
        ],
        [10.0, 12.0, 11.0, 9.0, 13.0, 14.0, 8.5, 11.5, 12.5],
        [
            500.0, 498.0, 495.0, 503.0, 500.0, 497.0, 507.0, 504.0, 500.0,
        ],
    ];
    for z in windows {
        let (slope, aspect) = (
            get("raster.terrain.slope-aspect", &z, 10.0, "slope"),
            get("raster.terrain.slope-aspect", &z, 10.0, "aspect"),
        );
        // Turning the window a quarter turn clockwise (north row becomes the
        // east column) turns the aspect 90 degrees and keeps the slope.
        let r = [z[6], z[3], z[0], z[7], z[4], z[1], z[8], z[5], z[2]];
        let (s2, a2) = (
            get("raster.terrain.slope-aspect", &r, 10.0, "slope"),
            get("raster.terrain.slope-aspect", &r, 10.0, "aspect"),
        );
        assert!((s2 - slope).abs() < 1e-9, "{z:?}");
        let d = (a2 - aspect - 90.0).rem_euclid(360.0);
        assert!(
            d.min(360.0 - d) < 1e-9,
            "{z:?}: aspect {aspect} turned to {a2}"
        );
        // Raising the ground changes none of the measures.
        let up = z.map(|v| v + 1000.0);
        for (tool, k) in [
            ("raster.terrain.slope-aspect", "slope"),
            ("raster.terrain.ruggedness", "tri"),
            ("raster.terrain.ruggedness", "tpi"),
            ("raster.terrain.ruggedness", "roughness"),
        ] {
            assert!(
                (get(tool, &up, 10.0, k) - get(tool, &z, 10.0, k)).abs() < 1e-9,
                "{k}"
            );
        }
        // Doubling the relief doubles tan(slope), and doubling the cell halves it.
        let steep = z.map(|v| 2.0 * v);
        let t = slope.to_radians().tan();
        assert!(
            (get("raster.terrain.slope-aspect", &steep, 10.0, "slope")
                .to_radians()
                .tan()
                - 2.0 * t)
                .abs()
                < 1e-9
        );
        assert!(
            (get("raster.terrain.slope-aspect", &z, 20.0, "slope")
                .to_radians()
                .tan()
                - 0.5 * t)
                .abs()
                < 1e-12
        );
        // Turning the ground upside down negates TPI and keeps TRI and roughness.
        let down = z.map(|v| -v);
        assert!(
            (get("raster.terrain.ruggedness", &down, 10.0, "tpi")
                + get("raster.terrain.ruggedness", &z, 10.0, "tpi"))
            .abs()
                < 1e-9
        );
        for k in ["tri", "tri_mean", "roughness"] {
            assert!(
                (get("raster.terrain.ruggedness", &down, 10.0, k)
                    - get("raster.terrain.ruggedness", &z, 10.0, k))
                .abs()
                    < 1e-9,
                "{k}"
            );
        }
    }
}

/// The focused terrain operations give exactly their parents' answers.
#[test]
fn focused_terrain_tools_match_their_parents() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/terrain_gdal.json");
    let fx: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("JSON");
    let call = |tool: &str, input: &str| -> Value {
        serde_json::from_str(&REGISTRY.invoke(tool, input)).unwrap()
    };
    for w in fx["windows"].as_array().unwrap().iter().step_by(7) {
        let rows: Vec<Value> = w["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| json!({"row": r}))
            .collect();
        let input = json!({"elevations": rows, "cell_size": format!("{} m", w["cell_size"]), "sun_azimuth": "200 deg"}).to_string();
        let plain =
            json!({"elevations": rows, "cell_size": format!("{} m", w["cell_size"])}).to_string();
        let sa = call("raster.terrain.slope-aspect", &input);
        let rg = call("raster.terrain.ruggedness", &plain);
        for (tool, parent, inp, keys) in [
            (
                "raster.terrain.slope",
                &sa,
                &plain,
                &["slope", "slope_percent"][..],
            ),
            (
                "raster.terrain.aspect",
                &sa,
                &plain,
                &["aspect", "aspect_text"][..],
            ),
            ("raster.terrain.hillshade", &sa, &input, &["hillshade"][..]),
            (
                "raster.terrain.roughness-tri",
                &rg,
                &plain,
                &["tri", "tri_mean"][..],
            ),
            ("raster.terrain.tpi", &rg, &plain, &["tpi", "position"][..]),
        ] {
            let got = call(tool, inp);
            assert_eq!(got["ok"], true, "{tool}: {got}");
            for k in keys {
                assert_eq!(got["result"][k], parent["result"][k], "{tool} {k} {inp}");
            }
        }
    }
}

/// What each focused terrain operation must do under moves that change the
/// ground in known ways, as their derivation notes state.
#[test]
fn focused_terrain_invariants() {
    let run = |tool: &str, z: &[f64; 9], cell: f64| -> Value {
        let r: Value = serde_json::from_str(&REGISTRY.invoke(tool, &window(z, cell))).unwrap();
        assert_eq!(r["ok"], true, "{tool}: {r}");
        r["result"].clone()
    };
    let num = |r: &Value, k: &str| r[k]["value"].as_f64().or(r[k].as_f64()).unwrap();
    let windows: [[f64; 9]; 4] = [
        [101.2, 100.6, 100.2, 100.4, 99.8, 99.2, 99.6, 99.0, 98.4],
        [
            250.0, 252.5, 255.1, 248.2, 251.0, 254.9, 246.0, 249.3, 253.3,
        ],
        [10.0, 12.0, 11.0, 9.0, 13.0, 14.0, 8.5, 11.5, 12.5],
        [
            842.0, 845.5, 841.2, 846.1, 851.3, 847.0, 840.4, 844.8, 839.9,
        ],
    ];
    for z in windows {
        let turned = [z[6], z[3], z[0], z[7], z[4], z[1], z[8], z[5], z[2]];
        let up = z.map(|v| v + 1000.0);
        let down = z.map(|v| -v);
        // Slope: a quarter turn keeps it; raising the ground keeps it.
        let s = num(&run("raster.terrain.slope", &z, 10.0), "slope");
        assert!((num(&run("raster.terrain.slope", &turned, 10.0), "slope") - s).abs() < 1e-9);
        assert!((num(&run("raster.terrain.slope", &up, 10.0), "slope") - s).abs() < 1e-9);
        // Aspect: a quarter turn clockwise turns it 90°.
        let a = num(&run("raster.terrain.aspect", &z, 10.0), "aspect");
        let a2 = num(&run("raster.terrain.aspect", &turned, 10.0), "aspect");
        let d = (a2 - a - 90.0).rem_euclid(360.0);
        assert!(d.min(360.0 - d) < 1e-9, "{z:?}: aspect {a} turned to {a2}");
        // Hillshade: between 1 and 255, and unchanged by raising the ground.
        let h = num(&run("raster.terrain.hillshade", &z, 10.0), "hillshade");
        assert!((1.0..=255.0).contains(&h), "{h}");
        assert_eq!(
            num(&run("raster.terrain.hillshade", &up, 10.0), "hillshade"),
            h
        );
        // TRI: never negative, unchanged by raising or inverting the ground.
        let tri = run("raster.terrain.roughness-tri", &z, 10.0);
        for k in ["tri", "tri_mean"] {
            assert!(num(&tri, k) >= 0.0);
            for moved in [&up, &down] {
                let m = run("raster.terrain.roughness-tri", moved, 10.0);
                assert!((num(&m, k) - num(&tri, k)).abs() < 1e-9, "{k}");
            }
        }
        // TPI: inverting the ground negates it; raising it changes nothing.
        let t = num(&run("raster.terrain.tpi", &z, 10.0), "tpi");
        assert!((num(&run("raster.terrain.tpi", &down, 10.0), "tpi") + t).abs() < 1e-9);
        assert!((num(&run("raster.terrain.tpi", &up, 10.0), "tpi") - t).abs() < 1e-9);
    }
    // A flat cell's hillshade is 1 + 254 sin(sun altitude), rounded: 181 at 45°.
    let flat = [75.0; 9];
    assert_eq!(
        num(&run("raster.terrain.hillshade", &flat, 30.0), "hillshade"),
        181.0
    );
    // A plane through the window has no position: TPI is zero.
    let plane = [12.0, 14.0, 16.0, 10.0, 12.0, 14.0, 8.0, 10.0, 12.0];
    assert_eq!(num(&run("raster.terrain.tpi", &plane, 1.0), "tpi"), 0.0);
}
