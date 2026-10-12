//! Polygon overlay against GEOS on the canonical JTS area/area geometries.
//!
//! The fixture is generated from the four JTS 1.20.0 area-overlay XML suites
//! by `tools/vectors/gen_overlay_jts.py`. GEOS 3.15.0 independently runs all
//! four operations on each polygon pair. This test compares area, separate
//! parts, holes, and first moments both in the planar engine and through the
//! public tool contract after an affine move into valid latitude/longitude.

use gp_geo::buffer::{Op, boolean};
use gp_geometry::REGISTRY;
use serde_json::{Value, json};

const OPS: [(&str, &str, Op); 4] = [
    ("intersection", "intersection", Op::Intersection),
    ("union", "union", Op::Union),
    ("difference", "difference", Op::Difference),
    (
        "symDifference",
        "symmetric-difference",
        Op::SymmetricDifference,
    ),
];

#[derive(Debug)]
struct Summary {
    area: f64,
    parts: usize,
    holes: usize,
    moment_x: f64,
    moment_y: f64,
}

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/overlay_jts_geos.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("JSON")
}

fn rings(value: &Value) -> Vec<Vec<(f64, f64)>> {
    value
        .as_array()
        .expect("rings")
        .iter()
        .map(|ring| {
            ring.as_array()
                .expect("ring")
                .iter()
                .map(|point| {
                    let point = point.as_array().expect("point");
                    (point[0].as_f64().expect("x"), point[1].as_f64().expect("y"))
                })
                .collect()
        })
        .collect()
}

fn ring_terms(ring: &[(f64, f64)]) -> (f64, f64, f64) {
    let mut area2 = 0.0;
    let mut mx6 = 0.0;
    let mut my6 = 0.0;
    for i in 0..ring.len() {
        let (x1, y1) = ring[i];
        let (x2, y2) = ring[(i + 1) % ring.len()];
        let cross = x1 * y2 - x2 * y1;
        area2 += cross;
        mx6 += (x1 + x2) * cross;
        my6 += (y1 + y2) * cross;
    }
    (area2 / 2.0, mx6 / 6.0, my6 / 6.0)
}

fn actual_summary(rings: &[Vec<(f64, f64)>]) -> Summary {
    let terms: Vec<_> = rings.iter().map(|ring| ring_terms(ring)).collect();
    Summary {
        area: terms.iter().map(|term| term.0).sum(),
        parts: terms.iter().filter(|term| term.0 > 0.0).count(),
        holes: terms.iter().filter(|term| term.0 < 0.0).count(),
        moment_x: terms.iter().map(|term| term.1).sum(),
        moment_y: terms.iter().map(|term| term.2).sum(),
    }
}

fn expected_summary(parts: &Value) -> Summary {
    let mut summary = Summary {
        area: 0.0,
        parts: 0,
        holes: 0,
        moment_x: 0.0,
        moment_y: 0.0,
    };
    for part in parts.as_array().expect("parts") {
        summary.parts += 1;
        for (ring_index, ring) in rings(part).iter().enumerate() {
            let (area, mx, my) = ring_terms(ring);
            let sign = if ring_index == 0 { 1.0 } else { -1.0 };
            let orientation = sign * area.signum();
            summary.area += orientation * area;
            summary.moment_x += orientation * mx;
            summary.moment_y += orientation * my;
            summary.holes += usize::from(ring_index > 0);
        }
    }
    summary
}

fn close(got: f64, expected: f64, scale: f64) -> bool {
    (got - expected).abs() <= 1e-8 * scale.max(expected.abs()).max(1.0)
}

fn compare(label: &str, got: &Summary, expected: &Summary, span: f64, wrong: &mut Vec<String>) {
    let area_scale = span * span;
    let moment_scale = area_scale * span;
    if got.parts != expected.parts
        || got.holes != expected.holes
        || !close(got.area, expected.area, area_scale)
        || !close(got.moment_x, expected.moment_x, moment_scale)
        || !close(got.moment_y, expected.moment_y, moment_scale)
    {
        wrong.push(format!("{label}: got {got:?}; GEOS {expected:?}"));
    }
}

fn extent(a: &[Vec<(f64, f64)>], b: &[Vec<(f64, f64)>]) -> (f64, f64, f64) {
    let mut bounds = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for &(x, y) in a.iter().chain(b).flatten() {
        bounds.0 = bounds.0.min(x);
        bounds.1 = bounds.1.min(y);
        bounds.2 = bounds.2.max(x);
        bounds.3 = bounds.3.max(y);
    }
    let span = (bounds.2 - bounds.0).max(bounds.3 - bounds.1).max(1.0);
    (
        (bounds.0 + bounds.2) / 2.0,
        (bounds.1 + bounds.3) / 2.0,
        span,
    )
}

fn rows(shape: &[Vec<(f64, f64)>], cx: f64, cy: f64, factor: f64) -> Value {
    Value::Array(
        shape
            .iter()
            .enumerate()
            .flat_map(|(ring, points)| {
                points.iter().map(move |&(x, y)| {
                    let mut point = json!({
                        "lat": 40.0 + (y - cy) * factor,
                        "lon": -105.0 + (x - cx) * factor,
                    });
                    if ring > 0 {
                        point["ring"] = json!(ring);
                    }
                    point
                })
            })
            .collect(),
    )
}

fn result_rings(result: &Value, cx: f64, cy: f64, factor: f64) -> Vec<Vec<(f64, f64)>> {
    let mut rings = Vec::new();
    let mut key = None;
    for row in result.as_array().expect("result rows") {
        let next = (
            row["part"].as_u64().expect("part"),
            row["ring"].as_u64().expect("ring"),
        );
        if key != Some(next) {
            rings.push(Vec::new());
            key = Some(next);
        }
        let lon = row["lon"]["value"].as_f64().expect("longitude");
        let lat = row["lat"]["value"].as_f64().expect("latitude");
        rings
            .last_mut()
            .expect("ring exists")
            .push(((lon + 105.0) / factor + cx, (lat - 40.0) / factor + cy));
    }
    rings
}

#[test]
fn overlay_matches_geos_on_jts_test_geometries() {
    let fixture = fixture();
    assert_eq!(fixture["source"]["version"], "1.20.0");
    assert_eq!(
        fixture["source"]["commit"],
        "6e95fe82feb986a7aa657f4ffa406d8c290af509"
    );
    assert_eq!(fixture["reference"], "GEOS 3.15.0");
    let cases = fixture["cases"].as_array().expect("cases");
    assert_eq!(cases.len(), 48);

    let mut wrong = Vec::new();
    for case in cases {
        let a = rings(&case["a"]);
        let b = rings(&case["b"]);
        let (cx, cy, span) = extent(&a, &b);
        let factor = 0.01 / span;
        for (fixture_name, tool_name, op) in OPS {
            let expected = expected_summary(&case["results"][fixture_name]);
            let direct = actual_summary(&boolean(&a, &b, op));
            let label = format!("{} {tool_name} direct", case["source"]);
            compare(&label, &direct, &expected, span, &mut wrong);

            let response: Value = serde_json::from_str(
                &REGISTRY.invoke(
                    "geometry.overlay.boolean",
                    &json!({
                        "polygon_a": rows(&a, cx, cy, factor),
                        "polygon_b": rows(&b, cx, cy, factor),
                        "operation": tool_name,
                        "edges": "planar",
                    })
                    .to_string(),
                ),
            )
            .expect("tool JSON");
            if response["ok"] != true {
                wrong.push(format!(
                    "{} {tool_name} public tool failed: {}",
                    case["source"], response["error"]
                ));
                continue;
            }
            let public =
                actual_summary(&result_rings(&response["result"]["result"], cx, cy, factor));
            let label = format!("{} {tool_name} public", case["source"]);
            compare(&label, &public, &expected, span, &mut wrong);
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {} JTS/GEOS differential checks failed:\n{}",
        wrong.len(),
        cases.len() * OPS.len() * 2,
        wrong[..wrong.len().min(20)].join("\n")
    );
}
