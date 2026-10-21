//! Approach references (add-aviation-practical-additions, aviation/approach-
//! references): the aircraft approach category of 14 CFR 97.3, from VREF or
//! 1.3 VS0, with the category bounds read from `data/regulations.json`; the
//! RVR to visibility table of 14 CFR 91.175(h)(2); and the circling approach
//! radius of Order 8260.3G.

use gp_base::error::{ToolError, Warning};
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

/// The Terminal Procedures Publication legend's Comparable Values of RVR and
/// Visibility (d-TPP cycle 2611): RVR in feet and the visibility as printed.
/// It repeats the regulation's seven rows and adds eight more; between rows
/// the legend says to use the next higher listed RVR.
const LEGEND_RVR: [(f64, &str, f64); 15] = [
    (1200.0, "1/4", 0.25),
    (1600.0, "1/4", 0.25),
    (1800.0, "1/2", 0.5),
    (2000.0, "1/2", 0.5),
    (2200.0, "1/2", 0.5),
    (2400.0, "1/2", 0.5),
    (2600.0, "1/2", 0.5),
    (3000.0, "5/8", 0.625),
    (3200.0, "5/8", 0.625),
    (3500.0, "5/8", 0.625),
    (4000.0, "3/4", 0.75),
    (4500.0, "7/8", 0.875),
    (5000.0, "1", 1.0),
    (5500.0, "1", 1.0),
    (6000.0, "1 1/4", 1.25),
];

const TPP_RVR: Reference = Reference {
    title: "Terminal Procedures Publication legend, Comparable Values of RVR and Visibility",
    issuer: "FAA Aeronautical Information Services",
    year: 2026,
    edition: "d-TPP cycle 2611, 29 OCT 2026 to 26 NOV 2026",
    locator: "Front matter (frntmatter.pdf), Terms/Landing Minima Data: the RVR table, its next-higher-value rule, and the starred rows repeated from 14 CFR 91.175",
    url: "https://www.faa.gov/air_traffic/flight_info/aeronav/digital_products/dtpp/",
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

pub static RVR_VISIBILITY: ToolDef = ToolDef {
    id: "aviation.ifr.rvr-visibility",
    version: "1.1.0",
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
    inputs: &[
        Field::new(
            "rvr",
            "RVR minimum",
            "As the approach chart gives it, like 2,400 ft",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .required()
        .core(),
        Field::new(
            "table",
            "Table",
            "regulation (91.175(h)(2), the default) or legend (the approach chart legend's longer table, next higher row between rows), like regulation",
            Kind::Choice(&["regulation", "legend"]),
        ),
    ],
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
            "rvr_used",
            "RVR row used",
            "The table row the visibility comes from",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .precision(Precision::Decimals(0)),
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
    warnings: &["RVR_NOT_IN_REGULATION", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Table lookup, 14 CFR 91.175(h)(2): RVR 1,600, 2,400, 3,200, 4,000, 4,500, 5,000, and 6,000 ft are 1/4, 1/2, 5/8, 3/4, 7/8, 1, and 1 1/4 statute miles; no interpolation. Optionally the chart legend's table of 15 rows from 1,200 to 6,000 ft, taking the next higher listed RVR between rows",
    accuracy: "Exact to the regulation's table; values between rows are not in the rule and are refused",
    when_to_use: "Use this when an approach chart gives an RVR minimum but the runway's RVR is not being reported: 14 CFR 91.175(h)(1) then makes the comparable ground visibility from the table in (h)(2) the minimum. Enter the RVR as charted to read its visibility.",
    limitations: "By default only the seven RVR values in the regulation's table convert; the rule does not interpolate between them, so neither does this tool. The chart legend's table adds rows and rounds up to the next listed RVR, but the legend keeps the regulation's rows for takeoff and landing minima, so its other rows carry a caution. It does not apply to Category II or III minimums, and it is the US rule only: EASA's conversion of reported visibility to RVR is a different rule, with factors that depend on the lights and time of day, which this tool does not cover. A dated reference, not legal advice.",
    references: &[CFR_91_175, TPP_RVR],
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
    sentence: "Without an RVR report, RVR {rvr_used} is {visibility} statute mile.{warn RVR_NOT_IN_REGULATION} The regulation does not list this row, so it is not a takeoff or landing minimum.{/warn}",
    limits: &[("batchRows", 10_000)],
    run: run_rvr_visibility,
    ..ToolDef::BLANK
};

fn run_rvr_visibility(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let ft = unit(QT::Length, "ft");
    let rvr = ctx.req_quantity("rvr")?.to(ft);
    let q = |v: f64, u| gp_base::tool::Q { value: v, unit: u };
    if ctx.choice("table")? == Some("legend") {
        // A value entered in meters converts to feet; a row matches to the foot,
        // and a value between rows takes the next higher listed RVR.
        let row = LEGEND_RVR
            .iter()
            .find(|(r, _, _)| rvr <= r + 0.5)
            .filter(|_| rvr >= LEGEND_RVR[0].0 - 0.5)
            .ok_or_else(|| {
                ToolError::invalid(
                    "/rvr",
                    "The chart legend's table runs from RVR 1,200 to 6,000 ft.",
                )
                .hint("Example: rvr 2800 ft")
            })?;
        let regulation = RVR_TABLE.iter().any(|(r, _, _)| *r == row.0);
        if !regulation {
            ctx.warnings.push(Warning::new(
                "RVR_NOT_IN_REGULATION",
                format!(
                    "RVR {} ft is in the chart legend's table but not in 14 CFR 91.175(h)(2); the legend keeps the regulation's rows for takeoff and landing minima.",
                    gp_base::display::number(row.0, Precision::Decimals(0), ctx.options.format)
                ),
            ));
        }
        return Ok(obj(vec![
            ("visibility", Json::str(row.1)),
            (
                "visibility_sm",
                ctx.out("visibility_sm", q(row.2 * 1609.344, unit(QT::Length, "m"))),
            ),
            ("rvr_used", ctx.out("rvr_used", q(row.0, ft))),
            (
                "applies",
                Json::str(
                    "The approach chart legend's table for turning RVR into ground or flight visibility; between rows it uses the next higher listed RVR, and only the rows the regulation lists are for takeoff or landing minima.",
                ),
            ),
            (
                "rules",
                Json::str(
                    "Terminal Procedures Publication legend, d-TPP cycle 2611 (29 OCT 2026 to 26 NOV 2026)",
                ),
            ),
        ]));
    }
    let row = RVR_TABLE.iter().find(|(r, _, _)| (r - rvr).abs() < 0.5).ok_or_else(|| {
        ToolError::invalid(
            "/rvr",
            "14 CFR 91.175(h)(2) lists only RVR 1,600, 2,400, 3,200, 4,000, 4,500, 5,000, and 6,000 ft, and does not interpolate between them.",
        )
        .hint("Example: rvr 2400 ft, or table legend for the chart legend's longer table")
    })?;
    let r = rule(row.1);
    Ok(obj(vec![
        ("visibility", Json::str(row.2)),
        (
            "visibility_sm",
            ctx.out(
                "visibility_sm",
                q(r.value * 1609.344, unit(QT::Length, "m")),
            ),
        ),
        ("rvr_used", ctx.out("rvr_used", q(row.0, ft))),
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

// ------------------------------------------------------- visibility to CMV

const EASA_AMC10: Reference = Reference {
    title: "Easy Access Rules for Air Operations, AMC10 CAT.OP.MPA.110, Conversion of visibility to CMV",
    issuer: "European Union Aviation Safety Agency",
    year: 2026,
    edition: "Revision 24, March 2026 (ED Decision 2022/012/R, applicable from 30 October 2022)",
    locator: "AMC10 CAT.OP.MPA.110 (a) to (c) and Table 16, Conversion of reported VIS to RVR/CMV (page 967)",
    url: "https://www.easa.europa.eu/en/document-library/easy-access-rules/easy-access-rules-air-operations-regulation-eu-no-9652012",
};

pub static VISIBILITY_TO_CMV: ToolDef = ToolDef {
    id: "aviation.ifr.visibility-to-cmv",
    title: "Converted meteorological visibility",
    summary: "The converted meteorological visibility (CMV) that stands in for an unavailable RVR under EASA rules: the reported visibility times 1.0, 1.5, or 2.0 by the lights in operation and day or night, from AMC10 CAT.OP.MPA.110 Table 16.",
    aliases: &[
        "CMV calculator",
        "visibility to RVR EASA",
        "converted meteorological visibility",
    ],
    keywords: &[
        "CMV",
        "RVR",
        "visibility",
        "EASA",
        "minima",
        "approach",
    ],
    inputs: &[
        Field::new(
            "visibility",
            "Reported visibility",
            "Meteorological visibility, like 1,500 m",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .required()
        .core(),
        Field::new(
            "lights",
            "Lights in operation",
            "hi (high-intensity approach and runway lights), other, or none, like hi",
            Kind::Choice(&["hi", "other", "none"]),
        )
        .required()
        .core(),
        Field::new(
            "period",
            "Day or night",
            "day or night, like night",
            Kind::Choice(&["day", "night"]),
        )
        .required()
        .core(),
        Field::new(
            "purpose",
            "Used for",
            "approach (the default), planning, takeoff, or lvo-continuation (going on with an approach in low-visibility operations), like approach",
            Kind::Choice(&["approach", "planning", "takeoff", "lvo-continuation"]),
        )
        .core(),
    ],
    outputs: &[
        Field::new(
            "cmv",
            "Converted visibility (CMV)",
            "Reported visibility × factor",
            Kind::Quantity {
                q: QT::Length,
                unit: "m",
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "factor",
            "Factor",
            "From Table 16, or 1.0 for flight planning",
            Kind::Number { min: 1.0, max: 2.0 },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "applies",
            "When it applies",
            "The rule's conditions",
            Kind::Text { max_len: 240 },
        ),
        Field::new(
            "rules",
            "Rules as of",
            "Citation and review date",
            Kind::Text { max_len: 120 },
        ),
    ],
    errors: &[gp_base::ErrorCode::OutOfDomain],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "CMV = reported visibility × factor: AMC10 CAT.OP.MPA.110 Table 16 (high-intensity approach and runway lights 1.5 day, 2.0 night; other lights 1.0 day, 1.5 night; no lights 1.0 day, not applicable at night), or 1.0 for flight planning; not for take-off minima or to continue an approach in LVOs",
    accuracy: "Exact to the AMC's table as of the review date",
    when_to_use: "Use this under EASA rules when the runway's RVR is not reported and you need the value that stands in for it: enter the reported visibility, the lights in use, and whether it is day or night. It returns the CMV to compare with the approach's RVR minimum. For flight planning it uses a factor of 1.0, as the AMC says.",
    limitations: "It quotes the EASA acceptable means of compliance for commercial air transport by aeroplane; your operator's approved procedures govern. The conversion is not allowed for take-off minima or to continue an approach in low-visibility operations, and at night with no lights the table gives no factor; the tool refuses all three. It is not the US rule, which converts the other way, from RVR to visibility. A dated reference, not legal advice.",
    references: &[EASA_AMC10],
    examples: &[Example {
        id: "primary",
        title: "1,500 m at night with high-intensity lights",
        input: r#"{"visibility":"1500 m","lights":"hi","period":"night"}"#,
        source: "AMC10 CAT.OP.MPA.110 Table 16: 1,500 m × 2.0 = 3,000 m",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.ifr.rvr-visibility",
            reason: "alternative",
        },
        Related {
            id: "aviation.weather.metar-decode",
            reason: "alternative",
        },
        Related {
            id: "aviation.ifr.approach-category",
            reason: "next",
        },
    ],
    sentence: "The CMV is {cmv}. That is the reported value times {factor}.",
    limits: &[("batchRows", 10_000)],
    run: run_visibility_to_cmv,
    ..ToolDef::BLANK
};

fn run_visibility_to_cmv(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let m = unit(QT::Length, "m");
    let vis = ctx.req_quantity("visibility")?.to(m);
    if !(vis > 0.0 && vis <= 100_000.0) {
        return Err(ToolError::invalid(
            "/visibility",
            "The reported visibility is more than 0 and at most 100 km.",
        ));
    }
    let lights = ctx.choice("lights")?.expect("required");
    let night = ctx.choice("period")?.expect("required") == "night";
    let refuse = |field: &str, why: &str| {
        Err(ToolError::new(gp_base::ErrorCode::OutOfDomain, why).at(field))
    };
    let id = match ctx.choice("purpose")?.unwrap_or("approach") {
        "takeoff" => {
            return refuse(
                "/purpose",
                "AMC10 CAT.OP.MPA.110 does not allow a CMV for take-off minima; take-off needs the reported RVR or visibility.",
            );
        }
        "lvo-continuation" => {
            return refuse(
                "/purpose",
                "AMC10 CAT.OP.MPA.110 does not allow a CMV to continue an approach in low-visibility operations; that needs the reported RVR.",
            );
        }
        "planning" => "easa-cmv-planning",
        _ => match (lights, night) {
            ("hi", false) => "easa-cmv-hi-day",
            ("hi", true) => "easa-cmv-hi-night",
            ("other", false) => "easa-cmv-other-day",
            ("other", true) => "easa-cmv-other-night",
            ("none", false) => "easa-cmv-none-day",
            _ => {
                return refuse(
                    "/lights",
                    "AMC10 CAT.OP.MPA.110 Table 16 gives no factor at night with no lights: the conversion is not applicable.",
                );
            }
        },
    };
    let r = rule(id);
    Ok(obj(vec![
        (
            "cmv",
            ctx.out(
                "cmv",
                gp_base::tool::Q {
                    value: vis * r.value,
                    unit: m,
                },
            ),
        ),
        ("factor", Json::Num(r.value)),
        (
            "applies",
            Json::str(
                "When the reported RVR is not available, or the approach's minimum RVR is more than the most the aerodrome assesses; not for take-off minima or to continue an approach in low-visibility operations.",
            ),
        ),
        (
            "rules",
            Json::str(format!("{}, rules as of {}", r.citation, r.reviewed)),
        ),
    ]))
}

// ------------------------------------------------------------ circling radius

const ORDER_8260_3G: Reference = Reference {
    title: "Order 8260.3G, United States Standard for Terminal Instrument Procedures (TERPS)",
    issuer: "FAA",
    year: 2024,
    edition: "Order 8260.3G, effective 07/01/2024",
    locator: "Paragraph 2-7-1.a: Formula 2-7-1 (true airspeed), Formula 2-7-2 (circling approach radius, minimum 1.30 NM), and Table 2-7-1 (page 2-58)",
    url: "https://www.faa.gov/documentLibrary/media/Order/Order_8260.3G.pdf",
};

const IPH_CIRCLING: Reference = Reference {
    title: "Instrument Procedures Handbook, FAA-H-8083-16B",
    issuer: "Federal Aviation Administration",
    year: 2017,
    edition: "FAA-H-8083-16B",
    locator: "Chapter 4, Figure 4-2 (the radii before Order 8260.3 Change 21) and its text (Category C at a sea-level airport with a 1,500 ft HAA: 2.86 NM)",
    url: "https://www.faa.gov/regulations_policies/handbooks_manuals/aviation/instrument_procedures_handbook",
};

/// The categories of Order 8260.3G Table 2-7-1, whose speed, bank, and
/// straight segment are read from `data/regulations.json`, with the radius
/// charted before Change 21 (IPH Figure 4-2, NM), which is not a rule.
const CIRCLING: [(&str, &str, f64); 5] = [
    ("A", "a", 1.3),
    ("B", "b", 1.5),
    ("C", "c", 1.7),
    ("D", "d", 2.3),
    ("E", "e", 4.5),
];

const TPP_LEGEND: Reference = Reference {
    title: "Terminal Procedures Publication legend, Circling Approach Maneuvering Airspace Radius",
    issuer: "FAA Aeronautical Information Services",
    year: 2026,
    edition: "d-TPP cycle 2611, 29 OCT 2026 to 26 NOV 2026",
    locator: "Front matter (frntmatter.pdf), Terms/Landing Minima Data: the circling radius by approach category and circling MDA",
    url: "https://www.faa.gov/air_traffic/flight_info/aeronav/digital_products/dtpp/",
};

/// The legend's table: the top of each circling MDA band (ft MSL; the last
/// band has none) and the radius it prints for Categories A to E (NM). Each
/// cell is Formula 2-7-2 at the band's lowest MDA (0, 1,001, 3,001, ... ft),
/// cut down to a tenth, and never under 1.3 NM: all 30 cells reproduce.
const TPP_CIRCLING: [(f64, [f64; 5]); 6] = [
    (1_000.0, [1.3, 1.7, 2.7, 3.6, 4.5]),
    (3_000.0, [1.3, 1.8, 2.8, 3.7, 4.6]),
    (5_000.0, [1.3, 1.8, 2.9, 3.8, 4.8]),
    (7_000.0, [1.3, 1.9, 3.0, 4.0, 5.0]),
    (9_000.0, [1.4, 2.0, 3.2, 4.2, 5.3]),
    (f64::INFINITY, [1.4, 2.1, 3.3, 4.4, 5.5]),
];

const fn ft(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Length,
            unit: "ft",
        },
    )
}

const fn nm(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Length,
            unit: "NM",
        },
    )
}

pub static CIRCLING_RADIUS: ToolDef = ToolDef {
    id: "aviation.ifr.circling-radius",
    version: "1.1.0",
    title: "Circling approach radius",
    summary: "The radius of the protected circling area for an approach category, by the FAA's TERPS formulas from the category's speed, bank, and the true airspeed at the airport's elevation, never less than 1.30 NM.",
    aliases: &[
        "circling approach radius",
        "circling area size",
        "TERPS circling radius",
    ],
    keywords: &["circling", "approach", "radius", "category", "TERPS", "MDA"],
    inputs: &[
        Field::new(
            "category",
            "Approach category",
            "A, B, C, D, or E, like C",
            Kind::Choice(&["A", "B", "C", "D", "E"]),
        )
        .required()
        .core(),
        ft(
            "airport_elevation",
            "Airport elevation",
            "Above sea level, like 5,000 ft",
        )
        .required()
        .core(),
        ft(
            "circling_altitude",
            "Circling altitude",
            "Optional, above sea level, like 6,400 ft; at least 1,000 ft above the airport is used",
        )
        .core(),
    ],
    outputs: &[
        nm(
            "radius",
            "Circling approach radius",
            "From each runway threshold",
        )
        .precision(Precision::Decimals(2)),
        nm(
            "radius_formula",
            "Radius by the formula",
            "Before the 1.30 NM minimum",
        )
        .precision(Precision::Decimals(3)),
        speed("tas", "True airspeed", "Formula 2-7-1").precision(Precision::Decimals(1)),
        speed("ias", "Indicated airspeed", "Table 2-7-1").precision(Precision::Decimals(0)),
        Field::new(
            "bank",
            "Bank angle",
            "Table 2-7-1",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(0)),
        nm("straight_segment", "Straight segment", "Table 2-7-1").precision(Precision::Decimals(1)),
        ft(
            "height_used",
            "Height above the airport used",
            "At least 1,000 ft",
        )
        .precision(Precision::Decimals(0)),
        nm(
            "legacy_radius",
            "Radius before Change 21",
            "The fixed radius charted before Order 8260.3 Change 21",
        )
        .precision(Precision::Decimals(1)),
        nm(
            "legend_radius",
            "Chart legend radius",
            "The Terminal Procedures legend's radius for this circling MDA band",
        )
        .precision(Precision::Decimals(1))
        .optional(),
    ],
    errors: &[gp_base::ErrorCode::OutOfDomain],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "V_KTAS = V_KIAS × 171233 × √(303 − 0.00198 (elevation + k)) / (288 − 0.00198 (elevation + k))^2.628, k ≥ 1,000 ft (Formula 2-7-1); CAR = 2 (V_KTAS + 25)² / (tan(bank) × 68625.4) + S, at least 1.30 NM (Formula 2-7-2), with Table 2-7-1's speed, bank, and S for the category (Order 8260.3G)",
    accuracy: "Exact to the order's formulas; the 25 kt in the formula is the order's wind allowance",
    when_to_use: "Use this to see how far from the runway a circling approach is protected for your category at a given airport, so you can stay inside it when you circle. Enter the category and the airport elevation, and the circling altitude when it is more than 1,000 ft above the airport. It shows each number the formula used.",
    limitations: "This is the design radius of the TERPS circling area, measured from each runway threshold; the chart's own circling minimums and any restricted sectors govern. Older procedures may still be built on the fixed radii charted before Order 8260.3 Change 21, which the tool shows for comparison. With a circling altitude it also shows the radius the chart legend prints for that MDA band, which is the formula at the band's lowest altitude, cut down to a tenth.",
    references: &[ORDER_8260_3G, IPH_CIRCLING, TPP_LEGEND],
    examples: &[Example {
        id: "primary",
        title: "Category C at a sea-level airport",
        input: r#"{"category":"C","airport_elevation":"0 ft","circling_altitude":"1000 ft"}"#,
        source: "Order 8260.3G Formulas 2-7-1 and 2-7-2 with Table 2-7-1",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.ifr.approach-category",
            reason: "parent",
        },
        Related {
            id: "aviation.performance.turn",
            reason: "alternative",
        },
        Related {
            id: "aviation.airspeed.cas-to-tas",
            reason: "alternative",
        },
    ],
    sentence: "The circling approach radius is {radius} from each runway threshold.{if radius_formula < 1.3} The formula gives {radius_formula}, so the 1.30 NM minimum applies.{/if}",
    limits: &[("batchRows", 10_000)],
    run: run_circling_radius,
    ..ToolDef::BLANK
};

fn run_circling_radius(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let ft_u = unit(QT::Length, "ft");
    let nm_u = unit(QT::Length, "NM");
    let cat = ctx.choice("category")?.expect("required");
    let elev = ctx.req_quantity("airport_elevation")?.to(ft_u);
    if !(-2_000.0..=15_000.0).contains(&elev) {
        return Err(ToolError::new(
            gp_base::ErrorCode::OutOfDomain,
            "The airport elevation is from -2,000 to 15,000 ft.",
        )
        .at("/airport_elevation"));
    }
    let mda = ctx.quantity("circling_altitude")?.map(|a| a.to(ft_u));
    let k = match mda {
        Some(a) => {
            let above = a - elev;
            if !(0.0..=10_000.0).contains(&above) {
                return Err(ToolError::new(
                    gp_base::ErrorCode::OutOfDomain,
                    "The circling altitude is from the airport elevation to 10,000 ft above it.",
                )
                .at("/circling_altitude"));
            }
            above.max(1_000.0)
        }
        None => 1_000.0,
    };
    let col = CIRCLING.iter().position(|r| r.0 == cat).expect("choice");
    let (_, key, legacy) = CIRCLING[col];
    let table = |what: &str| rule(&format!("faa-8260-3-t2-7-1-{key}-{what}")).value;
    let (ias, bank, s) = (table("kias"), table("bank"), table("straight"));
    let min_car = rule("faa-8260-3-car-min").value;
    let h = 0.00198 * (elev + k);
    let tas = ias * 171_233.0 * libm::sqrt(303.0 - h) / libm::pow(288.0 - h, 2.628);
    let formula = 2.0 * (tas + 25.0) * (tas + 25.0) / (libm::tan(bank.to_radians()) * 68_625.4) + s;
    let q = |v: f64, u| gp_base::tool::Q { value: v, unit: u };
    let legend = mda.map(|a| {
        let row = TPP_CIRCLING
            .iter()
            .find(|(top, _)| a <= *top)
            .expect("the last band is open");
        row.1[col]
    });
    let mut out = vec![
        ("radius", ctx.out("radius", q(formula.max(min_car), nm_u))),
        (
            "radius_formula",
            ctx.out("radius_formula", q(formula, nm_u)),
        ),
        ("tas", ctx.out("tas", knots(tas))),
        ("ias", ctx.out("ias", knots(ias))),
        ("bank", ctx.out("bank", q(bank, unit(QT::Angle, "deg")))),
        ("straight_segment", ctx.out("straight_segment", q(s, nm_u))),
        ("height_used", ctx.out("height_used", q(k, ft_u))),
        ("legacy_radius", ctx.out("legacy_radius", q(legacy, nm_u))),
    ];
    if let Some(r) = legend {
        out.push(("legend_radius", ctx.out("legend_radius", q(r, nm_u))));
    }
    Ok(obj(out))
}
