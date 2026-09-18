//! H3 C parity: indexes identical, and centers within 1e-12° of arc of H3 C
//! below 88° latitude (5e-11° nearer the poles, where operation order in the
//! trigonometry differs). 100,000 points × 16 resolutions against H3 C 4.4.1:
//! 0 index mismatches, worst center gap 4.5e-13° below 88° and 1.8e-11° above.
//! The committed fixture has 250 points × 16 resolutions; the full
//! 100,000-point run is `H3_DIFF=/path/full.csv cargo test -- --ignored`
//! with a file from tools/vectors/gen_h3_diff.py.

use h3o::{LatLng, Resolution};

/// Returns (comparisons, index mismatches, worst center gap as arc degrees,
/// worst gap below 88° latitude).
fn check(text: &str) -> (usize, Vec<String>, f64, f64) {
    let mut bad = Vec::new();
    let (mut n, mut worst, mut worst_mid) = (0, 0.0f64, 0.0f64);
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = line.split(',').collect();
        let (lat, lng): (f64, f64) = (f[0].parse().unwrap(), f[1].parse().unwrap());
        let res = Resolution::try_from(f[2].parse::<u8>().unwrap()).unwrap();
        let cell = LatLng::new(lat, lng).unwrap().to_cell(res);
        let center = LatLng::from(cell);
        let (clat, clng): (f64, f64) = (f[4].parse().unwrap(), f[5].parse().unwrap());
        let parent = cell.parent(Resolution::Zero).unwrap();
        if cell.to_string() != f[3] || parent.to_string() != f[6] {
            bad.push(format!("{line} -> {cell}"));
        }
        // Longitude is ill-conditioned near the poles: compare arc length.
        let dlng = (center.lng() - clng + 540.0) % 360.0 - 180.0;
        let gap = (center.lat() - clat).hypot(dlng * clat.to_radians().cos());
        worst = worst.max(gap);
        if clat.abs() < 88.0 {
            worst_mid = worst_mid.max(gap);
        }
        n += 1;
    }
    (n, bad, worst, worst_mid)
}

#[test]
fn committed_fixture() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/h3_c_diff.csv"
    ))
    .unwrap();
    let (n, bad, worst, worst_mid) = check(&text);
    assert_eq!(n, 4000);
    assert!(
        bad.is_empty(),
        "{} index mismatches:\n{}",
        bad.len(),
        bad[..bad.len().min(20)].join("\n")
    );
    assert!(
        worst < 5e-11 && worst_mid < 1e-12,
        "center gap {worst:e}° ({worst_mid:e}° below 88°)"
    );
}

#[test]
#[ignore = "needs a 100,000-point file from tools/vectors/gen_h3_diff.py in H3_DIFF"]
fn full_differential() {
    let path = std::env::var("H3_DIFF").expect("set H3_DIFF");
    let (n, bad, worst, worst_mid) = check(&std::fs::read_to_string(path).unwrap());
    println!(
        "{n} comparisons, {} index mismatches, worst center gap {worst:e}° ({worst_mid:e}° below 88°)",
        bad.len()
    );
    assert!(
        bad.is_empty(),
        "{} index mismatches:\n{}",
        bad.len(),
        bad[..bad.len().min(20)].join("\n")
    );
    assert!(worst < 5e-11 && worst_mid < 1e-12);
}
