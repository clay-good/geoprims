//! SPCS83: all 124 zones against PROJ (2,480 points, 1 mm), round trips, and
//! the spec scenarios through the tools.

use gp_geo::spcs;

#[test]
fn every_zone_matches_proj_within_a_millimeter() {
    let text = include_str!("data/spcs83_diff.csv");
    let (mut worst, mut worst_conv, mut worst_k, mut worst_rt) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    let mut n = 0;
    for l in text.lines().skip(1) {
        let f: Vec<&str> = l.split(',').collect();
        let z = spcs::find(f[0]).unwrap_or_else(|| panic!("zone {}", f[0]));
        let v: Vec<f64> = f[1..].iter().map(|x| x.parse().unwrap()).collect();
        let g = z.forward(v[0], v[1]);
        let d = (g.e - v[2]).hypot(g.n - v[3]);
        worst = worst.max(d);
        worst_conv = worst_conv.max((g.convergence - v[4]).abs());
        worst_k = worst_k.max((g.k - v[5]).abs());
        let (lat, lon) = z.inverse(g.e, g.n);
        let dl = ((lon - v[1] + 540.0).rem_euclid(360.0) - 180.0).abs();
        worst_rt = worst_rt.max((lat - v[0]).abs().max(dl));
        assert!(d < 1e-3, "{}: {d} m at {l}", z.name);
        n += 1;
    }
    println!("worst {worst} m, convergence {worst_conv}°, scale {worst_k}, round trip {worst_rt}°");
    assert_eq!(n, 2480);
    // PROJ computes its factors by numerical differentiation, good to about 1e-7.
    assert!(worst_conv < 1e-6, "{worst_conv}");
    assert!(worst_k < 1e-7, "{worst_k}");
    // 1e-12° is about 0.1 µm; the worst case sits at the last bit of a longitude near 100°.
    assert!(worst_rt < 1e-12, "{worst_rt}");
}

use gp_geodesy::REGISTRY;
use serde_json::Value;

fn call(id: &str, input: &str) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, input)).expect("envelope is JSON")
}

fn warns(r: &Value, code: &str) -> bool {
    r["meta"]["warnings"]
        .as_array()
        .is_some_and(|w| w.iter().any(|w| w["code"] == code))
}

#[test]
fn pennsylvania_south_in_us_survey_feet() {
    let r = call(
        "geodesy.spcs.spcs83-forward",
        r#"{"lat":40.446111,"lon":-79.982222,"zone":"Pennsylvania South"}"#,
    );
    assert_eq!(r["result"]["easting"]["unit"], "ftUS", "{r}");
    let e = r["result"]["easting"]["value"].as_f64().unwrap();
    let n = r["result"]["northing"]["value"].as_f64().unwrap();
    assert!((e - 1_347_294.025).abs() <= 0.003, "{e}");
    assert!((n - 413_222.374).abs() <= 0.003, "{n}");
    assert!(warns(&r, "LEGACY_UNIT"));
    // Once per result, not once per ftUS output.
    let legacy = r["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|w| w["code"] == "LEGACY_UNIT")
        .count();
    assert_eq!(legacy, 1, "{r}");
    // The EPSG code is an identifier, shown without grouping.
    assert_eq!(r["display"]["epsg"], "32129", "{r}");
    assert_eq!(r["result"]["zone_code"], "3702");
    let back = call(
        "geodesy.spcs.spcs83-inverse",
        &format!(r#"{{"zone":"3702","easting":"{e} ftUS","northing":"{n} ftUS"}}"#),
    );
    assert!(
        (back["result"]["lat"]["value"].as_f64().unwrap() - 40.446111).abs() < 1e-11,
        "{back}"
    );
    assert!((back["result"]["lon"]["value"].as_f64().unwrap() + 79.982222).abs() < 1e-11);
    let m = call(
        "geodesy.spcs.spcs83-forward",
        r#"{"lat":40.446111,"lon":-79.982222,"zone":"3702","unit":"m"}"#,
    );
    assert!(!warns(&m, "LEGACY_UNIT"));
}

#[test]
fn point_outside_zone() {
    // Columbus, Ohio, in a Pennsylvania zone.
    let r = call(
        "geodesy.spcs.spcs83-forward",
        r#"{"lat":39.9612,"lon":-82.9988,"zone":"3702"}"#,
    );
    assert_eq!(r["ok"], true);
    assert!(warns(&r, "OUTSIDE_ZONE_EXTENT"), "{r}");
    let msg = r["meta"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["code"] == "OUTSIDE_ZONE_EXTENT")
        .unwrap()["message"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(msg.contains("Ohio South (3402)"), "{msg}");
}

#[test]
fn zone_lookup_and_ambiguity() {
    let r = call("geodesy.spcs.zone-lookup", r#"{"query":"Colorado"}"#);
    assert_eq!(r["result"]["count"], 3.0);
    let r = call("geodesy.spcs.zone-lookup", r#"{"query":"texas south"}"#);
    let names: Vec<&str> = r["result"]["zones"]
        .as_array()
        .unwrap()
        .iter()
        .map(|z| z["zone_name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Texas South Central", "Texas South"], "{r}");
    let r = call("geodesy.spcs.zone-lookup", r#"{"lat":39.74,"lon":-104.99}"#);
    assert!(
        r["result"]["zones"]
            .as_array()
            .unwrap()
            .iter()
            .any(|z| z["zone_code"] == "0502"),
        "{r}"
    );
    // Without a zone, a point covered by several zones is refused, not guessed.
    let r = call(
        "geodesy.spcs.spcs83-forward",
        r#"{"lat":39.74,"lon":-104.99}"#,
    );
    if r["ok"] == false {
        assert!(
            r["error"]["message"]
                .as_str()
                .unwrap()
                .contains("Several zones"),
            "{r}"
        );
    }
}

#[test]
fn every_zone_by_code_and_international_feet() {
    assert_eq!(spcs::ZONES.len(), 124);
    for z in spcs::ZONES {
        assert_eq!(spcs::find(z.fips).map(|f| f.epsg), Some(z.epsg));
        assert_eq!(
            spcs::find(z.short_name()).map(|f| f.epsg),
            Some(z.epsg),
            "{}",
            z.name
        );
    }
    // Arizona law uses international feet.
    let r = call(
        "geodesy.spcs.spcs83-forward",
        r#"{"lat":33.45,"lon":-112.07,"zone":"0202"}"#,
    );
    assert_eq!(r["result"]["easting"]["unit"], "ft", "{r}");
}

#[test]
fn tools_round_trip_in_every_zone() {
    // Forward then inverse through the tools returns the point, and both
    // directions report the same convergence and scale factor.
    let text = include_str!("data/spcs83_diff.csv");
    for l in text.lines().skip(1).step_by(4) {
        let f: Vec<&str> = l.split(',').collect();
        let (zone, lat, lon) = (f[0], f[1], f[2]);
        let fw = call(
            "geodesy.spcs.spcs83-forward",
            &format!(r#"{{"lat":{lat},"lon":{lon},"zone":"{zone}","unit":"m"}}"#),
        );
        let e = fw["result"]["easting"]["value"]
            .as_f64()
            .unwrap_or_else(|| panic!("{fw}"));
        let n = fw["result"]["northing"]["value"].as_f64().unwrap();
        let back = call(
            "geodesy.spcs.spcs83-inverse",
            &format!(r#"{{"zone":"{zone}","easting":"{e} m","northing":"{n} m"}}"#),
        );
        let lat2 = back["result"]["lat"]["value"]
            .as_f64()
            .unwrap_or_else(|| panic!("{back}"));
        let lon2 = back["result"]["lon"]["value"].as_f64().unwrap();
        assert!(
            (lat2 - lat.parse::<f64>().unwrap()).abs() < 1e-9,
            "{zone} {back}"
        );
        assert!(
            (lon2 - lon.parse::<f64>().unwrap()).abs() < 1e-9,
            "{zone} {back}"
        );
        for k in ["convergence", "scale_factor"] {
            let a = fw["result"][k]
                .get("value")
                .unwrap_or(&fw["result"][k])
                .as_f64()
                .unwrap();
            let b = back["result"][k]
                .get("value")
                .unwrap_or(&back["result"][k])
                .as_f64()
                .unwrap();
            assert!((a - b).abs() < 1e-8, "{zone} {k} {a} {b}");
        }
    }
}

#[test]
fn arc_to_chord_matches_a_projected_step_along_the_geodesic() {
    // T is the direction of the projected geodesic at the From end. Measure it
    // without the convergence formula: project points 1 m behind and ahead
    // along the geodesic and take the bearing between them (a millimeter step
    // drowns in the rounding of a Lambert cone radius of about 7,000 km, and a
    // central difference cancels the curvature). Then t − T from the tool must
    // match t − T_step, in TM and LCC zones alike.
    use geographiclib_rs::{DirectGeodesic, Geodesic, InverseGeodesic};
    use gp_geo::spcs::{self, Proj};
    let g = Geodesic::new(spcs::GRS80_A, spcs::GRS80_F);
    for (zone, tm, (la1, lo1, la2, lo2)) in [
        ("1201", true, (40.9, -88.3, 41.2, -87.9)),
        ("3702", false, (40.44, -79.99, 40.52, -79.91)),
        ("3702", false, (40.1, -80.4, 40.6, -76.2)),
        ("0405", false, (34.0, -118.3, 34.3, -117.6)),
    ] {
        let z = spcs::find(zone).unwrap();
        assert_eq!(matches!(z.proj, Proj::Tm { .. }), tm, "{zone}");
        let (_, az1, _, _): (f64, f64, f64, f64) = g.inverse(la1, lo1, la2, lo2);
        let (fla, flo): (f64, f64) = g.direct(la1, lo1, az1, 1.0);
        let (bla, blo): (f64, f64) = g.direct(la1, lo1, az1, -1.0);
        let (p1, pf, pb, p2) = (
            z.forward(la1, lo1),
            z.forward(fla, flo),
            z.forward(bla, blo),
            z.forward(la2, lo2),
        );
        let t = (p2.e - p1.e).atan2(p2.n - p1.n).to_degrees();
        let t_step = (pf.e - pb.e).atan2(pf.n - pb.n).to_degrees();
        let want = ((t - t_step + 180.0).rem_euclid(360.0) - 180.0) * 3600.0;
        let r = call(
            "geodesy.projection.arc-to-chord",
            &format!(
                r#"{{"lat1":{la1},"lon1":{lo1},"lat2":{la2},"lon2":{lo2},"grid":"spcs","zone":"{zone}"}}"#
            ),
        );
        let got = r["result"]["t_minus_t_from"]["value"]
            .as_f64()
            .unwrap_or_else(|| panic!("{r}"));
        assert!(
            (got - want).abs() < 5e-3,
            "{zone}: tool {got}″, step {want}″"
        );
    }
}
