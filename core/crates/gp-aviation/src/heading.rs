//! The heading chain and cloud-base estimates (add-aviation-suite): true
//! course through wind correction, variation, and an interpolated compass
//! deviation card to the compass heading, each step shown; and convective
//! cloud base and freezing level from surface readings.

use crate::refs::*;
use crate::{obj, unit};
use gp_base::ErrorCode;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use libm::log;

const fn qty(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    q: QT,
    u: &'static str,
) -> Field {
    Field::new(name, title, help, Kind::Quantity { q, unit: u })
}

const PHAK_NAV: Reference = Reference {
    locator: "Navigation chapter (magnetic variation, compass deviation and the deviation card, and the compass heading)",
    ..PHAK
};

const CARD_ROW: &[Field] = &[
    qty(
        "heading",
        "For magnetic heading",
        "Like 060",
        QT::Angle,
        "deg",
    )
    .required(),
    qty(
        "deviation",
        "Deviation",
        "East positive, like +2 or -1",
        QT::Angle,
        "deg",
    )
    .required(),
];
const STEP_ROW: &[Field] = &[
    Field::new("step", "Step", "What changes", Kind::Text { max_len: 40 }),
    qty("heading", "Heading", "After this step", QT::Angle, "deg")
        .precision(Precision::Decimals(1)),
];

pub static HEADING_CHAIN: ToolDef = ToolDef {
    id: "aviation.wind.heading-chain",
    title: "True course to compass heading",
    summary: "From true course through the wind correction, variation, and your compass deviation card to the heading to fly on the compass, each step shown.",
    aliases: &[
        "compass heading",
        "true to compass",
        "TVMDC",
        "deviation card",
        "magnetic heading from true",
    ],
    keywords: &[
        "compass heading",
        "variation",
        "deviation",
        "deviation card",
        "true heading",
        "magnetic heading",
        "wind correction",
    ],
    inputs: &[
        qty("true_course", "True course", "Like 045", QT::Angle, "deg")
            .required()
            .core(),
        qty(
            "wind_correction",
            "Wind correction angle",
            "Right positive, like +8; 0 if none",
            QT::Angle,
            "deg",
        )
        .core(),
        qty(
            "variation",
            "Variation",
            "East positive, like -10 for 10° W, from the chart or a model",
            QT::Angle,
            "deg",
        )
        .required()
        .core(),
        Field::new(
            "deviation_card",
            "Deviation card",
            "Magnetic heading and deviation, one per line, like 060, +2",
            Kind::List {
                items: CARD_ROW,
                min: 0,
                max: 36,
            },
        )
        .core(),
    ],
    outputs: &[
        qty(
            "compass_heading",
            "Compass heading",
            "The heading to fly on the compass",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(1)),
        qty(
            "true_heading",
            "True heading",
            "True course plus wind correction",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(1)),
        qty(
            "magnetic_heading",
            "Magnetic heading",
            "True heading minus east variation",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(1)),
        qty(
            "deviation",
            "Deviation",
            "From the card at the magnetic heading, interpolated",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "steps",
            "Each step",
            "True course to compass heading",
            Kind::List {
                items: STEP_ROW,
                min: 0,
                max: 4,
            },
        ),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "True heading = true course + wind correction; magnetic = true − variation (east positive: \"east is least\"); compass = magnetic − deviation, the deviation read from the card by linear interpolation between its two nearest headings, around the circle (PHAK, navigation)",
    accuracy: "Exact arithmetic; the deviation is as good as the card",
    references: &[PHAK_NAV],
    examples: &[Example {
        id: "primary",
        title: "Course 070° true, 5° left wind correction, 10° W variation, and a card",
        input: r#"{"true_course":"070 deg","wind_correction":"-5 deg","variation":"-10 deg","deviation_card":[{"heading":"030 deg","deviation":"1 deg"},{"heading":"060 deg","deviation":"2 deg"},{"heading":"090 deg","deviation":"-1 deg"},{"heading":"120 deg","deviation":"-2 deg"}]}"#,
        source: "add-aviation-suite deviation-card scenario: magnetic 075°, between +2° at 060° and -1° at 090°, reads +0.5°; compass 074.5°",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.wind.heading-groundspeed",
            reason: "parent",
        },
        Related {
            id: "aviation.wind.uv",
            reason: "alternative",
        },
    ],
    sentence: "Fly {compass_heading} on the compass: {magnetic_heading} magnetic, with {deviation} of deviation.",
    limits: &[("batchRows", 10_000)],
    run: run_chain,
    ..ToolDef::BLANK
};

fn norm(a: f64) -> f64 {
    let v = a.rem_euclid(360.0);
    if v == 0.0 { 0.0 } else { v }
}

/// The card's deviation at `hdg`, interpolated between its neighbors around the circle.
pub fn card_deviation(card: &[(f64, f64)], hdg: f64) -> f64 {
    match card.len() {
        0 => 0.0,
        1 => card[0].1,
        n => {
            let h = norm(hdg);
            // The entry at or before h (around the circle), and the next one.
            let k = card.iter().rposition(|&(c, _)| c <= h).unwrap_or(n - 1);
            let (a, b) = (card[k], card[(k + 1) % n]);
            let span = (b.0 - a.0).rem_euclid(360.0);
            if span == 0.0 {
                return a.1;
            }
            let t = (h - a.0).rem_euclid(360.0) / span;
            a.1 + t * (b.1 - a.1)
        }
    }
}

fn run_chain(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let dg = unit(QT::Angle, "deg");
    let tc = ctx.req_quantity("true_course")?.to(dg);
    let wca = ctx.quantity("wind_correction")?.map_or(0.0, |q| q.to(dg));
    let var = ctx.req_quantity("variation")?.to(dg);
    if wca.abs() >= 90.0 || var.abs() > 180.0 {
        return Err(ToolError::invalid(
            "/wind_correction",
            "A wind correction under 90° and a variation within ±180°, please.",
        ));
    }
    let mut card: Vec<(f64, f64)> = Vec::new();
    if ctx.is_set("deviation_card") {
        let rows = ctx.rows("deviation_card")?;
        for (i, r) in rows.iter().enumerate() {
            let h = ctx
                .row_quantity("deviation_card", i, r, "heading")?
                .expect("required")
                .to(dg);
            let d = ctx
                .row_quantity("deviation_card", i, r, "deviation")?
                .expect("required")
                .to(dg);
            if d.abs() > 30.0 {
                return Err(ToolError::invalid(
                    &format!("/deviation_card/{i}/deviation"),
                    "A deviation over 30° means the compass needs swinging, not interpolating.",
                ));
            }
            card.push((norm(h), d));
        }
        card.sort_by(|a, b| a.0.total_cmp(&b.0));
        if card.windows(2).any(|w| w[0].0 == w[1].0) {
            return Err(ToolError::invalid(
                "/deviation_card",
                "Each heading appears once on a card.",
            ));
        }
    }
    let th = norm(tc + wca);
    let mh = norm(th - var);
    let dev = card_deviation(&card, mh);
    let ch = norm(mh - dev);
    let q = |v: f64| Q { value: v, unit: dg };
    let step =
        |name: &str, h: f64| Json::obj([("step", Json::str(name)), ("heading", q(h).to_json())]);
    Ok(obj(vec![
        ("compass_heading", ctx.out("compass_heading", q(ch))),
        ("true_heading", ctx.out("true_heading", q(th))),
        ("magnetic_heading", ctx.out("magnetic_heading", q(mh))),
        ("deviation", ctx.out("deviation", q(dev))),
        (
            "steps",
            Json::Arr(vec![
                step("true course", norm(tc)),
                step("+ wind correction: true heading", th),
                step("− variation: magnetic heading", mh),
                step("− deviation: compass heading", ch),
            ]),
        ),
    ]))
}

// ---------------------------------------------------------------- cloud base and freezing level

const BOLTON: Reference = Reference {
    title: "The Computation of Equivalent Potential Temperature",
    issuer: "Bolton, D., Monthly Weather Review",
    year: 1980,
    edition: "Vol. 108, No. 7",
    locator: "pp. 1046-1053, equation 15 (temperature at the lifting condensation level)",
    url: "https://doi.org/10.1175/1520-0493(1980)108%3C1046:TCOEPT%3E2.0.CO;2",
};

pub static CLOUD_BASE: ToolDef = ToolDef {
    id: "aviation.atmosphere.cloud-base",
    title: "Cloud base and freezing level",
    summary: "An estimate of the base of convective clouds from the temperature and dew point, by the 400 ft per degree rule and by the lifting condensation level, and of the freezing level.",
    aliases: &["cloud base", "cloud base calculator", "freezing level", "lifting condensation level", "LCL"],
    keywords: &["cloud base", "dew point spread", "LCL", "lifting condensation level", "freezing level", "cumulus", "ceiling estimate"],
    inputs: &[
        qty("temperature", "Surface temperature", "Like 25 degC", QT::Temperature, "degC").required().core(),
        qty("dew_point", "Dew point", "Like 15 degC", QT::Temperature, "degC").required().core(),
        qty("elevation", "Field elevation", "For heights above sea level, like 5430 ft", QT::Length, "ft").core(),
        Field::new("lapse_rate", "Lapse rate for the freezing level", "Degrees C per 1,000 ft, like 1.98 (the standard, default)", Kind::Number { min: 0.1, max: 5.0 }),
    ],
    outputs: &[
        qty("cloud_base_rule", "Cloud base by the rule", "Spread × 400 ft per °C, above the field", QT::Length, "ft").precision(Precision::Decimals(0)),
        qty("cloud_base_lcl", "Cloud base, lifting condensation level", "Bolton's formula, above the field", QT::Length, "ft").precision(Precision::Decimals(0)),
        qty("cloud_base_msl", "Cloud base above sea level", "The LCL plus the field elevation", QT::Length, "ft").precision(Precision::Decimals(0)).optional(),
        qty("freezing_level", "Freezing level", "Where the temperature reaches 0 °C at the lapse rate, above sea level when the field elevation is given", QT::Length, "ft").precision(Precision::Decimals(0)).optional(),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Rule of thumb: base ≈ (temperature − dew point) × 400 ft per °C. Lifting condensation level: Bolton (1980) eq. 15, T_L = 1/(1/(T_d − 56) + ln(T/T_d)/800) + 56 in kelvins, reached at (T − T_L) ÷ (g/c_p) above the surface, with g/c_p = 9.77 K per km. Freezing level: surface temperature ÷ lapse rate above the field (standard 1.98 °C per 1,000 ft)",
    accuracy: "Estimates for well-mixed surface air forming convective cloud: the LCL is within about 0.1 K in temperature by Bolton's fit; real bases vary with mixing and terrain",
    references: &[BOLTON, WEATHER_HANDBOOK],
    examples: &[Example {
        id: "primary",
        title: "25 °C with a 15 °C dew point",
        input: r#"{"temperature":"25 degC","dew_point":"15 degC"}"#,
        source: "add-aviation-suite cloud-base scenario (about 4,000 ft AGL by the rule, the LCL alongside)",
    }],
    primary_example: "primary",
    visualization: &[Layer { kind: "table-only", map: &[] }],
    related: &[
        Related { id: "aviation.altimetry.density-altitude", reason: "alternative" },
        Related { id: "aviation.weather.metar-decode", reason: "parent" },
    ],
    sentence: "Convective clouds should form about {cloud_base_lcl} above the field; the rule of thumb says {cloud_base_rule}.",
    limits: &[("batchRows", 10_000)],
    run: run_cloud_base,
    ..ToolDef::BLANK
};

const G_OVER_CP: f64 = 9.806_65 / 1_004.0; // K per m, dry adiabatic

fn run_cloud_base(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (c, ft, m) = (
        unit(QT::Temperature, "degC"),
        unit(QT::Length, "ft"),
        unit(QT::Length, "m"),
    );
    let t = ctx.req_quantity("temperature")?.to(c);
    let td = ctx.req_quantity("dew_point")?.to(c);
    if td > t {
        return Err(ToolError::invalid(
            "/dew_point",
            "The dew point cannot be above the temperature.",
        ));
    }
    if !(-60.0..=60.0).contains(&t) || td < -80.0 {
        return Err(ToolError::invalid(
            "/temperature",
            "Give surface readings between -60 °C and 60 °C.",
        ));
    }
    let rule_ft = (t - td) * 400.0;
    let (tk, tdk) = (t + 273.15, td + 273.15);
    let tl = 1.0 / (1.0 / (tdk - 56.0) + log(tk / tdk) / 800.0) + 56.0;
    let lcl_m = (tk - tl) / G_OVER_CP;
    let elev = ctx.quantity("elevation")?.map(|q| q.to(ft));
    let lapse = ctx.number("lapse_rate")?.unwrap_or(1.98);
    let mut out = vec![
        (
            "cloud_base_rule",
            ctx.out(
                "cloud_base_rule",
                Q {
                    value: rule_ft,
                    unit: ft,
                },
            ),
        ),
        (
            "cloud_base_lcl",
            ctx.out(
                "cloud_base_lcl",
                Q {
                    value: lcl_m,
                    unit: m,
                },
            ),
        ),
    ];
    if let Some(e) = elev {
        out.push((
            "cloud_base_msl",
            ctx.out(
                "cloud_base_msl",
                Q {
                    value: lcl_m / 0.3048 + e,
                    unit: ft,
                },
            ),
        ));
    }
    if t > 0.0 {
        let above = t / lapse * 1000.0;
        out.push((
            "freezing_level",
            ctx.out(
                "freezing_level",
                Q {
                    value: above + elev.unwrap_or(0.0),
                    unit: ft,
                },
            ),
        ));
    }
    Ok(obj(out))
}
