//! Sight distance on vertical curves (add-survey-suite, survey/alignment-
//! curves, "Sight distance references"): the minimum crest or sag curve
//! length for a sight distance from the standard formulas, with every design
//! value a user input citing its source; no design table is reproduced.

use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use gp_geo::point::plain_angle;
use libm::{sqrt, tan};

use crate::{common_unit, len, len_out};

const GHILANI: Reference = Reference {
    title: "Elementary Surveying: An Introduction to Geomatics",
    issuer: "Ghilani, C. D., and Wolf, P. R., Pearson",
    year: 2018,
    edition: "15th edition",
    locator: "Chapter 25 (vertical curves: sight distance on crest and sag curves)",
    url: "https://www.pearson.com/en-us/subject-catalog/p/elementary-surveying-an-introduction-to-geomatics/P200000003237",
};

pub static SIGHT_DISTANCE: ToolDef = ToolDef {
    id: "survey.curves.sight-distance",
    title: "Vertical curve length for sight distance",
    summary: "The shortest crest or sag vertical curve that gives a sight distance, from the standard formulas with your eye, object, or headlight heights, stating which case applies; design values come from your design manual.",
    aliases: &[
        "stopping sight distance curve length",
        "crest curve length",
        "sag curve length",
        "SSD vertical curve",
    ],
    keywords: &[
        "sight distance",
        "SSD",
        "crest",
        "sag",
        "headlight",
        "vertical curve",
        "K value",
        "minimum length",
    ],
    inputs: &[
        Field::new(
            "curve",
            "Curve",
            "crest or sag",
            Kind::Choice(&["crest", "sag"]),
        )
        .required()
        .core(),
        len(
            "sight_distance",
            "Sight distance S",
            "From your design manual's table, like 400 ft",
        )
        .required()
        .core(),
        Field::new(
            "grade_change",
            "Grade change A",
            "The algebraic difference of the grades in percent, like 5",
            Kind::Number {
                min: 0.0,
                max: 100.0,
            },
        )
        .required()
        .core(),
        len(
            "eye_height",
            "Eye height h1",
            "Crest: the driver's eye height your manual uses, like 3.5 ft",
        )
        .core(),
        len(
            "object_height",
            "Object height h2",
            "Crest: the object height your manual uses, like 2.0 ft",
        )
        .core(),
        len(
            "headlight_height",
            "Headlight height H",
            "Sag: the headlight height your manual uses, like 2 ft",
        ),
        Field::new(
            "divergence",
            "Headlight divergence β",
            "Sag: the upward spread of the beam your manual uses, like 1°",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .angle_range("unbounded"),
        Field::new(
            "design_k",
            "Design K",
            "Optional: the K from your design manual's table, to compare, like 44",
            Kind::Number {
                min: 0.0,
                max: 10_000.0,
            },
        ),
        Field::new(
            "design_k_source",
            "Design K source",
            "Title, edition, and table number, like AASHTO Green Book, 7th ed., Table 3-34",
            Kind::Text { max_len: 120 },
        ),
    ],
    outputs: &[
        len_out(
            "min_length",
            "Minimum curve length",
            "The shortest curve that gives the sight distance",
        ),
        Field::new(
            "case",
            "Case",
            "Whether the sight line is longer or shorter than the curve",
            Kind::Text { max_len: 40 },
        ),
        Field::new(
            "k",
            "K for that length",
            "L / A",
            Kind::Number { min: 0.0, max: 1e9 },
        )
        .precision(Precision::Decimals(1)),
        len_out("design_length", "Length from the design K", "K × A").optional(),
        Field::new(
            "design_source",
            "Design K source",
            "As you cited it",
            Kind::Text { max_len: 120 },
        )
        .optional(),
    ],
    errors: &[gp_base::ErrorCode::UnitMismatch],
    warnings: &["LEGACY_UNIT", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Crest: L = AS²/(200(√h1 + √h2)²) when S < L, else L = 2S − 200(√h1 + √h2)²/A. Sag (headlight): L = AS²/(200(H + S tan β)) when S < L, else L = 2S − 200(H + S tan β)/A (Ghilani & Wolf 2018, ch. 25). Design values are inputs: no design table is reproduced",
    accuracy: "Exact for the formulas; the design values, and whether they apply to your road, come from your design manual",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "A crest curve for 400 ft of sight with A = 5%",
        input: r#"{"curve":"crest","sight_distance":"400 ft","grade_change":5,"eye_height":"3.5 ft","object_height":"2.0 ft"}"#,
        source: "add-survey-suite crest SSD scenario: about 368.3 ft, S > L",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[Related {
        id: "survey.curves.vertical-curve",
        reason: "next",
    }],
    sentence: "The curve must be at least {min_length} long ({case}).",
    limits: &[("batchRows", 10_000)],
    run: run_sight,
    ..ToolDef::BLANK
};

fn run_sight(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let s = ctx.req_quantity("sight_distance")?;
    let (h1, h2, hl) = (
        ctx.quantity("eye_height")?,
        ctx.quantity("object_height")?,
        ctx.quantity("headlight_height")?,
    );
    let mut qs = vec![("sight_distance", s)];
    for (n, q) in [
        ("eye_height", h1),
        ("object_height", h2),
        ("headlight_height", hl),
    ] {
        qs.extend(q.map(|q| (n, q)));
    }
    let u = common_unit(&qs)?;
    let s = s.to(u);
    let a = ctx.number("grade_change")?.expect("required");
    if s <= 0.0 || a <= 0.0 {
        return Err(ToolError::invalid(
            "/grade_change",
            "The sight distance and the grade change must be positive.",
        ));
    }
    let crest = ctx.choice("curve")? == Some("crest");
    // The sight line's "height term": (√h1 + √h2)² on a crest, H + S tan β on a sag.
    let c = if crest {
        let (Some(h1), Some(h2)) = (h1, h2) else {
            return Err(ToolError::invalid(
                "/eye_height",
                "A crest curve needs the eye and object heights from your design manual.",
            ));
        };
        let r = sqrt(h1.to(u)) + sqrt(h2.to(u));
        r * r
    } else {
        let Some(h) = hl else {
            return Err(ToolError::invalid(
                "/headlight_height",
                "A sag curve needs the headlight height from your design manual.",
            ));
        };
        let beta = plain_angle(ctx, "divergence")?.ok_or_else(|| {
            ToolError::invalid(
                "/divergence",
                "A sag curve needs the headlight divergence from your design manual.",
            )
        })?;
        h.to(u) + s * tan(beta.to_radians())
    };
    // Zero or negative heights leave no sight line to design for.
    if !(c > 0.0 && c.is_finite()) {
        return Err(ToolError::invalid(
            if crest {
                "/eye_height"
            } else {
                "/headlight_height"
            },
            "The heights must be positive, like a 3.5 ft eye and a 2.0 ft object.",
        ));
    }
    let long = a * s * s / (200.0 * c); // S < L
    let (l, case) = if long >= s {
        (long, "S < L: the sight line lies within the curve")
    } else {
        (
            (2.0 * s - 200.0 * c / a).max(0.0),
            "S > L: the sight line extends past the curve",
        )
    };
    let q = |v: f64| Q { value: v, unit: u };
    let mut out = vec![
        ("min_length", ctx.emit("min_length", q(l), u)),
        ("case", Json::Str(case.into())),
        ("k", Json::Num(l / a)),
    ];
    if let Some(k) = ctx.number("design_k")? {
        out.push(("design_length", ctx.emit("design_length", q(k * a), u)));
        let src = ctx.text("design_k_source")?.ok_or_else(|| {
            ToolError::invalid(
                "/design_k_source",
                "Cite where the design K comes from: its title, edition, and table number.",
            )
        })?;
        out.push(("design_source", Json::Str(src)));
    }
    Ok(Json::obj(out))
}
