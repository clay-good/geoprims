//! The Open Location Code project's official test data, every case.

use gp_indexing::codes;

fn rows(name: &str) -> Vec<Vec<String>> {
    let path = format!("{}/tests/data/olc/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path)
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| l.split(',').map(str::to_owned).collect())
        .collect()
}

#[test]
fn encoding() {
    let mut bad = Vec::new();
    for r in rows("encoding.csv") {
        let (lat, lng, len): (f64, f64, usize) = (
            r[0].parse().unwrap(),
            r[1].parse().unwrap(),
            r[4].parse().unwrap(),
        );
        let got = codes::olc_encode(lat, lng, len);
        if got != r[5] {
            bad.push(format!("{lat},{lng},{len}: got {got}, want {}", r[5]));
        }
    }
    assert!(
        bad.is_empty(),
        "{} failures:\n{}",
        bad.len(),
        bad.join("\n")
    );
}

#[test]
fn decoding() {
    let mut bad = Vec::new();
    for r in rows("decoding.csv") {
        let code = &r[0];
        let len: usize = r[1].parse().unwrap();
        let want: Vec<f64> = r[2..6].iter().map(|v| v.parse().unwrap()).collect();
        let ((s, w, n, e), got_len) = codes::olc_decode(code);
        let ok = [s, w, n, e]
            .iter()
            .zip(&want)
            .all(|(a, b)| (a - b).abs() <= 1e-10)
            && got_len.min(15) == len;
        if !ok {
            bad.push(format!(
                "{code}: got {s},{w},{n},{e} len {got_len}, want {want:?} len {len}"
            ));
        }
    }
    assert!(
        bad.is_empty(),
        "{} failures:\n{}",
        bad.len(),
        bad.join("\n")
    );
}

#[test]
fn validity() {
    let mut bad = Vec::new();
    for r in rows("validityTests.csv") {
        let code = &r[0];
        let valid = codes::olc_check(code).is_ok();
        let full = valid && codes::olc_is_full(code);
        let short = valid && !full && code.find('+').is_some_and(|p| p < 8);
        let want = (r[1] == "true", r[2] == "true", r[3] == "true");
        if (valid, short, full) != want {
            bad.push(format!(
                "{code}: got {:?}, want {want:?}",
                (valid, short, full)
            ));
        }
    }
    assert!(
        bad.is_empty(),
        "{} failures:\n{}",
        bad.len(),
        bad.join("\n")
    );
}

#[test]
fn shortening() {
    let mut bad = Vec::new();
    for r in rows("shortCodeTests.csv") {
        let (full, lat, lng, short, kind) = (
            &r[0],
            r[1].parse::<f64>().unwrap(),
            r[2].parse::<f64>().unwrap(),
            &r[3],
            r[4].as_str(),
        );
        if kind != "R" {
            let got = codes::olc_shorten(full, lat, lng);
            if &got != short {
                bad.push(format!(
                    "shorten {full} @ {lat},{lng}: got {got}, want {short}"
                ));
            }
        }
        if kind != "S" {
            let got = codes::olc_recover(short, lat, lng);
            if &got != full {
                bad.push(format!(
                    "recover {short} @ {lat},{lng}: got {got}, want {full}"
                ));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "{} failures:\n{}",
        bad.len(),
        bad.join("\n")
    );
}
