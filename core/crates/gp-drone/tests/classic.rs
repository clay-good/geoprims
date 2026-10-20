//! The classic photogrammetry tools (add-photogrammetry-classics): what each
//! must do, as the spec's scenarios and invariants state.

use gp_drone::REGISTRY;
use serde_json::{Value, json};

fn call(id: &str, input: Value) -> Value {
    let r: Value = serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).unwrap();
    assert_eq!(r["ok"], true, "{id} {input}: {r}");
    r["result"].clone()
}

fn num(r: &Value, k: &str) -> f64 {
    r[k]["value"]
        .as_f64()
        .or(r[k].as_f64())
        .unwrap_or_else(|| panic!("{k}: {r}"))
}

#[test]
fn photo_scale_invariants() {
    // Scale from a height, then the height from that scale, round-trips.
    let a = call(
        "drone.photogrammetry.photo-scale",
        json!({"focal_length": "152.4 mm", "height": "1829 m", "elevation": "180 m", "photo_distance": "25.4 mm"}),
    );
    let n = num(&a, "scale_number");
    let b = call(
        "drone.photogrammetry.photo-scale",
        json!({"focal_length": "152.4 mm", "scale": n, "elevation": "180 m"}),
    );
    assert!((num(&b, "flying_height") - 1829.0).abs() < 1e-9);
    // Photo distance to ground and back.
    assert!((num(&a, "ground_distance") / n - 0.0254).abs() < 1e-12);
    // The flying height for a GSD, as the Penn State lesson works it.
    let c = call(
        "drone.photogrammetry.photo-scale",
        json!({"focal_length": "100 mm", "gsd": "1 ft", "pixel_pitch": "10 um"}),
    );
    assert!((num(&c, "height_above_ground") - 3048.0).abs() < 0.3048);
    // From the sensor and image width, the GSD is the GSD tool's, bit for bit.
    let cam = json!({"sensor_width": "13.2 mm", "focal_length": "8.8 mm", "image_width": 5472});
    for h in ["35 m", "100 m", "121.92 m", "400 ft"] {
        let mut gsd_in = cam.clone();
        gsd_in["height"] = json!(h);
        let g = call("drone.photogrammetry.gsd", gsd_in.clone());
        let p = call("drone.photogrammetry.photo-scale", gsd_in);
        assert_eq!(g["gsd"], p["gsd"], "{h}");
    }
}

#[test]
fn relief_displacement_invariants() {
    let base = json!({"flying_height": "535 m", "base_elevation": "259 m"});
    let with = |pairs: &[(&str, Value)]| {
        let mut v = base.clone();
        for (k, x) in pairs {
            v[*k] = x.clone();
        }
        v
    };
    let h = num(
        &call(
            "drone.photogrammetry.relief-displacement",
            with(&[
                ("displacement", json!("54.1 mm")),
                ("radial_distance", json!("121.7 mm")),
            ]),
        ),
        "object_height",
    );
    // The textbook tower, to its printed 0.1 m.
    assert!((h - 122.7).abs() < 0.05, "{h}");
    // Solved each way, the other unknowns come back.
    let d = num(
        &call(
            "drone.photogrammetry.relief-displacement",
            with(&[
                ("radial_distance", json!("121.7 mm")),
                ("object_height", json!(format!("{h} m"))),
            ]),
        ),
        "displacement",
    );
    assert!((d - 54.1).abs() < 1e-9, "{d}");
    let r = num(
        &call(
            "drone.photogrammetry.relief-displacement",
            with(&[
                ("displacement", json!("54.1 mm")),
                ("object_height", json!(format!("{h} m"))),
            ]),
        ),
        "radial_distance",
    );
    assert!((r - 121.7).abs() < 1e-9, "{r}");
    let fh = call(
        "drone.photogrammetry.relief-displacement",
        json!({"displacement": "54.1 mm", "radial_distance": "121.7 mm", "object_height": format!("{h} m"), "base_elevation": "259 m"}),
    );
    assert!((num(&fh, "flying_height") - 535.0).abs() < 1e-9);
}

#[test]
fn parallax_height_invariants() {
    // A zero parallax difference returns the control point's elevation.
    let z = call(
        "drone.photogrammetry.parallax-height",
        json!({"flying_height": "1500 m", "control_parallax": "92.4 mm", "parallax_difference": "0 mm", "control_elevation": "210 m"}),
    );
    assert!((num(&z, "elevation") - 210.0).abs() < 1e-12);
    // The difference form agrees with the absolute form it is derived from:
    // take two points' parallaxes from h = H − f B / p and difference them.
    let (fl, f, b) = (1830.0_f64, 0.1524_f64, 600.0_f64);
    for (hc, ha) in [
        (0.0_f64, 13.3_f64),
        (200.0, 260.5),
        (450.0, 380.0),
        (900.0, 1100.0),
    ] {
        let (pc, pa) = (f * b / (fl - hc), f * b / (fl - ha));
        let r = call(
            "drone.photogrammetry.parallax-height",
            json!({"flying_height": format!("{fl} m"), "control_parallax": format!("{} mm", pc * 1000.0), "parallax_difference": format!("{} mm", (pa - pc) * 1000.0), "control_elevation": format!("{hc} m")}),
        );
        assert!((num(&r, "elevation") - ha).abs() < 1e-6, "{hc} {ha}");
        let abs = call(
            "drone.photogrammetry.parallax-height",
            json!({"flying_height": format!("{fl} m"), "parallax": format!("{} mm", pa * 1000.0), "focal_length": "152.4 mm", "air_base": format!("{b} m")}),
        );
        assert!((num(&abs, "elevation") - ha).abs() < 1e-6, "{ha}");
    }
    // The textbook tree, to its printed 0.1 m.
    let t = call(
        "drone.photogrammetry.parallax-height",
        json!({"flying_height": "915 m", "control_parallax": "88.2 mm", "parallax_difference": "1.3 mm"}),
    );
    assert!((num(&t, "elevation") - 13.3).abs() < 0.05);
}

#[test]
fn stereo_base_invariants() {
    let run = |endlap: f64, sp: &str| {
        call(
            "drone.photogrammetry.stereo-base",
            json!({"height": "10000 ft", "focal_length": "100 mm", "frame_length": "70 mm", "endlap": endlap, "parallax_precision": sp}),
        )
    };
    let (a, b) = (run(60.0, "0.005 mm"), run(80.0, "0.005 mm"));
    // The 60% plan: a 2,800 ft air base.
    assert!((num(&a, "air_base") - 2800.0 * 0.3048).abs() < 1e-6);
    // From 60% to 80% end lap the base-to-height ratio halves and σ_h doubles.
    assert!((num(&b, "base_height_ratio") * 2.0 - num(&a, "base_height_ratio")).abs() < 1e-12);
    assert!((num(&b, "vertical_precision") - 2.0 * num(&a, "vertical_precision")).abs() < 1e-9);
    // σ_h is linear in the parallax precision.
    let c = run(60.0, "0.010 mm");
    assert!((num(&c, "vertical_precision") - 2.0 * num(&a, "vertical_precision")).abs() < 1e-9);
}
