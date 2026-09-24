//! Wind components and winds aloft between levels (add-aviation-suite,
//! aviation/wind-and-navigation, task 4.5): a wind's east (u) and north (v)
//! components and back, and the wind and temperature at an altitude between
//! two forecast levels, interpolated as a vector so a veering wind turns
//! smoothly instead of averaging directions.

use crate::refs::*;
use crate::{obj, unit};
use gp_base::ErrorCode;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Related, ToolDef};
use gp_base::units::Quantity as QT;
use libm::{atan2, cos, hypot, sin};

const fn qty(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    q: QT,
    unit: &'static str,
) -> Field {
    Field::new(name, title, help, Kind::Quantity { q, unit })
}

/// Wind FROM direction θ (degrees true) and speed s → (u east, v north).
fn to_uv(dir: f64, s: f64) -> (f64, f64) {
    let t = dir.to_radians();
    (-s * sin(t), -s * cos(t))
}

/// (u, v) → (FROM direction in [0, 360), speed); calm reads as 0°.
fn from_uv(u: f64, v: f64) -> (f64, f64) {
    let s = hypot(u, v);
    if s < 1e-9 {
        return (0.0, 0.0);
    }
    (atan2(-u, -v).to_degrees().rem_euclid(360.0), s)
}

pub static UV: ToolDef = ToolDef {
    id: "aviation.wind.uv",
    stability: gp_base::tool::Stability::Stable,
    title: "Wind components (u and v)",
    summary: "A wind's east-west (u) and north-south (v) components from its direction and speed, or the direction and speed from u and v, as weather models and forecasts give them.",
    aliases: &[
        "u v wind",
        "wind components",
        "u and v components",
        "wind vector",
        "zonal meridional wind",
    ],
    keywords: &[
        "u",
        "v",
        "wind",
        "components",
        "zonal",
        "meridional",
        "vector",
        "direction",
        "speed",
    ],
    inputs: &[
        qty(
            "direction",
            "Wind direction",
            "Where it blows from, degrees true, like 300",
            QT::Angle,
            "deg",
        )
        .core(),
        qty("speed", "Wind speed", "Like 25 kt", QT::Speed, "kt").core(),
        qty(
            "u",
            "u (toward east)",
            "Or give u and v instead, like 21.7 kt",
            QT::Speed,
            "kt",
        )
        .core(),
        qty("v", "v (toward north)", "Like -12.5 kt", QT::Speed, "kt").core(),
    ],
    outputs: &[
        qty(
            "u",
            "u (toward east)",
            "Positive toward the east",
            QT::Speed,
            "kt",
        )
        .precision(Precision::Decimals(2)),
        qty(
            "v",
            "v (toward north)",
            "Positive toward the north",
            QT::Speed,
            "kt",
        )
        .precision(Precision::Decimals(2)),
        qty(
            "direction",
            "Wind direction",
            "From, degrees true",
            QT::Angle,
            "deg",
        )
        .precision(Precision::Decimals(1)),
        qty(
            "speed",
            "Wind speed",
            "The vector's length",
            QT::Speed,
            "kt",
        )
        .precision(Precision::Decimals(2)),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED"],
    model: "Meteorological convention: a wind from direction θ (true) at speed s has u = −s·sin θ (toward east) and v = −s·cos θ (toward north); back, θ = atan2(−u, −v) and s = √(u² + v²). Winds aloft are given from true north (Aviation Weather Handbook §27.2)",
    accuracy: "Exact",
    when_to_use: "Use this to move between the two ways a wind is written: as a direction and speed, the way pilots, METARs, and winds-aloft forecasts give it, and as u and v components, the way weather models, GRIB files, and most meteorology software store it. Give either and it returns the other, so model output can be read as a wind and a wind can be averaged, interpolated, or added as a vector.",
    limitations: "The direction is where the wind blows from, measured from true north, as in meteorology; u is positive toward the east and v toward the north, so a wind from the west has a positive u. A magnetic direction, like a runway or tower wind, has to be turned to true first. A calm wind has no direction: with u and v both zero the direction comes back as 0°, which then means nothing, not a north wind. Components that come from a model's grid may be relative to the grid rather than to true north, which this cannot know.",
    references: &[WEATHER_HANDBOOK],
    examples: &[Example {
        id: "primary",
        title: "A wind from 300° at 25 kt",
        input: r#"{"direction":"300 deg","speed":"25 kt"}"#,
        source: "Hand check: u = 25·sin(60°) = 21.65 kt east, v = -25·cos(60°) = -12.5 kt",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.wind.aloft-interpolate",
            reason: "next",
        },
        Related {
            id: "aviation.wind.heading-groundspeed",
            reason: "next",
        },
        Related {
            id: "aviation.wind.find-wind",
            reason: "alternative",
        },
    ],
    sentence: "The wind from {direction} at {speed} is u = {u} and v = {v}.",
    limits: &[("batchRows", 10_000)],
    run: run_uv,
    ..ToolDef::BLANK
};

fn run_uv(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (dg, kt) = (unit(QT::Angle, "deg"), unit(QT::Speed, "kt"));
    let d = ctx.quantity("direction")?.map(|q| q.to(dg));
    let s = ctx.quantity("speed")?.map(|q| q.to(kt));
    let u = ctx.quantity("u")?.map(|q| q.to(kt));
    let v = ctx.quantity("v")?.map(|q| q.to(kt));
    let (dir, speed, u, v) = match (d, s, u, v) {
        (Some(d), Some(s), None, None) => {
            if s < 0.0 {
                return Err(ToolError::invalid(
                    "/speed",
                    "Wind speed cannot be negative.",
                ));
            }
            let (u, v) = to_uv(d, s);
            (d.rem_euclid(360.0), s, u, v)
        }
        (None, None, Some(u), Some(v)) => {
            let (d, s) = from_uv(u, v);
            (d, s, u, v)
        }
        _ => {
            return Err(ToolError::invalid(
                "/direction",
                "Give the direction and speed, or u and v, but not a mix.",
            ));
        }
    };
    Ok(obj(vec![
        ("u", ctx.out("u", Q { value: u, unit: kt })),
        ("v", ctx.out("v", Q { value: v, unit: kt })),
        (
            "direction",
            ctx.out(
                "direction",
                Q {
                    value: dir,
                    unit: dg,
                },
            ),
        ),
        (
            "speed",
            ctx.out(
                "speed",
                Q {
                    value: speed,
                    unit: kt,
                },
            ),
        ),
    ]))
}

const LEVEL: &[Field] = &[
    qty("altitude", "Altitude", "Like 9000 ft", QT::Length, "ft").required(),
    qty(
        "direction",
        "Direction",
        "From, degrees true, like 300",
        QT::Angle,
        "deg",
    )
    .required(),
    qty("speed", "Speed", "Like 30 kt", QT::Speed, "kt").required(),
    qty(
        "temperature",
        "Temperature",
        "Like -5 degC; leave out if not forecast",
        QT::Temperature,
        "degC",
    ),
];

pub static ALOFT: ToolDef = ToolDef {
    id: "aviation.wind.aloft-interpolate",
    title: "Winds aloft between levels",
    summary: "The wind and temperature at your cruising altitude from the forecast levels above and below it, interpolated as a vector so a turning wind turns smoothly.",
    aliases: &["winds aloft interpolation", "wind at altitude", "interpolate winds aloft", "FB winds between levels"],
    keywords: &["winds aloft", "interpolate", "altitude", "FB", "forecast", "cruise", "temperature aloft", "wind"],
    inputs: &[
        Field::new("levels", "Forecast levels", "Altitude, direction, speed, and temperature, one per line, like 9000 ft, 300, 30 kt, -5 degC", Kind::List { items: LEVEL, min: 2, max: 20 }).required().core(),
        qty("altitude", "Your altitude", "Between two levels, like 7500 ft", QT::Length, "ft").required().core(),
    ],
    outputs: &[
        qty("direction", "Wind direction", "From, degrees true", QT::Angle, "deg").precision(Precision::Decimals(0)),
        qty("speed", "Wind speed", "Interpolated as a vector", QT::Speed, "kt").precision(Precision::Decimals(1)),
        qty("temperature", "Temperature", "Linear between the levels", QT::Temperature, "degC").precision(Precision::Decimals(1)).optional(),
        qty("u", "u (toward east)", "Interpolated", QT::Speed, "kt").precision(Precision::Decimals(2)),
        qty("v", "v (toward north)", "Interpolated", QT::Speed, "kt").precision(Precision::Decimals(2)),
        qty("below", "Level below", "Used for the interpolation", QT::Length, "ft").precision(Precision::Decimals(0)),
        qty("above", "Level above", "Used for the interpolation", QT::Length, "ft").precision(Precision::Decimals(0)),
    ],
    errors: &[ErrorCode::OutOfDomain, ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Between the two forecast levels that bracket the altitude, u, v, and temperature vary linearly with altitude; the wind is read back from u and v. No extrapolation above the highest or below the lowest level. Forecast winds are true (Aviation Weather Handbook §27.2)",
    accuracy: "Exact for the linear model; the forecast's own error is larger",
    references: &[WEATHER_HANDBOOK],
    examples: &[Example {
        id: "primary",
        title: "7,500 ft between the 6,000 and 9,000 ft winds",
        input: r#"{"levels":[{"altitude":"6000 ft","direction":"270 deg","speed":"20 kt","temperature":"3 degC"},{"altitude":"9000 ft","direction":"300 deg","speed":"30 kt","temperature":"-3 degC"}],"altitude":"7500 ft"}"#,
        source: "Hand check: u = (20 + 25.98)/2 = 22.99 kt, v = (0 - 15)/2 = -7.5 kt, so 288° at 24.2 kt; temperature 0 °C",
    }],
    primary_example: "primary",
    visualization: &[Layer { kind: "table-only", map: &[] }],
    related: &[
        Related { id: "aviation.weather.fb-winds-decode", reason: "parent" },
        Related { id: "aviation.wind.uv", reason: "alternative" },
    ],
    sentence: "At {altitude} expect wind from {direction} at {speed}.",
    limits: &[("batchRows", 10_000)],
    run: run_aloft,
    ..ToolDef::BLANK
};

fn run_aloft(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (ft, dg, kt, c) = (
        unit(QT::Length, "ft"),
        unit(QT::Angle, "deg"),
        unit(QT::Speed, "kt"),
        unit(QT::Temperature, "degC"),
    );
    let rows = ctx.rows("levels")?;
    let mut levels = Vec::with_capacity(rows.len());
    for (i, r) in rows.iter().enumerate() {
        let alt = ctx
            .row_quantity("levels", i, r, "altitude")?
            .expect("required")
            .to(ft);
        let dir = ctx
            .row_quantity("levels", i, r, "direction")?
            .expect("required")
            .to(dg);
        let spd = ctx
            .row_quantity("levels", i, r, "speed")?
            .expect("required")
            .to(kt);
        let tmp = ctx
            .row_quantity("levels", i, r, "temperature")?
            .map(|q| q.to(c));
        if spd < 0.0 {
            return Err(ToolError::invalid(
                &format!("/levels/{i}/speed"),
                "Wind speed cannot be negative.",
            ));
        }
        levels.push((alt, to_uv(dir, spd), tmp));
    }
    levels.sort_by(|a, b| a.0.total_cmp(&b.0));
    if levels.windows(2).any(|w| w[1].0 == w[0].0) {
        return Err(ToolError::invalid(
            "/levels",
            "Two levels share an altitude; give each once.",
        ));
    }
    let h = ctx.req_quantity("altitude")?.to(ft);
    let (lo, hi) = (levels[0].0, levels[levels.len() - 1].0);
    if h < lo || h > hi {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            format!(
                "{h} ft is outside the levels given ({lo} to {hi} ft); winds are not extrapolated."
            ),
        )
        .at("/altitude"));
    }
    let k = levels
        .windows(2)
        .position(|w| h <= w[1].0)
        .expect("bracketed");
    let (a, b) = (levels[k], levels[k + 1]);
    let t = (h - a.0) / (b.0 - a.0);
    let (u, v) = (a.1.0 + t * (b.1.0 - a.1.0), a.1.1 + t * (b.1.1 - a.1.1));
    let (dir, speed) = from_uv(u, v);
    let mut out = vec![
        (
            "direction",
            ctx.out(
                "direction",
                Q {
                    value: dir,
                    unit: dg,
                },
            ),
        ),
        (
            "speed",
            ctx.out(
                "speed",
                Q {
                    value: speed,
                    unit: kt,
                },
            ),
        ),
    ];
    if let (Some(ta), Some(tb)) = (a.2, b.2) {
        out.push((
            "temperature",
            ctx.out(
                "temperature",
                Q {
                    value: ta + t * (tb - ta),
                    unit: c,
                },
            ),
        ));
    }
    out.extend([
        ("u", ctx.out("u", Q { value: u, unit: kt })),
        ("v", ctx.out("v", Q { value: v, unit: kt })),
        (
            "below",
            ctx.out(
                "below",
                Q {
                    value: a.0,
                    unit: ft,
                },
            ),
        ),
        (
            "above",
            ctx.out(
                "above",
                Q {
                    value: b.0,
                    unit: ft,
                },
            ),
        ),
    ]);
    Ok(obj(out))
}
