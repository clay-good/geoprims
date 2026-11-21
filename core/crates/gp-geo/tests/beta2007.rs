//! BeTA2007 against AdV's own published test records, read from the committed
//! package. AdV's documentation (BETA2007dokumentation, version 1.5, section
//! 4.2) prints the first six records of its 31-point test set: for each of two
//! points, the pure NTv2 shift from DHDN to ETRS89 in degrees, and the point
//! on two Gauss-Krüger zones carried to ETRS89 UTM zone 32.

use gp_geo::beta2007;
use gp_geo::hgrid::Grid;

fn package() -> Vec<u8> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../assets/data/beta2007/2009/beta2007.grid"
    );
    std::fs::read(path).expect("the committed BeTA2007 package")
}

#[test]
fn advs_published_records_both_ways() {
    let bytes = package();
    let grid = Grid::parse(&bytes).expect("package parses");

    // Record 1 of each point: the grid alone, DHDN to ETRS89, printed to
    // twelve decimals of a degree.
    for (lon, lat, elon, elat) in [
        (
            7.483_333_333_333,
            53.5,
            7.482_506_019_176,
            53.498_461_143_331,
        ),
        (
            10.466_666_666_667,
            52.5,
            10.465_380_298_337,
            52.498_573_633_365,
        ),
    ] {
        let (a, b) = grid.forward(lat, lon).expect("inside the grid");
        // AdV computed its table with PROJ 4.4.7 and printed twelve
        // decimals; this lands within 2e-11 degrees, 2 micrometres.
        assert!((a - elat).abs() < 2e-11, "latitude {a} against {elat}");
        assert!((b - elon).abs() < 2e-11, "longitude {b} against {elon}");
    }

    // The four Gauss-Krüger records, each to ETRS89 UTM zone 32, printed to a
    // micrometre. The same ground point is given on two zones, and both land
    // on the same UTM coordinate.
    for (gk, east, north, eu, nu) in [
        (
            2u8,
            2_598_417.333_192,
            5_930_677.980_308,
            399_340.601_863,
            5_928_794.177_992,
        ),
        (
            3,
            3_399_371.190_396,
            5_930_724.531_323,
            399_340.601_862,
            5_928_794.177_992,
        ),
        (
            3,
            3_599_586.686_397,
            5_819_391.659_845,
            599_474.934_168,
            5_817_502.626_999,
        ),
        (
            4,
            4_395_886.918_912,
            5_819_485.694_352,
            599_474.934_169,
            5_817_502.626_999,
        ),
    ] {
        // The easting states its own zone.
        assert_eq!(beta2007::gk_zone_of(east), Some(gk), "zone of {east}");
        let ((e, n), p, step) =
            beta2007::to_utm(&grid, gk, 32, east, north).expect("inside the grid");
        assert!((e - eu).abs() < 1e-5, "zone {gk}: easting {e} against {eu}");
        assert!(
            (n - nu).abs() < 1e-5,
            "zone {gk}: northing {n} against {nu}"
        );
        // UTM's scale is near its 0.9996 on the central meridian and grows
        // away from it; Germany never leaves 0.9996 to 1.0004.
        assert!((0.9996..1.0004).contains(&p.k), "zone {gk}: scale {}", p.k);
        // The grid's offsets are arcseconds, and the shift is metres, not the
        // hundreds of metres a wrong ellipsoid would give.
        assert!(
            step.offsets.0.abs() < 20.0 && step.offsets.1.abs() < 20.0,
            "{:?}",
            step.offsets
        );

        // And back onto the same Gauss-Krüger zone, to a tenth of a millimetre.
        let ((b, c), _, back) =
            beta2007::to_gauss_kruger(&grid, 32, gk, e, n).expect("inside the grid");
        assert!(
            (b - east).abs() < 1e-4,
            "zone {gk}: back to {b} from {east}"
        );
        assert!(
            (c - north).abs() < 1e-4,
            "zone {gk}: back to {c} from {north}"
        );
        assert!(
            (1..=6).contains(&back.iterations),
            "{} iterations",
            back.iterations
        );
    }

    // The zone an easting states, and what it refuses: no zone digits, a zone
    // Germany does not use, and a point 400 km off the zone's meridian.
    assert_eq!(beta2007::gk_zone_of(598_417.0), None);
    assert_eq!(beta2007::gk_zone_of(7_598_417.0), None);
    assert_eq!(beta2007::gk_zone_of(3_950_000.0), None);
    assert_eq!(beta2007::utm_zone_of(7.5), 32);
    assert_eq!(beta2007::utm_zone_of(13.4), 33);
    // A three-degree zone runs 1.5° either side of its meridian, so 7.4°E is
    // zone 2 (meridian 6°E) and 7.5°E is the edge, which belongs to zone 3.
    assert_eq!(beta2007::gk_zone_for(7.4), 2);
    assert_eq!(beta2007::gk_zone_for(7.5), 3);
    assert_eq!(beta2007::gk_zone_for(13.4), 4);
    assert_eq!(beta2007::gk_zone_for(5.9), 2);
}
