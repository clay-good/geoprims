//! Best runway (add-aviation-suite, wind-and-navigation, "Best runway
//! selection"): every runway end's wind components for one wind, ranked by
//! headwind, with runways beyond your limits flagged and ranked after the
//! ones within them.

use crate::refs::*;
use crate::{
    REFS, RunwayWind, WIND_FIELDS, knots, obj, read_wind, runway_wind, unit, variation, wind,
};
use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Related, Slot, ToolDef};
use gp_base::units::Quantity as QT;

const fn kt(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Speed,
            unit: "kt",
        },
    )
}

const ROW: &[Field] = &[
    Field::new(
        "runway",
        "Runway",
        "The runway end",
        Kind::Text { max_len: 4 },
    ),
    kt("headwind", "Headwind", "Negative for a tailwind").precision(Precision::Decimals(1)),
    kt("crosswind", "Crosswind", "Steady").precision(Precision::Decimals(1)),
    Field::new(
        "crosswind_from",
        "Crosswind from",
        "left, right, none, or either side",
        Kind::Text { max_len: 12 },
    ),
    kt("gust_crosswind", "Gust crosswind", "At the gust speed")
        .precision(Precision::Decimals(1))
        .optional(),
    Field::new(
        "limits",
        "Your limits",
        "Within, near, or beyond each limit you gave",
        Kind::Text { max_len: 120 },
    )
    .optional(),
];

/// At most this many runway ends.
const MAX_ENDS: usize = 24;

pub static BEST_RUNWAY: ToolDef = ToolDef {
    id: "aviation.wind.best-runway",
    title: "Rank runways for the wind",
    summary: "Headwind and crosswind on every runway at an airport for one wind, ranked by headwind, with runways beyond your limits flagged.",
    aliases: &[
        "best runway",
        "which runway",
        "runway selection",
        "active runway",
    ],
    keywords: &[
        "runway",
        "best",
        "headwind",
        "crosswind",
        "tailwind",
        "rank",
        "wind",
    ],
    inputs: &[
        Field::new(
            "runways",
            "Runways",
            "Designators, like 09/27, 18/36 or 27L 27R 36 (T for true)",
            Kind::Text { max_len: 120 },
        )
        .required()
        .core(),
        WIND_FIELDS[0],
        WIND_FIELDS[1],
        WIND_FIELDS[2],
        kt(
            "max_crosswind",
            "Your crosswind limit",
            "Optional, like 15 kt",
        )
        .core(),
        kt("max_tailwind", "Your tailwind limit", "Optional, like 5 kt"),
        Field::new(
            "wind_reference",
            "Wind reference",
            "magnetic (ATIS and tower, the default) or true (METAR)",
            Kind::Choice(REFS),
        ),
        WIND_FIELDS[3],
        WIND_FIELDS[4],
        WIND_FIELDS[5],
    ],
    outputs: &[
        Field::new(
            "best",
            "First choice",
            "The most headwind among runways within your limits",
            Kind::Text { max_len: 4 },
        ),
        kt("best_headwind", "Its headwind", "Negative for a tailwind")
            .precision(Precision::Decimals(1)),
        kt("best_crosswind", "Its crosswind", "Steady").precision(Precision::Decimals(1)),
        Field::new(
            "beyond_limits",
            "Runways beyond your limits",
            "How many runway ends exceed a limit you gave",
            Kind::Number {
                min: 0.0,
                max: MAX_ENDS as f64,
            },
        )
        .precision(Precision::Decimals(0))
        .optional(),
        Field::new(
            "runways",
            "Every runway, in rank order",
            "Components on each runway end",
            Kind::List {
                items: ROW,
                min: 1,
                max: MAX_ENDS,
            },
        ),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &[
        "RUNWAY_HEADING_APPROXIMATE",
        "VARIABLE_WIND",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Each runway end's headwind = W·cos θ and crosswind = W·sin θ, θ the wind's angle off the runway heading (10 × its number). Runways within every limit you gave come first, then by headwind, most first, then by crosswind, least first. A variable wind is judged by its worst case",
    accuracy: "Exact for the entered wind. Runway numbers round the heading to 10°, so components can be off by up to W·sin 5°; a close call between two runways may need the published headings. Planning aid: the runway in use is the one ATC or local procedures assign",
    references: &[PHAK, AC_150_5340, AIM],
    examples: &[Example {
        id: "primary",
        title: "Runways 09/27 and 18/36 with the wind 200° at 12 kt",
        input: r#"{"runways":"09/27, 18/36","wind_direction":"200 deg","wind_speed":"12 kt"}"#,
        source: "add-aviation-suite ranking scenario: runway 18 ranks first, every runway listed",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.wind.runway-components",
            reason: "parent",
        },
        Related {
            id: "aviation.weather.metar-decode",
            reason: "next",
        },
    ],
    sentence: "Runway {best} ranks first, with {abs(best_headwind)} {if best_headwind < 0}tailwind{else}headwind{/if} and {best_crosswind} crosswind.{warn VARIABLE_WIND} These are the worst case for the variable wind.{/warn}",
    limits: &[("batchRows", 10_000)],
    slots: &[
        Slot::new("wind_direction", &["wind", "winds", "from"]),
        Slot::new("wind_speed", &["wind", "winds"]),
        Slot::new("gust", &["gust", "gusts", "gusting", "peak"]),
        Slot::new(
            "max_crosswind",
            &["limit", "max", "maximum", "demonstrated"],
        ),
    ],
    run: run_best_runway,
    ..ToolDef::BLANK
};

/// Runway ends from "09/27, 18/36" or "27L 27R 36": (label, heading, true?).
fn parse_runways(s: &str) -> Result<Vec<(String, f64, bool)>, ToolError> {
    let mut ends: Vec<(String, f64, bool)> = Vec::new();
    for tok in s
        .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .filter(|t| {
            !t.is_empty()
                && !["RWY", "RWYS", "RUNWAY", "RUNWAYS"].contains(&t.to_ascii_uppercase().as_str())
        })
    {
        for end in tok.split('/') {
            let r = wind::parse_designator(end, "/runways")?;
            // "9l" and "RWY27" read as 09L and 27.
            let suffix = end.trim().to_ascii_uppercase();
            let suffix = suffix.trim_start_matches(|c: char| !c.is_ascii_digit());
            let suffix = suffix.trim_start_matches(|c: char| c.is_ascii_digit());
            let n = if r.heading == 0.0 {
                36
            } else {
                (r.heading / 10.0) as u32
            };
            let label = format!("{n:02}{suffix}");
            if ends.iter().any(|(l, _, _)| *l == label) {
                return Err(ToolError::invalid(
                    "/runways",
                    format!("Runway {label} is listed twice."),
                ));
            }
            ends.push((label, r.heading, r.true_ref));
        }
    }
    if ends.is_empty() {
        return Err(ToolError::invalid(
            "/runways",
            "List at least one runway, like 09/27.",
        ));
    }
    if ends.len() > MAX_ENDS {
        return Err(ToolError::invalid(
            "/runways",
            format!("At most {MAX_ENDS} runway ends, please."),
        ));
    }
    Ok(ends)
}

fn run_best_runway(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let ends = parse_runways(&ctx.text("runways")?.expect("required"))?;
    ctx.warnings.push(
        Warning::new(
            "RUNWAY_HEADING_APPROXIMATE",
            "Each runway was taken as 10 times its number. Actual headings can differ by up to 5°, which can decide a close call.",
        )
        .at("/runways"),
    );
    let (w, wind_ref) = read_wind(ctx, "magnetic")?;
    let var = variation(ctx)?;
    let speed = unit(QT::Speed, "kt");
    let limit = |q: Option<gp_base::tool::Q>| q.map(|q| (q.to(speed), q));
    let max_cross = limit(ctx.quantity("max_crosswind")?);
    let max_tail = limit(ctx.quantity("max_tailwind")?);
    let fmt = ctx.options.format;
    let phrase = |value: f64, (lim, q): (f64, gp_base::tool::Q), label: &str| {
        let text = format!(
            "{} {label} limit",
            gp_base::display::quantity(q.value, q.unit.symbol, Precision::Significant(4), fmt)
        );
        gp_base::status::threshold(value, lim, gp_base::status::NEAR_MARGIN, &text)
    };
    // (label, wind, beyond any limit, limits phrase)
    let mut rows: Vec<(String, RunwayWind, bool, Option<String>)> = Vec::new();
    let mut variable = false;
    for (label, heading, true_ref) in ends {
        let rwy_ref = if true_ref { "true" } else { "magnetic" };
        let rw = runway_wind(heading, rwy_ref, &w, wind_ref, var)?;
        variable |= rw.variable;
        let (cross, tail) = rw.worst();
        let mut phrases = Vec::new();
        let mut beyond = false;
        for (value, lim, name) in [
            (cross, max_cross, "crosswind"),
            (tail, max_tail, "tailwind"),
        ] {
            if let Some(lim) = lim {
                beyond |= value > lim.0;
                phrases.push(phrase(value, lim, name));
            }
        }
        rows.push((
            label,
            rw,
            beyond,
            (!phrases.is_empty()).then(|| phrases.join("; ")),
        ));
    }
    if variable {
        ctx.warnings.push(Warning::new(
            "VARIABLE_WIND",
            "The wind direction varies, so each runway's components are the worst case over its range.",
        ));
    }
    // Within limits first, then most headwind, then least crosswind; stable for ties.
    rows.sort_by(|a, b| {
        a.2.cmp(&b.2)
            .then(b.1.head.total_cmp(&a.1.head))
            .then(a.1.cross.total_cmp(&b.1.cross))
    });
    let beyond = rows.iter().filter(|r| r.2).count();
    let (best, bw) = (&rows[0].0, &rows[0].1);
    let mut out = vec![
        ("best", Json::str(best)),
        ("best_headwind", ctx.out("best_headwind", knots(bw.head))),
        ("best_crosswind", ctx.out("best_crosswind", knots(bw.cross))),
    ];
    if max_cross.is_some() || max_tail.is_some() {
        out.push(("beyond_limits", Json::Num(beyond as f64)));
    }
    let list = rows
        .iter()
        .map(|(label, rw, _, limits)| {
            let mut r = vec![
                ("runway", Json::str(label)),
                ("headwind", knots(rw.head).to_json()),
                ("crosswind", knots(rw.cross).to_json()),
                ("crosswind_from", Json::str(rw.from)),
            ];
            if let Some((gx, _)) = rw.gust {
                r.push(("gust_crosswind", knots(gx).to_json()));
            }
            if let Some(l) = limits {
                r.push(("limits", Json::str(l)));
            }
            Json::obj(r)
        })
        .collect();
    out.push(("runways", Json::Arr(list)));
    Ok(obj(out))
}
