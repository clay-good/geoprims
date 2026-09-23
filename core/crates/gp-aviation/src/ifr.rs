//! Instrument procedures (aviation/instrument-procedures spec): holding entry,
//! holding wind correction and timing, and maximum holding speeds from the AIM
//! as dated reference data. Training and planning aids, not navigation.

use crate::refs::*;
use crate::{deg, knots, obj, unit};
use gp_base::angle::wrap_azimuth;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use gp_base::{ErrorCode, display};
use libm::{asin, cos, sin};

pub const AIM_HOLDING: Reference = Reference {
    title: "Aeronautical Information Manual",
    issuer: "Federal Aviation Administration",
    year: 2026,
    edition: "Current edition (rules as of 2026-09-18)",
    locator: "Chapter 5, Section 3, paragraph 5-3-8j (holding: entry procedures, FIG 5-3-4, and maximum holding airspeeds, TBL 5-3-24)",
    url: "https://www.faa.gov/air_traffic/publications/atpubs/aim_html/chap5_section_3.html",
};
pub const IFH: Reference = Reference {
    title: "Instrument Flying Handbook, FAA-H-8083-15B",
    issuer: "Federal Aviation Administration",
    year: 2012,
    edition: "FAA-H-8083-15B",
    locator: "Chapter 10 (holding: wind drift correction, triple the inbound drift outbound, and outbound leg timing)",
    url: "https://www.faa.gov/sites/faa.gov/files/regulations_policies/handbooks_manuals/aviation/FAA-H-8083-15B.pdf",
};
/// ICAO and Transport Canada state the 5° zone of flexibility on the sector
/// boundaries; the FAA AIM does not give one.
pub const PANS_OPS_HOLD: Reference = Reference {
    title: "Procedures for Air Navigation Services, Aircraft Operations (PANS-OPS), Doc 8168",
    issuer: "International Civil Aviation Organization",
    year: 2018,
    edition: "Volume I, 6th edition",
    locator: "Holding procedures: entry by heading against the three entry sectors, with a zone of flexibility of 5° on either side of the sector boundaries",
    url: "https://store.icao.int/en/procedures-for-air-navigation-services-pans-aircraft-operations-volume-i-flight-procedures-doc-8168",
};
pub const TC_AIM: Reference = Reference {
    title: "Transport Canada Aeronautical Information Manual (TC AIM), TP 14371E",
    issuer: "Transport Canada",
    year: 2026,
    edition: "AIM 2026-1, effective March 19, 2026",
    locator: "RAC 10.2 (holding at a clearance limit, example 2), RAC 10.5 (entry procedures, Figure 10.2, the 5° zone of flexibility)",
    url: "https://tc.canada.ca/en/aviation/publications/transport-canada-aeronautical-information-manual-tc-aim-tp-14371",
};

const fn qty(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    q: QT,
    u: &'static str,
) -> Field {
    Field::new(name, title, help, Kind::Quantity { q, unit: u })
}

const fn out(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    q: QT,
    u: &'static str,
    d: u8,
) -> Field {
    qty(name, title, help, q, u).precision(Precision::Decimals(d))
}

const TURNS: Field = Field::new(
    "turns",
    "Turns",
    "right (standard, default) or left (non-standard)",
    Kind::Choice(&["right", "left"]),
)
.core();

const INBOUND: Field = qty(
    "inbound_course",
    "Inbound course",
    "The holding course to the fix, like 360 deg",
    QT::Angle,
    "deg",
)
.required()
.core()
.angle_range("[0,360)");

fn dg(ctx: &mut Ctx, name: &str) -> Result<Option<f64>, ToolError> {
    Ok(ctx.quantity(name)?.map(|q| q.to(unit(QT::Angle, "deg"))))
}

/// Signed angle b − a in (−180, 180].
fn rel(a: f64, b: f64) -> f64 {
    let d = wrap_azimuth(b - a);
    if d > 180.0 { d - 360.0 } else { d }
}

/// The AIM 5-3-8 entry for an arrival heading, relative to the inbound course
/// (degrees, (−180, 180]), for right turns. Left turns mirror it.
/// Direct: −70° to +110°; teardrop: +110° to 180°; parallel: −180° to −70°.
/// A boundary heading takes the direct entry at ±70°/110° and the teardrop at
/// 180°, for either turn direction, so a left hold mirrors a right one exactly.
pub fn entry(relative: f64, left: bool) -> &'static str {
    let r = toward_holding_side(relative, left);
    if (-70.0..=110.0).contains(&r) {
        "direct"
    } else if r > 110.0 {
        "teardrop"
    } else {
        "parallel"
    }
}

/// The arrival angle measured positive toward the holding side, in
/// (−180, 180]: the relative angle for right turns, its mirror for left.
fn toward_holding_side(relative: f64, left: bool) -> f64 {
    let r = if left { -relative } else { relative };
    if r <= -180.0 { r + 360.0 } else { r }
}

/// Sector boundaries (relative, right turns) and the entries on each side.
const BOUNDARIES: [(f64, &str, &str); 3] = [
    (-70.0, "parallel", "direct"),
    (110.0, "direct", "teardrop"),
    (180.0, "teardrop", "parallel"),
];

// ---------------------------------------------------------------- entry

pub static HOLD_ENTRY: ToolDef = ToolDef {
    id: "aviation.ifr.hold-entry",
    version: "1.1.0",
    stability: gp_base::tool::Stability::Stable,
    title: "Holding pattern entry",
    summary: "Which holding entry to fly (direct, teardrop, or parallel) from your heading to the fix, the inbound course, and the turn direction, per AIM 5-3-8, with the headings for each entry.",
    aliases: &[
        "hold entry calculator",
        "holding entry",
        "direct teardrop parallel",
        "which hold entry",
    ],
    keywords: &[
        "hold",
        "holding",
        "entry",
        "direct",
        "teardrop",
        "parallel",
        "IFR",
        "70 degree",
        "AIM 5-3-8",
    ],
    inputs: &[
        INBOUND,
        TURNS,
        qty(
            "heading",
            "Heading to the fix",
            "Your heading (or course) as you arrive, like 090 deg",
            QT::Angle,
            "deg",
        )
        .required()
        .core()
        .angle_range("[0,360)"),
    ],
    outputs: &[
        Field::new(
            "entry",
            "Entry",
            "direct, teardrop, or parallel",
            Kind::Text { max_len: 12 },
        ),
        Field::new(
            "alternative",
            "Also acceptable",
            "The neighboring entry when within 5° of a sector boundary",
            Kind::Text { max_len: 12 },
        )
        .optional(),
        out(
            "relative_angle",
            "Arrival angle",
            "Heading minus inbound course, right turns positive toward the holding side",
            QT::Angle,
            "deg",
            0,
        ),
        out(
            "outbound_course",
            "Outbound course",
            "Inbound course + 180°",
            QT::Angle,
            "deg",
            0,
        )
        .angle_range("[0,360)"),
        out(
            "teardrop_heading",
            "Teardrop outbound heading",
            "30° from the outbound course toward the holding side",
            QT::Angle,
            "deg",
            0,
        )
        .angle_range("[0,360)"),
        out(
            "parallel_heading",
            "Parallel outbound heading",
            "The outbound course, flown on the non-holding side",
            QT::Angle,
            "deg",
            0,
        )
        .angle_range("[0,360)"),
        Field::new(
            "explanation",
            "What to do",
            "The entry in plain words",
            Kind::Text { max_len: 200 },
        ),
    ],
    warnings: &["UNIT_ASSUMED"],
    model: "AIM 5-3-8 sectors relative to the inbound course, split by the 70° line on the holding side: direct 180°, teardrop 70°, parallel 110°; mirrored for left turns; within 5° of a boundary either entry is acceptable (ICAO PANS-OPS and TC AIM zone of flexibility)",
    accuracy: "Exact sector geometry, using your heading as you reach the fix. ATC instructions and the published procedure govern.",
    when_to_use: "Use this when you are cleared to hold, or a hold is published at your clearance limit, and you want to know before you reach the fix which entry to fly: direct, teardrop, or parallel. Give the inbound course, the turn direction, and your heading to the fix, and it names the entry, the headings to fly, and whether you are close enough to a sector line that the next entry is also fine.",
    limitations: "The sectors are read from your heading as you reach the fix. In a strong crosswind your heading and your track differ, and the FAA draws the sectors by where you come from, so near a boundary work from the course you are flying to the fix. It does not know the holding fix type: at a VOR intersection or a DME fix, ICAO and Transport Canada limit the entry to the radials or arc that form the fix. It says nothing about wind correction or timing on the legs, the maximum holding speed, or an RNAV system that flies the entry for you with a fly-by turn. ATC instructions and the published procedure come first.",
    references: &[AIM_HOLDING, IFH, PANS_OPS_HOLD, TC_AIM],
    examples: &[
        Example {
            id: "primary",
            title: "Inbound 360°, right turns, arriving on heading 090°",
            input: r#"{"inbound_course":"360 deg","heading":"90 deg"}"#,
            source: "AIM 5-3-8 FIG 5-3-4: an arrival 90° right of the inbound course is in the direct sector",
        },
        Example {
            id: "teardrop",
            title: "Inbound 360°, right turns, arriving on heading 150°",
            input: r#"{"inbound_course":"360 deg","heading":"150 deg"}"#,
            source: "AIM 5-3-8 FIG 5-3-4: teardrop sector (110° to 180° right of the inbound course)",
        },
        Example {
            id: "missed-approach",
            title: "Missed approach to the ZHZ beacon, holding on the inbound track of 234°",
            input: r#"{"inbound_course":"234 deg","heading":"234 deg","turns":"right"}"#,
            source: "TC AIM RAC 10.2 example 2: proceed directly to the beacon, make a right turn and hold on an inbound track of 234°",
        },
    ],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[("heading", "relative_angle")],
    }],
    related: &[
        Related {
            id: "aviation.ifr.hold-wind-timing",
            reason: "next",
        },
        Related {
            id: "aviation.ifr.hold-speed-limit",
            reason: "next",
        },
        Related {
            id: "aviation.wind.heading-groundspeed",
            reason: "next",
        },
    ],
    sentence: "Fly a {entry} entry. {explanation}",
    limits: &[("batchRows", 10_000)],
    run: run_hold_entry,
    ..ToolDef::BLANK
};

fn run_hold_entry(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let ic = dg(ctx, "inbound_course")?.expect("required");
    let h = dg(ctx, "heading")?.expect("required");
    let left = ctx.choice("turns")? == Some("left");
    let r = rel(ic, h);
    let e = entry(r, left);
    let oc = wrap_azimuth(ic + 180.0);
    // The holding side is right of the inbound course for right turns.
    let teardrop = wrap_azimuth(if left { oc + 30.0 } else { oc - 30.0 });
    let rr = toward_holding_side(r, left);
    let alt = BOUNDARIES.iter().find_map(|(b, lo, hi)| {
        let d = if *b == 180.0 {
            180.0 - rr.abs()
        } else {
            (rr - b).abs()
        };
        (d <= 5.0).then_some(if e == *lo { *hi } else { *lo })
    });
    // Headings read 001° to 360°, as pilots say them.
    let show = |x: f64| {
        let v = (wrap_azimuth(x).round() as i64) % 360;
        format!("{:03}°", if v == 0 { 360 } else { v })
    };
    let side = if left { "left" } else { "right" };
    let mut explanation = match e {
        "direct" => format!(
            "Cross the fix and turn {side} to the outbound course, {}.",
            show(oc)
        ),
        "teardrop" => format!(
            "Cross the fix, fly {} for about 1 minute, then turn {side} to intercept the inbound course.",
            show(teardrop)
        ),
        _ => format!(
            "Cross the fix, turn to parallel the outbound course on the non-holding side ({}) for about 1 minute, then turn {} back to the fix.",
            show(oc),
            if left { "right" } else { "left" }
        ),
    };
    if let Some(a) = alt {
        explanation.push_str(&format!(
            " You are on the boundary: a {a} entry is also acceptable."
        ));
    }
    if ctx.explaining() {
        // The sectors are defined by a figure, so the work is the angle the
        // figure is read at, and which sector it falls in.
        // A relative angle is not a heading: 0° prints as 0°, not 360°.
        let signed = |x: f64| {
            let v = x.round();
            format!("{}{}°", if v < 0.0 { "−" } else { "" }, v.abs())
        };
        ctx.step(
            "Angle onto the inbound course",
            "the heading measured against the inbound course",
            format!("heading {} against inbound {}", show(h), show(ic)),
            signed(r),
        );
        ctx.step(
            "Sector",
            format!("the entry sector that angle falls in, measured toward the holding side, for a hold with turns to the {side}"),
            format!("{} toward the holding side", signed(rr)),
            e.to_owned(),
        );
    }
    let mut o = vec![("entry", Json::str(e))];
    if let Some(a) = alt {
        o.push(("alternative", Json::str(a)));
    }
    o.extend([
        ("relative_angle", ctx.out("relative_angle", deg(r))),
        ("outbound_course", ctx.out("outbound_course", deg(oc))),
        (
            "teardrop_heading",
            ctx.out("teardrop_heading", deg(teardrop)),
        ),
        ("parallel_heading", ctx.out("parallel_heading", deg(oc))),
        ("explanation", Json::str(explanation)),
    ]);
    Ok(obj(o))
}

// ---------------------------------------------------------------- speed limit

/// AIM table 5-3-1 maximum holding airspeeds (KIAS) by altitude band (ft MSL).
pub const HOLD_SPEEDS: &[(f64, f64, &str)] = &[
    (6_000.0, 200.0, "minimum holding altitude through 6,000 ft"),
    (14_000.0, 230.0, "6,001 ft through 14,000 ft"),
    (f64::INFINITY, 265.0, "14,001 ft and above"),
];

fn max_hold_speed(alt_ft: f64) -> (f64, &'static str) {
    let (_, v, band) = HOLD_SPEEDS
        .iter()
        .find(|(top, _, _)| alt_ft <= *top)
        .expect("last band is open");
    (*v, band)
}

pub static HOLD_SPEED: ToolDef = ToolDef {
    id: "aviation.ifr.hold-speed-limit",
    title: "Maximum holding airspeed",
    summary: "The maximum holding airspeed for your altitude from the AIM table (200, 230, or 265 KIAS), or a published limit, with a flag when your planned speed is above it.",
    aliases: &[
        "holding speed",
        "max holding speed",
        "holding airspeed limit",
    ],
    keywords: &[
        "holding",
        "speed limit",
        "KIAS",
        "200 knots",
        "230 knots",
        "265 knots",
        "AIM 5-3-8",
    ],
    inputs: &[
        qty(
            "altitude",
            "Holding altitude",
            "Like 8000 ft MSL",
            QT::Length,
            "ft",
        )
        .required()
        .core(),
        qty(
            "planned_ias",
            "Planned airspeed",
            "Indicated, like 210 kt",
            QT::Speed,
            "kt",
        )
        .core(),
        qty(
            "published_limit",
            "Published limit",
            "Only if the chart shows one, like 175 kt",
            QT::Speed,
            "kt",
        ),
    ],
    outputs: &[
        out(
            "max_ias",
            "Maximum holding airspeed",
            "Indicated airspeed",
            QT::Speed,
            "kt",
            0,
        ),
        Field::new(
            "basis",
            "Basis",
            "The AIM altitude band or the published limit",
            Kind::Text { max_len: 80 },
        ),
        Field::new(
            "status",
            "Planned speed",
            "Within the limit or above it",
            Kind::Text { max_len: 40 },
        )
        .optional(),
    ],
    warnings: &[
        "ABOVE_MAX_HOLDING_SPEED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "AIM table 5-3-1: 200 KIAS through 6,000 ft, 230 KIAS from 6,001 to 14,000 ft, 265 KIAS above; a published limit on the chart overrides",
    accuracy: "Reference data as of the AIM edition cited. Military fields and some procedures use other limits; the chart and ATC govern.",
    references: &[AIM_HOLDING],
    examples: &[Example {
        id: "primary",
        title: "Holding at 8,000 ft planning 240 KIAS",
        input: r#"{"altitude":"8000 ft","planned_ias":"240 kt"}"#,
        source: "AIM 5-3-8 table 5-3-1: 230 KIAS from 6,001 ft through 14,000 ft",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "gauge",
        map: &[("value", "max_ias")],
    }],
    related: &[Related {
        id: "aviation.ifr.hold-entry",
        reason: "alternative",
    }],
    sentence: "The maximum holding speed is {max_ias} ({basis}).{warn ABOVE_MAX_HOLDING_SPEED} Your planned speed is above it.{/warn}",
    limits: &[("batchRows", 10_000)],
    run: run_hold_speed,
    ..ToolDef::BLANK
};

fn run_hold_speed(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let alt = ctx.req_quantity("altitude")?.to(unit(QT::Length, "ft"));
    if alt < 0.0 {
        return Err(ToolError::invalid(
            "/altitude",
            "The holding altitude cannot be negative.",
        ));
    }
    let kt = unit(QT::Speed, "kt");
    let (limit, basis) = match ctx.quantity("published_limit")? {
        Some(p) => (p.to(kt), "published on the chart".to_owned()),
        None => {
            let (v, band) = max_hold_speed(alt);
            (v, format!("AIM table 5-3-1, {band}"))
        }
    };
    let mut o = vec![
        ("max_ias", ctx.out("max_ias", knots(limit))),
        ("basis", Json::str(&basis)),
    ];
    if let Some(p) = ctx.quantity("planned_ias")? {
        let p = p.to(kt);
        if p > limit {
            ctx.warnings.push(
                Warning::new(
                    "ABOVE_MAX_HOLDING_SPEED",
                    format!(
                        "{} is above the {} limit ({basis}). Slow down 3 minutes before the fix (AIM 5-3-8).",
                        display::quantity(p, "kt", Precision::Decimals(0), ctx.options.format),
                        display::quantity(limit, "kt", Precision::Decimals(0), ctx.options.format)
                    ),
                )
                .at("/planned_ias"),
            );
            o.push(("status", Json::str("above the limit")));
        } else {
            o.push(("status", Json::str("within the limit")));
        }
    }
    Ok(obj(o))
}

// ---------------------------------------------------------------- wind and timing

pub static HOLD_WIND: ToolDef = ToolDef {
    id: "aviation.ifr.hold-wind-timing",
    title: "Holding wind correction and timing",
    summary: "Inbound and outbound headings for the wind, the outbound time that gives a 1-minute inbound leg, and the triple-the-drift rule of thumb beside the exact values.",
    aliases: &[
        "holding wind correction",
        "hold timing",
        "triple the drift",
        "outbound leg time",
    ],
    keywords: &[
        "holding",
        "wind correction",
        "WCA",
        "outbound",
        "inbound",
        "timing",
        "triple drift",
    ],
    inputs: &[
        INBOUND,
        TURNS,
        qty("tas", "True airspeed", "Like 120 kt", QT::Speed, "kt")
            .required()
            .core(),
        qty(
            "wind_direction",
            "Wind from",
            "True or the same reference as the course, like 300 deg",
            QT::Angle,
            "deg",
        )
        .required()
        .core()
        .angle_range("[0,360)"),
        qty("wind_speed", "Wind speed", "Like 20 kt", QT::Speed, "kt")
            .required()
            .core(),
        qty(
            "inbound_time",
            "Inbound leg time",
            "Default 1 min; the AIM uses 1.5 min above 14,000 ft",
            QT::Time,
            "min",
        ),
    ],
    outputs: &[
        out(
            "inbound_heading",
            "Inbound heading",
            "Holds the inbound course in this wind",
            QT::Angle,
            "deg",
            0,
        )
        .angle_range("[0,360)"),
        out(
            "inbound_wca",
            "Inbound wind correction",
            "Right positive",
            QT::Angle,
            "deg",
            1,
        ),
        out(
            "outbound_heading",
            "Outbound heading (triple the drift)",
            "Outbound course minus 3 × the inbound correction",
            QT::Angle,
            "deg",
            0,
        )
        .angle_range("[0,360)"),
        out(
            "outbound_track_heading",
            "Outbound heading to hold the track",
            "Straight-leg wind correction only",
            QT::Angle,
            "deg",
            0,
        )
        .angle_range("[0,360)"),
        out(
            "inbound_groundspeed",
            "Inbound groundspeed",
            "Along the inbound course",
            QT::Speed,
            "kt",
            0,
        ),
        out(
            "outbound_groundspeed",
            "Outbound groundspeed",
            "Along the outbound course",
            QT::Speed,
            "kt",
            0,
        ),
        out(
            "outbound_time",
            "Outbound time",
            "Gives the target inbound time on straight legs",
            QT::Time,
            "s",
            0,
        ),
    ],
    errors: &[ErrorCode::NoSolution],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Straight-leg wind triangle on each leg; outbound time = inbound GS × inbound time / outbound GS; the triple-the-drift heading (FAA-H-8083-15B) allows for the turns and is a rule of thumb",
    accuracy: "Exact for straight legs in a steady wind. Wind drift in the turns is not modeled; adjust on the next circuit.",
    references: &[IFH, AIM_HOLDING, PHAK],
    examples: &[Example {
        id: "primary",
        title: "Inbound 360°, 120 KTAS, wind 300° at 20 kt",
        input: r#"{"inbound_course":"360 deg","tas":"120 kt","wind_direction":"300 deg","wind_speed":"20 kt"}"#,
        source: "FAA-H-8083-15B chapter 10 method: correct into the wind inbound, triple it outbound",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[("heading", "outbound_heading")],
    }],
    related: &[Related {
        id: "aviation.ifr.hold-entry",
        reason: "alternative",
    }],
    sentence: "Fly {inbound_heading} inbound and {outbound_heading} outbound for {outbound_time}.",
    limits: &[("batchRows", 10_000)],
    run: run_hold_wind,
    ..ToolDef::BLANK
};

/// Wind correction angle (degrees, right positive) and groundspeed for a course.
fn wca(course: f64, tas: f64, wd: f64, ws: f64) -> Option<(f64, f64)> {
    let a = (wd - course).to_radians();
    let s = ws / tas * sin(a);
    if s.abs() > 1.0 {
        return None;
    }
    let w = asin(s);
    let gs = tas * cos(w) - ws * cos(a);
    (gs > 0.0).then_some((w.to_degrees(), gs))
}

fn run_hold_wind(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let ic = dg(ctx, "inbound_course")?.expect("required");
    let kt = unit(QT::Speed, "kt");
    let tas = ctx.req_quantity("tas")?.to(kt);
    let wd = dg(ctx, "wind_direction")?.expect("required");
    let ws = ctx.req_quantity("wind_speed")?.to(kt);
    if tas <= 0.0 || ws < 0.0 {
        return Err(ToolError::invalid(
            "/tas",
            "True airspeed must be positive and wind speed cannot be negative.",
        ));
    }
    let t_in = ctx.quantity("inbound_time")?.map_or(60.0, |t| t.base());
    if t_in <= 0.0 {
        return Err(ToolError::invalid(
            "/inbound_time",
            "Inbound leg time must be positive.",
        ));
    }
    let oc = wrap_azimuth(ic + 180.0);
    let none = || {
        ToolError::new(
            ErrorCode::NoSolution,
            "The wind is too strong for this airspeed to hold the course.",
        )
        .at("/wind_speed")
    };
    let (w_in, gs_in) = wca(ic, tas, wd, ws).ok_or_else(none)?;
    let (w_out, gs_out) = wca(oc, tas, wd, ws).ok_or_else(none)?;
    let s = |v: f64| Q {
        value: v,
        unit: unit(QT::Time, "s"),
    };
    Ok(obj(vec![
        (
            "inbound_heading",
            ctx.out("inbound_heading", deg(wrap_azimuth(ic + w_in))),
        ),
        ("inbound_wca", ctx.out("inbound_wca", deg(w_in))),
        (
            "outbound_heading",
            ctx.out("outbound_heading", deg(wrap_azimuth(oc - 3.0 * w_in))),
        ),
        (
            "outbound_track_heading",
            ctx.out("outbound_track_heading", deg(wrap_azimuth(oc + w_out))),
        ),
        (
            "inbound_groundspeed",
            ctx.out("inbound_groundspeed", knots(gs_in)),
        ),
        (
            "outbound_groundspeed",
            ctx.out("outbound_groundspeed", knots(gs_out)),
        ),
        (
            "outbound_time",
            ctx.out("outbound_time", s(gs_in * t_in / gs_out)),
        ),
    ]))
}
