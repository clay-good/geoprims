//! The exact geodesic method against GeographicLib's GeodSolve -E on
//! ellipsoids with f from 1/40 to 1/2 (tests/data/exact_diff.txt, from
//! tools/vectors/gen_exact_diff.py): both the inverse and the direct problem,
//! with the reduced length, geodesic scales, and area.

use gp_geo::exact::Exact;

#[test]
fn matches_geodsolve_exact() {
    let text = include_str!("data/exact_diff.txt");
    let (mut worst_s, mut worst_az, mut worst_pos, mut worst_m, mut worst_mm, mut worst_area) =
        (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64, 0.0f64);
    let mut n = 0;
    for line in text.lines() {
        let (kind, rest) = line.split_once(' ').unwrap();
        let v: Vec<f64> = rest
            .split_whitespace()
            .map(|x| x.parse().unwrap())
            .collect();
        let [
            f,
            lat1,
            lon1,
            azi1,
            lat2,
            lon2,
            azi2,
            s12,
            a12,
            m12,
            mm12,
            mm21,
            area,
        ] = v[..]
        else {
            panic!("{line}");
        };
        let _ = a12;
        let e = Exact::new(6_378_137.0, f);
        let ang = |x: f64, y: f64| ((x - y + 540.0).rem_euclid(360.0) - 180.0).abs();
        // Direct from the start and azimuth, for every line.
        let d = e.direct(lat1, lon1, azi1, s12);
        worst_pos = worst_pos.max((d.lat2 - lat2).abs()).max(ang(d.lon2, lon2));
        worst_m = worst_m.max((d.m12 - m12).abs() / s12.max(1.0));
        worst_mm = worst_mm
            .max((d.big_m12 - mm12).abs())
            .max((d.big_m21 - mm21).abs());
        worst_area = worst_area.max((d.area12 - area).abs() / (6_378_137.0f64.powi(2)));
        n += 1;
        // The inverse only where the line is the shortest geodesic.
        if kind != "I" {
            continue;
        }
        let r = e
            .inverse(lat1, lon1, lat2, lon2)
            .unwrap_or_else(|| panic!("no inverse: {line}"));
        worst_s = worst_s.max((r.s12 - s12).abs() / s12.max(1.0));
        worst_az = worst_az.max(ang(r.azi1, azi1)).max(ang(r.azi2, azi2));
        worst_area = worst_area.max((r.area12 - area).abs() / (6_378_137.0f64.powi(2)));
    }
    println!(
        "{n}: s {worst_s:e}, az {worst_az:e}°, pos {worst_pos:e}°, m12 {worst_m:e}, M {worst_mm:e}, area {worst_area:e} a²"
    );
    assert!(n >= 400);
    assert!(worst_s < 1e-12, "distance {worst_s:e}");
    assert!(worst_az < 1e-9, "azimuth {worst_az:e}");
    assert!(worst_pos < 1e-10, "position {worst_pos:e}");
    assert!(worst_m < 1e-12, "reduced length {worst_m:e}");
    assert!(worst_mm < 1e-12, "scale {worst_mm:e}");
    assert!(worst_area < 1e-12, "area {worst_area:e}");
}
