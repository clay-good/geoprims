//! Dated regulatory reference data (`data/regulations.json`), shared by every
//! domain that quotes a rule: each value carries its citation, the date it was
//! last checked against the authority, its status, and a link.

use serde_json::Value;

static REGULATIONS: &str = include_str!("../../../../data/regulations.json");

/// One reference entry: value, citation, review date, status, and link.
#[derive(Clone, Debug)]
pub struct Rule {
    pub value: f64,
    pub citation: String,
    pub reviewed: String,
    pub status: String,
    pub url: String,
}

/// Looks up a regulation entry by id. Panics on a missing id (a build error).
pub fn rule(id: &str) -> Rule {
    let v: Value = serde_json::from_str(REGULATIONS).expect("regulations.json parses");
    let e = v["entries"]
        .as_array()
        .and_then(|a| a.iter().find(|e| e["id"] == id))
        .unwrap_or_else(|| panic!("regulations.json has no entry {id}"));
    let s = |k: &str| e[k].as_str().unwrap_or_default().to_owned();
    Rule {
        value: e["value"].as_f64().unwrap_or(f64::NAN),
        citation: s("citation"),
        reviewed: s("reviewed"),
        status: s("status"),
        url: s("url"),
    }
}
