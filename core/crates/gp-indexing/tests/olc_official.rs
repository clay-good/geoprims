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

/// The same official cases through the public tools (what the web page and MCP call).
#[test]
fn official_cases_through_the_tools() {
    use gp_indexing::REGISTRY;
    use serde_json::{Value, json};
    let call = |id: &str, v: Value| -> Value {
        serde_json::from_str(&REGISTRY.invoke(id, &v.to_string())).unwrap()
    };
    let mut bad = Vec::new();
    let mut n = 0;
    for r in rows("encoding.csv") {
        let len: u32 = r[4].parse().unwrap();
        let lat: f64 = r[0].parse().unwrap();
        let out = call(
            "indexing.plus-code.encode",
            json!({"lat": lat, "lon": r[1].parse::<f64>().unwrap(), "length": len}),
        );
        // The library clips an impossible latitude or length; the tool refuses it and says why.
        if lat.abs() > 90.0 || len > 15 {
            if out["ok"] != false {
                bad.push(format!(
                    "encode {} {} {len}: should be refused, got {}",
                    r[0], r[1], out["result"]["code"]
                ));
            }
        } else if out["result"]["code"] != r[5].as_str() {
            bad.push(format!(
                "encode {} {} {len}: {}",
                r[0], r[1], out["result"]["code"]
            ));
        }
        n += 1;
    }
    for r in rows("decoding.csv") {
        let out = call("indexing.plus-code.decode", json!({"code": r[0]}));
        let got = ["south", "west", "north", "east"]
            .map(|k| out["result"][k]["value"].as_f64().unwrap_or(f64::NAN));
        let want: Vec<f64> = r[2..6].iter().map(|v| v.parse().unwrap()).collect();
        if got.iter().zip(&want).any(|(a, b)| (a - b).abs() > 1e-10) {
            bad.push(format!("decode {}: {got:?} vs {want:?}", r[0]));
        }
        n += 1;
    }
    for r in rows("shortCodeTests.csv") {
        let (full, lat, lon, short, kind) = (
            &r[0],
            r[1].parse::<f64>().unwrap(),
            r[2].parse::<f64>().unwrap(),
            &r[3],
            r[4].as_str(),
        );
        if kind == "S" || kind == "B" {
            let out = call(
                "indexing.plus-code.shorten",
                json!({"code": full, "ref_lat": lat, "ref_lon": lon}),
            );
            if out["result"]["short_code"] != short.as_str() {
                bad.push(format!(
                    "shorten {full} near {lat},{lon}: {}",
                    out["result"]["short_code"]
                ));
            }
        }
        if kind == "R" || kind == "B" {
            let out = call(
                "indexing.plus-code.decode",
                json!({"code": short, "ref_lat": lat, "ref_lon": lon}),
            );
            if out["result"]["full_code"] != full.as_str() {
                bad.push(format!(
                    "recover {short} near {lat},{lon}: {}",
                    out["result"]["full_code"]
                ));
            }
        }
        n += 1;
    }
    assert!(n >= 700, "{n} cases");
    assert!(
        bad.is_empty(),
        "{} failures:\n{}",
        bad.len(),
        bad[..bad.len().min(30)].join("\n")
    );
}
