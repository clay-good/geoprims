//! Rhumb lines against GeographicLib's RhumbSolve (tools/vectors/gen_rhumb_diff.py):
//! 2,000 pairs including nearly east-west, nearly meridional, and near-pole cases.

use gp_geo::rhumb::Rhumb;

const WGS84: (f64, f64) = (6_378_137.0, 1.0 / 298.257_223_563);

fn adiff(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

#[test]
fn matches_rhumbsolve() {
    let r = Rhumb::new(WGS84.0, WGS84.1);
    // Starts within 0.01° of a pole spiral many times around it, so the end
    // longitude is ill-conditioned in the start: those get 20 µm, the rest 1 µm.
    let (mut ds, mut da, mut dpos, mut dpole, mut n) = (0.0f64, 0.0f64, 0.0f64, 0.0f64, 0);
    for l in include_str!("data/rhumb_diff.csv").lines().skip(1) {
        let f: Vec<f64> = l.split(',').map(|x| x.parse().unwrap()).collect();
        let (lat1, lon1, lat2, lon2, azi, s) = (f[0], f[1], f[2], f[3], f[4], f[5]);
        let (s2, azi2) = r.inverse(lat1, lon1, lat2, lon2);
        ds = ds.max((s2 - s).abs());
        da = da.max(adiff(azi2, azi));
        let e = r.direct(lat1, lon1, azi, s);
        assert_eq!(e.beyond_pole, 0.0);
        let m = 6_371_000.0f64.to_radians();
        let d = ((e.lat - lat2) * m).hypot(adiff(e.lon, lon2) * m * lat2.to_radians().cos());
        if lat1.abs() > 89.99 {
            dpole = dpole.max(d);
        } else {
            dpos = dpos.max(d);
        }
        n += 1;
    }
    println!(
        "{n} rows: distance {ds:e} m, course {da:e}°, direct position {dpos:e} m ({dpole:e} m near a pole)"
    );
    assert!(n == 2000);
    assert!(ds <= 1e-6, "distance {ds} m");
    assert!(da <= 1e-9, "course {da}°");
    assert!(dpos <= 1e-6, "direct position {dpos} m");
    assert!(dpole <= 2e-5, "direct position near a pole {dpole} m");
}
