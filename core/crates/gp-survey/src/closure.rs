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

const WALLA_WALLA_TRAVERSE: Reference = Reference {
    title: "Surveying - Traverse Closure",
    issuer: "Yaw, L. L., Walla Walla University",
    year: 2019,
    edition: "June 27, 2019",
    locator: "Section 3.1 and table 1: seven measured interior angles sum to 903 degrees; the 3 degree misclosure is divided equally as a -0.42857 degree correction",
    url: "https://gab.wallawalla.edu/~louie.yaw/Documents/traverse.pdf",
};

const WASHINGTON_TRAVERSE_STANDARD: Reference = Reference {
    title: "WAC 332-130-090: Field traverse standards for land boundary surveys",
    issuer: "Washington State Legislature",
    year: 1990,
    edition: "Effective April 1, 1990; current through 2026",
    locator: "Subsection (2): maximum angular error is 10 arcseconds times the square root of n for 1:10,000 closure, or 30 arcseconds times the square root of n for 1:5,000 closure",
    url: "https://app.leg.wa.gov/wac/default.aspx?cite=332-130-090",
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
    version: "1.1.0",
    stability: gp_base::tool::Stability::Stable,
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
            "interior (default) or reflex exterior",
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
    warnings: &[],
    model: "Required sum (n − 2)·180° for interior angles and (n + 2)·180° for reflex exterior angles; misclosure = Σ measured − required; allowable = K·√n; each angle corrected by −misclosure/n (Ghilani & Wolf 2021, ch. 10)",
    accuracy: "Exact; the allowable depends on the standard, entered as K",
    when_to_use: "Use this when you have turned all the angles of a closed loop traverse and want to check them before you work out any coordinates. Enter the angles in order, say whether they are interior or exterior, and give the K your job standard uses. You get the misclosure, the allowable K√n, and each angle balanced by an equal share. The balanced angles then feed the bearings for the traverse closure.",
    limitations: "The tool splits the misclosure evenly across every angle. That assumes each angle was read with the same care, which may not hold when some sights are short. It cannot find a blunder; a large miss usually means one bad angle or a skipped station. It checks only the angle sum, not distances or the linear closure. The K value comes from the standard your contract or agency names, not from this tool.",
    references: &[GHILANI, WALLA_WALLA_TRAVERSE, WASHINGTON_TRAVERSE_STANDARD],
    examples: &[Example {
        id: "primary",
        title: "Walla Walla University seven-angle traverse",
        input: r#"{"angles":[{"angle":"128.5"},{"angle":"129"},{"angle":"211"},{"angle":"70.5"},{"angle":"80.5"},{"angle":"194"},{"angle":"89.5"}]}"#,
        source: "Yaw, Surveying - Traverse Closure (Walla Walla University, 2019), section 3.1 and table 1: 903° measured against 900° required, corrected by -3°/7 per angle",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "survey.cogo.traverse-closure",
            reason: "next",
        },
        Related {
            id: "survey.cogo.inverse",
            reason: "next",
        },
        Related {
            id: "survey.cogo.area-by-coordinates",
            reason: "next",
        },
    ],
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
