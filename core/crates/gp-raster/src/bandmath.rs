//! Band math (raster/imagery-indices, "Safe band-math expressions"): a small
//! expression language over named bands.
//!
//! The expression is parsed into a tree and evaluated here, in the core. It is
//! never handed to a JavaScript evaluator, which is how band-math boxes on the
//! web usually work and why they can be turned into a way of running arbitrary
//! code. Only named bands, numbers, the arithmetic and comparison operators, a
//! conditional, and a short list of functions exist; anything else is an
//! unknown identifier and is refused by name.

use gp_base::ErrorCode;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::Reference;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Related, ToolDef};
use libm::{exp, log, sqrt};

/// The arithmetic the expression is evaluated in. The tool's accuracy claim is
/// this standard's: binary64, in the order the expression is written.
pub const IEEE754: Reference = Reference {
    title: "IEEE Standard for Floating-Point Arithmetic (IEEE 754)",
    issuer: "Institute of Electrical and Electronics Engineers",
    year: 2019,
    edition: "IEEE 754-2019",
    locator: "Binary64 arithmetic and the behavior of division by zero and of operations giving infinities or NaN",
    url: "https://standards.ieee.org/ieee/754/6210/",
};

/// The most nodes an expression may hold, so a pathological input cannot make
/// the parser build an unbounded tree.
const MAX_NODES: usize = 1_000;

/// How deeply expressions may nest. The node cap alone does not bound this:
/// a thousand nested parentheses is only a thousand nodes but is a thousand
/// levels of recursion, which overflows the stack before the cap is reached.
const MAX_DEPTH: usize = 64;

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Name(String),
    Op(String),
    Open,
    Close,
    Comma,
}

fn lex(src: &str) -> Result<Vec<Tok>, String> {
    let cs: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || (c == '.' && i + 1 < cs.len() && cs[i + 1].is_ascii_digit())
        {
            let start = i;
            while i < cs.len() && (cs[i].is_ascii_digit() || cs[i] == '.') {
                i += 1;
            }
            // An exponent, as in 2.5e-3.
            if i < cs.len()
                && (cs[i] == 'e' || cs[i] == 'E')
                && i + 1 < cs.len()
                && (cs[i + 1].is_ascii_digit() || cs[i + 1] == '-' || cs[i + 1] == '+')
            {
                i += 2;
                while i < cs.len() && cs[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let text: String = cs[start..i].iter().collect();
            out.push(Tok::Num(
                text.parse()
                    .map_err(|_| format!("{text} is not a number"))?,
            ));
        } else if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < cs.len() && (cs[i].is_ascii_alphanumeric() || cs[i] == '_') {
                i += 1;
            }
            out.push(Tok::Name(cs[start..i].iter().collect()));
        } else if c == '(' {
            out.push(Tok::Open);
            i += 1;
        } else if c == ')' {
            out.push(Tok::Close);
            i += 1;
        } else if c == ',' {
            out.push(Tok::Comma);
            i += 1;
        } else {
            // Two-character operators first, so <= does not lex as < then =.
            let two: String = cs[i..(i + 2).min(cs.len())].iter().collect();
            if ["<=", ">=", "==", "!=", "&&", "||"].contains(&two.as_str()) {
                out.push(Tok::Op(two));
                i += 2;
            } else if "+-*/<>?:".contains(c) {
                out.push(Tok::Op(c.to_string()));
                i += 1;
            } else {
                // Report the whole run the character sits in, so "window.location"
                // is named rather than just the dot inside it.
                let mut start = i;
                while start > 0
                    && (cs[start - 1].is_ascii_alphanumeric()
                        || cs[start - 1] == '_'
                        || cs[start - 1] == '.')
                {
                    start -= 1;
                }
                let mut end = i;
                while end < cs.len()
                    && (cs[end].is_ascii_alphanumeric() || cs[end] == '_' || cs[end] == '.')
                {
                    end += 1;
                }
                let run: String = cs[start..end].iter().collect();
                return Err(format!(
                    "{run} is not part of the expression language: only the bands you name, numbers, + - * /, comparisons, a ? b : c, and abs, sqrt, log, exp, min, max, and clamp are."
                ));
            }
        }
    }
    Ok(out)
}

#[derive(Debug, Clone)]
enum Node {
    Num(f64),
    Band(String),
    Unary(String, Box<Node>),
    Binary(String, Box<Node>, Box<Node>),
    Cond(Box<Node>, Box<Node>, Box<Node>),
    Call(String, Vec<Node>),
}

/// The functions an expression may use, with how many arguments each takes.
const FUNCS: &[(&str, usize)] = &[
    ("abs", 1),
    ("sqrt", 1),
    ("log", 1),
    ("exp", 1),
    ("min", 2),
    ("max", 2),
    ("clamp", 3),
];

struct Parser<'a> {
    toks: &'a [Tok],
    at: usize,
    nodes: usize,
    depth: usize,
    bands: &'a [String],
}

impl Parser<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.at)
    }

    fn eat_op(&mut self, ops: &[&str]) -> Option<String> {
        match self.peek() {
            Some(Tok::Op(o)) if ops.contains(&o.as_str()) => {
                let o = o.clone();
                self.at += 1;
                Some(o)
            }
            _ => None,
        }
    }

    fn count(&mut self) -> Result<(), String> {
        self.nodes += 1;
        if self.nodes > MAX_NODES {
            return Err(format!("the expression is over {MAX_NODES} nodes"));
        }
        Ok(())
    }

    /// `a ? b : c`, right associative, lowest precedence.
    fn expr(&mut self) -> Result<Node, String> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(format!(
                "the expression nests deeper than {MAX_DEPTH} levels"
            ));
        }
        let out = self.expr_inner();
        self.depth -= 1;
        out
    }

    fn expr_inner(&mut self) -> Result<Node, String> {
        let cond = self.or()?;
        if self.eat_op(&["?"]).is_some() {
            let then = self.expr()?;
            if self.eat_op(&[":"]).is_none() {
                return Err("a ? needs a matching :".into());
            }
            let els = self.expr()?;
            self.count()?;
            return Ok(Node::Cond(Box::new(cond), Box::new(then), Box::new(els)));
        }
        Ok(cond)
    }

    fn or(&mut self) -> Result<Node, String> {
        let mut left = self.and()?;
        while let Some(op) = self.eat_op(&["||"]) {
            let right = self.and()?;
            self.count()?;
            left = Node::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn and(&mut self) -> Result<Node, String> {
        let mut left = self.compare()?;
        while let Some(op) = self.eat_op(&["&&"]) {
            let right = self.compare()?;
            self.count()?;
            left = Node::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn compare(&mut self) -> Result<Node, String> {
        let mut left = self.sum()?;
        while let Some(op) = self.eat_op(&["<", "<=", ">", ">=", "==", "!="]) {
            let right = self.sum()?;
            self.count()?;
            left = Node::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn sum(&mut self) -> Result<Node, String> {
        let mut left = self.product()?;
        while let Some(op) = self.eat_op(&["+", "-"]) {
            let right = self.product()?;
            self.count()?;
            left = Node::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn product(&mut self) -> Result<Node, String> {
        let mut left = self.unary()?;
        while let Some(op) = self.eat_op(&["*", "/"]) {
            let right = self.unary()?;
            self.count()?;
            left = Node::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Node, String> {
        if let Some(op) = self.eat_op(&["-", "+"]) {
            let inner = self.unary()?;
            self.count()?;
            return Ok(Node::Unary(op, Box::new(inner)));
        }
        self.atom()
    }

    fn atom(&mut self) -> Result<Node, String> {
        self.count()?;
        match self.peek().cloned() {
            Some(Tok::Num(v)) => {
                self.at += 1;
                Ok(Node::Num(v))
            }
            Some(Tok::Open) => {
                self.at += 1;
                let inner = self.expr()?;
                if !matches!(self.peek(), Some(Tok::Close)) {
                    return Err("a ( needs a matching )".into());
                }
                self.at += 1;
                Ok(inner)
            }
            Some(Tok::Name(name)) => {
                self.at += 1;
                if matches!(self.peek(), Some(Tok::Open)) {
                    let Some((_, arity)) = FUNCS.iter().find(|(f, _)| *f == name) else {
                        return Err(format!(
                            "{name} is not a function here; the functions are {}",
                            FUNCS.iter().map(|(f, _)| *f).collect::<Vec<_>>().join(", ")
                        ));
                    };
                    self.at += 1;
                    let mut args = Vec::new();
                    if !matches!(self.peek(), Some(Tok::Close)) {
                        loop {
                            args.push(self.expr()?);
                            if matches!(self.peek(), Some(Tok::Comma)) {
                                self.at += 1;
                                continue;
                            }
                            break;
                        }
                    }
                    if !matches!(self.peek(), Some(Tok::Close)) {
                        return Err(format!("{name}( needs a matching )"));
                    }
                    self.at += 1;
                    if args.len() != *arity {
                        return Err(format!(
                            "{name} takes {arity} {}, not {}",
                            if *arity == 1 { "argument" } else { "arguments" },
                            args.len()
                        ));
                    }
                    return Ok(Node::Call(name, args));
                }
                if !self.bands.contains(&name) {
                    return Err(format!(
                        "{name} is not one of the bands given ({})",
                        self.bands.join(", ")
                    ));
                }
                Ok(Node::Band(name))
            }
            Some(t) => Err(format!("{t:?} cannot start a value")),
            None => Err("the expression ends early".into()),
        }
    }
}

fn eval(node: &Node, value: &dyn Fn(&str) -> f64) -> f64 {
    let b = |x: bool| if x { 1.0 } else { 0.0 };
    match node {
        Node::Num(v) => *v,
        Node::Band(name) => value(name),
        Node::Unary(op, inner) => {
            let v = eval(inner, value);
            if op == "-" { -v } else { v }
        }
        Node::Binary(op, l, r) => {
            let (a, c) = (eval(l, value), eval(r, value));
            match op.as_str() {
                "+" => a + c,
                "-" => a - c,
                "*" => a * c,
                "/" => a / c,
                "<" => b(a < c),
                "<=" => b(a <= c),
                ">" => b(a > c),
                ">=" => b(a >= c),
                "==" => b(a == c),
                "!=" => b(a != c),
                "&&" => b(a != 0.0 && c != 0.0),
                _ => b(a != 0.0 || c != 0.0),
            }
        }
        Node::Cond(c, t, e) => {
            if eval(c, value) != 0.0 {
                eval(t, value)
            } else {
                eval(e, value)
            }
        }
        Node::Call(name, args) => {
            let a: Vec<f64> = args.iter().map(|n| eval(n, value)).collect();
            match name.as_str() {
                "abs" => a[0].abs(),
                "sqrt" => sqrt(a[0]),
                "log" => log(a[0]),
                "exp" => exp(a[0]),
                "min" => a[0].min(a[1]),
                "max" => a[0].max(a[1]),
                _ => a[0].max(a[1]).min(a[2]),
            }
        }
    }
}

/// Every band name the expression uses, in the order first seen.
fn used(node: &Node, out: &mut Vec<String>) {
    match node {
        Node::Band(n) => {
            if !out.contains(n) {
                out.push(n.clone());
            }
        }
        Node::Unary(_, a) => used(a, out),
        Node::Binary(_, a, b) => {
            used(a, out);
            used(b, out);
        }
        Node::Cond(a, b, c) => {
            used(a, out);
            used(b, out);
            used(c, out);
        }
        Node::Call(_, args) => args.iter().for_each(|a| used(a, out)),
        Node::Num(_) => {}
    }
}

fn count_nodes(node: &Node) -> usize {
    1 + match node {
        Node::Unary(_, a) => count_nodes(a),
        Node::Binary(_, a, b) => count_nodes(a) + count_nodes(b),
        Node::Cond(a, b, c) => count_nodes(a) + count_nodes(b) + count_nodes(c),
        Node::Call(_, args) => args.iter().map(count_nodes).sum(),
        _ => 0,
    }
}

fn run_bandmath(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let expression = ctx.text("expression")?.expect("required");
    let rows = ctx.rows("bands")?;
    let mut names = Vec::new();
    let mut values = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        let name = r
            .get("name")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                ToolError::invalid(&format!("/bands/{i}/name"), "Every band needs a name.")
            })?
            .to_owned();
        if !name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(ToolError::invalid(
                &format!("/bands/{i}/name"),
                format!(
                    "{name} is not a usable band name: use letters, digits, and underscores, starting with a letter."
                ),
            ));
        }
        if names.contains(&name) {
            return Err(ToolError::invalid(
                &format!("/bands/{i}/name"),
                format!("{name} is given twice."),
            ));
        }
        let value = r
            .get("value")
            .and_then(serde_json::Value::as_f64)
            .filter(|v| v.is_finite())
            .ok_or_else(|| {
                ToolError::invalid(
                    &format!("/bands/{i}/value"),
                    "Every band needs a finite numeric value.",
                )
            })?;
        names.push(name);
        values.push(value);
    }
    let toks = lex(&expression)
        .map_err(|m| ToolError::invalid("/expression", m).hint("The language is numbers, the bands you name, + - * /, comparisons, a ? b : c, and abs, sqrt, log, exp, min, max, clamp."))?;
    if toks.is_empty() {
        return Err(ToolError::invalid(
            "/expression",
            "The expression is empty.",
        ));
    }
    let mut parser = Parser {
        toks: &toks,
        at: 0,
        nodes: 0,
        depth: 0,
        bands: &names,
    };
    let tree = parser
        .expr()
        .map_err(|m| ToolError::invalid("/expression", m))?;
    if parser.at != toks.len() {
        return Err(ToolError::invalid(
            "/expression",
            "The expression has something left over after it ends.",
        ));
    }
    let value = |name: &str| {
        names
            .iter()
            .position(|n| n == name)
            .map_or(f64::NAN, |i| values[i])
    };
    let v = eval(&tree, &value);
    if !v.is_finite() {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The expression does not give a finite value here: it divides by zero, or takes a root or a logarithm of a negative number. In an image this pixel is no-data.",
        )
        .at("/expression"));
    }
    let mut mentioned = Vec::new();
    used(&tree, &mut mentioned);
    Ok(Json::obj([
        ("value", Json::Num(v)),
        ("nodes", Json::Num(count_nodes(&tree) as f64)),
        (
            "bands_used",
            Json::Arr(mentioned.into_iter().map(Json::str).collect()),
        ),
    ]))
}

const BAND_ROW: &[Field] = &[
    Field::new(
        "name",
        "Band name",
        "The name the expression uses, like nir",
        Kind::Text { max_len: 32 },
    ),
    Field::new(
        "value",
        "Value",
        "Its value at this pixel, like 0.45",
        Kind::Number {
            min: -1e12,
            max: 1e12,
        },
    ),
];

pub static BANDMATH: ToolDef = ToolDef {
    id: "raster.index.band-math",
    title: "Band math",
    summary: "Evaluates your own expression over named bands: arithmetic, comparisons, a conditional, and a short list of functions, parsed and evaluated in the core rather than by a JavaScript evaluator.",
    aliases: &[
        "band math",
        "raster calculator",
        "custom index",
        "expression calculator",
    ],
    keywords: &[
        "band math",
        "expression",
        "custom index",
        "raster calculator",
        "formula",
        "bands",
    ],
    inputs: &[
        Field::new(
            "expression",
            "Expression",
            "Over the bands you name, like (nir - red) / (nir + red)",
            Kind::Text { max_len: 4_000 },
        )
        .required()
        .core(),
        Field::new(
            "bands",
            "Bands",
            "Each band's name and its value at this pixel, like nir and 0.45",
            Kind::List {
                items: BAND_ROW,
                min: 1,
                max: 64,
            },
        )
        .required()
        .core(),
    ],
    outputs: &[
        Field::new(
            "value",
            "Value",
            "What the expression comes to",
            Kind::Number {
                min: -1e12,
                max: 1e12,
            },
        )
        .precision(Precision::Decimals(6)),
        Field::new(
            "nodes",
            "Expression size",
            "Nodes in the parsed expression, capped at 1,000",
            Kind::Number {
                min: 1.0,
                max: 1000.0,
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "bands_used",
            "Bands used",
            "The bands the expression actually reads",
            Kind::List {
                items: &[Field::new(
                    "name",
                    "Band",
                    "Name",
                    Kind::Text { max_len: 32 },
                )],
                min: 0,
                max: 64,
            },
        ),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "The expression is lexed and parsed into a tree, then evaluated in the core; identifiers are the bands given and the functions abs, sqrt, log, exp, min, max, and clamp, and nothing else",
    accuracy: "Exact double-precision arithmetic in the order the expression is written.",
    when_to_use: "Use this for an index the catalog does not have, or a combination of your own: a ratio, a difference, a threshold, or a masked value written as a conditional. It is also the way to try a formula from a paper before deciding whether it deserves a tool of its own.",
    limitations: "It evaluates one pixel's values, not an image, and it has no no-data handling beyond refusing a result that is not finite. The language is deliberately small: no assignment, no loops, no user functions, and no way to reach anything outside the bands you pass, which is the point. An expression over about a thousand nodes is refused rather than evaluated.",
    references: &[IEEE754],
    examples: &[
        Example {
            id: "primary",
            title: "NDVI written by hand",
            input: r#"{"expression":"(nir - red) / (nir + red)","bands":[{"name":"nir","value":0.45},{"name":"red","value":0.08}]}"#,
            source: "The NDVI definition written as band math: it gives the same 0.6981 as the NDVI tool",
        },
        Example {
            id: "masked",
            title: "A masked ratio",
            input: r#"{"expression":"nir + red > 0 ? (nir - red) / (nir + red) : -999","bands":[{"name":"nir","value":0.0},{"name":"red","value":0.0}]}"#,
            source: "A conditional standing in for no-data, the usual way band math handles a zero denominator",
        },
    ],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "raster.index.ndvi",
            reason: "alternative",
        },
        Related {
            id: "raster.scale.reflectance",
            reason: "parent",
        },
        Related {
            id: "raster.index.evi",
            reason: "alternative",
        },
    ],
    sentence: "The expression comes to {value}.",
    limits: &[("batchRows", 10_000)],
    run: run_bandmath,
    ..ToolDef::BLANK
};

#[cfg(test)]
mod tests {
    use super::*;

    fn at(expr: &str, bands: &[(&str, f64)]) -> Result<f64, String> {
        let names: Vec<String> = bands.iter().map(|(n, _)| (*n).to_owned()).collect();
        let toks = lex(expr)?;
        let mut p = Parser {
            toks: &toks,
            at: 0,
            nodes: 0,
            depth: 0,
            bands: &names,
        };
        let tree = p.expr()?;
        if p.at != toks.len() {
            return Err("trailing tokens".into());
        }
        let value = |name: &str| {
            bands
                .iter()
                .find(|(n, _)| *n == name)
                .map_or(f64::NAN, |(_, v)| *v)
        };
        Ok(eval(&tree, &value))
    }

    #[test]
    fn precedence_and_associativity() {
        let b = &[("a", 2.0), ("b", 3.0), ("c", 4.0)][..];
        assert_eq!(at("a + b * c", b).unwrap(), 14.0);
        assert_eq!(at("(a + b) * c", b).unwrap(), 20.0);
        assert_eq!(at("c / a / a", b).unwrap(), 1.0);
        assert_eq!(at("-a + b", b).unwrap(), 1.0);
        assert_eq!(at("a < b ? c : a", b).unwrap(), 4.0);
        assert_eq!(at("a > b ? c : a", b).unwrap(), 2.0);
        assert_eq!(at("a < b && b < c ? 1 : 0", b).unwrap(), 1.0);
        assert_eq!(at("clamp(10, a, b)", b).unwrap(), 3.0);
        assert_eq!(at("min(a, b) + max(a, b)", b).unwrap(), 5.0);
        assert_eq!(at("sqrt(c) == a ? 1 : 0", b).unwrap(), 1.0);
    }

    #[test]
    fn nothing_outside_the_language_parses() {
        let b = &[("nir", 0.4)][..];
        for expr in [
            "window.location",
            "red",
            "eval(1)",
            "nir = 1",
            "nir;",
            "import('x')",
            "nir ** 2",
            "() => 1",
            "nir + ",
            "(nir",
            "abs(nir, nir)",
            "nir 1",
            "",
        ] {
            assert!(at(expr, b).is_err(), "{expr} should not parse");
        }
        // The message names what it did not recognize.
        let m = at("window.location", b).unwrap_err();
        assert!(m.contains("window"), "{m}");
    }

    #[test]
    fn deep_expressions_are_capped() {
        let b = &[("a", 1.0)][..];
        let deep = format!("{}a{}", "(".repeat(600), ")".repeat(600));
        // Nesting is capped separately from node count: a thousand nested
        // parentheses is few nodes but deep recursion, and used to overflow
        // the stack before the node cap could stop it.
        assert!(
            at(&deep, b).unwrap_err().contains("nests deeper"),
            "deep parens: {}",
            at(&deep, b).unwrap_err()
        );
        let ok_depth = format!("{}a{}", "(".repeat(40), ")".repeat(40));
        assert_eq!(at(&ok_depth, b).unwrap(), 1.0);
        let long = std::iter::repeat_n("a", 900)
            .collect::<Vec<_>>()
            .join(" + ");
        assert!(at(&long, b).unwrap_err().contains("1000 nodes"), "long sum");
        // Something well inside the cap still works.
        let ok = std::iter::repeat_n("a", 50).collect::<Vec<_>>().join(" + ");
        assert_eq!(at(&ok, b).unwrap(), 50.0);
    }
}
