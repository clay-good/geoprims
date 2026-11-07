//! The ship herself (add-marine-navigation, navigation/under-keel-clearance):
//! what is left under the keel once the tide, the draft, the squat, and the
//! master's own margin are counted. Squat is estimated by Barrass's formula,
//! which is published with its parameter ranges; outside them the tool
//! refuses rather than guessing.

use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};

const PIANC_BARRASS: Reference = Reference {
    title: "Comparison between formulas of maximum ship squat",
    issuer: "\"Mircea cel Batran\" Naval Academy Scientific Bulletin (Constanta), Deleanu and Panaitescu",
    year: 2016,
    edition: "Volume XIX, Issue 1, pp. 105-111 (DOI 10.21279/1454-864X-16-I1-018, CC BY-NC-SA 4.0)",
    locator: "Table 1 (channel configurations and parameter constraints: Barrass, C_B 0.5 to 0.85, h/T 1.1 to 1.4), equations (2) and (3) (S_max = K C_B V_K^2 / 100, K = 5.74 S^0.76, with 0.10 <= S <= 0.25), and equation (4) (the other end of the ship)",
    url: "https://www.anmb.ro/buletinstiintific/buletine/2016_Issue1/NMS/105-111.pdf",
};
const BOWDITCH_SQUAT: Reference = Reference {
    title: "The American Practical Navigator (Bowditch), NGA Pub. No. 9, Volume I",
    issuer: "National Geospatial-Intelligence Agency",
    year: 2024,
    edition: "2024 edition",
    locator: "Chapter 9, Shiphandling in shallow water: squat, its growth with the square of the speed, and under-keel clearance as the master's judgment",
    url: "https://msi.nga.mil/Publications/APN",
};

/// Barrass's published ranges (PIANC Table 1 and the blockage-factor note).
const CB_MIN: f64 = 0.5;
const CB_MAX: f64 = 0.85;
const HT_MIN: f64 = 1.1;
const HT_MAX: f64 = 1.4;
const S_MIN: f64 = 0.10;
const S_MAX: f64 = 0.25;

const fn length(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Length,
            unit: "m",
        },
    )
    .precision(Precision::Decimals(2))
}

pub static UNDER_KEEL_CLEARANCE: ToolDef = ToolDef {
    id: "navigation.vessel.under-keel-clearance",
    title: "Under-keel clearance with squat",
    summary: "What is left under the keel from the charted depth, the tide, the draft, a squat estimate by Barrass's formula, and the margin you set, with every term of the stack shown.",
    aliases: &[
        "under keel clearance calculator",
        "UKC calculator",
        "ship squat calculator",
        "squat and clearance",
    ],
    keywords: &[
        "under-keel",
        "clearance",
        "squat",
        "draft",
        "depth",
        "tide",
        "Barrass",
        "ship",
    ],
    inputs: &[
        length(
            "charted_depth",
            "Charted depth",
            "At the shoalest point of the passage, on the chart datum, like 12.0 m",
        )
        .required()
        .core(),
        length(
            "draft",
            "Static draft",
            "The deepest static draft for the passage, like 10.8 m",
        )
        .required()
        .core(),
        length(
            "margin",
            "Your margin",
            "The clearance you or the port require under the keel, like 1.0 m",
        )
        .required()
        .core(),
        length(
            "tide",
            "Tide height",
            "Above the chart datum at the time, like 1.5 m; 0 without one",
        )
        .core(),
        Field::new(
            "speed",
            "Speed through the water",
            "For the squat estimate, like 10 kt",
            Kind::Quantity {
                q: QT::Speed,
                unit: "kt",
            },
        )
        .core()
        .precision(Precision::Decimals(1)),
        Field::new(
            "block_coefficient",
            "Block coefficient",
            "C_B from the ship's particulars, like 0.80",
            Kind::Number { min: 0.3, max: 1.0 },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "channel",
            "Channel",
            "Open water, a confined channel, or a blockage factor you work out, like open-water",
            Kind::Choice(&["open-water", "confined-channel", "blockage-factor"]),
        ),
        Field::new(
            "blockage_factor",
            "Blockage factor",
            "Midship section area divided by the channel's cross-section area, like 0.15",
            Kind::Number { min: 0.0, max: 1.0 },
        )
        .precision(Precision::Decimals(4)),
        length(
            "squat",
            "Squat",
            "Give your own instead of the estimate, like 0.6 m",
        ),
    ],
    outputs: &[
        length(
            "clearance",
            "Clearance after your margin",
            "Charted depth + tide − (draft + squat + margin)",
        ),
        length(
            "under_keel",
            "Water under the keel",
            "Charted depth + tide − (draft + squat)",
        ),
        Field::new(
            "clearance_ft",
            "Clearance after your margin, in feet",
            "The same number in feet",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .precision(Precision::Decimals(1)),
        length("squat_used", "Squat", "Estimated, or the one you gave"),
        Field::new(
            "squat_end",
            "Where she squats",
            "Bow above a block coefficient of 0.7, stern below it",
            Kind::Text { max_len: 40 },
        ),
        length(
            "squat_other_end",
            "Squat at the other end",
            "By Barrass's [1 − 40(0.7 − C_B)²] relation",
        )
        .optional(),
        length("water_depth", "Water depth", "Charted depth + tide"),
        length("keel_depth", "Draft with squat", "Draft + squat"),
        Field::new(
            "depth_to_draft",
            "Depth-to-draft ratio",
            "Water depth over static draft, like 1.25",
            Kind::Number {
                min: 0.0,
                max: 100.0,
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "stack",
            "The stack",
            "Every term, in meters",
            Kind::Text { max_len: 300 },
        ),
        Field::new(
            "notice",
            "Who decides",
            "Whose judgment the margin is",
            Kind::Text { max_len: 200 },
        ),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["BELOW_MARGIN", "EXPERIMENTAL_TOOL"],
    model: "Clearance = charted depth + tide − (draft + squat + margin). Squat by Barrass: S_max = K C_B V_K² / 100 with V_K in knots, K = 5.74 S^0.76 for a blockage factor S (K = 1 in open water, K ≈ 2 in a confined channel at S = 0.25), and the other end of the ship at [1 − 40(0.7 − C_B)²] S_max, the bow above C_B 0.7 and the stern below it",
    accuracy: "The arithmetic of the stack is exact in the units you give. Squat is an estimate from a regression on more than 600 model and ship measurements: published comparisons put Barrass above most other formulas for the same ship, by about a tenth of a meter at 8 knots on a 7,800 tdw cargo ship, so it is the cautious one of the set rather than the closest",
    when_to_use: "Use this when you plan a passage over a bar, a dredged channel, or any shoal patch and want the terms of under-keel clearance written out: the charted depth, the tide you expect at the time, your static draft, a squat estimate for the speed you intend, and the margin you or the port require. Send the tide height over from the tide prediction for the arrival hour. It shows the whole stack rather than one number, so you can see which term to change.",
    limitations: "Squat is an estimate, not a measurement, and Barrass's formula is published for block coefficients from 0.5 to 0.85 and depth-to-draft ratios from 1.1 to 1.4, with blockage factors from 0.10 to 0.25; outside those the tool refuses rather than extrapolate. It knows nothing of heel, list, trim, sea state, swell, density, or the state of the dredge, and the charted depth is the chart's, not today's. The margin is yours: the master and the port authority decide what is acceptable, and nothing here is a permission to proceed.",
    references: &[PIANC_BARRASS, BOWDITCH_SQUAT],
    examples: &[Example {
        id: "primary",
        title: "A 12.0 m channel, 1.5 m of tide, 10.8 m draft, 0.6 m of squat, 1.0 m margin",
        source: "add-marine-navigation navigation/under-keel-clearance, the Stack scenario: 2.1 m of water under the keel and 1.1 m left after the margin",
        input: r#"{"charted_depth":"12.0 m","tide":"1.5 m","draft":"10.8 m","squat":"0.6 m","margin":"1.0 m"}"#,
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[],
    }],
    related: &[
        Related {
            id: "navigation.tide.predict",
            reason: "parent",
        },
        Related {
            id: "navigation.tide.datums",
            reason: "alternative",
        },
        Related {
            id: "navigation.piloting.dead-reckoning",
            reason: "next",
        },
    ],
    sentence: "That leaves {under_keel} of water under the keel, {clearance} more than your margin.",
    limits: &[("batchRows", 10_000)],
    run: run_clearance,
    ..ToolDef::BLANK
};

/// Barrass's K for a blockage factor S, PIANC equation (3).
fn barrass_k(s: f64) -> f64 {
    5.74 * s.powf(0.76)
}

/// A length as the page shows it: two decimals with the trailing zeros
/// dropped, then the output's own unit, so the last explain step reads
/// exactly as the answer card does.
fn shown(v: f64, unit: &str) -> String {
    let s = format!("{v:.2}");
    let s = if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    };
    format!("{s} {unit}")
}

fn run_clearance(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let m_u = units::by_symbol(QT::Length, "m").expect("m");
    let ft_u = units::by_symbol(QT::Length, "ft").expect("ft");
    let kn_u = units::by_symbol(QT::Speed, "kt").expect("kn");
    let m = |v: f64| Q {
        value: v,
        unit: m_u,
    };
    let depth = ctx.req_quantity("charted_depth")?.to(m_u);
    let draft = ctx.req_quantity("draft")?.to(m_u);
    let margin = ctx.req_quantity("margin")?.to(m_u);
    let tide = ctx.quantity("tide")?.map_or(0.0, |q| q.to(m_u));
    if draft <= 0.0 {
        return Err(ToolError::invalid(
            "/draft",
            "The draft is a depth below the waterline, so it is more than zero.",
        ));
    }
    if margin < 0.0 {
        return Err(ToolError::invalid(
            "/margin",
            "A margin is a clearance you keep, so it is not negative.",
        ));
    }
    let water = depth + tide;
    if water <= 0.0 {
        return Err(ToolError::invalid(
            "/charted_depth",
            "The charted depth and the tide leave no water: there is nothing to float in.",
        ));
    }
    let ratio = water / draft;

    let (squat, end, other, how) = match ctx.quantity("squat")? {
        Some(q) => {
            let s = q.to(m_u);
            if s < 0.0 {
                return Err(ToolError::invalid(
                    "/squat",
                    "Squat is bodily sinkage, so it is not negative.",
                ));
            }
            (
                s,
                "as you gave it".to_string(),
                None,
                "your own squat".to_string(),
            )
        }
        None => {
            let speed = ctx.req_quantity("speed")?.to(kn_u);
            let cb = ctx.number("block_coefficient")?.ok_or_else(|| {
                ToolError::invalid(
                    "/block_coefficient",
                    "Give the block coefficient from the ship's particulars, or give the squat itself.",
                )
            })?;
            if speed < 0.0 {
                return Err(ToolError::invalid(
                    "/speed",
                    "Speed through the water is not negative.",
                ));
            }
            if !(CB_MIN..=CB_MAX).contains(&cb) {
                return Err(ToolError::new(
                    ErrorCode::OutOfDomain,
                    format!(
                        "Barrass's formula is published for block coefficients from {CB_MIN} to {CB_MAX}; this one is {cb:.3}."
                    ),
                )
                .at("/block_coefficient"));
            }
            if !(HT_MIN..=HT_MAX).contains(&ratio) {
                return Err(ToolError::new(
                    ErrorCode::OutOfDomain,
                    format!(
                        "Barrass's formula is published for depth-to-draft ratios from {HT_MIN} to {HT_MAX}; the water here is {ratio:.3} times the draft. Give the squat itself if you have it."
                    ),
                )
                .at("/charted_depth"));
            }
            let channel = ctx.choice("channel")?.unwrap_or("open-water");
            let (k, how) = match channel {
                "confined-channel" => (
                    barrass_k(S_MAX),
                    format!("a confined channel, blockage factor {S_MAX}"),
                ),
                "blockage-factor" => {
                    let s = ctx.number("blockage_factor")?.ok_or_else(|| {
                        ToolError::invalid(
                            "/blockage_factor",
                            "Give the blockage factor: the midship section area over the channel's cross-section area.",
                        )
                    })?;
                    if !(S_MIN..=S_MAX).contains(&s) {
                        return Err(ToolError::new(
                            ErrorCode::OutOfDomain,
                            format!(
                                "Barrass's K is published for blockage factors from {S_MIN} to {S_MAX}; this one is {s:.4}. Below {S_MIN} the channel is open water."
                            ),
                        )
                        .at("/blockage_factor"));
                    }
                    (barrass_k(s), format!("blockage factor {s:.4}"))
                }
                _ => (barrass_k(S_MIN), "open water".to_string()),
            };
            let smax = k * cb * speed * speed / 100.0;
            let other = (1.0 - 40.0 * (0.7 - cb).powi(2)) * smax;
            let end = if cb > 0.7 {
                "bow (block coefficient above 0.7)"
            } else if cb < 0.7 {
                "stern (block coefficient below 0.7)"
            } else {
                "both ends alike (block coefficient 0.7)"
            };
            if ctx.explaining() {
                ctx.step(
                    "Barrass's K",
                    "K = 5.74 S^0.76",
                    how.clone(),
                    format!("K {k:.4}"),
                );
                ctx.step(
                    "Maximum squat",
                    "S_max = K C_B V² / 100, V in knots",
                    format!("C_B {cb:.3}, {speed:.1} kn"),
                    format!("squat {smax:.2} m"),
                );
            }
            (smax, end.to_string(), Some(other), how)
        }
    };

    let keel = draft + squat;
    let under_keel = water - keel;
    let clearance = under_keel - margin;
    if clearance < 0.0 {
        ctx.warnings.push(Warning::new(
            "BELOW_MARGIN",
            format!(
                "This passage is {:.2} m short of the margin you set.",
                -clearance
            ),
        ));
    }
    if ctx.explaining() {
        ctx.step(
            "Water over the datum",
            "Charted depth + tide",
            format!("{depth:.2} m + {tide:.2} m"),
            format!("{water:.2} m"),
        );
        ctx.step(
            "What the ship draws",
            "Draft + squat",
            format!("{draft:.2} m + {squat:.2} m"),
            format!("{keel:.2} m"),
        );
        let unit = ctx.output_unit("clearance").symbol;
        ctx.step(
            "Clearance after the margin",
            "Water − (draft + squat) − margin",
            format!("{water:.2} m − {keel:.2} m − {margin:.2} m"),
            shown(clearance, unit),
        );
    }
    let stack = format!(
        "charted depth {depth:.2} + tide {tide:.2} − draft {draft:.2} − squat {squat:.2} − margin {margin:.2} = {clearance:.2}"
    );
    let mut out = vec![
        ("clearance", ctx.out("clearance", m(clearance))),
        ("under_keel", ctx.out("under_keel", m(under_keel))),
        (
            "clearance_ft",
            ctx.out(
                "clearance_ft",
                Q {
                    value: clearance / 0.3048,
                    unit: ft_u,
                },
            ),
        ),
        ("squat_used", ctx.out("squat_used", m(squat))),
        ("squat_end", Json::str(end)),
    ];
    if let Some(o) = other {
        out.push(("squat_other_end", ctx.out("squat_other_end", m(o))));
    }
    out.push(("water_depth", ctx.out("water_depth", m(water))));
    out.push(("keel_depth", ctx.out("keel_depth", m(keel))));
    out.push(("depth_to_draft", Json::Num((ratio * 1e3).round() / 1e3)));
    out.push(("stack", Json::str(format!("{stack} (m, {how})"))));
    out.push((
        "notice",
        Json::str("The master and the port authority decide what clearance is acceptable; this is arithmetic, not permission."),
    ));
    Ok(Json::obj(out))
}
