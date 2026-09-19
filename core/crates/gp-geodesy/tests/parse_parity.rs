//! The coordinate parser on 1,500 strings written independently by
//! tools/vectors/gen_parse_diff.py in ten notations (signed decimal,
//! hemisphere letters, DMS with symbols, primes, spaces, or hyphens,
//! degrees and decimal minutes, packed aviation, labeled pairs, and signed
//! DMS), each against the exact value it encodes.

use gp_geodesy::REGISTRY;
use serde_json::{Value, json};

#[test]
fn parser_reads_every_notation() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/parse_diff.jsonl")).unwrap();
    let mut bad = Vec::new();
    let mut n = 0;
    for line in text.lines() {
        let want: Value = serde_json::from_str(line).unwrap();
        let t = want["text"].as_str().unwrap();
        let r: Value = serde_json::from_str(&REGISTRY.invoke("geodesy.parse.coordinates", &json!({"text": t}).to_string())).unwrap();
        let (la, lo) = (r["result"]["lat"]["value"].as_f64(), r["result"]["lon"]["value"].as_f64());
        let ok = matches!((la, lo), (Some(a), Some(b)) if (a - want["lat"].as_f64().unwrap()).abs() < 1e-9 && (b - want["lon"].as_f64().unwrap()).abs() < 1e-9);
        if !ok {
            bad.push(format!("{t} -> {:?} {:?} ({})", la, lo, r["error"]["message"]));
        }
        n += 1;
    }
    assert_eq!(n, 1500);
    assert!(bad.is_empty(), "{} misreads:\n{}", bad.len(), bad[..bad.len().min(30)].join("\n"));
}
