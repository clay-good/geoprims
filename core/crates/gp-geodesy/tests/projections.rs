//! The projection methods with user-set parameters against PROJ and GeographicLib
//! (geodesy/projections, "Round-trip and differential accuracy"): 300 random
//! parameter sets and points per method from tools/vectors/gen_projections_proj.py,
//! forward and inverse, and the invariants each method must keep.

use gp_geodesy::REGISTRY;
use serde_json::{Map, Value, json};

/// Each fixture and the methods it holds.
const FIXTURES: [(&str, &[&str]); 4] = [
    (
        "projections_proj.json",
        &[
            "web-mercator",
            "lcc",
            "albers",
            "polar-stereographic",
            "equidistant-cylindrical",
            "orthographic",
            "hotine",
        ],
    ),
    (
        "projections_azimuthal.json",
        &["azimuthal-equidistant", "gnomonic"],
    ),
    ("projections_tm.json", &["tm"]),
    ("projections_tm_exact.json", &["tm-exact"]),
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
    let mut report = Vec::new();
    for (file, methods) in FIXTURES {
        let path = format!("{}/tests/data/{file}", env!("CARGO_MANIFEST_DIR"));
        let fx: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        for &m in methods {
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
                conv =
                    conv.max((val(&f, "convergence") - c["convergence"].as_f64().unwrap()).abs());
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
        (
            "azimuthal-equidistant",
            json!({"latitude_of_origin": 20, "longitude_of_origin": 10}),
        ),
        (
            "gnomonic",
            json!({"latitude_of_origin": 20, "longitude_of_origin": 10}),
        ),
        (
            "orthographic",
            json!({"latitude_of_origin": 20, "longitude_of_origin": 10}),
        ),
        (
            "tm",
            json!({"longitude_of_origin": 10, "latitude_of_origin": 49, "scale_factor": 0.9996, "false_easting": "500000 m"}),
        ),
        (
            "tm-exact",
            json!({"longitude_of_origin": 10, "scale_factor": 0.9996}),
        ),
        (
            "hotine",
            json!({"latitude_of_center": 20, "longitude_of_center": 10, "azimuth": 35, "scale_factor": 0.9996}),
        ),
    ];
    for (m, p) in setups {
        let north_only = m == "polar-stereographic";
        let south_only = p["latitude_of_origin"].as_f64().is_some_and(|l| l < 0.0) && m == "lcc";
        let mut worst = 0.0f64;
        for i in 0..400 {
            let t = f64::from(i);
            // Azimuthal: points spread over a cap around the center, 80° of
            // arc for the equidistant and 60° for the gnomonic.
            let cap = match m {
                "azimuthal-equidistant" => Some(80.0),
                "gnomonic" | "orthographic" => Some(60.0),
                "hotine" | "tm" => Some(30.0),
                // Clear of the branch point on the equator 82.6° out.
                "tm-exact" => Some(60.0),
                _ => None,
            };
            if let Some(cap) = cap {
                let (lat, lon) = (
                    20.0 + cap / 2.0 * (t * 0.37).sin(),
                    10.0 + cap * (t * 0.61).sin(),
                );
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
                continue;
            }
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
    // Azimuthal equidistant: the grid distance from the center is the
    // geodesic distance, and the grid bearing from it the geodesic azimuth.
    let center =
        json!({"latitude_of_origin": 40, "longitude_of_origin": -100, "false_easting": "1000 m"});
    for (lat, lon) in [(55.0, 20.0), (-30.0, 60.0), (41.0, -99.0)] {
        let r = run(
            "geodesy.projection.azimuthal-equidistant-forward",
            &with(&center, json!({"lat": lat, "lon": lon})),
        );
        let (e, n) = (val(&r, "easting") - 1000.0, val(&r, "northing"));
        let geod = geographiclib_rs::Geodesic::wgs84();
        let (s, azi, _, _): (f64, f64, f64, f64) =
            geographiclib_rs::InverseGeodesic::inverse(&geod, 40.0, -100.0, lat, lon);
        assert!((e.hypot(n) - s).abs() < 1e-8, "{lat} {lon}");
        assert!((e.atan2(n).to_degrees() - azi).abs() < 1e-9, "{lat} {lon}");
    }
    // Gnomonic: points along one geodesic through the center lie on a
    // straight line, and the horizon is refused.
    let geod = geographiclib_rs::Geodesic::wgs84();
    let line: Vec<(f64, f64)> = [500e3, 2000e3, 5000e3]
        .iter()
        .map(|&d| {
            let (la, lo): (f64, f64) =
                geographiclib_rs::DirectGeodesic::direct(&geod, 40.0, -100.0, 57.0, d);
            let r = run(
                "geodesy.projection.gnomonic-forward",
                &with(&center, json!({"lat": la, "lon": lo})),
            );
            (val(&r, "easting") - 1000.0, val(&r, "northing"))
        })
        .collect();
    for (e, n) in &line {
        assert!((e.atan2(*n).to_degrees() - 57.0).abs() < 1e-9);
    }
    let far: Value = serde_json::from_str(&REGISTRY.invoke(
        "geodesy.projection.gnomonic-forward",
        &with(&center, json!({"lat": -50, "lon": -100})).to_string(),
    ))
    .unwrap();
    assert_eq!(far["error"]["code"], "OUT_OF_DOMAIN");
    // Orthographic: the center lands at the falsings with true scale, and
    // the far side is refused.
    let o = run(
        "geodesy.projection.orthographic-forward",
        &with(&center, json!({"lat": 40, "lon": -100})),
    );
    assert!((val(&o, "easting") - 1000.0).abs() < 1e-9 && val(&o, "northing").abs() < 1e-9);
    assert!(
        (val(&o, "scale_meridian") - 1.0).abs() < 1e-15
            && (val(&o, "scale_parallel") - 1.0).abs() < 1e-15
    );
    let far: Value = serde_json::from_str(&REGISTRY.invoke(
        "geodesy.projection.orthographic-forward",
        &with(&center, json!({"lat": -40, "lon": 80})).to_string(),
    ))
    .unwrap();
    assert_eq!(far["error"]["code"], "OUT_OF_DOMAIN");
    // Hotine: variant B puts the center at the given easting and northing,
    // and variant A with its falsings moved by the center's variant-A
    // coordinates is the same grid; it is conformal.
    let hot = json!({"latitude_of_center": 46.5, "longitude_of_center": 8.0, "azimuth": 30, "scale_factor": 1});
    let c = run(
        "geodesy.projection.hotine-forward",
        &with(
            &hot,
            json!({"lat": 46.5, "lon": 8.0, "false_easting": "600000 m", "false_northing": "200000 m"}),
        ),
    );
    assert!(
        (val(&c, "easting") - 600000.0).abs() < 1e-8
            && (val(&c, "northing") - 200000.0).abs() < 1e-8
    );
    let a0 = run(
        "geodesy.projection.hotine-forward",
        &with(&hot, json!({"lat": 46.5, "lon": 8.0, "variant": "A"})),
    );
    let (ea, na) = (val(&a0, "easting"), val(&a0, "northing"));
    let pa = run(
        "geodesy.projection.hotine-forward",
        &with(
            &hot,
            json!({"lat": 47.3, "lon": 9.1, "variant": "A", "false_easting": format!("{} m", 600000.0 - ea), "false_northing": format!("{} m", 200000.0 - na)}),
        ),
    );
    let pb = run(
        "geodesy.projection.hotine-forward",
        &with(
            &hot,
            json!({"lat": 47.3, "lon": 9.1, "false_easting": "600000 m", "false_northing": "200000 m"}),
        ),
    );
    assert!((val(&pa, "easting") - val(&pb, "easting")).abs() < 1e-7);
    assert!((val(&pa, "northing") - val(&pb, "northing")).abs() < 1e-7);
    assert!((val(&pb, "scale_meridian") - val(&pb, "scale_parallel")).abs() < 1e-9);
    // Web Mercator's scales on the ellipsoid: a over the two radii of curvature times cos φ.
    let r = run(
        "geodesy.projection.web-mercator-forward",
        &json!({"lat": 0, "lon": 0}),
    );
    assert!((val(&r, "scale_parallel") - 1.0).abs() < 1e-15);
    assert!((val(&r, "scale_meridian") - 1.0 / (1.0 - 0.006_694_379_990_141_3)).abs() < 1e-12);
}

#[test]
fn nztm_round_trips_and_pins_its_origin() {
    // add-international-grids, geodesy/national-grids: NZTM2000 is EPSG 2193,
    // one transverse Mercator zone over the whole country on NZGD2000.
    let places = [
        (-41.278611, 174.776944),
        (-36.848461, 174.762191),
        (-43.530955, 172.636645),
        (-45.873889, 170.503611),
        (-46.413100, 168.352800),
        (-37.688900, 178.548600),
    ];
    for (lat, lon) in places {
        let f = run(
            "geodesy.projection.nztm-forward",
            &json!({"lat": lat, "lon": lon}),
        );
        let (e, n) = (val(&f, "easting"), val(&f, "northing"));
        let b = run(
            "geodesy.projection.nztm-inverse",
            &json!({"easting": format!("{e} m"), "northing": format!("{n} m")}),
        );
        assert!((val(&b, "lat") - lat).abs() < 1e-9, "{b}");
        assert!((val(&b, "lon") - lon).abs() < 1e-9, "{b}");
        // Both ends describe the same point, so both report the same
        // convergence and the same point scale factor there.
        assert!(
            (val(&f, "convergence") - val(&b, "convergence")).abs() < 1e-9,
            "{f} vs {b}"
        );
        assert!(
            (val(&f, "scale_factor") - val(&b, "scale_factor")).abs() < 1e-12,
            "{f} vs {b}"
        );
        // The grid covers the country in one zone, so every place lands inside
        // the band LINZ's own extent describes.
        assert!(
            (1_000_000.0..2_600_000.0).contains(&e) && (4_700_000.0..6_300_000.0).contains(&n),
            "{lat}, {lon} fell off the national grid: E {e} N {n}"
        );
    }
    // On the central meridian at the latitude of origin: the false origin
    // exactly, no convergence, and the scale factor the standard declares.
    let o = run(
        "geodesy.projection.nztm-forward",
        &json!({"lat": 0.0, "lon": 173.0}),
    );
    assert!((val(&o, "easting") - 1_600_000.0).abs() < 1e-6, "{o}");
    assert!((val(&o, "northing") - 10_000_000.0).abs() < 1e-6, "{o}");
    assert!(val(&o, "convergence").abs() < 1e-12, "{o}");
    assert!((val(&o, "scale_factor") - 0.9996).abs() < 1e-12, "{o}");
    // The convergence is degrees, and roughly -(lambda - lambda0)*sin(phi).
    // Checking only that the forward and inverse tools agree let a convergence
    // 57.3 times too large ship on the Irish grid, so check the size here.
    for (lat, lon) in places {
        let f = run(
            "geodesy.projection.nztm-forward",
            &json!({"lat": lat, "lon": lon}),
        );
        let rough = (lon - 173.0) * lat.to_radians().sin();
        assert!(
            (val(&f, "convergence") - rough).abs() < 0.01,
            "convergence at {lat}, {lon} should be about {rough} degrees, not {}",
            val(&f, "convergence")
        );
    }

    // The grid is defined on GRS 80, not WGS 84. The two differ only in the
    // ninth digit of the flattening, which moved the Beehive's northing by
    // 0.12 mm -- inside the millimetre the coordinate vectors allow, so
    // nothing caught it. Pinned against GeographicLib on GRS 80's parameters.
    let w = run(
        "geodesy.projection.nztm-forward",
        &json!({"lat": -41.278611, "lon": 174.776944}),
    );
    assert!(
        (val(&w, "northing") - 5_428_791.056_045_2).abs() < 1e-6,
        "the Beehive's northing on GRS 80 is 5428791.0560452 m: {}",
        val(&w, "northing")
    );

    // East and west of the central meridian the convergence turns the other
    // way, which is what makes it a correction and not a constant.
    let w = run(
        "geodesy.projection.nztm-forward",
        &json!({"lat": -41.0, "lon": 170.0}),
    );
    let e = run(
        "geodesy.projection.nztm-forward",
        &json!({"lat": -41.0, "lon": 176.0}),
    );
    assert!(
        val(&w, "convergence") * val(&e, "convergence") < 0.0,
        "convergence should change sign across the central meridian: {w} vs {e}"
    );
    // Its false northing is 10,000 km, so an NZTM northing read as a British
    // or Irish one is nowhere near either grid: they are separate tools.
    assert!(val(&w, "northing") > 5_000_000.0, "{w}");
}

#[test]
fn mga_rounds_trips_picks_its_zone_and_pins_its_origins() {
    // add-international-grids, geodesy/national-grids: the Map Grid of
    // Australia is UTM on GRS 80, registered by EPSG as zones 46 to 59 on
    // GDA2020 (7846 to 7859) and 48 to 58 on GDA94 (28348 to 28358).
    let places = [
        (-33.858611, 151.214167, 56),
        (-37.818078, 144.967105, 55),
        (-27.468968, 153.023499, 56),
        (-31.959500, 115.843900, 50),
        (-12.463400, 130.845600, 52),
        (-42.884400, 147.331000, 55),
        (-12.187800, 96.829100, 47),
        (-29.054600, 167.961900, 58),
    ];
    for (lat, lon, zone) in places {
        let f = run(
            "geodesy.projection.mga-forward",
            &json!({"lat": lat, "lon": lon}),
        );
        // The zone comes from the longitude, not from the caller.
        assert_eq!(
            val(&f, "zone") as i32,
            zone,
            "{lat}, {lon} should fall in MGA zone {zone}: {f}"
        );
        // The zone's central meridian is 6n - 183, and the point is inside
        // the zone's own six-degree band.
        let cm = val(&f, "central_meridian");
        assert!(
            (cm - (6.0 * f64::from(zone) - 183.0)).abs() < 1e-12,
            "zone {zone}'s central meridian is {cm}: {f}"
        );
        assert!(
            (lon - cm).abs() <= 3.0,
            "{lon} is not within zone {zone}: {f}"
        );

        let (e, n) = (val(&f, "easting"), val(&f, "northing"));
        let b = run(
            "geodesy.projection.mga-inverse",
            &json!({"zone": zone, "easting": format!("{e} m"), "northing": format!("{n} m")}),
        );
        assert!((val(&b, "lat") - lat).abs() < 1e-9, "{b}");
        assert!((val(&b, "lon") - lon).abs() < 1e-9, "{b}");
        // Both ends describe the same point, so both report the same
        // convergence and point scale factor there.
        assert!(
            (val(&f, "convergence") - val(&b, "convergence")).abs() < 1e-9,
            "{f} vs {b}"
        );
        assert!(
            (val(&f, "scale_factor") - val(&b, "scale_factor")).abs() < 1e-12,
            "{f} vs {b}"
        );
        // Inside its own zone a point stays within 3 degrees of the central
        // meridian, so the easting stays inside this band and the scale
        // factor between 0.9996 and about 1.0010 (Janssen's figure for the
        // zone edge). A wrong k0 or false easting breaks both.
        assert!(
            (100_000.0..900_000.0).contains(&e) && (5_000_000.0..9_900_000.0).contains(&n),
            "{lat}, {lon} fell off MGA zone {zone}: E {e} N {n}"
        );
        let k = val(&f, "scale_factor");
        assert!(
            (0.9996..1.0011).contains(&k),
            "the point scale factor inside a zone runs from 0.9996 to about 1.0010, not {k}"
        );
        // The convergence is degrees, and roughly -(lambda - lambda0) sin(phi).
        // Checking only that the two tools agree let a convergence 57.3 times
        // too large ship on the Irish grid, so check the size here too.
        let rough = (lon - cm) * lat.to_radians().sin();
        assert!(
            (val(&f, "convergence") - rough).abs() < 0.01,
            "convergence at {lat}, {lon} should be about {rough} degrees, not {}",
            val(&f, "convergence")
        );
    }

    // Each zone's own origin: on its central meridian at the equator the
    // coordinates are the false origin exactly, the convergence is zero, and
    // the scale factor is the declared 0.9996. This pins the parameters
    // rather than a place.
    for zone in 46..=59 {
        let cm = 6.0 * f64::from(zone) - 183.0;
        let o = run(
            "geodesy.projection.mga-forward",
            &json!({"lat": 0.0, "lon": cm}),
        );
        assert_eq!(val(&o, "zone") as i32, zone, "{o}");
        assert!((val(&o, "easting") - 500_000.0).abs() < 1e-6, "{o}");
        assert!((val(&o, "northing") - 10_000_000.0).abs() < 1e-6, "{o}");
        assert!(val(&o, "convergence").abs() < 1e-12, "{o}");
        assert!((val(&o, "scale_factor") - 0.9996).abs() < 1e-12, "{o}");
    }

    // The grid is on GRS 80, not WGS 84. The two differ only in the ninth
    // digit of the flattening, which moves a northing by well under the
    // millimetre the PROJ vectors allow -- the silent ellipsoid fallback that
    // shipped on the Irish grid. Pinned against GeographicLib on GRS 80.
    let s = run(
        "geodesy.projection.mga-forward",
        &json!({"lat": -33.858611, "lon": 151.214167}),
    );
    assert!(
        (val(&s, "northing") - 6_252_086.099_999_2).abs() < 1e-6,
        "the Opera House's northing on GRS 80 is 6252086.0999992 m, not {}",
        val(&s, "northing")
    );

    // East and west of a central meridian the convergence turns the other
    // way, which is what makes it a correction and not a constant.
    let w = run(
        "geodesy.projection.mga-forward",
        &json!({"lat": -33.0, "lon": 151.0}),
    );
    let e = run(
        "geodesy.projection.mga-forward",
        &json!({"lat": -33.0, "lon": 155.0}),
    );
    assert!(
        val(&w, "convergence") * val(&e, "convergence") < 0.0,
        "convergence should change sign across zone 56's central meridian: {w} vs {e}"
    );

    // MGA94 and MGA2020 are the same projection on two datums, so the same
    // latitude and longitude must give the same easting and northing under
    // either name. The tool says this in words; this says it in numbers, and
    // would catch a datum step creeping into a projection tool.
    let a = run(
        "geodesy.projection.mga-forward",
        &json!({"lat": -33.858611, "lon": 151.214167, "datum": "MGA94"}),
    );
    assert!(
        (val(&a, "easting") - val(&s, "easting")).abs() < 1e-9,
        "{a} vs {s}"
    );
    assert!(
        (val(&a, "northing") - val(&s, "northing")).abs() < 1e-9,
        "{a} vs {s}"
    );
    // What does change is the code the answer names, and MGA94 has no
    // registered code outside zones 48 to 58.
    assert_eq!(a["epsg"], "EPSG:28356 (GDA94 / MGA zone 56)", "{a}");
    assert_eq!(s["epsg"], "EPSG:7856 (GDA2020 / MGA zone 56)", "{s}");
    let far = run(
        "geodesy.projection.mga-forward",
        &json!({"lat": -12.0, "lon": 94.5, "datum": "MGA94"}),
    );
    assert_eq!(val(&far, "zone") as i32, 46, "{far}");
    assert!(
        far["epsg"].as_str().unwrap().starts_with("none:"),
        "EPSG registers GDA94 / MGA only for zones 48 to 58: {far}"
    );
}

#[test]
fn mga_refuses_a_zone_outside_australia_and_flags_a_forced_one() {
    // A zone outside 46 to 59 is not an Australian zone, and the refusal has
    // to name the range rather than return arithmetic.
    for zone in [1, 45, 60, 31] {
        let r: Value = serde_json::from_str(&REGISTRY.invoke(
            "geodesy.projection.mga-inverse",
            &json!({"zone": zone, "easting": "500000 m", "northing": "7000000 m"}).to_string(),
        ))
        .unwrap();
        assert_eq!(r["ok"], false, "zone {zone} should be refused: {r}");
        assert_eq!(r["error"]["code"], "INVALID_INPUT", "{r}");
        assert_eq!(r["error"]["field"], "/zone", "{r}");
        let msg = r["error"]["message"].as_str().unwrap();
        assert!(
            msg.contains("46") && msg.contains("59"),
            "the refusal must name the range 46 to 59: {msg}"
        );
    }
    // A longitude outside the Australian zones is refused the same way,
    // rather than quietly projecting onto a zone that is not an MGA zone.
    let r: Value = serde_json::from_str(&REGISTRY.invoke(
        "geodesy.projection.mga-forward",
        &json!({"lat": 51.5, "lon": -0.1}).to_string(),
    ))
    .unwrap();
    assert_eq!(r["ok"], false, "London is not on the MGA grid: {r}");
    assert_eq!(r["error"]["code"], "OUT_OF_DOMAIN", "{r}");

    // Forcing a neighbouring zone is allowed -- jobs do run across boundaries
    // -- but the answer has to say the zone is not the standard one.
    let r: Value = serde_json::from_str(&REGISTRY.invoke(
        "geodesy.projection.mga-forward",
        &json!({"lat": -27.468968, "lon": 153.023499, "zone": 55}).to_string(),
    ))
    .unwrap();
    assert_eq!(r["ok"], true, "{r}");
    let codes: Vec<&str> = r["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap())
        .collect();
    assert!(codes.contains(&"NONSTANDARD_ZONE"), "{r}");
    // Off its own zone the scale factor leaves the 0.9996 to 1.0010 band,
    // which is the cost the warning is about.
    assert!(
        r["result"]["scale_factor"].as_f64().unwrap() > 1.0011,
        "a forced zone costs scale: {r}"
    );

    // A point well outside the published Australian extent still computes,
    // but says so.
    let r: Value = serde_json::from_str(&REGISTRY.invoke(
        "geodesy.projection.mga-forward",
        &json!({"lat": 20.0, "lon": 120.0}).to_string(),
    ))
    .unwrap();
    assert_eq!(r["ok"], true, "{r}");
    let codes: Vec<&str> = r["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap())
        .collect();
    assert!(codes.contains(&"OUTSIDE_AREA_OF_USE"), "{r}");
}

#[test]
fn jgd2011_round_trips_every_system_and_pins_the_notices_origins() {
    // add-international-grids, geodesy/national-grids: the nineteen JGD2011
    // plane rectangular systems of MLIT Notice No. 9 of 2002, EPSG 6669 to
    // 6687. Origins are (latitude, longitude) as the notice prints them.
    let origins = [
        (33.0, 129.5),
        (33.0, 131.0),
        (36.0, 132.0 + 10.0 / 60.0),
        (33.0, 133.5),
        (36.0, 134.0 + 20.0 / 60.0),
        (36.0, 136.0),
        (36.0, 137.0 + 10.0 / 60.0),
        (36.0, 138.5),
        (36.0, 139.0 + 50.0 / 60.0),
        (40.0, 140.0 + 50.0 / 60.0),
        (44.0, 140.25),
        (44.0, 142.25),
        (44.0, 144.25),
        (26.0, 142.0),
        (26.0, 127.5),
        (26.0, 124.0),
        (26.0, 131.0),
        (20.0, 136.0),
        (26.0, 154.0),
    ];
    for (i, (lat0, lon0)) in origins.into_iter().enumerate() {
        let zone = i as i32 + 1;
        // At the origin X and Y are zero, the convergence is zero, and the
        // scale factor is the notice's 0.9999.
        let o = run(
            "geodesy.projection.jgd2011-forward",
            &json!({"lat": lat0, "lon": lon0, "zone": zone}),
        );
        assert!(val(&o, "easting").abs() < 1e-6, "system {zone}: {o}");
        assert!(val(&o, "northing").abs() < 1e-6, "system {zone}: {o}");
        assert!(val(&o, "convergence").abs() < 1e-12, "system {zone}: {o}");
        assert!((val(&o, "scale_factor") - 0.9999).abs() < 1e-12, "{o}");
        assert_eq!(
            o["epsg"].as_str().unwrap()[..9],
            format!("EPSG:{}", 6668 + zone)
        );

        // A point 1.3 degrees north-east of the origin and back.
        let (lat, lon) = (lat0 + 1.3, lon0 + 1.3);
        let f = run(
            "geodesy.projection.jgd2011-forward",
            &json!({"lat": lat, "lon": lon, "zone": zone}),
        );
        let (e, n) = (val(&f, "easting"), val(&f, "northing"));
        // North-east of the origin is positive X and positive Y: the axes are
        // the notice's, not swapped.
        assert!(e > 100_000.0 && n > 100_000.0, "system {zone}: {f}");
        let b = run(
            "geodesy.projection.jgd2011-inverse",
            &json!({"zone": zone, "easting": format!("{e} m"), "northing": format!("{n} m")}),
        );
        assert!((val(&b, "lat") - lat).abs() < 1e-9, "{b}");
        assert!((val(&b, "lon") - lon).abs() < 1e-9, "{b}");
        assert!(
            (val(&f, "convergence") - val(&b, "convergence")).abs() < 1e-9,
            "{f} vs {b}"
        );
        assert!(
            (val(&f, "scale_factor") - val(&b, "scale_factor")).abs() < 1e-12,
            "{f} vs {b}"
        );
        // The convergence is degrees, and roughly (lambda - lambda0) sin(phi).
        let rough = (lon - lon0) * lat.to_radians().sin();
        assert!(
            (val(&f, "convergence") - rough).abs() < 0.02,
            "convergence at {lat}, {lon} should be about {rough} degrees, not {}",
            val(&f, "convergence")
        );
    }

    // Tokyo Station on system IX, pinned against PROJ's EPSG:6677 at a
    // tenth of a millimetre: on GRS 80 and on the notice's origin.
    let t = run(
        "geodesy.projection.jgd2011-forward",
        &json!({"lat": 35.681236, "lon": 139.767125, "zone": 9}),
    );
    assert!((val(&t, "northing") + 35_363.237_745).abs() < 1e-4, "{t}");
    assert!((val(&t, "easting") + 5_992.919_570).abs() < 1e-4, "{t}");
}

#[test]
fn jgd2011_refuses_a_system_outside_the_nineteen_and_flags_a_point_off_japan() {
    for zone in [0, 20, -9] {
        let r: Value = serde_json::from_str(&REGISTRY.invoke(
            "geodesy.projection.jgd2011-inverse",
            &json!({"zone": zone, "easting": "0 m", "northing": "0 m"}).to_string(),
        ))
        .unwrap();
        assert_eq!(r["ok"], false, "system {zone} should be refused: {r}");
        assert_eq!(r["error"]["code"], "INVALID_INPUT", "{r}");
        assert_eq!(r["error"]["field"], "/zone", "{r}");
    }
    // The system is set by prefecture, so the forward tool does not guess it.
    let r: Value = serde_json::from_str(&REGISTRY.invoke(
        "geodesy.projection.jgd2011-forward",
        &json!({"lat": 35.681236, "lon": 139.767125}).to_string(),
    ))
    .unwrap();
    assert_eq!(r["ok"], false, "{r}");
    assert_eq!(r["error"]["field"], "/zone", "{r}");

    let r: Value = serde_json::from_str(&REGISTRY.invoke(
        "geodesy.projection.jgd2011-forward",
        &json!({"lat": 51.5, "lon": -0.1, "zone": 9}).to_string(),
    ))
    .unwrap();
    assert_eq!(r["ok"], true, "{r}");
    let codes: Vec<&str> = r["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap())
        .collect();
    assert!(codes.contains(&"OUTSIDE_AREA_OF_USE"), "{r}");
}
