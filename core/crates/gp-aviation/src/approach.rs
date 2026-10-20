//! Approach references (add-aviation-practical-additions, aviation/approach-
//! references): the aircraft approach category of 14 CFR 97.3, from VREF or
//! 1.3 VS0, with the category bounds read from `data/regulations.json`.

use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::regulation::rule;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;

use crate::{knots, obj, unit};

const CFR_97_3: Reference = Reference {
    title: "14 CFR 97.3, Symbols and terms used in procedures",
    issuer: "FAA (eCFR)",
    year: 2016,
    edition: "current eCFR text (amended 2016-12-30)",
    locator: "Definition of aircraft approach category, Categories A to E",
    url: "https://www.ecfr.gov/current/title-14/chapter-I/subchapter-F/part-97/subpart-A/section-97.3",
};

const fn speed(name: &'static str, title: &'static str, help: &'static str) -> Field {
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

/// The categories in order, with the regulation entry for each lower bound.
const CATEGORIES: [(&str, Option<&str>); 5] = [
    ("A", None),
    ("B", Some("faa-97-3-category-b")),
    ("C", Some("faa-97-3-category-c")),
    ("D", Some("faa-97-3-category-d")),
    ("E", Some("faa-97-3-category-e")),
];

pub static APPROACH_CATEGORY: ToolDef = ToolDef {
    id: "aviation.ifr.approach-category",
    title: "Aircraft approach category",
    summary: "The approach category, A to E, that sets an airplane's instrument approach minimums, from VREF or 1.3 × VS0, by the speed bands of 14 CFR 97.3.",
    aliases: &["approach category calculator", "aircraft approach category", "Vref category"],
    keywords: &["approach", "category", "Vref", "Vso", "minimums", "IFR", "97.3"],
    inputs: &[
        speed("vref", "VREF", "Reference landing speed, like 121 kt").core(),
        speed("vs0", "VS0", "Stall speed in the landing configuration at maximum landing weight, like 61 kt, when there is no VREF").core(),
    ],
    outputs: &[
        Field::new("category", "Approach category", "A, B, C, D, or E", Kind::Text { max_len: 1 }),
        speed("speed_used", "Speed used", "VREF, or 1.3 × VS0").precision(Precision::Decimals(1)),
        Field::new("basis", "Basis", "Which speed the category comes from", Kind::Text { max_len: 24 }),
        Field::new("band", "Speed band", "The category's range", Kind::Text { max_len: 40 }),
        Field::new("rules", "Rules as of", "Citation and review date", Kind::Text { max_len: 120 }),
    ],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Category by speed (VREF, or 1.3 VS0 at maximum certificated landing weight when VREF is not specified): A below 91 kt, B 91 to below 121, C 121 to below 141, D 141 to below 166, E 166 and above (14 CFR 97.3)",
    accuracy: "Exact to the regulation's bands; the speeds are the certificated values from the flight manual",
    when_to_use: "Use this to find which column of an instrument approach chart's minimums applies to your airplane. Enter VREF from the flight manual, or the landing-configuration stall speed VS0 at maximum landing weight when there is no VREF; the tool applies 14 CFR 97.3's bands and shows the speed it used.",
    limitations: "The category comes from certificated speeds at maximum certificated landing weight, not today's weight. If you fly the approach faster than your category's band, use the minimums for the category of the speed you fly. Helicopters and some operators use other rules. This is a reference to the regulation's text as of its review date, not legal advice.",
    references: &[CFR_97_3],
    examples: &[Example {
        id: "primary",
        title: "A VREF of 121 kt, on the C boundary",
        input: r#"{"vref":"121 kt"}"#,
        source: "14 CFR 97.3: Category C is 121 knots or more but less than 141 knots",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.performance.vdp",
            reason: "next",
        },
        Related {
            id: "aviation.airspeed.cas-to-tas",
            reason: "alternative",
        },
        Related {
            id: "aviation.wind.runway-components",
            reason: "next",
        },
    ],
    sentence: "The approach category is {category}, set by {basis} {speed_used}. If you fly faster, use the minimums for the faster speed.",
    limits: &[("batchRows", 10_000)],
    run: run_approach_category,
    ..ToolDef::BLANK
};

fn run_approach_category(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let kt = unit(QT::Speed, "kt");
    let vref = ctx.quantity("vref")?.map(|q| q.to(kt));
    let vs0 = ctx.quantity("vs0")?.map(|q| q.to(kt));
    let (v, basis) = match (vref, vs0) {
        (Some(v), _) => (v, "VREF"),
        (None, Some(s)) => (1.3 * s, "1.3 × VS0"),
        (None, None) => {
            return Err(ToolError::invalid(
                "/vref",
                "Give VREF, or VS0 when the airplane has no VREF.",
            )
            .hint("Example: vref 121 kt"));
        }
    };
    if !(v > 0.0 && v.is_finite()) {
        return Err(ToolError::invalid(
            if vref.is_some() { "/vref" } else { "/vs0" },
            "The speed must be more than zero.",
        ));
    }
    let bounds: Vec<(&str, f64)> = CATEGORIES
        .iter()
        .map(|(c, id)| (*c, id.map_or(0.0, |id| rule(id).value)))
        .collect();
    let i = bounds.iter().rposition(|(_, lo)| v >= *lo).unwrap_or(0);
    let band = match bounds.get(i + 1) {
        Some((_, hi)) if i == 0 => format!("below {hi} kt"),
        Some((_, hi)) => format!("{} kt to below {hi} kt", bounds[i].1),
        None => format!("{} kt and above", bounds[i].1),
    };
    let r = rule("faa-97-3-category-c");
    Ok(obj(vec![
        ("category", Json::str(bounds[i].0)),
        ("speed_used", ctx.out("speed_used", knots(v))),
        ("basis", Json::str(basis)),
        ("band", Json::str(band)),
        (
            "rules",
            Json::str(format!("14 CFR 97.3, rules as of {}", r.reviewed)),
        ),
    ]))
}
