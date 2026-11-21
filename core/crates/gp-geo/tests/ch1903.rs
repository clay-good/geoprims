//! CH1903+ to ETRS89 against swisstopo's own numerical example (Formulas and
//! constants, December 2016, section 7.1), which prints every intermediate
//! for the five EUREF points of Switzerland: the ellipsoidal coordinates on
//! CH1903+, the geocentric coordinates on each side of the translation, and
//! the ellipsoidal coordinates on ETRS89.

use gp_geo::ch1903;

/// Degrees, minutes and decimal seconds to degrees.
fn dms(d: f64, m: f64, s: f64) -> f64 {
    d + m / 60.0 + s / 3600.0
}

#[test]
fn swisstopos_five_euref_points_with_every_intermediate() {
    // name, CH1903+ (lon, lat, h), its geocentric coordinates, the ETRS89
    // geocentric coordinates, and the ETRS89 (lon, lat, h). All as printed.
    let points = [
        (
            "Zimmerwald",
            (
                dms(7.0, 27.0, 58.416328),
                dms(46.0, 52.0, 42.269284),
                897.361,
            ),
            (4_330_616.737, 567_539.766, 4_632_721.664),
            (4_331_291.111, 567_554.822, 4_633_127.010),
            (
                dms(7.0, 27.0, 54.983506),
                dms(46.0, 52.0, 37.540562),
                947.149,
            ),
        ),
        (
            "Chrischona",
            (
                dms(7.0, 40.0, 10.574820),
                dms(47.0, 34.0, 6.404965),
                457.138,
            ),
            (4_272_473.562, 575_353.239, 4_684_498.293),
            (4_273_147.936, 575_368.294, 4_684_903.639),
            (dms(7.0, 40.0, 6.983077), dms(47.0, 34.0, 1.385301), 504.935),
        ),
        (
            "Pfaender",
            (
                dms(9.0, 47.0, 8.465989),
                dms(47.0, 31.0, 0.092644),
                1043.616,
            ),
            (4_252_889.174, 733_507.303, 4_681_046.757),
            (4_253_563.548, 733_522.359, 4_681_452.103),
            (
                dms(9.0, 47.0, 3.697723),
                dms(47.0, 30.0, 55.172797),
                1089.372,
            ),
        ),
        (
            "La Givrine",
            (
                dms(6.0, 6.0, 9.983811),
                dms(46.0, 27.0, 19.272743),
                1206.367,
            ),
            (4_377_121.142, 467_993.592, 4_600_671.934),
            (4_377_795.516, 468_008.648, 4_601_077.280),
            (
                dms(6.0, 6.0, 7.326361),
                dms(46.0, 27.0, 14.690021),
                1258.274,
            ),
        ),
        (
            "Monte Generoso",
            (
                dms(9.0, 1.0, 20.606368),
                dms(45.0, 55.0, 49.707052),
                1634.472,
            ),
            (4_389_483.221, 696_984.352, 4_560_589.600),
            (4_390_157.595, 696_999.408, 4_560_994.946),
            (
                dms(9.0, 1.0, 16.389053),
                dms(45.0, 55.0, 45.438020),
                1685.027,
            ),
        ),
    ];
    // A thousandth of a second of arc is 3 cm; swisstopo prints six decimals
    // of a second, so the published values are held to a tenth of a
    // millimetre on the ground and the geocentric ones to the millimetre it
    // prints.
    let arcsec = 1.0 / 3600.0;
    for (name, plus, plus_ecef, etrs_ecef, etrs) in points {
        // CH1903+ to ETRS89: every intermediate swisstopo prints.
        let s = ch1903::transform(plus.1, plus.0, plus.2, false).unwrap();
        let ((lat, lon, h), a, b) = (s.point, s.from, s.to);
        assert!(
            (a.0 - plus_ecef.0).abs() < 0.001,
            "{name}: X {} {}",
            a.0,
            plus_ecef.0
        );
        assert!(
            (a.1 - plus_ecef.1).abs() < 0.001,
            "{name}: Y {} {}",
            a.1,
            plus_ecef.1
        );
        assert!(
            (a.2 - plus_ecef.2).abs() < 0.001,
            "{name}: Z {} {}",
            a.2,
            plus_ecef.2
        );
        assert!(
            (b.0 - etrs_ecef.0).abs() < 0.001,
            "{name}: X' {} {}",
            b.0,
            etrs_ecef.0
        );
        assert!(
            (b.1 - etrs_ecef.1).abs() < 0.001,
            "{name}: Y' {} {}",
            b.1,
            etrs_ecef.1
        );
        assert!(
            (b.2 - etrs_ecef.2).abs() < 0.001,
            "{name}: Z' {} {}",
            b.2,
            etrs_ecef.2
        );
        assert!(
            (lat - etrs.1).abs() < 1e-5 * arcsec,
            "{name}: latitude {lat} against {}",
            etrs.1
        );
        assert!(
            (lon - etrs.0).abs() < 1e-5 * arcsec,
            "{name}: longitude {lon} against {}",
            etrs.0
        );
        assert!(
            (h - etrs.2).abs() < 0.001,
            "{name}: height {h} against {}",
            etrs.2
        );

        // And back, which is section 7.2: the same numbers in reverse order.
        let (p, l, hh) = ch1903::transform(etrs.1, etrs.0, etrs.2, true)
            .unwrap()
            .point;
        assert!(
            (p - plus.1).abs() < 1e-5 * arcsec,
            "{name}: back {p} {}",
            plus.1
        );
        assert!(
            (l - plus.0).abs() < 1e-5 * arcsec,
            "{name}: back {l} {}",
            plus.0
        );
        assert!((hh - plus.2).abs() < 0.001, "{name}: back {hh} {}", plus.2);
    }
}
