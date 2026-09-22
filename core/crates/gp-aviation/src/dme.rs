//! DME and station geometry (add-practitioner-essentials,
//! aviation/instrument-procedures, "DME geometry"): ground distance from a
//! DME slant range, time and distance to a station from a timed bearing
//! change, and lead points for flying a DME arc. Training and planning aids,
//! not navigation.

use crate::ifr::IFH;
use crate::{obj, unit};
use gp_base::ErrorCode;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use libm::{asin, sin, sqrt};

const IFH_NAV: Reference = Reference {
    locator: "Chapter 9 (navigation systems: DME slant range, time and distance to a station from a bearing change, DME arcs)",
    ..IFH
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

const FT_PER_NM: f64 = 1852.0 / 0.3048;

pub static SLANT: ToolDef = ToolDef {
    id: "aviation.ifr.dme-slant-range",
    title: "DME slant range to ground distance",
    summary: "The distance over the ground to a DME station from the DME reading and your height above the station, and how much the reading overstates it.",
    aliases: &[
        "DME slant range",
        "slant range correction",
        "DME ground distance",
        "slant range error",
    ],
    keywords: &[
        "DME",
        "slant range",
        "ground distance",
        "VORTAC",
        "height above station",
        "error",
        "overhead",
    ],
    inputs: &[
        qty("dme", "DME reading", "Like 5.0 NM", QT::Distance, "NM")
            .required()
            .core(),
        qty(
            "height",
            "Height above the station",
            "Your altitude minus the station elevation, like 6000 ft",
            QT::Length,
            "ft",
        )
        .required()
        .core(),
    ],
    outputs: &[
        qty(
            "ground_distance",
            "Ground distance",
            "√(DME² − height²)",
            QT::Distance,
            "NM",
        )
        .precision(Precision::Decimals(3)),
        qty(
            "slant_error",
            "Slant-range error",
            "How much the reading exceeds the ground distance",
            QT::Distance,
            "NM",
        )
        .precision(Precision::Decimals(3)),
        qty(
            "elevation_angle",
            "Angle up to you from the station",
            "Above the horizontal",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(1)),
    ],
    errors: &[ErrorCode::NoSolution, ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "DME measures the straight-line (slant) distance; over flat ground the horizontal distance is √(DME² − h²) with h the height above the station in nautical miles (6,076.1 ft each). Error is largest close in and high up (Instrument Flying Handbook, ch. 9)",
    accuracy: "Exact geometry; ignores the Earth's curvature, which is under 0.01 NM inside 60 NM at 10,000 ft",
    references: &[IFH_NAV],
    examples: &[Example {
        id: "primary",
        title: "DME 5.0 at 6,000 ft above the station",
        input: r#"{"dme":"5.0 NM","height":"6000 ft"}"#,
        source: "add-practitioner-essentials slant-range scenario (4.902 NM)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.ifr.time-to-station",
            reason: "next",
        },
        Related {
            id: "aviation.ifr.dme-arc-lead",
            reason: "next",
        },
    ],
    sentence: "You are {ground_distance} from the station over the ground; the DME reads {slant_error} long.",
    limits: &[("batchRows", 10_000)],
    run: run_slant,
    ..ToolDef::BLANK
};

fn run_slant(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (nm, ft) = (unit(QT::Distance, "NM"), unit(QT::Length, "ft"));
    let dme = ctx.req_quantity("dme")?.to(nm);
    let h = ctx.req_quantity("height")?.to(ft) / FT_PER_NM;
    if dme < 0.0 || h < 0.0 {
        return Err(ToolError::invalid(
            "/dme",
            "The DME reading and the height cannot be negative.",
        ));
    }
    if h > dme {
        return Err(ToolError::new(
            ErrorCode::NoSolution,
            format!("A DME reading of {dme} NM is less than your height above the station ({h:.3} NM): you are essentially overhead, where DME shows your height."),
        )
        .at("/dme"));
    }
    let g = sqrt(dme * dme - h * h);
    let angle = if dme > 0.0 {
        asin(h / dme).to_degrees()
    } else {
        90.0
    };
    Ok(obj(vec![
        (
            "ground_distance",
            ctx.out("ground_distance", Q { value: g, unit: nm }),
        ),
        (
            "slant_error",
            ctx.out(
                "slant_error",
                Q {
                    value: dme - g,
                    unit: nm,
                },
            ),
        ),
        (
            "elevation_angle",
            ctx.out(
                "elevation_angle",
                Q {
                    value: angle,
                    unit: unit(QT::Angle, "deg"),
                },
            ),
        ),
    ]))
}

pub static TIME_TO_STATION: ToolDef = ToolDef {
    id: "aviation.ifr.time-to-station",
    title: "Time and distance to a station",
    summary: "How far and how long to a VOR or NDB from the time a bearing takes to change while you fly square to it, exactly and by the 60-times rule.",
    aliases: &[
        "time to station",
        "distance to station",
        "wingtip bearing change",
        "bearing change timing",
    ],
    keywords: &[
        "time to station",
        "distance to station",
        "bearing change",
        "wingtip",
        "VOR",
        "NDB",
        "timing",
        "60",
    ],
    inputs: &[
        qty(
            "minutes",
            "Time for the change",
            "Like 2 min",
            QT::Time,
            "min",
        )
        .required()
        .core(),
        qty(
            "bearing_change",
            "Bearing change",
            "Degrees, flying square to the station, like 10",
            QT::Angle,
            "deg",
        )
        .required()
        .core(),
        qty(
            "groundspeed",
            "Groundspeed",
            "For the distance, like 120 kt",
            QT::Speed,
            "kt",
        )
        .core(),
    ],
    outputs: &[
        qty(
            "time",
            "Time to station",
            "Exact, from where the timing ended",
            QT::Time,
            "min",
        )
        .precision(Precision::Decimals(2)),
        qty(
            "time_rule",
            "Time by the rule",
            "60 × minutes ÷ degrees",
            QT::Time,
            "min",
        )
        .precision(Precision::Decimals(2)),
        qty(
            "distance",
            "Distance to station",
            "Exact, at the groundspeed",
            QT::Distance,
            "NM",
        )
        .precision(Precision::Decimals(2))
        .optional(),
        qty(
            "distance_rule",
            "Distance by the rule",
            "Groundspeed × minutes ÷ degrees",
            QT::Distance,
            "NM",
        )
        .precision(Precision::Decimals(2))
        .optional(),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Flying square to the station, which is abeam when the timing starts, the track flown and the two bearings make a right triangle: from where the timing ends the station is (GS × t) ÷ sin Δ away, so the time is t ÷ sin Δ. The rule, 60 × t ÷ Δ, takes sin Δ ≈ Δ/60 (Instrument Flying Handbook, ch. 9)",
    accuracy: "Exact for a steady wind-free track square to the station; the rule reads about 5% long near 10° and grows with the change",
    references: &[IFH_NAV],
    examples: &[Example {
        id: "primary",
        title: "A 10° change in 2 minutes at 120 kt",
        input: r#"{"minutes":"2 min","bearing_change":"10 deg","groundspeed":"120 kt"}"#,
        source: "Hand check: rule 60 × 2 ÷ 10 = 12 min; exact 2 ÷ sin 10° = 11.52 min",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.ifr.dme-slant-range",
            reason: "alternative",
        },
        Related {
            id: "aviation.ifr.dme-arc-lead",
            reason: "next",
        },
    ],
    sentence: "The station is {time} away, {time_rule} by the rule.",
    limits: &[("batchRows", 10_000)],
    run: run_time_to_station,
    ..ToolDef::BLANK
};

fn run_time_to_station(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (min, dg, kt, nm) = (
        unit(QT::Time, "min"),
        unit(QT::Angle, "deg"),
        unit(QT::Speed, "kt"),
        unit(QT::Distance, "NM"),
    );
    let t = ctx.req_quantity("minutes")?.to(min);
    let d = ctx.req_quantity("bearing_change")?.to(dg);
    if t <= 0.0 {
        return Err(ToolError::invalid("/minutes", "The time must be positive."));
    }
    if !(d > 0.0 && d < 90.0) {
        return Err(ToolError::invalid(
            "/bearing_change",
            "Time a change between 0° and 90°; 5° to 20° works best.",
        ));
    }
    let exact = t / sin(d.to_radians());
    let rule = 60.0 * t / d;
    let mut out = vec![
        (
            "time",
            ctx.out(
                "time",
                Q {
                    value: exact,
                    unit: min,
                },
            ),
        ),
        (
            "time_rule",
            ctx.out(
                "time_rule",
                Q {
                    value: rule,
                    unit: min,
                },
            ),
        ),
    ];
    if let Some(gs) = ctx.quantity("groundspeed")?.map(|q| q.to(kt)) {
        if gs <= 0.0 {
            return Err(ToolError::invalid(
                "/groundspeed",
                "The groundspeed must be positive.",
            ));
        }
        out.push((
            "distance",
            ctx.out(
                "distance",
                Q {
                    value: gs * exact / 60.0,
                    unit: nm,
                },
            ),
        ));
        out.push((
            "distance_rule",
            ctx.out(
                "distance_rule",
                Q {
                    value: gs * rule / 60.0,
                    unit: nm,
                },
            ),
        ));
    }
    Ok(obj(out))
}

pub static ARC_LEAD: ToolDef = ToolDef {
    id: "aviation.ifr.dme-arc-lead",
    title: "DME arc lead points",
    summary: "Where to start the turn onto a DME arc from a radial, and how many radials early to turn off the arc onto an inbound course, from the arc's DME and your standard-rate turn.",
    aliases: &[
        "DME arc lead",
        "lead radial",
        "arc lead point",
        "turn onto DME arc",
    ],
    keywords: &[
        "DME arc",
        "lead radial",
        "lead point",
        "arc",
        "turn radius",
        "standard rate",
        "radial",
    ],
    inputs: &[
        qty("arc", "Arc", "Its DME, like 10 NM", QT::Distance, "NM")
            .required()
            .core(),
        qty(
            "groundspeed",
            "Groundspeed",
            "Like 120 kt, for a standard-rate turn",
            QT::Speed,
            "kt",
        )
        .required()
        .core(),
        qty(
            "turn_radius",
            "Turn radius",
            "Instead of standard rate, like 0.8 NM",
            QT::Distance,
            "NM",
        ),
    ],
    outputs: &[
        qty(
            "turn_radius",
            "Turn radius",
            "Standard rate: groundspeed ÷ (60π)",
            QT::Distance,
            "NM",
        )
        .precision(Precision::Decimals(2)),
        qty(
            "onto_arc_lead",
            "Lead onto the arc",
            "Start the turn this far before the arc's DME (a 90° turn)",
            QT::Distance,
            "NM",
        )
        .precision(Precision::Decimals(2)),
        qty(
            "lead_radials",
            "Lead off the arc",
            "Radials before the inbound course, exact",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(1)),
        qty(
            "lead_radials_rule",
            "Lead by the rule",
            "60 × turn radius ÷ arc DME",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(1)),
    ],
    errors: &[ErrorCode::NoSolution, ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "A standard-rate turn (3°/s) at groundspeed GS has radius GS ÷ (60π) NM. Turning 90° from a radial onto the arc, start one turn radius r short of the arc. Turning 90° from the arc onto an inbound radial, the turn circle sits inside the arc, tangent to it and to the radial, so the lead is asin(r ÷ (R − r)); the rule of thumb is 60 × r ÷ R radials (Instrument Flying Handbook, ch. 9)",
    accuracy: "Exact for a steady turn in still air; wind moves the lead points",
    references: &[IFH_NAV],
    examples: &[Example {
        id: "primary",
        title: "A 10 DME arc at 120 kt",
        input: r#"{"arc":"10 NM","groundspeed":"120 kt"}"#,
        source: "Hand check: radius 120 ÷ (60π) = 0.64 NM; lead asin(0.64 ÷ 9.36) = 3.9°, rule 3.8°",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.ifr.dme-slant-range",
            reason: "alternative",
        },
        Related {
            id: "aviation.ifr.time-to-station",
            reason: "alternative",
        },
    ],
    sentence: "Turn onto the arc {onto_arc_lead} before it, and off it {lead_radials} before the inbound course.",
    limits: &[("batchRows", 10_000)],
    run: run_arc_lead,
    ..ToolDef::BLANK
};

fn run_arc_lead(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (nm, kt, dg) = (
        unit(QT::Distance, "NM"),
        unit(QT::Speed, "kt"),
        unit(QT::Angle, "deg"),
    );
    let big_r = ctx.req_quantity("arc")?.to(nm);
    let gs = ctx.req_quantity("groundspeed")?.to(kt);
    if big_r <= 0.0 || gs <= 0.0 {
        return Err(ToolError::invalid(
            "/arc",
            "The arc's DME and the groundspeed must be positive.",
        ));
    }
    let r = match ctx.quantity("turn_radius")? {
        Some(q) => q.to(nm),
        None => gs / (60.0 * core::f64::consts::PI),
    };
    if r <= 0.0 {
        return Err(ToolError::invalid(
            "/turn_radius",
            "The turn radius must be positive.",
        ));
    }
    if 2.0 * r >= big_r {
        return Err(ToolError::new(
            ErrorCode::NoSolution,
            format!("A {r:.2} NM turn radius is too wide for a {big_r} NM arc: the turn would not fit inside it. Slow down or fly a larger arc."),
        )
        .at("/arc"));
    }
    let lead = asin(r / (big_r - r)).to_degrees();
    Ok(obj(vec![
        (
            "turn_radius",
            ctx.out("turn_radius", Q { value: r, unit: nm }),
        ),
        (
            "onto_arc_lead",
            ctx.out("onto_arc_lead", Q { value: r, unit: nm }),
        ),
        (
            "lead_radials",
            ctx.out(
                "lead_radials",
                Q {
                    value: lead,
                    unit: dg,
                },
            ),
        ),
        (
            "lead_radials_rule",
            ctx.out(
                "lead_radials_rule",
                Q {
                    value: 60.0 * r / big_r,
                    unit: dg,
                },
            ),
        ),
    ]))
}
