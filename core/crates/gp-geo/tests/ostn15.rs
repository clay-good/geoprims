//! OSTN15 and OSGM15 against Ordnance Survey's own worked example, Caister
//! Water Tower (Transformations and OSGM15 User Guide v1.3, annexe A), step by
//! step, read from the committed package the hosts serve.

use gp_geo::ostn15::{Grid, airy_grid, etrs89_grid};

fn package() -> Vec<u8> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../assets/data/ostn15/2016/ostn15-osgm15.grid"
    );
    std::fs::read(path).expect("the committed OSTN15 package")
}

#[test]
fn caister_water_tower_step_by_step() {
    let bytes = package();
    let grid = Grid::parse(&bytes).expect("package parses");

    // Step 1: 52°39'28.8282"N 1°42'57.8663"E onto the ETRS89 National Grid
    // projection. OS prints 651 307.0030, 313 255.6859.
    let lat = 52.0 + 39.0 / 60.0 + 28.8282 / 3600.0;
    let lon = 1.0 + 42.0 / 60.0 + 57.8663 / 3600.0;
    let p = etrs89_grid().forward(lat, lon);
    assert!((p.e - 651_307.003_0).abs() < 1e-4, "{}", p.e);
    assert!((p.n - 313_255.685_9).abs() < 1e-4, "{}", p.n);

    // Step 2: the cell's four records and their values, as OS lists them.
    let (x, y) = (651_307.003, 313_255.686);
    let (e, n, h, s) = grid.forward(x, y, 108.05).expect("inside");
    assert_eq!(s.records, [220_065, 220_066, 220_767, 220_766]);
    assert_eq!(s.corners[0], [102.787, -78.242, 44.236]);
    assert_eq!(s.corners[1], [102.825, -78.244, 44.221]);
    assert_eq!(s.corners[2], [102.834, -78.225, 44.210]);
    assert_eq!(s.corners[3], [102.795, -78.213, 44.224]);
    // The shifts to the millimetre OS prints, and the answer.
    assert!(
        (s.se - 102.801).abs() < 5e-4 && (s.sn + 78.236).abs() < 5e-4,
        "{s:?}"
    );
    assert!((s.sg - 44.228).abs() < 5e-4, "{s:?}");
    assert!(
        (e - 651_409.804).abs() < 5e-4 && (n - 313_177.450).abs() < 5e-4,
        "{e} {n}"
    );
    assert!((h - 63.822).abs() < 5e-4, "{h}");
    assert_eq!(s.flag, 1, "Newlyn");

    // The inverse, iteration by iteration, as OS tabulates it.
    let steps = grid
        .inverse(651_409.804, 313_177.450, 63.822)
        .expect("inside");
    let want = [
        (
            102.8041,
            -78.2384,
            44.2278,
            651_306.999_9,
            313_255.688_4,
            108.0498,
        ),
        (
            102.8008,
            -78.2360,
            44.2284,
            651_307.003_2,
            313_255.686_0,
            108.0504,
        ),
        (
            102.8008,
            -78.2360,
            44.2284,
            651_307.003_2,
            313_255.686_0,
            108.0504,
        ),
    ];
    assert_eq!(steps.len(), want.len(), "{steps:?}");
    for ((x, y, z, s), w) in steps.iter().zip(want) {
        assert!(
            (s.se - w.0).abs() < 5e-5 && (s.sn - w.1).abs() < 5e-5 && (s.sg - w.2).abs() < 5e-5,
            "{s:?} {w:?}"
        );
        assert!(
            (x - w.3).abs() < 5e-5 && (y - w.4).abs() < 5e-5 && (z - w.5).abs() < 5e-5,
            "{x} {y} {z} {w:?}"
        );
    }
    // And back to OS's latitude and longitude. The iteration stops when the
    // shifts move by under 0.1 mm, so its easting is 651 307.0032, not the
    // 651 307.003 that went in: 0.3 mm, or 4e-9 degrees of longitude here,
    // which is why this is not held tighter.
    let (x, y, _, _) = steps[steps.len() - 1];
    let (blat, blon) = etrs89_grid().inverse(x, y);
    assert!(
        (blat - 52.658_007_833).abs() < 1e-8 && (blon - 1.716_073_972).abs() < 1e-8,
        "{blat} {blon}"
    );
}

#[test]
fn the_projection_reproduces_both_of_os_worked_examples() {
    // Annexe B's own example, on Airy 1830: 52°39'27.2531"N 1°43'4.5177"E
    // gives E 651 409.903, N 313 177.270.
    let lat = 52.0 + 39.0 / 60.0 + 27.2531 / 3600.0;
    let lon = 1.0 + 43.0 / 60.0 + 4.5177 / 3600.0;
    let p = airy_grid().forward(lat, lon);
    assert!(
        (p.e - 651_409.903).abs() < 5e-4 && (p.n - 313_177.270).abs() < 5e-4,
        "{p:?}"
    );
    // And annexe C reads it back.
    let (b, l) = airy_grid().inverse(651_409.903, 313_177.270);
    assert!((b - lat).abs() < 1e-8 && (l - lon).abs() < 1e-8, "{b} {l}");
}

#[test]
fn outside_the_grid_is_none_and_a_bad_package_is_refused() {
    let bytes = package();
    let grid = Grid::parse(&bytes).unwrap();
    assert!(grid.shift(-1.0, 100.0).is_none());
    assert!(grid.shift(700_000.0, 100.0).is_none());
    assert!(grid.shift(100.0, 1_250_000.0).is_none());
    assert!(Grid::parse(&bytes[..bytes.len() - 1]).is_err());
    assert!(Grid::parse(b"NADCON5 1 2 3\n").is_err());
}
