//! CHENyx06 against swisstopo's own numerical example: the five EUREF points
//! of Switzerland, LV03 to LV95, read from the committed package.

use gp_geo::chenyx06;
use gp_geo::hgrid::Grid;

fn package() -> Vec<u8> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../assets/data/chenyx06/2009/chenyx06a.grid"
    );
    std::fs::read(path).expect("the committed CHENyx06 package")
}

#[test]
fn swisstopos_five_euref_points_both_ways() {
    let bytes = package();
    let grid = Grid::parse(&bytes).expect("package parses");
    // swisstopo, "Formulas and constants…" (December 2016), section 7.1: the
    // five EUREF points in LV03 and the LV95 coordinates its own FINELTRA
    // transformation of CHENyx06 gives for them.
    for (name, e03, n03, e95, n95) in [
        (
            "Zimmerwald",
            602_030.680,
            191_775.030,
            2_602_030.740,
            1_191_775.030,
        ),
        (
            "Chrischona",
            617_306.300,
            268_507.300,
            2_617_306.920,
            1_268_507.870,
        ),
        (
            "Pfaender",
            776_668.105,
            265_372.681,
            2_776_668.590,
            1_265_372.250,
        ),
        (
            "La Givrine",
            497_313.292,
            145_625.438,
            2_497_312.650,
            1_145_626.140,
        ),
        (
            "Monte Generoso",
            722_758.810,
            87_649.670,
            2_722_759.060,
            1_087_648.190,
        ),
    ] {
        let ((e, n), step) = chenyx06::to_lv95(&grid, e03, n03).expect("inside the grid");
        // The grid is swisstopo's approximation of FINELTRA, so it reproduces
        // its published results to 8.1 mm, not exactly.
        assert!(
            (e - e95).abs() < 0.0081,
            "{name}: easting {e} against {e95}"
        );
        assert!(
            (n - n95).abs() < 0.0081,
            "{name}: northing {n} against {n95}"
        );
        // The distortion the grid carries is metres, not the round 2,000 km.
        let d = ((e - LV95_OF(e03)).hypot(n - LV95_OF_N(n03))).abs();
        assert!(d < 2.0, "{name}: {d} m of distortion");
        // The offsets it read are seconds, not degrees or radians.
        assert!(
            step.offsets.0.abs() < 0.1 && step.offsets.1.abs() < 0.1,
            "{name}: {:?}",
            step.offsets
        );

        // And back onto LV03, to a tenth of a millimetre.
        let ((b, c), back) = chenyx06::to_lv03(&grid, e, n).expect("inside the grid");
        assert!((b - e03).abs() < 1e-4, "{name}: back to {b} from {e03}");
        assert!((c - n03).abs() < 1e-4, "{name}: back to {c} from {n03}");
        assert!(
            back.iterations >= 1 && back.iterations <= 6,
            "{name}: {} iterations",
            back.iterations
        );
    }
}

#[allow(non_snake_case)]
fn LV95_OF(e03: f64) -> f64 {
    e03 + chenyx06::LV03_EAST
}
#[allow(non_snake_case)]
fn LV95_OF_N(n03: f64) -> f64 {
    n03 + chenyx06::LV03_NORTH
}
