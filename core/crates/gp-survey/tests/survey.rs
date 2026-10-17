//! Survey tools: catalog lint, examples, golden vectors, spec scenarios, and hardening.

use std::path::Path;

use gp_base::{manifest, template, vectors};
use gp_survey::{REGISTRY, TOOLS};
use serde_json::Value;

fn repo(path: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(path);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

fn call(id: &str, input: &str) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, input)).expect("envelope is JSON")
}

fn num(r: &Value, path: &str) -> f64 {
    path.split('.')
        .fold(r, |v, k| &v[k])
        .as_f64()
        .unwrap_or_else(|| panic!("{path} missing in {r}"))
}

fn codes(r: &Value) -> Vec<String> {
    r["meta"]["warnings"].as_array().map_or(vec![], |a| {
        a.iter()
            .map(|w| w["code"].as_str().unwrap().to_owned())
            .collect()
    })
}

fn angular(input: Value) -> Value {
    call("survey.cogo.angular-closure", &input.to_string())
}

fn angle_value(text: &str) -> f64 {
    gp_geo::dms::parse_plain(text).unwrap_or_else(|e| panic!("parse {text}: {e:?}"))
}

#[test]
fn catalog_examples_vectors() {
    let tax: Value = serde_json::from_str(&repo("data/taxonomy.json")).unwrap();
    let owned: Vec<(String, Vec<String>)> = tax["domains"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(d, v)| {
            (
                d.clone(),
                v["groups"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|g| g.as_str().unwrap().to_owned())
                    .collect(),
            )
        })
        .collect();
    let taxonomy: Vec<(&str, Vec<&str>)> = owned
        .iter()
        .map(|(d, g)| (d.as_str(), g.iter().map(String::as_str).collect()))
        .collect();
    // Tools in other crates that survey tools point at; this crate cannot see
    // them, so they are named here.
    let mut failures = manifest::lint(
        TOOLS,
        &taxonomy,
        &[
            "geodesy.spcs.spcs83-forward",
            "time.scale.gps-week",
            "units.length.convert",
            "units.slope.convert",
        ],
    );
    let reg: Value = serde_json::from_str(&repo("data/codes.json")).unwrap();
    for t in TOOLS {
        failures.extend(
            t.warnings
                .iter()
                .filter(|w| reg["warnings"].get(**w).is_none())
                .map(|w| format!("{} unregistered {w}", t.id)),
        );
        for ex in t.examples {
            let r = call(t.id, ex.input);
            let s = r["summary"].as_str().unwrap_or_default();
            if r["ok"] != true || template::grade(s) > 8.0 || s.len() > template::MAX_CHARS {
                failures.push(format!(
                    "{} example (grade {:.1}): {r}",
                    t.id,
                    template::grade(s)
                ));
            }
        }
        let text = repo(&format!("core/vectors/{}.jsonl", t.id));
        failures.extend(vectors::lint(t.id, &text));
        failures.extend(vectors::run(&REGISTRY, t.id, &text));
        if vectors::count(&text) < 5 {
            failures.push(format!("{} has fewer than 5 vectors", t.id));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn inverse_and_bearings() {
    let parsed = call(
        "survey.cogo.direction-parse",
        r#"{"direction":"S 44°30'00\" W","reference":"grid"}"#,
    );
    assert_eq!(num(&parsed, "result.azimuth.value"), 224.5);
    assert_eq!(parsed["result"]["azimuth_dms"], "224°30'00.00\"");
    assert_eq!(parsed["result"]["bearing"], "S 44°30'00.00\" W");
    assert_eq!(parsed["result"]["reference"], "grid");
    assert!((num(&parsed, "result.gons.value") - 224.5 / 0.9).abs() < 1e-12);
    let invalid = call(
        "survey.cogo.direction-parse",
        r#"{"direction":"N 95°00'00\" E"}"#,
    );
    assert_eq!(invalid["error"]["code"], "INVALID_INPUT");
    assert_eq!(invalid["error"]["field"], "/direction");

    let r = call(
        "survey.cogo.inverse",
        r#"{"northing1":1000,"easting1":1000,"northing2":1100,"easting2":1100}"#,
    );
    assert_eq!(r["result"]["bearing"], "N 45°00'00\" E");
    assert_eq!(r["display"]["distance"], "141.421 ft");
    let f = call(
        "survey.cogo.forward",
        r#"{"northing":0,"easting":0,"direction":"S 44°30'00\" W","distance":100}"#,
    );
    let back = call(
        "survey.cogo.inverse",
        &format!(
            r#"{{"northing1":0,"easting1":0,"northing2":{},"easting2":{}}}"#,
            num(&f, "result.northing.value"),
            num(&f, "result.easting.value")
        ),
    );
    assert!(
        (num(&back, "result.azimuth.value") - 224.5).abs() < 1e-9,
        "{back}"
    );
    let same = call(
        "survey.cogo.inverse",
        r#"{"northing1":1,"easting1":1,"northing2":1,"easting2":1}"#,
    );
    assert!(codes(&same).contains(&"AZIMUTH_UNDEFINED".to_owned()));
    let bad = call(
        "survey.cogo.forward",
        r#"{"northing":0,"easting":0,"direction":"N 95 E","distance":1}"#,
    );
    assert_eq!(bad["error"]["code"], "INVALID_INPUT");
}

#[test]
fn named_offset_shots_match_the_general_tool() {
    let cases = [
        (
            "survey.reduction.distance-offset",
            serde_json::json!({
                "northing": "5000 ft", "easting": "5000 ft", "direction": "N 30°00'00\" E",
                "distance": "150 ft", "offset_right": "2.5 ft", "offset_out": "1 ft"
            }),
        ),
        (
            "survey.reduction.distance-offset",
            serde_json::json!({
                "northing": "1000 m", "easting": "2000 m", "direction": "315",
                "distance": "200 m", "offset_right": "-3 m"
            }),
        ),
        (
            "survey.reduction.angle-offset",
            serde_json::json!({
                "northing": "5000 ftUS", "easting": "5000 ftUS", "direction": "N 30°00'00\" E",
                "distance": "150 ftUS", "center_direction": "N 30°17'00\" E", "radius": "0.75 ftUS"
            }),
        ),
    ];
    for (tool, input) in cases {
        let named = call(tool, &input.to_string());
        let general = call("survey.cogo.offset-shot", &input.to_string());
        assert_eq!(named["ok"], true, "{tool}: {named}");
        assert_eq!(named["result"], general["result"], "{tool}");
        assert_eq!(named["display"], general["display"], "{tool}");
        assert_eq!(
            named["meta"]["warnings"], general["meta"]["warnings"],
            "{tool}"
        );
    }
    // Right is a quarter turn clockwise from the line of sight: looking north, east.
    let r = call(
        "survey.reduction.distance-offset",
        r#"{"northing":"0 m","easting":"0 m","direction":"0","distance":"100 m","offset_right":"3 m","offset_out":"-2 m"}"#,
    );
    assert!((num(&r, "result.northing.value") - 98.0).abs() < 1e-9);
    assert!((num(&r, "result.easting.value") - 3.0).abs() < 1e-9);
    // An angle offset puts the center a radius beyond the face, on the turned direction.
    let r = call(
        "survey.reduction.angle-offset",
        r#"{"northing":"0 m","easting":"0 m","direction":"88","distance":"40 m","center_direction":"90","radius":"0.5 m"}"#,
    );
    assert!(num(&r, "result.northing.value").abs() < 1e-9);
    assert!((num(&r, "result.easting.value") - 40.5).abs() < 1e-9);
    // Each focused operation refuses the other method's inputs rather than ignoring them.
    for (tool, input) in [
        (
            "survey.reduction.distance-offset",
            r#"{"northing":"0 m","easting":"0 m","direction":"0","distance":"10 m","offset_right":"1 m","radius":"1 m"}"#,
        ),
        (
            "survey.reduction.angle-offset",
            r#"{"northing":"0 m","easting":"0 m","direction":"0","distance":"10 m","center_direction":"1","radius":"1 m","offset_right":"1 m"}"#,
        ),
    ] {
        assert_eq!(call(tool, input)["ok"], false, "{tool}");
    }
}

#[test]
fn named_factor_operations_match_the_combined_factor() {
    let heights = [
        serde_json::json!({"ellipsoid_height": "1500 ft"}),
        serde_json::json!({"elevation": "865 ft", "geoid_height": "-100 ft", "radius": "20906000 ft"}),
        serde_json::json!({"elevation": "410.2 m"}),
    ];
    for height in heights {
        let mut general_input = height.clone();
        general_input["grid_scale"] = serde_json::json!(1.000045);
        let alone = call("survey.reduction.elevation-factor", &height.to_string());
        let mut to_grid_input = general_input.clone();
        to_grid_input["ground_distance"] = serde_json::json!("4805.468 ft");
        let general = call(
            "survey.reduction.combined-factor",
            &to_grid_input.to_string(),
        );
        let to_grid = call(
            "survey.reduction.ground-to-grid",
            &to_grid_input.to_string(),
        );
        assert_eq!(alone["ok"], true, "{alone}");
        assert_eq!(to_grid["ok"], true, "{to_grid}");
        assert_eq!(
            alone["result"]["elevation_factor"], general["result"]["elevation_factor"],
            "{height}"
        );
        for key in [
            "grid_distance",
            "ground_distance",
            "combined_factor",
            "elevation_factor",
        ] {
            assert_eq!(
                to_grid["result"][key], general["result"][key],
                "{key} {height}"
            );
            assert_eq!(
                to_grid["display"][key], general["display"][key],
                "{key} {height}"
            );
        }
        // The focused operations are experimental; the engine's own warnings must match.
        let own = |r: &Value| -> Vec<String> {
            codes(r)
                .into_iter()
                .filter(|c| c != "EXPERIMENTAL_TOOL")
                .collect()
        };
        assert_eq!(own(&alone), own(&general), "{height}");
        assert_eq!(own(&to_grid), own(&general), "{height}");

        // Back to the ground: the same engine in the other direction, and the round trip.
        let mut to_ground_input = general_input.clone();
        to_ground_input["grid_distance"] = to_grid["result"]["grid_distance"]["value"]
            .as_f64()
            .map(|v| serde_json::json!(format!("{v} ft")))
            .unwrap();
        let general = call(
            "survey.reduction.combined-factor",
            &to_ground_input.to_string(),
        );
        let to_ground = call(
            "survey.reduction.grid-to-ground",
            &to_ground_input.to_string(),
        );
        for key in [
            "grid_distance",
            "ground_distance",
            "combined_factor",
            "elevation_factor",
        ] {
            assert_eq!(
                to_ground["result"][key], general["result"][key],
                "{key} {height}"
            );
        }
        assert!((num(&to_ground, "result.ground_distance.value") - 4805.468).abs() < 1e-9);
    }
    // The elevation factor falls below 1 above the ellipsoid and rises above it below.
    let above = call(
        "survey.reduction.elevation-factor",
        r#"{"ellipsoid_height":"1000 m"}"#,
    );
    let below = call(
        "survey.reduction.elevation-factor",
        r#"{"ellipsoid_height":"-100 m"}"#,
    );
    assert!(num(&above, "result.elevation_factor") < 1.0);
    assert!(num(&below, "result.elevation_factor") > 1.0);
    assert_eq!(num(&above, "result.ellipsoid_height.value"), 1000.0);
    // The first output leads each focused result.
    let r = call(
        "survey.reduction.grid-to-ground",
        r#"{"grid_distance":"1000 ft","grid_scale":0.99991,"ellipsoid_height":"1500 ft"}"#,
    );
    assert!(num(&r, "result.ground_distance.value") > 1000.0);
}

#[test]
fn two_face_mean_matches_the_slope_reduction() {
    // The spec scenario: FL 85°00'10", FR 274°59'40" give 85°00'15" and −5".
    let input = serde_json::json!({"zenith": "85°00'10\"", "zenith_face_right": "274°59'40\""});
    let r = call("survey.reduction.two-face-mean", &input.to_string());
    assert_eq!(r["ok"], true, "{r}");
    assert_eq!(r["result"]["mean_zenith_dms"], "85°00'15.0\"");
    assert!((num(&r, "result.index_error.value") + 5.0).abs() < 1e-9);
    assert!((num(&r, "result.mean_zenith.value") - (85.0 + 15.0 / 3600.0)).abs() < 1e-12);
    assert!((num(&r, "result.vertical_angle.value") - (5.0 - 15.0 / 3600.0)).abs() < 1e-12);
    // The slope reduction reports the same mean and index error from the same readings.
    let mut with_distance = input.clone();
    with_distance["slope_distance"] = serde_json::json!("500 m");
    let slope = call("survey.reduction.slope", &with_distance.to_string());
    for key in ["mean_zenith", "mean_zenith_dms", "index_error"] {
        assert_eq!(r["result"][key], slope["result"][key], "{key}");
    }
    // A perfect instrument has no index error, and the mean is the face-left reading.
    let r = call(
        "survey.reduction.two-face-mean",
        r#"{"zenith":"92.5","zenith_face_right":"267.5"}"#,
    );
    assert_eq!(num(&r, "result.index_error.value"), 0.0);
    assert_eq!(num(&r, "result.mean_zenith.value"), 92.5);
    // An index error shifts both faces the same way and leaves the mean alone.
    let r = call(
        "survey.reduction.two-face-mean",
        r#"{"zenith":"92.51","zenith_face_right":"267.51"}"#,
    );
    assert!((num(&r, "result.mean_zenith.value") - 92.5).abs() < 1e-12);
    assert!((num(&r, "result.index_error.value") - 36.0).abs() < 1e-7);
    // Swapped faces are refused, not silently reordered.
    let r = call(
        "survey.reduction.two-face-mean",
        r#"{"zenith":"274°59'40\"","zenith_face_right":"85°00'10\""}"#,
    );
    assert_eq!(r["ok"], false);
    assert_eq!(r["error"]["field"], "/zenith");
}

#[test]
fn level_adjustment_matches_the_level_run() {
    let input = serde_json::json!({
        "start_elevation": "250 m", "end_elevation": "247.975 m",
        "shots": [
            {"station": "BM A", "backsight": "1.234 m"},
            {"station": "TP 1", "foresight": "0.987 m", "backsight": "2.110 m", "distance": "120 m"},
            {"station": "TP 2", "foresight": "3.004 m", "backsight": "0.542 m", "distance": "95 m"},
            {"station": "BM B", "foresight": "1.876 m", "distance": "150 m"}
        ]
    });
    let adjust = call("survey.reduction.level-adjust", &input.to_string());
    let run = call("survey.reduction.level-run", &input.to_string());
    assert_eq!(adjust["ok"], true, "{adjust}");
    for key in [
        "misclosure",
        "stations",
        "sum_backsights",
        "sum_foresights",
        "check",
    ] {
        assert_eq!(adjust["result"][key], run["result"][key], "{key}");
    }
    let mis = num(&adjust, "result.misclosure.value");
    assert!((mis - 0.044).abs() < 1e-9);
    // The closing benchmark lands on its known elevation and the start does not move.
    let station = |i: usize, key: &str| {
        adjust["result"]["stations"][i][key]["value"]
            .as_f64()
            .unwrap()
    };
    assert!((station(3, "adjusted") - 247.975).abs() < 1e-9);
    assert!((station(0, "adjusted") - 250.0).abs() < 1e-9);
    // Corrections grow with distance: 120, 215, and 365 m of 365 m.
    for (i, cum) in [(1, 120.0), (2, 215.0)] {
        let (observed, adjusted) = (station(i, "elevation"), station(i, "adjusted"));
        assert!(
            (observed - adjusted - mis * cum / 365.0).abs() < 1e-9,
            "station {i}"
        );
    }
    // An open run has nothing to adjust: the reduction accepts it, the adjustment says why not.
    let mut open = input.clone();
    open.as_object_mut().unwrap().remove("end_elevation");
    assert_eq!(
        call("survey.reduction.level-run", &open.to_string())["ok"],
        true
    );
    let refused = call("survey.reduction.level-adjust", &open.to_string());
    assert_eq!(refused["ok"], false);
    assert_eq!(refused["error"]["field"], "/end_elevation");
}

#[test]
fn named_earthwork_operations_match_their_general_tools() {
    // Rise and run: the same grade and lengths as the grade tool, from each pair of sides.
    for input in [
        serde_json::json!({"rise": "2.5 ft", "run": "30 ft"}),
        serde_json::json!({"rise": "3 m", "slope_length": "5 m"}),
        serde_json::json!({"run": "4 ftUS", "slope_length": "5 ftUS"}),
    ] {
        let named = call("survey.earthwork.rise-run", &input.to_string());
        let general = call("survey.earthwork.grade", &input.to_string());
        assert_eq!(named["ok"], true, "{named}");
        for key in [
            "percent",
            "degrees",
            "per_mille",
            "ratio_hv",
            "ratio_vh",
            "rise",
            "run",
            "slope_length",
        ] {
            assert_eq!(
                named["result"][key], general["result"][key],
                "{key} {input}"
            );
        }
    }
    let r = call(
        "survey.earthwork.rise-run",
        r#"{"rise":"3 m","slope_length":"5 m"}"#,
    );
    assert!((num(&r, "result.run.value") - 4.0).abs() < 1e-12);
    assert!((num(&r, "result.percent") - 75.0).abs() < 1e-9);
    // One length is not a slope, and a written grade belongs to the grade tool.
    assert_eq!(
        call("survey.earthwork.rise-run", r#"{"rise":"3 m"}"#)["error"]["field"],
        "/rise"
    );
    assert_eq!(
        call("survey.earthwork.rise-run", r#"{"grade":"5%","run":"3 m"}"#)["ok"],
        false
    );

    // Haul loads: the swell tool's loose volume and load count.
    let input =
        serde_json::json!({"bank_volume": "1000 yd3", "swell": 25, "truck_capacity": "12 yd3"});
    let named = call("survey.earthwork.haul-loads", &input.to_string());
    let general = call("survey.earthwork.shrink-swell", &input.to_string());
    assert_eq!(named["result"]["loads"], general["result"]["loads"]);
    assert_eq!(
        named["result"]["loose_volume"],
        general["result"]["loose_volume"]
    );
    assert_eq!(num(&named, "result.loads"), 105.0);
    // 1,250 loose yd³ is 104 full trucks of 12 and one carrying 2.
    assert!((num(&named, "result.last_load.value") - 2.0).abs() < 1e-9);
    // An exact multiple takes no extra truck, and its last truck is full.
    let exact = call(
        "survey.earthwork.haul-loads",
        r#"{"bank_volume":"960 yd3","swell":25,"truck_capacity":"12 yd3"}"#,
    );
    assert_eq!(num(&exact, "result.loads"), 100.0);
    assert!((num(&exact, "result.last_load.value") - 12.0).abs() < 1e-6);
    let none = call(
        "survey.earthwork.haul-loads",
        r#"{"bank_volume":"0 yd3","swell":25,"truck_capacity":"12 yd3"}"#,
    );
    assert_eq!(num(&none, "result.loads"), 0.0);
    assert_eq!(num(&none, "result.last_load.value"), 0.0);

    // Four-point average: the grid tool's cut and fill, and a net that is their difference.
    let input = serde_json::json!({
        "existing": [{"elevations": "102.0, 101.5, 101.0"}, {"elevations": "101.2, 100.6, 100.2"}, {"elevations": "100.4, 99.8, 99.2"}],
        "cell_size": "25 ft", "finished_grade": "100 ft"
    });
    let named = call("survey.earthwork.four-point-average", &input.to_string());
    let general = call("survey.earthwork.borrow-pit", &input.to_string());
    assert_eq!(named["ok"], true, "{named}");
    for key in ["cut_volume", "fill_volume", "balance_points"] {
        assert_eq!(named["result"][key], general["result"][key], "{key}");
    }
    let (cut, fill, net) = (
        num(&named, "result.cut_volume.value"),
        num(&named, "result.fill_volume.value"),
        num(&named, "result.net_volume.value"),
    );
    assert!((cut - fill - net).abs() < 1e-9);
    // The corner weights reach the same net by a different sum.
    assert!((net - num(&general, "result.net_volume.value")).abs() < 1e-6);
    // Swapping existing and proposed swaps cut and fill.
    let forward = serde_json::json!({
        "existing": [{"elevations": "10, 11"}, {"elevations": "9, 12"}],
        "proposed": [{"elevations": "10.5, 10.5"}, {"elevations": "10.5, 10.5"}], "cell_size": "20 m"
    });
    let reverse = serde_json::json!({
        "proposed": [{"elevations": "10, 11"}, {"elevations": "9, 12"}],
        "existing": [{"elevations": "10.5, 10.5"}, {"elevations": "10.5, 10.5"}], "cell_size": "20 m"
    });
    let (f, r) = (
        call("survey.earthwork.four-point-average", &forward.to_string()),
        call("survey.earthwork.four-point-average", &reverse.to_string()),
    );
    assert!(
        (num(&f, "result.cut_volume.value") - num(&r, "result.fill_volume.value")).abs() < 1e-9
    );
    assert!(
        (num(&f, "result.fill_volume.value") - num(&r, "result.cut_volume.value")).abs() < 1e-9
    );
}

#[test]
fn rise_run_invariants() {
    let rr = |body: &str| call("survey.earthwork.rise-run", body);
    for (rise, run) in [(2.5, 30.0), (3.0, 4.0), (1.0, 12.0), (165.288, 282.834)] {
        let from_sides = rr(&format!(r#"{{"rise":"{rise} ft","run":"{run} ft"}}"#));
        let slope = num(&from_sides, "result.slope_length.value");
        // The three sides make a right triangle.
        assert!(
            (slope * slope - (rise * rise + run * run)).abs() < 1e-6,
            "pythagorean at {rise}/{run}"
        );
        // Each remaining pair of sides returns the same third side.
        let from_slope_run = rr(&format!(
            r#"{{"run":"{run} ft","slope_length":"{slope} ft"}}"#
        ));
        assert!(
            (num(&from_slope_run, "result.rise.value") - rise).abs() < 1e-6,
            "rise from run+slope at {rise}/{run}"
        );
        let from_slope_rise = rr(&format!(
            r#"{{"rise":"{rise} ft","slope_length":"{slope} ft"}}"#
        ));
        assert!(
            (num(&from_slope_rise, "result.run.value") - run).abs() < 1e-6,
            "run from rise+slope at {rise}/{run}"
        );
        // The grade is the same whatever shared unit the two sides use.
        let in_meters = rr(&format!(r#"{{"rise":"{rise} m","run":"{run} m"}}"#));
        assert!(
            (num(&in_meters, "result.percent") - num(&from_sides, "result.percent")).abs() < 1e-9,
            "grade unit-independent at {rise}/{run}"
        );
    }
}

#[test]
fn named_curve_operations_match_their_general_tools() {
    // Degree of curve: each form returns the other two, and the circular curve agrees.
    let from_degree = call("survey.curves.degree-of-curve", r#"{"degree":"4"}"#);
    let radius = num(&from_degree, "result.radius.value");
    assert!((radius - 18_000.0 / std::f64::consts::PI / 4.0).abs() < 1e-9);
    let from_radius = call(
        "survey.curves.degree-of-curve",
        &serde_json::json!({"radius": format!("{radius} ft")}).to_string(),
    );
    assert!((num(&from_radius, "result.degree_arc.value") - 4.0).abs() < 1e-12);
    let chord = num(&from_radius, "result.degree_chord.value");
    let from_chord = call(
        "survey.curves.degree-of-curve",
        &serde_json::json!({"degree_chord": format!("{chord}")}).to_string(),
    );
    assert!((num(&from_chord, "result.radius.value") - radius).abs() < 1e-7);
    let general = call(
        "survey.curves.circular-curve",
        &serde_json::json!({"radius": format!("{radius} ft"), "delta": "24 deg"}).to_string(),
    );
    for key in ["degree_arc", "degree_chord"] {
        assert_eq!(from_radius["result"][key], general["result"][key], "{key}");
    }
    // The chord definition stops at a 50 ft radius; the arc definition does not.
    let tight = call("survey.curves.degree-of-curve", r#"{"radius":"40 ft"}"#);
    assert_eq!(tight["ok"], true);
    assert!(tight["result"]["degree_chord"].is_null());
    // A metric radius is still a degree per 100 ft.
    let metric = call("survey.curves.degree-of-curve", r#"{"radius":"304.8 m"}"#);
    assert!((num(&metric, "result.degree_arc.value") - 5.729_577_951_308_232).abs() < 1e-12);
    assert_eq!(metric["result"]["radius"]["unit"], "m");

    // Curve stationing: the circular curve's own PC and PT, from each pair of elements.
    for input in [
        serde_json::json!({"pi_station": "62+17.08", "radius": "1432.4 ft", "delta": "24 deg"}),
        serde_json::json!({"pi_station": "1+234.567", "radius": "300 m", "tangent": "80 m"}),
        serde_json::json!({"pi_station": "12+34.56", "degree": "4", "length": "600 ftUS"}),
    ] {
        let named = call("survey.curves.curve-stationing", &input.to_string());
        let general = call("survey.curves.circular-curve", &input.to_string());
        assert_eq!(named["ok"], true, "{named}");
        for key in [
            "pc_station",
            "pt_station",
            "tangent",
            "length",
            "radius",
            "delta",
        ] {
            assert_eq!(
                named["result"][key], general["result"][key],
                "{key} {input}"
            );
        }
    }
    let r = call(
        "survey.curves.curve-stationing",
        r#"{"pi_station":"62+17.08","radius":"1432.4 ft","delta":"24 deg"}"#,
    );
    assert_eq!(r["result"]["pc_station"], "59+12.61");
    assert_eq!(r["result"]["pt_station"], "65+12.62");
    // Without a PI there is nothing to station.
    assert_eq!(
        call(
            "survey.curves.curve-stationing",
            r#"{"radius":"500 ft","delta":"30 deg"}"#
        )["ok"],
        false
    );

    // Spiral-curve-spiral: the spiral tool's stations, and lengths that add up.
    let input = serde_json::json!({"spiral_length": "200 ft", "radius": "1000 ft", "delta": "40°00'00\"", "pi_station": "50+00"});
    let named = call("survey.curves.spiral-curve-spiral", &input.to_string());
    let general = call("survey.curves.spiral", &input.to_string());
    assert_eq!(named["ok"], true, "{named}");
    for key in [
        "ts_station",
        "sc_station",
        "cs_station",
        "st_station",
        "total_tangent",
        "arc_length",
        "spiral_angle",
    ] {
        assert_eq!(named["result"][key], general["result"][key], "{key}");
    }
    // The spec scenario: the arc is (Δ − 2θs) × R with θs = Ls / 2R.
    let arc = 1000.0 * (40f64.to_radians() - 0.2);
    assert!((num(&named, "result.arc_length.value") - arc).abs() < 1e-9);
    assert!((num(&named, "result.total_length.value") - (400.0 + arc)).abs() < 1e-9);
    assert!(named["result"]["layout"].is_null() && named["result"]["rows"].is_null());

    // Vertical curve turning point: the spec scenario, and the vertical curve's own answer.
    let input = serde_json::json!({"g1": 2, "g2": -3, "length": "600 ft", "pvi_station": "10+00", "pvi_elevation": "100 ft"});
    let named = call(
        "survey.curves.vertical-curve-turning-point",
        &input.to_string(),
    );
    let general = call("survey.curves.vertical-curve", &input.to_string());
    for key in ["turning_station", "turning_elevation", "curve_type", "k"] {
        assert_eq!(named["result"][key], general["result"][key], "{key}");
    }
    assert_eq!(named["result"]["turning_station"], "9+40.00");
    assert!((num(&named, "result.turning_elevation.value") - 96.4).abs() < 1e-9);
    assert!((num(&named, "result.distance_from_pvc.value") - 240.0).abs() < 1e-9);
    // Same-sign grades: the vertical curve explains, the focused operation refuses.
    let mut same = input.clone();
    same["g2"] = serde_json::json!(4);
    assert_eq!(
        call("survey.curves.vertical-curve", &same.to_string())["ok"],
        true
    );
    let refused = call(
        "survey.curves.vertical-curve-turning-point",
        &same.to_string(),
    );
    assert_eq!(refused["error"]["code"], "OUT_OF_DOMAIN");

    // Vertical curve elevations: ends on the PVC and PVT, passes through the high point.
    let table = call(
        "survey.curves.vertical-curve-elevations",
        &input.to_string(),
    );
    assert_eq!(table["ok"], true, "{table}");
    let rows = table["result"]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 7); // 7+00 to 13+00 at full stations
    assert_eq!(rows[0]["station"], general["result"]["pvc_station"]);
    assert_eq!(rows[0]["elevation"], general["result"]["pvc_elevation"]);
    assert_eq!(rows[6]["station"], general["result"]["pvt_station"]);
    assert!(
        (rows[6]["elevation"]["value"].as_f64().unwrap()
            - num(&general, "result.pvt_elevation.value"))
        .abs()
            < 1e-9
    );
    assert_eq!(rows[0]["grade"], 2.0);
    assert!((rows[6]["grade"].as_f64().unwrap() + 3.0).abs() < 1e-12);
    // The PVI station lies below the PVI by L × |A| / 800.
    assert!(
        (rows[3]["elevation"]["value"].as_f64().unwrap() - (100.0 - 600.0 * 5.0 / 800.0)).abs()
            < 1e-9
    );
    let mut one = input.clone();
    one["station"] = serde_json::json!("9+40");
    let at = call("survey.curves.vertical-curve-elevations", &one.to_string());
    let row = &at["result"]["rows"][0];
    assert!((row["elevation"]["value"].as_f64().unwrap() - 96.4).abs() < 1e-9);
    assert!(row["grade"].as_f64().unwrap().abs() < 1e-12);
    // An odd interval still starts on the PVC, lands on even stations, and ends on the PVT.
    let mut odd = input.clone();
    odd["pvi_station"] = serde_json::json!("10+12.50");
    odd["interval"] = serde_json::json!("50 ft");
    let rows = call("survey.curves.vertical-curve-elevations", &odd.to_string())["result"]["rows"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(rows[0]["station"], "7+12.50");
    assert_eq!(rows[1]["station"], "7+50.00");
    assert_eq!(rows.last().unwrap()["station"], "13+12.50");
    assert_eq!(rows.len(), 14);
}

#[test]
fn named_direction_and_station_modes_match_their_general_tools() {
    let direction_input = serde_json::json!({
        "direction": "N 10°15'30\" W",
        "reference": "assumed"
    });
    let named = call("survey.cogo.azimuth-bearing", &direction_input.to_string());
    let general = call("survey.cogo.direction-parse", &direction_input.to_string());
    assert_eq!(named["result"], general["result"]);
    assert_eq!(named["display"], general["display"]);
    assert_eq!(named["meta"]["warnings"], general["meta"]["warnings"]);

    let cases = [
        (
            "survey.cogo.perpendicular-foot",
            serde_json::json!({
                "start_northing": "0 ft", "start_easting": "0 ft", "start_station": "0+00",
                "direction": "0", "northing": "50 ft", "easting": "-20 ft"
            }),
        ),
        (
            "survey.cogo.point-from-station-offset",
            serde_json::json!({
                "start_northing": "5000 ft", "start_easting": "5000 ft", "start_station": "10+00",
                "direction": "60", "station": "13+50", "offset": "-12.5 ft"
            }),
        ),
    ];
    for (tool, input) in cases {
        let named = call(tool, &input.to_string());
        let general = call("survey.cogo.station-offset", &input.to_string());
        assert_eq!(named["result"], general["result"], "{tool}");
        assert_eq!(named["display"], general["display"], "{tool}");
        assert_eq!(
            named["meta"]["warnings"], general["meta"]["warnings"],
            "{tool}"
        );
    }
}

#[test]
fn deflection_angle_conventions() {
    let right = call(
        "survey.cogo.deflection-angle",
        r#"{"backsight_direction":"S 30 W","foresight_direction":"N 80 E","reference":"grid"}"#,
    );
    assert_eq!(num(&right, "result.forward_tangent_azimuth.value"), 30.0);
    assert_eq!(num(&right, "result.deflection.value"), 50.0);
    assert_eq!(num(&right, "result.signed_deflection.value"), 50.0);
    assert_eq!(right["result"]["turn"], "right");
    assert_eq!(right["result"]["reference"], "grid");

    let left = call(
        "survey.cogo.deflection-angle",
        r#"{"backsight_direction":"N 10 E","foresight_direction":"S 40 E"}"#,
    );
    assert_eq!(num(&left, "result.deflection.value"), 50.0);
    assert_eq!(num(&left, "result.signed_deflection.value"), -50.0);
    assert_eq!(left["result"]["turn"], "left");

    let forced = call(
        "survey.cogo.deflection-angle",
        r#"{"backsight_direction":"S 30 W","foresight_direction":"N 80 E","turn_convention":"left"}"#,
    );
    assert_eq!(num(&forced, "result.deflection.value"), 310.0);
    assert_eq!(num(&forced, "result.signed_deflection.value"), -310.0);
    assert_eq!(forced["result"]["turn"], "left");

    let invalid = call(
        "survey.cogo.deflection-angle",
        r#"{"backsight_direction":"N 95 E","foresight_direction":"N 10 E"}"#,
    );
    assert_eq!(invalid["error"]["code"], "INVALID_INPUT");
    assert_eq!(invalid["error"]["field"], "/backsight_direction");
}

#[test]
fn radial_sideshots_share_the_forward_result() {
    let radial = call(
        "survey.cogo.radial-sideshots",
        r#"{"station_northing":"5000 ft","station_easting":"10000 ft","shots":[{"point":"A","direction":"N 35-15-20 E","distance":"185.42 ft"},{"point":"B","direction":"S 70 E","distance":"240 ft"},{"point":"C","direction":"270","distance":"95.5 ft"}]}"#,
    );
    assert_eq!(num(&radial, "result.count"), 3.0);
    for (i, (direction, distance)) in [("N 35-15-20 E", 185.42), ("S 70 E", 240.0), ("270", 95.5)]
        .iter()
        .enumerate()
    {
        let forward = call(
            "survey.cogo.forward",
            &format!(
                r#"{{"northing":"5000 ft","easting":"10000 ft","direction":"{direction}","distance":"{distance} ft"}}"#
            ),
        );
        assert_eq!(
            radial["result"]["points"][i]["northing"]["value"]
                .as_f64()
                .unwrap(),
            num(&forward, "result.northing.value")
        );
        assert_eq!(
            radial["result"]["points"][i]["easting"]["value"]
                .as_f64()
                .unwrap(),
            num(&forward, "result.easting.value")
        );
    }

    let bad = call(
        "survey.cogo.radial-sideshots",
        r#"{"station_northing":"0 ft","station_easting":"0 ft","shots":[{"point":"A","direction":"N 0 E","distance":"0 ft"}]}"#,
    );
    assert_eq!(bad["error"]["code"], "INVALID_INPUT");
    assert_eq!(bad["error"]["field"], "/shots/0/distance");
}

#[test]
fn mixed_feet_are_refused() {
    let r = call(
        "survey.cogo.forward",
        r#"{"northing":"0 ft","easting":"0 ftUS","direction":"90","distance":"1 ft"}"#,
    );
    assert_eq!(r["error"]["code"], "UNIT_MISMATCH");
    let us = call(
        "survey.cogo.forward",
        r#"{"northing":"0 ftUS","easting":"0 ftUS","direction":"90","distance":"1 ftUS"}"#,
    );
    assert!(codes(&us).contains(&"LEGACY_UNIT".to_owned()));
    assert_eq!(us["result"]["easting"]["unit"], "ftUS");
}

#[test]
fn traverse_closure_scenario() {
    let r = call(
        "survey.cogo.traverse-closure",
        r#"{"courses":[{"direction":"0","distance":300.00},{"direction":"90","distance":400.02},{"direction":"180","distance":299.95},{"direction":"270.01","distance":400.00}]}"#,
    );
    assert!((num(&r, "result.misclosure.value") - 0.1215).abs() < 5e-5);
    assert_eq!(r["display"]["total_length"], "1,399.97 ft");
    assert_eq!(r["result"]["precision"], "1:11,525");
    // The compass-adjusted traverse closes exactly and every course is reported.
    let adj = r["result"]["adjusted"].as_array().unwrap();
    assert_eq!(adj.len(), 5);
    let (mut sl, mut sd) = (0.0, 0.0);
    for w in adj.windows(2) {
        sl += num(&w[1], "northing.value") - num(&w[0], "northing.value");
        sd += num(&w[1], "easting.value") - num(&w[0], "easting.value");
    }
    assert!(sl.abs() < 1e-9 && sd.abs() < 1e-9);
    let t = call(
        "survey.cogo.traverse-closure",
        r#"{"courses":[{"direction":"0","distance":300},{"direction":"90","distance":400.02},{"direction":"180","distance":299.95},{"direction":"270.01","distance":400}],"adjustment":"transit"}"#,
    );
    assert!(
        t["result"]["note"]
            .as_str()
            .unwrap()
            .contains("orientation")
    );
    let none = call(
        "survey.cogo.traverse-closure",
        r#"{"courses":[{"direction":"0","distance":1},{"direction":"180","distance":1}],"adjustment":"none"}"#,
    );
    assert!(none["result"].get("adjusted").is_none());
    assert!(none["result"].get("precision_ratio").is_none());
}

#[test]
fn named_traverse_adjustments_match_the_general_tool() {
    let courses = serde_json::json!([
        {"direction": "15", "distance": 120},
        {"direction": "75", "distance": 95},
        {"direction": "130", "distance": 140},
        {"direction": "200", "distance": 180},
        {"direction": "260", "distance": 90},
        {"direction": "320", "distance": 110}
    ]);
    for (tool, method) in [
        ("survey.cogo.bowditch", "compass"),
        ("survey.cogo.transit", "transit"),
        ("survey.cogo.crandall", "crandall"),
    ] {
        let named = call(
            tool,
            &serde_json::json!({
                "courses": courses,
                "start_northing": "1200 ft",
                "start_easting": "3400 ft"
            })
            .to_string(),
        );
        let general = call(
            "survey.cogo.traverse-closure",
            &serde_json::json!({
                "courses": courses,
                "adjustment": method,
                "start_northing": "1200 ft",
                "start_easting": "3400 ft"
            })
            .to_string(),
        );
        assert_eq!(named["result"], general["result"], "{tool}");
        assert_eq!(named["display"], general["display"], "{tool}");
        let named_warnings: Vec<&Value> = named["meta"]["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|warning| warning["code"] != "EXPERIMENTAL_TOOL")
            .collect();
        let general_warnings: Vec<&Value> = general["meta"]["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .collect();
        assert_eq!(named_warnings, general_warnings, "{tool}");
    }
}

#[test]
fn traverse_row_errors_point_at_the_row() {
    let r = call(
        "survey.cogo.traverse-closure",
        r#"{"courses":[{"direction":"0","distance":1},{"direction":"X","distance":1}]}"#,
    );
    assert_eq!(r["error"]["code"], "INVALID_INPUT");
    assert_eq!(r["error"]["field"], "/courses/1/direction", "{r}");
    let r = call(
        "survey.cogo.traverse-closure",
        r#"{"courses":[{"direction":"0","distance":1},{"direction":"90","distance":"1 ftUS"}]}"#,
    );
    assert_eq!(r["error"]["code"], "UNIT_MISMATCH", "{r}");
    let r = call(
        "survey.cogo.traverse-closure",
        r#"{"courses":[{"direction":"0","distance":1}]}"#,
    );
    assert_eq!(r["error"]["code"], "INVALID_INPUT");
    let r = call(
        "survey.cogo.traverse-closure",
        r#"{"courses":[{"direction":"0","distance":1},{"direction":"0","distance":1,"extra":1}]}"#,
    );
    assert_eq!(r["ok"], false);
}

#[test]
fn area_scenarios() {
    let r = call(
        "survey.cogo.area-by-coordinates",
        r#"{"points":[{"northing":0,"easting":0},{"northing":0,"easting":100},{"northing":50,"easting":100},{"northing":50,"easting":0}]}"#,
    );
    assert_eq!(r["display"]["area"], "5,000 ft²");
    assert_eq!(r["display"]["acres"], "0.1148 ac");
    // Large state plane coordinates keep their digits.
    let big = call(
        "survey.cogo.area-by-coordinates",
        r#"{"points":[{"northing":712345678.0,"easting":2345678.0},{"northing":712345678.0,"easting":2345778.0},{"northing":712345728.0,"easting":2345778.0},{"northing":712345728.0,"easting":2345678.0}]}"#,
    );
    assert_eq!(num(&big, "result.area.value"), 5000.0);
    let line = call(
        "survey.cogo.area-by-coordinates",
        r#"{"points":[{"northing":0,"easting":0},{"northing":1,"easting":1},{"northing":2,"easting":2}]}"#,
    );
    assert_eq!(line["error"]["code"], "DEGENERATE_GEOMETRY");
}

#[test]
fn circular_curve_any_two() {
    let base = call(
        "survey.curves.circular-curve",
        r#"{"radius":500,"delta":"30 deg"}"#,
    );
    for (k, want) in [
        ("tangent", 133.975),
        ("length", 261.799),
        ("chord", 258.819),
        ("external", 17.638),
        ("middle_ordinate", 17.037),
    ] {
        assert!(
            (num(&base, &format!("result.{k}.value")) - want).abs() < 5e-4,
            "{k}"
        );
    }
    for pair in [
        ("tangent", "chord"),
        ("length", "external"),
        ("chord", "middle_ordinate"),
        ("radius", "tangent"),
        ("delta", "external"),
    ] {
        let v = |k: &str| {
            if k == "delta" {
                "\"30 deg\"".to_owned()
            } else {
                num(&base, &format!("result.{k}.value")).to_string()
            }
        };
        let r = call(
            "survey.curves.circular-curve",
            &format!(
                r#"{{"{}":{},"{}":{}}}"#,
                pair.0,
                v(pair.0),
                pair.1,
                v(pair.1)
            ),
        );
        assert!(
            (num(&r, "result.radius.value") - 500.0).abs() < 1e-7,
            "{pair:?}: {r}"
        );
    }
    let three = call(
        "survey.curves.circular-curve",
        r#"{"radius":500,"delta":"30 deg","tangent":1}"#,
    );
    assert_eq!(three["error"]["code"], "INVALID_INPUT");
    let impossible = call(
        "survey.curves.circular-curve",
        r#"{"radius":100,"chord":300}"#,
    );
    assert_eq!(impossible["error"]["code"], "INVALID_INPUT");
    let d = call(
        "survey.curves.circular-curve",
        r#"{"degree":"4 deg","delta":"20 deg"}"#,
    );
    assert!((num(&d, "result.radius.value") - 1432.39).abs() < 0.01);
}

#[test]
fn vertical_curve_scenario() {
    let r = call(
        "survey.curves.vertical-curve",
        r#"{"g1":2,"g2":-3,"length":"600 ft","pvi_station":"10+00","pvi_elevation":"100 ft"}"#,
    );
    assert_eq!(r["result"]["pvc_station"], "7+00.00");
    assert_eq!(r["display"]["pvc_elevation"], "94 ft");
    assert_eq!(r["result"]["turning_station"], "9+40.00");
    assert_eq!(r["display"]["turning_elevation"], "96.4 ft");
    assert_eq!(num(&r, "result.k"), 120.0);
    let by_k = call(
        "survey.curves.vertical-curve",
        r#"{"g1":2,"g2":-3,"k":120,"pvi_station":"10+00","pvi_elevation":"100 ft"}"#,
    );
    assert_eq!(by_k["result"]["pvt_station"], "13+00.00");
    let same_sign = call(
        "survey.curves.vertical-curve",
        r#"{"g1":3,"g2":1,"length":"200 ft","pvi_station":"15+00","pvi_elevation":"75 ft"}"#,
    );
    assert!(same_sign["result"].get("turning_station").is_none());
    assert!(same_sign["result"]["turning_note"].is_string());
    let s = same_sign["summary"].as_str().unwrap();
    assert!(!s.contains("point is at"), "{s}");
    let metric = call(
        "survey.curves.vertical-curve",
        r#"{"g1":-1,"g2":1,"length":"200 m","pvi_station":"1+500","pvi_elevation":"50 m"}"#,
    );
    assert_eq!(metric["result"]["pvc_station"], "1+400.000", "{metric}");
}

#[test]
fn earthwork_scenarios() {
    let r = call(
        "survey.earthwork.average-end-area",
        r#"{"area1":"120 ft2","area2":"180 ft2","length":"100 ft"}"#,
    );
    assert_eq!(r["display"]["volume"], "555.56 yd³");
    assert_eq!(r["display"]["volume_ft3"], "15,000 ft³");
    let p = call(
        "survey.earthwork.prismoidal",
        r#"{"area1":"120 ft2","area2":"180 ft2","area_middle":"148 ft2","length":"100 ft"}"#,
    );
    assert_eq!(p["display"]["volume_ft3"], "14,866.7 ft³");
    let s = call(
        "survey.earthwork.shrink-swell",
        r#"{"bank_volume":"1000 yd3","swell":25,"truck_capacity":"12 yd3"}"#,
    );
    assert_eq!(num(&s, "result.loads"), 105.0);
    let exact = call(
        "survey.earthwork.shrink-swell",
        r#"{"bank_volume":"960 yd3","swell":25,"truck_capacity":"12 yd3"}"#,
    );
    assert_eq!(
        num(&exact, "result.loads"),
        100.0,
        "exact multiples do not round up"
    );
}

#[test]
fn combined_factor() {
    let r = call(
        "survey.reduction.combined-factor",
        r#"{"grid_scale":0.99991,"elevation":"1530 ft","geoid_height":"-30 ft","grid_distance":"999.838 ft"}"#,
    );
    let ef = 6_372_000.0 / (6_372_000.0 + 1500.0 * 0.3048);
    assert!((num(&r, "result.elevation_factor") - ef).abs() < 1e-15);
    assert!((num(&r, "result.ground_distance.value") - 999.838 / (0.99991 * ef)).abs() < 1e-9);
    assert!(codes(&r).iter().all(|c| c != "ORTHOMETRIC_AS_ELLIPSOIDAL"));
    let both = call(
        "survey.reduction.combined-factor",
        r#"{"grid_scale":1,"ellipsoid_height":0,"grid_distance":1,"ground_distance":1}"#,
    );
    assert_eq!(both["error"]["code"], "INVALID_INPUT");
}

#[test]
fn traverse_closure_invariants() {
    // Misclosure is the length of (Σlat, Σdep); rotating every course by the
    // same angle leaves misclosure and precision unchanged; reversing the
    // loop negates the sums; and after a compass or transit adjustment the
    // loop returns exactly to its start.
    let loops: [&[(f64, f64)]; 3] = [
        &[
            (0.0, 300.0),
            (90.0, 400.02),
            (180.0, 299.95),
            (270.01, 400.0),
        ],
        &[
            (12.25, 250.0),
            (95.5, 310.2),
            (170.75, 260.4),
            (281.0, 290.1),
        ],
        &[
            (60.0, 45.5),
            (140.0, 60.2),
            (230.0, 70.1),
            (320.0, 50.3),
            (355.0, 20.0),
        ],
    ];
    let run = |courses: &[(f64, f64)], method: &str| {
        let c: Vec<Value> = courses
            .iter()
            .map(|(a, d)| serde_json::json!({"direction": a.rem_euclid(360.0).to_string(), "distance": d}))
            .collect();
        call(
            "survey.cogo.traverse-closure",
            &serde_json::json!({"courses": c, "adjustment": method}).to_string(),
        )
    };
    for courses in loops {
        let base = run(courses, "compass");
        let (sl, sd) = (
            num(&base, "result.sum_latitudes.value"),
            num(&base, "result.sum_departures.value"),
        );
        let mis = num(&base, "result.misclosure.value");
        assert!((mis - sl.hypot(sd)).abs() < 1e-12);
        for rot in [17.0, 123.5, 271.25] {
            let turned: Vec<(f64, f64)> = courses.iter().map(|(a, d)| (a + rot, *d)).collect();
            let r = run(&turned, "compass");
            assert!((num(&r, "result.misclosure.value") - mis).abs() < 1e-9);
            assert!(
                (num(&r, "result.precision_ratio") - num(&base, "result.precision_ratio")).abs()
                    < 1e-6
            );
        }
        let back: Vec<(f64, f64)> = courses.iter().rev().map(|(a, d)| (a + 180.0, *d)).collect();
        let r = run(&back, "compass");
        assert!((num(&r, "result.sum_latitudes.value") + sl).abs() < 1e-9);
        assert!((num(&r, "result.sum_departures.value") + sd).abs() < 1e-9);
        for method in ["compass", "transit", "crandall"] {
            let r = run(courses, method);
            let pts = r["result"]["adjusted"].as_array().unwrap();
            let (first, last) = (&pts[0], &pts[pts.len() - 1]);
            for k in ["northing", "easting"] {
                let gap = first[k]["value"].as_f64().unwrap() - last[k]["value"].as_f64().unwrap();
                assert!(gap.abs() < 1e-9, "{method} {k} gap {gap}");
            }
        }
    }
}

#[test]
fn crandall_holds_every_bearing() {
    // The Crandall rule corrects distances only, so each adjusted course keeps
    // its measured azimuth; and unlike the transit rule it does not depend on
    // the grid's orientation, so turning the loop leaves the distances alone.
    let courses = [
        (15.0, 120.0),
        (75.0, 95.0),
        (130.0, 140.0),
        (200.0, 180.0),
        (260.0, 90.0),
        (320.0, 110.0),
    ];
    let run = |rot: f64| {
        let c: Vec<Value> = courses
            .iter()
            .map(|(a, d)| serde_json::json!({"direction": (a + rot).rem_euclid(360.0).to_string(), "distance": d}))
            .collect();
        call(
            "survey.cogo.traverse-closure",
            &serde_json::json!({"courses": c, "adjustment": "crandall"}).to_string(),
        )
    };
    let base = run(0.0);
    assert!(
        base["result"]["note"].as_str().unwrap().contains("bearing"),
        "{base}"
    );
    let pts = base["result"]["adjusted"].as_array().unwrap();
    for (i, (az, _)) in courses.iter().enumerate() {
        let dn = pts[i + 1]["northing"]["value"].as_f64().unwrap()
            - pts[i]["northing"]["value"].as_f64().unwrap();
        let de = pts[i + 1]["easting"]["value"].as_f64().unwrap()
            - pts[i]["easting"]["value"].as_f64().unwrap();
        let got = de.atan2(dn).to_degrees().rem_euclid(360.0);
        assert!((got - az).abs() < 1e-9, "course {i}: {got} vs {az}");
    }
    let turned = run(37.5);
    let d = |r: &Value, i: usize| {
        r["result"]["adjusted"][i]["distance"]["value"]
            .as_f64()
            .unwrap()
    };
    for i in 1..=courses.len() {
        assert!((d(&base, i) - d(&turned, i)).abs() < 1e-9);
    }
}

#[test]
fn combined_factor_invariants() {
    // Combined = grid scale × elevation factor; grid and ground distances
    // invert each other; h = H + N gives the same factor as the ellipsoid
    // height; and the elevation factor falls as the height rises.
    let mut prev = f64::INFINITY;
    for h in [-100.0, 0.0, 250.0, 1000.0, 3000.0, 4500.0] {
        let e = call(
            "survey.reduction.combined-factor",
            &format!(
                r#"{{"grid_scale":0.99993,"ellipsoid_height":"{h} m","ground_distance":"1000 m"}}"#
            ),
        );
        let ef = num(&e, "result.elevation_factor");
        assert!(ef < prev);
        prev = ef;
        let cf = num(&e, "result.combined_factor");
        assert!((cf - 0.99993 * ef).abs() < 1e-15);
        let grid = num(&e, "result.grid_distance.value");
        let back = call(
            "survey.reduction.combined-factor",
            &format!(
                r#"{{"grid_scale":0.99993,"ellipsoid_height":"{h} m","grid_distance":"{grid} m"}}"#
            ),
        );
        assert!((num(&back, "result.ground_distance.value") - 1000.0).abs() < 1e-9);
        let split = call(
            "survey.reduction.combined-factor",
            &format!(
                r#"{{"grid_scale":0.99993,"elevation":"{} m","geoid_height":"-30 m","ground_distance":"1000 m"}}"#,
                h + 30.0
            ),
        );
        assert!((num(&split, "result.combined_factor") - cf).abs() < 1e-15);
    }
}

#[test]
fn circular_curve_invariants() {
    // Any two independent elements give back the same curve, and the arc and
    // chord degrees of curve fix R = 5,729.578/D and R = 50/sin(D/2).
    let base = call(
        "survey.curves.circular-curve",
        r#"{"radius":"850 ft","delta":"37.5 deg"}"#,
    );
    let names = ["tangent", "length", "chord", "external", "middle_ordinate"];
    for (i, a) in names.iter().enumerate() {
        for b in &names[i + 1..] {
            let r = call(
                "survey.curves.circular-curve",
                &serde_json::json!({*a: format!("{} ft", num(&base, &format!("result.{a}.value"))),
                                    *b: format!("{} ft", num(&base, &format!("result.{b}.value")))})
                .to_string(),
            );
            assert!(
                (num(&r, "result.radius.value") - 850.0).abs() < 1e-6,
                "{a}+{b}: {r}"
            );
            assert!(
                (num(&r, "result.delta.value") - 37.5).abs() < 1e-9,
                "{a}+{b}"
            );
            // T and M fit a second, sharper curve too (their ratio has a minimum near Δ = 104°).
            let ambiguous = codes(&r).contains(&"AMBIGUOUS_INPUT".to_owned());
            assert_eq!(
                ambiguous,
                (*a, *b) == ("tangent", "middle_ordinate"),
                "{a}+{b}"
            );
        }
    }
    for d in [1.0, 4.5, 15.0, 30.0] {
        let arc = call(
            "survey.curves.circular-curve",
            &format!(r#"{{"degree":"{d} deg","delta":"20 deg"}}"#),
        );
        assert!(
            (num(&arc, "result.radius.value") - 18_000.0 / std::f64::consts::PI / d).abs() < 1e-9
        );
        let chord = call(
            "survey.curves.circular-curve",
            &format!(r#"{{"degree_chord":"{d} deg","delta":"20 deg"}}"#),
        );
        let r = num(&chord, "result.radius.value");
        assert!((r - 50.0 / (d / 2.0).to_radians().sin()).abs() < 1e-9);
        assert!((num(&chord, "result.degree_chord.value") - d).abs() < 1e-9);
    }
}

#[test]
fn degree_of_curve_invariants() {
    let doc = |body: &str| call("survey.curves.degree-of-curve", body);
    for r in [
        50.0001,
        100.0,
        850.0,
        1432.3944878270581,
        5729.578,
        9397.478,
    ] {
        // A radius gives both degrees of curve, and each degree alone returns
        // the radius it came from.
        let from_r = doc(&format!(r#"{{"radius":"{r} ft"}}"#));
        let da = num(&from_r, "result.degree_arc.value");
        let dc = num(&from_r, "result.degree_chord.value");
        let back_a = doc(&format!(r#"{{"degree":"{da} deg"}}"#));
        let back_c = doc(&format!(r#"{{"degree_chord":"{dc} deg"}}"#));
        assert!(
            (num(&back_a, "result.radius.value") - r).abs() < 1e-6,
            "arc round trip at R={r}"
        );
        assert!(
            (num(&back_c, "result.radius.value") - r).abs() < 1e-6,
            "chord round trip at R={r}"
        );
        // For the same radius the chord definition names a larger degree than
        // the arc definition (the chord is shorter than the arc it subtends).
        assert!(dc > da, "chord degree not larger than arc degree at R={r}");
    }
}

#[test]
fn curve_stationing_invariants() {
    let st = |body: &str| call("survey.curves.curve-stationing", body);
    // A station string like "62+17.08" as a single number of feet or metres.
    let value = |s: &str| -> f64 {
        let s = s.replace('+', "");
        s.parse::<f64>().unwrap()
    };
    for (pi, extra) in [
        ("62+17.08", r#""radius":"1432.4 ft","delta":"24 deg""#),
        ("10+120", r#""radius":"300 m","delta":"40 deg""#),
        ("5+00", r#""radius":"500 ft","tangent":"150 ft""#),
    ] {
        let r = st(&format!(r#"{{"pi_station":"{pi}",{extra}}}"#));
        let pc = value(r["result"]["pc_station"].as_str().unwrap());
        let pt = value(r["result"]["pt_station"].as_str().unwrap());
        let t = num(&r, "result.tangent.value");
        let l = num(&r, "result.length.value");
        // Stationing runs along the arc: PT − PC = L, and PI − PC = T, each
        // within the 0.01 ft the station strings are rounded to.
        assert!((pt - pc - l).abs() < 0.02, "PT-PC=L at {pi}");
        assert!((value(pi) - pc - t).abs() < 0.02, "PI-PC=T at {pi}");
    }
    // The same curve entered by a different pair of elements has the same PC
    // and PT (radius+delta vs radius+tangent for Δ = 24°, R = 1432.4 ft).
    let a = st(r#"{"pi_station":"62+17.08","radius":"1432.4 ft","delta":"24 deg"}"#);
    let t = num(&a, "result.tangent.value");
    let b = st(&format!(
        r#"{{"pi_station":"62+17.08","radius":"1432.4 ft","tangent":"{t} ft"}}"#
    ));
    assert_eq!(a["result"]["pc_station"], b["result"]["pc_station"]);
    assert_eq!(a["result"]["pt_station"], b["result"]["pt_station"]);
}

#[test]
fn vertical_curve_turning_point_invariants() {
    let tp = |g1: f64, g2: f64, l: f64| {
        call(
            "survey.curves.vertical-curve-turning-point",
            &serde_json::json!({"g1": g1, "g2": g2, "length": format!("{l} ft"),
                                "pvi_station": "50+00", "pvi_elevation": "500 ft"})
            .to_string(),
        )
    };
    for (g1, g2, l) in [
        (-4.0, 2.0, 500.0),
        (2.0, -3.0, 600.0),
        (-1.5, 1.5, 400.0),
        (3.0, -1.0, 800.0),
    ] {
        let r = tp(g1, g2, l);
        let x = num(&r, "result.distance_from_pvc.value");
        let k = num(&r, "result.k");
        // The grade is zero at the turning point: g1 + (g2 − g1)·x/L = 0.
        assert!(
            (g1 + (g2 - g1) * x / l).abs() < 1e-9,
            "grade not zero at the turning point for {g1}/{g2}"
        );
        // Its distance from the PVC is K·|g1| and lies on the curve.
        assert!((x - k * g1.abs()).abs() < 1e-6, "x != K*|g1| for {g1}/{g2}");
        assert!(
            (0.0..=l).contains(&x),
            "turning point off the curve for {g1}/{g2}"
        );
        // A rise-then-fall grade is a crest (high); a fall-then-rise is a sag (low).
        let want = if g1 > 0.0 { "high" } else { "low" };
        assert_eq!(
            r["result"]["curve_type"].as_str().unwrap(),
            want,
            "{g1}/{g2}"
        );
    }
}

#[test]
fn vertical_curve_invariants() {
    // The PVC and PVT lie on the tangents, K = L/|g2 − g1|, and the turning
    // point is the extreme: the curve is no lower (sag) or higher (crest) at
    // points 1 ft either side.
    for (g1, g2, l) in [
        (-1.75, 2.25, 500.0),
        (3.0, -2.0, 800.0),
        (-4.0, 1.0, 350.0),
        (0.5, -3.5, 1000.0),
    ] {
        let r = call(
            "survey.curves.vertical-curve",
            &format!(
                r#"{{"g1":{g1},"g2":{g2},"length":"{l} ft","pvi_station":"50+00","pvi_elevation":"1000 ft"}}"#
            ),
        );
        let pvc = num(&r, "result.pvc_elevation.value");
        assert!((pvc - (1000.0 - g1 / 100.0 * l / 2.0)).abs() < 1e-9);
        assert!(
            (num(&r, "result.pvt_elevation.value") - (1000.0 + g2 / 100.0 * l / 2.0)).abs() < 1e-9
        );
        assert!((num(&r, "result.k") - l / (g2 - g1).abs()).abs() < 1e-9);
        let y = |x: f64| pvc + g1 / 100.0 * x + (g2 - g1) / 100.0 * x * x / (2.0 * l);
        let x = -g1 / 100.0 * l / ((g2 - g1) / 100.0);
        let turn = num(&r, "result.turning_elevation.value");
        assert!((turn - y(x)).abs() < 1e-9);
        let sag = g2 > g1;
        for dx in [-1.0, 1.0] {
            assert!(if sag {
                y(x + dx) >= turn
            } else {
                y(x + dx) <= turn
            });
        }
    }
}

#[test]
fn unequal_vertical_curve_invariants() {
    // Equal halves are the same parabola as the stable equal-tangent tool.
    let equal = call(
        "survey.curves.vertical-curve",
        r#"{"g1":2,"g2":-3,"length":"600 ft","pvi_station":"10+00","pvi_elevation":"100 ft"}"#,
    );
    let unequal = call(
        "survey.curves.unequal-vertical-curve",
        r#"{"g1":2,"g2":-3,"length1":"300 ft","length2":"300 ft","pvi_station":"10+00","pvi_elevation":"100 ft"}"#,
    );
    for field in ["pvc_elevation", "pvt_elevation", "turning_elevation"] {
        assert!(
            (num(&equal, &format!("result.{field}.value"))
                - num(&unequal, &format!("result.{field}.value")))
            .abs()
                < 1e-12,
            "{field}"
        );
    }
    assert_eq!(
        equal["result"]["pvc_station"],
        unequal["result"]["pvc_station"]
    );
    assert_eq!(
        equal["result"]["pvt_station"],
        unequal["result"]["pvt_station"]
    );
    assert_eq!(
        equal["result"]["turning_station"],
        unequal["result"]["turning_station"]
    );
    assert!((num(&unequal, "result.cvc_elevation.value") - 96.25).abs() < 1e-12);

    // The two parabolas meet continuously under the PVI with the same grade.
    let at = |station: &str| {
        let r = call(
            "survey.curves.unequal-vertical-curve",
            &format!(
                r#"{{"g1":2,"g2":-3,"length1":"200 ft","length2":"400 ft","pvi_station":"10+00","pvi_elevation":"100 ft","at_station":"{station}"}}"#
            ),
        );
        num(&r, "result.elevation.value")
    };
    let (left, center, right) = (at("9+99.99"), at("10+00"), at("10+00.01"));
    let (left_grade, right_grade) = ((center - left) / 0.01, (right - center) / 0.01);
    assert!((left_grade - right_grade).abs() < 2e-6);

    // Pearson's problem 25.15 prints the 61+00 elevation as 3306.219 ft.
    let published = call(
        "survey.curves.unequal-vertical-curve",
        r#"{"g1":2.5,"g2":-1.25,"length1":"600 ft","length2":"400 ft","pvi_station":"60+00","pvi_elevation":"3310 ft","at_station":"61+00"}"#,
    );
    assert!((num(&published, "result.elevation.value") - 3306.219).abs() < 0.001);
}

#[test]
fn area_by_coordinates_invariants() {
    // Area does not change when the parcel is shifted, rotated, or started
    // at another corner; reversing the order flips only the orientation.
    let p = [
        (100.0, 100.0),
        (340.0, 180.0),
        (520.0, 90.0),
        (610.0, 400.0),
        (300.0, 520.0),
        (80.0, 330.0),
    ];
    let area = |pts: &[(f64, f64)]| {
        let v: Vec<Value> = pts
            .iter()
            .map(|(n, e)| serde_json::json!({"northing": n, "easting": e}))
            .collect();
        let r = call(
            "survey.cogo.area-by-coordinates",
            &serde_json::json!({"points": v}).to_string(),
        );
        (
            num(&r, "result.area.value"),
            r["result"]["orientation"].as_str().unwrap().to_owned(),
        )
    };
    let (a, o) = area(&p);
    let shifted: Vec<_> = p
        .iter()
        .map(|(n, e)| (n + 1_234_567.0, e - 765_432.0))
        .collect();
    assert!((area(&shifted).0 - a).abs() < 1e-6 * a);
    let (s, c) = (0.6_f64.sin(), 0.6_f64.cos());
    let turned: Vec<_> = p
        .iter()
        .map(|(n, e)| (n * c - e * s, n * s + e * c))
        .collect();
    assert!((area(&turned).0 - a).abs() < 1e-9 * a);
    let mut rolled = p.to_vec();
    rolled.rotate_left(2);
    assert_eq!(area(&rolled), (a, o.clone()));
    let mut rev = p.to_vec();
    rev.reverse();
    let (ra, ro) = area(&rev);
    assert!((ra - a).abs() < 1e-9 && ro != o);
}

#[test]
fn circular_curve_extreme_angles() {
    // Pairs other than R and Δ still solve at very flat and very sharp curves.
    for delta in [0.1, 0.2, 179.8, 179.9] {
        let base = call(
            "survey.curves.circular-curve",
            &format!(r#"{{"radius":"1000 ft","delta":"{delta} deg"}}"#),
        );
        let (t, c) = (
            num(&base, "result.tangent.value"),
            num(&base, "result.chord.value"),
        );
        let r = call(
            "survey.curves.circular-curve",
            &format!(r#"{{"tangent":"{t} ft","chord":"{c} ft"}}"#),
        );
        assert!(
            (num(&r, "result.delta.value") - delta).abs() < 1e-6,
            "{delta}: {r}"
        );
    }
}

#[test]
fn slope_reduction_scenarios() {
    // 500 m at 85°00'00": HD ≈ 498.097 m and VD ≈ 43.578 m.
    let r = call(
        "survey.reduction.slope",
        r#"{"slope_distance":"500 m","zenith":"85°00'00\""}"#,
    );
    assert_eq!(r["display"]["horizontal_distance"], "498.097 m", "{r}");
    assert_eq!(r["display"]["vertical_difference"], "43.578 m", "{r}");
    // Two faces: FL 85°00'10", FR 274°59'40" give a mean of 85°00'15" and an index error of -5".
    let r = call(
        "survey.reduction.slope",
        r#"{"slope_distance":"500 m","zenith":"85°00'10\"","zenith_face_right":"274°59'40\""}"#,
    );
    assert_eq!(r["result"]["mean_zenith_dms"], "85°00'15.0\"", "{r}");
    assert_eq!(r["display"]["index_error"], "-5″", "{r}");
    // A face-right reading entered as the zenith is caught, and says why.
    let r = call(
        "survey.reduction.slope",
        r#"{"slope_distance":"500 m","zenith":"274°59'40\""}"#,
    );
    assert!(
        r["error"]["message"]
            .as_str()
            .unwrap()
            .contains("face right"),
        "{r}"
    );
}

#[test]
fn slope_reduction_invariants() {
    let reduce = |distance: f64, zenith: f64| {
        call(
            "survey.reduction.slope",
            &format!(r#"{{"slope_distance":"{distance} m","zenith":"{zenith} deg"}}"#),
        )
    };

    // The horizontal and vertical components reconstruct the measured line,
    // and scaling the line scales both components.
    let base = reduce(100.0, 37.0);
    let (h, v) = (
        num(&base, "result.horizontal_distance.value"),
        num(&base, "result.vertical_difference.value"),
    );
    assert!((h.hypot(v) - 100.0).abs() < 1e-10);
    let scaled = reduce(250.0, 37.0);
    assert!((num(&scaled, "result.horizontal_distance.value") - 2.5 * h).abs() < 1e-10);
    assert!((num(&scaled, "result.vertical_difference.value") - 2.5 * v).abs() < 1e-10);

    // A vertical angle is the complement of a zenith angle.
    let vertical = call(
        "survey.reduction.slope",
        r#"{"slope_distance":"100 m","vertical_angle":"53 deg"}"#,
    );
    assert_eq!(vertical["result"], base["result"]);

    // Equal face readings cancel the index error; unequal readings expose it
    // while using their mean for the reduction.
    let faces = call(
        "survey.reduction.slope",
        r#"{"slope_distance":"100 m","zenith":"80 deg","zenith_face_right":"279 deg"}"#,
    );
    assert_eq!(num(&faces, "result.mean_zenith.value"), 80.5);
    assert_eq!(num(&faces, "result.index_error.value"), -1800.0);
    let mean = reduce(100.0, 80.5);
    assert_eq!(
        num(&faces, "result.horizontal_distance.value"),
        num(&mean, "result.horizontal_distance.value")
    );

    // Instrument height adds and target height subtracts one for one.
    let heights = call(
        "survey.reduction.slope",
        r#"{"slope_distance":"100 m","zenith":"90 deg","instrument_height":"2 m","target_height":"0.5 m"}"#,
    );
    assert!((num(&heights, "result.elevation_difference.value") - 1.5).abs() < 1e-12);

    // The optional long-sight correction turns on only beyond its threshold.
    let at_threshold = call(
        "survey.reduction.slope",
        r#"{"slope_distance":"150 m","zenith":"90 deg","curvature_beyond":"150 m"}"#,
    );
    assert!(at_threshold["result"].get("curvature_refraction").is_none());
    let beyond = call(
        "survey.reduction.slope",
        r#"{"slope_distance":"151 m","zenith":"90 deg","curvature_beyond":"150 m"}"#,
    );
    assert!(num(&beyond, "result.curvature_refraction.value") > 0.0);
}

#[test]
fn curvature_refraction_scenario() {
    // 1 km: 0.0675 m with k = 0.14 and 0.0683 m with k = 0.13, each labeled with its k.
    let a = call(
        "survey.reduction.curvature-refraction",
        r#"{"distance":"1000 m","refraction":0.14}"#,
    );
    let b = call(
        "survey.reduction.curvature-refraction",
        r#"{"distance":"1000 m","refraction":0.13}"#,
    );
    assert!(
        (a["result"]["correction"]["value"].as_f64().unwrap() - 0.0675).abs() < 5e-5,
        "{a}"
    );
    assert!(
        (b["result"]["correction"]["value"].as_f64().unwrap() - 0.0683).abs() < 5e-5,
        "{b}"
    );
    assert_eq!(a["result"]["coefficient_label"], "0.0675 m/km² (k = 0.14)");
    assert_eq!(b["result"]["coefficient_label"], "0.0683 m/km² (k = 0.13)");
}

#[test]
fn edm_trimble_access_presets_match_the_published_formula_and_constants() {
    for (preset, j, n) in [
        ("trimble-access-5600", 274.41, 79.39),
        ("trimble-access-3300-3600", 278.77, 80.653),
        ("trimble-access-tts300-500", 270.0, 79.167),
    ] {
        let result = call(
            "survey.reduction.edm-correction",
            &format!(
                r#"{{"distance":"1000 m","temperature":"20 degC","pressure":"1013.25 hPa","instrument_formula":"{preset}"}}"#
            ),
        );
        let expected = j - n * 1013.25 / (273.16 + 20.0);
        assert_eq!(result["ok"], true, "{result}");
        assert!((num(&result, "result.ppm") - expected).abs() < 1e-12);
        assert!(
            (num(&result, "result.atmospheric_correction.value") - expected * 0.001).abs() < 1e-12
        );
    }
}

#[test]
fn edm_instrument_formula_does_not_silently_mix_with_other_methods() {
    for input in [
        r#"{"distance":"1000 m","ppm":5,"instrument_formula":"trimble-access-5600"}"#,
        r#"{"distance":"1000 m","temperature":"20 degC","pressure":"1000 hPa","humidity":50,"instrument_formula":"trimble-access-5600"}"#,
        r#"{"distance":"1000 m","temperature":"20 degC","pressure":"1000 hPa","wavelength":0.658,"instrument_formula":"trimble-access-5600"}"#,
    ] {
        let result = call("survey.reduction.edm-correction", input);
        assert_eq!(result["error"]["field"], "/instrument_formula", "{result}");
    }
    let direct = call(
        "survey.reduction.edm-correction",
        r#"{"distance":"1000 m","ppm":5,"humidity":50}"#,
    );
    assert_eq!(direct["error"]["field"], "/ppm", "{direct}");
}

#[test]
fn curvature_refraction_matches_the_usgs_table() {
    let result = call(
        "survey.reduction.curvature-refraction",
        r#"{"distance":"500 ft","refraction":0.14285714285714285}"#,
    );
    assert_eq!(result["ok"], true, "{result}");
    assert!((num(&result, "result.curvature.value") - 0.0060).abs() <= 0.00005);
    assert!((num(&result, "result.refraction_part.value").abs() - 0.0009).abs() <= 0.00005);
    assert!((num(&result, "result.correction.value") - 0.0051).abs() <= 0.00005);
}

#[test]
fn curvature_refraction_grows_with_distance_squared() {
    let run = |distance| {
        call(
            "survey.reduction.curvature-refraction",
            &format!(r#"{{"distance":"{distance} m","refraction":0.13}}"#),
        )
    };
    let near = run(250);
    let far = run(500);
    for field in ["correction", "curvature", "refraction_part"] {
        let ratio = num(&far, &format!("result.{field}.value"))
            / num(&near, &format!("result.{field}.value"));
        assert!((ratio - 4.0).abs() < 1e-12, "{field}: {near} {far}");
    }
}

#[test]
fn curvature_refraction_parts_recombine() {
    for (distance, k) in [(0.0, 0.13), (150.0, 0.0), (1000.0, 0.142), (5000.0, 0.2)] {
        let result = call(
            "survey.reduction.curvature-refraction",
            &format!(r#"{{"distance":"{distance} m","refraction":{k}}}"#),
        );
        let combined = num(&result, "result.correction.value");
        let parts =
            num(&result, "result.curvature.value") + num(&result, "result.refraction_part.value");
        assert!((combined - parts).abs() < 1e-12, "{result}");
    }
}

#[test]
fn curvature_refraction_coefficient_endpoints_are_consistent() {
    let run = |k| {
        call(
            "survey.reduction.curvature-refraction",
            &format!(r#"{{"distance":"1 km","refraction":{k}}}"#),
        )
    };
    let none = run(0.0);
    let half = run(0.5);
    let full = run(1.0);
    assert!(
        (num(&none, "result.correction.value") - num(&none, "result.curvature.value")).abs()
            < 1e-12
    );
    assert!(num(&full, "result.correction.value").abs() < 1e-12);
    assert!(
        (num(&full, "result.refraction_part.value") + num(&full, "result.curvature.value")).abs()
            < 1e-12
    );
    assert!(
        (num(&half, "result.correction.value") * 2.0 - num(&none, "result.correction.value")).abs()
            < 1e-12
    );
}

#[test]
fn curvature_refraction_is_unit_invariant_and_names_invalid_fields() {
    let miles = call(
        "survey.reduction.curvature-refraction",
        r#"{"distance":"1 mi","radius":"20906000 ft"}"#,
    );
    let meters = call(
        "survey.reduction.curvature-refraction",
        r#"{"distance":"1609.344 m","radius":"6372148.8 m"}"#,
    );
    assert!(
        (num(&miles, "result.correction.value") * 1609.344
            - num(&meters, "result.correction.value"))
        .abs()
            < 1e-12
    );
    for (input, field) in [
        (r#"{"distance":"-1 m"}"#, "/distance"),
        (r#"{"distance":"1 km","radius":"0 m"}"#, "/radius"),
    ] {
        let result = call("survey.reduction.curvature-refraction", input);
        assert_eq!(result["error"]["field"], field, "{result}");
    }
}

#[test]
fn ambiguous_ratio_shows_both_readings() {
    let r = call("survey.earthwork.grade", r#"{"grade":"3:1"}"#);
    assert_eq!(r["error"]["field"], "/convention", "{r}");
    let m = r["error"]["message"].as_str().unwrap();
    assert!(
        m.contains("3H:1V (about 18.43°, 33.3%)") && m.contains("3V:1H (about 71.57°, 300.0%)"),
        "{m}"
    );
    let r = call("survey.earthwork.grade", r#"{"grade":"3H:1V"}"#);
    assert_eq!(r["result"]["ratio_hv"], "3H:1V", "{r}");
    assert_eq!(r["display"]["percent"], "33.333", "{r}");
}

#[test]
fn grade_forms_and_length_invariants() {
    for input in [
        r#"{"grade":"5%"}"#,
        r#"{"grade":"50‰"}"#,
        r#"{"grade":"0.05"}"#,
        r#"{"grade":"1V:20H"}"#,
        r#"{"grade":"20H:1V"}"#,
    ] {
        let r = call("survey.earthwork.grade", input);
        assert!((num(&r, "result.percent") - 5.0).abs() < 1e-12, "{r}");
    }

    let rise_run = call("survey.earthwork.grade", r#"{"rise":"3 m","run":"4 m"}"#);
    let rise_slope = call(
        "survey.earthwork.grade",
        r#"{"rise":"3 m","slope_length":"5 m"}"#,
    );
    let run_slope = call(
        "survey.earthwork.grade",
        r#"{"run":"4 m","slope_length":"5 m"}"#,
    );
    for r in [&rise_run, &rise_slope, &run_slope] {
        assert!((num(r, "result.rise.value") - 3.0).abs() < 1e-12, "{r}");
        assert!((num(r, "result.run.value") - 4.0).abs() < 1e-12, "{r}");
        assert!(
            (num(r, "result.slope_length.value") - 5.0).abs() < 1e-12,
            "{r}"
        );
    }
}

#[test]
fn level_run_carries_its_arithmetic_check() {
    let r = call(
        "survey.reduction.level-run",
        r#"{"start_elevation":"100 ft","shots":[{"station":"BM 1","backsight":"4.52 ft"},{"station":"TP 1","foresight":"3.97 ft","backsight":"6.13 ft","distance":"300 ft"},{"station":"TP 2","foresight":"5.26 ft","backsight":"2.84 ft","distance":"280 ft"},{"station":"BM 1","foresight":"4.28 ft","distance":"310 ft"}]}"#,
    );
    let check = r["result"]["check"].as_str().unwrap();
    assert!(
        check.contains("ΣBS − ΣFS = 13.490 − 13.510 = -0.020 ft") && check.ends_with("balanced"),
        "{check}"
    );
    assert!(
        (num(&r, "result.misclosure.value") + 0.02).abs() < 1e-9,
        "{r}"
    );
    assert_eq!(r["display"]["misclosure"], "-0.02 ft", "{r}");
}

#[test]
fn tin_stockpile_approaches_an_analytic_cone() {
    // A cone of radius 30 and height 12: a 24-sided base, and about 500
    // surface shots on rings. The TIN's volume approaches πr²h/3 from below.
    let (r, h) = (30.0_f64, 12.0_f64);
    let mut base = Vec::new();
    for k in 0..24 {
        let a = k as f64 * std::f64::consts::TAU / 24.0;
        base.push(format!(
            r#"{{"easting":{:.6},"northing":{:.6},"elevation":0}}"#,
            r * a.cos(),
            r * a.sin()
        ));
    }
    let mut surf = vec![format!(r#"{{"easting":0,"northing":0,"elevation":{h}}}"#)];
    for ring in 1..=10 {
        let rr = r * ring as f64 / 11.0;
        let n = 10 * ring;
        for k in 0..n {
            let a = (k as f64 + 0.5 * (ring % 2) as f64) * std::f64::consts::TAU / n as f64;
            surf.push(format!(
                r#"{{"easting":{:.6},"northing":{:.6},"elevation":{:.9}}}"#,
                rr * a.cos(),
                rr * a.sin(),
                h * (1.0 - rr / r)
            ));
        }
    }
    assert!(surf.len() >= 500, "{} shots", surf.len());
    let input = format!(
        r#"{{"base":[{}],"surface":[{}]}}"#,
        base.join(","),
        surf.join(",")
    );
    let res = call("survey.earthwork.stockpile", &input);
    let v = num(&res, "result.volume.value");
    // The base polygon inscribes the circle, so compare with the cone on that polygon's area.
    let poly_area = 0.5 * 24.0 * r * r * (std::f64::consts::TAU / 24.0).sin();
    let cone = poly_area * h / 3.0;
    assert!((v - cone).abs() / cone < 0.01, "TIN {v} vs cone {cone}");
    let triangle_count = num(&res, "result.triangles") as usize;
    assert!(triangle_count > 900);
    let preview = res["result"]["surface_triangles"].as_array().unwrap();
    assert_eq!(preview.len(), triangle_count);
    assert!(preview.iter().all(|t| {
        ["a", "b", "c"].iter().all(|k| {
            t[k].as_u64()
                .is_some_and(|i| i < (base.len() + surf.len()) as u64)
        })
    }));
}

#[test]
fn sight_distance_invariants() {
    // Scaling every length scales L and K by the same factor. Swapping the
    // crest eye and object heights leaves the symmetric height term unchanged.
    let crest = call(
        "survey.curves.sight-distance",
        r#"{"curve":"crest","eye_height":"1.5 m","grade_change":10,"object_height":"0 m","sight_distance":"38.7 m"}"#,
    );
    assert!((num(&crest, "result.min_length.value") - 49.923).abs() < 1e-12);
    let crest_scaled = call(
        "survey.curves.sight-distance",
        r#"{"curve":"crest","eye_height":"3 m","grade_change":10,"object_height":"0 m","sight_distance":"77.4 m"}"#,
    );
    assert!(
        (num(&crest_scaled, "result.min_length.value")
            - 2.0 * num(&crest, "result.min_length.value"))
        .abs()
            < 1e-12
    );
    let a = call(
        "survey.curves.sight-distance",
        r#"{"curve":"crest","eye_height":"3.5 ft","grade_change":5,"object_height":"2 ft","sight_distance":"400 ft"}"#,
    );
    let b = call(
        "survey.curves.sight-distance",
        r#"{"curve":"crest","eye_height":"2 ft","grade_change":5,"object_height":"3.5 ft","sight_distance":"400 ft"}"#,
    );
    assert!(
        (num(&a, "result.min_length.value") - num(&b, "result.min_length.value")).abs() < 1e-12
    );

    let sag = call(
        "survey.curves.sight-distance",
        r#"{"curve":"sag","headlight_height":"2 ft","grade_change":6,"divergence":"1 deg","sight_distance":"500 ft"}"#,
    );
    let sag_scaled = call(
        "survey.curves.sight-distance",
        r#"{"curve":"sag","headlight_height":"4 ft","grade_change":6,"divergence":"1 deg","sight_distance":"1000 ft"}"#,
    );
    assert!(
        (num(&sag_scaled, "result.min_length.value") - 2.0 * num(&sag, "result.min_length.value"))
            .abs()
            < 1e-9
    );

    // Zero object height is a valid FHWA case; zero eye and headlight heights
    // are not, and invalid beam angles must not reach tan().
    for (input, field) in [
        (
            r#"{"curve":"crest","eye_height":0,"grade_change":5,"object_height":"2 ft","sight_distance":"400 ft"}"#,
            "/eye_height",
        ),
        (
            r#"{"curve":"sag","headlight_height":0,"grade_change":5,"divergence":"1 deg","sight_distance":"400 ft"}"#,
            "/headlight_height",
        ),
        (
            r#"{"curve":"sag","headlight_height":"2 ft","grade_change":5,"divergence":"90 deg","sight_distance":"400 ft"}"#,
            "/divergence",
        ),
    ] {
        let r = call("survey.curves.sight-distance", input);
        assert_eq!(r["error"]["field"], field, "{r}");
    }
}

#[test]
fn chi_square_quantiles_match_the_tables() {
    use gp_survey::lsq::chi2_inv;
    // Standard chi-square table values (NIST/SEMATECH e-Handbook, 1.3.6.7.4).
    for (p, k, want) in [
        (0.95, 2.0, 5.991),
        (0.975, 4.0, 11.143),
        (0.025, 4.0, 0.484),
        (0.975, 10.0, 20.483),
        (0.025, 10.0, 3.247),
        (0.975, 1.0, 5.024),
        (0.025, 30.0, 16.791),
    ] {
        let got = chi2_inv(p, k);
        assert!(
            (got - want).abs() < 6e-4,
            "χ²({p}, {k}) = {got}, table {want}"
        );
    }
}

#[test]
fn a_perfect_fit_has_no_standardized_residuals_rather_than_infinite_ones() {
    // Found by the bounds fuzzer: repeating two angles until the adjustment
    // fits them exactly drives the reference variance to zero, and a
    // standardized residual of 0/0 is not a residual of zero.
    let mut angles = String::new();
    for i in 0..30 {
        if i > 0 {
            angles.push(',');
        }
        angles.push_str(r#"{"station":"A","backsight":"B","foresight":"P","angle":"303-41-26","sd":5},{"station":"B","backsight":"P","foresight":"A","angle":"303-41-20","sd":5}"#);
    }
    let r = call(
        "survey.cogo.least-squares-2d",
        &format!(
            r#"{{"angles":[{angles}],"control":[{{"name":"A","easting":"1000 m","northing":"1000 m"}},{{"name":"B","easting":"1400 m","northing":"1000 m"}}],"unknowns":[{{"name":"P","easting":"1200 m","northing":"1300 m"}}]}}"#
        ),
    );
    assert_eq!(r["ok"], true, "{r}");
    for row in r["result"]["residuals"].as_array().unwrap() {
        let z = row["standardized"]
            .as_f64()
            .unwrap_or_else(|| panic!("{row}"));
        assert!(z.is_finite(), "{row}");
    }
}

/// A tool's own text names Ghilani and Wolf by the edition its references
/// cite (Elementary Surveying, 16th edition, 2021), never an older year.
#[test]
fn ghilani_citations_match_the_cited_edition() {
    let mut bad = Vec::new();
    for t in TOOLS {
        let cited: Vec<String> = t
            .references
            .iter()
            .filter(|r| r.issuer.contains("Ghilani"))
            .map(|r| r.year.to_string())
            .collect();
        let texts = [t.model, t.accuracy, t.when_to_use, t.limitations]
            .into_iter()
            .chain(t.examples.iter().map(|e| e.source));
        for text in texts {
            for (i, _) in text.match_indices("Ghilani") {
                let tail: String = text[i..].chars().take(40).collect();
                let year: String = tail
                    .split(|c: char| !c.is_ascii_digit())
                    .find(|w| w.len() == 4)
                    .unwrap_or_default()
                    .to_owned();
                if !year.is_empty() && !cited.contains(&year) {
                    bad.push(format!("{}: {tail}", t.id));
                }
            }
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn inverse_forward_invariants() {
    // Forward undoes inverse; swapping the points turns the azimuth by 180°
    // and keeps the distance; shifting both points changes nothing; turning
    // the pair about the origin adds the turn to the azimuth.
    let inv = |n1: f64, e1: f64, n2: f64, e2: f64| {
        let r = call(
            "survey.cogo.inverse",
            &serde_json::json!({"northing1": n1, "easting1": e1, "northing2": n2, "easting2": e2})
                .to_string(),
        );
        (
            num(&r, "result.distance.value"),
            num(&r, "result.azimuth.value"),
        )
    };
    let fwd = |n: f64, e: f64, az: f64, d: f64| {
        let r = call(
            "survey.cogo.forward",
            &serde_json::json!({"northing": n, "easting": e, "direction": format!("{az}"), "distance": d})
                .to_string(),
        );
        (
            num(&r, "result.northing.value"),
            num(&r, "result.easting.value"),
        )
    };
    let turn = |a: f64| a.rem_euclid(360.0);
    let close = |a: f64, b: f64| {
        let d = (a - b).rem_euclid(360.0);
        d.min(360.0 - d)
    };
    let pts = [
        (512_345.678, 2_104_567.891, 513_001.234, 2_103_210.987),
        (100.0, 100.0, 40.0, 180.0),
        (0.0, 0.0, -0.37, -1234.5),
        (8_765.4, 1_000.0, 8_765.4, 1_000.25),
    ];
    for (n1, e1, n2, e2) in pts {
        let (d, az) = inv(n1, e1, n2, e2);
        let (n, e) = fwd(n1, e1, az, d);
        assert!((n - n2).abs() < 1e-6 && (e - e2).abs() < 1e-6, "{n} {e}");
        let (d2, az2) = inv(n2, e2, n1, e1);
        assert!((d2 - d).abs() <= 1e-12 * d.max(1.0));
        assert!(close(az2, turn(az + 180.0)) < 1e-9);
        let (d3, az3) = inv(n1 - 3_000.0, e1 + 7_000.0, n2 - 3_000.0, e2 + 7_000.0);
        assert!((d3 - d).abs() < 1e-8 && close(az3, az) < 1e-8);
        let t = 0.7_f64;
        let rot = |n: f64, e: f64| (n * t.cos() - e * t.sin(), n * t.sin() + e * t.cos());
        let (rn1, re1) = rot(n1, e1);
        let (rn2, re2) = rot(n2, e2);
        let (d4, az4) = inv(rn1, re1, rn2, re2);
        assert!((d4 - d).abs() < 1e-8 * d.max(1.0));
        assert!(close(az4, turn(az + t.to_degrees())) < 1e-7);
    }
}

#[test]
fn average_end_area_invariants() {
    // The end areas may be swapped; the volume scales with the length; units
    // do not change the answer; and splitting a span at a section whose area
    // is the average of its ends gives the same total.
    let vol = |a1: &str, a2: &str, l: &str| {
        let r = call(
            "survey.earthwork.average-end-area",
            &serde_json::json!({"area1": a1, "area2": a2, "length": l}).to_string(),
        );
        num(&r, "result.volume_ft3.value")
    };
    for (a, b, l) in [(120.0, 180.0, 100.0), (0.0, 915.5, 37.25), (2.5, 2.5, 0.5)] {
        let v = vol(&format!("{a} ft2"), &format!("{b} ft2"), &format!("{l} ft"));
        assert!(
            (vol(&format!("{b} ft2"), &format!("{a} ft2"), &format!("{l} ft")) - v).abs()
                <= 1e-9 * v.max(1.0)
        );
        let v3 = vol(
            &format!("{a} ft2"),
            &format!("{b} ft2"),
            &format!("{} ft", 3.0 * l),
        );
        assert!((v3 - 3.0 * v).abs() <= 1e-9 * v.max(1.0));
        let m2 = 0.3048 * 0.3048;
        let vm = vol(
            &format!("{} m2", a * m2),
            &format!("{} m2", b * m2),
            &format!("{} m", l * 0.3048),
        );
        assert!((vm - v).abs() <= 1e-9 * v.max(1.0));
        let mid = (a + b) / 2.0;
        let half = format!("{} ft", l / 2.0);
        let split = vol(&format!("{a} ft2"), &format!("{mid} ft2"), &half)
            + vol(&format!("{mid} ft2"), &format!("{b} ft2"), &half);
        assert!((split - v).abs() <= 1e-9 * v.max(1.0));
    }
}

#[test]
fn level_run_invariants() {
    // Raising every elevation by a constant raises every result by it; the
    // arithmetic check always holds; the adjusted closing benchmark is the
    // known one; and a book whose sums close exactly has no misclosure.
    let run = |start: f64, end: f64| {
        let input = serde_json::json!({
            "start_elevation": format!("{start} ft"),
            "end_elevation": format!("{end} ft"),
            "shots": [
                {"station": "BM A", "backsight": "8.42 ft"},
                {"station": "TP 1", "backsight": "11.56 ft", "foresight": "1.20 ft", "distance": "300 ft"},
                {"station": "TP 2", "backsight": "6.15 ft", "foresight": "1.35 ft", "distance": "280 ft"},
                {"station": "TP 3", "backsight": "4.39 ft", "foresight": "10.90 ft", "distance": "250 ft"},
                {"station": "BM K", "foresight": "5.94 ft", "distance": "310 ft"},
            ],
        });
        call("survey.reduction.level-run", &input.to_string())
    };
    let a = run(820.0, 831.15);
    let b = run(1820.0, 1831.15);
    for i in 0..5 {
        let e = |r: &Value, k: &str| r["result"]["stations"][i][k]["value"].as_f64().unwrap();
        assert!((e(&b, "elevation") - e(&a, "elevation") - 1000.0).abs() < 1e-9);
        assert!((e(&b, "adjusted") - e(&a, "adjusted") - 1000.0).abs() < 1e-9);
    }
    assert!((num(&a, "result.misclosure.value") - num(&b, "result.misclosure.value")).abs() < 1e-9);
    let sums = num(&a, "result.sum_backsights.value") - num(&a, "result.sum_foresights.value");
    let st =
        |r: &Value, i: usize, k: &str| r["result"]["stations"][i][k]["value"].as_f64().unwrap();
    let last = st(&a, 4, "elevation");
    assert!((sums - (last - 820.0)).abs() < 1e-9);
    assert!((st(&a, 4, "adjusted") - 831.15).abs() < 1e-9);
    let exact = run(820.0, last);
    assert!(num(&exact, "result.misclosure.value").abs() < 1e-9);
    for i in 0..5 {
        assert!((st(&exact, i, "adjusted") - st(&exact, i, "elevation")).abs() < 1e-9);
    }
}

#[test]
fn slope_stake_invariants() {
    // Raising the grade and the ground together moves nothing; the catch lies
    // on both the ground and the design slope; and mirroring sides changes
    // only the letter in the stake.
    let run = |g: f64, ground: &[(f64, f64)], side: &str| {
        let pts: Vec<Value> = ground
            .iter()
            .map(|(x, y)| serde_json::json!({"offset": format!("{x} ft"), "elevation": format!("{y} ft")}))
            .collect();
        let input = serde_json::json!({"grade_elevation": format!("{g} ft"), "half_width": "16 ft", "ground": pts, "cut_slope": 2, "fill_slope": 3, "side": side});
        call("survey.earthwork.slope-stake", &input.to_string())
    };
    for ground in [
        vec![(0.0, 102.0), (60.0, 108.0)],
        vec![(0.0, 98.0), (30.0, 95.5), (90.0, 88.0)],
        vec![(0.0, 101.0), (20.0, 99.0), (70.0, 104.0)],
    ] {
        let a = run(100.0, &ground, "right");
        let up: Vec<_> = ground.iter().map(|(x, y)| (*x, y + 250.0)).collect();
        let b = run(350.0, &up, "right");
        let x = num(&a, "result.catch_offset.value");
        assert!((num(&b, "result.catch_offset.value") - x).abs() < 1e-6);
        assert!((num(&b, "result.cut_fill.value") - num(&a, "result.cut_fill.value")).abs() < 1e-6);
        let cut = a["result"]["cut_or_fill"] == "cut";
        let design = if cut {
            100.0 + (x - 16.0) / 2.0
        } else {
            100.0 - (x - 16.0) / 3.0
        };
        assert!(
            (num(&a, "result.ground_at_catch.value") - design).abs() < 1e-5,
            "{a}"
        );
        let left = run(100.0, &ground, "left");
        let (ra, la) = (
            a["result"]["stake"].as_str().unwrap(),
            left["result"]["stake"].as_str().unwrap(),
        );
        assert_eq!(ra.trim_end_matches('R'), la.trim_end_matches('L'));
    }
}

#[test]
fn intersection_invariants() {
    // Each answer lies on what defined it: the inverse from each known point
    // gives back its direction or distance. Shifting the whole problem shifts
    // the answer, and swapping the two points leaves a crossing where it was.
    let run = |v: Value| call("survey.cogo.intersection", &v.to_string());
    let sol =
        |r: &Value, i: usize, k: &str| r["result"]["solutions"][i][k]["value"].as_f64().unwrap();
    let inverse = |n1: f64, e1: f64, n2: f64, e2: f64| {
        let r = call(
            "survey.cogo.inverse",
            &serde_json::json!({"northing1": n1, "easting1": e1, "northing2": n2, "easting2": e2})
                .to_string(),
        );
        (
            num(&r, "result.azimuth.value"),
            num(&r, "result.distance.value"),
        )
    };
    let (na, ea, nb, eb) = (4080.822, 5447.330, 4377.864, 5752.796);
    let (a1, a2) = (334.813_055_555_555_6, 308.666_111_111_111_1);
    let ll = run(
        serde_json::json!({"northing1": na, "easting1": ea, "direction1": a1.to_string(), "northing2": nb, "easting2": eb, "direction2": a2.to_string()}),
    );
    let (nc, ec) = (sol(&ll, 0, "northing"), sol(&ll, 0, "easting"));
    assert!((inverse(na, ea, nc, ec).0 - a1).abs() < 1e-9);
    assert!((inverse(nb, eb, nc, ec).0 - a2).abs() < 1e-9);
    let swapped = run(
        serde_json::json!({"northing1": nb, "easting1": eb, "direction1": a2.to_string(), "northing2": na, "easting2": ea, "direction2": a1.to_string()}),
    );
    assert!(
        (sol(&swapped, 0, "northing") - nc).abs() < 1e-8
            && (sol(&swapped, 0, "easting") - ec).abs() < 1e-8
    );
    let shifted = run(
        serde_json::json!({"northing1": na + 1e5, "easting1": ea - 2e5, "direction1": a1.to_string(), "northing2": nb + 1e5, "easting2": eb - 2e5, "direction2": a2.to_string()}),
    );
    assert!((sol(&shifted, 0, "northing") - nc - 1e5).abs() < 1e-6);
    assert!((sol(&shifted, 0, "easting") - ec + 2e5).abs() < 1e-6);
    // Two circles: both answers sit at the two distances.
    let cc = run(
        serde_json::json!({"northing1": na, "easting1": ea, "distance1": 700.0, "northing2": nb, "easting2": eb, "distance2": 520.0}),
    );
    for i in 0..2 {
        let (n, e) = (sol(&cc, i, "northing"), sol(&cc, i, "easting"));
        assert!((inverse(na, ea, n, e).1 - 700.0).abs() < 1e-8);
        assert!((inverse(nb, eb, n, e).1 - 520.0).abs() < 1e-8);
    }
    // A line and a circle: both answers on the line and on the circle.
    let lc = run(
        serde_json::json!({"northing1": na, "easting1": ea, "direction1": "10", "northing2": nb, "easting2": eb, "distance2": 600.0}),
    );
    for i in 0..sol_count(&lc) {
        let (n, e) = (sol(&lc, i, "northing"), sol(&lc, i, "easting"));
        assert!((inverse(na, ea, n, e).0 - 10.0).abs() < 1e-8);
        assert!((inverse(nb, eb, n, e).1 - 600.0).abs() < 1e-8);
    }
}

#[test]
fn named_intersections_match_the_general_tool() {
    let cases = [
        (
            "survey.cogo.bearing-bearing",
            serde_json::json!({
                "northing1": "0 m", "easting1": "0 m", "direction1": "N 45 E",
                "northing2": "0 m", "easting2": "100 m", "direction2": "N 45 W"
            }),
        ),
        (
            "survey.cogo.bearing-distance",
            serde_json::json!({
                "northing1": "0 m", "easting1": "0 m", "direction1": "0",
                "northing2": "40 m", "easting2": "30 m", "distance2": "50 m"
            }),
        ),
        (
            "survey.cogo.distance-distance",
            serde_json::json!({
                "northing1": "1000 ft", "easting1": "1000 ft", "distance1": "300 ft",
                "northing2": "1000 ft", "easting2": "1400 ft", "distance2": "250 ft"
            }),
        ),
    ];
    for (tool, input) in cases {
        let named = call(tool, &input.to_string());
        let general = call("survey.cogo.intersection", &input.to_string());
        assert_eq!(named["result"], general["result"], "{tool}");
        assert_eq!(named["display"], general["display"], "{tool}");
        let named_warnings: Vec<&Value> = named["meta"]["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|warning| warning["code"] != "EXPERIMENTAL_TOOL")
            .collect();
        let general_warnings: Vec<&Value> = general["meta"]["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .collect();
        assert_eq!(named_warnings, general_warnings, "{tool}");
    }
}

fn sol_count(r: &Value) -> usize {
    r["result"]["count"].as_u64().unwrap() as usize
}

#[test]
fn angular_closure_matches_the_walla_walla_worked_example() {
    let measured = [128.5, 129.0, 211.0, 70.5, 80.5, 194.0, 89.5];
    let result = angular(serde_json::json!({
        "angles": measured.map(|angle| serde_json::json!({"angle": angle.to_string()}))
    }));
    assert_eq!(result["ok"], true, "{result}");
    assert_eq!(num(&result, "result.misclosure.value"), 3.0 * 3600.0);
    assert!((num(&result, "result.correction.value") + 3.0 * 3600.0 / 7.0).abs() < 1e-9);
    let adjusted = result["result"]["adjusted"].as_array().unwrap();
    for (row, published) in adjusted
        .iter()
        .zip([128.071, 128.571, 210.571, 70.071, 80.071, 193.571, 89.071])
    {
        assert!((angle_value(row["angle"].as_str().unwrap()) - published).abs() <= 0.0005);
    }
}

#[test]
fn angular_closure_adjusted_angles_reach_the_geometric_sum() {
    for (angles, required) in [
        (vec![73.2, 94.8, 121.1, 70.95], 360.0),
        (vec![108.2, 107.9, 109.1, 106.8, 108.3], 540.0),
    ] {
        let result = angular(serde_json::json!({
            "angles": angles.iter().map(|angle| serde_json::json!({"angle": angle.to_string()})).collect::<Vec<_>>()
        }));
        let sum: f64 = result["result"]["adjusted"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| angle_value(row["angle"].as_str().unwrap()))
            .sum();
        assert!((sum - required).abs() < 0.0002, "{result}");
    }
}

#[test]
fn angular_closure_equal_bias_is_removed_by_the_adjustment() {
    let base = [72.0, 83.0, 91.0, 114.0];
    let run = |bias: f64| {
        angular(serde_json::json!({
            "angles": base.map(|angle| serde_json::json!({"angle": (angle + bias).to_string()}))
        }))
    };
    assert_eq!(
        run(0.0)["result"]["adjusted"],
        run(0.25)["result"]["adjusted"]
    );
}

#[test]
fn angular_closure_reflex_exteriors_complement_interiors() {
    let interiors = [72.0, 83.0, 91.0, 114.0];
    let inside = angular(serde_json::json!({
        "angles": interiors.map(|angle| serde_json::json!({"angle": angle.to_string()}))
    }));
    let outside = angular(serde_json::json!({
        "angles": interiors.map(|angle| serde_json::json!({"angle": (360.0 - angle).to_string()})),
        "kind": "exterior"
    }));
    let a = inside["result"]["adjusted"].as_array().unwrap();
    let b = outside["result"]["adjusted"].as_array().unwrap();
    for (interior, exterior) in a.iter().zip(b) {
        let total = angle_value(interior["angle"].as_str().unwrap())
            + angle_value(exterior["angle"].as_str().unwrap());
        assert!((total - 360.0).abs() < 1e-9);
    }
}

#[test]
fn angular_closure_allowable_boundary_and_invalid_row_are_explicit() {
    let at_limit = angular(serde_json::json!({
        "angles": [
            {"angle":"90°00'05\""},
            {"angle":"90°00'05\""},
            {"angle":"90°00'05\""},
            {"angle":"90°00'05\""}
        ],
        "allowable_k": 10
    }));
    assert_eq!(at_limit["result"]["closure"], "within the allowable");
    assert!((num(&at_limit, "result.misclosure.value") - 20.0).abs() < 1e-9);
    assert_eq!(num(&at_limit, "result.allowable.value"), 20.0);

    let invalid = angular(serde_json::json!({
        "angles": [{"angle":"60"}, {"angle":"bad"}, {"angle":"120"}]
    }));
    assert_eq!(invalid["error"]["field"], "/angles/1/angle");
}
