//! Manifest generation (JSON Schema 2020-12 plus the closed `x-` extensions of
//! `contracts/manifest-extensions`) and the catalog lint that fails the build on
//! malformed definitions.

use crate::json::Json;
use crate::profile::Profile;
use crate::tool::{Field, Kind, Precision, Stability, ToolDef};
use crate::units;

const SCHEMA: &str = "https://json-schema.org/draft/2020-12/schema";

fn strs(items: &[&str]) -> Json {
    Json::Arr(items.iter().map(|s| Json::str(*s)).collect())
}

/// The small and large steps the Field-mode buttons move an input by
/// (`x-step`, `ux/mobile-and-field`). A field can override them; otherwise
/// they come from what the input measures, so every numeric input has a pair.
///
/// These are typing conveniences, not tolerances: nothing in a result depends
/// on them.
pub fn steps(f: &Field) -> Option<(f64, f64)> {
    if let Some(pair) = f.step {
        return Some(pair);
    }
    match f.kind {
        Kind::Quantity { unit, .. } => Some(match unit {
            // A degree of latitude is 60 NM, so coordinates step finer.
            "deg" if matches!(f.angle_range, Some("[-90,90]") | Some("[-180,180)")) => {
                (0.001, 0.01)
            }
            "inHg" => (0.01, 0.1),
            "rad" => (0.01, 0.1),
            _ => (1.0, 10.0),
        }),
        // A bounded plain number: a hundredth and a tenth of its span, rounded
        // down to 1, 2, or 5 times a power of ten.
        Kind::Number { min, max } if min.is_finite() && max.is_finite() && max > min => {
            let span = max - min;
            let (small, large) = (nice(span / 100.0), nice(span / 10.0));
            (small > 0.0 && large > small).then_some((small, large))
        }
        Kind::Number { .. } => Some((1.0, 10.0)),
        _ => None,
    }
}

/// Rounds down to 1, 2, or 5 times a power of ten, so a step reads as a round
/// number however odd the span it came from.
fn nice(x: f64) -> f64 {
    if !x.is_finite() || x <= 0.0 {
        return 0.0;
    }
    let decade = 10f64.powi(x.log10().floor() as i32);
    let lead = x / decade;
    decade
        * if lead >= 5.0 {
            5.0
        } else if lead >= 2.0 {
            2.0
        } else {
            1.0
        }
}

fn field_schema(f: &Field, is_input: bool) -> Json {
    let mut o: Vec<(String, Json)> = Vec::new();
    let mut put = |k: &str, v: Json| o.push((k.to_owned(), v));
    match f.kind {
        Kind::Quantity { q, unit } => {
            if is_input {
                put("type", strs(&["number", "string"]));
            } else {
                put("type", Json::str("object"));
                put(
                    "properties",
                    Json::obj([
                        ("value", Json::obj([("type", Json::str("number"))])),
                        ("unit", Json::obj([("type", Json::str("string"))])),
                    ]),
                );
                put("required", strs(&["value", "unit"]));
                put("additionalProperties", Json::Bool(false));
            }
            put("title", Json::str(f.title));
            put("description", Json::str(f.help));
            put("x-quantity", Json::str(q.id()));
            put("x-unit", Json::str(unit));
        }
        Kind::Unit(q) => {
            put("type", Json::str("string"));
            put("title", Json::str(f.title));
            put("description", Json::str(f.help));
            put(
                "enum",
                Json::Arr(units::units_of(q).map(|u| Json::str(u.symbol)).collect()),
            );
        }
        Kind::Choice(options) => {
            put("type", Json::str("string"));
            put("title", Json::str(f.title));
            put("description", Json::str(f.help));
            put("enum", strs(options));
        }
        Kind::AnyQuantity => {
            if is_input {
                put("type", Json::str("string"));
            } else {
                put("type", Json::str("object"));
                put(
                    "properties",
                    Json::obj([
                        ("value", Json::obj([("type", Json::str("number"))])),
                        ("unit", Json::obj([("type", Json::str("string"))])),
                    ]),
                );
                put("required", strs(&["value", "unit"]));
                put("additionalProperties", Json::Bool(false));
            }
            put("title", Json::str(f.title));
            put("description", Json::str(f.help));
            put("x-quantity", Json::str("any"));
        }
        Kind::List { items, min, max } => {
            let props: Vec<(String, Json)> = items
                .iter()
                .map(|it| (it.name.to_owned(), field_schema(it, is_input)))
                .collect();
            let required: Vec<&str> = items
                .iter()
                .filter(|it| if is_input { it.required } else { !it.optional })
                .map(|it| it.name)
                .collect();
            put("type", Json::str("array"));
            put("title", Json::str(f.title));
            put("description", Json::str(f.help));
            put(
                "items",
                Json::obj([
                    ("type", Json::str("object")),
                    ("properties", Json::Obj(props)),
                    ("required", strs(&required)),
                    ("additionalProperties", Json::Bool(false)),
                ]),
            );
            put("minItems", Json::Num(min as f64));
            put("maxItems", Json::Num(max as f64));
        }
        Kind::Text { max_len } => {
            put("type", Json::str("string"));
            put("title", Json::str(f.title));
            put("description", Json::str(f.help));
            put("maxLength", Json::Num(max_len as f64));
        }
        Kind::Number { min, max } => {
            put("type", Json::str("number"));
            put("title", Json::str(f.title));
            put("description", Json::str(f.help));
            put("minimum", Json::Num(min));
            put("maximum", Json::Num(max));
            let (dim, unit) = f.measure.unwrap_or(("dimensionless", "1"));
            put("x-quantity", Json::str(dim));
            put("x-unit", Json::str(unit));
        }
    }
    if let Some(r) = f.angle_range {
        put("x-angle-range", Json::str(r));
    }
    if let Some(p) = f.precision {
        put(
            "x-display-precision",
            match p {
                Precision::Decimals(n) => Json::obj([("decimals", Json::Num(n.into()))]),
                Precision::Significant(n) => Json::obj([("significant", Json::Num(n.into()))]),
                Precision::DecimalsMinSig(n, s) => Json::obj([
                    ("decimals", Json::Num(n.into())),
                    ("minSignificant", Json::Num(s.into())),
                ]),
                Precision::Plain(n) => Json::obj([
                    ("decimals", Json::Num(n.into())),
                    ("grouping", Json::Bool(false)),
                ]),
            },
        );
    }
    if is_input {
        put("x-help", Json::str(f.help));
        if f.core {
            put("x-core", Json::Bool(true));
        }
        if let Some((small, large)) = steps(f) {
            put(
                "x-step",
                Json::obj([("small", Json::Num(small)), ("large", Json::Num(large))]),
            );
        }
    } else if let Some(s) = f.status {
        put(
            "x-status",
            Json::obj([("kind", Json::str(s.kind)), ("source", Json::str(s.source))]),
        );
    }
    Json::Obj(o)
}

fn options_schema(def: &ToolDef) -> Json {
    let mut props = vec![(
        "numberFormat".to_owned(),
        Json::obj([
            ("type", Json::str("string")),
            ("enum", strs(&["decimal-point", "decimal-comma"])),
            (
                "description",
                Json::str("How to read numbers written as strings. Default decimal-point."),
            ),
        ]),
    )];
    if def.has_quantity_outputs() {
        props.push((
            "profile".to_owned(),
            Json::obj([
                ("type", Json::str("string")),
                ("enum", strs(&Profile::IDS)),
                (
                    "description",
                    Json::str("Unit profile for quantity outputs."),
                ),
            ]),
        ));
        props.push((
            "outputUnits".to_owned(),
            Json::obj([
                ("type", Json::str("object")),
                (
                    "additionalProperties",
                    Json::obj([("type", Json::str("string"))]),
                ),
                (
                    "description",
                    Json::str("Unit per output field, overriding the profile."),
                ),
            ]),
        ));
    }
    props.push((
        "explain".to_owned(),
        Json::obj([
            ("type", Json::str("boolean")),
            (
                "description",
                Json::str(
                    "Show the tool's work: each step's formula, the formula with these values in it, and what it came to. Default false.",
                ),
            ),
        ]),
    ));
    Json::obj([
        ("type", Json::str("object")),
        ("properties", Json::Obj(props)),
        ("additionalProperties", Json::Bool(false)),
    ])
}

fn object_schema(fields: &[Field], is_input: bool, def: &ToolDef) -> Json {
    let mut props: Vec<(String, Json)> = fields
        .iter()
        .map(|f| (f.name.to_owned(), field_schema(f, is_input)))
        .collect();
    if is_input {
        props.push(("options".to_owned(), options_schema(def)));
    }
    let required: Vec<&str> = fields
        .iter()
        .filter(|f| if is_input { f.required } else { !f.optional })
        .map(|f| f.name)
        .collect();
    Json::obj([
        ("$schema", Json::str(SCHEMA)),
        ("type", Json::str("object")),
        ("properties", Json::Obj(props)),
        ("required", strs(&required)),
        ("additionalProperties", Json::Bool(false)),
    ])
}

/// The manifest for one tool, with a fixed key order.
pub fn manifest(def: &ToolDef) -> Json {
    let mut o: Vec<(String, Json)> = Vec::new();
    let mut put = |k: &str, v: Json| o.push((k.to_owned(), v));
    put("id", Json::str(def.id));
    put("version", Json::str(def.version));
    put("title", Json::str(def.title));
    put("summary", Json::str(def.summary));
    put("domain", Json::str(def.domain()));
    put("group", Json::str(def.group()));
    put("aliases", strs(def.aliases));
    put("keywords", strs(def.keywords));
    put("inputs", object_schema(def.inputs, true, def));
    put("outputs", object_schema(def.outputs, false, def));
    put(
        "errors",
        Json::Arr(def.errors.iter().map(|c| Json::str(c.as_str())).collect()),
    );
    // A stable tool never emits EXPERIMENTAL_TOOL, so it does not advertise it.
    let warnings: Vec<&str> = def
        .warnings
        .iter()
        .copied()
        .filter(|w| def.stability == Stability::Experimental || *w != "EXPERIMENTAL_TOOL")
        .collect();
    put("warnings", strs(&warnings));
    put("model", Json::str(def.model));
    put("accuracy", Json::str(def.accuracy));
    if !def.when_to_use.is_empty() {
        put("whenToUse", Json::str(def.when_to_use));
    }
    if !def.limitations.is_empty() {
        put("limitations", Json::str(def.limitations));
    }
    put(
        "references",
        Json::Arr(
            def.references
                .iter()
                .map(|r| {
                    Json::obj([
                        ("title", Json::str(r.title)),
                        ("issuer", Json::str(r.issuer)),
                        ("year", Json::Num(r.year.into())),
                        ("edition", Json::str(r.edition)),
                        ("locator", Json::str(r.locator)),
                        ("url", Json::str(r.url)),
                    ])
                })
                .collect(),
        ),
    );
    put(
        "examples",
        Json::Arr(
            def.examples
                .iter()
                .map(|e| {
                    let input: serde_json::Value =
                        serde_json::from_str(e.input).expect("example input is JSON");
                    Json::obj([
                        ("id", Json::str(e.id)),
                        ("title", Json::str(e.title)),
                        ("input", from_value(&input)),
                        ("source", Json::str(e.source)),
                    ])
                })
                .collect(),
        ),
    );
    put(
        "vectors",
        Json::str(format!("core/vectors/{}.jsonl", def.id)),
    );
    put("assets", strs(def.assets));
    put(
        "visualization",
        Json::Arr(
            def.visualization
                .iter()
                .map(|l| {
                    Json::obj([
                        ("kind", Json::str(l.kind)),
                        (
                            "map",
                            Json::Obj(
                                l.map
                                    .iter()
                                    .map(|(k, v)| ((*k).to_owned(), Json::str(*v)))
                                    .collect(),
                            ),
                        ),
                    ])
                })
                .collect(),
        ),
    );
    // One slot per input, in input order: explicit slots in full, the rest
    // by name only (the parser derives their keywords from the input).
    put(
        "prefill",
        Json::Arr(
            def.inputs
                .iter()
                .map(|f| {
                    let mut o = vec![("input", Json::str(f.name))];
                    if let Some(sl) = def.slots.iter().find(|sl| sl.input == f.name) {
                        o.push((
                            "keywords",
                            Json::Arr(sl.keywords.iter().map(|k| Json::str(*k)).collect()),
                        ));
                        if sl.range.0.is_finite() || sl.range.1.is_finite() {
                            let end = |x: f64| {
                                if x.is_finite() {
                                    Json::Num(x)
                                } else {
                                    Json::Null
                                }
                            };
                            o.push(("range", Json::Arr(vec![end(sl.range.0), end(sl.range.1)])));
                        }
                        o.push((
                            "bare",
                            Json::str(match sl.bare {
                                crate::tool::Bare::Never => "never",
                                crate::tool::Bare::Any => "any",
                                crate::tool::Bare::Decimal => "decimal",
                            }),
                        ));
                    }
                    Json::obj(o)
                })
                .collect(),
        ),
    );
    if let Some(t) = def.timeline {
        put(
            "timeline",
            Json::obj([
                ("input", Json::str(t.input)),
                ("end", Json::str(t.end)),
                ("key", Json::str(t.key)),
            ]),
        );
    }
    put(
        "related",
        Json::Arr(
            def.related
                .iter()
                .map(|r| Json::obj([("id", Json::str(r.id)), ("reason", Json::str(r.reason))]))
                .collect(),
        ),
    );
    put("stability", Json::str(def.stability.id()));
    if let Stability::Deprecated {
        replacement,
        removal,
    } = def.stability
    {
        put(
            "deprecation",
            Json::obj([
                ("replacement", Json::str(replacement)),
                ("removal", Json::str(removal)),
            ]),
        );
    }
    put("since", Json::str(def.since));
    put("composedOf", strs(def.composed_of));
    if !def.preset.is_empty() {
        let preset = def
            .preset
            .iter()
            .map(|(k, v)| {
                let v: serde_json::Value = serde_json::from_str(v).expect("preset is JSON");
                ((*k).to_owned(), from_value(&v))
            })
            .collect();
        put("preset", Json::Obj(preset));
        put("justification", Json::str(def.justification));
    }
    put(
        "limits",
        Json::Obj(
            def.limits
                .iter()
                .map(|(k, v)| ((*k).to_owned(), Json::Num(*v as f64)))
                .collect(),
        ),
    );
    put("x-sentence", Json::str(def.sentence));
    put(
        "x-comparison",
        if def.comparison.text.is_empty() {
            Json::obj([("kind", Json::str(def.comparison.kind))])
        } else {
            Json::obj([
                ("kind", Json::str(def.comparison.kind)),
                ("text", Json::str(def.comparison.text)),
            ])
        },
    );
    if let Some(l) = def.limitation {
        put(
            "x-limitation",
            Json::obj([
                ("simplification", Json::str(l.simplification)),
                ("instead", Json::str(l.instead)),
                ("governs", Json::str(l.governs)),
            ]),
        );
    }
    if !def.assumptions.is_empty() {
        put(
            "x-assumptions",
            Json::Arr(
                def.assumptions
                    .iter()
                    .map(|a| {
                        Json::obj([
                            ("name", Json::str(a.name)),
                            ("value", Json::str(a.value)),
                            ("unit", Json::str(a.unit)),
                            ("source", Json::str(a.source)),
                        ])
                    })
                    .collect(),
            ),
        );
    }
    if def.diagram_inline {
        put("x-diagram-inline", Json::Bool(true));
    }
    put("x-clock-default", Json::str("forbidden"));
    put("x-primary-example", Json::str(def.primary_example));
    Json::Obj(o)
}

/// Converts parsed JSON to the ordered output type, keeping object key order.
pub fn from_value(v: &serde_json::Value) -> Json {
    match v {
        serde_json::Value::Null => Json::Null,
        serde_json::Value::Bool(b) => Json::Bool(*b),
        serde_json::Value::Number(n) => Json::Num(n.as_f64().unwrap_or(f64::NAN)),
        serde_json::Value::String(s) => Json::str(s.clone()),
        serde_json::Value::Array(a) => Json::Arr(a.iter().map(from_value).collect()),
        serde_json::Value::Object(m) => {
            Json::Obj(m.iter().map(|(k, v)| (k.clone(), from_value(v))).collect())
        }
    }
}

/// The thing a reader counts, for a core input: a coordinate's two fields are
/// one point, and anything else counts as itself.
pub fn core_group(name: &str) -> &str {
    const POINT: &[&str] = &[
        "lat",
        "latitude",
        "lon",
        "lng",
        "longitude",
        "north",
        "northing",
        "east",
        "easting",
    ];
    // "lat1" and "lon1" are the same point; "lat2" is a different one.
    let digits = name.trim_end_matches(|c: char| c.is_ascii_digit());
    if POINT.contains(&digits) {
        return match &name[digits.len()..] {
            "" => "point",
            "1" => "point1",
            "2" => "point2",
            "3" => "point3",
            _ => "pointN",
        };
    }
    // "b_north" and "b_east" are the same point.
    if let Some((prefix, part)) = name.rsplit_once('_')
        && POINT.contains(&part)
        && !prefix.is_empty()
    {
        return prefix;
    }
    name
}

/// Visualization layer kinds (tool-contract "Each tool declares its visualization").
pub const LAYER_KINDS: &[&str] = &[
    "point",
    "line-geodesic",
    "line-rhumb",
    "polygon",
    "bbox",
    "cell-set",
    "vector-diagram",
    "profile-chart",
    "gauge",
    "table-only",
];

const RELATED_REASONS: &[&str] = &["inverse", "next", "alternative", "parent"];

fn valid_id(id: &str) -> bool {
    let segs: Vec<&str> = id.split('.').collect();
    segs.len() == 3
        && segs.iter().all(|s| {
            !s.is_empty()
                && s.split('-').all(|p| {
                    !p.is_empty()
                        && p.bytes()
                            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
                })
        })
}

fn valid_semver(v: &str) -> bool {
    let p: Vec<&str> = v.split('.').collect();
    p.len() == 3
        && p.iter()
            .all(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// Taxonomy: (domain, groups).
pub type Taxonomy<'a> = &'a [(&'a str, Vec<&'a str>)];

/// Checks every rule a single build can check. Returns one message per problem,
/// each naming the tool.
pub fn lint(tools: &[&ToolDef], taxonomy: Taxonomy, known_ids: &[&str]) -> Vec<String> {
    let mut errs = Vec::new();
    let exists = |id: &str| tools.iter().any(|t| t.id == id) || known_ids.contains(&id);
    for (i, t) in tools.iter().enumerate() {
        let id = t.id;
        let mut e = |m: String| errs.push(format!("{id}: {m}"));
        if !valid_id(id) {
            e("id must match ^[a-z0-9]+(-[a-z0-9]+)*(\\.[a-z0-9]+(-[a-z0-9]+)*){2}$ (<domain>.<group>.<operation>)".into());
        }
        match taxonomy.iter().find(|(d, _)| *d == t.domain()) {
            None => e(format!("domain {} is not in the taxonomy", t.domain())),
            Some((_, groups)) if !groups.contains(&t.group()) => e(format!(
                "group {} is not in the {} taxonomy",
                t.group(),
                t.domain()
            )),
            _ => {}
        }
        if tools[..i].iter().any(|o| o.id == id) {
            e("id is declared twice".into());
        }
        for (name, v) in [("version", t.version), ("since", t.since)] {
            if !valid_semver(v) {
                e(format!("{name} {v} is not semver"));
            }
        }
        for (name, v) in [
            ("title", t.title),
            ("summary", t.summary),
            ("model", t.model),
            ("accuracy", t.accuracy),
            ("sentence", t.sentence),
        ] {
            if v.trim().is_empty() {
                e(format!("missing {name}"));
            }
        }
        // A stable tool has a page a reader is meant to land on, so it says
        // when to reach for it and where its answer stops.
        if t.stability == Stability::Stable && t.parent.is_none() {
            for (name, v) in [
                ("when_to_use", t.when_to_use),
                ("limitations", t.limitations),
            ] {
                if v.trim().is_empty() {
                    e(format!("missing {name}"));
                }
            }
        }
        let fields = t
            .inputs
            .iter()
            .chain(t.parent.into_iter().flat_map(|p| p.inputs));
        let known: Vec<&str> = fields.clone().chain(t.outputs).map(|f| f.name).collect();
        match crate::template::check(t.sentence, &known) {
            Ok(codes) => {
                for c in codes {
                    if !t.warnings.contains(&c.as_str()) {
                        e(format!(
                            "sentence uses warning {c}, which the tool does not declare"
                        ));
                    }
                }
            }
            Err(m) => e(format!("sentence template {m}")),
        }
        for f in fields.clone() {
            if f.title.is_empty() || f.help.is_empty() {
                e(format!("input {} needs a title and help", f.name));
            }
            if let Kind::Quantity { q, unit } = f.kind
                && units::by_symbol(q, unit).is_none()
            {
                e(format!(
                    "input {} declares unknown unit {unit} for {}",
                    f.name,
                    q.id()
                ));
            }
        }
        // A coordinate is two fields and one decision, so it counts once, the
        // same way a list of rows does (contracts/manifest-extensions).
        let mut groups: Vec<&str> = t
            .inputs
            .iter()
            .filter(|f| f.core)
            .map(|f| core_group(f.name))
            .collect();
        groups.sort_unstable();
        groups.dedup();
        if groups.len() > 5 {
            e("more than 5 x-core inputs".into());
        }
        for f in t.outputs {
            if let Kind::Quantity { q, unit } = f.kind
                && units::by_symbol(q, unit).is_none()
            {
                e(format!(
                    "output {} declares unknown unit {unit} for {}",
                    f.name,
                    q.id()
                ));
            }
            if matches!(
                f.kind,
                Kind::Quantity { .. } | Kind::Number { .. } | Kind::AnyQuantity
            ) && f.precision.is_none()
            {
                e(format!(
                    "numeric output {} needs x-display-precision",
                    f.name
                ));
            }
        }
        if let Some(l) = t.limitation {
            let (a, b, c) = crate::tool::LIMITATION_CAPS;
            for (what, text, cap) in [
                ("simplification", l.simplification, a),
                ("instead", l.instead, b),
                ("governs", l.governs, c),
            ] {
                if text.is_empty() {
                    e(format!("x-limitation needs {what}"));
                } else if text.chars().count() > cap {
                    e(format!(
                        "x-limitation {what} is {} characters (at most {cap})",
                        text.chars().count()
                    ));
                }
            }
        }
        if !crate::tool::COMPARISON_KINDS.contains(&t.comparison.kind) {
            e(format!(
                "x-comparison kind {} is not one of {}",
                t.comparison.kind,
                crate::tool::COMPARISON_KINDS.join(", ")
            ));
        }
        if (t.comparison.kind == "none") != t.comparison.text.is_empty() {
            e("x-comparison kind none takes no text, and any other kind needs it".into());
        }
        if !t.comparison.text.is_empty() {
            match crate::template::check(t.comparison.text, &known) {
                Ok(codes) => {
                    for c in codes {
                        if !t.warnings.iter().any(|w| *w == c) {
                            e(format!(
                                "comparison uses warning {c}, which the tool does not declare"
                            ));
                        }
                    }
                }
                Err(m) => e(format!("comparison template {m}")),
            }
        }
        for f in t.outputs {
            let Some(st) = f.status else { continue };
            if !crate::tool::STATUS_KINDS.contains(&st.kind) {
                e(format!(
                    "output {} declares x-status kind {}, which is not one of {}",
                    f.name,
                    st.kind,
                    crate::tool::STATUS_KINDS.join(", ")
                ));
            }
            if st.source.is_empty() {
                e(format!(
                    "output {} needs a source for its threshold",
                    f.name
                ));
            }
            if !matches!(f.kind, Kind::Text { .. }) {
                e(format!("x-status output {} must be text", f.name));
            }
        }
        for f in t.inputs {
            if f.status.is_some() {
                e(format!("input {} may not declare x-status", f.name));
            }
        }
        if t.references.is_empty() {
            e("needs at least one reference".into());
        }
        for r in t.references {
            if r.issuer.is_empty() || r.year < 1800 || r.title.is_empty() {
                e(format!(
                    "reference \"{}\" needs a title, issuing body, and year",
                    r.title
                ));
            }
        }
        if t.examples.is_empty() {
            e("needs at least one worked example".into());
        }
        if !t.examples.iter().any(|x| x.id == t.primary_example) {
            e(format!(
                "primary example {} is not among its examples",
                t.primary_example
            ));
        }
        for x in t.examples {
            if !matches!(
                serde_json::from_str::<serde_json::Value>(x.input),
                Ok(serde_json::Value::Object(_))
            ) {
                e(format!("example {} input is not a JSON object", x.id));
            }
        }
        if t.visualization.is_empty() {
            e("needs a visualization descriptor".into());
        }
        for sl in t.slots {
            if !t.inputs.iter().any(|f| f.name == sl.input) {
                e(format!("prefill slot {} is not an input", sl.input));
            }
            for k in sl.keywords {
                if k.is_empty()
                    || !k
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
                {
                    e(format!(
                        "prefill slot {} keyword {k:?} must be one lowercase word",
                        sl.input
                    ));
                }
            }
            // Not `>=`: a NaN bound is an empty range too, and `>=` would pass it.
            if !matches!(
                sl.range.0.partial_cmp(&sl.range.1),
                Some(core::cmp::Ordering::Less)
            ) {
                e(format!("prefill slot {} range is empty", sl.input));
            }
        }
        if let Some(tl) = t.timeline {
            if !t.inputs.iter().any(|f| f.name == tl.input) {
                e(format!("timeline input {} is not an input", tl.input));
            }
            for out in [tl.end, tl.key] {
                if !t.outputs.iter().any(|f| f.name == out) {
                    e(format!("timeline output {out} is not an output"));
                }
            }
        }
        for l in t.visualization {
            if !LAYER_KINDS.contains(&l.kind) {
                e(format!("unknown visualization layer kind {}", l.kind));
            }
            for (layer_input, out) in l.map {
                if !t.outputs.iter().any(|f| f.name == *out) {
                    e(format!(
                        "visualization maps {layer_input} to missing output field {out}"
                    ));
                }
            }
        }
        for r in t.related {
            if !RELATED_REASONS.contains(&r.reason) {
                e(format!("related {} has unknown reason {}", r.id, r.reason));
            }
            if !exists(r.id) {
                e(format!("related tool {} does not exist", r.id));
            } else if r.reason == "inverse" {
                let back = tools.iter().find(|o| o.id == r.id);
                if let Some(back) = back
                    && !back
                        .related
                        .iter()
                        .any(|b| b.id == id && b.reason == "inverse")
                {
                    e(format!(
                        "declares {} as its inverse, but {} does not declare it back",
                        r.id, r.id
                    ));
                }
            }
        }
        for c in t.composed_of {
            if !exists(c) {
                e(format!("composedOf {c} does not exist"));
            }
        }
        if !t.preset.is_empty() && t.justification.is_empty() {
            e("generated endpoint needs an allow-list justification".into());
        }
        if !t.composed_of.is_empty() && t.parent.is_none() && t.preset.is_empty() {
            e("generated endpoint needs a parent or preset".into());
        }
        for (k, v) in t.preset {
            if serde_json::from_str::<serde_json::Value>(v).is_err() {
                e(format!("preset {k} is not JSON"));
            }
        }
        for a in t.aliases {
            for o in tools.iter() {
                if o.id != id && (o.aliases.contains(a) || o.id == *a) {
                    e(format!("alias {a} is also used by {}", o.id));
                }
            }
        }
    }
    errs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::{Example, Layer, Reference, Related};
    use crate::units::Quantity;

    const IN: &[Field] = &[Field::new(
        "x",
        "X",
        "12 ft",
        Kind::Quantity {
            q: Quantity::Length,
            unit: "ft",
        },
    )
    .required()];
    const OUT: &[Field] = &[Field::new(
        "y",
        "Y",
        "3.6576 m",
        Kind::Quantity {
            q: Quantity::Length,
            unit: "m",
        },
    )
    .precision(Precision::Decimals(3))];
    const REF: &[Reference] = &[Reference {
        title: "NIST SP 811",
        issuer: "NIST",
        year: 2008,
        edition: "2008",
        locator: "App. B",
        url: "https://www.nist.gov/pml/special-publication-811",
    }];
    const EX: &[Example] = &[Example {
        id: "ex1",
        title: "t",
        input: r#"{"x":12}"#,
        source: "s",
    }];
    const VIZ: &[Layer] = &[Layer {
        kind: "table-only",
        map: &[],
    }];

    const GOOD: ToolDef = ToolDef {
        id: "units.length.sample",
        title: "Sample",
        summary: "A sample tool.",
        inputs: IN,
        outputs: OUT,
        model: "exact",
        accuracy: "exact",
        references: REF,
        examples: EX,
        primary_example: "ex1",
        visualization: VIZ,
        sentence: "{x} is {y}.",
        ..ToolDef::BLANK
    };

    fn tax() -> Vec<(&'static str, Vec<&'static str>)> {
        vec![("units", vec!["length"])]
    }

    #[test]
    fn good_tool_passes() {
        assert_eq!(lint(&[&GOOD], &tax(), &[]), Vec::<String>::new());
    }

    fn fails(t: &ToolDef, needle: &str) {
        let errs = lint(&[t], &tax(), &[]);
        assert!(
            errs.iter().any(|e| e.contains(needle)),
            "expected {needle:?} in {errs:?}"
        );
    }

    /// Every field the sample manifest must carry, blanked one at a time. A
    /// required field with nothing in it has to be refused, whichever one it
    /// Every numeric input carries a step pair for the Field-mode buttons, and
    /// the pair suits what the input measures (ux/mobile-and-field).
    #[test]
    fn every_numeric_input_has_a_sensible_step_pair() {
        let len = |unit| Kind::Quantity {
            q: Quantity::Length,
            unit,
        };
        let angle = Kind::Quantity {
            q: Quantity::Angle,
            unit: "deg",
        };
        let f = |k| Field::new("v", "V", "help", k);
        assert_eq!(steps(&f(len("ft"))), Some((1.0, 10.0)));
        // A degree of latitude is 60 NM, so a coordinate steps finer.
        assert_eq!(
            steps(&f(angle).angle_range("[-90,90]")),
            Some((0.001, 0.01))
        );
        // A heading is not a coordinate, however, and steps by the degree.
        assert_eq!(steps(&f(angle).angle_range("[0,360)")), Some((1.0, 10.0)));
        assert_eq!(
            steps(&f(Kind::Quantity {
                q: Quantity::Pressure,
                unit: "inHg"
            })),
            Some((0.01, 0.1))
        );
        // A bounded plain number steps by a hundredth and a tenth of its span.
        assert_eq!(
            steps(&f(Kind::Number { min: 0.0, max: 1.0 })),
            Some((0.01, 0.1))
        );
        assert_eq!(
            steps(&f(Kind::Number {
                min: 0.0,
                max: 30.0
            })),
            Some((0.2, 2.0))
        );
        // A field may say so itself.
        assert_eq!(
            steps(&f(len("ft")).step(100.0, 1000.0)),
            Some((100.0, 1000.0))
        );
        // Nothing to step: a choice, a unit, a piece of text.
        assert_eq!(steps(&f(Kind::Text { max_len: 8 })), None);
        assert_eq!(steps(&f(Kind::Choice(&["a", "b"]))), None);
    }

    /// is (platform/tool-contract, "the meta-schema rejects each missing
    /// required field").
    #[test]
    fn a_coordinate_counts_as_one_core_input() {
        assert_eq!(core_group("lat1"), core_group("lon1"));
        assert_ne!(core_group("lat1"), core_group("lat2"));
        assert_eq!(core_group("lat"), core_group("lon"));
        assert_eq!(core_group("b_north"), core_group("b_east"));
        assert_eq!(core_group("temperature"), "temperature");
        assert_eq!(core_group("northing"), core_group("easting"));
    }

    #[test]
    fn each_missing_required_field_is_rejected() {
        let cases: Vec<(&str, ToolDef)> = vec![
            ("id", ToolDef { id: "", ..GOOD }),
            ("title", ToolDef { title: "", ..GOOD }),
            (
                "summary",
                ToolDef {
                    summary: "",
                    ..GOOD
                },
            ),
            ("model", ToolDef { model: "", ..GOOD }),
            (
                "accuracy",
                ToolDef {
                    accuracy: "",
                    ..GOOD
                },
            ),
            (
                "references",
                ToolDef {
                    references: &[],
                    ..GOOD
                },
            ),
            (
                "examples",
                ToolDef {
                    examples: &[],
                    ..GOOD
                },
            ),
            (
                "primary_example",
                ToolDef {
                    primary_example: "",
                    ..GOOD
                },
            ),
            (
                "sentence",
                ToolDef {
                    sentence: "",
                    ..GOOD
                },
            ),
            (
                "outputs",
                ToolDef {
                    outputs: &[],
                    ..GOOD
                },
            ),
            (
                "visualization",
                ToolDef {
                    visualization: &[],
                    ..GOOD
                },
            ),
        ];
        let mut accepted = Vec::new();
        for (field, def) in &cases {
            if lint(&[def], &tax(), &[]).is_empty() {
                accepted.push(*field);
            }
        }
        assert!(
            accepted.is_empty(),
            "a manifest with no {accepted:?} was accepted"
        );
        // The sample itself, with everything present, passes.
        assert_eq!(lint(&[&GOOD], &tax(), &[]), Vec::<String>::new());
    }

    #[test]
    fn invalid_id_rejected() {
        fails(
            &ToolDef {
                id: "Geodesy.UTM_Forward",
                ..GOOD
            },
            "id must match",
        );
        fails(
            &ToolDef {
                id: "units.length",
                ..GOOD
            },
            "id must match",
        );
    }

    #[test]
    fn missing_reference_rejected() {
        fails(
            &ToolDef {
                references: &[],
                ..GOOD
            },
            "at least one reference",
        );
        const BAD: &[Reference] = &[Reference { year: 0, ..REF[0] }];
        fails(
            &ToolDef {
                references: BAD,
                ..GOOD
            },
            "issuing body, and year",
        );
    }

    #[test]
    fn invalid_slot_rejected() {
        // natural-language-prefill "Invalid slot": a slot must name a real input.
        use crate::tool::Slot;
        const BAD: &[Slot] = &[Slot::new("nonexistent", &["x"])];
        fails(
            &ToolDef { slots: BAD, ..GOOD },
            "prefill slot nonexistent is not an input",
        );
        const CASE: &[Slot] = &[Slot::new(GOOD.inputs[0].name, &["OAT"])];
        fails(
            &ToolDef {
                slots: CASE,
                ..GOOD
            },
            "must be one lowercase word",
        );
    }

    #[test]
    fn visualization_mapping_validated() {
        const V: &[Layer] = &[Layer {
            kind: "line-geodesic",
            map: &[("path", "route")],
        }];
        fails(
            &ToolDef {
                visualization: V,
                ..GOOD
            },
            "missing output field route",
        );
        const K: &[Layer] = &[Layer {
            kind: "sparkles",
            map: &[],
        }];
        fails(
            &ToolDef {
                visualization: K,
                ..GOOD
            },
            "unknown visualization layer kind",
        );
    }

    #[test]
    fn too_many_core_inputs() {
        const F: Field = Field::new(
            "a",
            "A",
            "1 m",
            Kind::Quantity {
                q: Quantity::Length,
                unit: "m",
            },
        )
        .core();
        // Six separate inputs, not one repeated: a coordinate's two fields count
        // once, so the names have to differ for this to be six.
        const SIX: &[Field] = &[
            Field { name: "a", ..F },
            Field { name: "b", ..F },
            Field { name: "c", ..F },
            Field { name: "d", ..F },
            Field { name: "e", ..F },
            Field { name: "f", ..F },
        ];
        fails(
            &ToolDef {
                inputs: SIX,
                ..GOOD
            },
            "more than 5 x-core",
        );
    }

    #[test]
    fn taxonomy_enforced() {
        fails(
            &ToolDef {
                id: "units.mass.sample",
                ..GOOD
            },
            "group mass is not in the units taxonomy",
        );
        fails(
            &ToolDef {
                id: "weather.x.y",
                ..GOOD
            },
            "domain weather is not in the taxonomy",
        );
    }

    #[test]
    fn inverse_symmetry() {
        const A_REL: &[Related] = &[Related {
            id: "units.length.b",
            reason: "inverse",
        }];
        let a = ToolDef {
            id: "units.length.a",
            related: A_REL,
            ..GOOD
        };
        let b = ToolDef {
            id: "units.length.b",
            ..GOOD
        };
        let errs = lint(&[&a, &b], &tax(), &[]);
        assert!(
            errs.iter().any(|e| e.contains("does not declare it back")),
            "{errs:?}"
        );
        const B_REL: &[Related] = &[Related {
            id: "units.length.a",
            reason: "inverse",
        }];
        let b = ToolDef {
            id: "units.length.b",
            related: B_REL,
            ..GOOD
        };
        assert_eq!(lint(&[&a, &b], &tax(), &[]), Vec::<String>::new());
    }

    #[test]
    fn alias_uniqueness() {
        const AL: &[&str] = &["feet converter"];
        let a = ToolDef {
            id: "units.length.a",
            aliases: AL,
            ..GOOD
        };
        let b = ToolDef {
            id: "units.length.b",
            aliases: AL,
            ..GOOD
        };
        let errs = lint(&[&a, &b], &tax(), &[]);
        assert!(
            errs.iter()
                .any(|e| e.contains("alias feet converter is also used")),
            "{errs:?}"
        );
    }

    #[test]
    fn bad_sentence_template() {
        fails(
            &ToolDef {
                sentence: "{z} is {y}.",
                ..GOOD
            },
            "sentence template names unknown field z",
        );
        fails(
            &ToolDef {
                sentence: "{if y < 1}x",
                ..GOOD
            },
            "sentence template unclosed",
        );
    }

    #[test]
    fn missing_precision_and_primary_example() {
        const O: &[Field] = &[Field::new(
            "y",
            "Y",
            "1 m",
            Kind::Quantity {
                q: Quantity::Length,
                unit: "m",
            },
        )];
        fails(&ToolDef { outputs: O, ..GOOD }, "x-display-precision");
        fails(
            &ToolDef {
                primary_example: "nope",
                ..GOOD
            },
            "primary example nope",
        );
    }

    #[test]
    fn manifest_snapshot() {
        let m = manifest(&GOOD).to_string().unwrap();
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/sample-manifest.json"
        );
        if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
            std::fs::write(path, format!("{m}\n")).unwrap();
        }
        let want = std::fs::read_to_string(path).unwrap();
        let want = want.trim();
        assert_eq!(m, want);
    }
}
