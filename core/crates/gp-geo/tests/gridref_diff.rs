//! GARS and GEOREF against GeographicLib's C++ classes: encodes at every
//! precision and decodes of each code (south-west corner and center), on
//! seeded random points plus the poles and the antimeridian. Data from
//! tools/vectors/gen_gridref.py.

use gp_geo::gridref::{gars_decode, gars_encode, georef_decode, georef_encode};

#[test]
fn matches_geographiclib() {
    let data = include_str!("data/gridref_diff.txt");
    let (mut n, mut worst) = (0, 0f64);
    for line in data.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        let (lat, lon, prec): (f64, f64, i32) = (
            f[1].parse().unwrap(),
            f[2].parse().unwrap(),
            f[3].parse().unwrap(),
        );
        let code = f[4];
        let want: Vec<f64> = f[5..9].iter().map(|x| x.parse().unwrap()).collect();
        let (got, cell) = if f[0] == "g" {
            (
                gars_encode(lat, lon, prec as usize),
                gars_decode(code).unwrap(),
            )
        } else {
            (
                georef_encode(lat, lon, prec),
                georef_decode(code).unwrap().0,
            )
        };
        assert_eq!(got, code, "{line}");
        let (clat, clon) = cell.center();
        for (g, w) in [cell.south, cell.west, clat, clon].into_iter().zip(&want) {
            worst = worst.max((g - w).abs());
            assert!((g - w).abs() < 1e-12, "{line}: {g} vs {w}");
        }
        n += 1;
    }
    assert!(n > 5_000, "{n} rows");
    eprintln!("{n} rows, worst decode difference {worst:e}°");
}
