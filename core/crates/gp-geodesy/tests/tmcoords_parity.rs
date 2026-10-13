//! Exact transverse Mercator parity with the standard-domain part of
//! GeographicLib's published `TMcoords.dat` test set. The first 258,000 cases
//! are the cases GeographicLib designates for implementations following the
//! standard convention. The remaining 29,000 exercise its optional extended
//! domain, which this implementation deliberately does not expose. The data
//! use WGS 84, central meridian 0, central scale 0.9996, and no false origin;
//! GeographicLib computed the reference values with 80-digit arithmetic.
//!
//! The 34 MB source file is not committed. Download the archive whose digest
//! is pinned in `tests/data/README.md`, decompress it, then run:
//!
//! `TMCOORDS=/path/TMcoords.dat cargo test -p gp-geodesy --test tmcoords_parity -- --ignored --nocapture`

use gp_geo::tmexact::TmExactGrid;

fn ground_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let dlon = ((lon2 - lon1 + 540.0) % 360.0) - 180.0;
    let x = dlon.to_radians() * ((lat1 + lat2) / 2.0).to_radians().cos();
    let y = (lat2 - lat1).to_radians();
    6_378_137.0 * x.hypot(y)
}

#[test]
#[ignore = "needs GeographicLib's complete TMcoords.dat file in TMCOORDS"]
fn geographiclib_tmcoords_standard_domain() {
    let path = std::env::var("TMCOORDS").expect("set TMCOORDS to the decompressed TMcoords.dat");
    let text = std::fs::read_to_string(path).expect("read TMCOORDS");
    let grid = TmExactGrid::new(
        6_378_137.0,
        1.0 / 298.257_223_563,
        0.0,
        0.0,
        0.9996,
        0.0,
        0.0,
    );
    let (mut count, mut dxy, mut dll, mut dg, mut dk) = (0usize, 0.0f64, 0.0f64, 0.0f64, 0.0f64);
    let (mut dxy_at, mut dll_at, mut dg_at, mut dk_at) =
        (String::new(), String::new(), String::new(), String::new());

    for (line_no, line) in text.lines().take(258_000).enumerate() {
        let fields: Vec<f64> = line
            .split_whitespace()
            .map(|field| {
                field
                    .parse()
                    .unwrap_or_else(|_| panic!("line {}: {field}", line_no + 1))
            })
            .collect();
        assert_eq!(fields.len(), 6, "line {}: {line}", line_no + 1);
        let (lat, lon, x, y, gamma, scale) = (
            fields[0], fields[1], fields[2], fields[3], fields[4], fields[5],
        );

        let (got_x, got_y, got_gamma, got_scale) = grid.forward(lat, lon);
        assert!(
            [got_x, got_y, got_gamma, got_scale]
                .into_iter()
                .all(f64::is_finite),
            "line {} produced a non-finite forward result",
            line_no + 1
        );
        // GeographicLib specifies position error as ground distance: grid
        // error divided by the point scale. Longitude and convergence are
        // ill-conditioned at the pole; convergence and scale are also
        // ill-conditioned at the branch point. Check those two directly only
        // outside a 1e-6° neighborhood of either singularity.
        let this_dxy = (got_x - x).hypot(got_y - y) / scale;
        let singular =
            90.0 - lat < 1e-6 || (lat < 1e-6 && (lon - 82.636_272_824_164_07).abs() < 1e-6);
        let this_dg = if singular {
            0.0
        } else {
            (got_gamma - gamma).abs()
        };
        let this_dk = if singular {
            0.0
        } else {
            ((got_scale - scale) / scale).abs()
        };
        if this_dxy > dxy {
            dxy = this_dxy;
            dxy_at = format!("line {}: {line}", line_no + 1);
        }
        if this_dg > dg {
            dg = this_dg;
            dg_at = format!("line {}: {line}", line_no + 1);
        }
        if this_dk > dk {
            dk = this_dk;
            dk_at = format!("line {}: {line}", line_no + 1);
        }

        let (got_lat, got_lon) = grid.inverse(x, y);
        let this_dll = ground_distance(lat, lon, got_lat, got_lon);
        if this_dll > dll {
            dll = this_dll;
            dll_at = format!("line {}: {line}", line_no + 1);
        }
        count += 1;
    }

    println!(
        "{count} cases; forward ground error {dxy:e} m at {dxy_at}; inverse ground error {dll:e} m at {dll_at}; nonsingular convergence {dg:e} deg at {dg_at}; nonsingular relative scale {dk:e} at {dk_at}"
    );
    assert_eq!(
        text.lines().count(),
        287_000,
        "not the complete published test set"
    );
    assert_eq!(count, 258_000, "not the complete standard-domain subset");
    assert!(dxy <= 1e-7, "forward ground error {dxy:e} m");
    assert!(dll <= 1e-7, "inverse ground error {dll:e} m");
    assert!(dg <= 1e-6, "convergence {dg:e} deg");
    assert!(dk <= 1e-12, "relative scale {dk:e}");
}
