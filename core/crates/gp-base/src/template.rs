//! The sentence-template language (`contracts/manifest-extensions`, "Sentence-
//! template language"), rendered by the core so the web answer card and the MCP
//! `summary` are identical:
//!
//! - `{field}` or `{field:unit}` inserts a value with the field's display precision
//! - `{delta(a,b)}` and `{abs(x)}` are the only arithmetic helpers
//! - `{if <expr> <op> <number>}…{else}…{/if}` with ops `< <= > >= == !=`
//! - `{warn CODE}…{/warn}` includes text only when that warning is present
//! - `{plural <expr> "one" "many"}`

use crate::display;
use crate::parse::NumberFormat;
use crate::tool::Precision;
use crate::units::{self, Unit};

/// A value the template can show.
#[derive(Clone, Copy, Debug)]
pub struct Val {
    pub value: f64,
    pub unit: Option<&'static Unit>,
    pub precision: Precision,
}

/// Where a template reads values and warnings from.
pub trait Scope {
    fn get(&self, name: &str) -> Option<Val>;
    fn has_warning(&self, code: &str) -> bool;
    fn format(&self) -> NumberFormat;
}

#[derive(Clone, Debug, PartialEq)]
enum Expr {
    Field(String),
    Delta(Box<Expr>, Box<Expr>),
    Abs(Box<Expr>),
}

#[derive(Clone, Debug, PartialEq)]
enum Node {
    Text(String),
    Value {
        expr: Expr,
        unit: Option<String>,
    },
    If {
        expr: Expr,
        op: String,
        rhs: f64,
        then: Vec<Node>,
        els: Vec<Node>,
    },
    Warn {
        code: String,
        body: Vec<Node>,
    },
    Plural {
        expr: Expr,
        one: String,
        many: String,
    },
}

/// The longest rendered sentence allowed.
pub const MAX_CHARS: usize = 280;

fn parse_expr(s: &str) -> Result<Expr, String> {
    let s = s.trim();
    let call = |name: &str| {
        s.strip_prefix(name)
            .and_then(|r| r.strip_prefix('('))
            .and_then(|r| r.strip_suffix(')'))
    };
    if let Some(inner) = call("delta") {
        let mut depth = 0;
        let split = inner.char_indices().find(|(_, c)| {
            match c {
                '(' => depth += 1,
                ')' => depth -= 1,
                ',' if depth == 0 => return true,
                _ => {}
            }
            false
        });
        let (i, _) = split.ok_or_else(|| format!("delta needs two arguments: {s}"))?;
        return Ok(Expr::Delta(
            Box::new(parse_expr(&inner[..i])?),
            Box::new(parse_expr(&inner[i + 1..])?),
        ));
    }
    if let Some(inner) = call("abs") {
        return Ok(Expr::Abs(Box::new(parse_expr(inner)?)));
    }
    if !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Ok(Expr::Field(s.to_owned()));
    }
    Err(format!("not a field or helper: {s}"))
}

fn quoted(s: &str) -> Result<(String, &str), String> {
    let s = s.trim_start();
    let rest = s
        .strip_prefix('"')
        .ok_or_else(|| format!("expected a quoted word in {s}"))?;
    let end = rest.find('"').ok_or("unterminated quote")?;
    Ok((rest[..end].to_owned(), &rest[end + 1..]))
}

/// Parses a template into nodes, or explains what is wrong.
fn parse(template: &str) -> Result<Vec<Node>, String> {
    // Stack of (open tag, nodes, else-nodes) for nested blocks.
    enum Open {
        Root,
        If {
            expr: Expr,
            op: String,
            rhs: f64,
            in_else: bool,
        },
        Warn(String),
    }
    let mut stack: Vec<(Open, Vec<Node>, Vec<Node>)> = vec![(Open::Root, Vec::new(), Vec::new())];
    let push = |stack: &mut Vec<(Open, Vec<Node>, Vec<Node>)>, n: Node| {
        let top = stack.last_mut().expect("root");
        match top.0 {
            Open::If { in_else: true, .. } => top.2.push(n),
            _ => top.1.push(n),
        }
    };
    let mut rest = template;
    while !rest.is_empty() {
        let Some(open) = rest.find('{') else {
            push(&mut stack, Node::Text(rest.to_owned()));
            break;
        };
        if open > 0 {
            push(&mut stack, Node::Text(rest[..open].to_owned()));
        }
        let close = rest[open..].find('}').ok_or("unclosed {")? + open;
        let tag = rest[open + 1..close].trim();
        rest = &rest[close + 1..];
        if let Some(cond) = tag.strip_prefix("if ") {
            let parts: Vec<&str> = cond.split_whitespace().collect();
            let [lhs, op, rhs] = parts[..] else {
                return Err(format!("if needs <field> <op> <number>: {tag}"));
            };
            if !["<", "<=", ">", ">=", "==", "!="].contains(&op) {
                return Err(format!("unknown operator {op}"));
            }
            let rhs: f64 = rhs
                .parse()
                .map_err(|_| format!("if compares with a number: {tag}"))?;
            stack.push((
                Open::If {
                    expr: parse_expr(lhs)?,
                    op: op.to_owned(),
                    rhs,
                    in_else: false,
                },
                Vec::new(),
                Vec::new(),
            ));
        } else if tag == "else" {
            match stack.last_mut() {
                Some((Open::If { in_else, .. }, _, _)) if !*in_else => *in_else = true,
                _ => return Err("else without if".into()),
            }
        } else if tag == "/if" {
            match stack.pop() {
                Some((Open::If { expr, op, rhs, .. }, then, els)) => push(
                    &mut stack,
                    Node::If {
                        expr,
                        op,
                        rhs,
                        then,
                        els,
                    },
                ),
                _ => return Err("/if without if".into()),
            }
        } else if let Some(code) = tag.strip_prefix("warn ") {
            stack.push((Open::Warn(code.trim().to_owned()), Vec::new(), Vec::new()));
        } else if tag == "/warn" {
            match stack.pop() {
                Some((Open::Warn(code), body, _)) => push(&mut stack, Node::Warn { code, body }),
                _ => return Err("/warn without warn".into()),
            }
        } else if let Some(p) = tag.strip_prefix("plural ") {
            let p = p.trim_start();
            let end = p.find(' ').ok_or("plural needs a field and two words")?;
            let expr = parse_expr(&p[..end])?;
            let (one, r) = quoted(&p[end..])?;
            let (many, r) = quoted(r)?;
            if !r.trim().is_empty() {
                return Err(format!("unexpected text after plural: {r}"));
            }
            push(&mut stack, Node::Plural { expr, one, many });
        } else {
            let (e, unit) = match tag.rsplit_once(':') {
                Some((e, u)) if !e.contains('(') || e.ends_with(')') => {
                    (e, Some(u.trim().to_owned()))
                }
                _ => (tag, None),
            };
            push(
                &mut stack,
                Node::Value {
                    expr: parse_expr(e)?,
                    unit,
                },
            );
        }
    }
    match stack.pop() {
        Some((Open::Root, nodes, _)) if stack.is_empty() => Ok(nodes),
        _ => Err("unclosed {if} or {warn}".into()),
    }
}

fn eval(e: &Expr, s: &dyn Scope) -> Option<Val> {
    match e {
        Expr::Field(f) => s.get(f),
        Expr::Abs(x) => eval(x, s).map(|v| Val {
            value: v.value.abs(),
            ..v
        }),
        Expr::Delta(a, b) => {
            let (a, b) = (eval(a, s)?, eval(b, s)?);
            let bv = match (a.unit, b.unit) {
                (Some(ua), Some(ub)) if ua.quantity.dimension() == ub.quantity.dimension() => {
                    units::convert(b.value, ub, ua)
                }
                _ => b.value,
            };
            Some(Val {
                value: a.value - bv,
                ..a
            })
        }
    }
}

fn show(v: Val, forced: Option<&str>, fmt: NumberFormat) -> String {
    let (value, unit) = match (v.unit, forced) {
        (Some(u), Some(sym)) => match units::by_symbol(u.quantity, sym) {
            Some(to) => (units::convert(v.value, u, to), Some(to)),
            None => (v.value, Some(u)),
        },
        (u, _) => (v.value, u),
    };
    match unit {
        Some(u) => display::quantity(value, u.symbol, v.precision, fmt),
        None => display::number(value, v.precision, fmt),
    }
}

fn render_nodes(nodes: &[Node], s: &dyn Scope, out: &mut String) {
    for n in nodes {
        match n {
            Node::Text(t) => out.push_str(t),
            Node::Value { expr, unit } => {
                if let Some(v) = eval(expr, s) {
                    out.push_str(&show(v, unit.as_deref(), s.format()));
                }
            }
            Node::If {
                expr,
                op,
                rhs,
                then,
                els,
            } => {
                let x = eval(expr, s).map(|v| v.value);
                let yes = x.is_some_and(|x| match op.as_str() {
                    "<" => x < *rhs,
                    "<=" => x <= *rhs,
                    ">" => x > *rhs,
                    ">=" => x >= *rhs,
                    "==" => x == *rhs,
                    _ => x != *rhs,
                });
                render_nodes(if yes { then } else { els }, s, out);
            }
            Node::Warn { code, body } => {
                if s.has_warning(code) {
                    render_nodes(body, s, out);
                }
            }
            Node::Plural { expr, one, many } => {
                out.push_str(if eval(expr, s).is_some_and(|v| v.value == 1.0) {
                    one
                } else {
                    many
                });
            }
        }
    }
}

/// Renders a template. A template that does not parse renders as an empty string
/// (the catalog lint rejects such templates before release).
pub fn render(template: &str, scope: &dyn Scope) -> String {
    let mut out = String::new();
    if let Ok(nodes) = parse(template) {
        render_nodes(&nodes, scope, &mut out);
    }
    // Collapse ASCII whitespace only: U+202F groups digits in decimal-comma mode.
    out.split(|c: char| c.is_ascii_whitespace())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn fields(nodes: &[Node], out: &mut Vec<String>) {
    fn expr_fields(e: &Expr, out: &mut Vec<String>) {
        match e {
            Expr::Field(f) => out.push(f.clone()),
            Expr::Abs(x) => expr_fields(x, out),
            Expr::Delta(a, b) => {
                expr_fields(a, out);
                expr_fields(b, out);
            }
        }
    }
    for n in nodes {
        match n {
            Node::Text(_) => {}
            Node::Value { expr, .. } | Node::Plural { expr, .. } => expr_fields(expr, out),
            Node::If {
                expr, then, els, ..
            } => {
                expr_fields(expr, out);
                fields(then, out);
                fields(els, out);
            }
            Node::Warn { body, .. } => fields(body, out),
        }
    }
}

/// Checks a template: it parses, and every field it names is in `known`.
/// Returns the warning codes it references, for registry checks.
pub fn check(template: &str, known: &[&str]) -> Result<Vec<String>, String> {
    let nodes = parse(template)?;
    let mut names = Vec::new();
    fields(&nodes, &mut names);
    if let Some(bad) = names.iter().find(|n| !known.contains(&n.as_str())) {
        return Err(format!("names unknown field {bad}"));
    }
    fn warns(nodes: &[Node], out: &mut Vec<String>) {
        for n in nodes {
            match n {
                Node::Warn { code, body } => {
                    out.push(code.clone());
                    warns(body, out);
                }
                Node::If { then, els, .. } => {
                    warns(then, out);
                    warns(els, out);
                }
                _ => {}
            }
        }
    }
    let mut codes = Vec::new();
    warns(&nodes, &mut codes);
    Ok(codes)
}

/// Flesch-Kincaid grade level, with a vowel-group syllable estimate. Numbers
/// and unit symbols count as one syllable.
pub fn grade(text: &str) -> f64 {
    let sentences = text
        .split(['.', '!', '?'])
        .filter(|s| s.chars().any(char::is_alphanumeric))
        .count()
        .max(1);
    let words: Vec<&str> = text
        .split_whitespace()
        .filter(|w| w.chars().any(char::is_alphanumeric))
        .collect();
    if words.is_empty() {
        return 0.0;
    }
    let syllables: usize = words
        .iter()
        .map(|w| {
            let w: String = w
                .to_lowercase()
                .chars()
                .filter(|c| c.is_ascii_alphabetic())
                .collect();
            if w.is_empty() {
                return 1;
            }
            let mut n = 0;
            let mut prev = false;
            for c in w.chars() {
                let v = "aeiouy".contains(c);
                if v && !prev {
                    n += 1;
                }
                prev = v;
            }
            if w.ends_with('e') && n > 1 && !w.ends_with("le") {
                n -= 1;
            }
            n.max(1)
        })
        .sum();
    0.39 * (words.len() as f64 / sentences as f64) + 11.8 * (syllables as f64 / words.len() as f64)
        - 15.59
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::{Quantity, by_symbol};

    struct S(Vec<(&'static str, Val)>, Vec<&'static str>);
    impl Scope for S {
        fn get(&self, name: &str) -> Option<Val> {
            self.0.iter().find(|(k, _)| *k == name).map(|(_, v)| *v)
        }
        fn has_warning(&self, code: &str) -> bool {
            self.1.contains(&code)
        }
        fn format(&self) -> NumberFormat {
            NumberFormat::DecimalPoint
        }
    }

    fn ft(x: f64) -> Val {
        Val {
            value: x,
            unit: by_symbol(Quantity::Length, "ft"),
            precision: Precision::Decimals(0),
        }
    }

    fn scope(da: f64) -> S {
        S(
            vec![
                ("da", ft(da)),
                ("elevation", ft(5000.0)),
                (
                    "n",
                    Val {
                        value: 1.0,
                        unit: None,
                        precision: Precision::Decimals(0),
                    },
                ),
            ],
            vec![],
        )
    }

    const DA: &str = "Density altitude is {da}, about {abs(delta(da,elevation))} {if delta(da,elevation) < 0}lower{else}higher{/if} than the field.";

    #[test]
    fn density_altitude_sentence_and_conditional() {
        assert_eq!(
            render(DA, &scope(7932.0)),
            "Density altitude is 7,932 ft, about 2,932 ft higher than the field."
        );
        assert_eq!(
            render(DA, &scope(4500.0)),
            "Density altitude is 4,500 ft, about 500 ft lower than the field."
        );
    }

    #[test]
    fn forced_unit_warn_and_plural() {
        let s = S(
            vec![
                ("da", ft(1000.0)),
                (
                    "n",
                    Val {
                        value: 1.0,
                        unit: None,
                        precision: Precision::Decimals(0),
                    },
                ),
            ],
            vec!["ISA_TEMPERATURE_ASSUMED"],
        );
        assert_eq!(render("{da:m}", &s), "305 m");
        assert_eq!(
            render("A{warn ISA_TEMPERATURE_ASSUMED} (ISA assumed){/warn}.", &s),
            "A (ISA assumed)."
        );
        assert_eq!(render("A{warn OTHER} x{/warn}.", &s), "A.");
        assert_eq!(render("{n} {plural n \"image\" \"images\"}", &s), "1 image");
    }

    #[test]
    fn check_reports_problems() {
        assert!(check(DA, &["da", "elevation"]).is_ok());
        assert!(check(DA, &["da"]).unwrap_err().contains("elevation"));
        for bad in [
            "{if da ~ 3}x{/if}",
            "{if da < 3}x",
            "{/if}",
            "{else}",
            "{warn X}x",
            "{delta(da)}",
            "{da",
            "{plural n \"one\"}",
        ] {
            assert!(check(bad, &["da", "n"]).is_err(), "{bad} should fail");
        }
        assert_eq!(
            check("{warn LEGACY_UNIT}x{/warn}", &[]).unwrap(),
            vec!["LEGACY_UNIT"]
        );
    }

    #[test]
    fn readability_grade() {
        assert!(grade("100 kt is 115.078 mph.") < 8.0);
        assert!(
            grade(
                "Density altitude is 7,932 ft, about 2,900 ft higher than the field. Expect a longer takeoff roll and weaker climb."
            ) <= 8.0
        );
        assert!(
            grade(
                "Notwithstanding considerable atmospheric variability, computational methodologies necessitate comprehensive verification."
            ) > 8.0
        );
    }
}
