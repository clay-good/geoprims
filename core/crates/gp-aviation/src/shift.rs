//! Weight shift and ballast (add-aviation-suite, fuel and loading): how far
//! the CG moves when weight is moved, how much weight to move to reach a
//! CG, and how much ballast at a given arm brings the CG to a target, by the
//! handbook's proportions.

use crate::refs::*;
use crate::{obj, unit};
use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;

const fn qty(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    q: QT,
    u: &'static str,
) -> Field {
    Field::new(name, title, help, Kind::Quantity { q, unit: u })
}

const WB_SHIFT: Reference = Reference {
    locator: "Chapter 2 (shifting the CG: weight moved ÷ total weight = CG change ÷ distance moved; adding or removing weight and ballast)",
    ..WB_HANDBOOK
};

const TOTAL: Field = qty(
    "total_weight",
    "Total weight",
    "The loaded weight now, like 2250 lb",
    QT::Mass,
    "lb",
)
.required()
.core();
const CG: Field = qty(
    "cg",
    "CG now",
    "Inches aft of the datum, like 84.3 in",
    QT::Length,
    "in",
)
.required()
.core();

fn common(ctx: &mut Ctx) -> Result<(f64, f64), ToolError> {
    let w = ctx.req_quantity("total_weight")?.to(unit(QT::Mass, "lb"));
    let cg = ctx.req_quantity("cg")?.to(unit(QT::Length, "in"));
    if w.is_nan() || w <= 0.0 {
        return Err(ToolError::invalid(
            "/total_weight",
            "The total weight must be above zero.",
        ));
    }
    Ok((w, cg))
}

pub static WEIGHT_SHIFT: ToolDef = ToolDef {
    id: "aviation.loading.weight-shift",
    stability: gp_base::tool::Stability::Stable,
    title: "Weight shift",
    summary: "How far the CG moves when you move weight from one station to another, or how much weight to move to put the CG where you need it.",
    aliases: &[
        "weight shift formula",
        "shift CG",
        "move baggage CG",
        "CG change",
    ],
    keywords: &[
        "weight shift",
        "CG",
        "center of gravity",
        "move",
        "baggage",
        "station",
        "arm",
    ],
    inputs: &[
        TOTAL,
        CG,
        qty(
            "from_arm",
            "Moved from arm",
            "Like 150 in (the baggage area)",
            QT::Length,
            "in",
        )
        .required()
        .core(),
        qty(
            "to_arm",
            "Moved to arm",
            "Like 90 in (the front seats)",
            QT::Length,
            "in",
        )
        .required()
        .core(),
        qty(
            "weight",
            "Weight moved",
            "Like 50 lb; or give the target CG",
            QT::Mass,
            "lb",
        )
        .core(),
        qty(
            "target_cg",
            "Target CG",
            "To find the weight to move, like 83.5 in",
            QT::Length,
            "in",
        ),
    ],
    outputs: &[
        qty(
            "new_cg",
            "New CG",
            "After the move; the total weight is unchanged",
            QT::Length,
            "in",
        )
        .precision(Precision::Decimals(2)),
        qty(
            "weight",
            "Weight moved",
            "Total weight × CG change ÷ distance moved",
            QT::Mass,
            "lb",
        )
        .precision(Precision::Decimals(1)),
        qty(
            "cg_change",
            "CG change",
            "Positive is aft",
            QT::Length,
            "in",
        )
        .precision(Precision::Decimals(2)),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::NoSolution],
    warnings: &["UNIT_ASSUMED"],
    when_to_use: "Use this when a loading comes out of the CG envelope and you want to fix it by moving something already aboard: how far the CG moves when you shift a bag or a passenger between two stations, or how much weight must move to put the CG where you need it. It saves redoing the whole loading.",
    limitations: "The total weight stays the same, because nothing is added or removed; to add weight, use ballast or rerun weight and balance. The arms must be the aircraft's own stations from the same datum as the CG, and the new CG still has to be checked against the envelope at every weight the flight will pass through.",
    model: "Weight moved ÷ total weight = CG change ÷ distance moved (FAA-H-8083-1B, chapter 2)",
    accuracy: "Exact for the entered weights and arms",
    references: &[WB_SHIFT],
    examples: &[Example {
        id: "primary",
        title: "50 lb from the baggage area (150 in) to the front seats (90 in), 2,250 lb at 84.3 in",
        input: r#"{"total_weight":"2250 lb","cg":"84.3 in","from_arm":"150 in","to_arm":"90 in","weight":"50 lb"}"#,
        source: "Worked from the handbook proportion: 50 × (90 − 150) ÷ 2,250 = −1.33 in, so the CG moves to 82.97 in",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.loading.weight-balance",
            reason: "parent",
        },
        Related {
            id: "aviation.loading.ballast",
            reason: "alternative",
        },
        Related {
            id: "aviation.loading.fuel-weight",
            reason: "alternative",
        },
    ],
    sentence: "Moving {weight} puts the CG at {new_cg}, a change of {cg_change}.",
    limits: &[("batchRows", 10_000)],
    run: run_shift,
    ..ToolDef::BLANK
};

fn run_shift(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lb, inch) = (unit(QT::Mass, "lb"), unit(QT::Length, "in"));
    let (w, cg) = common(ctx)?;
    let from = ctx.req_quantity("from_arm")?.to(inch);
    let to = ctx.req_quantity("to_arm")?.to(inch);
    let d = to - from;
    if d == 0.0 {
        return Err(ToolError::invalid(
            "/to_arm",
            "Moving weight to the same arm does not change the CG.",
        ));
    }
    let (moved, new_cg) = match (
        ctx.quantity("weight")?.map(|q| q.to(lb)),
        ctx.quantity("target_cg")?.map(|q| q.to(inch)),
    ) {
        (Some(m), None) => {
            if !(0.0..=w).contains(&m) {
                return Err(ToolError::invalid(
                    "/weight",
                    "The weight moved must be between zero and the total weight.",
                ));
            }
            (m, cg + m * d / w)
        }
        (None, Some(t)) => {
            let m = w * (t - cg) / d;
            if m < 0.0 {
                return Err(ToolError::new(
                    ErrorCode::NoSolution,
                    "Moving weight that way takes the CG away from the target; swap the two arms.",
                )
                .at("/target_cg"));
            }
            if m > w {
                return Err(ToolError::new(
                    ErrorCode::NoSolution,
                    "It would take more than the whole airplane's weight to move the CG that far between these arms.",
                )
                .at("/target_cg"));
            }
            (m, t)
        }
        _ => {
            return Err(ToolError::invalid(
                "/weight",
                "Give either the weight moved or the target CG, not both.",
            ));
        }
    };
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, dp: u8| display::number(x, Precision::Decimals(dp), fmt);
        ctx.step(
            "CG change",
            "weight moved × distance moved ÷ total weight",
            format!("{} × {} ÷ {}", n(moved, 1), n(d, 2), n(w, 1)),
            format!("{} in", n(new_cg - cg, 2)),
        );
        ctx.step(
            "New CG",
            "CG now + CG change",
            format!("{} + {}", n(cg, 2), n(new_cg - cg, 2)),
            display::quantity(new_cg, "in", Precision::Decimals(2), fmt),
        );
    }
    Ok(obj(vec![
        (
            "new_cg",
            ctx.out(
                "new_cg",
                Q {
                    value: new_cg,
                    unit: inch,
                },
            ),
        ),
        (
            "weight",
            ctx.out(
                "weight",
                Q {
                    value: moved,
                    unit: lb,
                },
            ),
        ),
        (
            "cg_change",
            ctx.out(
                "cg_change",
                Q {
                    value: new_cg - cg,
                    unit: inch,
                },
            ),
        ),
    ]))
}

pub static BALLAST: ToolDef = ToolDef {
    id: "aviation.loading.ballast",
    stability: gp_base::tool::Stability::Stable,
    title: "Ballast to move the CG",
    summary: "How much ballast at a chosen arm brings the CG to a target, and the new total weight.",
    aliases: &[
        "ballast calculator",
        "ballast weight",
        "add weight to move CG",
    ],
    keywords: &[
        "ballast",
        "CG",
        "center of gravity",
        "add weight",
        "forward limit",
        "aft limit",
    ],
    inputs: &[
        TOTAL,
        CG,
        qty(
            "target_cg",
            "Target CG",
            "Like 82.0 in, inside the envelope",
            QT::Length,
            "in",
        )
        .required()
        .core(),
        qty(
            "ballast_arm",
            "Ballast arm",
            "Where the ballast goes, like 10 in (the nose)",
            QT::Length,
            "in",
        )
        .required()
        .core(),
    ],
    outputs: &[
        qty(
            "ballast",
            "Ballast weight",
            "Total weight × CG change ÷ (ballast arm − target CG)",
            QT::Mass,
            "lb",
        )
        .precision(Precision::Decimals(1)),
        qty(
            "new_weight",
            "New total weight",
            "Check it against the maximum weight",
            QT::Mass,
            "lb",
        )
        .precision(Precision::Decimals(1)),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::NoSolution],
    warnings: &["UNIT_ASSUMED"],
    when_to_use: "Use this when an aircraft's CG is out of limits and nothing aboard can be moved to fix it: how much ballast, fixed or temporary, has to go at a given station to bring the CG exactly to a target such as the nearest limit. It is the handbook's ballast formula with the arithmetic shown.",
    limitations: "The ballast adds weight, so the new total must still be under the maximum weight, and the new CG must be checked at every loading the ballast will fly with, not just this one. Permanent ballast is an alteration to the aircraft and needs the proper maintenance records; the arm must be a real place to mount it.",
    model: "Ballast = total weight × (target CG − CG now) ÷ (ballast arm − target CG), from taking moments about the datum (FAA-H-8083-1B, chapter 2)",
    accuracy: "Exact for the entered weights and arms. Check the new total weight against the maximum, and secure ballast as the aircraft's data requires",
    references: &[WB_SHIFT],
    examples: &[Example {
        id: "primary",
        title: "2,250 lb at 84.3 in, to 83.0 in with ballast at 10 in",
        input: r#"{"total_weight":"2250 lb","cg":"84.3 in","target_cg":"83.0 in","ballast_arm":"10 in"}"#,
        source: "Worked from moments: 2,250 × (83.0 − 84.3) ÷ (10 − 83.0) = 40.1 lb",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.loading.weight-balance",
            reason: "parent",
        },
        Related {
            id: "aviation.loading.weight-shift",
            reason: "alternative",
        },
        Related {
            id: "aviation.loading.fuel-weight",
            reason: "alternative",
        },
    ],
    sentence: "Add {ballast} of ballast; the airplane then weighs {new_weight}.",
    limits: &[("batchRows", 10_000)],
    run: run_ballast,
    ..ToolDef::BLANK
};

fn run_ballast(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (lb, inch) = (unit(QT::Mass, "lb"), unit(QT::Length, "in"));
    let (w, cg) = common(ctx)?;
    let t = ctx.req_quantity("target_cg")?.to(inch);
    let arm = ctx.req_quantity("ballast_arm")?.to(inch);
    if arm == t {
        return Err(ToolError::invalid(
            "/ballast_arm",
            "Ballast at the target CG itself cannot move the CG there.",
        ));
    }
    let b = w * (t - cg) / (arm - t);
    if b < 0.0 || !b.is_finite() {
        return Err(ToolError::new(
            ErrorCode::NoSolution,
            "Ballast at that arm moves the CG away from the target; put it on the other side of the target CG.",
        )
        .at("/ballast_arm"));
    }
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, dp: u8| display::number(x, Precision::Decimals(dp), fmt);
        ctx.step(
            "CG change needed",
            "target CG − CG now",
            format!("{} − {}", n(t, 2), n(cg, 2)),
            format!("{} in", n(t - cg, 2)),
        );
        ctx.step(
            "Ballast",
            "total weight × (target − CG) ÷ (arm − target)",
            format!(
                "{} × ({} − {}) ÷ ({} − {})",
                n(w, 1),
                n(t, 2),
                n(cg, 2),
                n(arm, 2),
                n(t, 2)
            ),
            display::quantity(b, "lb", Precision::Decimals(1), fmt),
        );
    }
    Ok(obj(vec![
        ("ballast", ctx.out("ballast", Q { value: b, unit: lb })),
        (
            "new_weight",
            ctx.out(
                "new_weight",
                Q {
                    value: w + b,
                    unit: lb,
                },
            ),
        ),
    ]))
}
