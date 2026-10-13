//! Off-course correction (add-aviation-suite, wind-and-navigation, "Off-course
//! correction (1-in-60)"): the track error from distance flown and distance
//! off course, the turn to parallel the course, and the turn to reach the
//! destination, each exactly and by the 1-in-60 rule.

use crate::{deg, obj, unit};
use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{
    Assumption, Ctx, Example, Field, Kind, Layer, Precision, Reference, Related, ToolDef,
};
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

const AP3456_NAV: Reference = Reference {
    title: "AP3456: The Central Flying School Manual of Flying, Volume 9, Navigation",
    issuer: "UK Ministry of Defence",
    year: 2011,
    edition: "Volume 9, revised March 2011",
    locator: "Chapter 19, paragraphs 2-4 and Figure 3 (2 NM off after 30 NM gives 4°)",
    url: "https://assets.publishing.service.gov.uk/media/5e18b94740f0b65dcaba9817/Volume_09_Navigation.pdf",
};

pub static ONE_IN_SIXTY: ToolDef = ToolDef {
    id: "aviation.wind.one-in-sixty",
    version: "1.1.0",
    stability: gp_base::tool::Stability::Stable,
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
    warnings: &["UNIT_ASSUMED"],
    model: "Track error = atan(off ÷ flown); closing angle = atan(off ÷ remaining); turning by the track error parallels the course, and by the sum reaches the destination. The 1-in-60 rule takes 1 NM off in 60 NM as 1°, so each angle ≈ 60 × off ÷ distance (AP3456, chapter 19)",
    accuracy: "The exact angles are flat-plane geometry, fine for legs of a few hundred miles. The rule uses the small-angle approximation tan θ ≈ θ and 60 in place of 57.3, so it reads about 5% high under 10° and further off above 20°",
    when_to_use: "Use this when a checkpoint shows you are off course on a dead reckoning leg. Enter the distance flown, how far off course you are, and the distance left to go. It gives the turn to parallel the course and the turn to reach the destination, both exactly and by the 1-in-60 rule, so you can see how close the mental math is.",
    limitations: "It uses flat-plane geometry and assumes you flew a straight track from the start. A wind that changed partway will make the new heading wrong again. The rule reads a little high for small angles and worse past 20°. Confirm the correction at the next checkpoint and recheck the wind.",
    references: &[AP3456_NAV],
    examples: &[Example {
        id: "primary",
        title: "2 NM off course after 30 NM, with 30 NM to go",
        input: r#"{"flown":"30 NM","off_course":"2 NM","remaining":"30 NM"}"#,
        source: "UK Ministry of Defence AP3456, Volume 9, chapter 19, Figure 3: 2 NM off after 30 NM gives 4° by the rule",
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
    assumptions: &[Assumption {
        name: "1-in-60 angular factor",
        value: "60",
        unit: "deg",
        source: "uk-mod-ap3456",
    }],
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
