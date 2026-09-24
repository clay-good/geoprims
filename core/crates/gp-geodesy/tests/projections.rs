//! The projection methods with user-set parameters against PROJ
//! (geodesy/projections, "Round-trip and differential accuracy"): 300 random
//! parameter sets and points per method from tools/vectors/gen_projections_proj.py,
//! forward and inverse, and the invariants each method must keep.

use gp_geodesy::REGISTRY;
use serde_json::{Map, Value, json};

const METHODS: [&str; 5] = [
    "web-mercator",
    "lcc",
    "albers",
    "polar-stereographic",
    "equidistant-cylindrical",
];

fn run(tool: &str, input: &Value) -> Value {
    let r: Value = serde_json::from_str(&REGISTRY.invoke(tool, &input.to_string())).unwrap();
    assert_eq!(r["ok"], true, "{tool} {input}: {r}");
    r["result"].clone()
}

fn val(r: &Value, k: &str) -> f64 {
    r[k]["value"]
        .as_f64()
        .or_else(|| r[k].as_f64())
        .unwrap_or_else(|| panic!("{k}: {r}"))
}

fn with(params: &Value, extra: Value) -> Value {
    let mut m: Map<String, Value> = params.as_object().unwrap().clone();
    m.extend(extra.as_object().unwrap().clone());
    Value::Object(m)
}

/// Ground distance between two nearby points, meters (small-angle, sphere of
/// the Earth's radius: good to a part in a thousand, plenty for a tolerance).
fn ground(la1: f64, lo1: f64, la2: f64, lo2: f64) -> f64 {
    let dl = ((lo2 - lo1 + 540.0) % 360.0) - 180.0;
    let x = dl.to_radians() * ((la1 + la2) / 2.0).to_radians().cos();
    let y = (la2 - la1).to_radians();
    6_371_000.0 * x.hypot(y)
}

/// One unit in the last place of a coordinate, degrees.
fn ulp(x: f64) -> f64 {
    let a = x.abs().max(f64::MIN_POSITIVE);
    f64::from_bits(a.to_bits() + 1) - a
}

#[test]
fn projections_match_proj() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/projections_proj.json"
    );
    let fx: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mut report = Vec::new();
    for m in METHODS {
        let cases = fx["methods"][m].as_array().unwrap();
        assert_eq!(cases.len(), 300);
        let (mut en, mut conv, mut scale, mut back) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
        for c in cases {
            let (lat, lon) = (c["lat"].as_f64().unwrap(), c["lon"].as_f64().unwrap());
            let f = run(
                &format!("geodesy.projection.{m}-forward"),
                &with(&c["params"], json!({"lat": lat, "lon": lon})),
            );
            en = en
                .max((val(&f, "easting") - c["e"].as_f64().unwrap()).abs())
                .max((val(&f, "northing") - c["n"].as_f64().unwrap()).abs());
            conv = conv.max((val(&f, "convergence") - c["convergence"].as_f64().unwrap()).abs());
            // Relative: Web Mercator's scale passes 11 near its limit.
            for (k, r) in [("scale_meridian", "h"), ("scale_parallel", "k")] {
                let want = c[r].as_f64().unwrap();
                scale = scale.max((val(&f, k) - want).abs() / want);
            }
            // The reference's easting and northing, back to the point.
            let i = run(
                &format!("geodesy.projection.{m}-inverse"),
                &with(
                    &c["params"],
                    json!({"easting": format!("{} m", c["e"]), "northing": format!("{} m", c["n"])}),
                ),
            );
            back = back.max(ground(lat, lon, val(&i, "lat"), val(&i, "lon")));
        }
        report.push(format!(
            "{m}: grid {en:.1e} m, convergence {conv:.1e} deg, scale {scale:.1e}, inverse {back:.1e} m"
        ));
        // The spec's 1 mm, held here a hundred times tighter. Standard
        // parallels a fiftieth of a degree apart make the cone's constant a
        // ratio of two near-cancelling differences, and both sides lose
        // digits: in the worst case here (14.3234 and 14.342 degrees) a
        // 50-digit evaluation puts this tool 0.4 µm off and PROJ 0.9 µm.
        assert!(en < 1e-5, "{m}: easting or northing off by {en} m");
        assert!(back < 1e-5, "{m}: inverse off by {back} m");
        // Central differences of the reference carry about 1e-9 of noise.
        assert!(conv < 1e-7, "{m}: convergence off by {conv} deg");
        assert!(scale < 1e-8, "{m}: scale off by {scale} of itself");
    }
    println!("{}", report.join("\n"));
}

/// Forward then inverse returns the point, within a nanometer on the ground
/// away from each method's singular limits (a degree clear of the cut
/// opposite the central meridian), beyond what the numbers themselves can
/// hold. A double holds a longitude near 180 degrees to 2.8e-14 degrees, 3 nm
/// at the equator, and a northing of 10,000 km to 1.9 nm, so 1 nm alone is
/// finer than the coordinates. The floor is one unit in the last place of the
/// latitude, of a longitude near 180 (a longitude is formed from differences
/// with the central meridian), and of the easting and northing over the
/// scale; four of those are allowed.
///
/// Albers is held to 0.1 µm, and to 80 degrees of latitude: its inverse
/// recovers the latitude from q, whose slope falls to zero at the poles, and
/// its cone radii of 10,000 km carry 2 nm of rounding each. Guidance Note
/// 7-2's formulas, which PROJ uses too, give about 10 nm at low latitudes,
/// 80 nm at 80 degrees, and 2 µm at 89.5.
#[test]
fn projections_round_trip() {
    let setups = [
        ("web-mercator", json!({})),
        (
            "lcc",
            json!({"standard_parallel_1": 33, "standard_parallel_2": 45, "latitude_of_origin": 23, "longitude_of_origin": -96}),
        ),
        (
            "lcc",
            json!({"variant": "1SP", "latitude_of_origin": -35, "longitude_of_origin": 145, "scale_factor": 0.9996, "false_easting": "500000 m", "false_northing": "10000000 m", "ellipsoid": "grs80"}),
        ),
        (
            "albers",
            json!({"standard_parallel_1": -18, "standard_parallel_2": -36, "longitude_of_origin": 132, "ellipsoid": "grs80"}),
        ),
        (
            "polar-stereographic",
            json!({"pole": "N", "standard_parallel": 70, "longitude_of_origin": -45}),
        ),
        (
            "equidistant-cylindrical",
            json!({"standard_parallel": 30, "longitude_of_origin": 10}),
        ),
    ];
    for (m, p) in setups {
        let north_only = m == "polar-stereographic";
        let south_only = p["latitude_of_origin"].as_f64().is_some_and(|l| l < 0.0) && m == "lcc";
        let mut worst = 0.0f64;
        for i in 0..400 {
            let t = f64::from(i);
            let lat = if north_only {
                1.0 + 88.0 * ((t * 0.37).sin() + 1.0) / 2.0
            } else if south_only {
                -(1.0 + 80.0 * ((t * 0.37).sin() + 1.0) / 2.0)
            } else if m == "albers" {
                80.0 * (t * 0.37).sin()
            } else {
                84.0 * (t * 0.37).sin()
            };
            // A degree clear of the cut opposite the central meridian.
            let lon0 = p["longitude_of_origin"].as_f64().unwrap_or(0.0);
            // Wrapped by one exact subtraction (Sterbenz), not by `% 360`,
            // which would round the longitude by more than the test allows.
            let lon = match lon0 + 179.0 * (t * 0.61).sin() {
                l if l >= 180.0 => l - 360.0,
                l if l < -180.0 => l + 360.0,
                l => l,
            };
            let f = run(
                &format!("geodesy.projection.{m}-forward"),
                &with(&p, json!({"lat": lat, "lon": lon})),
            );
            let (e, n) = (val(&f, "easting"), val(&f, "northing"));
            let scale = val(&f, "scale_meridian").min(val(&f, "scale_parallel"));
            let i = run(
                &format!("geodesy.projection.{m}-inverse"),
                &with(
                    &p,
                    json!({"easting": format!("{e} m"), "northing": format!("{n} m")}),
                ),
            );
            let floor =
                ground(lat, lon, lat + ulp(lat), lon + ulp(180.0)) + (ulp(e) + ulp(n)) / scale;
            worst = worst.max(ground(lat, lon, val(&i, "lat"), val(&i, "lon")) - 4.0 * floor);
        }
        println!("{m}: round trip within 1 nm + {worst:.1e} m of four ulps");
        let bound = if m == "albers" { 1e-7 } else { 1e-9 };
        assert!(
            worst < bound,
            "{m}: round trip off by {worst} m beyond four ulps"
        );
    }
}

/// What each method must keep, whatever its parameters.
#[test]
fn projection_invariants() {
    let lcc = json!({"standard_parallel_1": 33, "standard_parallel_2": 45, "latitude_of_origin": 23, "longitude_of_origin": -96, "false_easting": "1000 m"});
    let albers = json!({"standard_parallel_1": 29.5, "standard_parallel_2": 45.5, "latitude_of_origin": 23, "longitude_of_origin": -96, "false_easting": "1000 m"});
    let polar = json!({"pole": "S", "scale_factor": 0.994, "longitude_of_origin": 0, "false_easting": "1000 m"});
    let eqc =
        json!({"standard_parallel": 30, "longitude_of_origin": -96, "false_easting": "1000 m"});
    for (m, p, lat) in [
        ("lcc", &lcc, 40.0),
        ("albers", &albers, 40.0),
        ("polar-stereographic", &polar, -80.0),
        ("equidistant-cylindrical", &eqc, 40.0),
    ] {
        let tool = format!("geodesy.projection.{m}-forward");
        let lon0 = p["longitude_of_origin"].as_f64().unwrap();
        // On the central meridian grid north is true north and the easting is the false easting.
        let c = run(&tool, &with(p, json!({"lat": lat, "lon": lon0})));
        assert!(val(&c, "convergence").abs() < 1e-12, "{m}");
        assert!((val(&c, "easting") - 1000.0).abs() < 1e-9, "{m}");
        // Mirroring the point across the central meridian mirrors the grid.
        let e = run(&tool, &with(p, json!({"lat": lat, "lon": lon0 + 7.0})));
        let w = run(&tool, &with(p, json!({"lat": lat, "lon": lon0 - 7.0})));
        assert!(
            (val(&e, "easting") - 1000.0 + val(&w, "easting") - 1000.0).abs() < 1e-7,
            "{m}"
        );
        assert!(
            (val(&e, "northing") - val(&w, "northing")).abs() < 1e-7,
            "{m}"
        );
        assert!(
            (val(&e, "convergence") + val(&w, "convergence")).abs() < 1e-12,
            "{m}"
        );
        let (h, k) = (val(&e, "scale_meridian"), val(&e, "scale_parallel"));
        match m {
            // Conformal: the same scale in every direction.
            "lcc" | "polar-stereographic" => assert!((h - k).abs() < 1e-15, "{m}"),
            // Equal area: the two scales multiply to 1.
            "albers" => assert!((h * k - 1.0).abs() < 1e-14, "{m}"),
            // Meridians keep their length.
            _ => assert_eq!(h, 1.0, "{m}"),
        }
    }
    // Two-parallel conics are true to scale on both standard parallels.
    for m in ["lcc", "albers"] {
        let p = if m == "lcc" { &lcc } else { &albers };
        for sp in ["standard_parallel_1", "standard_parallel_2"] {
            let r = run(
                &format!("geodesy.projection.{m}-forward"),
                &with(p, json!({"lat": p[sp], "lon": -80})),
            );
            assert!((val(&r, "scale_parallel") - 1.0).abs() < 1e-12, "{m} {sp}");
        }
    }
    // Equidistant cylindrical is true east-west on its standard parallel.
    let r = run(
        "geodesy.projection.equidistant-cylindrical-forward",
        &with(&eqc, json!({"lat": 30, "lon": 0})),
    );
    assert!((val(&r, "scale_parallel") - 1.0).abs() < 1e-12);
    // Polar stereographic's scale at the pole is its scale factor.
    let r = run(
        "geodesy.projection.polar-stereographic-forward",
        &with(&polar, json!({"lat": -90, "lon": 0})),
    );
    assert!((val(&r, "scale_parallel") - 0.994).abs() < 1e-12);
    // UPS is polar stereographic variant A with k0 = 0.994 at 2,000 km falsings.
    let ups = run("geodesy.ups.forward", &json!({"lat": 87.5, "lon": 123.4}));
    let ps = run(
        "geodesy.projection.polar-stereographic-forward",
        &json!({"lat": 87.5, "lon": 123.4, "pole": "N", "scale_factor": 0.994, "false_easting": "2000000 m", "false_northing": "2000000 m"}),
    );
    assert!((val(&ups, "easting") - val(&ps, "easting")).abs() < 1e-8);
    assert!((val(&ups, "northing") - val(&ps, "northing")).abs() < 1e-8);
    // Web Mercator's scales on the ellipsoid: a over the two radii of curvature times cos φ.
    let r = run(
        "geodesy.projection.web-mercator-forward",
        &json!({"lat": 0, "lon": 0}),
    );
    assert!((val(&r, "scale_parallel") - 1.0).abs() < 1e-15);
    assert!((val(&r, "scale_meridian") - 1.0 / (1.0 - 0.006_694_379_990_141_3)).abs() < 1e-12);
}
