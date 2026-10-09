//! The spiral-curve-spiral layout in plan closes on itself.

use gp_survey::REGISTRY;
use serde_json::{Value, json};

fn run(ls: f64, r: f64, delta: f64) -> Value {
    let v: Value = serde_json::from_str(
        &REGISTRY.invoke(
            "survey.curves.spiral",
            &json!({"spiral_length": format!("{ls} ft"), "radius": format!("{r} ft"),
            "delta": format!("{delta} deg"), "pi_station": "50+00"})
            .to_string(),
        ),
    )
    .unwrap();
    assert_eq!(v["ok"], true, "{v}");
    v["result"].clone()
}

#[test]
fn the_layout_closes_and_meets_both_tangents() {
    for (ls, r, delta) in [
        (200.0, 1000.0, 40.0),
        (300.0, 600.0, 75.0),
        (100.0, 3000.0, 10.0),
        (250.0, 500.0, 120.0),
    ] {
        let res = run(ls, r, delta);
        let pts: Vec<(f64, f64, String)> = res["layout"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                (
                    p["x"]["value"].as_f64().unwrap(),
                    p["y"]["value"].as_f64().unwrap(),
                    p["point"].as_str().unwrap().to_owned(),
                )
            })
            .collect();
        let at = |name: &str| pts.iter().position(|p| p.2 == name).unwrap();
        let (ts, sc, cs, st) = (at("TS"), at("SC"), at("CS"), at("ST"));
        assert_eq!((ts, st), (0, pts.len() - 1));
        assert!(ts < sc && sc < cs && cs < st);
        let gap = |i: usize| (pts[i + 1].0 - pts[i].0).hypot(pts[i + 1].1 - pts[i].1);
        // Every spiral step is Ls / 24 of curve, a chord just shorter; every
        // arc step the same chord of the arc. A misplaced arc or exit spiral
        // (a wrong p, k, or Ts) would leave a gap or an overlap at SC or CS.
        let step = ls / 24.0;
        for i in (ts..sc).chain(cs..st) {
            assert!(
                (gap(i) - step).abs() < 1e-3 * step,
                "{ls}/{r}/{delta}: spiral step {i} is {}",
                gap(i)
            );
        }
        let arc = gap(sc);
        for i in sc..cs {
            assert!(
                (gap(i) - arc).abs() < 1e-6 * arc,
                "{ls}/{r}/{delta}: arc step {i}"
            );
        }
        // TS and ST stand the total tangent from the PI, which is on the back
        // tangent; the ST is on the ahead tangent, deflected by delta.
        let t = res["total_tangent"]["value"].as_f64().unwrap();
        let (px, py) = (t, 0.0);
        assert!(pts[ts].0.abs() < 1e-9 && pts[ts].1.abs() < 1e-9);
        let (ex, ey) = (pts[st].0 - px, pts[st].1 - py);
        assert!((ex.hypot(ey) - t).abs() < 1e-6 * t, "{ls}/{r}/{delta}");
        assert!((ey.atan2(ex).to_degrees() - delta).abs() < 1e-6);
        // The SC is the published X and Y.
        assert!((pts[sc].0 - res["x"]["value"].as_f64().unwrap()).abs() < 1e-9);
        assert!((pts[sc].1 - res["y"]["value"].as_f64().unwrap()).abs() < 1e-9);
    }
}
