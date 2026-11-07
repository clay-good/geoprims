//! 3D conformal fit (add-survey-partition-and-adjustment, "3D conformal fit
//! from common points").

use gp_survey::REGISTRY;
use serde_json::{Value, json};

const ARCSEC: f64 = 206_264.806_247_096_36;

/// The IOGP linearized Helmert transformation, as `geodesy.datum.helmert` applies it.
fn transform(p: [f64; 7], v: [f64; 3], cf: bool) -> [f64; 3] {
    let s = if cf { -1.0 } else { 1.0 };
    let (rx, ry, rz) = (s * p[4] / ARCSEC, s * p[5] / ARCSEC, s * p[6] / ARCSEC);
    let k = 1.0 + p[3] * 1e-6;
    [
        p[0] + k * (v[0] - rz * v[1] + ry * v[2]),
        p[1] + k * (rz * v[0] + v[1] - rx * v[2]),
        p[2] + k * (-ry * v[0] + rx * v[1] + v[2]),
    ]
}

fn fit(points: &[([f64; 3], [f64; 3])], cf: bool) -> Value {
    let rows: Vec<Value> = points
        .iter()
        .enumerate()
        .map(|(i, (s, t))| {
            json!({"name": format!("P{i}"), "x": format!("{} m", s[0]), "y": format!("{} m", s[1]), "z": format!("{} m", s[2]),
                   "x_target": format!("{} m", t[0]), "y_target": format!("{} m", t[1]), "z_target": format!("{} m", t[2])})
        })
        .collect();
    let input = json!({"points": rows, "convention": if cf { "coordinate-frame" } else { "position-vector" }});
    let r: Value =
        serde_json::from_str(&REGISTRY.invoke("survey.cogo.helmert-3d-fit", &input.to_string()))
            .unwrap();
    assert_eq!(r["ok"], true, "{r}");
    r["result"].clone()
}

#[test]
fn helmert_3d_fit_invariants() {
    // Ten synthetic points in a local system, both conventions: every
    // parameter comes back within 1e-9.
    let p = [2.5, -1.25, 0.75, 4.2, 12.0, -7.5, 30.0];
    let src: Vec<[f64; 3]> = (0..10)
        .map(|i| {
            let t = f64::from(i);
            [
                37.0 * t - 150.0,
                (t * 1.7).sin() * 300.0,
                (t * 0.9).cos() * 250.0 + 40.0,
            ]
        })
        .collect();
    for cf in [false, true] {
        let pts: Vec<_> = src.iter().map(|s| (*s, transform(p, *s, cf))).collect();
        let r = fit(&pts, cf);
        for (k, want) in ["tx", "ty", "tz", "scale", "rx", "ry", "rz"].iter().zip(p) {
            let got = r[*k]["value"].as_f64().unwrap();
            assert!((got - want).abs() < 1e-9, "{k}: {got} vs {want} ({cf})");
        }
        assert!(r["rms"]["value"].as_f64().unwrap() < 1e-6, "rms in mm");
    }
    // Noise on one point shows in its residual, and the fit, applied to the
    // sources, reproduces every reported residual.
    let mut pts: Vec<_> = src.iter().map(|s| (*s, transform(p, *s, false))).collect();
    pts[3].1[2] += 0.05;
    let r = fit(&pts, false);
    let q: [f64; 7] =
        ["tx", "ty", "tz", "scale", "rx", "ry", "rz"].map(|k| r[k]["value"].as_f64().unwrap());
    let mut ss = 0.0;
    for (s, t) in &pts {
        let f = transform(q, *s, false);
        ss += (0..3).map(|a| (f[a] - t[a]).powi(2)).sum::<f64>();
    }
    let rms_mm = (ss / 30.0).sqrt() * 1000.0;
    assert!(
        (rms_mm - r["rms"]["value"].as_f64().unwrap()).abs() < 1e-6,
        "{rms_mm}"
    );
    let worst = r["residuals"][3]["residual"].as_str().unwrap();
    assert!(worst.contains("mm"), "{worst}");
}

/// Systems turned by degrees are outside the small-angle model, and the tool
/// says so rather than reporting the least-squares answer of a model that far
/// from the truth. The case is Deakin's (2007, "3D Coordinate Transformations",
/// Appendix): a plane figure ABC whose survey and design systems differ by
/// rotations of about 130°, 86°, and 190°.
#[test]
fn rotations_past_a_degree_are_refused() {
    let pts = [
        ([1000.0, 1000.0, 1000.0], [1911.9, 1435.2, 554.1]),
        ([1620.0, 740.0, 340.0], [2540.6, 1668.1, 1216.1]),
        ([1100.0, 1200.0, 1300.0], [1547.5, 1396.7, 629.8]),
    ];
    let rows: Vec<Value> = pts
        .iter()
        .enumerate()
        .map(|(i, (s, t))| {
            json!({"name": format!("P{i}"), "x": format!("{} m", s[0]), "y": format!("{} m", s[1]), "z": format!("{} m", s[2]),
                   "x_target": format!("{} m", t[0]), "y_target": format!("{} m", t[1]), "z_target": format!("{} m", t[2])})
        })
        .collect();
    let r: Value = serde_json::from_str(&REGISTRY.invoke(
        "survey.cogo.helmert-3d-fit",
        &json!({"points": rows}).to_string(),
    ))
    .unwrap();
    assert_eq!(r["ok"], false, "{r}");
    assert_eq!(r["error"]["code"], "NO_SOLUTION", "{r}");
    assert!(
        r["error"]["message"]
            .as_str()
            .unwrap()
            .contains("one degree"),
        "{r}"
    );
    // A turn well inside the model still answers: half a degree about Z.
    let small = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1800.0];
    let src: Vec<[f64; 3]> = (0..5)
        .map(|i| {
            let t = f64::from(i);
            [
                100.0 * t - 200.0,
                (t * 2.1).sin() * 180.0,
                (t * 1.3).cos() * 90.0,
            ]
        })
        .collect();
    let turned: Vec<_> = src
        .iter()
        .map(|s| (*s, transform(small, *s, false)))
        .collect();
    let ok = fit(&turned, false);
    assert!(
        (ok["rz"]["value"].as_f64().unwrap() - 1800.0).abs() < 1e-6,
        "{ok}"
    );
}
