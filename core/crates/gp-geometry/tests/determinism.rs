//! The same input gives the same bytes whatever ran before (AGENTS.md,
//! "Determinism"). A HashMap's iteration order follows its hasher's keys, and
//! each new map in a thread steps those keys on, so any output that followed
//! a map's order depended on how many maps earlier calls had made: the MCP
//! server, which had run other tools first, listed make-valid's problems in
//! another order than a fresh page did.

use gp_geometry::REGISTRY;
use std::collections::hash_map::RandomState;

#[test]
fn results_do_not_depend_on_earlier_calls() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vectors/geometry.validity.make-valid.jsonl"
    );
    let text = std::fs::read_to_string(path).unwrap();
    let inputs: Vec<String> = text
        .lines()
        .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap()["input"].to_string())
        .collect();
    assert!(inputs.len() > 20);
    for input in &inputs {
        let first = REGISTRY.invoke("geometry.validity.make-valid", input);
        for k in 0..7 {
            // Each new RandomState steps the thread's keys on.
            let _ = (0..=k).map(|_| RandomState::new()).count();
            assert_eq!(
                REGISTRY.invoke("geometry.validity.make-valid", input),
                first,
                "{input}"
            );
        }
    }
}
