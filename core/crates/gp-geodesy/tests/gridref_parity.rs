//! GARS, GEOREF, and Maidenhead against separately written Python libraries
//! (pygeodesy's gars and wgrs, and the maidenhead package), through the
//! public tools. Fixture from tools/vectors/gen_gridref_diff.py.

use gp_geodesy::REGISTRY;
use serde_json::{Value, json};

fn call(id: &str, input: &Value) -> Value {
    serde_json::from_str(&REGISTRY.invoke(id, &input.to_string())).expect("envelope is JSON")
}

fn v(r: &Value, k: &str) -> f64 {
    r["result"][k]["value"]
        .as_f64()
        .unwrap_or_else(|| panic!("{k} missing in {r}"))
}

const GARS: [&str; 3] = ["30min", "15min", "5min"];
const GEOREF: [&str; 7] = [
    "15deg",
    "1deg",
    "1min",
    "0.1min",
    "0.01min",
    "0.001min",
    "0.0001min",
];
const MAIDENHEAD: [&str; 4] = ["4", "6", "8", "10"];

/// Each system: the forward tool and its code field, the inverse tool and
/// its input field, the precisions, and the fixture key.
type System = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static [&'static str],
    &'static str,
);

const SYSTEMS: [System; 3] = [
    (
        "geodesy.grid-ref.gars-forward",
        "gars",
        "geodesy.grid-ref.gars-inverse",
        "gars",
        &GARS,
        "gars",
    ),
    (
        "geodesy.grid-ref.georef-forward",
        "georef",
        "geodesy.grid-ref.georef-inverse",
        "georef",
        &GEOREF,
        "georef",
    ),
    (
        "geodesy.grid-ref.maidenhead-forward",
        "locator",
        "geodesy.grid-ref.maidenhead-inverse",
        "locator",
        &MAIDENHEAD,
        "maidenhead",
    ),
];

#[test]
fn grid_references_match_python_libraries() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/gridref_diff.jsonl"
    ))
    .unwrap();
    let mut bad = Vec::new();
    let mut n = 0;
    for line in text.lines().skip(1) {
        let r: Value = serde_json::from_str(line).unwrap();
        let (lat, lon) = (r["lat"].as_f64().unwrap(), r["lon"].as_f64().unwrap());
        for (fwd, out, inv, field, precs, key) in SYSTEMS {
            for (p, want) in precs.iter().zip(r[key].as_array().unwrap()) {
                let [code, s, w, clat, clon] = [&want[0], &want[1], &want[2], &want[3], &want[4]];
                let code = code.as_str().unwrap();
                n += 1;
                let f = call(fwd, &json!({"lat": lat, "lon": lon, "precision": p}));
                let got = f["result"][out].as_str().unwrap_or("");
                if !got.eq_ignore_ascii_case(code) {
                    bad.push(format!("{key} {p} @ {lat},{lon}: got {got}, want {code}"));
                    continue;
                }
                let d = call(inv, &json!({ field: code }));
                let close =
                    |k: &str, want: &Value| (v(&d, k) - want.as_f64().unwrap()).abs() < 1e-9;
                if !(close("south", s)
                    && close("west", w)
                    && close("lat", clat)
                    && close("lon", clon))
                {
                    bad.push(format!("{key} {code}: got {}", d["result"]));
                }
            }
        }
    }
    assert!(n > 7_000 / 2, "{n}");
    assert!(
        bad.is_empty(),
        "{} of {n} mismatches:\n{}",
        bad.len(),
        bad[..bad.len().min(12)].join("\n")
    );
}

/// USNG precisions by digits per axis (0 to 5).
const USNG: [&str; 6] = ["100km", "10km", "1km", "100m", "10m", "1m"];

/// An MGRS reference written as USNG: 18SUJ2348306479 → 18S UJ 23483 06479.
fn spaced(m: &str) -> String {
    let k = m.find(|c: char| c.is_ascii_alphabetic()).unwrap();
    let (gzd, rest) = m.split_at(k + 1);
    if gzd.len() == 1 {
        return m.to_owned(); // UPS: USNG does not cover the polar caps.
    }
    let (sq, digits) = rest.split_at(2);
    let h = digits.len() / 2;
    [gzd, sq, &digits[..h], &digits[h..]]
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn usng_matches_geotrans_mgrs() {
    // USNG is MGRS on NAD 83 written with spaces, so the MGRS fixture (NGA
    // GEOTRANS, digits from PROJ) checks it outside the polar caps.
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/mgrs_diff.csv"
    ))
    .unwrap();
    let (mut n, mut worst) = (0, 0.0f64);
    let mut bad = Vec::new();
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let c: Vec<&str> = line.split(',').collect();
        let f = |i: usize| c[i].parse::<f64>().unwrap();
        if !c[3].starts_with(|ch: char| ch.is_ascii_digit()) {
            continue;
        }
        let want = spaced(c[3]);
        let d: usize = c[2].parse().unwrap();
        let r = call(
            "geodesy.grid-ref.usng-forward",
            &json!({"lat": f(0), "lon": f(1), "precision": USNG[d]}),
        );
        if r["result"]["usng"] != want.as_str() {
            bad.push(format!("{line} -> {}", r["result"]["usng"]));
            continue;
        }
        let b = call("geodesy.grid-ref.usng-inverse", &json!({"usng": want}));
        let (la, lo) = (v(&b, "corner_lat"), v(&b, "corner_lon"));
        let dlon = (lo - f(5) + 540.0).rem_euclid(360.0) - 180.0;
        worst = worst.max((la - f(4)).hypot(dlon * f(4).to_radians().cos()) * 111_320.0);
        n += 1;
    }
    assert!(
        bad.is_empty(),
        "{} mismatches:\n{}",
        bad.len(),
        bad[..bad.len().min(10)].join("\n")
    );
    assert!(n > 1_500, "{n}");
    assert!(worst < 0.02, "worst corner gap {worst} m");
}
