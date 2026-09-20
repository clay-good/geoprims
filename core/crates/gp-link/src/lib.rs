//! Permalink fragments (`contracts/routes-and-urls`, "Permalink fragment
//! grammar"):
//!
//! ```text
//! fragment     = example-frag / state-frag
//! example-frag = "example" [ flags ]
//! state-frag   = "v1:" payload [ flags ]
//! payload      = base64url(deflate-raw(canonical-json)), no padding
//! flags        = *( ";" flag ), flag = "report" / "nofx"
//! ```
//!
//! The canonical JSON has only the keys `i` (inputs), `u` (unit overrides),
//! `v` (canvas view), `e` (pinned epoch), and `c` (the tool a chained value
//! came from), sorted at every level, with numbers in the core's ECMAScript
//! format. Built as its own `link` module.

pub mod deflate;

use gp_base::envelope;
use gp_base::error::{ErrorCode, ToolError};
use gp_base::json::Json;
use gp_base::manifest::from_value;
use gp_base::tool::Registry;
use serde_json::Value;

pub static REGISTRY: Registry = Registry {
    module: "link",
    tools: &[],
};

gp_base::export_module!("link", REGISTRY);

/// The largest decoded state accepted, so a crafted link cannot exhaust memory.
pub const MAX_STATE_BYTES: usize = 65_536;
/// The longest fragment accepted.
pub const MAX_FRAGMENT_CHARS: usize = 16_384;

const FLAGS: &[&str] = &["report", "nofx"];
const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

pub fn base64url(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = u32::from(b[0]) << 16 | u32::from(b[1]) << 8 | u32::from(b[2]);
        for k in 0..=chunk.len() {
            out.push(B64[(n >> (18 - 6 * k) & 63) as usize] as char);
        }
    }
    out
}

pub fn unbase64url(s: &str) -> Option<Vec<u8>> {
    if s.len() % 4 == 1 {
        return None;
    }
    let vals: Vec<u32> = s
        .bytes()
        .map(|c| B64.iter().position(|&b| b == c).map(|p| p as u32))
        .collect::<Option<_>>()?;
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    for chunk in vals.chunks(4) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |acc, (k, v)| acc | v << (18 - 6 * k));
        let bytes = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
        out.extend_from_slice(&bytes[..chunk.len() - 1]);
    }
    // Reject non-canonical trailing bits so each state has exactly one encoding.
    if base64url(&out) != s {
        return None;
    }
    Some(out)
}

/// Sorts object keys at every level.
fn canonical(v: &Value) -> Json {
    match v {
        Value::Object(m) => {
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort();
            Json::Obj(
                keys.into_iter()
                    .map(|k| (k.clone(), canonical(&m[k])))
                    .collect(),
            )
        }
        Value::Array(a) => Json::Arr(a.iter().map(canonical).collect()),
        other => from_value(other),
    }
}

fn check_state(state: &Value) -> Result<(), ToolError> {
    let Value::Object(m) = state else {
        return Err(ToolError::invalid(
            "/state",
            "The state must be an object with keys i, u, v, e, c.",
        ));
    };
    for (k, v) in m {
        let at = format!("/state/{k}");
        match k.as_str() {
            "i" | "u" => {
                let Value::Object(fields) = v else {
                    return Err(ToolError::invalid(&at, format!("{k} must be an object.")));
                };
                // A value is a string or number, or a list of flat rows
                // (list inputs such as traverse courses).
                let scalar = |x: &Value| x.is_string() || x.is_number();
                let row = |x: &Value| x.as_object().is_some_and(|r| r.values().all(scalar));
                for (f, x) in fields {
                    let ok = scalar(x) || x.as_array().is_some_and(|rows| rows.iter().all(row));
                    if !ok {
                        return Err(ToolError::invalid(
                            &format!("{at}/{f}"),
                            "Values must be strings, numbers, or lists of rows of them.",
                        ));
                    }
                }
            }
            // The chain breadcrumb: a tool id, so a link cannot carry a
            // sentence of someone else's text into the page.
            "c" => {
                let ok = v.as_str().is_some_and(|id| {
                    id.len() <= 64
                        && id.split('.').count() == 3
                        && id.split('.').all(|part| {
                            !part.is_empty()
                                && part.bytes().all(|b| {
                                    b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'
                                })
                        })
                });
                if !ok {
                    return Err(ToolError::invalid(&at, "c must be a tool id."));
                }
            }
            "v" if !v.is_object() => return Err(ToolError::invalid(&at, "v must be an object.")),
            "e" if !(v.is_string() || v.is_number()) => {
                return Err(ToolError::invalid(
                    &at,
                    "e must be a date string or a decimal year.",
                ));
            }
            "v" | "e" => {}
            _ => {
                return Err(ToolError::invalid(
                    &at,
                    format!("{k} is not a fragment key. Keys: i, u, v, e, c."),
                ));
            }
        }
    }
    Ok(())
}

fn ok(result: Json) -> String {
    Json::obj([("ok", Json::Bool(true)), ("result", result)])
        .to_string()
        .unwrap_or_else(|e| envelope::failure(&e))
}

/// Encodes `{state, flags?}` to `{fragment}`.
pub fn encode(request: &str) -> String {
    let run = || -> Result<String, ToolError> {
        let req: Value = serde_json::from_str(request).map_err(|e| {
            ToolError::new(
                ErrorCode::InvalidInput,
                format!("The request is not JSON: {e}."),
            )
        })?;
        let state = &req["state"];
        check_state(state)?;
        let mut frag = format!(
            "v1:{}",
            base64url(&deflate::compress(canonical(state).to_string()?.as_bytes()))
        );
        for f in req["flags"].as_array().into_iter().flatten() {
            let f = f
                .as_str()
                .filter(|f| FLAGS.contains(f))
                .ok_or_else(|| ToolError::invalid("/flags", "Flags are report and nofx."))?;
            frag.push(';');
            frag.push_str(f);
        }
        if frag.len() > MAX_FRAGMENT_CHARS {
            return Err(ToolError::new(
                ErrorCode::LimitExceeded,
                "The state is too large for a link.",
            ));
        }
        Ok(frag)
    };
    match run() {
        Ok(f) => ok(Json::obj([("fragment", Json::str(f))])),
        Err(e) => envelope::failure(&e),
    }
}

/// Decodes a fragment (with or without the leading `#`) to
/// `{kind: "example"|"state", state?, flags}`.
pub fn decode(fragment: &str) -> String {
    let run = || -> Result<Json, ToolError> {
        let frag = fragment.strip_prefix('#').unwrap_or(fragment);
        if frag.len() > MAX_FRAGMENT_CHARS {
            return Err(ToolError::new(
                ErrorCode::LimitExceeded,
                "The link is too long.",
            ));
        }
        let mut parts = frag.split(';');
        let head = parts.next().unwrap_or_default();
        let mut flags = Vec::new();
        for f in parts {
            if !FLAGS.contains(&f) {
                return Err(ToolError::invalid(
                    "/fragment",
                    format!("Unknown link flag {f}."),
                ));
            }
            flags.push(Json::str(f));
        }
        let flags = Json::Arr(flags);
        if head == "example" {
            return Ok(Json::obj([
                ("kind", Json::str("example")),
                ("flags", flags),
            ]));
        }
        let Some(payload) = head.strip_prefix("v1:") else {
            if head.len() > 1
                && head.starts_with('v')
                && head[1..]
                    .split(':')
                    .next()
                    .is_some_and(|n| n.bytes().all(|b| b.is_ascii_digit()))
            {
                return Err(ToolError::new(
                    ErrorCode::Unsupported,
                    "This link was made by a newer version of geoprims",
                )
                .hint("The tool opens with example values instead."));
            }
            return Err(ToolError::invalid(
                "/fragment",
                "This is not a geoprims link.",
            ));
        };
        let bad = || ToolError::invalid("/fragment", "This link is damaged.");
        let bytes = unbase64url(payload).ok_or_else(bad)?;
        let json = deflate::decompress(&bytes, MAX_STATE_BYTES).map_err(|e| match e {
            deflate::InflateError::TooLarge => {
                ToolError::new(ErrorCode::LimitExceeded, "The link's state is too large.")
            }
            deflate::InflateError::Corrupt => bad(),
        })?;
        let state: Value = serde_json::from_slice(&json).map_err(|_| bad())?;
        check_state(&state)?;
        Ok(Json::obj([
            ("kind", Json::str("state")),
            ("state", canonical(&state)),
            ("flags", flags),
        ]))
    };
    match run() {
        Ok(r) => ok(r),
        Err(e) => envelope::failure(&e),
    }
}

#[cfg(target_arch = "wasm32")]
mod exports {
    /// # Safety
    /// The range must be a readable UTF-8 buffer from `gp_alloc`.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn gp_link_encode(ptr: *const u8, len: usize) -> *const u8 {
        let out = match unsafe { gp_base::abi::read_str(ptr, len) } {
            Ok(s) => super::encode(s),
            Err(_) => gp_base::abi::bad_utf8(),
        };
        gp_base::abi::set_out(&out)
    }

    /// # Safety
    /// The range must be a readable UTF-8 buffer from `gp_alloc`.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn gp_link_decode(ptr: *const u8, len: usize) -> *const u8 {
        let out = match unsafe { gp_base::abi::read_str(ptr, len) } {
            Ok(s) => super::decode(s),
            Err(_) => gp_base::abi::bad_utf8(),
        };
        gp_base::abi::set_out(&out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Value {
        serde_json::from_str(s).unwrap()
    }

    #[test]
    fn base64url_round_trip_and_canonical_only() {
        for n in 0..20 {
            let d: Vec<u8> = (0..n).map(|i| (i * 37 + 11) as u8).collect();
            assert_eq!(unbase64url(&base64url(&d)).unwrap(), d);
        }
        assert_eq!(base64url(b"\xfb\xff"), "-_8");
        assert!(unbase64url("-_9").is_none(), "non-zero trailing bits");
        assert!(unbase64url("a").is_none());
        assert!(unbase64url("ab+c").is_none());
    }

    /// A chained link carries where the value came from, and nothing else
    /// (web/app-shell, "Tool chaining").
    #[test]
    fn chain_breadcrumb_is_a_tool_id() {
        let sent = v(&encode(
            r#"{"state":{"c":"navigation.geodesic.inverse","i":{"course":"64.5 deg"}}}"#,
        ));
        let d = v(&decode(sent["result"]["fragment"].as_str().unwrap()));
        assert_eq!(d["result"]["state"]["c"], "navigation.geodesic.inverse");
        assert_eq!(d["result"]["state"]["i"]["course"], "64.5 deg");
        // Anything that is not a tool id is refused, so a link cannot carry
        // someone else's words onto the page.
        for bad in [
            r#"{"state":{"c":"Sent from a friend"}}"#,
            r#"{"state":{"c":"navigation.geodesic"}}"#,
            r#"{"state":{"c":"Navigation.Geodesic.Inverse"}}"#,
            r#"{"state":{"c":42}}"#,
        ] {
            assert_eq!(v(&encode(bad))["error"]["field"], "/state/c", "{bad}");
        }
    }

    #[test]
    fn encode_decode_round_trip_with_sorted_keys() {
        let a = v(&encode(
            r#"{"state":{"i":{"value":"100 kt","to":"mph"}},"flags":["report"]}"#,
        ));
        let b = v(&encode(
            r#"{"state":{"i":{"to":"mph","value":"100 kt"}},"flags":["report"]}"#,
        ));
        assert_eq!(a, b, "key order must not matter");
        let frag = a["result"]["fragment"].as_str().unwrap();
        assert!(
            frag.starts_with("v1:") && frag.ends_with(";report"),
            "{frag}"
        );
        let d = v(&decode(&format!("#{frag}")));
        assert_eq!(d["result"]["kind"], "state");
        assert_eq!(d["result"]["state"]["i"]["value"], "100 kt");
        assert_eq!(d["result"]["flags"][0], "report");
    }

    #[test]
    fn example_and_errors() {
        assert_eq!(v(&decode("example;nofx"))["result"]["kind"], "example");
        let newer = v(&decode("v9:abc"));
        assert_eq!(newer["error"]["code"], "UNSUPPORTED");
        assert_eq!(
            newer["error"]["message"],
            "This link was made by a newer version of geoprims"
        );
        assert_eq!(
            v(&decode("v1:%%%"))["error"]["message"],
            "This link is damaged."
        );
        assert_eq!(v(&decode("v1:AAAA;boom"))["error"]["code"], "INVALID_INPUT");
        assert_eq!(
            v(&encode(r#"{"state":{"x":{}}}"#))["error"]["field"],
            "/state/x"
        );
        assert_eq!(
            v(&encode(r#"{"state":{"i":{"a":[1]}}}"#))["error"]["field"],
            "/state/i/a"
        );
        assert_eq!(
            v(&encode(r#"{"state":{"i":{"a":[{"b":{"c":1}}]}}}"#))["error"]["field"],
            "/state/i/a"
        );
    }

    #[test]
    fn list_inputs_round_trip() {
        let e = v(&encode(
            r#"{"state":{"i":{"courses":[{"distance":300,"direction":"N 45°30'15\" E"},{"direction":"90","distance":"400.02 ft"}]}}}"#,
        ));
        let frag = e["result"]["fragment"].as_str().unwrap();
        let d = v(&decode(frag));
        let rows = &d["result"]["state"]["i"]["courses"];
        assert_eq!(rows[0]["direction"], "N 45°30'15\" E");
        assert_eq!(rows[1]["distance"], "400.02 ft");
    }

    #[test]
    fn decompression_bomb_refused() {
        let big = format!(r#"{{"i":{{"a":"{}"}}}}"#, "x".repeat(MAX_STATE_BYTES));
        let frag = format!("v1:{}", base64url(&deflate::compress(big.as_bytes())));
        assert_eq!(v(&decode(&frag))["error"]["code"], "LIMIT_EXCEEDED");
    }
}
