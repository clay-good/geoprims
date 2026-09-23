//! Grades and slopes (add-survey-suite, survey/earthwork-and-grade): one grade
//! as percent, degrees, per mille, and ratios labeled H:V and V:H, with rise,
//! run, and slope length from any two. A bare ratio like `3:1` is ambiguous,
//! so it is refused with both readings shown until the reader chooses.

use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use libm::{atan, atan2, hypot, sqrt, tan};

use crate::{common_unit, len, len_out};

const GHILANI: Reference = Reference {
    title: "Elementary Surveying: An Introduction to Geomatics",
    issuer: "Ghilani, C. D., and Wolf, P. R., Pearson",
    year: 2018,
    edition: "15th edition",
    locator: "Grades and slopes in route surveying and construction layout",
    url: "https://www.pearson.com/en-us/subject-catalog/p/elementary-surveying-an-introduction-to-geomatics/P200000003148",
};

/// A grade as rise over run, from its written form. A bare `a:b` needs a
/// convention; `3H:1V` and `1V:3H` carry their own.
fn parse(text: &str, convention: Option<&str>) -> Result<f64, ToolError> {
    let t = text.trim().replace(' ', "");
    let num = |s: &str| -> Option<f64> {
        s.replace(',', "")
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
    };
    let bad = || {
        ToolError::invalid(
            "/grade",
            format!("\"{text}\" is not a grade. Write 5%, 3H:1V, 1V:3H, 2.86°, 50‰, or 0.05."),
        )
    };
    if let Some(p) = t.strip_suffix('%') {
        return num(p).map(|v| v / 100.0).ok_or_else(bad);
    }
    if let Some(p) = t.strip_suffix('‰') {
        return num(p).map(|v| v / 1000.0).ok_or_else(bad);
    }
    if let Some(p) = t.strip_suffix('°').or_else(|| t.strip_suffix("deg")) {
        let d = num(p).ok_or_else(bad)?;
        if !(-90.0 < d && d < 90.0) {
            return Err(ToolError::invalid(
                "/grade",
                "A slope angle is between -90° and 90°.",
            ));
        }
        return Ok(tan(d.to_radians()));
    }
    if let Some((a, b)) = t.split_once(':') {
        let up = |s: &str| s.to_ascii_uppercase();
        let (a, b) = (up(a), up(b));
        let side = |s: &str| -> Option<(f64, char)> {
            let (v, tag) = match s.chars().last()? {
                c @ ('H' | 'V') => (&s[..s.len() - 1], Some(c)),
                _ => (s, None),
            };
            num(v).map(|v| (v, tag.unwrap_or('?')))
        };
        let ((x, tx), (y, ty)) = (side(&a).ok_or_else(bad)?, side(&b).ok_or_else(bad)?);
        let hv = match (tx, ty, convention) {
            ('H', 'V', _) => true,
            ('V', 'H', _) => false,
            ('?', '?', Some("H:V")) => true,
            ('?', '?', Some("V:H")) => false,
            ('?', '?', None) => {
                let (h1, v1) = (atan2(y, x).to_degrees(), 100.0 * y / x);
                let (h2, v2) = (atan2(x, y).to_degrees(), 100.0 * x / y);
                return Err(ToolError::invalid(
                    "/convention",
                    format!(
                        "\"{text}\" could mean {x}H:{y}V (about {h1:.2}°, {v1:.1}%) or {x}V:{y}H (about {h2:.2}°, {v2:.1}%). Choose H:V or V:H."
                    ),
                )
                .hint("Set the ratio convention, or write the ratio with its letters, like 3H:1V."));
            }
            _ => return Err(bad()),
        };
        let (h, v) = if hv { (x, y) } else { (y, x) };
        if h <= 0.0 {
            return Err(ToolError::invalid(
                "/grade",
                "A ratio's horizontal part must be positive.",
            ));
        }
        return Ok(v / h);
    }
    // A plain number is a decimal grade (0.05 is 5%).
    num(&t).ok_or_else(bad)
}

pub static GRADE: ToolDef = ToolDef {
    id: "survey.earthwork.grade",
    title: "Side slope and grade (H:V)",
    summary: "A construction side slope or grade with its ratio labeled H:V and V:H, refusing an unlabeled 3:1, with rise, run, or slope length from any two; for a plain unit conversion of a slope, use the slope converter.",
    aliases: &["side slope ratio", "H:V slope", "rise run slope length"],
    keywords: &[
        "side slope",
        "H:V",
        "V:H",
        "ratio",
        "rise",
        "run",
        "slope length",
        "embankment",
        "cut slope",
        "fill slope",
    ],
    inputs: &[
        Field::new(
            "grade",
            "Grade",
            "Like 5%, 3H:1V, 2.86°, 50‰, or 0.05",
            Kind::Text { max_len: 24 },
        )
        .core(),
        Field::new(
            "convention",
            "Ratio convention",
            "For a bare ratio like 3:1: H:V (run first) or V:H (rise first)",
            Kind::Choice(&["H:V", "V:H"]),
        )
        .core(),
        len("rise", "Rise", "Vertical change, like 5 ft").core(),
        len("run", "Run", "Horizontal distance, like 100 ft").core(),
        len(
            "slope_length",
            "Slope length",
            "Along the slope, like 100.125 ft",
        ),
    ],
    outputs: &[
        Field::new(
            "percent",
            "Percent grade",
            "Rise over run × 100",
            Kind::Number {
                min: -1e9,
                max: 1e9,
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "degrees",
            "Slope angle",
            "atan(rise / run)",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(4)),
        Field::new(
            "per_mille",
            "Per mille",
            "Rise over run × 1,000",
            Kind::Number {
                min: -1e9,
                max: 1e9,
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "ratio_hv",
            "Ratio H:V",
            "Horizontal for one vertical",
            Kind::Text { max_len: 24 },
        ),
        Field::new(
            "ratio_vh",
            "Ratio V:H",
            "Vertical for one horizontal",
            Kind::Text { max_len: 24 },
        ),
        len_out("rise", "Rise", "Vertical change").optional(),
        len_out("run", "Run", "Horizontal distance").optional(),
        len_out("slope_length", "Slope length", "Along the slope").optional(),
    ],
    errors: &[gp_base::ErrorCode::UnitMismatch],
    warnings: &["LEGACY_UNIT", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Grade g = rise / run; percent = 100·g, per mille = 1,000·g, angle = atan g, ratio H:V = (1/g):1 and V:H = g:1; slope length = √(rise² + run²)",
    accuracy: "Exact",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "A 3H:1V side slope",
        input: r#"{"grade":"3H:1V"}"#,
        source: "add-survey-suite scenario: 3H:1V is about 18.43° and 33.3%",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "survey.curves.vertical-curve",
            reason: "next",
        },
        Related {
            id: "survey.earthwork.average-end-area",
            reason: "next",
        },
    ],
    sentence: "That grade is {percent}%, a slope of {degrees}, or {ratio_hv}.",
    limits: &[("batchRows", 10_000)],
    run: run_grade,
    ..ToolDef::BLANK
};

fn fmt_ratio(first: f64, a: char, b: char) -> String {
    let v = (first * 1000.0).round() / 1000.0;
    format!("{v}{a}:1{b}")
}

fn run_grade(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let conv = ctx.choice("convention")?;
    let grade = ctx.text("grade")?.map(|g| parse(&g, conv)).transpose()?;
    let rise = ctx.quantity("rise")?;
    let run = ctx.quantity("run")?;
    let slope = ctx.quantity("slope_length")?;
    let named: Vec<(&str, Q)> = [("rise", rise), ("run", run), ("slope_length", slope)]
        .into_iter()
        .filter_map(|(n, q)| q.map(|q| (n, q)))
        .collect();
    let u = common_unit(&named)?;
    let (r, h, s) = (
        rise.map(|q| q.to(u)),
        run.map(|q| q.to(u)),
        slope.map(|q| q.to(u)),
    );
    // The grade, from itself or from two lengths; then the lengths it leaves open.
    let g = match (grade, r, h, s) {
        (Some(g), ..) => g,
        (None, Some(r), Some(h), _) if h > 0.0 => r / h,
        (None, Some(r), None, Some(s)) if s > r.abs() => r / sqrt(s * s - r * r),
        (None, None, Some(h), Some(s)) if s > h && h > 0.0 => sqrt(s * s - h * h) / h,
        _ => {
            return Err(ToolError::invalid(
                "/grade",
                "Give a grade, or two of rise, run, and slope length (the slope length longer than either).",
            ));
        }
    };
    let q = |x: f64| Q { value: x, unit: u };
    let mut out = vec![
        ("percent", Json::Num(g * 100.0)),
        (
            "degrees",
            ctx.out(
                "degrees",
                Q {
                    value: atan(g).to_degrees(),
                    unit: crate::unit(QT::Angle, "deg"),
                },
            ),
        ),
        ("per_mille", Json::Num(g * 1000.0)),
        (
            "ratio_hv",
            Json::Str(if g == 0.0 {
                "level".into()
            } else {
                fmt_ratio(1.0 / g.abs(), 'H', 'V')
            }),
        ),
        ("ratio_vh", Json::Str(fmt_ratio(g.abs(), 'V', 'H'))),
    ];
    let (rr, hh) = match (r, h, s) {
        (Some(r), Some(h), _) => (Some(r), Some(h)),
        (Some(r), None, _) if g != 0.0 => (Some(r), Some(r / g)),
        (None, Some(h), _) => (Some(h * g), Some(h)),
        (None, None, Some(s)) => {
            let h = s / hypot(1.0, g);
            (Some(h * g), Some(h))
        }
        (Some(r), None, Some(s)) => (Some(r), Some(sqrt((s * s - r * r).max(0.0)))),
        _ => (None, None),
    };
    if let (Some(rr), Some(hh)) = (rr, hh) {
        out.push(("rise", ctx.emit("rise", q(rr), u)));
        out.push(("run", ctx.emit("run", q(hh), u)));
        out.push((
            "slope_length",
            ctx.emit("slope_length", q(hypot(rr, hh)), u),
        ));
    }
    Ok(Json::obj(out))
}
