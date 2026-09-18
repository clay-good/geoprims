//! The shared fragment vector file pins each encoding byte for byte, so the
//! website and the MCP server cannot drift (contracts/routes-and-urls).

use std::path::Path;

use serde_json::{Value, json};

fn repo(p: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(p)
}

#[test]
fn encodings_match_the_vector_file() {
    let states: Value =
        serde_json::from_str(&std::fs::read_to_string(repo("data/fragment-states.json")).unwrap())
            .unwrap();
    let mut vectors = Vec::new();
    for case in states["cases"].as_array().unwrap() {
        let out: Value = serde_json::from_str(&gp_link::encode(&case.to_string())).unwrap();
        let fragment = out["result"]["fragment"]
            .as_str()
            .expect("encodes")
            .to_owned();
        let back: Value = serde_json::from_str(&gp_link::decode(&fragment)).unwrap();
        assert_eq!(
            back["result"]["state"], case["state"],
            "round trip of {fragment}"
        );
        vectors.push(json!({"state": case["state"], "flags": case["flags"], "fragment": fragment}));
    }
    let text = serde_json::to_string_pretty(&json!({"vectors": vectors})).unwrap() + "\n";
    let file = repo("data/fragment-vectors.json");
    if std::env::var_os("UPDATE_FRAGMENTS").is_some() {
        std::fs::write(&file, &text).unwrap();
    }
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        text,
        "fragment encodings changed"
    );
}
