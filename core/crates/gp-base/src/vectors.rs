//! Golden vectors (verification spec): JSON Lines files in `core/vectors/`, one
//! per tool id. Each line is
//!
//! ```json
//! {"id":"v001","input":{…},"expect":{"result.converted.value":115.07794480235425,…},
//!  "tolerance":{"result.converted.value":{"rel":5e-16}},"source":"…","sourceVersion":"…"}
//! ```
//!
//! `expect` keys are dot paths into the result envelope (array indexes are
//! numbers). Every numeric expectation needs a tolerance (`abs` in the field's
//! unit and/or `rel`); strings and booleans must match exactly. A superseded
//! vector keeps its line and gains `"supersededBy"` and `"reason"`; it is not run.

use serde_json::Value;

use crate::tool::Registry;

/// Checks one vector file's provenance. Returns problems naming the vector.
pub fn lint(tool: &str, text: &str) -> Vec<String> {
    let mut errs = Vec::new();
    let mut ids: Vec<String> = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let at = format!("{tool} line {}", n + 1);
        let v: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                errs.push(format!("{at}: not JSON ({e})"));
                continue;
            }
        };
        let id = v["id"].as_str().unwrap_or_default().to_owned();
        if id.is_empty() {
            errs.push(format!("{at}: missing id"));
        } else if ids.contains(&id) {
            errs.push(format!("{at}: duplicate id {id}"));
        }
        ids.push(id.clone());
        for key in ["source", "sourceVersion"] {
            if v[key].as_str().is_none_or(|s| s.trim().is_empty()) {
                errs.push(format!("{at} ({id}): missing {key}"));
            }
        }
        if !v["input"].is_object() {
            errs.push(format!("{at} ({id}): input must be an object"));
        }
        let Some(expect) = v["expect"].as_object().filter(|e| !e.is_empty()) else {
            errs.push(format!("{at} ({id}): expect must be a non-empty object"));
            continue;
        };
        for (path, want) in expect {
            if want.is_number() && v["tolerance"][path].as_object().is_none() {
                errs.push(format!(
                    "{at} ({id}): numeric expectation {path} has no tolerance"
                ));
            }
        }
        if v.get("supersededBy").is_some() && v["reason"].as_str().is_none_or(|r| r.is_empty()) {
            errs.push(format!("{at} ({id}): a superseded vector needs a reason"));
        }
    }
    errs
}

fn lookup<'a>(v: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.').try_fold(v, |cur, seg| match cur {
        Value::Array(a) => seg.parse::<usize>().ok().and_then(|i| a.get(i)),
        Value::Object(o) => o.get(seg),
        _ => None,
    })
}

/// Runs every live vector for `tool` through `reg`. Returns one failure per mismatch.
pub fn run(reg: &Registry, tool: &str, text: &str) -> Vec<String> {
    let mut fails = Vec::new();
    for line in text.lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if v.get("supersededBy").is_some() {
            continue;
        }
        let id = v["id"].as_str().unwrap_or("?");
        let out = reg.invoke(tool, &v["input"].to_string());
        let got: Value = match serde_json::from_str(&out) {
            Ok(g) => g,
            Err(e) => {
                fails.push(format!("{tool} {id}: result is not JSON ({e}): {out}"));
                continue;
            }
        };
        for (path, want) in v["expect"].as_object().into_iter().flatten() {
            let actual = lookup(&got, path);
            let ok = match (want, actual) {
                (Value::Number(w), Some(Value::Number(a))) => {
                    let (w, a) = (
                        w.as_f64().unwrap_or(f64::NAN),
                        a.as_f64().unwrap_or(f64::NAN),
                    );
                    let tol = &v["tolerance"][path];
                    let bound = tol["abs"].as_f64().unwrap_or(0.0)
                        + tol["rel"].as_f64().unwrap_or(0.0) * w.abs();
                    (a - w).abs() <= bound
                }
                (w, Some(a)) => w == a,
                (_, None) => false,
            };
            if !ok {
                fails.push(format!(
                    "{tool} {id}: {path} expected {want}, got {}; full result {out}",
                    actual.map_or("nothing".to_owned(), Value::to_string)
                ));
            }
        }
    }
    fails
}

/// The number of live (not superseded) vectors in a file.
pub fn count(text: &str) -> usize {
    text.lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter(|v| v.get("supersededBy").is_none())
        .count()
}

#[cfg(test)]
mod tests {
    use super::lint;

    #[test]
    fn lint_rejects_missing_provenance_and_tolerance() {
        let bad = concat!(
            r#"{"id":"v1","input":{},"expect":{"result.x":1},"sourceVersion":"1"}"#,
            "\n",
            r#"{"id":"v1","input":{},"expect":{"result.x":1},"source":"s","sourceVersion":"1","tolerance":{"result.x":{"abs":0}}}"#,
            "\n",
            r#"{"id":"v3","input":{},"expect":{"ok":true},"source":"s","sourceVersion":"1","supersededBy":"v4"}"#,
        );
        let errs = lint("t", bad);
        assert!(
            errs.iter().any(|e| e.contains("missing source")),
            "{errs:?}"
        );
        assert!(
            errs.iter().any(|e| e.contains("has no tolerance")),
            "{errs:?}"
        );
        assert!(
            errs.iter().any(|e| e.contains("duplicate id v1")),
            "{errs:?}"
        );
        assert!(
            errs.iter().any(|e| e.contains("needs a reason")),
            "{errs:?}"
        );
    }

    #[test]
    fn lint_accepts_a_good_vector() {
        let good = r#"{"id":"v1","input":{"a":1},"expect":{"ok":true,"result.x":1},"tolerance":{"result.x":{"rel":1e-15}},"source":"NIST","sourceVersion":"2008"}"#;
        assert_eq!(lint("t", good), Vec::<String>::new());
    }
}
