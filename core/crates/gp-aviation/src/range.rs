//! Specific range (add-aviation-suite, flight performance): distance flown per
//! unit of fuel, through the air at TAS and over the ground at groundspeed,
//! and the fuel each 100 NM takes. Your fuel flow, from the POH or the gauge.

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

const fn per_gal(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(name, title, help, Kind::Number { min: 0.0, max: 1e6 })
        .measure("specific_range", "NM/galUS")
        .precision(Precision::Decimals(2))
}

const PHAK_RANGE: Reference = Reference {
    locator: "Aircraft Performance chapter (range performance and specific range: distance per unit of fuel)",
    ..PHAK
};

pub static SPECIFIC_RANGE: ToolDef = ToolDef {
    id: "aviation.performance.specific-range",
    version: "1.0.1",
    title: "Specific range",
    summary: "How far each gallon of fuel takes you, through the air and over the ground, and the fuel each 100 NM takes, from your airspeed and fuel flow.",
    aliases: &[
        "nautical miles per gallon",
        "NM per gallon",
        "fuel economy",
        "fuel per 100 NM",
    ],
    keywords: &[
        "specific range",
        "range",
        "fuel flow",
        "gph",
        "economy",
        "miles per gallon",
    ],
    inputs: &[
        qty("tas", "True airspeed", "Like 120 kt", QT::Speed, "kt")
            .required()
            .core(),
        qty(
            "fuel_flow",
            "Fuel flow",
            "From the POH cruise table or the gauge, like 8.5 gal/h",
            QT::VolumeFlow,
            "galUS/h",
        )
        .required()
        .core(),
        qty(
            "groundspeed",
            "Groundspeed",
            "For range over the ground, like 105 kt",
            QT::Speed,
            "kt",
        )
        .core(),
    ],
    outputs: &[
        per_gal(
            "air_range",
            "Air distance per gallon",
            "TAS ÷ fuel flow, NM per US gallon",
        ),
        qty(
            "fuel_per_100nm",
            "Fuel per 100 NM through the air",
            "100 × fuel flow ÷ TAS",
            QT::Volume,
            "galUS",
        )
        .precision(Precision::Decimals(1)),
        per_gal(
            "ground_range",
            "Ground distance per gallon",
            "Groundspeed ÷ fuel flow, NM per US gallon",
        )
        .optional(),
        qty(
            "ground_fuel_per_100nm",
            "Fuel per 100 NM over the ground",
            "100 × fuel flow ÷ groundspeed",
            QT::Volume,
            "galUS",
        )
        .precision(Precision::Decimals(1))
        .optional(),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Specific range = speed ÷ fuel flow: through the air with TAS, over the ground with groundspeed (PHAK, aircraft performance). Fuel per 100 NM is its reciprocal times 100",
    accuracy: "Exact for the entered speed and flow; as good as the POH or gauge flow you enter. Wind changes the ground figure, not the air one",
    references: &[PHAK_RANGE],
    examples: &[Example {
        id: "primary",
        title: "120 kt true at 8.5 gal/h, 105 kt over the ground",
        input: r#"{"tas":"120 kt","fuel_flow":"8.5 gph","groundspeed":"105 kt"}"#,
        source: "Worked from the definition: 120 ÷ 8.5 = 14.12 NM per gallon through the air, 105 ÷ 8.5 = 12.35 over the ground",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.loading.fuel-plan",
            reason: "next",
        },
        Related {
            id: "aviation.wind.heading-groundspeed",
            reason: "parent",
        },
    ],
    sentence: "Each US gallon takes you {air_range} NM through the air; 100 NM takes {fuel_per_100nm}.",
    limits: &[("batchRows", 10_000)],
    run: run_specific_range,
    ..ToolDef::BLANK
};

fn run_specific_range(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (kt, gph, gal) = (
        unit(QT::Speed, "kt"),
        unit(QT::VolumeFlow, "galUS/h"),
        unit(QT::Volume, "galUS"),
    );
    let tas = ctx.req_quantity("tas")?.to(kt);
    let ff = ctx.req_quantity("fuel_flow")?.to(gph);
    if tas.is_nan() || tas <= 0.0 {
        return Err(ToolError::invalid(
            "/tas",
            "True airspeed must be above zero.",
        ));
    }
    if ff.is_nan() || ff <= 0.0 {
        return Err(ToolError::invalid(
            "/fuel_flow",
            "Fuel flow must be above zero, like 8.5 gal/h.",
        ));
    }
    let gs = ctx.quantity("groundspeed")?.map(|q| q.to(kt));
    if gs.is_some_and(|g| g.is_nan() || g <= 0.0) {
        return Err(ToolError::invalid(
            "/groundspeed",
            "Groundspeed must be above zero.",
        ));
    }
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Fuel per 100 NM",
            "100 × fuel flow ÷ TAS",
            format!("100 × {} gal/h ÷ {} kt", n(ff, 2), n(tas, 1)),
            display::quantity(100.0 * ff / tas, "galUS", Precision::Decimals(1), fmt),
        );
        ctx.step(
            "Air distance per gallon",
            "TAS ÷ fuel flow",
            format!("{} kt ÷ {} gal/h", n(tas, 1), n(ff, 2)),
            display::quantity(tas / ff, "NM/galUS", Precision::Decimals(2), fmt),
        );
    }
    let g = |v: f64| Q {
        value: v,
        unit: gal,
    };
    let mut out = vec![
        ("air_range", Json::Num(tas / ff)),
        (
            "fuel_per_100nm",
            ctx.out("fuel_per_100nm", g(100.0 * ff / tas)),
        ),
    ];
    if let Some(gs) = gs {
        out.push(("ground_range", Json::Num(gs / ff)));
        out.push((
            "ground_fuel_per_100nm",
            ctx.out("ground_fuel_per_100nm", g(100.0 * ff / gs)),
        ));
    }
    Ok(obj(out))
}
