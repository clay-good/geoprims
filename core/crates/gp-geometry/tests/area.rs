//! Polygon area against GeographicLib's Planimeter (tools/vectors/gen_area_diff.py):
//! 500 polygons from fields to continents, both orientations, across the
//! antimeridian, and around the poles; plus the catalog lint and vectors.

use std::path::Path;

use gp_base::{manifest, template, vectors};
use gp_geometry::{REGISTRY, TOOLS};
use serde_json::{Value, json};

fn repo(path: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(path);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

fn call(id: &str, input: &str) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, input)).expect("envelope is JSON")
}

#[test]
fn matches_planimeter() {
    let text = include_str!("data/planimeter_diff.txt");
    let mut lines = text.lines();
    let (mut worst_area, mut worst_per, mut n) = (0.0f64, 0.0f64, 0);
    while let Some(head) = lines.next() {
        let h: Vec<f64> = head
            .split_whitespace()
            .map(|x| x.parse().unwrap())
            .collect();
        let rows: Vec<Value> = (0..h[0] as usize)
            .map(|_| {
                let p: Vec<f64> = lines
                    .next()
                    .unwrap()
                    .split_whitespace()
                    .map(|x| x.parse().unwrap())
                    .collect();
                json!({"lat": p[0], "lon": p[1]})
            })
            .collect();
        let r = call(
            "geometry.area.polygon",
            &json!({ "polygon": rows }).to_string(),
        );
        let area = r["result"]["area"]["value"]
            .as_f64()
            .unwrap_or_else(|| panic!("{r}"))
            * 1e6;
        let per = r["result"]["perimeter"]["value"].as_f64().unwrap() * 1000.0;
        worst_area = worst_area.max((area - h[2].abs()).abs() / h[2].abs().max(1.0));
        worst_per = worst_per.max((per - h[1]).abs() / h[1]);
        let ccw = r["result"]["orientation"] == "counterclockwise";
        assert_eq!(ccw, h[2] > 0.0, "orientation {r}");
        n += 1;
    }
    println!("{n} polygons: area {worst_area:e} relative, perimeter {worst_per:e} relative");
    assert_eq!(n, 500);
    assert!(worst_area <= 1e-8, "area {worst_area}");
    assert!(worst_per <= 1e-10, "perimeter {worst_per}");
}

#[test]
fn catalog_lint() {
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
    // Related tools in other modules.
    let external = [
        "survey.cogo.area-by-coordinates",
        "navigation.geodesic.inverse",
        "navigation.geodesic.waypoints",
    ];
    let errs = manifest::lint(TOOLS, &taxonomy, &external);
    assert!(errs.is_empty(), "{}", errs.join("\n"));
}

#[test]
fn examples_and_vectors() {
    let mut failures = Vec::new();
    for t in TOOLS {
        for ex in t.examples {
            let r = call(t.id, ex.input);
            let s = r["summary"].as_str().unwrap_or_default();
            if r["ok"] != true || template::grade(s) > 8.0 || s.len() > template::MAX_CHARS {
                failures.push(format!("{} example: {r}", t.id));
            }
        }
        let text = repo(&format!("core/vectors/{}.jsonl", t.id));
        failures.extend(vectors::lint(t.id, &text));
        failures.extend(vectors::run(&REGISTRY, t.id, &text));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn spec_scenarios() {
    // A ring at 80° N: the polar cap, not the rest of the Earth.
    let cap: Vec<Value> = (0..12)
        .map(|k| json!({"lat": 80, "lon": -180 + 30 * k}))
        .collect();
    let r = call(
        "geometry.area.polygon",
        &json!({ "polygon": cap }).to_string(),
    );
    assert_eq!(r["result"]["pole"], "north");
    assert!(r["result"]["area"]["value"].as_f64().unwrap() < 5e6, "{r}");
    // Holes that cover more than the outline are refused.
    let bad = json!({"polygon": [
        {"lat": 0, "lon": 0}, {"lat": 0, "lon": 1}, {"lat": 1, "lon": 1},
        {"lat": -1, "lon": -1, "ring": 1}, {"lat": -1, "lon": 2, "ring": 1}, {"lat": 2, "lon": 2, "ring": 1}, {"lat": 2, "lon": -1, "ring": 1}
    ]});
    assert_eq!(
        call("geometry.area.polygon", &bad.to_string())["error"]["code"],
        "INVALID_INPUT"
    );
    let two =
        json!({"polygon": [{"lat": 0, "lon": 0}, {"lat": 0, "lon": 1}, {"lat": 0, "lon": 0}]});
    assert_eq!(
        call("geometry.area.polygon", &two.to_string())["error"]["code"],
        "INVALID_INPUT"
    );
}

#[test]
fn a_bow_tie_has_no_single_area_and_is_sent_to_the_repair_tool() {
    let r = call(
        "geometry.area.polygon",
        r#"{"polygon":[{"lat":40.0,"lon":-105.0},{"lat":40.004,"lon":-104.995},{"lat":40.0,"lon":-104.995},{"lat":40.004,"lon":-105.0}]}"#,
    );
    assert_eq!(r["error"]["code"], "DEGENERATE_GEOMETRY", "{r}");
    assert!(
        r["error"]["message"].as_str().unwrap().contains("40.002"),
        "{r}"
    );
    assert!(
        r["error"]["hint"]
            .as_str()
            .unwrap()
            .contains("geometry.validity.make-valid")
    );
    // The same corners in a sensible order are a plain rectangle.
    let ok = call(
        "geometry.area.polygon",
        r#"{"polygon":[{"lat":40.0,"lon":-105.0},{"lat":40.0,"lon":-104.995},{"lat":40.004,"lon":-104.995},{"lat":40.004,"lon":-105.0}]}"#,
    );
    assert_eq!(ok["ok"], true, "{ok}");
}

#[test]
fn planar_mode_on_degrees_warns_and_points_to_geodesic() {
    let r = call(
        "geometry.area.polygon",
        r#"{"polygon":[{"lat":37,"lon":-109.05},{"lat":41,"lon":-109.05},{"lat":41,"lon":-102.05},{"lat":37,"lon":-102.05}],"edges":"planar"}"#,
    );
    let w = r["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["code"] == "PLANAR_ON_GEOGRAPHIC")
        .unwrap_or_else(|| panic!("{r}"));
    assert!(w["message"].as_str().unwrap().contains("geodesic"), "{w}");
    // Geodesic, the default, has no such warning.
    let g = call(
        "geometry.area.polygon",
        r#"{"polygon":[{"lat":37,"lon":-109.05},{"lat":41,"lon":-109.05},{"lat":41,"lon":-102.05},{"lat":37,"lon":-102.05}]}"#,
    );
    assert!(!g.to_string().contains("PLANAR_ON_GEOGRAPHIC"));
}

/// Layer E for `geometry.area.polygon`. Area does not depend on where the ring
/// starts or which way it is walked, and a polygon cut in two has the area of
/// its parts.
#[test]
fn polygon_area_invariants() {
    let ring = |pts: &[(f64, f64)]| {
        let rows: Vec<String> = pts
            .iter()
            .map(|(la, lo)| format!(r#"{{"lat":{la},"lon":{lo}}}"#))
            .collect();
        format!(r#"{{"polygon":[{}]}}"#, rows.join(","))
    };
    let area = |pts: &[(f64, f64)]| {
        let r = call("geometry.area.polygon", &ring(pts));
        assert_eq!(r["ok"], true, "{r}");
        (
            r["result"]["area"]["value"].as_f64().expect("area"),
            r["result"]["perimeter"]["value"]
                .as_f64()
                .expect("perimeter"),
            r["result"]["orientation"]
                .as_str()
                .expect("orientation")
                .to_owned(),
        )
    };
    // Colorado, as the worked example draws it.
    let colorado = [
        (37.0, -109.05),
        (41.0, -109.05),
        (41.0, -102.05),
        (37.0, -102.05),
    ];
    let (a0, p0, o0) = area(&colorado);

    // Starting the ring at a different vertex changes nothing.
    for start in 1..colorado.len() {
        let mut rotated = colorado[start..].to_vec();
        rotated.extend_from_slice(&colorado[..start]);
        let (a, p, o) = area(&rotated);
        assert!(
            (a - a0).abs() < 1e-6,
            "starting at vertex {start} gives {a}"
        );
        assert!(
            (p - p0).abs() < 1e-6,
            "starting at vertex {start} gives perimeter {p}"
        );
        assert_eq!(o, o0, "starting at vertex {start} changes the orientation");
    }

    // Walking it the other way turns the orientation round and leaves the
    // area and the perimeter alone.
    let mut reversed = colorado;
    reversed.reverse();
    let (a, p, o) = area(&reversed);
    assert!((a - a0).abs() < 1e-6, "reversed gives {a}");
    assert!((p - p0).abs() < 1e-6, "reversed gives perimeter {p}");
    assert_ne!(o, o0, "reversed keeps the same orientation");

    // Cut along 39 N: the halves have the area of the whole.
    let north = [
        (39.0, -109.05),
        (41.0, -109.05),
        (41.0, -102.05),
        (39.0, -102.05),
    ];
    let south = [
        (37.0, -109.05),
        (39.0, -109.05),
        (39.0, -102.05),
        (37.0, -102.05),
    ];
    let (an, _, _) = area(&north);
    let (as_, _, _) = area(&south);
    assert!(
        (an + as_ - a0).abs() < 1e-3,
        "the halves are {an} and {as_}, the whole is {a0}"
    );

    // Points strung along a meridian make a ring that retraces its own path,
    // because a meridian is itself a geodesic. That has no single area, and is
    // refused with somewhere to go rather than answered with zero, which is
    // what Planimeter returns for it.
    let r = call(
        "geometry.area.polygon",
        &ring(&[(10.0, 20.0), (11.0, 20.0), (12.0, 20.0)]),
    );
    assert_eq!(r["ok"], false, "a ring along a meridian: {r}");
    assert_eq!(r["error"]["code"], "DEGENERATE_GEOMETRY");
    assert!(
        r["error"]["hint"]
            .as_str()
            .is_some_and(|h| h.contains("make-valid")),
        "{r}"
    );

    // Points along a parallel do not: a parallel is not a geodesic, every edge
    // bows towards the pole, and the long closing edge bows further than the
    // two short hops, which leaves a sliver between them. GeographicLib's
    // Planimeter gives 18218566.2 m2 for this ring, run for this test.
    let (parallel, _, _) = area(&[(10.0, 20.0), (10.0, 21.0), (10.0, 22.0)]);
    assert!(
        (parallel - 18.2185662).abs() < 1e-6,
        "three points on a parallel enclose {parallel}, not the 18.2185662 km2 Planimeter gives"
    );
}
