//! Structured errors and warnings (tool-contract error model; codes live in
//! `data/codes.json`, which a test keeps in sync with this file).

use crate::json::Json;

/// The closed error enumeration from `contracts/codes-registry`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    InvalidInput,
    OutOfDomain,
    UnitMismatch,
    DidNotConverge,
    DegenerateGeometry,
    NoSolution,
    AssetUnavailable,
    AssetIntegrity,
    LimitExceeded,
    Unsupported,
    Internal,
}

impl ErrorCode {
    pub const ALL: [ErrorCode; 11] = [
        Self::InvalidInput,
        Self::OutOfDomain,
        Self::UnitMismatch,
        Self::DidNotConverge,
        Self::DegenerateGeometry,
        Self::NoSolution,
        Self::AssetUnavailable,
        Self::AssetIntegrity,
        Self::LimitExceeded,
        Self::Unsupported,
        Self::Internal,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidInput => "INVALID_INPUT",
            Self::OutOfDomain => "OUT_OF_DOMAIN",
            Self::UnitMismatch => "UNIT_MISMATCH",
            Self::DidNotConverge => "DID_NOT_CONVERGE",
            Self::DegenerateGeometry => "DEGENERATE_GEOMETRY",
            Self::NoSolution => "NO_SOLUTION",
            Self::AssetUnavailable => "ASSET_UNAVAILABLE",
            Self::AssetIntegrity => "ASSET_INTEGRITY",
            Self::LimitExceeded => "LIMIT_EXCEEDED",
            Self::Unsupported => "UNSUPPORTED",
            Self::Internal => "INTERNAL",
        }
    }
}

/// A failed invocation: `code`, plain-language `message`, JSON Pointer `field`, and `hint`.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolError {
    pub code: ErrorCode,
    pub message: String,
    pub field: Option<String>,
    pub hint: Option<String>,
    /// For `ASSET_UNAVAILABLE` and `ASSET_INTEGRITY`: the asset `id`, `version`, and `key` (file or tile).
    pub asset: Option<Box<(String, String, String)>>,
}

impl ToolError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            field: None,
            hint: None,
            asset: None,
        }
    }

    /// `ASSET_UNAVAILABLE` for one asset file or tile the host must supply.
    pub fn asset_unavailable(id: &str, version: &str, key: &str) -> Self {
        let mut e = Self::new(
            ErrorCode::AssetUnavailable,
            format!("This needs the {id} data ({version}, {key}), which is not loaded yet."),
        );
        e.asset = Some(Box::new((
            id.to_owned(),
            version.to_owned(),
            key.to_owned(),
        )));
        e
    }

    pub fn invalid(field: &str, message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidInput, message).at(field)
    }

    pub fn at(mut self, field: &str) -> Self {
        self.field = Some(field.to_owned());
        self
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn to_json(&self) -> Json {
        let mut obj = vec![
            ("code".to_owned(), Json::Str(self.code.as_str().to_owned())),
            ("message".to_owned(), Json::Str(self.message.clone())),
        ];
        if let Some(f) = &self.field {
            obj.push(("field".to_owned(), Json::Str(f.clone())));
        }
        if let Some(h) = &self.hint {
            obj.push(("hint".to_owned(), Json::Str(h.clone())));
        }
        if let Some(a) = &self.asset {
            let (id, version, key) = &**a;
            obj.push((
                "asset".to_owned(),
                Json::obj([
                    ("id", Json::Str(id.clone())),
                    ("version", Json::Str(version.clone())),
                    ("key", Json::Str(key.clone())),
                ]),
            ));
        }
        Json::Obj(obj)
    }
}

/// A result caveat. `code` must be registered in `data/codes.json`.
#[derive(Clone, Debug, PartialEq)]
pub struct Warning {
    pub code: &'static str,
    pub message: String,
    pub field: Option<String>,
}

impl Warning {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            field: None,
        }
    }

    pub fn at(mut self, field: &str) -> Self {
        self.field = Some(field.to_owned());
        self
    }

    pub fn to_json(&self) -> Json {
        let mut obj = vec![
            ("code".to_owned(), Json::Str(self.code.to_owned())),
            ("message".to_owned(), Json::Str(self.message.clone())),
        ];
        if let Some(f) = &self.field {
            obj.push(("field".to_owned(), Json::Str(f.clone())));
        }
        Json::Obj(obj)
    }
}
