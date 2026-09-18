//! The codes registry (`data/codes.json`) is the single source of message
//! templates and severities. These tests keep the core in sync with it.

use gp_base::ErrorCode;
use serde_json::Value;
use std::path::Path;

fn registry() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data/codes.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("read data/codes.json"))
        .expect("valid JSON")
}

#[test]
fn error_enum_matches_registry_exactly() {
    let reg = registry();
    let errors = reg["errors"].as_object().expect("errors object");
    let rust: Vec<&str> = ErrorCode::ALL.iter().map(|c| c.as_str()).collect();
    let json: Vec<&str> = errors.keys().map(String::as_str).collect();
    let mut a = rust.clone();
    let mut b = json.clone();
    a.sort();
    b.sort();
    assert_eq!(a, b);
}

#[test]
fn warnings_have_severity_and_message() {
    let reg = registry();
    for (code, w) in reg["warnings"].as_object().expect("warnings object") {
        let sev = w["severity"].as_str().unwrap_or_default();
        assert!(
            ["caution", "accuracy", "info"].contains(&sev),
            "{code}: bad severity"
        );
        assert!(
            !w["message"].as_str().unwrap_or_default().is_empty(),
            "{code}: empty message"
        );
    }
}

/// Every warning code the core source emits must be registered.
#[test]
fn emitted_warning_codes_are_registered() {
    let reg = registry();
    let warnings = reg["warnings"].as_object().expect("warnings object");
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut found = 0;
    for krate in std::fs::read_dir(&crates).expect("crates dir") {
        let src = krate.expect("entry").path().join("src");
        for file in walk(&src) {
            let text = std::fs::read_to_string(&file).expect("read source");
            for code in warning_literals(&text) {
                found += 1;
                assert!(
                    warnings.contains_key(&code),
                    "{} emits unregistered warning {code}",
                    file.display()
                );
            }
        }
    }
    assert!(found > 0, "scanner found no warning literals");
}

fn walk(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(walk(&p));
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    out
}

/// Finds `Warning::new("CODE"` literals.
fn warning_literals(text: &str) -> Vec<String> {
    text.match_indices("Warning::new(")
        .filter_map(|(i, m)| {
            let rest = text[i + m.len()..].trim_start();
            let rest = rest.strip_prefix('"')?;
            Some(rest[..rest.find('"')?].to_owned())
        })
        .collect()
}
