//! Land descriptions (survey/land-descriptions spec): legacy land units, the
//! metes-and-bounds deed parser, deed plotting with closure and area (no silent
//! adjustment), the PLSS aliquot parser, and basis-of-bearing rotation. The
//! tools do the arithmetic; the professional keeps the interpretation.

use crate::direction;
use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Reference, Related, ToolDef};
use libm::{atan2, cos, hypot, sin};

pub const NOTICE: &str = "Parsing and arithmetic aid. Boundary determination requires a licensed surveyor and the governing records.";

const BLM_MANUAL: Reference = Reference {
    title: "Manual of Surveying Instructions for the Survey of the Public Lands of the United States",
    issuer: "Bureau of Land Management",
    year: 2009,
    edition: "2009 Manual",
    locator: "Chapter 1 (units: chain of 66 feet, link of 0.66 feet) and Chapter 3 (subdivision of sections, aliquot parts)",
    url: "https://www.blm.gov/sites/default/files/documents/files/Manual_of_Surveying_Instructions_2009.pdf",
};
const BROWN: Reference = Reference {
    title: "Brown's Boundary Control and Legal Principles",
    issuer: "Robillard, W. G., and Wilson, D. A., Wiley",
    year: 2014,
    edition: "7th edition",
    locator: "Chapter 5 (metes-and-bounds descriptions, calls, and the order of importance of conflicting elements)",
    url: "https://www.wiley.com/en-us/Brown%27s+Boundary+Control+and+Legal+Principles%2C+7th+Edition-p-9781118431429",
};
const GHILANI: Reference = Reference {
    title: "Elementary Surveying: An Introduction to Geomatics",
    issuer: "Ghilani, C. D., and Wolf, P. R., Pearson",
    year: 2018,
    edition: "15th edition",
    locator: "Chapters 10 (traverse computations) and 12 (area by coordinates)",
    url: "https://www.pearson.com/en-us/subject-catalog/p/elementary-surveying-an-introduction-to-geomatics/P200000003148",
};
const VARA_SOURCES: Reference = Reference {
    title: "Units of Measure: land measurements and definitions (vara, arpent)",
    issuer: "Texas General Land Office definitions as compiled by surveying references (Texas vara of 33⅓ inches by the 1919 statute; Florida and California vara of 33.372 inches; Louisiana arpent of 191.994 feet; Missouri arpent of 0.8507 acre)",
    year: 2021,
    edition: "Compiled definitions, checked 2026-09-18",
    locator: "Vara and arpent entries",
    url: "https://www.landsource.com/resources/units/",
};

fn ftus(v: f64) -> gp_base::tool::Q {
    gp_base::tool::Q {
        value: v,
        unit: gp_base::units::by_symbol(gp_base::units::Quantity::Length, "ftUS").expect("ftUS"),
    }
}

/// A legacy linear unit: name, meters per unit, and its definition.
struct LinUnit {
    names: &'static [&'static str],
    label: &'static str,
    /// US survey feet per unit, or None when it needs a jurisdiction (vara, arpent).
    ftus: Option<f64>,
    basis: &'static str,
}

const LINEAR: &[LinUnit] = &[
    LinUnit {
        names: &["chains", "chain", "ch", "chs"],
        label: "chain",
        ftus: Some(66.0),
        basis: "1 chain = 66 US survey feet (Gunter's chain)",
    },
    LinUnit {
        names: &["links", "link", "lk", "lks", "li"],
        label: "link",
        ftus: Some(0.66),
        basis: "1 link = 0.66 US survey feet (1/100 chain)",
    },
    LinUnit {
        names: &[
            "rods", "rod", "rd", "rds", "poles", "pole", "perches", "perch",
        ],
        label: "rod",
        ftus: Some(16.5),
        basis: "1 rod (pole, perch) = 16.5 US survey feet (1/4 chain)",
    },
    LinUnit {
        names: &["furlongs", "furlong", "fur"],
        label: "furlong",
        ftus: Some(660.0),
        basis: "1 furlong = 660 US survey feet (10 chains)",
    },
    LinUnit {
        names: &["feet", "foot", "ft", "'"],
        label: "foot",
        ftus: Some(1.0),
        basis: "feet taken as US survey feet (the legacy-deed basis; say so if the deed used international feet)",
    },
    LinUnit {
        names: &["inches", "inch", "in", "\""],
        label: "inch",
        ftus: Some(1.0 / 12.0),
        basis: "1 inch = 1/12 US survey foot",
    },
    LinUnit {
        names: &["meters", "meter", "metres", "metre", "m"],
        label: "meter",
        ftus: Some(3937.0 / 1200.0),
        basis: "SI meter",
    },
    LinUnit {
        names: &["varas", "vara", "vrs", "vr"],
        label: "vara",
        ftus: None,
        basis: "the vara depends on the jurisdiction",
    },
    LinUnit {
        names: &["arpents", "arpent", "arp"],
        label: "arpent",
        ftus: None,
        basis: "the arpent depends on the jurisdiction",
    },
];

/// Jurisdictional varas and arpents (US survey feet per unit, definition).
fn jurisdictional(label: &str, j: &str) -> Option<(f64, &'static str)> {
    let inch = 1.0 / 12.0;
    Some(match (label, j) {
        ("vara", "texas") => (
            100.0 / 3.0 * inch,
            "Texas vara = 33⅓ inches (statute of 1919)",
        ),
        ("vara", "california") => (
            33.372 * inch,
            "California vara = 33.372 inches (Los Angeles County records; older records range from 32.953 inches)",
        ),
        ("vara", "florida") => (33.372 * inch, "Florida vara = 33.372 inches"),
        ("arpent", "louisiana") => (191.994, "Louisiana arpent (linear) = 191.994 feet"),
        ("arpent", "missouri") => (
            192.5,
            "Missouri arpent (linear) = 192.5 feet, from 0.8507 acre square",
        ),
        _ => return None,
    })
}

pub const JURISDICTIONS: &[&str] = &["texas", "california", "florida", "louisiana", "missouri"];

fn lin_unit(word: &str) -> Option<&'static LinUnit> {
    let w = word.trim_end_matches(['.', ',', ';']).to_ascii_lowercase();
    LINEAR.iter().find(|u| u.names.contains(&w.as_str()))
}

/// Parses a length like "12 chains 34 links", "1,000 varas", or "814.44 feet"
/// to US survey feet, with the bases used. `j` resolves varas and arpents.
pub fn parse_length(s: &str, j: Option<&str>) -> Result<(f64, Vec<&'static str>), String> {
    let spaced = s.replace('\'', " ' ").replace('"', " \" ");
    let toks: Vec<&str> = spaced.split_whitespace().collect();
    if toks.is_empty() {
        return Err("the length is empty".into());
    }
    let (mut total, mut bases, mut i) = (0.0, Vec::new(), 0);
    while i < toks.len() {
        let t = toks[i].replace(',', "");
        // A number glued to its unit: "12ch", "100ft".
        let split = t.find(|c: char| c.is_ascii_alphabetic()).filter(|&k| k > 0);
        let (num, unit_tok) = match split {
            Some(k) => (t[..k].to_owned(), Some(t[k..].to_owned())),
            None => (t.clone(), toks.get(i + 1).map(|u| u.to_string())),
        };
        let v: f64 = num
            .parse()
            .map_err(|_| format!("\"{}\" is not a number", toks[i]))?;
        let unit_tok = unit_tok.ok_or_else(|| format!("{v} needs a unit, like chains or feet"))?;
        let u = lin_unit(&unit_tok)
            .ok_or_else(|| format!("\"{unit_tok}\" is not a length unit this tool knows"))?;
        let (m, basis) = match u.ftus {
            Some(m) => (m, u.basis),
            None => match j {
                None => {
                    return Err(format!(
                        "A {} needs a jurisdiction: choose texas (vara 33⅓ in), california or florida (vara 33.372 in), louisiana (arpent 191.994 ft), or missouri (arpent 192.5 ft).",
                        u.label
                    ));
                }
                Some(j) => jurisdictional(u.label, j).ok_or_else(|| {
                    format!("There is no {} defined for {j} in this tool.", u.label)
                })?,
            },
        };
        total += v * m;
        if !bases.contains(&basis) {
            bases.push(basis);
        }
        i += if split.is_some() { 1 } else { 2 };
    }
    Ok((total, bases))
}

// ---------------------------------------------------------------- legacy units

pub static LEGACY_UNITS: ToolDef = ToolDef {
    id: "survey.land.legacy-units",
    title: "Legacy land units",
    summary: "Converts chains and links, rods, furlongs, varas, and arpents from old deeds and plats to US survey feet, feet, and meters, with the definition used; varas and arpents need a jurisdiction.",
    aliases: &[
        "chains to feet",
        "vara to feet",
        "arpent to feet",
        "rods to feet",
        "chains and links calculator",
    ],
    keywords: &[
        "chain",
        "link",
        "rod",
        "pole",
        "perch",
        "furlong",
        "vara",
        "arpent",
        "Gunter",
        "deed",
        "legacy units",
    ],
    inputs: &[
        Field::new(
            "length",
            "Length",
            "Like 12 chains 34 links, or 1,000 varas",
            Kind::Text { max_len: 120 },
        )
        .required()
        .core(),
        Field::new(
            "jurisdiction",
            "Jurisdiction",
            "For varas and arpents: texas, california, florida, louisiana, or missouri",
            Kind::Choice(JURISDICTIONS),
        )
        .core(),
    ],
    outputs: &[
        Field::new(
            "us_survey_feet",
            "US survey feet",
            "The legacy basis",
            Kind::Quantity {
                q: gp_base::units::Quantity::Length,
                unit: "ftUS",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "feet",
            "International feet",
            "0.3048 m",
            Kind::Quantity {
                q: gp_base::units::Quantity::Length,
                unit: "ft",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "meters",
            "Meters",
            "SI",
            Kind::Quantity {
                q: gp_base::units::Quantity::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "basis",
            "Basis",
            "The definitions used",
            Kind::Text { max_len: 400 },
        ),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["LEGACY_UNIT", "EXPERIMENTAL_TOOL"],
    model: "Gunter's chain of 66 US survey feet (100 links), rod of 16.5 ft, furlong of 660 ft; jurisdictional varas and arpents",
    accuracy: "Exact for the stated definitions. Old records sometimes used local variants; check the survey's own statement of units.",
    references: &[BLM_MANUAL, VARA_SOURCES],
    examples: &[Example {
        id: "primary",
        title: "12 chains 34 links",
        input: r#"{"length":"12 chains 34 links"}"#,
        source: "add-practitioner-essentials legacy-units scenario: 814.44 US survey feet (1 chain = 66 ftUS; 1 link = 0.66 ftUS)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[Related {
        id: "survey.land.deed-parse",
        reason: "next",
    }],
    sentence: "That is {us_survey_feet}, or {meters}.",
    limits: &[("batchRows", 10_000)],
    run: run_units,
    ..ToolDef::BLANK
};

fn run_units(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let s = ctx.text("length")?.unwrap_or_default();
    let j = ctx.choice("jurisdiction")?;
    let (f, bases) = parse_length(&s, j).map_err(|e| {
        ToolError::invalid(
            if e.contains("jurisdiction") {
                "/jurisdiction"
            } else {
                "/length"
            },
            e,
        )
    })?;
    let q = ftus(f);
    Ok(Json::obj(vec![
        ("us_survey_feet", ctx.out("us_survey_feet", q)),
        ("feet", ctx.out("feet", q)),
        ("meters", ctx.out("meters", q)),
        ("basis", Json::str(bases.join("; "))),
    ]))
}

// ---------------------------------------------------------------- deed parsing

/// One parsed call. Distances are US survey feet.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Call {
    pub kind: &'static str,
    pub direction: Option<f64>,
    pub distance: Option<f64>,
    pub radius: Option<f64>,
    pub arc_length: Option<f64>,
    pub delta: Option<f64>,
    pub chord_direction: Option<f64>,
    pub chord: Option<f64>,
    pub turn: Option<&'static str>,
    pub tangent: Option<bool>,
    pub monument: Option<String>,
    pub source: String,
    pub flags: Vec<&'static str>,
}

/// A word and its byte span in the original text.
fn words(s: &str) -> Vec<(String, usize, usize)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in s.char_indices() {
        if c.is_whitespace() || c == ',' || c == ';' {
            if let Some(b) = start.take() {
                out.push((s[b..i].to_owned(), b, i));
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if let Some(b) = start {
        out.push((s[b..].to_owned(), b, s.len()));
    }
    out
}

fn clean(w: &str) -> String {
    w.trim_matches(|c: char| c == '.' || c == '(' || c == ')' || c == ':')
        .to_ascii_lowercase()
}

/// Angle words to DMS marks: "45 degrees 30 minutes 15 seconds" → 45°30'15".
fn angle_text(ws: &[String]) -> String {
    let mut t = String::new();
    for w in ws {
        let c = clean(w);
        let mark = match c.as_str() {
            "degrees" | "degree" | "deg" | "d" => Some("°"),
            "minutes" | "minute" | "min" | "mins" | "m" => Some("'"),
            "seconds" | "second" | "sec" | "secs" | "s" => Some("\""),
            "and" => Some(""),
            _ => None,
        };
        match mark {
            Some(m) => t.push_str(m),
            None => {
                t.push(' ');
                t.push_str(w.trim_end_matches('.'));
            }
        }
    }
    t.trim().to_owned()
}

/// Finds the first bearing in `ws[from..]`: quadrant ("N 45°30'15" E", "North 45
/// degrees East", "N45-30-15E") or "due north". Returns (azimuth, first, last word index).
fn find_bearing(ws: &[(String, usize, usize)], from: usize) -> Option<(f64, usize, usize)> {
    let ns = |c: &str| matches!(c, "n" | "north" | "s" | "south");
    let ew = |c: &str| matches!(c, "e" | "east" | "w" | "west");
    for i in from..ws.len() {
        let c = clean(&ws[i].0);
        if c == "due"
            && let Some(n) = ws.get(i + 1)
        {
            let az = match clean(&n.0).as_str() {
                "north" => 0.0,
                "east" => 90.0,
                "south" => 180.0,
                "west" => 270.0,
                _ => continue,
            };
            return Some((az, i, i + 1));
        }
        // Compact: N45-30-15E, N45°30'15"E
        let raw = ws[i].0.trim_end_matches(['.', ',']);
        let up = raw.to_ascii_uppercase();
        if up.len() > 2
            && up.starts_with(['N', 'S'])
            && up.ends_with(['E', 'W'])
            && up[1..up.len() - 1].starts_with(|c: char| c.is_ascii_digit())
            && let Ok(az) = direction::parse(raw)
        {
            return Some((az, i, i));
        }
        if ns(&c) {
            for j in i + 1..(i + 10).min(ws.len()) {
                let cj = clean(&ws[j].0);
                if ew(&cj) {
                    let a = ws[i + 1..j].iter().map(|w| w.0.clone()).collect::<Vec<_>>();
                    if a.is_empty() {
                        break;
                    }
                    let txt = format!(
                        "{} {} {}",
                        c[..1].to_ascii_uppercase(),
                        angle_text(&a),
                        cj[..1].to_ascii_uppercase()
                    );
                    if let Ok(az) = direction::parse(&txt) {
                        return Some((az, i, j));
                    }
                    break;
                }
            }
        }
    }
    None
}

fn is_number(w: &str) -> bool {
    let t = w.replace(',', "");
    let t = t.trim_end_matches(|c: char| c.is_ascii_alphabetic() || c == '\'' || c == '"');
    !t.is_empty() && t.parse::<f64>().is_ok()
}

/// Finds the first length (one or more number-unit pairs) in `ws[from..]`.
fn find_length(
    ws: &[(String, usize, usize)],
    from: usize,
    j: Option<&str>,
) -> Result<Option<(f64, usize, usize)>, String> {
    let mut i = from;
    while i < ws.len() {
        if is_number(&ws[i].0) {
            // Grow the phrase over consecutive number-unit pairs.
            let mut k = i;
            let mut last_ok = None;
            while k < ws.len() && is_number(&ws[k].0) {
                let glued = ws[k]
                    .0
                    .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ',');
                let end = if !glued.is_empty() { k } else { k + 1 };
                if end >= ws.len()
                    || lin_unit(if glued.is_empty() { &ws[end].0 } else { glued }).is_none()
                {
                    break;
                }
                last_ok = Some(end);
                k = end + 1;
            }
            if let Some(end) = last_ok {
                let phrase = ws[i..=end]
                    .iter()
                    .map(|w| w.0.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                let (f, _) = parse_length(&phrase, j)?;
                return Ok(Some((f, i, end)));
            }
        }
        i += 1;
    }
    Ok(None)
}

fn find_word(ws: &[(String, usize, usize)], from: usize, keys: &[&str]) -> Option<usize> {
    (from..ws.len()).find(|&i| keys.contains(&clean(&ws[i].0).as_str()))
}

const MONUMENTS: &[&str] = &[
    "pin", "pipe", "rod", "rebar", "stake", "stone", "monument", "nail", "hub", "post", "corner",
    "tree", "mark",
];

/// Parses one call phrase (the text between two "thence").
pub fn parse_call(src: &str, j: Option<&str>) -> Result<Call, String> {
    let ws = words(src);
    let lower = src.to_ascii_lowercase();
    let mut c = Call {
        source: src.trim().to_owned(),
        kind: "line",
        ..Call::default()
    };
    if let Some(p) = lower.rfind(" to ").or_else(|| lower.rfind(" to a")) {
        let tail = &src[p + 4..];
        if MONUMENTS
            .iter()
            .any(|m| tail.to_ascii_lowercase().contains(m))
        {
            c.monument = Some(tail.trim().trim_end_matches(['.', ';', ',']).to_owned());
        }
    }
    if lower.contains("curve") || lower.contains("arc ") {
        c.kind = "curve";
        c.turn = if lower.contains("left") {
            Some("left")
        } else if lower.contains("right") {
            Some("right")
        } else {
            None
        };
        c.tangent = if lower.contains("non-tangent")
            || lower.contains("nontangent")
            || lower.contains("non tangent")
            || lower.contains("not tangent")
        {
            Some(false)
        } else if lower.contains("tangent") {
            Some(true)
        } else {
            None
        };
        if let Some(i) = find_word(&ws, 0, &["radius"]) {
            c.radius = find_length(&ws, i + 1, j)?.map(|x| x.0);
        }
        if let Some(i) = find_word(&ws, 0, &["arc"]) {
            c.arc_length = find_length(&ws, i + 1, j)?.map(|x| x.0);
        }
        if let Some(i) = find_word(&ws, 0, &["delta", "angle"]) {
            let stop = (i + 1..ws.len()).find(|&k| {
                let w = clean(&ws[k].0);
                w != "of"
                    && w != "="
                    && !is_number(&ws[k].0)
                    && !matches!(
                        w.as_str(),
                        "degrees"
                            | "degree"
                            | "deg"
                            | "minutes"
                            | "minute"
                            | "min"
                            | "seconds"
                            | "second"
                            | "sec"
                            | "and"
                    )
                    && !w.contains(['°', '\''])
            });
            let a: Vec<String> = ws[i + 1..stop.unwrap_or(ws.len())]
                .iter()
                .map(|w| w.0.clone())
                .filter(|w| clean(w) != "of" && w != "=")
                .collect();
            if !a.is_empty() {
                c.delta = gp_geo::dms::parse_plain(&angle_text(&a)).ok();
            }
        }
        if let Some(i) = find_word(&ws, 0, &["chord"])
            && let Some((az, _, e)) = find_bearing(&ws, i + 1)
        {
            c.chord_direction = Some(az);
            c.chord = find_length(&ws, e + 1, j)?.map(|x| x.0);
        }
        let plot_by_chord = c.chord_direction.is_some() && c.chord.is_some();
        let plot_by_tangent = c.tangent == Some(true)
            && c.radius.is_some()
            && (c.delta.is_some() || c.arc_length.is_some())
            && c.turn.is_some();
        if !plot_by_chord && !plot_by_tangent {
            c.flags.push("CURVE_CALL_INCOMPLETE");
        }
        return Ok(c);
    }
    let b = find_bearing(&ws, 0);
    if let Some((az, _, e)) = b {
        c.direction = Some(az);
        c.distance = find_length(&ws, e + 1, j)?.map(|x| x.0);
    } else {
        c.distance = find_length(&ws, 0, j)?.map(|x| x.0);
    }
    if c.direction.is_none() || c.distance.is_none() {
        c.kind = "non-metric";
        c.flags.push("NON_METRIC_CALL");
    }
    Ok(c)
}

/// Splits deed text into the point-of-beginning phrase and the call phrases.
pub fn split_deed(text: &str) -> (String, Vec<String>) {
    let lower = text.to_ascii_lowercase();
    let mut cuts: Vec<usize> = lower.match_indices("thence").map(|(i, _)| i).collect();
    cuts.push(text.len());
    let pob = text[..cuts[0]]
        .trim()
        .trim_end_matches([';', ','])
        .to_owned();
    let calls = cuts
        .windows(2)
        .map(|w| {
            text[w[0] + "thence".len()..w[1]]
                .trim()
                .trim_end_matches([';', ',', '.'])
                .trim()
                .to_owned()
        })
        .filter(|s| {
            !s.is_empty()
                && !s
                    .to_ascii_lowercase()
                    .starts_with("to the point of beginning")
                && !s
                    .to_ascii_lowercase()
                    .starts_with("to the place of beginning")
        })
        .collect();
    (pob, calls)
}

fn ft_text(v: f64) -> String {
    let r = (v * 1e6).round() / 1e6;
    format!("{} ftUS", gp_base::num::format_f64(r).unwrap_or_default())
}

const CALL_ROW: &[Field] = &[
    Field::new(
        "kind",
        "Kind",
        "line, curve, or non-metric",
        Kind::Text { max_len: 12 },
    ),
    Field::new(
        "direction",
        "Direction",
        "Bearing, like N 45°30'15\" E",
        Kind::Text { max_len: 40 },
    )
    .optional(),
    Field::new(
        "distance",
        "Distance",
        "US survey feet",
        Kind::Text { max_len: 40 },
    )
    .optional(),
    Field::new(
        "radius",
        "Radius",
        "Curve radius, like 300 ftUS",
        Kind::Text { max_len: 40 },
    )
    .optional(),
    Field::new(
        "arc_length",
        "Arc length",
        "Curve length along the arc, like 120 ftUS",
        Kind::Text { max_len: 40 },
    )
    .optional(),
    Field::new(
        "delta",
        "Delta",
        "Central angle",
        Kind::Text { max_len: 40 },
    )
    .optional(),
    Field::new(
        "chord_direction",
        "Chord bearing",
        "Curve chord bearing",
        Kind::Text { max_len: 40 },
    )
    .optional(),
    Field::new(
        "chord",
        "Chord",
        "Chord distance, like 118 ftUS",
        Kind::Text { max_len: 40 },
    )
    .optional(),
    Field::new(
        "turn",
        "Turn",
        "left or right, like left",
        Kind::Text { max_len: 8 },
    )
    .optional(),
    Field::new(
        "tangent",
        "Tangent",
        "tangent or non-tangent, like tangent",
        Kind::Text { max_len: 12 },
    )
    .optional(),
    Field::new(
        "monument",
        "Monument",
        "Text only, never interpreted",
        Kind::Text { max_len: 200 },
    )
    .optional(),
    Field::new(
        "source",
        "Source phrase",
        "The deed text for this call",
        Kind::Text { max_len: 2000 },
    ),
    Field::new(
        "flags",
        "Flags",
        "What needs your attention",
        Kind::Text { max_len: 200 },
    )
    .optional(),
];

pub static DEED_PARSE: ToolDef = ToolDef {
    id: "survey.land.deed-parse",
    title: "Deed parser (metes and bounds)",
    summary: "Turns pasted metes-and-bounds deed text into a table of calls (bearings, distances, curves, and monuments) for you to confirm and edit before anything is plotted.",
    aliases: &["metes and bounds parser", "legal description parser", "deed calls"],
    keywords: &["deed", "metes and bounds", "thence", "bearing", "distance", "curve", "monument", "legal description", "plot"],
    inputs: &[
        Field::new("text", "Deed text", "Paste the description, like: Beginning at an iron pin; thence N 45°30'15\" E 200.00 feet; thence ...", Kind::Text { max_len: 20_000 }).required().core(),
        Field::new("jurisdiction", "Jurisdiction", "For varas and arpents: texas, california, florida, louisiana, or missouri", Kind::Choice(JURISDICTIONS)).core(),
    ],
    outputs: &[
        Field::new("calls", "Calls", "One row per call, to confirm or edit", Kind::List { items: CALL_ROW, min: 0, max: 500 }),
        Field::new("count", "Calls", "Number of calls", Kind::Number { min: 0.0, max: 500.0 }).precision(Precision::Decimals(0)),
        Field::new("point_of_beginning", "Point of beginning", "The text before the first thence", Kind::Text { max_len: 2000 }),
        Field::new("notice", "Notice", "Professional-use notice", Kind::Text { max_len: 200 }),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["CURVE_CALL_INCOMPLETE", "NON_METRIC_CALL", "EXPERIMENTAL_TOOL"],
    model: "Calls split at each \"thence\"; quadrant bearings in symbols or words, lengths in feet, chains and links, rods, varas, or meters; curves by radius, arc, delta, chord bearing and chord",
    accuracy: "A parser, not an interpreter: every call must be confirmed against the deed. Nothing is computed until you confirm the table.",
    references: &[BROWN, BLM_MANUAL],
    examples: &[Example {
        id: "primary",
        title: "A four-call lot",
        input: r#"{"text":"Beginning at an iron pin at the northeast corner of Lot 7; thence S 00°15'00\" E 150.00 feet to an iron pin; thence South 89 degrees 45 minutes West 100.00 feet; thence N 0-15-00 W 150.00 feet; thence along the centerline of Mill Creek to the point of beginning."}"#,
        source: "Constructed example exercising symbol, word, and dash bearings and a natural-boundary call (Brown's Boundary Control, ch. 5 call forms)",
    }],
    primary_example: "primary",
    visualization: &[Layer { kind: "table-only", map: &[] }],
    related: &[Related { id: "survey.land.deed-plot", reason: "next" }],
    sentence: "Found {count} {plural count \"call\" \"calls\"}. Check each one against the deed before plotting.",
    limits: &[("batchRows", 100)],
    run: run_parse,
    ..ToolDef::BLANK
};

fn run_parse(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let text = ctx.text("text")?.unwrap_or_default();
    let j = ctx.choice("jurisdiction")?;
    let (pob, phrases) = split_deed(&text);
    if phrases.is_empty() {
        return Err(
            ToolError::invalid("/text", "No calls found: calls start with \"thence\".")
                .hint("Example: Beginning at an iron pin; thence N 45°30'15\" E 200.00 feet; ..."),
        );
    }
    let mut rows = Vec::new();
    let (mut curves_incomplete, mut non_metric) = (Vec::new(), Vec::new());
    for (i, p) in phrases.iter().enumerate() {
        let c = parse_call(p, j).map_err(|e| {
            ToolError::invalid("/text", format!("Call {}: {e}", i + 1)).hint(
                if e.contains("jurisdiction") {
                    "Set the jurisdiction field."
                } else {
                    "Edit the call in the table."
                },
            )
        })?;
        if c.flags.contains(&"CURVE_CALL_INCOMPLETE") {
            curves_incomplete.push(i + 1);
        }
        if c.flags.contains(&"NON_METRIC_CALL") {
            non_metric.push(i + 1);
        }
        let mut row = vec![("kind", Json::str(c.kind))];
        let b = |v: Option<f64>| v.map(|a| Json::str(direction::bearing(a, 0)));
        let d = |v: Option<f64>| v.map(|x| Json::str(ft_text(x)));
        for (k, v) in [
            ("direction", b(c.direction)),
            ("distance", d(c.distance)),
            ("radius", d(c.radius)),
            ("arc_length", d(c.arc_length)),
        ] {
            if let Some(v) = v {
                row.push((k, v));
            }
        }
        if let Some(a) = c.delta {
            row.push(("delta", Json::str(direction::azimuth(a, 0))));
        }
        for (k, v) in [
            ("chord_direction", b(c.chord_direction)),
            ("chord", d(c.chord)),
        ] {
            if let Some(v) = v {
                row.push((k, v));
            }
        }
        if let Some(t) = c.turn {
            row.push(("turn", Json::str(t)));
        }
        if let Some(t) = c.tangent {
            row.push((
                "tangent",
                Json::str(if t { "tangent" } else { "non-tangent" }),
            ));
        }
        if let Some(m) = &c.monument {
            row.push(("monument", Json::str(m)));
        }
        row.push(("source", Json::str(&c.source)));
        if !c.flags.is_empty() {
            row.push(("flags", Json::str(c.flags.join(", "))));
        }
        rows.push(Json::obj(row));
    }
    let list = |v: &[usize]| {
        v.iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };
    if !curves_incomplete.is_empty() {
        ctx.warnings.push(Warning::new("CURVE_CALL_INCOMPLETE", format!("Curve calls {} lack the elements to plot (chord bearing and chord, or tangent with radius, delta or arc, and direction). Add them in the table.", list(&curves_incomplete))));
    }
    if !non_metric.is_empty() {
        ctx.warnings.push(Warning::new("NON_METRIC_CALL", format!("Calls {} have no bearing and distance (a monument, adjoiner, or natural boundary). They are kept as text; add a bearing and distance or leave the figure open.", list(&non_metric))));
    }
    Ok(Json::obj(vec![
        ("calls", Json::Arr(rows)),
        ("count", Json::Num(phrases.len() as f64)),
        ("point_of_beginning", Json::str(pob)),
        ("notice", Json::str(NOTICE)),
    ]))
}

// ---------------------------------------------------------------- deed plot

const PLOT_ROW: &[Field] = &[
    Field::new(
        "kind",
        "Kind",
        "line (default) or curve, like line",
        Kind::Text { max_len: 12 },
    ),
    Field::new(
        "direction",
        "Direction",
        "Bearing or azimuth, like N 45°30'15\" E",
        Kind::Text { max_len: 40 },
    ),
    Field::new(
        "distance",
        "Distance",
        "Like 200.00 ftUS",
        Kind::Quantity {
            q: gp_base::units::Quantity::Length,
            unit: "ftUS",
        },
    ),
    Field::new(
        "radius",
        "Radius",
        "Curve radius, like 300 ftUS",
        Kind::Quantity {
            q: gp_base::units::Quantity::Length,
            unit: "ftUS",
        },
    ),
    Field::new(
        "arc_length",
        "Arc length",
        "Curve length along the arc, like 120 ftUS",
        Kind::Quantity {
            q: gp_base::units::Quantity::Length,
            unit: "ftUS",
        },
    ),
    Field::new(
        "delta",
        "Delta",
        "Central angle, like 25°30'00\"",
        Kind::Text { max_len: 40 },
    ),
    Field::new(
        "chord_direction",
        "Chord bearing",
        "For non-tangent curves, like N 45°00'00\" E",
        Kind::Text { max_len: 40 },
    ),
    Field::new(
        "chord",
        "Chord",
        "Chord distance, like 118 ftUS",
        Kind::Quantity {
            q: gp_base::units::Quantity::Length,
            unit: "ftUS",
        },
    ),
    Field::new(
        "turn",
        "Turn",
        "left or right, like left",
        Kind::Text { max_len: 8 },
    ),
    Field::new(
        "tangent",
        "Tangent",
        "tangent or non-tangent, like tangent",
        Kind::Text { max_len: 12 },
    ),
];

const POINT_ROW: &[Field] = &[
    Field::new(
        "point",
        "Point",
        "1 is the point of beginning",
        Kind::Number { min: 1.0, max: 1e6 },
    )
    .precision(Precision::Decimals(0)),
    Field::new(
        "northing",
        "Northing",
        "Relative to the start",
        Kind::Quantity {
            q: gp_base::units::Quantity::Length,
            unit: "ftUS",
        },
    )
    .precision(Precision::Decimals(3)),
    Field::new(
        "easting",
        "Easting",
        "Relative to the start",
        Kind::Quantity {
            q: gp_base::units::Quantity::Length,
            unit: "ftUS",
        },
    )
    .precision(Precision::Decimals(3)),
];

pub static DEED_PLOT: ToolDef = ToolDef {
    id: "survey.land.deed-plot",
    stability: gp_base::tool::Stability::Stable,
    version: "1.1.0",
    title: "Deed plot, closure, and area",
    summary: "Plots confirmed deed calls (lines and curves), then reports the misclosure, its direction, the precision ratio, and the area closed by the implied closing line. Courses are never adjusted unless you choose to.",
    aliases: &[
        "deed plotter",
        "deed closure calculator",
        "metes and bounds plotter",
        "legal description closure",
    ],
    keywords: &[
        "deed",
        "closure",
        "misclosure",
        "precision",
        "area",
        "acres",
        "metes and bounds",
        "plot",
    ],
    inputs: &[
        Field::new(
            "calls",
            "Calls",
            "Confirmed calls, from the deed parser or typed, like N 0°00'00\" E and 500 ftUS",
            Kind::List {
                items: PLOT_ROW,
                min: 2,
                max: 500,
            },
        )
        .required()
        .core(),
        Field::new(
            "adjustment",
            "Adjustment",
            "none (default) or compass (Bowditch), labeled when used",
            Kind::Choice(&["none", "compass"]),
        )
        .core(),
    ],
    outputs: &[
        Field::new(
            "misclosure",
            "Misclosure",
            "Distance from the last point back to the start",
            Kind::Quantity {
                q: gp_base::units::Quantity::Length,
                unit: "ftUS",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "misclosure_direction",
            "Misclosure direction",
            "Bearing from the start to the last point",
            Kind::Text { max_len: 24 },
        ),
        Field::new(
            "precision",
            "Precision",
            "1 : total length / misclosure",
            Kind::Text { max_len: 24 },
        ),
        Field::new(
            "total_length",
            "Total length",
            "Lines plus arc lengths",
            Kind::Quantity {
                q: gp_base::units::Quantity::Length,
                unit: "ftUS",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "area",
            "Area",
            "Closed by the implied closing line",
            Kind::Quantity {
                q: gp_base::units::Quantity::Area,
                unit: "ftUS2",
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "acres",
            "Acres",
            "US survey acres",
            Kind::Quantity {
                q: gp_base::units::Quantity::Area,
                unit: "acUS",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "area_basis",
            "Area basis",
            "The closing assumption or adjustment used",
            Kind::Text { max_len: 200 },
        ),
        Field::new(
            "points",
            "Points",
            "Plotted corners",
            Kind::List {
                items: POINT_ROW,
                min: 0,
                max: 501,
            },
        ),
        Field::new(
            "notice",
            "Notice",
            "Professional-use notice",
            Kind::Text { max_len: 200 },
        ),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::DegenerateGeometry],
    warnings: &[
        "PERFECT_CLOSURE",
        "CURVE_CALL_INCOMPLETE",
        "LEGACY_UNIT",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Latitudes and departures; curves advance by their chord (given, or 2R·sin(Δ/2) on a bearing of the incoming tangent ± Δ/2) and add or subtract their circular segment R²(Δ − sin Δ)/2; area by coordinates with the implied closing line",
    accuracy: "Exact arithmetic on the confirmed calls. The area includes the closing line's gap, stated in the result.",
    when_to_use: "Use this when a deed's calls have to become a figure: it plots the lines and curves you confirmed, reports the misclosure and its direction, the precision ratio, and the area closed by the implied closing line. It is the way to see whether a description closes before the coordinates are used.",
    limitations: "It plots what the deed says: courses are not adjusted unless you ask, because a survey retracement is about what the document calls for. Monuments and calls for adjoiners govern over bearings and distances on the ground, and that is a surveyor's judgement rather than arithmetic. The area includes the closing gap, which the result states.",
    references: &[GHILANI, BROWN],
    examples: &[Example {
        id: "primary",
        title: "A lot that misses closure",
        input: r#"{"calls":[{"direction":"N 0°00'00\" E","distance":"500 ftUS"},{"direction":"N 90°00'00\" E","distance":"425 ftUS"},{"direction":"S 0°00'00\" E","distance":"500 ftUS"},{"direction":"S 89°56'36\" W","distance":"425 ftUS"}]}"#,
        source: "Hand-computed latitudes and departures (Ghilani and Wolf, ch. 10): misclosure about 0.42 ft over 1,850 ft, precision about 1:4,400",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "profile-chart",
        map: &[("value", "misclosure")],
    }],
    related: &[
        Related {
            id: "survey.land.deed-parse",
            reason: "parent",
        },
        Related {
            id: "survey.cogo.traverse-closure",
            reason: "alternative",
        },
        Related {
            id: "survey.cogo.area-by-coordinates",
            reason: "next",
        },
    ],
    sentence: "The deed closes within {misclosure} ({precision}). The area is {acres}, {area_basis}.",
    limits: &[("batchRows", 100)],
    run: run_plot,
    ..ToolDef::BLANK
};

fn angle_of(ctx: &Ctx, s: &str, at: &str) -> Result<f64, ToolError> {
    let _ = ctx;
    direction::parse(s).map_err(|e| ToolError::invalid(at, e))
}

fn run_plot(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let rows = ctx.rows("calls")?;
    let fu = gp_base::units::by_symbol(gp_base::units::Quantity::Length, "ftUS").expect("ftUS");
    let mut pts = vec![(0.0f64, 0.0f64)]; // (N, E)
    let mut segs = 0.0; // Σ signed circular-segment areas (CCW positive)
    let mut total = 0.0;
    let mut legs: Vec<(f64, f64, f64)> = Vec::new(); // (dN, dE, length) for adjustment
    let mut tangent_out: Option<f64> = None;
    // Every length as entered, to report in the calls' own unit (and refuse
    // a mix of US survey and international feet).
    let mut entered: Vec<(String, gp_base::tool::Q)> = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        let at = |f: &str| format!("/calls/{i}/{f}");
        let text = |k: &str| {
            r.get(k)
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
        };
        let mut q = |ctx: &mut Ctx, k: &str| -> Result<Option<f64>, ToolError> {
            let x = ctx.row_quantity("calls", i, r, k)?;
            if let Some(x) = x {
                entered.push((format!("/calls/{i}/{k}"), x));
            }
            Ok(x.map(|x| x.to(fu)))
        };
        let kind = text("kind")
            .unwrap_or_else(|| "line".into())
            .to_ascii_lowercase();
        let (az, len, arc) = if kind == "curve" {
            let radius = q(ctx, "radius")?;
            let arc_len = q(ctx, "arc_length")?;
            let delta = match text("delta") {
                Some(d) => Some(gp_geo::dms::parse_plain(&d).map_err(|_| {
                    ToolError::invalid(&at("delta"), "Delta must be an angle like 25°30'00\".")
                })?),
                None => match (radius, arc_len) {
                    (Some(rr), Some(a)) if rr > 0.0 => Some((a / rr).to_degrees()),
                    _ => None,
                },
            };
            let turn = text("turn").map(|t| t.to_ascii_lowercase());
            let sign = match turn.as_deref() {
                Some("right") => Some(1.0),
                Some("left") => Some(-1.0),
                Some(_) => {
                    return Err(ToolError::invalid(
                        &at("turn"),
                        "Turn must be left or right.",
                    ));
                }
                None => None,
            };
            let non_tangent =
                text("tangent").is_some_and(|t| t.to_ascii_lowercase().starts_with("non"));
            let chord_given = match (text("chord_direction"), q(ctx, "chord")?) {
                (Some(cd), Some(c)) => Some((angle_of(ctx, &cd, &at("chord_direction"))?, c)),
                _ => None,
            };
            let (caz, chord) = match (chord_given, tangent_out, delta, radius, sign, non_tangent) {
                (Some(g), ..) => g,
                (None, Some(tin), Some(d), Some(rr), Some(s), false) => {
                    (tin + s * d / 2.0, 2.0 * rr * sin((d / 2.0).to_radians()))
                }
                _ => {
                    return Err(ToolError::invalid(
                        &at("chord"),
                        format!(
                            "Curve call {} cannot be plotted: give the chord bearing and chord, or (after a line or curve it is tangent to) the radius, delta or arc, and turn.",
                            i + 1
                        ),
                    ));
                }
            };
            let arc_total = match (arc_len, radius, delta) {
                (Some(a), _, _) => a,
                (None, Some(rr), Some(d)) => rr * d.to_radians(),
                _ => chord,
            };
            if let (Some(rr), Some(d), Some(s)) = (radius, delta, sign) {
                let dr = d.to_radians();
                // A right turn bulges left of its chord (−A), a left turn right (+A).
                segs += -s * rr * rr * (dr - sin(dr)) / 2.0;
                tangent_out = Some(caz + s * d / 2.0);
            } else {
                tangent_out = None;
            }
            (caz, chord, arc_total)
        } else {
            let d = text("direction").ok_or_else(|| {
                ToolError::invalid(
                    &at("direction"),
                    format!("Call {} needs a direction.", i + 1),
                )
            })?;
            let az = angle_of(ctx, &d, &at("direction"))?;
            let l = q(ctx, "distance")?.ok_or_else(|| {
                ToolError::invalid(&at("distance"), format!("Call {} needs a distance.", i + 1))
            })?;
            tangent_out = Some(az);
            (az, l, l)
        };
        if len <= 0.0 {
            return Err(ToolError::invalid(
                &at("distance"),
                format!("Call {} needs a positive length.", i + 1),
            ));
        }
        let (dn, de) = (len * cos(az.to_radians()), len * sin(az.to_radians()));
        let (n, e) = *pts.last().expect("start");
        pts.push((n + dn, e + de));
        legs.push((dn, de, arc));
        total += arc;
    }
    let u = crate::common_unit(
        &entered
            .iter()
            .map(|(p, q)| (p.as_str(), *q))
            .collect::<Vec<_>>(),
    )?;
    let len_u = |v: f64| gp_base::tool::Q {
        value: ftus(v).to(u),
        unit: u,
    };
    let (n_end, e_end) = *pts.last().expect("points");
    let mis = hypot(n_end, e_end);
    let perfect = mis <= 1e-9 * total;
    if perfect {
        ctx.warnings.push(Warning::new(
            "PERFECT_CLOSURE",
            "The calls close exactly; the precision ratio is undefined.",
        ));
    }
    let compass = ctx.choice("adjustment")? == Some("compass");
    let mut basis = if perfect {
        "the figure closes exactly".to_owned()
    } else {
        format!(
            "computed with an implied closing line of {}",
            display::quantity(
                ftus(mis).to(u),
                u.symbol,
                Precision::Decimals(2),
                ctx.options.format
            )
        )
    };
    let corners: Vec<(f64, f64)> = if compass && !perfect {
        basis = "after a compass-rule (Bowditch) adjustment you chose; the recorded courses are unchanged".into();
        let mut acc = (0.0, 0.0);
        let mut run = 0.0;
        let mut out = vec![(0.0, 0.0)];
        for (dn, de, l) in &legs {
            run += l;
            acc = (acc.0 + dn, acc.1 + de);
            out.push((acc.0 - n_end * run / total, acc.1 - e_end * run / total));
        }
        out
    } else {
        pts.clone()
    };
    // Shoelace on (E, N): positive when counterclockwise.
    let ring = &corners[..corners.len() - if compass && !perfect { 1 } else { 0 }];
    let mut twice = 0.0;
    for k in 0..ring.len() {
        let (a, b) = (ring[k], ring[(k + 1) % ring.len()]);
        twice += a.1 * b.0 - b.1 * a.0;
    }
    // Arcs bulging left of their chord subtract from the counterclockwise-positive area.
    let area = (twice / 2.0 + segs).abs();
    if area == 0.0 {
        return Err(
            ToolError::new(ErrorCode::DegenerateGeometry, "The calls enclose no area.")
                .at("/calls"),
        );
    }
    let area_of =
        |s: &str| gp_base::units::by_symbol(gp_base::units::Quantity::Area, s).expect("area unit");
    let aq = gp_base::tool::Q {
        value: area,
        unit: area_of("ftUS2"),
    };
    // US survey feet report US survey acres; other units report international acres.
    let (area_unit, acre_unit) = match u.symbol {
        "ft" => ("ft2", "ac"),
        "ftUS" => ("ftUS2", "acUS"),
        _ => ("m2", "ac"),
    };
    let points = corners
        .iter()
        .take(if compass && !perfect {
            corners.len() - 1
        } else {
            corners.len()
        })
        .enumerate()
        .map(|(k, (n, e))| {
            Json::obj([
                ("point", Json::Num((k + 1) as f64)),
                ("northing", len_u(*n).to_json()),
                ("easting", len_u(*e).to_json()),
            ])
        })
        .collect();
    Ok(Json::obj(vec![
        ("misclosure", ctx.emit("misclosure", ftus(mis), u)),
        (
            "misclosure_direction",
            Json::str(if perfect {
                "none".to_owned()
            } else {
                direction::bearing(atan2(e_end, n_end).to_degrees(), 0)
            }),
        ),
        (
            "precision",
            Json::str(if perfect {
                "perfect".to_owned()
            } else {
                format!(
                    "1:{}",
                    display::number(total / mis, Precision::Significant(2), ctx.options.format)
                )
            }),
        ),
        ("total_length", ctx.emit("total_length", ftus(total), u)),
        ("area", ctx.emit("area", aq, area_of(area_unit))),
        ("acres", ctx.emit("acres", aq, area_of(acre_unit))),
        ("area_basis", Json::str(basis)),
        ("points", Json::Arr(points)),
        ("notice", Json::str(NOTICE)),
    ]))
}

// ---------------------------------------------------------------- PLSS

/// Principal meridians and base lines (BLM CadNSDI meridian codes 01-48).
pub const MERIDIANS: &[(&str, &str, &[&str])] = &[
    (
        "01",
        "First Principal Meridian",
        &[
            "1st pm",
            "first pm",
            "1st principal meridian",
            "first principal meridian",
        ],
    ),
    (
        "02",
        "Second Principal Meridian",
        &[
            "2nd pm",
            "second pm",
            "2nd principal meridian",
            "second principal meridian",
        ],
    ),
    (
        "03",
        "Third Principal Meridian",
        &[
            "3rd pm",
            "third pm",
            "3rd principal meridian",
            "third principal meridian",
        ],
    ),
    (
        "04",
        "Fourth Principal Meridian",
        &[
            "4th pm",
            "fourth pm",
            "4th principal meridian",
            "fourth principal meridian",
        ],
    ),
    (
        "05",
        "Fifth Principal Meridian",
        &[
            "5th pm",
            "fifth pm",
            "5th principal meridian",
            "fifth principal meridian",
        ],
    ),
    (
        "06",
        "Sixth Principal Meridian",
        &[
            "6th pm",
            "sixth pm",
            "6th principal meridian",
            "sixth principal meridian",
        ],
    ),
    ("07", "Black Hills Meridian", &["black hills"]),
    ("08", "Boise Meridian", &["boise"]),
    ("09", "Chickasaw Meridian", &["chickasaw"]),
    ("10", "Choctaw Meridian", &["choctaw"]),
    ("11", "Cimarron Meridian", &["cimarron"]),
    ("12", "Copper River Meridian", &["copper river"]),
    ("13", "Fairbanks Meridian", &["fairbanks"]),
    (
        "14",
        "Gila and Salt River Meridian",
        &["gila and salt river", "gsrm", "g&srm"],
    ),
    ("15", "Humboldt Meridian", &["humboldt"]),
    ("16", "Huntsville Meridian", &["huntsville"]),
    ("17", "Indian Meridian", &["indian"]),
    ("18", "Louisiana Meridian", &["louisiana"]),
    ("19", "Michigan Meridian", &["michigan"]),
    (
        "20",
        "Principal Meridian Montana",
        &["principal meridian montana", "montana", "pmm"],
    ),
    (
        "21",
        "Mount Diablo Meridian",
        &["mount diablo", "mt diablo", "mdm", "mdb&m"],
    ),
    ("22", "Navajo Meridian", &["navajo"]),
    (
        "23",
        "New Mexico Principal Meridian",
        &["new mexico", "nmpm"],
    ),
    ("24", "St. Helena Meridian", &["st helena", "saint helena"]),
    (
        "25",
        "St. Stephens Meridian",
        &["st stephens", "saint stephens"],
    ),
    ("26", "Salt Lake Meridian", &["salt lake", "slm", "slb&m"]),
    (
        "27",
        "San Bernardino Meridian",
        &["san bernardino", "sbm", "sbb&m"],
    ),
    ("28", "Seward Meridian", &["seward"]),
    ("29", "Tallahassee Meridian", &["tallahassee"]),
    ("30", "Uintah Special Meridian", &["uintah"]),
    ("31", "Ute Principal Meridian", &["ute"]),
    ("32", "Washington Meridian", &["washington"]),
    ("33", "Willamette Meridian", &["willamette", "wm"]),
    ("34", "Wind River Meridian", &["wind river"]),
    ("44", "Kateel River Meridian", &["kateel river"]),
    ("45", "Umiat Meridian", &["umiat"]),
];

/// An aliquot part: (name, fraction of its parent).
fn aliquot(tok: &str) -> Option<(&'static str, f64)> {
    let t = tok
        .to_ascii_uppercase()
        .replace("1/4", "¼")
        .replace("1/2", "½")
        .replace('4', "¼")
        .replace('2', "½");
    let t = t.trim_end_matches([',', '.', ';', 'O', 'F']).to_owned();
    Some(match t.as_str() {
        "NE¼" => ("northeast quarter", 0.25),
        "NW¼" => ("northwest quarter", 0.25),
        "SE¼" => ("southeast quarter", 0.25),
        "SW¼" => ("southwest quarter", 0.25),
        "N½" => ("north half", 0.5),
        "S½" => ("south half", 0.5),
        "E½" => ("east half", 0.5),
        "W½" => ("west half", 0.5),
        _ => return None,
    })
}

pub struct Plss {
    pub parts: Vec<(&'static str, f64)>,
    pub lot: Option<u32>,
    pub section: u32,
    pub township: (u32, char),
    pub range: (u32, char),
    pub meridian: Option<(&'static str, &'static str)>,
}

pub fn parse_plss(s: &str) -> Result<Plss, String> {
    let norm = s
        .replace(['¼'], "¼ ")
        .replace(['½'], "½ ")
        .replace(',', " ");
    let toks: Vec<String> = norm.split_whitespace().map(str::to_owned).collect();
    let (mut parts, mut lot, mut section, mut township, mut range) =
        (Vec::new(), None, None, None, None);
    let mut i = 0;
    let num_dir = |t: &str, lead: char| -> Option<(u32, char)> {
        let u = t.to_ascii_uppercase();
        let b = u.strip_prefix(lead)?;
        let d = b.chars().last()?;
        let n: u32 = b[..b.len() - 1].parse().ok()?;
        Some((n, d))
    };
    while i < toks.len() {
        let t = &toks[i];
        let u = t.to_ascii_uppercase();
        if let Some(a) = aliquot(t) {
            parts.push(a);
        } else if u == "LOT" {
            lot = toks
                .get(i + 1)
                .and_then(|x| x.trim_end_matches(',').parse().ok());
            i += 1;
        } else if matches!(u.as_str(), "SEC" | "SEC." | "SECTION" | "S") {
            section = toks
                .get(i + 1)
                .and_then(|x| x.trim_end_matches(['.', ',']).parse().ok());
            i += 1;
        } else if let Some(td) = num_dir(&u, 'T').filter(|(_, d)| matches!(d, 'N' | 'S')) {
            township = Some(td);
        } else if let Some(rd) = num_dir(&u, 'R').filter(|(_, d)| matches!(d, 'E' | 'W')) {
            range = Some(rd);
        }
        i += 1;
    }
    let lower = format!(
        " {} ",
        s.to_ascii_lowercase()
            .replace(['.', ','], " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    );
    let meridian = MERIDIANS
        .iter()
        .find(|(_, _, keys)| {
            keys.iter().any(|k| {
                lower.contains(&format!(" {k} ")) || lower.contains(&format!(" {k} meridian"))
            })
        })
        .map(|(c, n, _)| (*c, *n));
    let section = section.ok_or("the description has no section, like Sec 12")?;
    if !(1..=36).contains(&section) {
        return Err(format!("section {section} is not 1 to 36"));
    }
    let township = township.ok_or("the description has no township, like T3N")?;
    let range = range.ok_or("the description has no range, like R4W")?;
    if township.0 == 0 || township.0 > 200 || range.0 == 0 || range.0 > 200 {
        return Err("township and range numbers must be between 1 and 200".into());
    }
    Ok(Plss {
        parts,
        lot,
        section,
        township,
        range,
        meridian,
    })
}

pub static PLSS_PARSE: ToolDef = ToolDef {
    id: "survey.land.plss-parse",
    title: "PLSS legal description reader",
    summary: "Reads a Public Land Survey System description like NE¼ SW¼ Sec 12, T3N R4W, 6th PM into plain words, and gives the nominal aliquot area of a standard 640-acre section.",
    aliases: &[
        "township range section",
        "PLSS decoder",
        "legal description reader",
        "aliquot calculator",
    ],
    keywords: &[
        "PLSS",
        "township",
        "range",
        "section",
        "aliquot",
        "quarter section",
        "principal meridian",
        "BLM",
        "legal description",
    ],
    inputs: &[Field::new(
        "description",
        "Description",
        "Like NE¼ SW¼ Sec 12, T3N R4W, 6th PM",
        Kind::Text { max_len: 300 },
    )
    .required()
    .core()],
    outputs: &[
        Field::new(
            "plain",
            "In words",
            "The description read right to left",
            Kind::Text { max_len: 400 },
        ),
        Field::new(
            "nominal_area",
            "Nominal area",
            "Standard 640-acre section, labeled nominal",
            Kind::Quantity {
                q: gp_base::units::Quantity::Area,
                unit: "acUS",
            },
        )
        .precision(Precision::Decimals(2))
        .optional(),
        Field::new(
            "section",
            "Section",
            "1 to 36",
            Kind::Number {
                min: 1.0,
                max: 36.0,
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new("township", "Township", "Like 3N", Kind::Text { max_len: 8 }),
        Field::new("range", "Range", "Like 4W", Kind::Text { max_len: 8 }),
        Field::new(
            "meridian",
            "Principal meridian",
            "BLM name",
            Kind::Text { max_len: 60 },
        ),
        Field::new(
            "meridian_code",
            "Meridian code",
            "BLM CadNSDI code",
            Kind::Text { max_len: 4 },
        ),
        Field::new(
            "next_step",
            "Location",
            "What locating it takes",
            Kind::Text { max_len: 200 },
        ),
        Field::new(
            "notice",
            "Notice",
            "Professional-use notice",
            Kind::Text { max_len: 200 },
        ),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["NOMINAL_VALUE_USED", "EXPERIMENTAL_TOOL"],
    model: "Aliquot parts in words (NE¼ SW¼ is the northeast quarter of the southwest quarter); nominal area = 640 acres × the product of the fractions",
    accuracy: "Nominal only: real sections vary from 640 acres, and fractional sections and lots are not nominal. Locating the parcel needs the BLM section geometry.",
    references: &[BLM_MANUAL],
    examples: &[Example {
        id: "primary",
        title: "NE¼ SW¼ Sec 12, T3N R4W, 6th PM",
        input: r#"{"description":"NE¼ SW¼ Sec 12, T3N R4W, 6th PM"}"#,
        source: "add-practitioner-essentials PLSS scenario: the northeast quarter of the southwest quarter of section 12, nominal 40 acres",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[Related {
        id: "survey.land.legacy-units",
        reason: "alternative",
    }],
    sentence: "This is {plain}.{if nominal_area > 0} Nominal area: {nominal_area}.{/if}",
    limits: &[("batchRows", 10_000)],
    run: run_plss,
    ..ToolDef::BLANK
};

fn run_plss(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let d = ctx.text("description")?.unwrap_or_default();
    let p = parse_plss(&d)
        .map_err(|e| ToolError::invalid("/description", format!("Could not read it: {e}.")))?;
    let (code, meridian) = p.meridian.ok_or_else(|| {
        ToolError::invalid(
            "/description",
            "Name the principal meridian: township numbers repeat under each meridian.",
        )
        .hint("Examples: 6th PM, Willamette, Mount Diablo, Salt Lake")
    })?;
    let dir = |c: char| match c {
        'N' => "north",
        'S' => "south",
        'E' => "east",
        _ => "west",
    };
    let mut words: Vec<String> = Vec::new();
    for (k, (name, _)) in p.parts.iter().enumerate() {
        words.push(format!("{}the {name}", if k == 0 { "" } else { "of " }));
    }
    if let Some(l) = p.lot {
        words.push(format!(
            "{}lot {l}",
            if words.is_empty() { "" } else { "of " }
        ));
    }
    let plain = format!(
        "{}{}section {}, township {} {}, range {} {}, {}",
        words.join(" "),
        if words.is_empty() { "" } else { " of " },
        p.section,
        p.township.0,
        dir(p.township.1),
        p.range.0,
        dir(p.range.1),
        meridian
    );
    let mut o = vec![("plain", Json::str(plain))];
    if p.lot.is_none() {
        let frac: f64 = p.parts.iter().map(|(_, f)| f).product();
        let ac_us =
            gp_base::units::by_symbol(gp_base::units::Quantity::Area, "acUS").expect("acUS");
        ctx.warnings.push(Warning::new("NOMINAL_VALUE_USED", "The area assumes a standard 640-acre section. Real sections vary, and lots are not nominal."));
        o.push((
            "nominal_area",
            ctx.out(
                "nominal_area",
                gp_base::tool::Q {
                    value: 640.0 * frac,
                    unit: ac_us,
                },
            ),
        ));
    }
    o.extend([
        ("section", Json::Num(p.section as f64)),
        ("township", Json::str(format!("{}{}", p.township.0, p.township.1))),
        ("range", Json::str(format!("{}{}", p.range.0, p.range.1))),
        ("meridian", Json::str(meridian)),
        ("meridian_code", Json::str(code)),
        ("next_step", Json::str("To locate it, install the state's BLM PLSS data (CadNSDI); aliquot lines are never drawn by quartering an ideal square.")),
        ("notice", Json::str(NOTICE)),
    ]);
    Ok(Json::obj(o))
}

// ---------------------------------------------------------------- basis rotation

const DIR_ROW: &[Field] = &[Field::new(
    "direction",
    "Direction",
    "Like N 45°30'15\" E",
    Kind::Text { max_len: 40 },
)];
const ROT_ROW: &[Field] = &[
    Field::new("record", "Record", "As written", Kind::Text { max_len: 40 }),
    Field::new(
        "rotated",
        "Rotated",
        "On the new basis",
        Kind::Text { max_len: 40 },
    ),
];

pub static BASIS_ROTATION: ToolDef = ToolDef {
    id: "survey.land.basis-rotation",
    title: "Basis-of-bearing rotation",
    summary: "Rotates deed or plat bearings to a new basis of bearing (like grid north) from one line whose bearing is known in both, and reports the rotation angle.",
    aliases: &[
        "rotate bearings",
        "basis of bearing rotation",
        "record to grid bearings",
    ],
    keywords: &[
        "basis of bearing",
        "rotation",
        "record bearing",
        "grid bearing",
        "deed",
        "plat",
    ],
    inputs: &[
        Field::new(
            "record_bearing",
            "Line's record bearing",
            "Like N 10°00'00\" E",
            Kind::Text { max_len: 40 },
        )
        .required()
        .core(),
        Field::new(
            "new_bearing",
            "Same line on the new basis",
            "Like N 10°02'30\" E",
            Kind::Text { max_len: 40 },
        )
        .required()
        .core(),
        Field::new(
            "directions",
            "Bearings to rotate",
            "One per row, like S 80°00'00\" E",
            Kind::List {
                items: DIR_ROW,
                min: 0,
                max: 500,
            },
        )
        .core(),
    ],
    outputs: &[
        Field::new(
            "rotation",
            "Rotation",
            "New minus record, clockwise positive",
            Kind::Quantity {
                q: gp_base::units::Quantity::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(6)),
        Field::new(
            "rotation_dms",
            "Rotation",
            "In DMS with its sign",
            Kind::Text { max_len: 24 },
        ),
        Field::new(
            "rotated",
            "Rotated bearings",
            "Record and rotated",
            Kind::List {
                items: ROT_ROW,
                min: 0,
                max: 500,
            },
        ),
        Field::new(
            "notice",
            "Notice",
            "Professional-use notice",
            Kind::Text { max_len: 200 },
        ),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "Rotation = new azimuth − record azimuth; each bearing's azimuth plus the rotation",
    accuracy: "Exact. The rotation is only as good as the common line's two bearings.",
    references: &[GHILANI, BROWN],
    examples: &[Example {
        id: "primary",
        title: "Record N 10°00'00\" E is N 10°02'30\" E on grid",
        input: r#"{"record_bearing":"N 10°00'00\" E","new_bearing":"N 10°02'30\" E","directions":[{"direction":"S 80°00'00\" E"},{"direction":"N 45°30'00\" W"}]}"#,
        source: "add-practitioner-essentials rotation scenario: every call rotates by +0°02'30\"",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[Related {
        id: "survey.land.deed-plot",
        reason: "next",
    }],
    sentence: "Rotate every bearing by {rotation_dms}.",
    limits: &[("batchRows", 1_000)],
    run: run_rotation,
    ..ToolDef::BLANK
};

fn run_rotation(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let rec = ctx.text("record_bearing")?.unwrap_or_default();
    let new = ctx.text("new_bearing")?.unwrap_or_default();
    let a = direction::parse(&rec).map_err(|e| ToolError::invalid("/record_bearing", e))?;
    let b = direction::parse(&new).map_err(|e| ToolError::invalid("/new_bearing", e))?;
    let mut rot = gp_base::angle::wrap_azimuth(b - a);
    if rot > 180.0 {
        rot -= 360.0;
    }
    let rows = ctx.rows("directions")?;
    let mut out = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        let d = ctx
            .row_text("directions", i, r, "direction")?
            .unwrap_or_default();
        let az = direction::parse(&d)
            .map_err(|e| ToolError::invalid(&format!("/directions/{i}/direction"), e))?;
        // Keep the precision of the record: one more decimal of seconds than needed is noise.
        out.push(Json::obj([
            ("record", Json::str(&d)),
            ("rotated", Json::str(direction::bearing(az + rot, 1))),
        ]));
    }
    let deg = gp_base::units::by_symbol(gp_base::units::Quantity::Angle, "deg").expect("deg");
    Ok(Json::obj(vec![
        (
            "rotation",
            ctx.out(
                "rotation",
                gp_base::tool::Q {
                    value: rot,
                    unit: deg,
                },
            ),
        ),
        (
            "rotation_dms",
            Json::str(format!(
                "{}{}",
                if rot < 0.0 { "-" } else { "+" },
                direction::azimuth(rot.abs(), 1)
            )),
        ),
        ("rotated", Json::Arr(out)),
        ("notice", Json::str(NOTICE)),
    ]))
}
