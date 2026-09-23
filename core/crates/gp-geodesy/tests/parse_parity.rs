//! The coordinate parser on 1,500 strings that must parse and 504 that must
//! not, written independently by
//! tools/vectors/gen_parse_diff.py in ten notations (signed decimal,
//! hemisphere letters, DMS with symbols, primes, spaces, or hyphens,
//! degrees and decimal minutes, packed aviation, labeled pairs, and signed
//! DMS), each against the exact value it encodes.

use gp_geodesy::REGISTRY;
use serde_json::{Value, json};

#[test]
fn parser_reads_every_notation() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/parse_diff.jsonl"
    ))
    .unwrap();
    let mut bad = Vec::new();
    let mut n = 0;
    for line in text.lines() {
        let want: Value = serde_json::from_str(line).unwrap();
        let t = want["text"].as_str().unwrap();
        let r: Value = serde_json::from_str(
            &REGISTRY.invoke("geodesy.parse.coordinates", &json!({"text": t}).to_string()),
        )
        .unwrap();
        let (la, lo) = (
            r["result"]["lat"]["value"].as_f64(),
            r["result"]["lon"]["value"].as_f64(),
        );
        let ok = matches!((la, lo), (Some(a), Some(b)) if (a - want["lat"].as_f64().unwrap()).abs() < 1e-9 && (b - want["lon"].as_f64().unwrap()).abs() < 1e-9);
        if !ok {
            bad.push(format!(
                "{t} -> {:?} {:?} ({})",
                la, lo, r["error"]["message"]
            ));
        }
        n += 1;
    }
    assert_eq!(n, 1500);
    assert!(
        bad.is_empty(),
        "{} misreads:\n{}",
        bad.len(),
        bad[..bad.len().min(30)].join("\n")
    );
}

/// The other half of the corpus: 504 strings that must not parse, each wrong
/// in exactly one named way and written without hemisphere letters wherever
/// the rule does not need them, since letters take another branch through the
/// parser and a rule can pass there while doing nothing on a bare pair.
#[test]
fn parser_refuses_every_malformed_notation() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/parse_reject.jsonl"
    ))
    .unwrap();
    let mut wrong = Vec::new();
    let mut n = 0;
    for line in text.lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let (t, why) = (row["text"].as_str().unwrap(), row["why"].as_str().unwrap());
        let r: Value = serde_json::from_str(
            &REGISTRY.invoke("geodesy.parse.coordinates", &json!({"text": t}).to_string()),
        )
        .unwrap();
        if r["ok"] == true {
            wrong.push(format!(
                "{t} was read as {} {} instead of being refused for {why}",
                r["result"]["lat"]["value"], r["result"]["lon"]["value"]
            ));
        } else if !r["error"]["message"]
            .as_str()
            .is_some_and(|m| m.contains(why))
        {
            wrong.push(format!(
                "{t} was refused for {} and not {why}",
                r["error"]["message"]
            ));
        }
        n += 1;
    }
    assert_eq!(n, 504);
    assert!(
        wrong.is_empty(),
        "{} of {n} wrong:\n{}",
        wrong.len(),
        wrong
            .iter()
            .take(12)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
