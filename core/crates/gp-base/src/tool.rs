//! Declarative tool definitions and the invocation runtime (tool-contract and
//! compute-core specs). A tool is a static [`ToolDef`]; its manifest, input
//! validation, output unit handling, and provenance all come from that one
//! definition, so the surfaces cannot drift apart.

use serde_json::{Map, Value};

use crate::envelope::{self, AssetRef, Meta};
use crate::error::{ErrorCode, ToolError, Warning};
use crate::json::Json;
use crate::parse::{self, NumberFormat};
use crate::profile::Profile;
use crate::template::{self, Scope, Val};
use crate::units::{self, Quantity, Unit};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stability {
    Experimental,
    Stable,
    Deprecated {
        replacement: &'static str,
        removal: &'static str,
    },
}

impl Stability {
    pub fn id(self) -> &'static str {
        match self {
            Self::Experimental => "experimental",
            Self::Stable => "stable",
            Self::Deprecated { .. } => "deprecated",
        }
    }
}

/// Display rounding for humans (`x-display-precision`). Machine values are never rounded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Precision {
    Decimals(u8),
    Significant(u8),
    /// Decimals without digit grouping, for identifiers (GPS week 2436, JD 2461301.5).
    Plain(u8),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// A physical quantity: a JSON number in `unit`, or a unit-tagged string ("145 kts").
    Quantity { q: Quantity, unit: &'static str },
    /// A unit symbol (or alias) of quantity `q`.
    Unit(Quantity),
    /// One of a closed set of strings.
    Choice(&'static [&'static str]),
    /// A dimensionless number within `[min, max]`.
    Number { min: f64, max: f64 },
    /// A value of any quantity: a unit-tagged string in, `{value, unit}` out.
    AnyQuantity,
    /// A short string: a designator or coded group in, a status phrase out.
    Text { max_len: usize },
    /// A list of rows, each an object with the given fields (traverse courses,
    /// polygon vertices). Outputs use the same kind for tables.
    List {
        items: &'static [Field],
        min: usize,
        max: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Field {
    pub name: &'static str,
    pub title: &'static str,
    /// One line of help with an example value (`x-help`).
    pub help: &'static str,
    pub kind: Kind,
    pub required: bool,
    /// For outputs: may be absent from the result.
    pub optional: bool,
    /// Shown by default (`x-core`); at most 5 per tool.
    pub core: bool,
    pub precision: Option<Precision>,
    /// `x-angle-range` for angular fields.
    pub angle_range: Option<&'static str>,
    /// For a `Number` in a fixed unit the unit system does not convert (nT,
    /// A): its dimension and unit, emitted as `x-quantity` and `x-unit`.
    pub measure: Option<(&'static str, &'static str)>,
}

impl Field {
    pub const fn new(
        name: &'static str,
        title: &'static str,
        help: &'static str,
        kind: Kind,
    ) -> Field {
        Field {
            name,
            title,
            help,
            kind,
            required: false,
            optional: false,
            core: false,
            precision: None,
            angle_range: None,
            measure: None,
        }
    }

    pub const fn required(mut self) -> Field {
        self.required = true;
        self
    }

    pub const fn optional(mut self) -> Field {
        self.optional = true;
        self
    }

    pub const fn core(mut self) -> Field {
        self.core = true;
        self
    }

    pub const fn precision(mut self, p: Precision) -> Field {
        self.precision = Some(p);
        self
    }

    pub const fn angle_range(mut self, r: &'static str) -> Field {
        self.angle_range = Some(r);
        self
    }

    pub const fn measure(mut self, dimension: &'static str, unit: &'static str) -> Field {
        self.measure = Some((dimension, unit));
        self
    }
}

/// A cited source: standard, paper, or agency publication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reference {
    pub title: &'static str,
    pub issuer: &'static str,
    pub year: u16,
    pub edition: &'static str,
    /// Section, table, or page.
    pub locator: &'static str,
    pub url: &'static str,
}

/// A worked example. `input` is a JSON object literal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Example {
    pub id: &'static str,
    pub title: &'static str,
    pub input: &'static str,
    pub source: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Related {
    pub id: &'static str,
    /// `inverse`, `next`, `alternative`, or `parent`.
    pub reason: &'static str,
}

/// A canvas layer and its mapping from layer inputs to output fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layer {
    pub kind: &'static str,
    pub map: &'static [(&'static str, &'static str)],
}

/// A scene the canvas can play (tool-contract "timeline"): the input that sets
/// the playhead, the output that ends its range, and the output holding the
/// key moment the scene opens on. Every value at the playhead comes from the
/// core, because the playhead is an input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Timeline {
    pub input: &'static str,
    pub end: &'static str,
    pub key: &'static str,
}

/// Whether a bare number (no unit) can fill a prefill slot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Bare {
    Never,
    /// Any bare number inside the slot's range.
    Any,
    /// Only a bare number written with a decimal point (29.92, not 30).
    Decimal,
}

/// How a typed question fills one input (discovery/natural-language-prefill
/// "Slot definitions per tool"). Inputs without a slot get defaults: their
/// name and title words as keywords and no range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slot {
    pub input: &'static str,
    /// Lowercase words that name the input in a question ("oat", "rwy").
    pub keywords: &'static [&'static str],
    /// Plausible values in the input's unit; values outside do not fill it.
    pub range: (f64, f64),
    pub bare: Bare,
}

impl Slot {
    pub const fn new(input: &'static str, keywords: &'static [&'static str]) -> Slot {
        Slot {
            input,
            keywords,
            range: (f64::NEG_INFINITY, f64::INFINITY),
            bare: Bare::Never,
        }
    }
    pub const fn range(mut self, lo: f64, hi: f64) -> Slot {
        self.range = (lo, hi);
        self
    }
    pub const fn bare(mut self, bare: Bare) -> Slot {
        self.bare = bare;
        self
    }
}

pub type RunFn = fn(&mut Ctx) -> Result<Json, ToolError>;

/// Everything about a tool. Construct with `ToolDef { id: …, ..ToolDef::BLANK }`.
pub struct ToolDef {
    pub id: &'static str,
    pub version: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
    pub aliases: &'static [&'static str],
    pub keywords: &'static [&'static str],
    pub inputs: &'static [Field],
    pub outputs: &'static [Field],
    /// Error codes the tool may return beyond `INVALID_INPUT`/`UNIT_MISMATCH`/`INTERNAL`.
    pub errors: &'static [ErrorCode],
    /// Warning codes the tool may emit.
    pub warnings: &'static [&'static str],
    pub model: &'static str,
    pub accuracy: &'static str,
    pub references: &'static [Reference],
    pub examples: &'static [Example],
    pub primary_example: &'static str,
    pub assets: &'static [&'static str],
    pub visualization: &'static [Layer],
    /// For results that unfold over time or distance: the scene's timeline.
    pub timeline: Option<Timeline>,
    pub related: &'static [Related],
    pub stability: Stability,
    pub since: &'static str,
    /// For generated endpoints: the operations this endpoint chains.
    pub composed_of: &'static [&'static str],
    /// For generated endpoints: the operation whose run function and preset
    /// field definitions this endpoint reuses.
    pub parent: Option<&'static ToolDef>,
    /// For generated endpoints: fixed inputs, as (field, JSON literal).
    pub preset: &'static [(&'static str, &'static str)],
    /// The pair allow-list justification for a generated endpoint.
    pub justification: &'static str,
    /// Plain-language answer template (`x-sentence`).
    pub sentence: &'static str,
    /// Declared limits, as (name, value).
    pub limits: &'static [(&'static str, u64)],
    /// How free-text questions fill the inputs (natural-language prefill).
    pub slots: &'static [Slot],
    pub run: RunFn,
}

fn unimplemented_run(_: &mut Ctx) -> Result<Json, ToolError> {
    Err(ToolError::new(
        ErrorCode::Internal,
        "This tool has no implementation.",
    ))
}

impl ToolDef {
    pub const BLANK: ToolDef = ToolDef {
        id: "",
        version: "1.0.0",
        title: "",
        summary: "",
        aliases: &[],
        keywords: &[],
        inputs: &[],
        outputs: &[],
        errors: &[],
        warnings: &[],
        model: "",
        accuracy: "",
        references: &[],
        examples: &[],
        primary_example: "",
        assets: &[],
        visualization: &[],
        timeline: None,
        related: &[],
        stability: Stability::Experimental,
        since: "0.1.0",
        composed_of: &[],
        parent: None,
        preset: &[],
        justification: "",
        sentence: "",
        limits: &[],
        slots: &[],
        run: unimplemented_run,
    };

    pub fn domain(&self) -> &'static str {
        self.id.split('.').next().unwrap_or("")
    }

    pub fn group(&self) -> &'static str {
        self.id.split('.').nth(1).unwrap_or("")
    }

    /// True when outputs include quantities, so profiles and `outputUnits` apply.
    pub fn has_quantity_outputs(&self) -> bool {
        self.outputs
            .iter()
            .any(|f| matches!(f.kind, Kind::Quantity { .. }))
    }
}

/// Per-call options, passed as the reserved input key `options`.
#[derive(Clone, Debug, Default)]
pub struct Options {
    pub profile: Option<Profile>,
    pub output_units: Vec<(String, &'static Unit)>,
    pub format: NumberFormat,
}

/// A value with the unit it was given in. Convert exactly with [`Q::to`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Q {
    pub value: f64,
    pub unit: &'static Unit,
}

impl Q {
    pub fn to(self, unit: &Unit) -> f64 {
        units::convert(self.value, self.unit, unit)
    }

    /// The value in the quantity's registry base unit (SI; degrees for angles).
    pub fn base(self) -> f64 {
        units::to_base(self.value, self.unit)
    }

    pub fn to_json(self) -> Json {
        Json::obj([
            ("value", Json::Num(self.value)),
            ("unit", Json::str(self.unit.symbol)),
        ])
    }
}

/// The invocation context a tool's `run` function receives.
pub struct Ctx<'a> {
    pub def: &'static ToolDef,
    input: &'a Map<String, Value>,
    preset: Vec<(&'static str, Value)>,
    pub options: Options,
    pub warnings: Vec<Warning>,
    /// Overrides `meta.model` for this call (e.g. to name a custom ellipsoid).
    pub model: Option<String>,
    /// Reference data this call used, echoed in `meta.assets` (id and version).
    pub assets: Vec<AssetRef>,
    /// Operating context agents should relay (model epoch, validity window,
    /// uncertainty), echoed in `meta.context`.
    pub context: Vec<(&'static str, Json)>,
}

/// The magnitude bounds for quantity inputs, in registry base units.
pub const MAX_MAGNITUDE: f64 = 1e50;
pub const MIN_MAGNITUDE: f64 = 1e-50;

fn pointer(name: &str) -> String {
    format!("/{name}")
}

impl<'a> Ctx<'a> {
    fn field(&self, name: &str) -> &'static Field {
        self.def
            .inputs
            .iter()
            .chain(self.def.parent.into_iter().flat_map(|p| p.inputs))
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("{} reads undeclared input {name}", self.def.id))
    }

    /// True when this tool (or its parent) declares input `name`.
    pub fn declares(&self, name: &str) -> bool {
        self.def
            .inputs
            .iter()
            .chain(self.def.parent.into_iter().flat_map(|p| p.inputs))
            .any(|f| f.name == name)
    }

    /// The raw JSON for an input: a preset wins, then the caller's value.
    pub fn raw(&self, name: &str) -> Option<&Value> {
        self.preset
            .iter()
            .find(|(k, _)| *k == name)
            .map(|(_, v)| v)
            .or_else(|| self.input.get(name).filter(|v| !v.is_null()))
    }

    pub fn is_set(&self, name: &str) -> bool {
        self.raw(name).is_some()
    }

    fn missing(&self, name: &str) -> ToolError {
        let f = self.field(name);
        ToolError::invalid(&pointer(name), format!("{} is required.", f.title))
            .hint(format!("Example: {}", f.help))
    }

    /// Reads a quantity input, collecting any parse warnings.
    pub fn quantity(&mut self, name: &str) -> Result<Option<Q>, ToolError> {
        self.quantity_or(name, None)
    }

    /// Reads a quantity input; a bare number is in `default` (or the field's unit).
    pub fn quantity_or(
        &mut self,
        name: &str,
        default: Option<&'static Unit>,
    ) -> Result<Option<Q>, ToolError> {
        let f = self.field(name);
        let v = self.raw(name).cloned();
        self.parse_quantity(f, v.as_ref(), &pointer(name), default)
    }

    fn parse_quantity(
        &mut self,
        f: &'static Field,
        v: Option<&Value>,
        at: &str,
        default: Option<&'static Unit>,
    ) -> Result<Option<Q>, ToolError> {
        let Kind::Quantity { q, unit } = f.kind else {
            panic!("{} is not a quantity field", f.name)
        };
        let default = default.unwrap_or_else(|| {
            units::by_symbol(q, unit).expect("declared default unit is registered")
        });
        let Some(v) = v.filter(|v| !v.is_null()) else {
            return Ok(None);
        };
        let q = match v {
            Value::Number(n) => {
                let x = n.as_f64().filter(|x| x.is_finite()).ok_or_else(|| {
                    ToolError::invalid(at, format!("{} must be a finite number.", f.title))
                })?;
                Q {
                    value: x,
                    unit: default,
                }
            }
            Value::String(s) => {
                let t = parse::parse_tagged(s, q, default, self.options.format, at)?;
                self.warnings.extend(t.warnings);
                Q {
                    value: t.value,
                    unit: t.unit,
                }
            }
            _ => {
                return Err(ToolError::invalid(
                    at,
                    format!(
                        "{} must be a number or a number with a unit, like {}.",
                        f.title, f.help
                    ),
                ));
            }
        };
        // No physical input needs magnitudes past 1e50 (or below 1e-50) in SI
        // units; such values only overflow or underflow the arithmetic. A
        // nonzero value that underflows to zero in base units ("5e-324 ft")
        // counts as below the range, not as zero.
        let base = q.base().abs();
        if !base.is_finite() || (q.value != 0.0 && !(MIN_MAGNITUDE..=MAX_MAGNITUDE).contains(&base))
        {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                format!("{} is far outside any meaningful range.", f.title),
            )
            .at(at));
        }
        Ok(Some(q))
    }

    /// The rows of a list input, validated: an array within its size limits,
    /// each row an object with only declared keys and every required key.
    pub fn rows(&self, name: &str) -> Result<Vec<Map<String, Value>>, ToolError> {
        let f = self.field(name);
        let Kind::List { items, min, max } = f.kind else {
            panic!("{name} is not a list field")
        };
        let at = pointer(name);
        let arr = match self.raw(name) {
            None => return Ok(Vec::new()),
            Some(Value::Array(a)) => a,
            Some(_) => {
                return Err(ToolError::invalid(
                    &at,
                    format!("{} must be a list.", f.title),
                ));
            }
        };
        if arr.len() > max {
            return Err(ToolError::new(
                ErrorCode::LimitExceeded,
                format!("{} has {} rows; the limit is {max}.", f.title, arr.len()),
            )
            .at(&at));
        }
        if arr.len() < min {
            return Err(ToolError::invalid(
                &at,
                format!("{} needs at least {min} rows.", f.title),
            ));
        }
        let mut out = Vec::with_capacity(arr.len());
        for (i, row) in arr.iter().enumerate() {
            let row_at = format!("{at}/{i}");
            let Value::Object(m) = row else {
                return Err(ToolError::invalid(
                    &row_at,
                    format!("Each row of {} must be an object.", f.title),
                ));
            };
            if let Some(k) = m
                .keys()
                .find(|k| !items.iter().any(|it| it.name == k.as_str()))
            {
                let names: Vec<&str> = items.iter().map(|it| it.name).collect();
                return Err(ToolError::invalid(
                    &format!("{row_at}/{k}"),
                    format!("{k} is not a column of {}.", f.title),
                )
                .hint(format!("Columns: {}", names.join(", "))));
            }
            if let Some(it) = items
                .iter()
                .find(|it| it.required && m.get(it.name).is_none_or(Value::is_null))
            {
                return Err(ToolError::invalid(
                    &format!("{row_at}/{}", it.name),
                    format!("Row {} needs {}.", i + 1, it.title),
                ));
            }
            out.push(m.clone());
        }
        Ok(out)
    }

    fn item(&self, list: &str, name: &str) -> &'static Field {
        let Kind::List { items, .. } = self.field(list).kind else {
            panic!("{list} is not a list field")
        };
        items
            .iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("{list} has no column {name}"))
    }

    /// Reads a quantity from row `i` of list `list`.
    pub fn row_quantity(
        &mut self,
        list: &str,
        i: usize,
        row: &Map<String, Value>,
        name: &str,
    ) -> Result<Option<Q>, ToolError> {
        let f = self.item(list, name);
        self.parse_quantity(f, row.get(name), &format!("/{list}/{i}/{name}"), None)
    }

    /// Reads text from row `i` of list `list`.
    pub fn row_text(
        &self,
        list: &str,
        i: usize,
        row: &Map<String, Value>,
        name: &str,
    ) -> Result<Option<String>, ToolError> {
        let f = self.item(list, name);
        let at = format!("/{list}/{i}/{name}");
        match row.get(name).filter(|v| !v.is_null()) {
            None => Ok(None),
            Some(Value::String(s)) => Ok(Some(s.trim().to_owned())),
            Some(Value::Number(n)) => Ok(Some(n.to_string())),
            Some(_) => Err(ToolError::invalid(
                &at,
                format!("{} must be text.", f.title),
            )),
        }
    }

    /// An asset file or tile the host supplied, recorded in `meta.assets`.
    /// Returns `ASSET_UNAVAILABLE` naming `id`, `version`, and `file` when the
    /// host has not supplied it yet. The tool must declare `id` in `assets`.
    pub fn asset(
        &mut self,
        id: &str,
        version: &str,
        file: &str,
    ) -> Result<std::rc::Rc<[u8]>, ToolError> {
        assert!(
            self.def.assets.contains(&id),
            "{} reads undeclared asset {id}",
            self.def.id
        );
        if !self
            .assets
            .iter()
            .any(|a| a.id == id && a.version == version)
        {
            self.assets.push(AssetRef {
                id: id.to_owned(),
                version: version.to_owned(),
            });
        }
        crate::assets::get(&crate::assets::key(id, version, file))
            .ok_or_else(|| ToolError::asset_unavailable(id, version, file))
    }

    pub fn req_quantity(&mut self, name: &str) -> Result<Q, ToolError> {
        self.quantity(name)?.ok_or_else(|| self.missing(name))
    }

    /// Reads a unit-symbol input.
    pub fn unit(&mut self, name: &str) -> Result<Option<&'static Unit>, ToolError> {
        let f = self.field(name);
        let Kind::Unit(q) = f.kind else {
            panic!("{name} is not a unit field")
        };
        let at = pointer(name);
        match self.raw(name) {
            None => Ok(None),
            Some(Value::String(s)) => {
                let (u, w) = parse::resolve_unit(q, s.trim(), &at)?;
                self.warnings.extend(w);
                Ok(Some(u))
            }
            Some(_) => Err(ToolError::invalid(
                &at,
                format!("{} must be a unit symbol.", f.title),
            )),
        }
    }

    pub fn req_unit(&mut self, name: &str) -> Result<&'static Unit, ToolError> {
        self.unit(name)?.ok_or_else(|| self.missing(name))
    }

    pub fn choice(&self, name: &str) -> Result<Option<&'static str>, ToolError> {
        let f = self.field(name);
        let Kind::Choice(options) = f.kind else {
            panic!("{name} is not a choice field")
        };
        match self.raw(name) {
            None => Ok(None),
            Some(Value::String(s)) => options
                .iter()
                .copied()
                .find(|o| o == s)
                .map(Some)
                .ok_or_else(|| {
                    ToolError::invalid(
                        &pointer(name),
                        format!("{} must be one of: {}.", f.title, options.join(", ")),
                    )
                }),
            Some(_) => Err(ToolError::invalid(
                &pointer(name),
                format!("{} must be a string.", f.title),
            )),
        }
    }

    pub fn number(&self, name: &str) -> Result<Option<f64>, ToolError> {
        let f = self.field(name);
        let Kind::Number { min, max } = f.kind else {
            panic!("{name} is not a number field")
        };
        let at = pointer(name);
        let x = match self.raw(name) {
            None => return Ok(None),
            Some(Value::Number(n)) => n.as_f64().unwrap_or(f64::NAN),
            Some(Value::String(s)) => parse::parse_number(s, self.options.format, &at)?,
            Some(_) => {
                return Err(ToolError::invalid(
                    &at,
                    format!("{} must be a number.", f.title),
                ));
            }
        };
        if !(min..=max).contains(&x) {
            let show = |v: f64| crate::num::format_f64(v).unwrap_or_default();
            return Err(ToolError::invalid(
                &at,
                format!(
                    "{} must be between {} and {}.",
                    f.title,
                    show(min),
                    show(max)
                ),
            ));
        }
        Ok(Some(x))
    }

    /// Reads a text input, trimmed, within its declared length.
    pub fn text(&self, name: &str) -> Result<Option<String>, ToolError> {
        let f = self.field(name);
        let Kind::Text { max_len } = f.kind else {
            panic!("{name} is not a text field")
        };
        match self.raw(name) {
            None => Ok(None),
            Some(Value::String(s)) if s.trim().len() <= max_len => Ok(Some(s.trim().to_owned())),
            Some(Value::String(_)) => Err(ToolError::invalid(
                &pointer(name),
                format!("{} is longer than {max_len} characters.", f.title),
            )),
            Some(_) => Err(ToolError::invalid(
                &pointer(name),
                format!("{} must be text.", f.title),
            )),
        }
    }

    /// The unit an output quantity field is reported in: `options.outputUnits`,
    /// then the unit profile, then the field's declared unit.
    pub fn output_unit(&self, name: &str) -> &'static Unit {
        let f = self
            .def
            .outputs
            .iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("{} writes undeclared output {name}", self.def.id));
        let Kind::Quantity { q, unit } = f.kind else {
            panic!("{name} is not a quantity output")
        };
        if let Some((_, u)) = self.options.output_units.iter().find(|(k, _)| k == name) {
            return u;
        }
        match self.options.profile {
            Some(p) => p.unit_for(q),
            None => units::by_symbol(q, unit).expect("declared output unit is registered"),
        }
    }

    /// Converts a value to the output unit for `name` and returns `{value, unit}`,
    /// adding `LEGACY_UNIT` when that unit is the US survey foot.
    pub fn out(&mut self, name: &str, value: Q) -> Json {
        let unit = self.output_unit(name);
        self.emit(name, value, unit)
    }

    /// Returns `{value, unit}` in an explicit unit (e.g. the `to` of a conversion).
    pub fn emit(&mut self, name: &str, value: Q, unit: &'static Unit) -> Json {
        if unit.symbol.starts_with("ftUS") {
            self.warnings.push(
                Warning::new(
                    "LEGACY_UNIT",
                    "The US survey foot was deprecated by NIST and NOAA on January 1, 2023. Use it only for legacy data.",
                )
                .at(&pointer(name)),
            );
        }
        Q {
            value: value.to(unit),
            unit,
        }
        .to_json()
    }
}

/// A module's tools, in manifest order.
pub struct Registry {
    pub module: &'static str,
    pub tools: &'static [&'static ToolDef],
}

/// Which module serves a domain (`units` lives in `base`).
pub fn module_for_domain(domain: &str) -> Option<&'static str> {
    Some(match domain {
        "units" => "base",
        "geodesy" => "geodesy",
        "navigation" => "navigation",
        "geometry" => "geometry",
        "aviation" => "aviation",
        "drone" => "drone",
        "survey" => "survey",
        "indexing" => "indexing",
        "raster" => "raster",
        "time" => "time",
        _ => return None,
    })
}

/// Default and maximum batch size.
pub const MAX_BATCH: usize = 10_000;

impl Registry {
    pub fn find(&self, id: &str) -> Option<&'static ToolDef> {
        self.tools.iter().copied().find(|t| t.id == id)
    }

    /// `invoke(toolId, inputJson) -> resultJson`.
    pub fn invoke(&self, id: &str, input_json: &str) -> String {
        match self.find(id) {
            None => envelope::failure(&self.unknown(id)),
            Some(def) => match serde_json::from_str::<Value>(input_json) {
                Ok(v) => self.run(def, &v),
                Err(e) => envelope::failure(&ToolError::new(
                    ErrorCode::InvalidInput,
                    format!("The input is not valid JSON: {e}."),
                )),
            },
        }
    }

    /// `invokeBatch(toolId, inputsJson) -> resultsJson`: one envelope per record,
    /// in input order. A failing record does not fail the batch.
    pub fn invoke_batch(&self, id: &str, inputs_json: &str) -> String {
        let Some(def) = self.find(id) else {
            return envelope::failure(&self.unknown(id));
        };
        let records = match serde_json::from_str::<Value>(inputs_json) {
            Ok(Value::Array(a)) => a,
            Ok(_) => {
                return envelope::failure(&ToolError::new(
                    ErrorCode::InvalidInput,
                    "A batch must be a JSON array of input objects.",
                ));
            }
            Err(e) => {
                return envelope::failure(&ToolError::new(
                    ErrorCode::InvalidInput,
                    format!("The batch is not valid JSON: {e}."),
                ));
            }
        };
        let max = limit(def, "batchRows").map_or(MAX_BATCH, |n| n as usize);
        if records.len() > max {
            return envelope::failure(
                &ToolError::new(
                    ErrorCode::LimitExceeded,
                    format!(
                        "The batch has {} records; the limit is {max}.",
                        records.len()
                    ),
                )
                .hint("Split the batch into smaller parts."),
            );
        }
        let mut out = String::from("[");
        for (i, r) in records.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&self.run(def, r));
        }
        out.push(']');
        out
    }

    /// `manifest() -> manifestJson`: every tool in this module.
    pub fn manifest(&self) -> String {
        Json::Arr(
            self.tools
                .iter()
                .map(|t| crate::manifest::manifest(t))
                .collect(),
        )
        .to_string()
        .expect("manifests contain only finite numbers")
    }

    fn unknown(&self, id: &str) -> ToolError {
        let domain = id.split('.').next().unwrap_or("");
        let err = ToolError::new(
            ErrorCode::Unsupported,
            format!("There is no tool with id \"{id}\" here."),
        );
        match module_for_domain(domain) {
            Some(m) if m != self.module => err.hint(format!(
                "{domain} tools live in the {m} module, which is not loaded. Load it and retry."
            )),
            _ => err.hint("Search the catalog for the right id."),
        }
    }

    fn run(&self, def: &'static ToolDef, input: &Value) -> String {
        match execute(def, input) {
            Ok((result, summary, display, warnings, model, assets, context)) => {
                let meta = Meta {
                    tool: def.id.to_owned(),
                    tool_version: def.version.to_owned(),
                    assets,
                    model: model.unwrap_or_else(|| def.model.to_owned()),
                    accuracy: def.accuracy.to_owned(),
                    warnings,
                    context,
                    notice: operational_notice(def),
                };
                envelope::success(result, summary.as_deref(), display, &meta)
            }
            Err(e) => envelope::failure(&e),
        }
    }
}

fn limit(def: &ToolDef, name: &str) -> Option<u64> {
    def.limits.iter().find(|(k, _)| *k == name).map(|(_, v)| *v)
}

/// Result, rendered sentence, display strings per output, and warnings.
type Executed = (
    Json,
    Option<String>,
    Json,
    Vec<Warning>,
    Option<String>,
    Vec<AssetRef>,
    Vec<(&'static str, Json)>,
);

fn execute(def: &'static ToolDef, input: &Value) -> Result<Executed, ToolError> {
    let Value::Object(map) = input else {
        return Err(ToolError::new(
            ErrorCode::InvalidInput,
            "The input must be a JSON object.",
        ));
    };
    for key in map.keys() {
        if key != "options" && !def.inputs.iter().any(|f| f.name == key) {
            let names: Vec<&str> = def.inputs.iter().map(|f| f.name).collect();
            return Err(ToolError::invalid(
                &pointer(key),
                format!("{key} is not an input of this tool."),
            )
            .hint(format!("Inputs: {}", names.join(", "))));
        }
    }
    let options = parse_options(def, map.get("options"))?;
    let preset = def
        .preset
        .iter()
        .map(|(k, v)| (*k, serde_json::from_str(v).expect("preset is valid JSON")))
        .collect();
    let mut ctx = Ctx {
        def,
        input: map,
        preset,
        options,
        warnings: Vec::new(),
        model: None,
        assets: Vec::new(),
        context: Vec::new(),
    };
    for f in def.inputs {
        if f.required && !ctx.is_set(f.name) {
            return Err(ctx.missing(f.name));
        }
    }
    let mut result = (def.run)(&mut ctx)?;
    // Results list outputs in schema order (the first is the primary result).
    if let Json::Obj(pairs) = &mut result {
        let rank = |k: &str| {
            def.outputs
                .iter()
                .position(|f| f.name == k)
                .unwrap_or(usize::MAX)
        };
        pairs.sort_by_key(|(k, _)| rank(k));
    }
    if def.stability == Stability::Experimental {
        ctx.warnings.push(Warning::new(
            "EXPERIMENTAL_TOOL",
            "This tool is experimental: it has not yet met the stable verification bar.",
        ));
    }
    let (summary, display) = render_summary(&mut ctx, &result);
    Ok((
        result,
        Some(summary),
        display,
        ctx.warnings,
        ctx.model,
        ctx.assets,
        ctx.context,
    ))
}

/// The notice aviation, drone, navigation, and magnetic results carry in
/// `meta.notice`, the same words the website shows on those tools.
pub const NOT_FOR_NAVIGATION: &str = "Planning and education aid. Not for primary navigation.";

fn operational_notice(def: &ToolDef) -> Option<&'static str> {
    let operational = matches!(def.domain(), "aviation" | "drone" | "navigation")
        || def.id.starts_with("geodesy.magnetic.");
    operational.then_some(NOT_FOR_NAVIGATION)
}

/// Display precision for input values echoed in sentences.
const INPUT_PRECISION: Precision = Precision::Significant(12);

struct SentenceScope {
    values: Vec<(&'static str, Val)>,
    warnings: Vec<&'static str>,
    format: NumberFormat,
}

impl Scope for SentenceScope {
    fn get(&self, name: &str) -> Option<Val> {
        self.values
            .iter()
            .find(|(k, _)| *k == name)
            .map(|(_, v)| v.clone())
    }
    fn has_warning(&self, code: &str) -> bool {
        self.warnings.contains(&code)
    }
    fn format(&self) -> NumberFormat {
        self.format
    }
}

/// Renders the tool's `x-sentence` from its outputs (first) and inputs, and
/// each output as display text (rounded to its display precision, with its unit).
fn render_summary(ctx: &mut Ctx, result: &Json) -> (String, Json) {
    let def = ctx.def;
    let get = |name: &str| match result {
        Json::Obj(pairs) => pairs.iter().find(|(k, _)| k == name).map(|(_, v)| v),
        _ => None,
    };
    let mut values: Vec<(&'static str, Val)> = Vec::new();
    for f in def.outputs {
        let precision = f.precision.unwrap_or(INPUT_PRECISION);
        let val = match (f.kind, get(f.name)) {
            (Kind::Quantity { .. } | Kind::AnyQuantity, Some(Json::Obj(q))) => {
                let num = q.iter().find(|(k, _)| k == "value").map(|(_, v)| v);
                let sym = q.iter().find(|(k, _)| k == "unit").map(|(_, v)| v);
                match (num, sym, f.kind) {
                    (Some(Json::Num(x)), Some(Json::Str(u)), Kind::Quantity { q, .. }) => {
                        Some(Val::num(*x, units::by_symbol(q, u), precision))
                    }
                    (Some(Json::Num(x)), Some(Json::Str(u)), _) => {
                        Some(Val::num(*x, units::any_by_symbol(u), precision))
                    }
                    _ => None,
                }
            }
            (Kind::Number { .. }, Some(Json::Num(x))) => Some(Val::num(*x, None, precision)),
            (Kind::Text { .. }, Some(Json::Str(t))) => Some(Val::text(t.clone())),
            _ => None,
        };
        if let Some(v) = val {
            values.push((f.name, v));
        }
    }
    let n = ctx.warnings.len();
    let inputs: Vec<&'static Field> = def
        .inputs
        .iter()
        .chain(def.parent.into_iter().flat_map(|p| p.inputs))
        .collect();
    for f in inputs {
        if values.iter().any(|(k, _)| *k == f.name) {
            continue;
        }
        let v = match f.kind {
            Kind::Quantity { .. } => ctx
                .quantity(f.name)
                .ok()
                .flatten()
                .map(|q| Val::num(q.value, Some(q.unit), INPUT_PRECISION)),
            Kind::Number { .. } => ctx
                .number(f.name)
                .ok()
                .flatten()
                .map(|x| Val::num(x, None, INPUT_PRECISION)),
            Kind::Text { .. } => ctx.text(f.name).ok().flatten().map(Val::text),
            _ => None,
        };
        if let Some(v) = v {
            values.push((f.name, v));
        }
    }
    ctx.warnings.truncate(n);
    let display = Json::Obj(
        values
            .iter()
            .filter(|(k, _)| def.outputs.iter().any(|f| f.name == *k))
            .map(|(k, v)| {
                let text = match (&v.text, v.unit) {
                    (Some(t), _) => t.clone(),
                    (None, Some(u)) => {
                        crate::display::quantity(v.value, u.symbol, v.precision, ctx.options.format)
                    }
                    (None, None) => {
                        crate::display::number(v.value, v.precision, ctx.options.format)
                    }
                };
                ((*k).to_owned(), Json::str(text))
            })
            .collect(),
    );
    let scope = SentenceScope {
        values,
        warnings: ctx.warnings.iter().map(|w| w.code).collect(),
        format: ctx.options.format,
    };
    (template::render(def.sentence, &scope), display)
}

fn parse_options(def: &ToolDef, v: Option<&Value>) -> Result<Options, ToolError> {
    let mut o = Options::default();
    let Some(v) = v.filter(|v| !v.is_null()) else {
        return Ok(o);
    };
    let Value::Object(m) = v else {
        return Err(ToolError::invalid("/options", "options must be an object."));
    };
    for (k, v) in m {
        match (k.as_str(), v) {
            ("profile", Value::String(p)) => {
                o.profile = Some(Profile::from_id(p).ok_or_else(|| {
                    ToolError::invalid("/options/profile", format!("Unknown unit profile {p}."))
                        .hint(format!("Profiles: {}", Profile::IDS.join(", ")))
                })?);
            }
            ("numberFormat", Value::String(f)) => {
                o.format = match f.as_str() {
                    "decimal-point" => NumberFormat::DecimalPoint,
                    "decimal-comma" => NumberFormat::DecimalComma,
                    _ => {
                        return Err(ToolError::invalid(
                            "/options/numberFormat",
                            "numberFormat must be decimal-point or decimal-comma.",
                        ));
                    }
                };
            }
            ("outputUnits", Value::Object(units)) => {
                for (field, u) in units {
                    let at = format!("/options/outputUnits/{field}");
                    let Some(Kind::Quantity { q, .. }) =
                        def.outputs.iter().find(|f| f.name == field).map(|f| f.kind)
                    else {
                        return Err(ToolError::invalid(
                            &at,
                            format!("{field} is not a quantity output of this tool."),
                        ));
                    };
                    let Value::String(s) = u else {
                        return Err(ToolError::invalid(
                            &at,
                            "An output unit must be a unit symbol.",
                        ));
                    };
                    let (unit, _) = parse::resolve_unit(q, s.trim(), &at)?;
                    o.output_units.push((field.clone(), unit));
                }
            }
            _ => {
                return Err(ToolError::invalid(
                    &format!("/options/{k}"),
                    format!("{k} is not a valid option."),
                )
                .hint("Options: profile, outputUnits, numberFormat"));
            }
        }
    }
    Ok(o)
}
