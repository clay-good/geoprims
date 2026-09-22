//! The manifest lint over the raster tools, and their golden vectors. Every
//! crate runs this for its own registry; a crate that skips it ships tools the
//! lint has never seen, which is how a sentence template that cannot render
//! reached a build.

use std::path::Path;

use gp_base::{manifest, vectors};
use gp_raster::{REGISTRY, TOOLS};
use serde_json::Value;

fn repo(path: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(path);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

#[test]
fn catalog_lint_and_registry() {
    let tax: Value = serde_json::from_str(&repo("data/taxonomy.json")).unwrap();
    let owned: Vec<(String, Vec<String>)> = tax["domains"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(d, v)| {
            (
                d.clone(),
                v["groups"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|g| g.as_str().unwrap().to_owned())
                    .collect(),
            )
        })
        .collect();
    let taxonomy: Vec<(&str, Vec<&str>)> = owned
        .iter()
        .map(|(d, g)| (d.as_str(), g.iter().map(String::as_str).collect()))
        .collect();
    let errs = manifest::lint(TOOLS, &taxonomy, &[]);
    assert!(errs.is_empty(), "{}", errs.join("\n"));
    let codes: Value = serde_json::from_str(&repo("data/codes.json")).unwrap();
    for t in TOOLS {
        for w in t.warnings {
            assert!(
                codes["warnings"].get(*w).is_some(),
                "{} declares unregistered {w}",
                t.id
            );
        }
    }
}

#[test]
fn golden_vectors() {
    for t in TOOLS {
        let path = format!("core/vectors/{}.jsonl", t.id);
        let text = repo(&path);
        let failures = vectors::run(&REGISTRY, t.id, &text);
        assert!(failures.is_empty(), "{path}:\n{}", failures.join("\n"));
    }
}
