//! Profile slope analysis (add-survey-suite, survey/earthwork-and-grade,
//! "Profile slope analysis"): segment grades along an elevation profile, the
//! steepest and average grade, total climb and descent, and every segment
//! steeper than a chosen limit flagged.

use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use serde_json::{Map, Value};

use crate::{common_unit, len, len_out};

const GHILANI: Reference = Reference {
    title: "Elementary Surveying: An Introduction to Geomatics",
    issuer: "Ghilani, C. D., and Wolf, P. R., Pearson",
    year: 2021,
    edition: "16th edition",
    locator: "Chapter 25 (grades on profiles)",
    url: "https://www.pearson.com/en-us/subject-catalog/p/elementary-surveying-an-introduction-to-geomatics/P200000003148",
};

const PT: &[Field] = &[
    len(
        "distance",
        "Distance",
        "Along the profile from its start, like 150 ft",
    )
    .required(),
    len("elevation", "Elevation", "Like 412.6 ft").required(),
];
const SEG: &[Field] = &[
    len_out("from", "From", "Distance at the segment's start"),
    len_out("to", "To", "Distance at its end"),
    Field::new(
        "grade",
        "Grade",
        "Rise over run, in percent",
        Kind::Number {
            min: -1e9,
            max: 1e9,
        },
    )
    .precision(Precision::Decimals(2)),
    Field::new(
        "over_limit",
        "Over the limit",
        "yes when steeper than the limit either way",
        Kind::Text { max_len: 4 },
    ),
];

pub static PROFILE_GRADES: ToolDef = ToolDef {
    id: "survey.earthwork.profile-grades",
    title: "Profile grade analysis",
    summary: "The grade of every segment along an elevation profile, the steepest and average grade, total climb and descent, and each segment steeper than your limit flagged.",
    aliases: &[
        "profile grades",
        "grade analysis",
        "steepest grade",
        "climb and descent",
    ],
    keywords: &[
        "profile", "grade", "slope", "climb", "descent", "steepest", "limit", "trail", "road",
        "segment",
    ],
    inputs: &[
        Field::new(
            "points",
            "Profile",
            "Distance and elevation, in order, one per line, like 0, 400.0",
            Kind::List {
                items: PT,
                min: 2,
                max: 100_000,
            },
        )
        .required()
        .core(),
        Field::new(
            "limit",
            "Grade limit",
            "Percent; steeper segments, up or down, are flagged, like 8",
            Kind::Number {
                min: 0.0,
                max: 1000.0,
            },
        )
        .core(),
    ],
    outputs: &[
        Field::new(
            "segments",
            "Segments",
            "Each pair of neighboring points",
            Kind::List {
                items: SEG,
                min: 1,
                max: 100_000,
            },
        ),
        Field::new(
            "max_grade",
            "Steepest grade",
            "The largest grade either way, in percent, with its sign",
            Kind::Number {
                min: -1e9,
                max: 1e9,
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "average_grade",
            "Average grade",
            "Net rise over total run, in percent",
            Kind::Number {
                min: -1e9,
                max: 1e9,
            },
        )
        .precision(Precision::Decimals(2)),
        len_out("climb", "Total climb", "Sum of the rises"),
        len_out("descent", "Total descent", "Sum of the drops"),
        Field::new(
            "flagged",
            "Segments over the limit",
            "How many",
            Kind::Number { min: 0.0, max: 1e9 },
        )
        .precision(Precision::Decimals(0))
        .optional(),
    ],
    errors: &[gp_base::ErrorCode::UnitMismatch],
    warnings: &["LEGACY_UNIT", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Grade of each segment = Δelevation / Δdistance × 100; average = (last − first elevation) / total distance × 100; climb and descent are the sums of positive and negative Δelevation",
    accuracy: "Exact for the points given; grades between them are assumed uniform",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "A trail profile against an 8% limit",
        input: r#"{"points":[{"distance":"0 ft","elevation":"400 ft"},{"distance":"100 ft","elevation":"406 ft"},{"distance":"200 ft","elevation":"417 ft"},{"distance":"300 ft","elevation":"414 ft"}],"limit":8}"#,
        source: "add-survey-suite grade-threshold scenario: the 11% segment is flagged",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[Related {
        id: "survey.earthwork.grade",
        reason: "alternative",
    }],
    sentence: "The steepest grade is {max_grade}%, the average {average_grade}%.{if flagged > 0} {flagged} {plural flagged \"segment is\" \"segments are\"} over the limit.{/if}",
    limits: &[("batchRows", 1_000)],
    run: run_profile,
    ..ToolDef::BLANK
};

fn run_profile(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let rows: Vec<Map<String, Value>> = ctx.rows("points")?;
    let mut qs = Vec::new();
    let mut raw = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let d = ctx
            .row_quantity("points", i, row, "distance")?
            .expect("required");
        let e = ctx
            .row_quantity("points", i, row, "elevation")?
            .expect("required");
        qs.push(("distance", d));
        qs.push(("elevation", e));
        raw.push((d, e));
    }
    let u = common_unit(&qs)?;
    let pts: Vec<(f64, f64)> = raw.iter().map(|(d, e)| (d.to(u), e.to(u))).collect();
    if let Some(i) = pts.windows(2).position(|w| w[1].0 <= w[0].0) {
        return Err(ToolError::invalid(
            &format!("/points/{}/distance", i + 1),
            "Each distance must be farther along than the one before.",
        ));
    }
    let limit = ctx.number("limit")?;
    let q = |v: f64| Q { value: v, unit: u };
    let (mut climb, mut descent, mut max, mut flagged) = (0.0, 0.0, 0.0_f64, 0.0);
    let mut segs = Vec::new();
    for w in pts.windows(2) {
        let (dx, dy) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
        let g = dy / dx * 100.0;
        if dy > 0.0 {
            climb += dy
        } else {
            descent -= dy
        }
        if g.abs() > max.abs() {
            max = g;
        }
        let over = limit.is_some_and(|l| g.abs() > l);
        if over {
            flagged += 1.0;
        }
        segs.push(Json::obj(vec![
            ("from", ctx.emit("from", q(w[0].0), u)),
            ("to", ctx.emit("to", q(w[1].0), u)),
            ("grade", Json::Num(g)),
            (
                "over_limit",
                Json::Str(if over { "yes" } else { "no" }.into()),
            ),
        ]));
    }
    let (first, last) = (pts[0], pts[pts.len() - 1]);
    let mut out = vec![
        ("segments", Json::Arr(segs)),
        ("max_grade", Json::Num(max)),
        (
            "average_grade",
            Json::Num((last.1 - first.1) / (last.0 - first.0) * 100.0),
        ),
        ("climb", ctx.emit("climb", q(climb), u)),
        ("descent", ctx.emit("descent", q(descent), u)),
    ];
    if limit.is_some() {
        out.push(("flagged", Json::Num(flagged)));
    }
    Ok(Json::obj(out))
}
