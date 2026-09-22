//! The last two wind-triangle forms (add-aviation-suite, wind-and-navigation,
//! "Wind triangle, all forms"): the true airspeed and heading that make good
//! a course at a groundspeed, and the course and groundspeed from a heading
//! and airspeed. Vectors add head to tail: ground = air + wind.

use crate::refs::*;
use crate::{REFS, WIND_FIELDS, deg, knots, obj, read_wind, to_reference, unit, variation};
use gp_base::ErrorCode;
use gp_base::angle::{wrap_azimuth, wrap_lon};
use gp_base::display;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Related, ToolDef};
use gp_base::units::Quantity as QT;
use libm::{atan2, cos, sin, sqrt};

const fn angle(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .angle_range("[0,360)")
}

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

/// East and north components of a vector of `s` toward `az` degrees.
fn en(az: f64, s: f64) -> (f64, f64) {
    let a = az.to_radians();
    (s * sin(a), s * cos(a))
}

/// Direction (deg, [0, 360)) and length of an east/north vector.
fn polar(e: f64, n: f64) -> (f64, f64) {
    (wrap_azimuth(atan2(e, n).to_degrees()), sqrt(e * e + n * n))
}

/// The wind in `reference`, as (from direction, speed); variable winds refused.
fn steady_wind(ctx: &mut Ctx, reference: &str) -> Result<(f64, f64), ToolError> {
    let (w, wind_ref) = read_wind(ctx, "true")?;
    let var = variation(ctx)?;
    match w.dir {
        Some(d) => Ok((
            to_reference(d, wind_ref, reference, var, "/wind_reference")?,
            w.speed,
        )),
        None => Err(ToolError::invalid(
            "/wind",
            "The wind triangle needs a steady wind direction, not a variable one.",
        )),
    }
}

const WIND_REF: Field = Field::new(
    "wind_reference",
    "Wind reference",
    "true (the default: METAR, TAF, winds aloft) or magnetic",
    Kind::Choice(REFS),
);

pub static TAS_FROM_GROUNDSPEED: ToolDef = ToolDef {
    id: "aviation.wind.tas-from-groundspeed",
    title: "Wind triangle: airspeed and heading for a groundspeed",
    summary: "The true airspeed and heading that make good a course at the groundspeed you need, for a known wind.",
    aliases: &[
        "airspeed for groundspeed",
        "TAS for groundspeed",
        "required true airspeed",
    ],
    keywords: &[
        "true airspeed",
        "groundspeed",
        "heading",
        "course",
        "wind triangle",
        "E6B",
        "arrival time",
    ],
    inputs: &[
        angle("course", "Course", "Like 090 deg").required().core(),
        speed("groundspeed", "Groundspeed needed", "Like 108.7 kt")
            .required()
            .core(),
        WIND_FIELDS[0],
        WIND_FIELDS[1],
        Field::new(
            "course_reference",
            "Course reference",
            "true (the default, as plotted on a chart) or magnetic",
            Kind::Choice(REFS),
        ),
        WIND_REF,
        WIND_FIELDS[3],
        WIND_FIELDS[5],
    ],
    outputs: &[
        speed("tas", "True airspeed", "The airspeed to fly").precision(Precision::Decimals(1)),
        angle(
            "heading",
            "Heading",
            "The heading to fly, in the course's reference",
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "wind_correction_angle",
            "Wind correction angle",
            "Negative is a correction to the left",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(1))
        .angle_range("unbounded"),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Air vector = ground vector − wind vector: the ground vector runs along the course at the groundspeed, and the wind vector points where the wind blows to (its direction + 180°). TAS is the air vector's length and the heading its direction",
    accuracy: "Exact for steady wind and flat-earth geometry over a leg. Planning aid, not certified for navigation.",
    references: &[PHAK, AIM],
    examples: &[Example {
        id: "primary",
        title: "Course 090° at 108.7 kt over the ground, wind 030° at 20 kt",
        input: r#"{"course":"90 deg","groundspeed":"108.7 kt","wind_direction":"30 deg","wind_speed":"20 kt"}"#,
        source: "add-aviation-suite heading-and-groundspeed scenario solved the other way: about 120 kt on heading 081.7°",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.wind.heading-groundspeed",
            reason: "alternative",
        },
        Related {
            id: "aviation.wind.course-from-heading",
            reason: "alternative",
        },
    ],
    sentence: "Fly {tas} true airspeed on heading {heading} to make good the course at that groundspeed.",
    limits: &[("batchRows", 10_000)],
    run: run_tas_from_groundspeed,
    ..ToolDef::BLANK
};

fn run_tas_from_groundspeed(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let dg = unit(QT::Angle, "deg");
    let course = wrap_azimuth(ctx.req_quantity("course")?.to(dg));
    let gs = ctx.req_quantity("groundspeed")?.to(unit(QT::Speed, "kt"));
    if gs <= 0.0 {
        return Err(ToolError::invalid(
            "/groundspeed",
            "The groundspeed must be greater than zero.",
        ));
    }
    let course_ref = ctx.choice("course_reference")?.unwrap_or("true");
    let (wd, ws) = steady_wind(ctx, course_ref)?;
    let (ge, gn) = en(course, gs);
    let (we, wn) = en(wd + 180.0, ws);
    let (heading, tas) = polar(ge - we, gn - wn);
    let wca = wrap_lon(heading - course);
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Air vector",
            "air = ground − wind (east, north)",
            format!(
                "({}, {}) − ({}, {})",
                n(ge, 1),
                n(gn, 1),
                n(we, 1),
                n(wn, 1)
            ),
            format!("({}, {}) kt", n(ge - we, 1), n(gn - wn, 1)),
        );
        ctx.step(
            "Heading",
            "heading = atan2(east, north)",
            format!("atan2({}, {})", n(ge - we, 1), n(gn - wn, 1)),
            format!("{}°", n(heading, 1)),
        );
        ctx.step(
            "True airspeed",
            "TAS = |air|",
            format!("√({}² + {}²)", n(ge - we, 1), n(gn - wn, 1)),
            format!("{} kt", n(tas, 1)),
        );
    }
    Ok(obj(vec![
        ("tas", ctx.out("tas", knots(tas))),
        ("heading", ctx.out("heading", deg(heading))),
        (
            "wind_correction_angle",
            ctx.out("wind_correction_angle", deg(wca)),
        ),
    ]))
}

pub static COURSE_FROM_HEADING: ToolDef = ToolDef {
    id: "aviation.wind.course-from-heading",
    title: "Wind triangle: course and groundspeed from a heading",
    summary: "Where a heading and true airspeed will take you in a known wind: the course over the ground, the groundspeed, and the drift.",
    aliases: &["track from heading", "drift angle", "course made good"],
    keywords: &[
        "track",
        "course",
        "groundspeed",
        "drift",
        "heading",
        "wind triangle",
        "E6B",
    ],
    inputs: &[
        angle("heading", "Heading", "Like 081.7 deg")
            .required()
            .core(),
        speed("tas", "True airspeed", "Like 120 kt")
            .required()
            .core(),
        WIND_FIELDS[0],
        WIND_FIELDS[1],
        Field::new(
            "heading_reference",
            "Heading reference",
            "true (the default) or magnetic",
            Kind::Choice(REFS),
        ),
        WIND_REF,
        WIND_FIELDS[3],
        WIND_FIELDS[5],
    ],
    outputs: &[
        angle(
            "course",
            "Course over the ground",
            "The track, in the heading's reference",
        )
        .precision(Precision::Decimals(1)),
        speed("groundspeed", "Groundspeed", "Speed over the ground")
            .precision(Precision::Decimals(1)),
        Field::new(
            "drift_angle",
            "Drift angle",
            "Course minus heading; negative is drift to the left",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(1))
        .angle_range("unbounded"),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::NoSolution],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Ground vector = air vector + wind vector: the air vector runs along the heading at TAS, and the wind vector points where the wind blows to (its direction + 180°). The course is the ground vector's direction and the groundspeed its length",
    accuracy: "Exact for steady wind and flat-earth geometry over a leg. Planning aid, not certified for navigation.",
    references: &[PHAK, AIM],
    examples: &[Example {
        id: "primary",
        title: "Heading 081.7° at 120 kt, wind 030° at 20 kt",
        input: r#"{"heading":"81.7 deg","tas":"120 kt","wind_direction":"30 deg","wind_speed":"20 kt"}"#,
        source: "add-aviation-suite heading-and-groundspeed scenario solved the other way: course about 090° at 108.7 kt",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.wind.heading-groundspeed",
            reason: "alternative",
        },
        Related {
            id: "aviation.wind.find-wind",
            reason: "alternative",
        },
    ],
    sentence: "Heading {heading} makes good a course of {course} at {groundspeed} over the ground.",
    limits: &[("batchRows", 10_000)],
    run: run_course_from_heading,
    ..ToolDef::BLANK
};

fn run_course_from_heading(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let dg = unit(QT::Angle, "deg");
    let heading = wrap_azimuth(ctx.req_quantity("heading")?.to(dg));
    let tas = ctx.req_quantity("tas")?.to(unit(QT::Speed, "kt"));
    if tas <= 0.0 {
        return Err(ToolError::invalid(
            "/tas",
            "True airspeed must be greater than zero.",
        ));
    }
    let heading_ref = ctx.choice("heading_reference")?.unwrap_or("true");
    let (wd, ws) = steady_wind(ctx, heading_ref)?;
    let (ae, an) = en(heading, tas);
    let (we, wn) = en(wd + 180.0, ws);
    let (course, gs) = polar(ae + we, an + wn);
    if gs < 1e-9 {
        return Err(ToolError::new(
            ErrorCode::NoSolution,
            "The wind exactly cancels the airspeed: the aircraft hovers over one spot, with no course over the ground.",
        ));
    }
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Ground vector",
            "ground = air + wind (east, north)",
            format!(
                "({}, {}) + ({}, {})",
                n(ae, 1),
                n(an, 1),
                n(we, 1),
                n(wn, 1)
            ),
            format!("({}, {}) kt", n(ae + we, 1), n(an + wn, 1)),
        );
        ctx.step(
            "Groundspeed",
            "GS = |ground|",
            format!("√({}² + {}²)", n(ae + we, 1), n(an + wn, 1)),
            format!("{} kt", n(gs, 1)),
        );
        ctx.step(
            "Course over the ground",
            "course = atan2(east, north)",
            format!("atan2({}, {})", n(ae + we, 1), n(an + wn, 1)),
            format!("{}°", n(course, 1)),
        );
    }
    Ok(obj(vec![
        ("course", ctx.out("course", deg(course))),
        ("groundspeed", ctx.out("groundspeed", knots(gs))),
        (
            "drift_angle",
            ctx.out("drift_angle", deg(wrap_lon(course - heading))),
        ),
    ]))
}
