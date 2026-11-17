//! The OSi/LPS polynomial and OSGM15's Belfast geoid against the Northern
//! Ireland test point the specification names, NI_IG_1 of Ordnance Survey's
//! OSGM15 test files, read from the committed package.

use gp_geo::osi::{self, Geoid};
use gp_geo::ostn15::irish_grid;

fn package(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/../../../assets/data/osgm15-ireland/2016/{name}.grid",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("the committed OSGM15 Ireland package")
}

#[test]
fn ni_test_point_both_ways() {
    // IG 223657.156, 340618.902, 37.958 m on Belfast gives 54°18'49.673618"N
    // 7°38'14.133209"W, 95.4675 m.
    let (lat, lon) = irish_grid().inverse(223_657.156, 340_618.902);
    let (elat, elon) = osi::to_etrs89(lat, lon);
    let want_lat = 54.0 + 18.0 / 60.0 + 49.673_618 / 3600.0;
    let want_lon = -(7.0 + 38.0 / 60.0 + 14.133_209 / 3600.0);
    assert!((elat - want_lat).abs() * 3600.0 < 5e-7, "{elat}");
    assert!((elon - want_lon).abs() * 3600.0 < 5e-7, "{elon}");
    let bytes = package("belfast");
    let g = Geoid::parse(&bytes).unwrap();
    let h = 37.958 + g.height(elat, elon).unwrap();
    assert!((h - 95.4675).abs() < 1e-3, "{h}");

    // And back: the iteration returns the TM75 point the polynomial came from.
    let (blat, blon, k) = osi::to_tm75(elat, elon);
    assert!(
        (blat - lat).abs() < 1e-10 && (blon - lon).abs() < 1e-10,
        "{blat} {blon}"
    );
    assert!(k < 10, "{k} iterations");
    let p = irish_grid().forward(blat, blon);
    assert!(
        (p.e - 223_657.156).abs() < 1e-4 && (p.n - 340_618.902).abs() < 1e-4,
        "{p:?}"
    );
}

#[test]
fn the_geoid_grid_is_bounded_and_a_bad_package_refused() {
    let bytes = package("malin");
    let g = Geoid::parse(&bytes).unwrap();
    assert!(g.geodetic(50.9, -8.0).is_none());
    assert!(g.geodetic(53.0, -4.9).is_none());
    // Corners of the grid read their own nodes.
    assert!(g.geodetic(56.0, -11.5).is_some() && g.geodetic(51.0, -5.0).is_some());
    assert!(Geoid::parse(&bytes[..bytes.len() - 2]).is_err());
}
