//! Flight performance (aviation/flight-performance spec): turns and load factor,
//! climb and descent gradients, top of descent, the visual descent point, glide
//! range with wind, and pivotal altitude. First principles only: nothing here
//! estimates an aircraft's performance without its POH data.

use crate::atmosphere::G0;
use crate::refs::*;
use crate::{deg, m, obj, unit};
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Related, ToolDef};
use gp_base::units::Quantity as QT;
use gp_base::{ErrorCode, display};
use libm::{acos, atan, cos, sqrt, tan};

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

fn q(v: f64, q: QT, s: &str) -> Q {
    Q {
        value: v,
        unit: unit(q, s),
    }
}

fn mps(v: f64) -> Q {
    q(v, QT::Speed, "m/s")
}

fn vs(v: f64) -> Q {
    q(v, QT::VerticalSpeed, "m/s")
}

fn nm_dist(v_m: f64) -> Q {
    q(v_m, QT::Distance, "m")
}

fn seconds(v: f64) -> Q {
    q(v, QT::Time, "s")
}

fn ratio(v: f64) -> Q {
    q(v, QT::Slope, "ratio")
}

const KT: f64 = 1852.0 / 3600.0;
const FT: f64 = 0.3048;
const NM: f64 = 1852.0;

fn positive(ctx: &mut Ctx, name: &str, what: &str) -> Result<f64, ToolError> {
    let v = ctx.req_quantity(name)?.base();
    if v <= 0.0 {
        return Err(ToolError::invalid(
            &format!("/{name}"),
            format!("{what} must be positive."),
        ));
    }
    Ok(v)
}

// ---------------------------------------------------------------- turn

pub static TURN: ToolDef = ToolDef {
    id: "aviation.performance.turn",
    title: "Turn performance and load factor",
    summary: "Bank angle, rate of turn, turn radius, load factor, and stall speed in the turn, from true airspeed and a bank angle or turn rate (standard rate by default).",
    aliases: &[
        "standard rate turn calculator",
        "turn radius calculator",
        "load factor calculator",
        "bank angle for standard rate",
    ],
    keywords: &[
        "turn",
        "bank",
        "standard rate",
        "rate one",
        "turn radius",
        "load factor",
        "g",
        "stall speed",
        "accelerated stall",
    ],
    inputs: &[
        qty("tas", "True airspeed", "Like 100 kt", QT::Speed, "kt").core(),
        qty(
            "bank",
            "Bank angle",
            "Like 30 deg; leave empty for a standard-rate turn",
            QT::Angle,
            "deg",
        )
        .core(),
        qty(
            "turn_rate",
            "Turn rate",
            "Like 3 deg/s (standard rate)",
            QT::AngularRate,
            "deg/s",
        )
        .core(),
        qty(
            "heading_change",
            "Heading change",
            "Like 180 deg; default 360",
            QT::Angle,
            "deg",
        ),
        qty(
            "stall_speed",
            "1 g stall speed",
            "Like 50 kt (CAS, from the POH)",
            QT::Speed,
            "kt",
        )
        .core(),
        Field::new(
            "load_limit",
            "Limit load factor",
            "Like 3.8 (normal category)",
            Kind::Number {
                min: 1.0,
                max: 12.0,
            },
        ),
    ],
    outputs: &[
        out("bank", "Bank angle", "atan(V·ω / g0)", QT::Angle, "deg", 2),
        out(
            "turn_rate",
            "Turn rate",
            "g0·tan φ / V",
            QT::AngularRate,
            "deg/s",
            2,
        ),
        out(
            "tas",
            "True airspeed",
            "g0·tan φ / ω when solved",
            QT::Speed,
            "kt",
            1,
        ),
        out(
            "radius",
            "Turn radius",
            "V² / (g0·tan φ)",
            QT::Length,
            "ft",
            0,
        ),
        Field::new(
            "load_factor",
            "Load factor",
            "1 / cos φ",
            Kind::Number { min: 1.0, max: 1e9 },
        )
        .precision(Precision::Decimals(2)),
        out(
            "turn_time",
            "Time for the heading change",
            "Heading change / turn rate",
            QT::Time,
            "s",
            1,
        ),
        out(
            "stall_speed_in_turn",
            "Stall speed in the turn",
            "Vs·√n",
            QT::Speed,
            "kt",
            1,
        )
        .optional(),
        out(
            "rule_of_thumb_bank",
            "KTAS/10 + 7 rule",
            "Standard-rate bank rule of thumb",
            QT::Angle,
            "deg",
            1,
        )
        .optional(),
        out(
            "rule_error",
            "Rule error",
            "Rule minus the exact bank",
            QT::Angle,
            "deg",
            1,
        )
        .optional(),
        out(
            "max_bank",
            "Maximum bank for the load limit",
            "acos(1 / n limit)",
            QT::Angle,
            "deg",
            1,
        )
        .optional(),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &["LOAD_LIMIT_EXCEEDED", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Coordinated level turn: r = V²/(g0·tan φ), ω = g0·tan φ/V, n = 1/cos φ. In feet and knots r = V²/(11.294·tan φ), from g0 = 9.80665 m/s² (the textbook 11.26 is a rounded variant).",
    accuracy: "Exact for a coordinated, level turn at constant true airspeed. Planning aid, not certified for navigation.",
    references: &[AFH, PHAK],
    examples: &[
        Example {
            id: "primary",
            title: "A standard-rate turn at 100 KTAS",
            input: r#"{"tas":"100 kt"}"#,
            source: "add-aviation-suite turn scenario: bank 15.36°, rule of thumb 17°",
        },
        Example {
            id: "steep",
            title: "A 60° bank with a 50 KCAS stall speed",
            input: r#"{"tas":"100 kt","bank":"60 deg","stall_speed":"50 kt"}"#,
            source: "add-aviation-suite turn scenario: load factor 2.0, stall speed 70.7 KCAS (AFH chapter 3)",
        },
    ],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "gauge",
        map: &[("value", "bank")],
    }],
    related: &[
        Related {
            id: "aviation.performance.pivotal-altitude",
            reason: "next",
        },
        Related {
            id: "aviation.airspeed.cas-to-tas",
            reason: "next",
        },
    ],
    sentence: "Bank {bank} for {turn_rate} at {tas}. The turn radius is {radius} and the load factor is {load_factor} g.{if stall_speed_in_turn > 0} Stall speed rises to {stall_speed_in_turn}.{/if}{warn LOAD_LIMIT_EXCEEDED} This is past the load limit.{/warn}",
    limits: &[("batchRows", 10_000)],
    run: run_turn,
    ..ToolDef::BLANK
};

fn run_turn(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let v = ctx.quantity("tas")?.map(|x| x.base());
    let bank = ctx.quantity("bank")?.map(|x| x.base());
    let rate = ctx.quantity("turn_rate")?.map(|x| x.base());
    if let Some(b) = bank
        && !(b > 0.0 && b < 90.0)
    {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "Bank angle must be between 0° and 90°.",
        )
        .at("/bank"));
    }
    if rate.is_some_and(|r| r <= 0.0) {
        return Err(ToolError::invalid(
            "/turn_rate",
            "Turn rate must be positive.",
        ));
    }
    if v.is_some_and(|v| v <= 0.0) {
        return Err(ToolError::invalid(
            "/tas",
            "True airspeed must be positive.",
        ));
    }
    let (v, phi, standard) = match (v, bank, rate) {
        (Some(v), Some(b), None) => (v, b.to_radians(), false),
        (Some(v), None, r) => {
            let w = r.unwrap_or(3.0).to_radians();
            let phi = atan(v * w / G0);
            (v, phi, r.is_none_or(|r| r == 3.0))
        }
        (None, Some(b), Some(r)) => {
            let phi = b.to_radians();
            (G0 * tan(phi) / r.to_radians(), phi, r == 3.0)
        }
        (Some(_), Some(_), Some(_)) => {
            return Err(ToolError::invalid(
                "/turn_rate",
                "Give two of true airspeed, bank angle, and turn rate, not all three.",
            ));
        }
        _ => {
            return Err(ToolError::invalid(
                "/tas",
                "True airspeed is required unless both bank angle and turn rate are given.",
            )
            .hint("Example: 100 kt for a standard-rate turn."));
        }
    };
    let w = G0 * tan(phi) / v; // rad/s
    let n = 1.0 / cos(phi);
    let change = ctx
        .quantity("heading_change")?
        .map_or(360.0, |x| x.base().abs());
    let mut o = vec![
        ("bank", ctx.out("bank", deg(phi.to_degrees()))),
        (
            "turn_rate",
            ctx.out("turn_rate", q(w.to_degrees(), QT::AngularRate, "deg/s")),
        ),
        ("tas", ctx.out("tas", mps(v))),
        ("radius", ctx.out("radius", m(v * v / (G0 * tan(phi))))),
        ("load_factor", Json::Num(n)),
        (
            "turn_time",
            ctx.out("turn_time", seconds(change / w.to_degrees())),
        ),
    ];
    if let Some(vs) = ctx.quantity("stall_speed")? {
        o.push((
            "stall_speed_in_turn",
            ctx.out("stall_speed_in_turn", mps(vs.base() * sqrt(n))),
        ));
    }
    if standard {
        let rule = v / KT / 10.0 + 7.0;
        o.push((
            "rule_of_thumb_bank",
            ctx.out("rule_of_thumb_bank", deg(rule)),
        ));
        o.push((
            "rule_error",
            ctx.out("rule_error", deg(rule - phi.to_degrees())),
        ));
    }
    if let Some(lim) = ctx.number("load_limit")? {
        let max_bank = acos(1.0 / lim).to_degrees();
        o.push(("max_bank", ctx.out("max_bank", deg(max_bank))));
        if n > lim {
            ctx.warnings.push(
                Warning::new(
                    "LOAD_LIMIT_EXCEEDED",
                    format!(
                        "A load factor of {} g exceeds the limit of {} g. The steepest level turn within the limit is {} of bank.",
                        display::number(n, Precision::Decimals(2), ctx.options.format),
                        display::number(lim, Precision::Significant(3), ctx.options.format),
                        display::quantity(max_bank, "deg", Precision::Decimals(1), ctx.options.format),
                    ),
                )
                .at("/bank"),
            );
        }
    }
    Ok(obj(o))
}

// ---------------------------------------------------------------- descent

pub static DESCENT: ToolDef = ToolDef {
    id: "aviation.performance.top-of-descent",
    stability: gp_base::tool::Stability::Stable,
    title: "Top of descent",
    summary: "Where to start down: descent distance, vertical speed, and time for an altitude change on a descent angle or at a vertical speed, with the 3-to-1 and 5 × groundspeed rules beside them.",
    aliases: &[
        "TOD calculator",
        "top of descent calculator",
        "descent planning",
        "3 to 1 rule",
    ],
    keywords: &[
        "TOD",
        "descent",
        "descent rate",
        "vertical speed",
        "3:1",
        "glidepath",
        "descent angle",
        "fpm",
    ],
    inputs: &[
        qty(
            "from_altitude",
            "Cruise altitude",
            "Like 35000 ft",
            QT::Length,
            "ft",
        )
        .required()
        .core(),
        qty(
            "to_altitude",
            "Target altitude",
            "Like 3000 ft",
            QT::Length,
            "ft",
        )
        .required()
        .core(),
        qty("groundspeed", "Groundspeed", "Like 420 kt", QT::Speed, "kt")
            .required()
            .core(),
        qty(
            "descent_angle",
            "Descent angle",
            "Like 3 deg (the default)",
            QT::Angle,
            "deg",
        )
        .core(),
        qty(
            "vertical_speed",
            "Vertical speed",
            "Like 1500 fpm, instead of an angle",
            QT::VerticalSpeed,
            "ft/min",
        )
        .core(),
    ],
    outputs: &[
        out(
            "distance",
            "Descent distance",
            "Altitude change / tan(angle)",
            QT::Distance,
            "NM",
            1,
        ),
        out(
            "vertical_speed",
            "Vertical speed",
            "Groundspeed × tan(angle)",
            QT::VerticalSpeed,
            "ft/min",
            0,
        ),
        out(
            "descent_angle",
            "Descent angle",
            "atan(vertical speed / groundspeed)",
            QT::Angle,
            "deg",
            2,
        ),
        out(
            "gradient",
            "Descent gradient",
            "Feet per nautical mile",
            QT::Slope,
            "ft/NM",
            0,
        ),
        out(
            "time",
            "Descent time",
            "Distance / groundspeed",
            QT::Time,
            "min",
            1,
        ),
        out(
            "rule_3_to_1",
            "3-to-1 rule distance",
            "3 NM per 1,000 ft",
            QT::Distance,
            "NM",
            1,
        )
        .optional(),
        out(
            "rule_3_to_1_error",
            "3-to-1 rule error",
            "Rule minus the exact distance",
            QT::Distance,
            "NM",
            1,
        )
        .optional(),
        out(
            "rule_5_gs",
            "5 × groundspeed rule",
            "Vertical speed ≈ 5 × groundspeed (3° path)",
            QT::VerticalSpeed,
            "ft/min",
            0,
        )
        .optional(),
        out(
            "rule_5_gs_error",
            "5 × groundspeed rule error",
            "Rule minus the exact vertical speed",
            QT::VerticalSpeed,
            "ft/min",
            0,
        )
        .optional(),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Straight descent path over flat ground at constant groundspeed: distance = Δh / tan θ, vertical speed = GS · tan θ",
    accuracy: "Exact for a constant angle and groundspeed; real descents slow down and meet changing winds. Planning aid, not certified for navigation.",
    references: &[IPH, PHAK],
    examples: &[Example {
        id: "primary",
        title: "FL350 to 3,000 ft on a 3° path at 420 kt",
        input: r#"{"from_altitude":"35000 ft","to_altitude":"3000 ft","groundspeed":"420 kt"}"#,
        source: "add-aviation-suite descent scenario: 100.5 NM (3:1 rule 96 NM), 2,229 fpm (5 × GS rule 2,100 fpm)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "profile-chart",
        map: &[("value", "distance")],
    }],
    related: &[
        Related {
            id: "aviation.performance.climb-gradient",
            reason: "alternative",
        },
        Related {
            id: "aviation.performance.vdp",
            reason: "next",
        },
        Related {
            id: "aviation.performance.glide",
            reason: "alternative",
        },
    ],
    sentence: "Start down {distance} out and descend at {vertical_speed}, taking {time}.{if rule_3_to_1 > 0} The 3-to-1 rule says {rule_3_to_1}.{/if}",
    limits: &[("batchRows", 10_000)],
    run: run_descent,
    ..ToolDef::BLANK
};

/// The descent (or climb) angle in radians from an angle or a vertical speed at `gs`.
fn path_angle(
    ctx: &mut Ctx,
    gs: f64,
    angle: &str,
    vspeed: &str,
    default_deg: Option<f64>,
) -> Result<f64, ToolError> {
    let a = ctx.quantity(angle)?.map(|x| x.base());
    let v = ctx.quantity(vspeed)?.map(|x| x.base());
    let theta = match (a, v) {
        (Some(_), Some(_)) => {
            return Err(ToolError::invalid(
                &format!("/{vspeed}"),
                "Give a path angle or a vertical speed, not both.",
            ));
        }
        (Some(a), None) => a.to_radians(),
        (None, Some(v)) => atan(v.abs() / gs),
        (None, None) => match default_deg {
            Some(d) => d.to_radians(),
            None => {
                return Err(ToolError::invalid(
                    &format!("/{angle}"),
                    "Give a path angle or a vertical speed.",
                ));
            }
        },
    };
    if !(theta > 0.0 && theta < 45f64.to_radians()) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The path angle must be between 0° and 45°.",
        )
        .at(&format!("/{angle}")));
    }
    Ok(theta)
}

fn run_descent(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let from = ctx.req_quantity("from_altitude")?.base();
    let to = ctx.req_quantity("to_altitude")?.base();
    let dh = from - to;
    if dh <= 0.0 {
        return Err(ToolError::invalid(
            "/to_altitude",
            "The target altitude must be below the cruise altitude.",
        ));
    }
    let gs = positive(ctx, "groundspeed", "Groundspeed")?;
    let theta = path_angle(ctx, gs, "descent_angle", "vertical_speed", Some(3.0))?;
    let dist = dh / tan(theta);
    let v = gs * tan(theta);
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Altitude to lose",
            "Δh = cruise altitude − target altitude",
            format!("{} ft − {} ft", n(from / FT, 0), n(to / FT, 0)),
            format!("{} ft", n(dh / FT, 0)),
        );
        ctx.step(
            "Distance to start down",
            "d = Δh / tan θ",
            format!("{} ft / tan {}°", n(dh / FT, 0), n(theta.to_degrees(), 2)),
            format!("{} NM", n(dist / NM, 1)),
        );
    }
    let mut o = vec![
        ("distance", ctx.out("distance", nm_dist(dist))),
        ("vertical_speed", ctx.out("vertical_speed", vs(v))),
        (
            "descent_angle",
            ctx.out("descent_angle", deg(theta.to_degrees())),
        ),
        ("gradient", ctx.out("gradient", ratio(tan(theta)))),
        ("time", ctx.out("time", seconds(dist / gs))),
    ];
    // Both rules describe a 3° path; outside 2.5° to 3.5° they would mislead.
    if (2.5..=3.5).contains(&theta.to_degrees()) {
        let r1 = 3.0 * (dh / FT / 1000.0) * NM;
        let r2 = 5.0 * (gs / KT) * FT / 60.0;
        o.push(("rule_3_to_1", ctx.out("rule_3_to_1", nm_dist(r1))));
        o.push((
            "rule_3_to_1_error",
            ctx.out("rule_3_to_1_error", nm_dist(r1 - dist)),
        ));
        o.push(("rule_5_gs", ctx.out("rule_5_gs", vs(r2))));
        o.push(("rule_5_gs_error", ctx.out("rule_5_gs_error", vs(r2 - v))));
    }
    Ok(obj(o))
}

// ---------------------------------------------------------------- climb gradient

pub static CLIMB_GRADIENT: ToolDef = ToolDef {
    id: "aviation.performance.climb-gradient",
    title: "Climb gradient and vertical speed",
    summary: "Converts a climb or descent gradient (feet per nautical mile, percent, or degrees) to the vertical speed you need at a groundspeed, or the reverse.",
    aliases: &[
        "climb gradient calculator",
        "ft per NM to fpm",
        "required climb rate",
        "departure climb gradient",
    ],
    keywords: &[
        "climb gradient",
        "ft/NM",
        "fpm",
        "vertical speed",
        "departure procedure",
        "obstacle",
        "percent gradient",
    ],
    inputs: &[
        qty(
            "gradient",
            "Gradient",
            "Like 200 ft/NM or 3.3 %",
            QT::Slope,
            "ft/NM",
        )
        .core(),
        qty(
            "angle",
            "Angle",
            "Like 3 deg, instead of a gradient",
            QT::Angle,
            "deg",
        )
        .core(),
        qty(
            "vertical_speed",
            "Vertical speed",
            "Like 400 fpm, to find the gradient",
            QT::VerticalSpeed,
            "ft/min",
        )
        .core(),
        qty("groundspeed", "Groundspeed", "Like 120 kt", QT::Speed, "kt")
            .required()
            .core(),
    ],
    outputs: &[
        out(
            "vertical_speed",
            "Vertical speed",
            "Groundspeed × gradient",
            QT::VerticalSpeed,
            "ft/min",
            0,
        ),
        out(
            "gradient",
            "Gradient",
            "Feet per nautical mile",
            QT::Slope,
            "ft/NM",
            0,
        ),
        out(
            "gradient_percent",
            "Gradient",
            "Rise over run in percent",
            QT::Slope,
            "%",
            2,
        ),
        out("angle", "Angle", "atan(gradient)", QT::Angle, "deg", 2),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Vertical speed = groundspeed × gradient (rise over horizontal run); 1 ft/NM = 0.3048/1852",
    accuracy: "Exact. Planning aid, not certified for navigation.",
    references: &[IPH],
    examples: &[Example {
        id: "primary",
        title: "200 ft/NM at 120 kt",
        input: r#"{"gradient":"200 ft/NM","groundspeed":"120 kt"}"#,
        source: "add-aviation-suite climb scenario: 400 fpm",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "profile-chart",
        map: &[("value", "gradient")],
    }],
    related: &[Related {
        id: "aviation.performance.top-of-descent",
        reason: "alternative",
    }],
    sentence: "At that groundspeed, {gradient} takes {vertical_speed}.",
    limits: &[("batchRows", 10_000)],
    run: run_climb_gradient,
    ..ToolDef::BLANK
};

fn run_climb_gradient(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let gs = positive(ctx, "groundspeed", "Groundspeed")?;
    let g = ctx.quantity("gradient")?.map(|x| x.base());
    let a = ctx.quantity("angle")?.map(|x| x.base());
    let v = ctx.quantity("vertical_speed")?.map(|x| x.base());
    let grad = match (g, a, v) {
        (Some(g), None, None) => g,
        (None, Some(a), None) => {
            if !(a > 0.0 && a < 90.0) {
                return Err(ToolError::new(
                    ErrorCode::OutOfDomain,
                    "The angle must be between 0° and 90°.",
                )
                .at("/angle"));
            }
            tan(a.to_radians())
        }
        (None, None, Some(v)) => v / gs,
        _ => {
            return Err(ToolError::invalid(
                "/gradient",
                "Give exactly one of gradient, angle, or vertical speed.",
            ));
        }
    };
    if grad <= 0.0 {
        return Err(ToolError::invalid(
            "/gradient",
            "The gradient must be positive.",
        ));
    }
    Ok(obj(vec![
        ("vertical_speed", ctx.out("vertical_speed", vs(gs * grad))),
        ("gradient", ctx.out("gradient", ratio(grad))),
        ("gradient_percent", ctx.out("gradient_percent", ratio(grad))),
        ("angle", ctx.out("angle", deg(atan(grad).to_degrees()))),
    ]))
}

// ---------------------------------------------------------------- VDP

pub static VDP: ToolDef = ToolDef {
    id: "aviation.performance.vdp",
    stability: gp_base::tool::Stability::Stable,
    title: "Visual descent point",
    summary: "The visual descent point for a non-precision approach: the distance from the threshold where a normal descent from the MDA begins, with the HAT/300 rule and the descent rate at your groundspeed.",
    aliases: &["VDP calculator", "visual descent point calculator"],
    keywords: &[
        "VDP",
        "MDA",
        "HAT",
        "non-precision approach",
        "descent angle",
        "VDA",
        "glidepath",
    ],
    inputs: &[
        qty(
            "height_above_touchdown",
            "Height above touchdown",
            "MDA minus touchdown zone elevation, like 400 ft",
            QT::Length,
            "ft",
        )
        .required()
        .core(),
        qty(
            "descent_angle",
            "Descent angle",
            "Like 3 deg (the default)",
            QT::Angle,
            "deg",
        )
        .core(),
        qty(
            "threshold_crossing_height",
            "Threshold crossing height",
            "Like 50 ft; default 0 (aim at the threshold)",
            QT::Length,
            "ft",
        )
        .core(),
        qty(
            "groundspeed",
            "Groundspeed",
            "Like 90 kt, for time and descent rate",
            QT::Speed,
            "kt",
        )
        .core(),
    ],
    outputs: &[
        out(
            "distance",
            "VDP distance",
            "(HAT − TCH) / tan(angle), from the threshold",
            QT::Distance,
            "NM",
            2,
        ),
        out(
            "rule_hat_300",
            "HAT / 300 rule",
            "HAT in feet / 300, in NM",
            QT::Distance,
            "NM",
            2,
        ),
        out(
            "rule_error",
            "Rule error",
            "Rule minus the exact distance",
            QT::Distance,
            "NM",
            2,
        ),
        out(
            "rule_hat_318",
            "HAT / 318 rule",
            "HAT in feet / 318 (feet per NM on a 3° path), in NM",
            QT::Distance,
            "NM",
            2,
        ),
        out(
            "vertical_speed",
            "Descent rate",
            "Groundspeed × tan(angle)",
            QT::VerticalSpeed,
            "ft/min",
            0,
        )
        .optional(),
        out(
            "time_to_threshold",
            "Time from VDP to threshold",
            "Distance / groundspeed",
            QT::Time,
            "s",
            0,
        )
        .optional(),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "A straight path at the descent angle from the MDA to the threshold crossing height",
    accuracy: "Exact geometry. A published VDP on the chart always governs. Planning aid, not certified for navigation.",
    references: &[IPH, AIM],
    examples: &[Example {
        id: "primary",
        title: "An MDA 400 ft above the touchdown zone, 3° path, 90 kt",
        input: r#"{"height_above_touchdown":"400 ft","groundspeed":"90 kt"}"#,
        source: "Instrument Procedures Handbook chapter 4: VDP ≈ HAT/300 NM for a 3° path (400/300 = 1.33 NM)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "profile-chart",
        map: &[("value", "distance")],
    }],
    related: &[
        Related {
            id: "aviation.performance.top-of-descent",
            reason: "alternative",
        },
        Related {
            id: "aviation.performance.climb-gradient",
            reason: "alternative",
        },
        Related {
            id: "aviation.ifr.hold-entry",
            reason: "next",
        },
    ],
    sentence: "Start down from the MDA {distance} from the threshold.{if vertical_speed > 0} Descend at about {vertical_speed}.{/if}",
    limits: &[("batchRows", 10_000)],
    run: run_vdp,
    ..ToolDef::BLANK
};

fn run_vdp(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let hat = positive(ctx, "height_above_touchdown", "Height above touchdown")?;
    let tch = ctx
        .quantity("threshold_crossing_height")?
        .map_or(0.0, |x| x.base());
    if tch < 0.0 || tch >= hat {
        return Err(ToolError::invalid(
            "/threshold_crossing_height",
            "The threshold crossing height must be at least 0 and below the height above touchdown.",
        ));
    }
    let theta = ctx.quantity("descent_angle")?.map_or(3.0, |x| x.base());
    if !(theta > 0.0 && theta <= 10.0) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The descent angle must be between 0° and 10°.",
        )
        .at("/descent_angle"));
    }
    let t = tan(theta.to_radians());
    let dist = (hat - tch) / t;
    let rule = hat / FT / 300.0 * NM;
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Height to lose",
            "h = height above touchdown − threshold crossing height",
            format!("{} ft − {} ft", n(hat / FT, 0), n(tch / FT, 0)),
            format!("{} ft", n((hat - tch) / FT, 0)),
        );
        ctx.step(
            "Distance from the threshold",
            "d = h / tan θ",
            format!("{} ft / tan {}°", n((hat - tch) / FT, 0), n(theta, 2)),
            format!("{} NM", n(dist / NM, 2)),
        );
    }
    let mut o = vec![
        ("distance", ctx.out("distance", nm_dist(dist))),
        ("rule_hat_300", ctx.out("rule_hat_300", nm_dist(rule))),
        ("rule_error", ctx.out("rule_error", nm_dist(rule - dist))),
        (
            "rule_hat_318",
            ctx.out("rule_hat_318", nm_dist(hat / FT / 318.0 * NM)),
        ),
    ];
    if let Some(gs) = ctx.quantity("groundspeed")? {
        let gs = gs.base();
        if gs <= 0.0 {
            return Err(ToolError::invalid(
                "/groundspeed",
                "Groundspeed must be positive.",
            ));
        }
        o.push(("vertical_speed", ctx.out("vertical_speed", vs(gs * t))));
        o.push((
            "time_to_threshold",
            ctx.out("time_to_threshold", seconds(dist / gs)),
        ));
    }
    Ok(obj(o))
}

// ---------------------------------------------------------------- glide

pub static GLIDE: ToolDef = ToolDef {
    id: "aviation.performance.glide",
    title: "Glide range with wind",
    summary: "How far you can glide from a height with your aircraft's glide ratio, in still air and with a headwind or tailwind.",
    aliases: &[
        "glide range calculator",
        "glide distance calculator",
        "engine out glide",
    ],
    keywords: &[
        "glide",
        "glide ratio",
        "L/D",
        "best glide",
        "engine failure",
        "headwind",
        "tailwind",
        "range ring",
    ],
    inputs: &[
        qty(
            "height",
            "Height above terrain",
            "Like 5000 ft AGL",
            QT::Length,
            "ft",
        )
        .required()
        .core(),
        Field::new(
            "glide_ratio",
            "Glide ratio",
            "From the POH best-glide data, like 9 (9:1)",
            Kind::Number {
                min: 1.0,
                max: 80.0,
            },
        )
        .required()
        .core(),
        qty(
            "tas",
            "True airspeed",
            "Best-glide TAS, like 70 kt",
            QT::Speed,
            "kt",
        )
        .required()
        .core(),
        qty(
            "headwind",
            "Headwind",
            "Like 20 kt; negative for a tailwind",
            QT::Speed,
            "kt",
        )
        .core(),
    ],
    outputs: &[
        out(
            "still_air_range",
            "Still-air range",
            "Height × glide ratio",
            QT::Distance,
            "NM",
            2,
        ),
        out(
            "wind_range",
            "Range with wind",
            "Still-air range × (TAS − headwind) / TAS",
            QT::Distance,
            "NM",
            2,
        ),
        out(
            "glide_angle",
            "Glide angle",
            "atan(1 / glide ratio)",
            QT::Angle,
            "deg",
            2,
        ),
        out(
            "sink_rate",
            "Sink rate",
            "TAS / glide ratio",
            QT::VerticalSpeed,
            "ft/min",
            0,
        ),
        out(
            "time_aloft",
            "Time aloft",
            "Height / sink rate",
            QT::Time,
            "min",
            1,
        ),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Steady glide at the given glide ratio, with TAS taken as the horizontal airspeed (under 1% error for glide ratios of 7 or more) and a steady along-track wind",
    accuracy: "Exact for the model. Real glides lose height in turns and with a windmilling propeller. Planning aid, not certified for navigation.",
    references: &[AFH, PHAK],
    examples: &[Example {
        id: "primary",
        title: "5,000 ft AGL, 9:1, 70 kt, into a 20 kt headwind",
        input: r#"{"height":"5000 ft","glide_ratio":9,"tas":"70 kt","headwind":"20 kt"}"#,
        source: "add-aviation-suite glide scenario: still air 7.41 NM, headwind 5.29 NM",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "gauge",
        map: &[("value", "wind_range")],
    }],
    related: &[Related {
        id: "aviation.performance.turn",
        reason: "alternative",
    }],
    sentence: "You can glide about {wind_range} with this wind, or {still_air_range} in still air, for {time_aloft}.",
    limits: &[("batchRows", 10_000)],
    run: run_glide,
    ..ToolDef::BLANK
};

fn run_glide(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let h = positive(ctx, "height", "Height above terrain")?;
    let ld = ctx.number("glide_ratio")?.expect("required");
    let v = positive(ctx, "tas", "True airspeed")?;
    let hw = ctx.quantity("headwind")?.map_or(0.0, |x| x.base());
    if hw >= v {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The headwind is at least the airspeed, so the aircraft makes no progress over the ground.",
        )
        .at("/headwind"));
    }
    let still = h * ld;
    let sink = v / ld;
    Ok(obj(vec![
        (
            "still_air_range",
            ctx.out("still_air_range", nm_dist(still)),
        ),
        (
            "wind_range",
            ctx.out("wind_range", nm_dist(still * (v - hw) / v)),
        ),
        (
            "glide_angle",
            ctx.out("glide_angle", deg(atan(1.0 / ld).to_degrees())),
        ),
        ("sink_rate", ctx.out("sink_rate", vs(sink))),
        ("time_aloft", ctx.out("time_aloft", seconds(h / sink))),
    ]))
}

// ---------------------------------------------------------------- pivotal altitude

pub static PIVOTAL_ALTITUDE: ToolDef = ToolDef {
    id: "aviation.performance.pivotal-altitude",
    title: "Pivotal altitude",
    summary: "The pivotal altitude for eights on pylons at your groundspeed, with the knots-squared over 11.3 rule.",
    aliases: &["pivotal altitude calculator", "eights on pylons"],
    keywords: &[
        "pivotal altitude",
        "eights on pylons",
        "ground reference",
        "commercial maneuvers",
    ],
    inputs: &[
        qty("groundspeed", "Groundspeed", "Like 100 kt", QT::Speed, "kt")
            .required()
            .core(),
    ],
    outputs: &[
        out(
            "pivotal_altitude",
            "Pivotal altitude",
            "V² / g0, above ground",
            QT::Length,
            "ft",
            0,
        ),
        out(
            "rule_of_thumb",
            "kt² / 11.3 rule",
            "Groundspeed in knots squared / 11.3",
            QT::Length,
            "ft",
            0,
        ),
        out(
            "rule_error",
            "Rule error",
            "Rule minus the exact value",
            QT::Length,
            "ft",
            0,
        ),
    ],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "h = V²/g0 (the line of sight to the pylon is parallel to the lateral axis)",
    accuracy: "Exact for level flight at a steady groundspeed. It changes with groundspeed around the pylon.",
    references: &[AFH],
    examples: &[Example {
        id: "primary",
        title: "100 kt groundspeed",
        input: r#"{"groundspeed":"100 kt"}"#,
        source: "add-aviation-suite pivotal-altitude scenario: about 885 ft AGL (AFH chapter 6: GS²/11.3)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "gauge",
        map: &[("value", "pivotal_altitude")],
    }],
    related: &[Related {
        id: "aviation.performance.turn",
        reason: "alternative",
    }],
    sentence: "The pivotal altitude is {pivotal_altitude} above the ground.",
    limits: &[("batchRows", 10_000)],
    run: run_pivotal,
    ..ToolDef::BLANK
};

fn run_pivotal(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let v = positive(ctx, "groundspeed", "Groundspeed")?;
    let h = v * v / G0;
    let kt = v / KT;
    let rule = kt * kt / 11.3 * FT;
    Ok(obj(vec![
        ("pivotal_altitude", ctx.out("pivotal_altitude", m(h))),
        ("rule_of_thumb", ctx.out("rule_of_thumb", m(rule))),
        ("rule_error", ctx.out("rule_error", m(rule - h))),
    ]))
}
