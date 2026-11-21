//! RDNAPTRANS™2018 against the check point the national-grids specification
//! states, read from the committed packages the hosts serve.

use gp_geo::hgrid::Grids;
use gp_geo::rdnap;
use gp_geo::vgrid;

fn read(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/../../../assets/data/rdnaptrans/2018/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("the committed {name}: {e}"))
}

#[test]
fn the_nsgi_check_point_both_ways() {
    let h = read("rdtrans2018.grid");
    let v = read("nlgeo2018.grid");
    let grids = Grids::parse(&h).expect("the correction grid parses");
    let geoid = vgrid::Grid::parse(&v).expect("the quasi-geoid parses");

    // 52°N 5°E, ellipsoidal height 43 m on ETRS89.
    let (p, step) = rdnap::to_rd(&grids, 52.0, 5.0).expect("inside the grid");
    assert!((p.e - 128_410.095_8).abs() < 1e-4, "easting {}", p.e);
    assert!((p.n - 445_806.496_0).abs() < 1e-4, "northing {}", p.n);
    // The iteration settles, and the point in real Bessel is about 100 m away.
    assert!(
        step.iterations >= 2 && step.iterations <= 6,
        "{}",
        step.iterations
    );
    let n = rdnap::quasi_geoid(&geoid, 52.0, 5.0).expect("inside the quasi-geoid");
    let nap = 43.0 - n;
    assert!((nap + 0.4754).abs() < 1e-4, "NAP {nap} from N {n}");

    // And back: the same point, to a nanodegree and a tenth of a millimetre.
    let ((lat, lon), _) = rdnap::to_etrs89(&grids, 128_410.095_8, 445_806.496_0).expect("inside");
    assert!((lat - 52.0).abs() < 1e-9, "latitude {lat}");
    assert!((lon - 5.0).abs() < 1e-9, "longitude {lon}");
    let back = nap + rdnap::quasi_geoid(&geoid, lat, lon).expect("inside");
    assert!((back - 43.0).abs() < 1e-4, "height {back}");
}
