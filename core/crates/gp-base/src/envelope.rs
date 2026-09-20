//! The result envelope every tool returns, with its `meta` provenance block
//! (tool-contract, "Results carry provenance").
//!
//! Success: `{"ok":true,"result":{…},"meta":{…}}`
//! Failure: `{"ok":false,"error":{"code","message","field"?,"hint"?}}`

use crate::error::{ToolError, Warning};
use crate::json::Json;

/// The version of the whole core catalog (`coreVersion`).
pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, PartialEq)]
pub struct AssetRef {
    pub id: String,
    pub version: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Meta {
    pub tool: String,
    pub tool_version: String,
    pub assets: Vec<AssetRef>,
    pub model: String,
    pub accuracy: String,
    pub warnings: Vec<Warning>,
    /// Operating context agents should relay (`meta.context`), when any.
    pub context: Vec<(&'static str, Json)>,
    /// A standing notice for operational tools (`meta.notice`), when any.
    pub notice: Option<&'static str>,
    /// The simplified-method banner (`meta.limitation`), when the tool has one.
    pub limitation: Option<crate::tool::Limitation>,
}

impl Meta {
    pub fn to_json(&self) -> Json {
        // Warnings are ordered by code so identical inputs give identical bytes
        // regardless of the order a tool discovered them.
        let mut warnings = self.warnings.clone();
        warnings.sort_by(|a, b| (a.code, &a.field).cmp(&(b.code, &b.field)));
        let base = Json::obj([
            ("tool", Json::str(&self.tool)),
            ("toolVersion", Json::str(&self.tool_version)),
            ("coreVersion", Json::str(CORE_VERSION)),
            (
                "assets",
                Json::Arr(
                    self.assets
                        .iter()
                        .map(|a| {
                            Json::obj([
                                ("id", Json::str(&a.id)),
                                ("version", Json::str(&a.version)),
                            ])
                        })
                        .collect(),
                ),
            ),
            ("model", Json::str(&self.model)),
            ("accuracy", Json::str(&self.accuracy)),
            (
                "warnings",
                Json::Arr(warnings.iter().map(Warning::to_json).collect()),
            ),
        ]);
        let Json::Obj(mut pairs) = base else {
            unreachable!("an object")
        };
        if !self.context.is_empty() {
            pairs.push((
                "context".to_owned(),
                Json::Obj(
                    self.context
                        .iter()
                        .map(|(k, v)| ((*k).to_owned(), v.clone()))
                        .collect(),
                ),
            ));
        }
        if let Some(n) = self.notice {
            pairs.push(("notice".to_owned(), Json::str(n)));
        }
        if let Some(l) = self.limitation {
            pairs.push((
                "limitation".to_owned(),
                Json::obj([
                    ("simplification", Json::str(l.simplification)),
                    ("instead", Json::str(l.instead)),
                    ("governs", Json::str(l.governs)),
                ]),
            ));
        }
        Json::Obj(pairs)
    }
}

/// Serializes a success envelope:
/// `{"ok":true,"result":…,"summary"?:…,"comparison"?:…,"display"?:{…},"meta":…}`.
/// `summary` is the rendered sentence, `comparison` the line that frames the
/// answer against something familiar, and `display` holds each output as
/// rounded text with its unit. A non-finite number anywhere becomes an
/// `INTERNAL` error envelope instead, so NaN never escapes.
pub fn success(
    result: Json,
    summary: Option<&str>,
    comparison: Option<&str>,
    display: Json,
    meta: &Meta,
) -> String {
    let mut pairs = vec![("ok", Json::Bool(true)), ("result", result)];
    if let Some(s) = summary.filter(|s| !s.is_empty()) {
        pairs.push(("summary", Json::str(s)));
    }
    if let Some(c) = comparison.filter(|c| !c.is_empty()) {
        pairs.push(("comparison", Json::str(c)));
    }
    if matches!(&display, Json::Obj(p) if !p.is_empty()) {
        pairs.push(("display", display));
    }
    pairs.push(("meta", meta.to_json()));
    let env = Json::obj(pairs);
    match env.to_string() {
        Ok(s) => s,
        Err(e) => failure(&e),
    }
}

pub fn failure(error: &ToolError) -> String {
    Json::obj([("ok", Json::Bool(false)), ("error", error.to_json())])
        .to_string()
        .expect("error envelopes contain no numbers")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;

    fn meta() -> Meta {
        Meta {
            tool: "units.speed.convert".into(),
            tool_version: "1.0.0".into(),
            assets: vec![],
            model: "Exact unit definitions (NIST SP 811)".into(),
            accuracy: "exact to double precision".into(),
            warnings: vec![
                Warning::new("UNIT_ASSUMED", "b").at("/value"),
                Warning::new("INPUT_NORMALIZED", "a").at("/lon"),
            ],
            context: vec![],
            notice: None,
            limitation: None,
        }
    }

    #[test]
    fn success_snapshot() {
        let got = success(
            Json::obj([
                ("value", Json::Num(115.07794480235425)),
                ("unit", Json::str("mph")),
            ]),
            Some("100 kt is 115.078 mph."),
            Some("About a tenth faster than the knots reading."),
            Json::obj([("value", Json::str("115.078 mph"))]),
            &meta(),
        );
        let want = format!(
            concat!(
                r#"{{"ok":true,"result":{{"value":115.07794480235425,"unit":"mph"}},"summary":"100 kt is 115.078 mph.","#,
                r#""comparison":"About a tenth faster than the knots reading.","display":{{"value":"115.078 mph"}},"#,
                r#""meta":{{"tool":"units.speed.convert","toolVersion":"1.0.0","coreVersion":"{}","assets":[],"#,
                r#""model":"Exact unit definitions (NIST SP 811)","accuracy":"exact to double precision","#,
                r#""warnings":[{{"code":"INPUT_NORMALIZED","message":"a","field":"/lon"}},"#,
                r#"{{"code":"UNIT_ASSUMED","message":"b","field":"/value"}}]}}}}"#
            ),
            CORE_VERSION
        );
        assert_eq!(got, want);
    }

    #[test]
    fn nan_result_becomes_internal_error() {
        let got = success(
            Json::obj([("value", Json::Num(f64::NAN))]),
            None,
            None,
            Json::Obj(vec![]),
            &meta(),
        );
        assert!(
            got.starts_with(r#"{"ok":false,"error":{"code":"INTERNAL""#),
            "{got}"
        );
        assert!(got.contains("/result/value"), "{got}");
    }

    #[test]
    fn every_error_code_serializes_with_all_fields() {
        for code in ErrorCode::ALL {
            let e = ToolError::new(code, "msg").at("/f").hint("try this");
            let s = failure(&e);
            assert_eq!(
                s,
                format!(
                    r#"{{"ok":false,"error":{{"code":"{}","message":"msg","field":"/f","hint":"try this"}}}}"#,
                    code.as_str()
                )
            );
        }
    }
}
