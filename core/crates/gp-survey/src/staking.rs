//! Slope staking (add-survey-suite, survey/earthwork-and-grade, "Slope
//! staking"): where a road template's side slope meets existing ground, found
//! by Brent's method to well within 0.01 of the unit, and written as the stake
//! is marked (`C 4.2 / 28.6 L`).

use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Q, Reference, Related, ToolDef};
use serde_json::{Map, Value};

use crate::{common_unit, len, len_out};

const GHILANI: Reference = Reference {
    title: "Elementary Surveying: An Introduction to Geomatics",
    issuer: "Ghilani, C. D., and Wolf, P. R., Pearson",
    year: 2021,
    edition: "16th edition",
    locator: "Chapter 26 (volumes: slope staking and catch points)",
    url: "https://www.pearson.com/en-us/subject-catalog/p/elementary-surveying-an-introduction-to-geomatics/P200000003148",
};

const GROUND: &[Field] = &[
    len(
        "offset",
        "Offset",
        "From the centerline on this side, like 20 ft",
    )
    .required(),
    len("elevation", "Ground elevation", "Like 104.6 ft").required(),
];

pub static SLOPE_STAKE: ToolDef = ToolDef {
    id: "survey.earthwork.slope-stake",
    stability: gp_base::tool::Stability::Stable,
    title: "Slope stake (catch point)",
    summary: "Where a road template's cut or fill slope meets existing ground on one side, its offset and the cut or fill to the shoulder, found by iteration and written in stake notation.",
    aliases: &["slope staking", "catch point", "slope stake notation", "daylight point"],
    keywords: &["slope stake", "catch point", "cut", "fill", "side slope", "template", "shoulder", "daylight", "earthwork"],
    inputs: &[
        len("grade_elevation", "Grade elevation", "Design elevation at the shoulder (subgrade edge), like 100.00 ft").required().core(),
        len("half_width", "Half width", "Centerline to the shoulder, like 16 ft").required().core(),
        Field::new("ground", "Existing ground", "Offset and elevation pairs across this side, one per line, like 0, 103.2 then 40, 107.8", Kind::List { items: GROUND, min: 2, max: 500 })
            .required()
            .core(),
        Field::new("cut_slope", "Cut slope", "Horizontal for one vertical, like 2 for 2H:1V", Kind::Number { min: 0.01, max: 100.0 }).core(),
        Field::new("fill_slope", "Fill slope", "Horizontal for one vertical, like 3 for 3H:1V", Kind::Number { min: 0.01, max: 100.0 }).core(),
        Field::new("side", "Side", "left or right of the centerline, looking ahead", Kind::Choice(&["left", "right"])),
    ],
    outputs: &[
        len_out("catch_offset", "Catch point offset", "From the centerline"),
        len_out("cut_fill", "Cut or fill", "Ground at the catch point above (cut) or below (fill) the shoulder"),
        Field::new("cut_or_fill", "Cut or fill", "cut or fill", Kind::Text { max_len: 8 }),
        Field::new("stake", "Stake notation", "As marked on the stake: C or F, the depth, and the offset", Kind::Text { max_len: 40 }),
        len_out("ground_at_catch", "Ground elevation at the catch point", "Interpolated from the ground you gave"),
    ],
    errors: &[
        gp_base::ErrorCode::UnitMismatch,
        gp_base::ErrorCode::DidNotConverge,
        gp_base::ErrorCode::InvalidInput,
    ],
    warnings: &["LEGACY_UNIT", "UNIT_ASSUMED"],
    when_to_use: "Use this to set slope stakes: from the design elevation at the shoulder, the half width, the cut and fill slopes, and ground shots across one side of the line, it finds where the side slope meets the ground, how deep the cut or fill is there, and the stake marking, such as C 5.0 / 39.0 R. It replaces the trial-and-error readings in the field.",
    limitations: "The ground is taken as straight between the shots you give, so shots at every break in the ground matter: a ridge or ditch between two shots is invisible. The shots must reach from the shoulder past the catch point. Rounded corners, benches, and ditches in the template are not modeled; each side is one straight slope from the shoulder.",
    model: "Design line from the shoulder: grade + (x − w)/s_cut in cut, grade − (x − w)/s_fill in fill; ground linear between the points given; the catch point solves ground(x) = design(x) by Brent's method to 1e-6 of the unit (Ghilani & Wolf 2021, ch. 26)",
    accuracy: "The iteration converges far inside 0.01 of the unit; the answer is as good as the ground points and the template",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "A cut on ground rising 10% to the right",
        input: r#"{"grade_elevation":"100 ft","half_width":"16 ft","ground":[{"offset":"0 ft","elevation":"102 ft"},{"offset":"60 ft","elevation":"108 ft"}],"cut_slope":2,"fill_slope":3,"side":"right"}"#,
        source: "add-survey-suite catch-point scenario on planar ground",
    }],
    primary_example: "primary",
    visualization: &[Layer { kind: "table-only", map: &[] }],
    related: &[
        Related { id: "survey.earthwork.grade", reason: "next" },
        Related { id: "survey.earthwork.average-end-area", reason: "next" },
        Related { id: "survey.earthwork.section-area", reason: "next" },
    ],
    sentence: "Set the stake at {catch_offset} from the centerline: {stake}.",
    limits: &[("batchRows", 10_000)],
    run: run_slope_stake,
    ..ToolDef::BLANK
};

/// Brent's method on [a, b] with f(a) and f(b) of opposite sign.
fn brent(f: impl Fn(f64) -> f64, mut a: f64, mut b: f64, tol: f64) -> Option<f64> {
    let (mut fa, mut fb) = (f(a), f(b));
    if fa * fb > 0.0 {
        return None;
    }
    if fa.abs() < fb.abs() {
        core::mem::swap(&mut a, &mut b);
        core::mem::swap(&mut fa, &mut fb);
    }
    let (mut c, mut fc) = (a, fa);
    let mut mflag = true;
    let mut d = 0.0;
    for _ in 0..200 {
        if fb == 0.0 || (b - a).abs() < tol {
            return Some(b);
        }
        let mut s = if fa != fc && fb != fc {
            a * fb * fc / ((fa - fb) * (fa - fc))
                + b * fa * fc / ((fb - fa) * (fb - fc))
                + c * fa * fb / ((fc - fa) * (fc - fb))
        } else {
            b - fb * (b - a) / (fb - fa)
        };
        let lo = (3.0 * a + b) / 4.0;
        let between = if lo < b {
            s > lo && s < b
        } else {
            s < lo && s > b
        };
        if !between
            || (mflag && (s - b).abs() >= (b - c).abs() / 2.0)
            || (!mflag && (s - b).abs() >= (c - d).abs() / 2.0)
            || (mflag && (b - c).abs() < tol)
            || (!mflag && (c - d).abs() < tol)
        {
            s = (a + b) / 2.0;
            mflag = true;
        } else {
            mflag = false;
        }
        let fs = f(s);
        d = c;
        c = b;
        fc = fb;
        if fa * fs < 0.0 {
            b = s;
            fb = fs;
        } else {
            a = s;
            fa = fs;
        }
        if fa.abs() < fb.abs() {
            core::mem::swap(&mut a, &mut b);
            core::mem::swap(&mut fa, &mut fb);
        }
    }
    None
}

fn run_slope_stake(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let grade = ctx.req_quantity("grade_elevation")?;
    let w = ctx.req_quantity("half_width")?;
    let rows: Vec<Map<String, Value>> = ctx.rows("ground")?;
    let mut qs = vec![("grade_elevation", grade), ("half_width", w)];
    let mut raw = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let x = ctx
            .row_quantity("ground", i, row, "offset")?
            .expect("required");
        let y = ctx
            .row_quantity("ground", i, row, "elevation")?
            .expect("required");
        qs.push(("offset", x));
        qs.push(("elevation", y));
        raw.push((x, y));
    }
    let u = common_unit(&qs)?;
    let (g, w) = (grade.to(u), w.to(u));
    if w < 0.0 {
        return Err(ToolError::invalid(
            "/half_width",
            "The half width cannot be negative.",
        ));
    }
    let pts: Vec<(f64, f64)> = raw.iter().map(|(x, y)| (x.to(u), y.to(u))).collect();
    if pts.windows(2).any(|p| p[1].0 <= p[0].0) {
        return Err(ToolError::invalid(
            "/ground",
            "List the ground points outward, each offset farther than the last.",
        ));
    }
    let (x0, x1) = (pts[0].0, pts[pts.len() - 1].0);
    if w < x0 || w >= x1 {
        return Err(ToolError::invalid(
            "/ground",
            "The ground points must reach from the shoulder outward past where the slope meets them.",
        ));
    }
    let ground = |x: f64| -> f64 {
        let k = pts
            .windows(2)
            .position(|p| x <= p[1].0)
            .unwrap_or(pts.len() - 2);
        let ((xa, ya), (xb, yb)) = (pts[k], pts[k + 1]);
        ya + (yb - ya) * (x - xa) / (xb - xa)
    };
    // Cut when the ground at the shoulder stands above grade, fill when below.
    let cut = ground(w) > g;
    // Both slopes are read, though only one is used: a slope outside its range
    // is a mistake worth reporting whichever way the section turns out, and a
    // field that is never read is never checked against the range it declares.
    let (cut_given, fill_given) = (ctx.number("cut_slope")?, ctx.number("fill_slope")?);
    let slope = if cut { cut_given } else { fill_given };
    let Some(s) = slope else {
        let field = if cut { "/cut_slope" } else { "/fill_slope" };
        return Err(ToolError::invalid(
            field,
            if cut {
                "The ground is above grade here, so the section is in cut: give the cut slope."
            } else {
                "The ground is below grade here, so the section is in fill: give the fill slope."
            },
        ));
    };
    let design = |x: f64| {
        if cut {
            g + (x - w) / s
        } else {
            g - (x - w) / s
        }
    };
    let f = |x: f64| ground(x) - design(x);
    let x = if f(w) == 0.0 {
        w
    } else {
        brent(f, w, x1, 1e-6).ok_or_else(|| {
            ToolError::new(
                gp_base::ErrorCode::DidNotConverge,
                "The side slope does not meet the ground within the points given: the ground may be steeper than the slope, or the points may stop short.",
            )
            .at("/ground")
        })?
    };
    let depth = (ground(x) - g).abs();
    let side = ctx.choice("side")?.unwrap_or("right");
    let q = |v: f64| Q { value: v, unit: u };
    let stake = format!(
        "{} {:.1} / {:.1} {}",
        if cut { "C" } else { "F" },
        depth,
        x,
        if side == "left" { "L" } else { "R" }
    );
    Ok(Json::obj(vec![
        ("catch_offset", ctx.emit("catch_offset", q(x), u)),
        ("cut_fill", ctx.emit("cut_fill", q(depth), u)),
        (
            "cut_or_fill",
            Json::Str(if cut { "cut" } else { "fill" }.into()),
        ),
        ("stake", Json::Str(stake)),
        (
            "ground_at_catch",
            ctx.emit("ground_at_catch", q(ground(x)), u),
        ),
    ]))
}
