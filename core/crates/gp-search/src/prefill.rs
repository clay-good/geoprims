//! Natural-language prefill (discovery/natural-language-prefill): pulls the
//! quantities out of a typed question, leaves the words for ranking, and maps
//! the quantities onto the top tool's inputs through its slots. Only
//! unambiguous inputs are filled; the rest are listed with their candidates.

use gp_base::json::Json;
use gp_base::units::{self, Quantity};
use serde_json::Value;

/// What a question contributed: values for the inputs and the words left for ranking.
pub struct Parsed {
    pub values: Vec<Val>,
    pub words: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum Kind {
    /// A number with its unit as typed (resolved per slot), or none.
    Num {
        num: f64,
        text: String,
        decimal: bool,
        unit: Option<String>,
    },
    /// A label, like a runway designator.
    Text(String),
}

#[derive(Clone, Debug)]
pub struct Val {
    pub kind: Kind,
    /// Words next to the value that may name its input: after first, then before.
    pub near: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num {
        num: f64,
        text: String,
        decimal: bool,
        glued: bool,
    },
    Word(String),
    Sym(char),
}

/// Spellings people type that the unit registry keeps case-sensitive.
const SYNONYMS: &[(&str, &str)] = &[
    ("c", "degC"),
    ("°c", "degC"),
    ("f", "degF"),
    ("°f", "degF"),
    ("hg", "inHg"),
    ("\"", "inHg"),
    ("feet", "ft"),
    ("foot", "ft"),
    ("meters", "m"),
    ("metres", "m"),
    ("meter", "m"),
    ("metre", "m"),
];

/// Words that sit between a value and its name and never name an input.
const FILLER: &[&str] = &[
    "at", "of", "is", "to", "a", "the", "and", "with", "on", "for", "in", "by",
];

/// Resolves a word to a unit spelling the registry knows, if it is a unit.
fn unit_word(w: &str) -> Option<String> {
    if let Some((_, to)) = SYNONYMS.iter().find(|(from, _)| *from == w.to_lowercase()) {
        return Some((*to).to_owned());
    }
    let any = |t: &str| {
        units::UNITS
            .iter()
            .any(|u| u.symbol == t || u.aliases.contains(&t))
    };
    if any(w) {
        return Some(w.to_owned());
    }
    let lower = w.to_lowercase();
    units::UNITS
        .iter()
        .flat_map(|u| std::iter::once(u.symbol).chain(u.aliases.iter().copied()))
        .find(|s| s.to_lowercase() == lower)
        .map(str::to_owned)
}

fn scan(q: &str) -> Vec<Tok> {
    let cs: Vec<char> = q.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let word_char = |c: char| {
        c.is_alphanumeric() || matches!(c, '°' | '/' | '²' | '℃' | '℉' | '″' | '"' | '\'' | '-')
    };
    while i < cs.len() {
        let c = cs[i];
        let prev_space = i == 0 || !cs[i - 1].is_alphanumeric();
        let sign = matches!(c, '-' | '+' | '−')
            && prev_space
            && cs
                .get(i + 1)
                .is_some_and(|d| d.is_ascii_digit() || *d == '.');
        if c.is_ascii_digit()
            || sign
            || (c == '.' && cs.get(i + 1).is_some_and(char::is_ascii_digit))
        {
            let mut text = String::new();
            if sign {
                if c != '+' {
                    text.push('-');
                }
                i += 1;
            }
            let mut decimal = false;
            while i < cs.len() {
                let d = cs[i];
                if d.is_ascii_digit() {
                    text.push(d);
                } else if d == ','
                    && cs
                        .get(i + 1..i + 4)
                        .is_some_and(|w| w.iter().all(char::is_ascii_digit))
                    && !cs.get(i + 4).is_some_and(char::is_ascii_digit)
                    && !text.contains('.')
                {
                    // Thousands separator: 5,000.
                } else if d == '.' && !decimal && cs.get(i + 1).is_some_and(char::is_ascii_digit) {
                    decimal = true;
                    text.push('.');
                } else {
                    break;
                }
                i += 1;
            }
            let glued = cs
                .get(i)
                .is_some_and(|d| d.is_alphabetic() || matches!(d, '°' | '"' | '″' | '℃' | '℉'));
            if let Ok(num) = text.parse::<f64>() {
                out.push(Tok::Num {
                    num,
                    text,
                    decimal,
                    glued,
                });
            }
        } else if c.is_alphabetic() || matches!(c, '°' | '"' | '″' | '℃' | '℉') {
            let mut w = String::new();
            while i < cs.len() && word_char(cs[i]) {
                // A trailing hyphen or quote joins nothing.
                w.push(cs[i]);
                i += 1;
            }
            let w = w.trim_end_matches(['-', '\'']).to_owned();
            if !w.is_empty() {
                out.push(Tok::Word(w));
            }
        } else {
            if matches!(c, '@' | ',' | '/') {
                out.push(Tok::Sym(c));
            }
            i += 1;
        }
    }
    out
}

fn num(t: Option<&Tok>) -> Option<(f64, &str, bool)> {
    match t {
        Some(Tok::Num {
            num, text, decimal, ..
        }) => Some((*num, text.as_str(), *decimal)),
        _ => None,
    }
}

fn word(t: Option<&Tok>) -> Option<&str> {
    match t {
        Some(Tok::Word(w)) => Some(w.as_str()),
        _ => None,
    }
}

fn numv(num: f64, text: &str, decimal: bool, unit: Option<&str>) -> Kind {
    Kind::Num {
        num,
        text: text.to_owned(),
        decimal,
        unit: unit.map(str::to_owned),
    }
}

/// Splits a question into values and ranking words.
pub fn parse(q: &str) -> Parsed {
    let toks = scan(q);
    // Each token is consumed by a value, kept as a word, or dropped.
    let mut values: Vec<(Kind, usize, usize, Option<&'static str>)> = Vec::new(); // kind, first, last, hint
    let mut used = vec![false; toks.len()];
    let mut i = 0;
    while i < toks.len() {
        if used[i] {
            i += 1;
            continue;
        }
        let w = word(toks.get(i)).map(str::to_lowercase);
        // Runway designator after rwy or runway: 27, 09L, 27R.
        if matches!(w.as_deref(), Some("rwy" | "runway" | "rw"))
            && let Some((n, _, false)) = num(toks.get(i + 1))
            && (1.0..=36.0).contains(&n)
        {
            let mut label = format!("{:02}", n as u32);
            let mut last = i + 1;
            if let Some(Tok::Num { glued: true, .. }) = toks.get(i + 1)
                && let Some(side) = word(toks.get(i + 2))
                    .filter(|s| matches!(s.to_uppercase().as_str(), "L" | "R" | "C"))
            {
                label.push_str(&side.to_uppercase());
                last = i + 2;
            }
            values.push((Kind::Text(label), i + 1, last, Some("runway")));
            used[i + 1..=last].iter_mut().for_each(|u| *u = true);
            i = last + 1;
            continue;
        }
        // Altimeter groups: A2992 (inHg), Q1013 (hPa).
        if let Some(w) = word(toks.get(i)) {
            let head_len = w.chars().next().map_or(0, char::len_utf8);
            let (head, digits) = w.split_at(head_len);
            if digits.len() == 4 && digits.chars().all(|c| c.is_ascii_digit()) {
                let n: f64 = digits.parse().unwrap_or(0.0);
                let got = match head {
                    "A" | "a" if (2600.0..=3200.0).contains(&n) => Some((
                        n / 100.0,
                        format!("{}.{}", &digits[..2], &digits[2..]),
                        "inHg",
                    )),
                    "Q" | "q" if (900.0..=1100.0).contains(&n) => {
                        Some((n, digits.to_owned(), "hPa"))
                    }
                    _ => None,
                };
                if let Some((v, text, unit)) = got {
                    values.push((
                        numv(v, &text, unit == "inHg", Some(unit)),
                        i,
                        i,
                        Some("altimeter"),
                    ));
                    used[i] = true;
                    i += 1;
                    continue;
                }
            }
        }
        if let Some((n, text, dec)) = num(toks.get(i)) {
            // Wind: 300@15, 300/15, 300 at 15 (three-digit direction), and METAR 30015KT.
            let sep = matches!(toks.get(i + 1), Some(Tok::Sym('@' | '/')))
                || word(toks.get(i + 1)).is_some_and(|w| w.eq_ignore_ascii_case("at"));
            if sep
                && text.len() == 3
                && !dec
                && n <= 360.0
                && let Some((s, stext, _)) = num(toks.get(i + 2))
            {
                let unit = word(toks.get(i + 3)).and_then(unit_word);
                let last = if unit.is_some() { i + 3 } else { i + 2 };
                values.push((numv(n, text, false, Some("deg")), i, i, Some("wind")));
                values.push((
                    numv(s, stext, false, Some(unit.as_deref().unwrap_or("kt"))),
                    i + 2,
                    last,
                    Some("wind"),
                ));
                used[i..=last].iter_mut().for_each(|u| *u = true);
                i = last + 1;
                continue;
            }
            // METAR wind: 30015KT, or 30015G25KT with a gust.
            let metar = word(toks.get(i + 1)).and_then(|w| {
                let u = w.to_uppercase();
                if u == "KT" || u == "KTS" {
                    return Some(None);
                }
                let g = u.strip_prefix('G')?.strip_suffix("KT")?;
                (matches!(g.len(), 2 | 3) && g.chars().all(|c| c.is_ascii_digit()))
                    .then(|| Some(g.to_owned()))
            });
            if text.len() == 5
                && !dec
                && let Some(gust) = metar
            {
                let (d, sp) = text.split_at(3);
                if let (Ok(dn), Ok(sn)) = (d.parse::<f64>(), sp.parse::<f64>())
                    && dn <= 360.0
                {
                    values.push((numv(dn, d, false, Some("deg")), i, i, Some("wind")));
                    values.push((numv(sn, sp, false, Some("kt")), i, i + 1, Some("wind")));
                    if let Some(g) = gust {
                        let gn = g.parse::<f64>().unwrap_or(0.0);
                        values.push((numv(gn, &g, false, Some("kt")), i, i + 1, Some("gust")));
                    }
                    used[i] = true;
                    used[i + 1] = true;
                    i += 2;
                    continue;
                }
            }
            // A coordinate pair: 40.4461, -79.9822.
            // Whole numbers need the comma: 40, -105.
            let comma = toks.get(i + 1) == Some(&Tok::Sym(','));
            if n.abs() <= 90.0
                && (dec || comma)
                && let Some(j) = [i + 1, i + 2].into_iter().find(|&j| {
                    matches!(toks.get(j), Some(Tok::Num { .. })) && (j == i + 1 || comma)
                })
                && let Some((m, mtext, mdec)) = num(toks.get(j))
                && (mdec || comma)
                && (dec == mdec || comma)
                && m.abs() <= 180.0
                && word(toks.get(j + 1)).and_then(unit_word).is_none()
            {
                values.push((numv(n, text, dec, Some("deg")), i, i, Some("lat")));
                values.push((numv(m, mtext, mdec, Some("deg")), j, j, Some("lon")));
                used[i..=j].iter_mut().for_each(|u| *u = true);
                i = j + 1;
                continue;
            }
            // A number, with its unit when the next word is one.
            let unit = word(toks.get(i + 1)).and_then(unit_word);
            let last = if unit.is_some() { i + 1 } else { i };
            values.push((numv(n, text, dec, unit.as_deref()), i, last, None));
            used[i..=last].iter_mut().for_each(|u| *u = true);
            i = last + 1;
            continue;
        }
        i += 1;
    }
    // Names next to each value: the word after it first, then the word before.
    // A word another value consumed (its unit) names nothing.
    let near_word = |idx: Option<usize>| -> Option<String> {
        let idx = idx?;
        if used.get(idx) == Some(&true) {
            return None;
        }
        match toks.get(idx)? {
            Tok::Word(w) if !FILLER.contains(&w.to_lowercase().as_str()) => Some(w.to_lowercase()),
            _ => None,
        }
    };
    let vals = values
        .into_iter()
        .map(|(kind, first, last, hint)| {
            let mut near: Vec<String> = hint.map(str::to_owned).into_iter().collect();
            near.extend(near_word(Some(last + 1)));
            near.extend(near_word(first.checked_sub(1)));
            Val { kind, near }
        })
        .collect();
    // A conversion ("32 f to c") ranks on its units too.
    let conversion = toks.iter().any(
        |t| matches!(t, Tok::Word(w) if matches!(w.to_lowercase().as_str(), "to" | "into" | "in")),
    );
    let words = toks
        .iter()
        .zip(&used)
        .filter(|(t, u)| {
            !**u || (conversion && matches!(t, Tok::Word(w) if unit_word(w).is_some()))
        })
        .filter_map(|(t, _)| match t {
            Tok::Word(w) => Some(w.clone()),
            _ => None,
        })
        .collect();
    Parsed {
        values: vals,
        words,
    }
}

/// One input as the prefill sees it.
#[derive(Clone, Debug)]
pub struct SlotDef {
    pub input: String,
    pub quantity: Option<Quantity>,
    pub unit: String,
    pub text: bool,
    /// Declared by the tool; only these text inputs take numbers ("rwy 27").
    pub explicit: bool,
    pub keywords: Vec<String>,
    pub range: (f64, f64),
    /// "never", "any", or "decimal".
    pub bare: String,
}

/// Slots for every input of a manifest, in input order (the manifest's
/// `prefill` list): explicit entries as declared, defaults otherwise.
pub fn slots(m: &Value) -> Vec<SlotDef> {
    let Some(props) = m["inputs"]["properties"].as_object() else {
        return Vec::new();
    };
    let listed: Vec<&Value> = m["prefill"]
        .as_array()
        .map(|a| a.iter().collect())
        .unwrap_or_default();
    let order: Vec<(&str, Option<&Value>)> = if listed.is_empty() {
        props.keys().map(|k| (k.as_str(), None)).collect()
    } else {
        listed
            .iter()
            .filter_map(|e| Some((e["input"].as_str()?, Some(*e))))
            .collect()
    };
    order
        .into_iter()
        .filter(|(name, _)| *name != "options")
        .filter_map(|(name, ex)| {
            let p = props.get(name)?;
            let explicit =
                ex.is_some_and(|e| e["keywords"].as_array().is_some_and(|k| !k.is_empty()));
            let quantity = p["x-quantity"].as_str().and_then(Quantity::from_id);
            let types: Vec<&str> = match &p["type"] {
                Value::String(s) => vec![s.as_str()],
                Value::Array(a) => a.iter().filter_map(Value::as_str).collect(),
                _ => vec![],
            };
            let is_num =
                quantity.is_some() || types.iter().any(|t| matches!(*t, "number" | "integer"));
            let text = !is_num && types.contains(&"string") && p["enum"].is_null();
            if !is_num && !text {
                return None;
            }
            let mut keywords: Vec<String> = ex
                .map(|e| {
                    e["keywords"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|k| k.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default();
            // lat1 and lon2 answer to "lat" and "lon" too.
            keywords.extend(
                name.split('_')
                    .map(|w| w.trim_end_matches(|c: char| c.is_ascii_digit()).to_owned()),
            );
            keywords.extend(crate::tokens(p["title"].as_str().unwrap_or_default()));
            let bound = |k: &str, d: f64| p[k].as_f64().unwrap_or(d);
            let range = match ex.map(|e| &e["range"]) {
                Some(Value::Array(r)) => (
                    r[0].as_f64().unwrap_or(f64::NEG_INFINITY),
                    r[1].as_f64().unwrap_or(f64::INFINITY),
                ),
                _ if quantity.is_none() => (
                    bound("minimum", f64::NEG_INFINITY),
                    bound("maximum", f64::INFINITY),
                ),
                _ => (f64::NEG_INFINITY, f64::INFINITY),
            };
            let bare = match ex.and_then(|e| e["bare"].as_str()) {
                Some(b) => b.to_owned(),
                None if quantity.is_none() && is_num => "any".to_owned(),
                None => "never".to_owned(),
            };
            Some(SlotDef {
                input: name.to_owned(),
                quantity,
                unit: p["x-unit"].as_str().unwrap_or_default().to_owned(),
                text,
                explicit,
                keywords,
                range,
                bare,
            })
        })
        .collect()
}

/// The value a slot would take from `v`, or None when it cannot.
fn fit(s: &SlotDef, v: &Val, named: bool) -> Option<Json> {
    match &v.kind {
        Kind::Text(t) => (s.text && s.explicit && named).then(|| Json::str(t)),
        Kind::Num {
            num,
            text,
            decimal,
            unit,
        } => {
            if s.text {
                return (named && s.explicit).then(|| Json::str(text));
            }
            let in_range = |x: f64| x >= s.range.0 && x <= s.range.1;
            match (unit, s.quantity) {
                (Some(u), Some(q)) => {
                    let from = units::lookup(q, u)?;
                    let to = units::by_symbol(q, &s.unit)?;
                    // Kept as typed ("40 gal"): the core reads every registry spelling.
                    in_range(units::convert(*num, from, to))
                        .then(|| Json::str(format!("{text} {u}")))
                }
                (Some(_), None) => None,
                (None, q) => {
                    // A bare number enters a quantity input only through a slot the
                    // tool declared: "temperature 0.8" is not 0.8 °C by default.
                    let dimensionless = q.is_none_or(|q| q == Quantity::Dimensionless);
                    let allowed = (named && (s.explicit || dimensionless))
                        || s.bare == "any"
                        || (s.bare == "decimal" && *decimal);
                    if !allowed || !in_range(*num) {
                        return None;
                    }
                    Some(match q {
                        Some(q) if q != Quantity::Dimensionless => {
                            Json::str(format!("{text} {}", s.unit))
                        }
                        _ => Json::Num(*num),
                    })
                }
            }
        }
    }
}

/// How many values some input of the tool could take: a ranking tiebreak, so
/// "utm 40.4461, -79.9822" prefers the tool that takes a latitude.
pub fn fits(slots: &[SlotDef], values: &[Val]) -> u32 {
    values
        .iter()
        .filter(|v| slots.iter().any(|s| fit(s, v, names(s, v)).is_some()))
        .count() as u32
}

fn names(s: &SlotDef, v: &Val) -> bool {
    v.near.iter().any(|w| {
        s.keywords
            .iter()
            .any(|k| k == w || crate::stem(k) == crate::stem(w))
    })
}

fn show(v: &Val) -> String {
    match &v.kind {
        Kind::Text(t) => t.clone(),
        Kind::Num { text, unit, .. } => match unit {
            Some(u) => format!("{text} {u}"),
            None => text.clone(),
        },
    }
}

/// Maps values onto slots. Returns (fields in slot order, ambiguous values with candidates).
pub fn map(slots: &[SlotDef], values: &[Val]) -> (Vec<(String, Json)>, Vec<(String, Vec<String>)>) {
    let mut filled: Vec<Option<Json>> = vec![None; slots.len()];
    let mut placed = vec![false; values.len()];
    // 1. A value next to a word that names a compatible input fills the first such input.
    for (vi, v) in values.iter().enumerate() {
        if let Some(si) = (0..slots.len()).find(|&si| {
            filled[si].is_none() && names(&slots[si], v) && fit(&slots[si], v, true).is_some()
        }) {
            filled[si] = fit(&slots[si], v, true);
            placed[vi] = true;
        }
    }
    let cands = |vi: usize, filled: &[Option<Json>]| -> Vec<usize> {
        (0..slots.len())
            .filter(|&si| filled[si].is_none() && fit(&slots[si], &values[vi], false).is_some())
            .collect()
    };
    // 2. Elimination: a value with one possible input, or an input with one possible value.
    loop {
        let mut changed = false;
        for vi in 0..values.len() {
            if placed[vi] {
                continue;
            }
            let c = cands(vi, &filled);
            let rivals = (0..values.len())
                .filter(|&o| o != vi && !placed[o] && c.len() == 1 && cands(o, &filled) == c)
                .count();
            if c.len() == 1 && rivals == 0 {
                filled[c[0]] = fit(&slots[c[0]], &values[vi], false);
                placed[vi] = true;
                changed = true;
            }
        }
        for si in 0..slots.len() {
            if filled[si].is_some() {
                continue;
            }
            let takers: Vec<usize> = (0..values.len())
                .filter(|&vi| !placed[vi] && cands(vi, &filled).contains(&si))
                .collect();
            if takers.len() == 1
                && cands(takers[0], &filled)
                    .iter()
                    .all(|&o| o == si || takers_of(o, &placed, values, &filled, slots) > 1)
            {
                filled[si] = fit(&slots[si], &values[takers[0]], false);
                placed[takers[0]] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    // 3. A lone value among inputs of the same kind fills the first of them;
    //    several values competing for the same inputs are ambiguous.
    let mut ambiguous = Vec::new();
    for vi in 0..values.len() {
        if placed[vi] {
            continue;
        }
        let c = cands(vi, &filled);
        if c.is_empty() {
            continue;
        }
        let rivals: Vec<usize> = (0..values.len())
            .filter(|&o| !placed[o] && cands(o, &filled).iter().any(|s| c.contains(s)))
            .collect();
        if rivals.len() == 1 {
            filled[c[0]] = fit(&slots[c[0]], &values[vi], false);
            placed[vi] = true;
        } else {
            ambiguous.push((
                show(&values[vi]),
                c.iter().map(|&s| slots[s].input.clone()).collect(),
            ));
        }
    }
    let fields = slots
        .iter()
        .zip(filled)
        .filter_map(|(s, f)| f.map(|f| (s.input.clone(), f)))
        .collect();
    (fields, ambiguous)
}

/// How many unplaced values could still fill slot `si`.
fn takers_of(
    si: usize,
    placed: &[bool],
    values: &[Val],
    filled: &[Option<Json>],
    slots: &[SlotDef],
) -> usize {
    (0..values.len())
        .filter(|&vi| {
            !placed[vi] && filled[si].is_none() && fit(&slots[si], &values[vi], false).is_some()
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shown(q: &str) -> (Vec<String>, Vec<String>) {
        let p = parse(q);
        let vals = p
            .values
            .iter()
            .map(|v| {
                format!(
                    "{}{}",
                    show(v),
                    v.near
                        .first()
                        .map(|n| format!(" [{n}]"))
                        .unwrap_or_default()
                )
            })
            .collect();
        (vals, p.words)
    }

    #[test]
    fn extracts_quantities_and_leaves_words() {
        assert_eq!(
            shown("density altitude 5000 ft 30C 29.80"),
            (
                vec![
                    "5000 ft [altitude]".into(),
                    "30 degC".into(),
                    "29.80".into()
                ],
                vec!["density".into(), "altitude".into()]
            )
        );
        assert_eq!(shown("5,000ft").0, vec!["5000 ft"]);
        assert_eq!(shown("-5°C").0, vec!["-5 degC"]);
        assert_eq!(
            shown("A2992 Q1013").0,
            vec!["29.92 inHg [altimeter]", "1013 hPa [altimeter]"]
        );
    }

    #[test]
    fn runway_wind_and_coordinates() {
        assert_eq!(
            shown("rwy 27 wind 300 at 15").0,
            vec!["27 [runway]", "300 deg [wind]", "15 kt [wind]"]
        );
        assert_eq!(shown("runway 9l").0, vec!["09L [runway]"]);
        assert_eq!(shown("300@15").0, vec!["300 deg [wind]", "15 kt [wind]"]);
        assert_eq!(
            shown("30015G25KT").0,
            vec!["300 deg [wind]", "15 kt [wind]", "25 kt [gust]"]
        );
        assert_eq!(
            shown("40.4461, -79.9822").0,
            vec!["40.4461 deg [lat]", "-79.9822 deg [lon]"]
        );
        assert_eq!(shown("40, -105").0, vec!["40 deg [lat]", "-105 deg [lon]"]);
        // Whole numbers without a comma are not a coordinate.
        assert_eq!(shown("40 105").0, vec!["40", "105"]);
    }

    #[test]
    fn names_next_to_values() {
        let p = parse("gsd 13.2mm sensor 8.8mm lens");
        assert_eq!(p.values[0].near, vec!["sensor", "gsd"]);
        assert_eq!(p.values[1].near, vec!["lens", "sensor"]);
    }

    #[test]
    fn nothing_to_extract() {
        let p = parse("density altitude");
        assert!(p.values.is_empty());
        assert_eq!(p.words, vec!["density", "altitude"]);
        // Multi-byte first letters never split mid-character.
        assert!(parse("°C ℃ é1234").values.is_empty());
    }
}
