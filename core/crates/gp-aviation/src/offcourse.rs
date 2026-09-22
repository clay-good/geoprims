//! Off-course correction (add-aviation-suite, wind-and-navigation, "Off-course
//! correction (1-in-60)"): the track error from distance flown and distance
//! off course, the turn to parallel the course, and the turn to reach the
//! destination, each exactly and by the 1-in-60 rule.

use crate::refs::*;
use crate::{deg, obj, unit};
use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use libm::atan;

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
    locator: "Navigation chapter (dead reckoning, off-course correction, and the 1-in-60 rule)",
    ..PHAK
};

pub static ONE_IN_SIXTY: ToolDef = ToolDef {
    id: "aviation.wind.one-in-sixty",
    title: "Off-course correction (1-in-60)",
    summary: "How far you have drifted off track, the turn to parallel your course, and the turn to reach your destination, exactly and by the 1-in-60 rule.",
    aliases: &[
        "1 in 60 rule",
        "one in sixty rule",
        "off course correction",
        "track error",
        "double the track error",
    ],
    keywords: &[
        "1-in-60",
        "off course",
        "track error",
        "closing angle",
        "correction",
        "dead reckoning",
        "drift",
    ],
    inputs: &[
        qty(
            "flown",
            "Distance flown",
            "From the start to where you found the error, like 60 NM",
            QT::Distance,
            "NM",
        )
        .required()
        .core(),
        qty(
            "off_course",
            "Distance off course",
            "Square to the course line, like 4 NM",
            QT::Distance,
            "NM",
        )
        .required()
        .core(),
        qty(
            "remaining",
            "Distance remaining",
            "Along the course to the destination, like 60 NM",
            QT::Distance,
            "NM",
        )
        .required()
        .core(),
    ],
    outputs: &[
        qty(
            "to_destination",
            "Turn to reach the destination",
            "Track error plus closing angle, exact",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(2)),
        qty(
            "to_destination_rule",
            "Turn to reach the destination by the rule",
            "60 × off ÷ flown + 60 × off ÷ remaining",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(1)),
        qty(
            "track_error",
            "Track error, and the turn to parallel",
            "atan(off ÷ flown), exact",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(2)),
        qty(
            "track_error_rule",
            "Track error by the rule",
            "60 × off ÷ flown",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(1)),
        qty(
            "closing_angle",
            "Closing angle",
            "atan(off ÷ remaining), exact",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(2)),
        qty(
            "closing_angle_rule",
            "Closing angle by the rule",
            "60 × off ÷ remaining",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(1)),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Track error = atan(off ÷ flown); closing angle = atan(off ÷ remaining); turning by the track error parallels the course, and by the sum reaches the destination. The 1-in-60 rule takes 1 NM off in 60 NM as 1°, so each angle ≈ 60 × off ÷ distance (PHAK, navigation)",
    accuracy: "The exact angles are flat-plane geometry, fine for legs of a few hundred miles. The rule reads small (tan θ ≈ θ in radians, and 60 ≈ 57.3): about 5% high under 10°, and further off above 20°",
    references: &[PHAK_NAV],
    examples: &[Example {
        id: "primary",
        title: "4 NM off course after 60 NM, with 60 NM to go",
        input: r#"{"flown":"60 NM","off_course":"4 NM","remaining":"60 NM"}"#,
        source: "add-aviation-suite 1-in-60 scenario: track error 3.81° (rule 4°), turn to the destination 7.63° (rule 8°)",
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
            id: "aviation.wind.find-wind",
            reason: "next",
        },
        Related {
            id: "navigation.route.cross-track",
            reason: "alternative",
        },
    ],
    sentence: "Turn {to_destination} back toward the course to reach your destination, or {track_error} to parallel it.",
    limits: &[("batchRows", 10_000)],
    run: run_one_in_sixty,
    ..ToolDef::BLANK
};

fn run_one_in_sixty(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let nm = unit(QT::Distance, "NM");
    let flown = ctx.req_quantity("flown")?.to(nm);
    let off = ctx.req_quantity("off_course")?.to(nm);
    let remaining = ctx.req_quantity("remaining")?.to(nm);
    for (v, field) in [(flown, "/flown"), (remaining, "/remaining")] {
        if v <= 0.0 {
            return Err(ToolError::invalid(
                field,
                "The distances flown and remaining must be more than zero.",
            ));
        }
    }
    if off < 0.0 {
        return Err(ToolError::invalid(
            "/off_course",
            "Give the distance off course as a positive number; the turn is back toward the course.",
        ));
    }
    let te = atan(off / flown).to_degrees();
    let ca = atan(off / remaining).to_degrees();
    let (te_rule, ca_rule) = (60.0 * off / flown, 60.0 * off / remaining);
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Track error",
            "track error = atan(off ÷ flown)",
            format!("atan({} ÷ {})", n(off, 2), n(flown, 2)),
            format!("{}° (rule: {}°)", n(te, 2), n(te_rule, 1)),
        );
        ctx.step(
            "Closing angle",
            "closing angle = atan(off ÷ remaining)",
            format!("atan({} ÷ {})", n(off, 2), n(remaining, 2)),
            format!("{}° (rule: {}°)", n(ca, 2), n(ca_rule, 1)),
        );
        ctx.step(
            "Turn to the destination",
            "turn = track error + closing angle",
            format!("{}° + {}°", n(te, 2), n(ca, 2)),
            format!("{}°", n(te + ca, 2)),
        );
    }
    Ok(obj(vec![
        ("to_destination", ctx.out("to_destination", deg(te + ca))),
        (
            "to_destination_rule",
            ctx.out("to_destination_rule", deg(te_rule + ca_rule)),
        ),
        ("track_error", ctx.out("track_error", deg(te))),
        (
            "track_error_rule",
            ctx.out("track_error_rule", deg(te_rule)),
        ),
        ("closing_angle", ctx.out("closing_angle", deg(ca))),
        (
            "closing_angle_rule",
            ctx.out("closing_angle_rule", deg(ca_rule)),
        ),
    ]))
}
