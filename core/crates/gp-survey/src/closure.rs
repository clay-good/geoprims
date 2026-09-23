//! Angular closure (add-survey-suite, survey/cogo-and-traverse, "Traverse
//! closure": the angular misclosure against (n − 2) × 180° for a loop's
//! interior angles, compared with an allowable K·√n, and the angles adjusted).

use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use gp_geo::dms::{self, Axis, Style};
use libm::sqrt;

use crate::unit;

const GHILANI: Reference = Reference {
    title: "Elementary Surveying: An Introduction to Geomatics",
    issuer: "Ghilani, C. D., and Wolf, P. R., Pearson",
    year: 2021,
    edition: "16th edition",
    locator: "Chapter 10 (traverse computations: angular misclosure and balancing angles)",
    url: "https://www.pearson.com/en-us/subject-catalog/p/elementary-surveying-an-introduction-to-geomatics/P200000003148",
};

const ANGLE: &[Field] = &[Field::new(
    "angle",
    "Angle",
    "Like 108°00'05\" or 108-00-05",
    Kind::Text { max_len: 24 },
)
.required()];
const ADJUSTED: &[Field] = &[Field::new(
    "angle",
    "Adjusted angle",
    "With its share of the misclosure removed",
    Kind::Text { max_len: 24 },
)];

pub static ANGULAR: ToolDef = ToolDef {
    id: "survey.cogo.angular-closure",
    title: "Angular closure of a loop traverse",
    summary: "How far a loop traverse's measured angles miss their geometric sum, (n − 2) × 180° for interior angles, against an allowable K√n, with the angles balanced.",
    aliases: &[
        "angular misclosure",
        "balance angles",
        "interior angles sum",
        "angle closure",
    ],
    keywords: &[
        "angular misclosure",
        "interior angles",
        "exterior angles",
        "traverse",
        "balance",
        "allowable",
        "K root n",
    ],
    inputs: &[
        Field::new(
            "angles",
            "Measured angles",
            "One per line, in order, like 108°00'05\"",
            Kind::List {
                items: ANGLE,
                min: 3,
                max: 500,
            },
        )
        .required()
        .core(),
        Field::new(
            "kind",
            "Angles",
            "interior (default) or exterior",
            Kind::Choice(&["interior", "exterior"]),
        )
        .core(),
        Field::new(
            "allowable_k",
            "Allowable K",
            "Seconds; the allowable misclosure is K√n, like 10",
            Kind::Number {
                min: 0.0,
                max: 3600.0,
            },
        )
        .core(),
    ],
    outputs: &[
        Field::new(
            "misclosure",
            "Angular misclosure",
            "The sum measured less the sum the geometry requires",
            Kind::Quantity {
                q: QT::Angle,
                unit: "arcsec",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "sum",
            "Sum of the angles",
            "As measured",
            Kind::Text { max_len: 24 },
        ),
        Field::new(
            "required_sum",
            "Required sum",
            "(n − 2) × 180° inside, (n + 2) × 180° outside",
            Kind::Text { max_len: 24 },
        ),
        Field::new(
            "allowable",
            "Allowable misclosure",
            "K × √n",
            Kind::Quantity {
                q: QT::Angle,
                unit: "arcsec",
            },
        )
        .precision(Precision::Decimals(1))
        .optional(),
        Field::new(
            "closure",
            "Closure",
            "Within or beyond the allowable",
            Kind::Text { max_len: 40 },
        )
        .optional(),
        Field::new(
            "correction",
            "Correction per angle",
            "The misclosure spread equally, with the opposite sign",
            Kind::Quantity {
                q: QT::Angle,
                unit: "arcsec",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "adjusted",
            "Adjusted angles",
            "Each with its correction",
            Kind::List {
                items: ADJUSTED,
                min: 3,
                max: 500,
            },
        ),
    ],
    errors: &[],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "Required sum (n − 2)·180° for interior angles and (n + 2)·180° for exterior; misclosure = Σ measured − required; allowable = K·√n; each angle corrected by −misclosure/n (Ghilani & Wolf 2018, ch. 10)",
    accuracy: "Exact; the allowable depends on the standard, entered as K",
    references: &[GHILANI],
    examples: &[Example {
        id: "primary",
        title: "Five interior angles summing to 540°00'25\"",
        input: r#"{"angles":[{"angle":"108°00'05\""},{"angle":"101°59'55\""},{"angle":"115°00'10\""},{"angle":"95°00'00\""},{"angle":"120°00'15\""}],"allowable_k":10}"#,
        source: "add-survey-suite angular-misclosure scenario: +25\"",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[Related {
        id: "survey.cogo.traverse-closure",
        reason: "next",
    }],
    sentence: "The angles miss their required sum by {misclosure}.{if allowable > 0} The allowable is {allowable}, so the closure is {closure}.{/if}",
    limits: &[("batchRows", 10_000)],
    run: run_angular,
    ..ToolDef::BLANK
};

fn run_angular(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let rows = ctx.rows("angles")?;
    let mut angles = Vec::with_capacity(rows.len());
    for (i, row) in rows.iter().enumerate() {
        let t = row
            .get("angle")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .replace('-', " ");
        let a = dms::parse_plain(&t).map_err(|_| {
            ToolError::invalid(
                &format!("/angles/{i}/angle"),
                format!("\"{t}\" is not an angle."),
            )
        })?;
        if !(0.0 < a && a < 360.0) {
            return Err(ToolError::invalid(
                &format!("/angles/{i}/angle"),
                "A traverse angle is between 0° and 360°.",
            ));
        }
        angles.push(a);
    }
    let n = angles.len() as f64;
    let exterior = ctx.choice("kind")? == Some("exterior");
    let required = if exterior {
        (n + 2.0) * 180.0
    } else {
        (n - 2.0) * 180.0
    };
    // Summed in degrees: round-off stays far below the 0.1" the result shows.
    let sum: f64 = angles.iter().sum();
    let mis = (sum - required) * 3600.0;
    let s = unit(QT::Angle, "arcsec");
    let fmt = |d: f64| dms::format(d, Axis::Lon, Style::Dms, 1, false);
    let corr = -mis / n;
    let mut out = vec![
        (
            "misclosure",
            ctx.out(
                "misclosure",
                Q {
                    value: mis,
                    unit: s,
                },
            ),
        ),
        ("sum", Json::Str(fmt(sum))),
        ("required_sum", Json::Str(fmt(required))),
        (
            "correction",
            ctx.out(
                "correction",
                Q {
                    value: corr,
                    unit: s,
                },
            ),
        ),
        (
            "adjusted",
            Json::Arr(
                angles
                    .iter()
                    .map(|a| Json::obj(vec![("angle", Json::Str(fmt(a + corr / 3600.0)))]))
                    .collect(),
            ),
        ),
    ];
    if let Some(k) = ctx.number("allowable_k")? {
        let allow = k * sqrt(n);
        out.push((
            "allowable",
            ctx.out(
                "allowable",
                Q {
                    value: allow,
                    unit: s,
                },
            ),
        ));
        out.push((
            "closure",
            Json::Str(if mis.abs() <= allow + 1e-9 {
                "within the allowable".into()
            } else {
                "beyond the allowable".into()
            }),
        ));
    }
    Ok(Json::obj(out))
}
