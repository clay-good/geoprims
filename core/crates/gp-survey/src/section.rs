//! Cross-section areas from points (add-survey-suite, survey/earthwork-and-
//! grade, "Cross-section area from points"): cut and fill areas between an
//! existing-ground line and a design template, each split where the two cross,
//! with the grade (daylight) points where they meet.

use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use serde_json::{Map, Value};

use crate::{common_unit, len, len_out, unit};

const GHILANI: Reference = Reference {
    title: "Elementary Surveying: An Introduction to Geomatics",
    issuer: "Ghilani, C. D., and Wolf, P. R., Pearson",
    year: 2021,
    edition: "16th edition",
    locator: "Chapter 26 (volumes: cross-section areas by coordinates, mixed sections)",
    url: "https://www.pearson.com/en-us/subject-catalog/p/elementary-surveying-an-introduction-to-geomatics/P200000003148",
};

const PT: &[Field] = &[
    len(
        "offset",
        "Offset",
        "From the centerline, left negative, like -30 ft",
    )
    .required(),
    len("elevation", "Elevation", "Like 102.5 ft").required(),
];
const GRADE_POINT: &[Field] = &[len_out(
    "offset",
    "Offset",
    "Where the ground meets the template",
)];

pub static SECTION_AREA: ToolDef = ToolDef {
    id: "survey.earthwork.section-area",
    title: "Cross-section cut and fill areas",
    summary: "Cut and fill areas between the existing ground and a design template across one station, each split where the two cross, with the grade points where they meet.",
    aliases: &[
        "cross section area",
        "cut and fill area",
        "end area",
        "mixed section",
    ],
    keywords: &[
        "cross section",
        "cut",
        "fill",
        "end area",
        "template",
        "ground",
        "grade point",
        "daylight",
        "earthwork",
    ],
    inputs: &[
        Field::new(
            "ground",
            "Existing ground",
            "Offset and elevation, left to right, one per line, like -40, 104.2",
            Kind::List {
                items: PT,
                min: 2,
                max: 2000,
            },
        )
        .required()
        .core(),
        Field::new(
            "template",
            "Design template",
            "Offset and elevation, left to right, including the side slopes, like -28, 100.0",
            Kind::List {
                items: PT,
                min: 2,
                max: 2000,
            },
        )
        .required()
        .core(),
    ],
    outputs: &[
        Field::new(
            "cut_area",
            "Cut area",
            "Ground above the template",
            Kind::Quantity {
                q: QT::Area,
                unit: "ft2",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "fill_area",
            "Fill area",
            "Ground below the template",
            Kind::Quantity {
                q: QT::Area,
                unit: "ft2",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "grade_points",
            "Grade points",
            "Where the ground crosses the template",
            Kind::List {
                items: GRADE_POINT,
                min: 0,
                max: 2000,
            },
        ),
        len_out("left_limit", "Left limit", "Where both lines begin"),
        len_out("right_limit", "Right limit", "Where both lines end"),
    ],
    errors: &[gp_base::ErrorCode::UnitMismatch],
    warnings: &["LEGACY_UNIT", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Both lines are linear between their points; over the span both cover, the area between them is integrated interval by interval, each interval split where the difference changes sign (Ghilani & Wolf 2021, ch. 26)",
    accuracy: "Exact for the lines given; the areas are as good as the ground shots and the template",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "A mixed section: cut on the left, fill on the right",
        input: r#"{"ground":[{"offset":"-40 ft","elevation":"106 ft"},{"offset":"40 ft","elevation":"96 ft"}],"template":[{"offset":"-40 ft","elevation":"100 ft"},{"offset":"40 ft","elevation":"100 ft"}]}"#,
        source: "add-survey-suite mixed-section scenario",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "survey.earthwork.average-end-area",
            reason: "next",
        },
        Related {
            id: "survey.earthwork.slope-stake",
            reason: "alternative",
        },
    ],
    sentence: "The section has {cut_area} of cut and {fill_area} of fill.",
    limits: &[("batchRows", 10_000)],
    run: run_section,
    ..ToolDef::BLANK
};

fn read(
    ctx: &mut Ctx,
    list: &str,
    qs: &mut Vec<(&'static str, Q)>,
) -> Result<Vec<(Q, Q)>, ToolError> {
    let rows: Vec<Map<String, Value>> = ctx.rows(list)?;
    let mut out = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let x = ctx.row_quantity(list, i, row, "offset")?.expect("required");
        let y = ctx
            .row_quantity(list, i, row, "elevation")?
            .expect("required");
        qs.push(("offset", x));
        qs.push(("elevation", y));
        out.push((x, y));
    }
    Ok(out)
}

fn at(pts: &[(f64, f64)], x: f64) -> f64 {
    let k = pts
        .windows(2)
        .position(|p| x <= p[1].0)
        .unwrap_or(pts.len() - 2);
    let ((xa, ya), (xb, yb)) = (pts[k], pts[k + 1]);
    ya + (yb - ya) * (x - xa) / (xb - xa)
}

fn run_section(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let mut qs = Vec::new();
    let g = read(ctx, "ground", &mut qs)?;
    let t = read(ctx, "template", &mut qs)?;
    let u = common_unit(&qs)?;
    let pts =
        |v: &[(Q, Q)]| -> Vec<(f64, f64)> { v.iter().map(|(x, y)| (x.to(u), y.to(u))).collect() };
    let (g, t) = (pts(&g), pts(&t));
    for (name, p) in [("/ground", &g), ("/template", &t)] {
        if p.windows(2).any(|w| w[1].0 <= w[0].0) {
            return Err(ToolError::invalid(
                name,
                "List the points left to right, each offset greater than the last.",
            ));
        }
    }
    let lo = g[0].0.max(t[0].0);
    let hi = g[g.len() - 1].0.min(t[t.len() - 1].0);
    if hi <= lo {
        return Err(ToolError::invalid(
            "/template",
            "The ground and the template do not overlap across the section.",
        ));
    }
    // Every offset where either line bends, inside the span both cover.
    let mut xs: Vec<f64> = g
        .iter()
        .chain(t.iter())
        .map(|p| p.0)
        .filter(|x| *x > lo && *x < hi)
        .collect();
    xs.push(lo);
    xs.push(hi);
    xs.sort_by(f64::total_cmp);
    xs.dedup();
    let d = |x: f64| at(&g, x) - at(&t, x);
    let (mut cut, mut fill) = (0.0, 0.0);
    let mut grade = Vec::new();
    for w in xs.windows(2) {
        let (x0, x1) = (w[0], w[1]);
        let (d0, d1) = (d(x0), d(x1));
        if d0 == 0.0 && (grade.last() != Some(&x0)) {
            grade.push(x0);
        }
        if d0 * d1 < 0.0 {
            // The lines cross inside this interval: split it at the grade point.
            let xz = x0 + (x1 - x0) * d0 / (d0 - d1);
            grade.push(xz);
            let (a0, a1) = (d0 * (xz - x0) / 2.0, d1 * (x1 - xz) / 2.0);
            for a in [a0, a1] {
                if a > 0.0 { cut += a } else { fill -= a }
            }
        } else {
            let a = (d0 + d1) * (x1 - x0) / 2.0;
            if a > 0.0 { cut += a } else { fill -= a }
        }
    }
    if d(hi) == 0.0 && grade.last() != Some(&hi) {
        grade.push(hi);
    }
    // In the square of the entered unit.
    let per_m = Q {
        value: 1.0,
        unit: u,
    }
    .to(unit(QT::Length, "m"));
    let m2 = unit(QT::Area, "m2");
    let au = unit(
        QT::Area,
        match u.symbol {
            "ft" => "ft2",
            "ftUS" => "ftUS2",
            _ => "m2",
        },
    );
    let area = |v: f64| Q {
        value: v * per_m * per_m,
        unit: m2,
    };
    let q = |v: f64| Q { value: v, unit: u };
    Ok(Json::obj(vec![
        ("cut_area", ctx.emit("cut_area", area(cut), au)),
        ("fill_area", ctx.emit("fill_area", area(fill), au)),
        (
            "grade_points",
            Json::Arr(
                grade
                    .iter()
                    .map(|&x| Json::obj(vec![("offset", ctx.emit("offset", q(x), u))]))
                    .collect(),
            ),
        ),
        ("left_limit", ctx.emit("left_limit", q(lo), u)),
        ("right_limit", ctx.emit("right_limit", q(hi), u)),
    ]))
}
