//! Output JSON with a fixed key order and ECMAScript number formatting.
//! Serialization is the only place numbers become text, so NaN is rejected
//! and negative zero is normalized here, once, for every tool.

use crate::error::{ErrorCode, ToolError};
use crate::num::format_f64;

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    /// Keys are written in insertion order (the output schema's order).
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn obj<K: Into<String>>(pairs: impl IntoIterator<Item = (K, Json)>) -> Json {
        Json::Obj(pairs.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }

    pub fn str(s: impl Into<String>) -> Json {
        Json::Str(s.into())
    }

    /// Serializes to compact JSON. Fails with `INTERNAL` naming the JSON Pointer of
    /// any non-finite number, so a NaN can never reach a caller.
    pub fn to_string(&self) -> Result<String, ToolError> {
        let mut out = String::new();
        self.write(&mut out, &mut String::new())?;
        Ok(out)
    }

    fn write(&self, out: &mut String, path: &mut String) -> Result<(), ToolError> {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Num(x) => match format_f64(*x) {
                Some(s) => out.push_str(&s),
                None => {
                    let at = if path.is_empty() { "/" } else { path.as_str() };
                    return Err(ToolError::new(
                        ErrorCode::Internal,
                        format!("The calculation produced a non-finite number at {at}."),
                    ));
                }
            },
            Json::Str(s) => write_str(out, s),
            Json::Arr(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    let len = path.len();
                    path.push('/');
                    path.push_str(&i.to_string());
                    item.write(out, path)?;
                    path.truncate(len);
                }
                out.push(']');
            }
            Json::Obj(pairs) => {
                out.push('{');
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_str(out, k);
                    out.push(':');
                    let len = path.len();
                    path.push('/');
                    path.push_str(&k.replace('~', "~0").replace('/', "~1"));
                    v.write(out, path)?;
                    path.truncate(len);
                }
                out.push('}');
            }
        }
        Ok(())
    }
}

/// Writes a JSON string with the same escaping as `JSON.stringify`.
fn write_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::Json;

    #[test]
    fn keeps_key_order_and_normalizes_zero() {
        let j = Json::obj([
            ("z", Json::Num(-0.0)),
            ("a", Json::Num(1e21)),
            ("m", Json::str("a\"b\n\u{1}")),
        ]);
        assert_eq!(
            j.to_string().unwrap(),
            concat!(r#"{"z":0,"a":1e+21,"m":"a\"b\n"#, "\\u0001", r#""}"#)
        );
    }

    #[test]
    fn nan_is_an_internal_error_with_pointer() {
        let j = Json::obj([("out", Json::Arr(vec![Json::Num(1.0), Json::Num(f64::NAN)]))]);
        let e = j.to_string().unwrap_err();
        assert_eq!(e.code.as_str(), "INTERNAL");
        assert!(e.message.contains("/out/1"), "{}", e.message);
    }
}
