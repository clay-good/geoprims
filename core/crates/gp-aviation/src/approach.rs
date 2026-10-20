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

const CFR_91_175: Reference = Reference {
    title: "14 CFR 91.175(h), Comparable values of RVR and ground visibility",
    issuer: "FAA (eCFR)",
    year: 2018,
    edition: "current eCFR text (amended 2018-04-27)",
    locator: "Paragraph (h)(2) table",
    url: "https://www.ecfr.gov/current/title-14/chapter-I/subchapter-F/part-91/subpart-B/section-91.175",
};

/// The 91.175(h)(2) rows: RVR in feet, its regulation entry, and the
/// visibility as the table writes it.
const RVR_TABLE: [(f64, &str, &str); 7] = [
    (1600.0, "faa-91-175-h2-1600", "1/4"),
    (2400.0, "faa-91-175-h2-2400", "1/2"),
    (3200.0, "faa-91-175-h2-3200", "5/8"),
    (4000.0, "faa-91-175-h2-4000", "3/4"),
    (4500.0, "faa-91-175-h2-4500", "7/8"),
    (5000.0, "faa-91-175-h2-5000", "1"),
    (6000.0, "faa-91-175-h2-6000", "1 1/4"),
];

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

pub static RVR_VISIBILITY: ToolDef = ToolDef {
    id: "aviation.ifr.rvr-visibility",
    title: "RVR to ground visibility",
    summary: "The ground visibility an RVR minimum stands for when RVR is not reported, by the table in 14 CFR 91.175(h)(2): 1,600 ft is 1/4 statute mile up to 6,000 ft at 1 1/4.",
    aliases: &[
        "RVR to visibility",
        "RVR conversion table",
        "runway visual range to statute miles",
    ],
    keywords: &[
        "RVR",
        "visibility",
        "minimums",
        "runway",
        "approach",
        "91.175",
    ],
    inputs: &[Field::new(
        "rvr",
        "RVR minimum",
        "As the approach chart gives it, like 2,400 ft",
        Kind::Quantity {
            q: QT::Length,
            unit: "ft",
        },
    )
    .required()
    .core()],
    outputs: &[
        Field::new(
            "visibility",
            "Ground visibility",
            "In statute miles, as the table writes it",
            Kind::Text { max_len: 16 },
        ),
        Field::new(
            "visibility_sm",
            "Ground visibility (decimal)",
            "Statute miles",
            Kind::Quantity {
                q: QT::Length,
                unit: "mi",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "applies",
            "When it applies",
            "The rule's conditions",
            Kind::Text { max_len: 200 },
        ),
        Field::new(
            "rules",
            "Rules as of",
            "Citation and review date",
            Kind::Text { max_len: 120 },
        ),
    ],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Table lookup, 14 CFR 91.175(h)(2): RVR 1,600, 2,400, 3,200, 4,000, 4,500, 5,000, and 6,000 ft are 1/4, 1/2, 5/8, 3/4, 7/8, 1, and 1 1/4 statute miles; no interpolation",
    accuracy: "Exact to the regulation's table; values between rows are not in the rule and are refused",
    when_to_use: "Use this when an approach chart gives an RVR minimum but the runway's RVR is not being reported: 14 CFR 91.175(h)(1) then makes the comparable ground visibility from the table in (h)(2) the minimum. Enter the RVR as charted to read its visibility.",
    limitations: "Only the seven RVR values in the regulation's table convert; the rule does not interpolate between them, so neither does this tool. It does not apply to Category II or III minimums, and it is the US rule only: EASA's conversion of reported visibility to RVR is a different rule, with factors that depend on the lights and time of day, which this tool does not cover. A dated reference, not legal advice.",
    references: &[CFR_91_175],
    examples: &[Example {
        id: "primary",
        title: "An RVR 2,400 minimum with RVR not reported",
        input: r#"{"rvr":"2400 ft"}"#,
        source: "14 CFR 91.175(h)(2): 2,400 ft RVR is 1/2 statute mile",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.ifr.approach-category",
            reason: "next",
        },
        Related {
            id: "aviation.weather.metar-decode",
            reason: "alternative",
        },
        Related {
            id: "aviation.performance.vdp",
            reason: "next",
        },
    ],
    sentence: "Without an RVR report, use {visibility} mile as the limit.",
    limits: &[("batchRows", 10_000)],
    run: run_rvr_visibility,
    ..ToolDef::BLANK
};

fn run_rvr_visibility(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let rvr = ctx.req_quantity("rvr")?.to(unit(QT::Length, "ft"));
    // A value entered in meters converts to feet; match the table to the foot.
    let row = RVR_TABLE.iter().find(|(r, _, _)| (r - rvr).abs() < 0.5).ok_or_else(|| {
        ToolError::invalid(
            "/rvr",
            "14 CFR 91.175(h)(2) lists only RVR 1,600, 2,400, 3,200, 4,000, 4,500, 5,000, and 6,000 ft, and does not interpolate between them.",
        )
        .hint("Example: rvr 2400 ft")
    })?;
    let r = rule(row.1);
    Ok(obj(vec![
        ("visibility", Json::str(row.2)),
        (
            "visibility_sm",
            ctx.out(
                "visibility_sm",
                gp_base::tool::Q {
                    value: r.value * 1609.344,
                    unit: unit(QT::Length, "m"),
                },
            ),
        ),
        (
            "applies",
            Json::str(
                "When RVR minimums are prescribed but RVR is not reported for the runway, except for Category II or III minimums (91.175(h)(1)).",
            ),
        ),
        (
            "rules",
            Json::str(format!("{}, rules as of {}", r.citation, r.reviewed)),
        ),
    ]))
}
