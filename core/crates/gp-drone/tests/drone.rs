//! Drone photogrammetry: catalog lint, examples, vectors, and spec scenarios.

use std::path::Path;

use gp_base::{manifest, template, vectors};
use gp_drone::{REGISTRY, TOOLS};
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

const CAM: &str = r#""sensor_width":"13.2 mm","sensor_height":"8.8 mm","focal_length":"8.8 mm","image_width":5472,"image_height":3648"#;

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
    let mut failures = manifest::lint(
        TOOLS,
        &taxonomy,
        &[
            "navigation.los.fresnel",
            "navigation.los.horizon",
            "geodesy.geoid.geoid-height",
            "aviation.weather.metar-decode",
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
        let minimum = if t.inputs.is_empty() { 1 } else { 5 };
        if vectors::count(&text) < minimum {
            failures.push(format!("{} has fewer than {minimum} vectors", t.id));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn focused_photogrammetry_operations_match_their_general_tools() {
    let camera = serde_json::json!({
        "height": "90 m", "sensor_width": "13.2 mm", "sensor_height": "8.8 mm", "focal_length": "8.8 mm"
    });
    let with = |extra: Value| {
        let mut v = camera.clone();
        for (k, x) in extra.as_object().unwrap() {
            v[k] = x.clone();
        }
        v.to_string()
    };
    // The general tools also need the pixel counts the focused ones leave out.
    let pixels = serde_json::json!({"image_width": 5472, "image_height": 3648});
    let general = |extra: Value| {
        let mut v = extra;
        for (k, x) in pixels.as_object().unwrap() {
            v[k] = x.clone();
        }
        with(v)
    };

    // Footprint: the GSD tool's footprints, and their product.
    let fp = call("drone.photogrammetry.footprint", &camera.to_string());
    let gsd = call("drone.photogrammetry.gsd", &general(serde_json::json!({})));
    assert_eq!(fp["ok"], true, "{fp}");
    for key in ["footprint_across", "footprint_along", "focal_length_used"] {
        assert_eq!(fp["result"][key], gsd["result"][key], "{key}");
    }
    assert!((num(&fp, "result.footprint_across.value") - 135.0).abs() < 1e-9);
    assert!((num(&fp, "result.footprint_along.value") - 90.0).abs() < 1e-9);
    assert!((num(&fp, "result.footprint_area.value") - 12_150.0).abs() < 1e-6);

    // Spacing: each focused operation is one output of the overlap tool.
    let overlaps = serde_json::json!({"front_overlap": 80, "side_overlap": 70, "groundspeed": "12 m/s", "min_interval": "2 s"});
    let parent = call("drone.photogrammetry.trigger", &general(overlaps.clone()));
    let distance = call(
        "drone.photogrammetry.trigger-distance",
        &with(serde_json::json!({"front_overlap": 80})),
    );
    let line = call(
        "drone.photogrammetry.line-spacing",
        &with(serde_json::json!({"side_overlap": 70})),
    );
    let interval = call(
        "drone.photogrammetry.trigger-interval",
        &with(
            serde_json::json!({"front_overlap": 80, "groundspeed": "12 m/s", "min_interval": "2 s"}),
        ),
    );
    assert_eq!(
        distance["result"]["trigger_distance"],
        parent["result"]["trigger_distance"]
    );
    assert_eq!(
        distance["result"]["footprint_along"],
        parent["result"]["footprint_along"]
    );
    assert_eq!(
        line["result"]["line_spacing"],
        parent["result"]["line_spacing"]
    );
    assert_eq!(
        line["result"]["footprint_across"],
        parent["result"]["footprint_across"]
    );
    for key in ["trigger_interval", "trigger_distance", "max_groundspeed"] {
        assert_eq!(interval["result"][key], parent["result"][key], "{key}");
    }
    assert!((num(&distance, "result.trigger_distance.value") - 18.0).abs() < 1e-9);
    assert!((num(&line, "result.line_spacing.value") - 40.5).abs() < 1e-9);
    assert!((num(&interval, "result.trigger_interval.value") - 1.5).abs() < 1e-9);
    assert!((num(&interval, "result.max_groundspeed.value") - 9.0).abs() < 1e-9);
    // 1.5 s is faster than a 2 s camera: both tools say so.
    assert!(codes(&interval).contains(&"TRIGGER_TOO_FAST".to_owned()));
    assert!(codes(&parent).contains(&"TRIGGER_TOO_FAST".to_owned()));
    // Left out, the overlaps default to the general 75% and 60%.
    let d = call("drone.photogrammetry.trigger-distance", &camera.to_string());
    let l = call("drone.photogrammetry.line-spacing", &camera.to_string());
    assert!((num(&d, "result.trigger_distance.value") - 22.5).abs() < 1e-9);
    assert!((num(&l, "result.line_spacing.value") - 54.0).abs() < 1e-9);
    // Portrait swaps which side of the sensor runs along the line.
    let p = call(
        "drone.photogrammetry.trigger-distance",
        &with(serde_json::json!({"orientation": "portrait"})),
    );
    assert!((num(&p, "result.footprint_along.value") - 135.0).abs() < 1e-9);

    // Max shutter: the blur tool's limit, without an exposure to judge.
    let shutter = call(
        "drone.photogrammetry.max-shutter",
        r#"{"groundspeed":"12 m/s","gsd":"2.5 cm"}"#,
    );
    let blur = call(
        "drone.photogrammetry.motion-blur",
        r#"{"groundspeed":"12 m/s","gsd":"2.5 cm","exposure":"0.001 s"}"#,
    );
    for key in ["max_exposure", "max_exposure_fraction"] {
        assert_eq!(shutter["result"][key], blur["result"][key], "{key}");
    }
    assert_eq!(shutter["result"]["max_exposure_fraction"], "1/960 s");
    assert_eq!(num(&shutter, "result.blur_limit"), 0.5);
    // Twice the allowed blur doubles the exposure.
    let loose = call(
        "drone.photogrammetry.max-shutter",
        r#"{"groundspeed":"12 m/s","gsd":"2.5 cm","blur_limit":1}"#,
    );
    assert!(
        (num(&loose, "result.max_exposure.value") / num(&shutter, "result.max_exposure.value")
            - 2.0)
            .abs()
            < 1e-12
    );

    // Crop factor: a full frame is 1, and the two focal lengths invert each other.
    let full = call(
        "drone.photogrammetry.crop-factor",
        r#"{"sensor_width":"36 mm","sensor_height":"24 mm"}"#,
    );
    assert!((num(&full, "result.crop_factor") - 1.0).abs() < 1e-15);
    let one_inch = call(
        "drone.photogrammetry.crop-factor",
        r#"{"sensor_width":"13.2 mm","sensor_height":"8.8 mm","focal_length":"8.8 mm"}"#,
    );
    let equivalent = num(&one_inch, "result.equivalent_focal_length.value");
    assert!((equivalent - 24.0).abs() < 0.01, "{equivalent}");
    let back = call(
        "drone.photogrammetry.crop-factor",
        &serde_json::json!({"sensor_width": "13.2 mm", "sensor_height": "8.8 mm", "equivalent_focal_length": format!("{equivalent} mm")}).to_string(),
    );
    assert!((num(&back, "result.focal_length.value") - 8.8).abs() < 1e-12);
    // The GSD tool derives the same physical focal length from the equivalent one.
    let derived = call(
        "drone.photogrammetry.gsd",
        &serde_json::json!({
            "height": "100 m", "sensor_width": "13.2 mm", "sensor_height": "8.8 mm", "image_width": 5472,
            "focal_length": format!("{equivalent} mm"), "focal_length_type": "equivalent-35mm"
        })
        .to_string(),
    );
    assert!((num(&derived, "result.focal_length_used.value") - 8.8).abs() < 1e-12);

    // Checkpoint requirements: Table C.1 at its edges, as the plan tool counts them.
    for (area, want) in [
        ("12 km2", 30.0),
        ("500 km2", 30.0),
        ("501 km2", 35.0),
        ("2500 km2", 70.0),
    ] {
        let r = call(
            "drone.photogrammetry.checkpoint-requirements",
            &serde_json::json!({"area_size": area}).to_string(),
        );
        assert_eq!(num(&r, "result.checkpoints"), want, "{area}");
    }
    let split = call(
        "drone.photogrammetry.checkpoint-requirements",
        r#"{"area_size":"12 km2","vegetated":25,"horizontal_class":"5 cm","vertical_class":"10 cm"}"#,
    );
    assert_eq!(num(&split, "result.checkpoints"), 60.0);
    assert_eq!(num(&split, "result.checkpoints_vegetated"), 30.0);
    assert!((num(&split, "result.checkpoint_rmse_h.value") - 2.5).abs() < 1e-12);
    assert!((num(&split, "result.checkpoint_rmse_v.value") - 5.0).abs() < 1e-12);
    let beyond = call(
        "drone.photogrammetry.checkpoint-requirements",
        r#"{"area_size":"2600 km2"}"#,
    );
    assert_eq!(beyond["error"]["code"], "OUT_OF_DOMAIN");
}

#[test]
fn focused_power_operations_match_their_general_tools() {
    // Usable energy: the endurance tool's energy budget, without a power draw.
    let budget = serde_json::json!({"energy": "99 Wh", "usable": 85, "reserve": 25, "battery_temperature": "5 degC"});
    let mut with_power = budget.clone();
    with_power["power"] = serde_json::json!("180 W");
    let usable = call("drone.power.usable-energy", &budget.to_string());
    let endurance = call("drone.power.endurance", &with_power.to_string());
    assert_eq!(usable["ok"], true, "{usable}");
    for key in ["usable_energy", "reserve_energy", "derating_applied"] {
        assert_eq!(usable["result"][key], endurance["result"][key], "{key}");
    }
    // 99 Wh × 85% × (1 − 15%) × (1 − 25%).
    let flyable = num(&usable, "result.flyable_energy.value");
    assert!((flyable - 99.0 * 0.85 * 0.85 * 0.75).abs() < 1e-9);
    assert!(codes(&usable).contains(&"HEURISTIC_DERATING".to_owned()));
    // Energy to fly on at the hover power is the endurance tool's hover time.
    assert!((flyable / 180.0 * 60.0 - num(&endurance, "result.hover_time.value")).abs() < 1e-9);

    // Cold derating: the rule of thumb's fixed points, and the same figure the budget used.
    for (temp, want) in [
        ("20 degC", 0.0),
        ("0 degC", 20.0),
        ("5 degC", 15.0),
        ("-40 degC", 50.0),
        ("32 degF", 20.0),
    ] {
        let r = call(
            "drone.power.temperature-derating",
            &serde_json::json!({"battery_temperature": temp}).to_string(),
        );
        assert!(
            (num(&r, "result.derating_applied") - want).abs() < 1e-9,
            "{temp}"
        );
        assert!(
            (num(&r, "result.share_remaining") - (100.0 - want)).abs() < 1e-9,
            "{temp}"
        );
        // It is a rule of thumb whenever it takes anything off, and says so.
        assert_eq!(
            codes(&r).contains(&"HEURISTIC_DERATING".to_owned()),
            want > 0.0,
            "{temp}"
        );
    }

    // Range: the endurance tool's range at the same power and groundspeed.
    let mut flight = budget.clone();
    flight["power"] = serde_json::json!("180 W");
    flight["groundspeed"] = serde_json::json!("14 m/s");
    let range = call("drone.power.range", &flight.to_string());
    let endurance = call("drone.power.endurance", &flight.to_string());
    assert_eq!(range["result"]["range"], endurance["result"]["range"]);
    assert_eq!(
        range["result"]["usable_energy"],
        endurance["result"]["usable_energy"]
    );
    // Range is flight time times groundspeed.
    assert!(
        (num(&range, "result.flight_time.value") * 60.0 * 14.0 / 1000.0
            - num(&range, "result.range.value"))
        .abs()
            < 1e-9
    );
    // A headwind shortens it by the same share it takes off the groundspeed.
    let mut headwind = budget.clone();
    for (k, v) in [
        ("power", "180 W"),
        ("airspeed", "14 m/s"),
        ("course", "90 deg"),
        ("wind_direction", "90 deg"),
        ("wind_speed", "4 m/s"),
    ] {
        headwind[k] = serde_json::json!(v);
    }
    let into_wind = call("drone.power.range", &headwind.to_string());
    assert!((num(&into_wind, "result.groundspeed_used.value") - 10.0).abs() < 1e-9);
    assert!(
        (num(&into_wind, "result.range.value") / num(&range, "result.range.value") - 10.0 / 14.0)
            .abs()
            < 1e-9
    );

    // Disk area: the hover tool's area and loading from the same airframe.
    let frame = serde_json::json!({"rotors": 6, "rotor_diameter": "15 in", "mass": "6.2 kg"});
    let disk = call("drone.power.disk-area", &frame.to_string());
    let hover = call("drone.power.hover-power", &frame.to_string());
    for key in ["disk_area", "disk_loading"] {
        assert_eq!(disk["result"][key], hover["result"][key], "{key}");
    }
    assert!(
        (num(&disk, "result.area_per_rotor.value") * 6.0 - num(&disk, "result.disk_area.value"))
            .abs()
            < 1e-12
    );
    // Without a mass there is an area and no loading.
    let bare = call(
        "drone.power.disk-area",
        r#"{"rotors":4,"rotor_diameter":"10 in"}"#,
    );
    assert_eq!(bare["ok"], true);
    assert!(bare["result"]["disk_loading"].is_null());

    // C-rate: the battery tool's current and C-rate for the same draw.
    let pack =
        serde_json::json!({"capacity": "10000 mAh", "cells": 6, "power": "480 W", "max_c_rate": 2});
    let rate = call("drone.power.c-rate", &pack.to_string());
    let battery = call("drone.power.battery-energy", &pack.to_string());
    for key in ["c_rate", "current", "voltage"] {
        assert_eq!(rate["result"][key], battery["result"][key], "{key}");
    }
    assert!((num(&rate, "result.c_rate") - 480.0 / 22.2 / 10.0).abs() < 1e-12);
    // 2.16 C is over a 2 C rating: both tools say so, and both name the nominal voltage.
    for r in [&rate, &battery] {
        assert!(codes(r).contains(&"C_RATE_EXCEEDED".to_owned()));
        assert!(codes(r).contains(&"NOMINAL_VALUE_USED".to_owned()));
    }
    // A draw is required: there is no rate without one.
    assert_eq!(
        call(
            "drone.power.c-rate",
            r#"{"capacity":"5000 mAh","voltage":"14.8 V"}"#
        )["ok"],
        false
    );
}

#[test]
fn focused_mission_and_ops_operations_match_their_general_tools() {
    // Structure altitude: the Part 107 tool's answer, with the structure required.
    for input in [
        serde_json::json!({"structure_height": "1150 ft", "structure_distance": "250 ft"}),
        serde_json::json!({"structure_height": "300 ft", "structure_distance": "401 ft", "ground_elevation": "5280 ft"}),
        serde_json::json!({"structure_height": "90 m", "structure_distance": "60 m", "ground_elevation": "640 ft", "geoid_height": "-17 m"}),
    ] {
        let named = call("drone.ops.structure-altitude", &input.to_string());
        let general = call("drone.ops.part107-altitude", &input.to_string());
        assert_eq!(named["ok"], true, "{named}");
        assert_eq!(named["result"], general["result"], "{input}");
    }
    let edge = |d: &str| {
        num(
            &call(
                "drone.ops.structure-altitude",
                &serde_json::json!({"structure_height": "300 ft", "structure_distance": d})
                    .to_string(),
            ),
            "result.max_agl.value",
        )
    };
    // Exactly 400 ft away is within the radius; a foot more is not.
    assert_eq!(edge("400 ft"), 700.0);
    assert_eq!(edge("401 ft"), 400.0);
    // With no structure there is nothing for this operation to answer.
    assert_eq!(call("drone.ops.structure-altitude", "{}")["ok"], false);
    assert_eq!(call("drone.ops.part107-altitude", "{}")["ok"], true);

    // Crosshatch: the survey grid with its crosshatch switch on, and the sum of two single passes.
    let site = serde_json::json!({
        "area": [{"lat": 35, "lon": -100}, {"lat": 35, "lon": -99.99704}, {"lat": 35.0018, "lon": -99.99704}, {"lat": 35.0018, "lon": -100}],
        "line_spacing": "45 m", "photo_spacing": "25 m", "groundspeed": "9 m/s", "height": "100 m"
    });
    let with = |extra: Value| {
        let mut v = site.clone();
        for (k, x) in extra.as_object().unwrap() {
            v[k] = x.clone();
        }
        v.to_string()
    };
    let cross = call("drone.mission.crosshatch", &site.to_string());
    let switched = call(
        "drone.mission.survey-grid",
        &with(serde_json::json!({"crosshatch": "yes"})),
    );
    assert_eq!(cross["ok"], true, "{}", cross["error"]);
    assert_eq!(cross["result"], switched["result"]);
    let first = call("drone.mission.survey-grid", &site.to_string());
    let direction = num(&first, "result.direction.value");
    let second = call(
        "drone.mission.survey-grid",
        &with(serde_json::json!({"direction": format!("{} deg", direction + 90.0)})),
    );
    for key in ["lines", "photos"] {
        assert_eq!(
            num(&cross, &format!("result.{key}")),
            num(&first, &format!("result.{key}")) + num(&second, &format!("result.{key}")),
            "{key}"
        );
    }
    let survey = |r: &Value| num(r, "result.survey_length.value");
    assert!((survey(&cross) - survey(&first) - survey(&second)).abs() < 1e-9);
    // A crosshatch is not a setting to turn off here.
    assert_eq!(
        call(
            "drone.mission.crosshatch",
            &with(serde_json::json!({"crosshatch": "no"}))
        )["ok"],
        false
    );

    // Geofence check: the geofence tool's verdict on the same waypoints.
    let fence = serde_json::json!({
        "area": [{"lat": 41, "lon": -96}, {"lat": 41, "lon": -95.997}, {"lat": 41.002, "lon": -95.997}, {"lat": 41.002, "lon": -96}],
        "distance": "40 m", "warning_distance": "25 m",
        "waypoints": [{"lat": 41.001, "lon": -95.9985}, {"lat": 41.0022, "lon": -95.9985}, {"lat": 41.0026, "lon": -95.9985}, {"lat": 41.00232, "lon": -95.9985}]
    });
    let check = call("drone.mission.geofence-check", &fence.to_string());
    let general = call("drone.mission.geofence", &fence.to_string());
    assert_eq!(check["ok"], true, "{}", check["error"]);
    for key in ["outside_count", "warning_count", "flagged", "max_deviation"] {
        assert_eq!(check["result"][key], general["result"][key], "{key}");
    }
    // 22 m north is inside both fences, 36 m is past the warning fence, 67 m is outside.
    assert_eq!(num(&check, "result.waypoints_checked"), 4.0);
    assert_eq!(num(&check, "result.outside_count"), 1.0);
    assert_eq!(num(&check, "result.warning_count"), 1.0);
    assert!(codes(&check).contains(&"WAYPOINT_OUTSIDE_GEOFENCE".to_owned()));
    assert!(check["result"]["fence"].is_null());
    // The check needs something to check.
    let mut none = fence.clone();
    none.as_object_mut().unwrap().remove("waypoints");
    assert_eq!(
        call("drone.mission.geofence-check", &none.to_string())["ok"],
        false
    );
    assert_eq!(
        call("drone.mission.geofence", &none.to_string())["ok"],
        true
    );
}

#[test]
fn pattern_stats_invariants() {
    let stats = |points: &[(f64, f64)], speed: Option<&str>| {
        let mut input = serde_json::json!({
            "waypoints": points.iter().map(|(lat, lon)| serde_json::json!({"lat": lat, "lon": lon})).collect::<Vec<_>>()
        });
        if let Some(v) = speed {
            input["groundspeed"] = serde_json::json!(v);
        }
        call("drone.mission.pattern-stats", &input.to_string())
    };
    let km = |r: &Value| num(r, "result.path_length.value");
    let box_path = [
        (44.98, -93.27),
        (44.9818, -93.27),
        (44.9818, -93.2662),
        (44.98, -93.2662),
        (44.98, -93.27),
    ];
    let r = stats(&box_path, Some("8 m/s"));
    assert_eq!(r["ok"], true, "{r}");
    assert_eq!(num(&r, "result.legs"), 4.0);
    // A closed box turns three times on the way round and about 270° in all.
    assert_eq!(num(&r, "result.turns"), 3.0);
    assert!((num(&r, "result.total_turning.value") - 270.0).abs() < 0.01);
    // Time is length over speed.
    assert!((num(&r, "result.flight_time.value") * 60.0 * 8.0 / 1000.0 - km(&r)).abs() < 1e-12);
    // The path flown backwards is as long, with the same turns.
    let mut reversed = box_path;
    reversed.reverse();
    let back = stats(&reversed, None);
    assert!((km(&back) - km(&r)).abs() < 1e-12);
    assert_eq!(num(&back, "result.turns"), 3.0);
    assert!(back["result"]["flight_time"].is_null());
    // A straight line has no turns, and its far end is the farthest point.
    let line = stats(&[(0.0, 10.0), (0.0, 10.001), (0.0, 10.002)], None);
    assert_eq!(num(&line, "result.turns"), 0.0);
    assert!((num(&line, "result.farthest_from_start.value") / 1000.0 - km(&line)).abs() < 1e-9);
    // Out and back: twice the leg, one turn of 180°, and the farthest point is the leg.
    let out_back = stats(&[(51.5, -0.12), (51.501, -0.118), (51.5, -0.12)], None);
    assert_eq!(num(&out_back, "result.turns"), 1.0);
    assert!((num(&out_back, "result.total_turning.value") - 180.0).abs() < 1e-6);
    assert!(
        (num(&out_back, "result.farthest_from_start.value") * 2.0 / 1000.0 - km(&out_back)).abs()
            < 1e-9
    );
    // Splitting a leg at its midpoint adds a leg, not length.
    let whole = stats(&[(40.0, -105.0), (40.002, -105.0)], None);
    let split = stats(&[(40.0, -105.0), (40.001, -105.0), (40.002, -105.0)], None);
    assert!((km(&whole) - km(&split)).abs() < 1e-9);
    assert_eq!(num(&split, "result.legs"), 2.0);
    // A repeated waypoint has no course to turn from.
    let repeat = stats(&[(40.0, -105.0), (40.0, -105.0), (40.001, -105.0)], None);
    assert_eq!(repeat["error"]["field"], "/waypoints/1");
}

#[test]
fn one_inch_at_100_m() {
    let r = call(
        "drone.photogrammetry.gsd",
        &format!(r#"{{"height":"100 m",{CAM}}}"#),
    );
    assert!((num(&r, "result.gsd.value") - 2.741).abs() < 5e-4);
    assert!((num(&r, "result.footprint_across.value") - 150.0).abs() < 1e-9);
    assert!(codes(&r).iter().all(|c| c != "EQUIVALENT_FOCAL_LENGTH"));
}

#[test]
fn gsd_context_band_is_cited_and_not_a_verdict() {
    let r = call(
        "drone.photogrammetry.gsd",
        &format!(r#"{{"height":"437.76 m",{CAM}}}"#),
    );
    assert_eq!(
        r["comparison"],
        "12 cm/px: coarser than typical mapping work (1–5 cm/px). Basis: Ruiz et al. (2013), Table 2."
    );
    let references = r["meta"]["references"].as_array().unwrap();
    assert!(references.iter().any(|reference| {
        reference["issuer"]
            .as_str()
            .is_some_and(|text| text.contains("Ruiz"))
            && reference["locator"]
                .as_str()
                .is_some_and(|text| text.contains("Table 2"))
    }));
}

#[test]
fn equivalent_focal_length_suspected() {
    let r = call(
        "drone.photogrammetry.gsd",
        r#"{"height":"100 m","sensor_width":"13.2 mm","sensor_height":"8.8 mm","focal_length":"24 mm","image_width":5472}"#,
    );
    assert!(codes(&r).contains(&"EQUIVALENT_FOCAL_LENGTH".to_owned()));
    let fixed = call(
        "drone.photogrammetry.gsd",
        r#"{"height":"100 m","sensor_width":"13.2 mm","sensor_height":"8.8 mm","focal_length":"24 mm","focal_length_type":"equivalent-35mm","image_width":5472}"#,
    );
    assert!(
        (num(&fixed, "result.focal_length_used.value") - 24.0 * 15.864_425_6 / 43.266_615_3).abs()
            < 1e-3
    );
}

#[test]
fn two_cm_target() {
    let r = call(
        "drone.photogrammetry.altitude-for-gsd",
        &format!(r#"{{"target_gsd":"2 cm",{CAM}}}"#),
    );
    assert!((num(&r, "result.height.value") - 72.96).abs() < 1e-9);
    let high = call(
        "drone.photogrammetry.altitude-for-gsd",
        &format!(r#"{{"target_gsd":"8 cm",{CAM}}}"#),
    );
    assert!(codes(&high).contains(&"ABOVE_ALTITUDE_CEILING".to_owned()));
}

#[test]
fn overlap_75_65_and_camera_too_slow() {
    let base = r#""height":"100 m","sensor_width":"13.2 mm","sensor_height":"8.8 mm","focal_length":"8.8 mm","image_width":5472,"groundspeed":"10 m/s","front_overlap":75,"side_overlap":65"#;
    let r = call("drone.photogrammetry.trigger", &format!("{{{base}}}"));
    assert!((num(&r, "result.trigger_distance.value") - 25.0).abs() < 1e-9);
    assert!((num(&r, "result.trigger_interval.value") - 2.5).abs() < 1e-9);
    assert!((num(&r, "result.line_spacing.value") - 52.5).abs() < 1e-9);
    let slow = call(
        "drone.photogrammetry.trigger",
        &format!(r#"{{{base},"min_interval":"3 s"}}"#),
    );
    assert!(codes(&slow).contains(&"TRIGGER_TOO_FAST".to_owned()));
    assert!((num(&slow, "result.max_groundspeed.value") - 8.333_333).abs() < 1e-5);
}

#[test]
fn forest_preset() {
    let r = call(
        "drone.photogrammetry.trigger",
        r#"{"height":"100 m","sensor_width":"13.2 mm","sensor_height":"8.8 mm","focal_length":"8.8 mm","image_width":5472,"groundspeed":"10 m/s","preset":"forest"}"#,
    );
    assert!((num(&r, "result.trigger_distance.value") - 15.0).abs() < 1e-9);
    // Pix4D: at least 85% frontal and side overlap for forest.
    assert!((num(&r, "result.line_spacing.value") - 22.5).abs() < 1e-9);
}

#[test]
fn blur_at_one_thousandth() {
    let r = call(
        "drone.photogrammetry.motion-blur",
        r#"{"groundspeed":"10 m/s","exposure":"0.001 s","gsd":"2.741 cm"}"#,
    );
    assert!((num(&r, "result.blur") - 0.365).abs() < 5e-4);
    assert_eq!(r["result"]["max_exposure_fraction"], "1/730 s");
}

#[test]
fn motion_blur_invariants() {
    let run = |speed: f64, exposure: f64, gsd_cm: f64, limit: f64| {
        call(
            "drone.photogrammetry.motion-blur",
            &format!(
                r#"{{"groundspeed":"{speed} m/s","exposure":"{exposure} s","gsd":"{gsd_cm} cm","blur_limit":{limit}}}"#
            ),
        )
    };
    let close = |a: f64, b: f64| (a - b).abs() <= 1e-12 * b.abs().max(1.0);
    for (speed, exposure, gsd_cm, limit) in [
        (1.0, 1.0 / 250.0, 5.0, 0.25),
        (8.0, 1.0 / 800.0, 2.0, 0.5),
        (15.0, 1.0 / 2_000.0, 1.0, 1.0),
        (30.0, 1.0 / 4_000.0, 0.5, 2.0),
    ] {
        let base = run(speed, exposure, gsd_cm, limit);
        let fast = run(2.0 * speed, exposure, gsd_cm, limit);
        let long = run(speed, 2.0 * exposure, gsd_cm, limit);
        let coarse = run(speed, exposure, 2.0 * gsd_cm, limit);
        let loose = run(speed, exposure, gsd_cm, 2.0 * limit);
        let blur = num(&base, "result.blur");
        let max = num(&base, "result.max_exposure.value");
        assert!(close(num(&fast, "result.blur"), 2.0 * blur));
        assert!(close(num(&long, "result.blur"), 2.0 * blur));
        assert!(close(num(&coarse, "result.blur"), 0.5 * blur));
        assert!(close(num(&fast, "result.max_exposure.value"), 0.5 * max));
        assert!(close(num(&coarse, "result.max_exposure.value"), 2.0 * max));
        assert!(close(num(&loose, "result.max_exposure.value"), 2.0 * max));
    }
}

#[test]
fn asprs_scenarios() {
    let r = call(
        "drone.photogrammetry.asprs-accuracy",
        r#"{"rmse_z":"1.00 cm","checkpoint_rmse":"2.0 cm","checkpoints":30}"#,
    );
    assert!((num(&r, "result.vertical.value") - 2.24).abs() < 0.005);
    let few = call(
        "drone.photogrammetry.asprs-accuracy",
        r#"{"rmse_z":"1 cm","checkpoint_rmse":"1 cm","checkpoints":20}"#,
    );
    assert!(codes(&few).contains(&"INSUFFICIENT_CHECKPOINTS".to_owned()));
    assert!(
        few["result"]["checkpoint_status"]
            .as_str()
            .unwrap()
            .contains("30")
    );
}

#[test]
fn gsd_invariants() {
    // GSD is linear in height, the footprint is GSD × pixels, and the height
    // for a target GSD inverts the GSD tool exactly.
    let cams = [
        (13.2, 8.8, 8.8, 5472, 3648),
        (17.3, 13.0, 12.29, 5280, 3956),
        (6.17, 4.55, 4.5, 4000, 3000),
        (35.9, 24.0, 35.0, 8192, 5460),
    ];
    let v = |r: &Value, k: &str| num(r, &format!("result.{k}.value"));
    for (w, h, f, iw, ih) in cams {
        let cam = serde_json::json!({"sensor_width": format!("{w} mm"), "sensor_height": format!("{h} mm"),
            "focal_length": format!("{f} mm"), "image_width": iw, "image_height": ih});
        let gsd = |m: f64| {
            let mut i = cam.clone();
            i["height"] = serde_json::json!(format!("{m} m"));
            call("drone.photogrammetry.gsd", &i.to_string())
        };
        for m in [10.0, 45.0, 100.0, 120.0, 400.0] {
            let r = gsd(m);
            let g = v(&r, "gsd");
            assert!((v(&gsd(2.0 * m), "gsd") - 2.0 * g).abs() < 1e-12 * g.max(1.0));
            assert!((v(&r, "footprint_across") - g / 100.0 * f64::from(iw)).abs() < 1e-9);
            assert!(
                (v(&r, "footprint_along") - v(&r, "gsd_along") / 100.0 * f64::from(ih)).abs()
                    < 1e-9
            );
            let mut i = cam.clone();
            i["target_gsd"] = serde_json::json!(format!("{g} cm"));
            let back = call("drone.photogrammetry.altitude-for-gsd", &i.to_string());
            assert!((v(&back, "height") - m).abs() < 1e-9 * m, "{back}");
        }
    }
}

#[test]
fn oblique_45_degrees_is_coarser_than_nadir_and_a_trapezoid() {
    let r = call(
        "drone.photogrammetry.oblique-gsd",
        &format!(r#"{{{CAM},"height":"100 m","pitch":"45 deg"}}"#),
    );
    let (c, n) = (
        num(&r, "result.gsd_center.value"),
        num(&r, "result.gsd_nadir.value"),
    );
    assert!(c > n, "center {c} should be coarser than nadir {n}");
    // Across at the center is nadir ÷ cos θ; along, ÷ cos² θ.
    assert!((c - n * 2f64.sqrt()).abs() < 1e-9);
    assert!(num(&r, "result.gsd_center_along.value") > c);
    let fp = &r["result"]["footprint"];
    let x = |i: usize| fp[i]["right"]["value"].as_f64().unwrap();
    let w = |a: usize, b: usize| x(b) - x(a);
    assert!(w(3, 2) > w(0, 1), "the far edge is wider: a trapezoid");
    assert!(codes(&r).iter().all(|c| c != "BEYOND_HORIZON"));
}

#[test]
fn oblique_gsd_invariants() {
    let run = |height: f64, pitch: f64| {
        call(
            "drone.photogrammetry.oblique-gsd",
            &format!(r#"{{{CAM},"height":"{height} m","pitch":"{pitch} deg"}}"#),
        )
    };
    let value = |r: &Value, name: &str| num(r, &format!("result.{name}.value"));

    let nadir = run(100.0, 0.0);
    let g = value(&nadir, "gsd_nadir");
    assert!((value(&nadir, "gsd_center") - g).abs() < 1e-12);
    assert!((value(&nadir, "gsd_center_along") - g).abs() < 1e-9);
    assert_eq!(
        value(&nadir, "near_distance"),
        -value(&nadir, "far_distance")
    );

    let mut previous = g;
    for pitch in [10.0, 30.0, 45.0, 55.0] {
        let r = run(100.0, pitch);
        let center = value(&r, "gsd_center_along");
        assert!(center > previous, "{pitch}°: {r}");
        assert!(value(&r, "gsd_near") < center, "{pitch}°: {r}");
        assert!(value(&r, "gsd_far") > center, "{pitch}°: {r}");
        previous = center;

        let twice = run(200.0, pitch);
        for name in ["gsd_center", "gsd_center_along", "gsd_near", "gsd_far"] {
            assert!(
                (value(&twice, name) - 2.0 * value(&r, name)).abs() < 1e-9,
                "{pitch}° {name}"
            );
        }
    }
}

#[test]
fn hill_reduces_overlap() {
    let r = call(
        "drone.photogrammetry.terrain-overlap",
        r#"{"height":"100 m","highest_terrain":"40 m","front_overlap":75}"#,
    );
    assert_eq!(num(&r, "result.effective_height.value"), 60.0);
    let f = num(&r, "result.front_overlap_worst");
    assert!((f - 58.333_333_333).abs() < 1e-6, "{f}");
    assert!(codes(&r).iter().any(|c| c == "OVERLAP_BELOW_TARGET"));
    let msg = r["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["code"] == "OVERLAP_BELOW_TARGET")
        .and_then(|w| w["message"].as_str())
        .unwrap();
    assert!(msg.starts_with("Front overlap falls to 58.3%"), "{msg}");
}

#[test]
fn a_vanishing_focal_length_is_refused_not_a_crash() {
    // Found by the fuzzer: a 1e-12 mm focal length trapped the module.
    let r = call(
        "drone.photogrammetry.oblique-gsd",
        r#"{"focal_length":1e-12,"height":"100 m","image_height":3648,"image_width":5472,"pitch":"45 deg","sensor_width":"13.2 mm"}"#,
    );
    assert_eq!(r["ok"], false, "{r}");
    assert_ne!(r["error"]["code"], "INTERNAL", "{r}");
}

#[test]
fn trigger_invariants() {
    // Spacing is the footprint times (1 − overlap), the footprint is the GSD
    // tool's, everything scales linearly with height, the interval times the
    // groundspeed is the trigger distance, portrait swaps the two axes, and
    // the camera check fires exactly when the interval is under the minimum.
    let cams = [
        (13.2, 8.8, 8.8, 5472),
        (17.3, 13.0, 12.29, 5280),
        (6.17, 4.55, 4.5, 4000),
        (35.9, 24.0, 35.0, 8192),
    ];
    let v = |r: &Value, k: &str| num(r, &format!("result.{k}.value"));
    let close = |a: f64, b: f64| (a - b).abs() <= 1e-9 * b.abs().max(1.0);
    for (w, h, f, iw) in cams {
        let run = |m: f64, gs: f64, fo: f64, so: f64, extra: &str| {
            call(
                "drone.photogrammetry.trigger",
                &format!(
                    r#"{{"sensor_width":"{w} mm","sensor_height":"{h} mm","focal_length":"{f} mm","image_width":{iw},"height":"{m} m","groundspeed":"{gs} m/s","front_overlap":{fo},"side_overlap":{so}{extra}}}"#
                ),
            )
        };
        for m in [20.0, 60.0, 100.0, 120.0] {
            let gsd = call(
                "drone.photogrammetry.gsd",
                &format!(
                    r#"{{"sensor_width":"{w} mm","sensor_height":"{h} mm","focal_length":"{f} mm","image_width":{iw},"height":"{m} m"}}"#
                ),
            );
            for (fo, so) in [(0.0, 0.0), (60.0, 30.0), (75.0, 65.0), (85.0, 70.0)] {
                let r = run(m, 10.0, fo, so, "");
                let (along, across) = (v(&r, "footprint_along"), v(&r, "footprint_across"));
                assert!(close(across, v(&gsd, "footprint_across")), "{r}");
                assert!(close(along, v(&gsd, "footprint_along")), "{r}");
                assert!(close(v(&r, "trigger_distance"), along * (1.0 - fo / 100.0)));
                assert!(close(v(&r, "line_spacing"), across * (1.0 - so / 100.0)));
                assert!(close(
                    v(&r, "trigger_interval") * 10.0,
                    v(&r, "trigger_distance")
                ));
                // Linear in height, inverse in groundspeed.
                let hi = run(2.0 * m, 10.0, fo, so, "");
                assert!(close(
                    v(&hi, "trigger_distance"),
                    2.0 * v(&r, "trigger_distance")
                ));
                assert!(close(v(&hi, "line_spacing"), 2.0 * v(&r, "line_spacing")));
                let fast = run(m, 20.0, fo, so, "");
                assert!(close(
                    2.0 * v(&fast, "trigger_interval"),
                    v(&r, "trigger_interval")
                ));
                // Portrait: the long side runs along the track.
                let p = run(m, 10.0, fo, so, r#","orientation":"portrait""#);
                assert!(close(v(&p, "footprint_along"), across));
                assert!(close(v(&p, "footprint_across"), along));
                // The camera check: max speed × minimum interval = distance.
                let t = v(&r, "trigger_interval");
                for min in [0.5 * t, 2.0 * t] {
                    let c = run(m, 10.0, fo, so, &format!(r#","min_interval":"{min} s""#));
                    assert!(close(
                        v(&c, "max_groundspeed") * min,
                        v(&r, "trigger_distance")
                    ));
                    let warned = codes(&c).iter().any(|x| x == "TRIGGER_TOO_FAST");
                    assert_eq!(warned, t < min, "{c}");
                }
            }
            // More overlap never spreads photos or lines further apart.
            let mut last = (f64::INFINITY, f64::INFINITY);
            for o in [0.0, 30.0, 50.0, 60.0, 70.0, 80.0, 90.0, 99.0] {
                let r = run(m, 10.0, o, o, "");
                let now = (v(&r, "trigger_distance"), v(&r, "line_spacing"));
                assert!(now.0 < last.0 && now.1 < last.1);
                last = now;
            }
        }
    }
}

#[test]
fn terrain_overlap_invariants() {
    // Flat ground keeps the plan, rising ground only lowers overlap, the
    // result is the trigger tool's spacing over the footprint at h − t, it is
    // scale-free in (h, t), the GSD shrinks by (h − t)/h, and flying at the
    // reported height holds the accepted overlap exactly.
    let v = |r: &Value, k: &str| num(r, &format!("result.{k}.value"));
    let pct = |r: &Value, k: &str| num(r, &format!("result.{k}"));
    let close = |a: f64, b: f64| (a - b).abs() <= 1e-9 * b.abs().max(1.0);
    let cam = r#""sensor_width":"13.2 mm","focal_length":"8.8 mm","image_width":5472"#;
    let run = |h: f64, t: f64, fo: f64, so: f64, extra: &str| {
        call(
            "drone.photogrammetry.terrain-overlap",
            &format!(
                r#"{{"height":"{h} m","highest_terrain":"{t} m","front_overlap":{fo},"side_overlap":{so}{extra}}}"#
            ),
        )
    };
    for (fo, so) in [(60.0, 30.0), (75.0, 65.0), (80.0, 70.0), (85.0, 75.0)] {
        for h in [45.0, 100.0, 120.0] {
            let flat = run(h, 0.0, fo, so, "");
            assert!(close(pct(&flat, "front_overlap_worst"), fo));
            assert!(close(pct(&flat, "side_overlap_worst"), so));
            assert!(codes(&flat).iter().all(|c| c != "OVERLAP_BELOW_TARGET"));
            let plan = call(
                "drone.photogrammetry.trigger",
                &format!(
                    r#"{{{cam},"sensor_height":"8.8 mm","height":"{h} m","groundspeed":"10 m/s","front_overlap":{fo},"side_overlap":{so}}}"#
                ),
            );
            let mut last = (f64::INFINITY, f64::INFINITY);
            for k in [-0.4, -0.1, 0.1, 0.3, 0.5, 0.7, 0.9] {
                let t = k * h;
                let r = run(h, t, fo, so, "");
                let now = (
                    pct(&r, "front_overlap_worst"),
                    pct(&r, "side_overlap_worst"),
                );
                assert!(now.0 < last.0 && now.1 < last.1, "{r}");
                last = now;
                assert_eq!(
                    codes(&r).iter().any(|c| c == "OVERLAP_BELOW_TARGET"),
                    t > 0.0,
                    "{r}"
                );
                // The same answer from the trigger tool's spacing at h and the
                // GSD tool's footprint at h − t.
                let there = call(
                    "drone.photogrammetry.gsd",
                    &format!(
                        r#"{{{cam},"sensor_height":"8.8 mm","height":"{} m"}}"#,
                        h - t
                    ),
                );
                let o = |spacing: f64, fp: f64| 100.0 * (1.0 - spacing / fp);
                let front = o(v(&plan, "trigger_distance"), v(&there, "footprint_along"));
                let side = o(v(&plan, "line_spacing"), v(&there, "footprint_across"));
                assert!((now.0 - front).abs() < 1e-9, "{now:?} {front}");
                assert!((now.1 - side).abs() < 1e-9, "{now:?} {side}");
                // Scale-free: the same ratio of terrain to height, the same overlap.
                let big = run(3.0 * h, 3.0 * t, fo, so, "");
                assert!(close(pct(&big, "front_overlap_worst"), now.0));
                // The GSD over the high ground shrinks by (h − t)/h.
                let g = run(h, t, fo, so, &format!(",{cam}"));
                assert!(close(
                    v(&g, "gsd_worst"),
                    v(&g, "gsd_takeoff") * (h - t) / h
                ));
            }
            // Flying at the reported height holds the accepted overlap.
            let t = 0.3 * h;
            let target = so - 10.0;
            let r = run(h, t, fo, so, &format!(r#","target_overlap":{target}"#));
            let up = v(&r, "min_height");
            let again = run(up, t, fo, so, "");
            let worst = pct(&again, "front_overlap_worst").min(pct(&again, "side_overlap_worst"));
            assert!((worst - target).abs() < 1e-9, "{again}");
        }
    }
}

#[test]
fn checkpoint_requirements_invariants() {
    let run = |area: f64, vegetated: f64, h: &str, v: &str| -> Value {
        let r: Value = serde_json::from_str(&REGISTRY.invoke(
            "drone.photogrammetry.checkpoint-requirements",
            &serde_json::json!({"area_size": format!("{area} km2"), "vegetated": vegetated, "horizontal_class": h, "vertical_class": v}).to_string(),
        ))
        .unwrap();
        assert_eq!(r["ok"], true, "{r}");
        r["result"].clone()
    };
    for vegetated in [0.0, 25.0, 60.0, 100.0] {
        let mut last = 0.0;
        for area in [0.5, 10.0, 499.0, 500.0, 501.0, 750.0, 1200.0, 2500.0] {
            let r = run(area, vegetated, "5 cm", "10 cm");
            // The count never falls as the area grows.
            let n = r["checkpoints"].as_f64().unwrap();
            assert!(n >= last, "{area} km² at {vegetated}%: {n} after {last}");
            last = n;
            // A wholly open project has no vegetated checkpoints.
            if vegetated == 0.0 {
                assert!(
                    r["checkpoints_vegetated"].is_null() || r["checkpoints_vegetated"] == 0.0,
                    "{r}"
                );
            }
            // Checkpoints at least twice as accurate as the product, at any area.
            assert_eq!(r["checkpoint_rmse_h"]["value"], 2.5);
            assert_eq!(r["checkpoint_rmse_v"]["value"], 5.0);
        }
    }
}
