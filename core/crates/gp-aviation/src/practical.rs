//! Practical performance additions (add-aviation-practical-additions,
//! aviation/performance-and-cabin): maneuvering speed at a weight and tire
//! hydroplaning speed.

use gp_base::ErrorCode;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use libm::sqrt;

use crate::{knots, obj, unit};

const CFR_23_335: Reference = Reference {
    title: "14 CFR 23.335, Design airspeeds",
    issuer: "FAA (eCFR)",
    year: 2016,
    edition: "text in force until 2017-08-30 (removed by the Part 23 rewrite)",
    locator: "Paragraph (c), design maneuvering speed: V_A not less than V_S √n",
    url: "https://www.ecfr.gov/api/versioner/v1/full/2017-01-01/title-14.xml?section=23.335",
};

const TN_D_2056: Reference = Reference {
    title: "NASA TN D-2056, Phenomena of Pneumatic Tire Hydroplaning",
    issuer: "Horne, W. B., and Dreher, R. C., NASA Langley Research Center",
    year: 1963,
    edition: "November 1963",
    locator: "Equations 4 and 5 and the validity they state, page 5",
    url: "https://ntrs.nasa.gov/citations/19640000612",
};

const fn kt_field(name: &'static str, title: &'static str, help: &'static str) -> Field {
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

const fn lb_field(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Mass,
            unit: "lb",
        },
    )
}

pub static VA_FOR_WEIGHT: ToolDef = ToolDef {
    id: "aviation.performance.va-for-weight",
    title: "Maneuvering speed at a weight",
    summary: "Maneuvering speed (VA) at today's weight from the published VA at maximum weight, scaled by the square root of the weight ratio; lighter airplanes get a lower VA.",
    aliases: &[
        "maneuvering speed calculator",
        "Va at weight",
        "Va for weight",
    ],
    keywords: &[
        "maneuvering",
        "Va",
        "weight",
        "turbulence",
        "speed",
        "airspeed",
    ],
    inputs: &[
        kt_field(
            "va_max",
            "VA at maximum weight",
            "From the flight manual, like 105 kt",
        )
        .required()
        .core(),
        lb_field(
            "max_weight",
            "Maximum weight",
            "The weight that VA is for, like 2,550 lb",
        )
        .required()
        .core(),
        lb_field("weight", "Weight now", "Like 2,000 lb")
            .required()
            .core(),
        kt_field(
            "vs",
            "Stall speed",
            "Optional, clean stall speed, like 55 kt, for the 1.7 × VS cross-check",
        ),
    ],
    outputs: &[
        kt_field("va", "Maneuvering speed", "VA at this weight").precision(Precision::Decimals(0)),
        Field::new(
            "ratio",
            "Weight ratio",
            "Weight now / maximum weight",
            Kind::Number { min: 0.0, max: 1.0 },
        )
        .precision(Precision::Significant(4)),
        kt_field(
            "va_rule_of_thumb",
            "1.7 × VS",
            "The handbook cross-check, when VS is given",
        )
        .precision(Precision::Decimals(0))
        .optional(),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "VA = VA,max × √(W / Wmax), derived from 14 CFR 23.335(c) (VA not less than VS √n, pre-2017 text) with VS proportional to √W; cross-check VA ≈ 1.7 VS (PHAK ch. 5)",
    accuracy: "Exact for the square-root relation; your flight manual governs if it publishes VA by weight",
    when_to_use: "Use this before flying into turbulence at less than maximum weight: maneuvering speed falls as the airplane gets lighter, so the VA on the placard, set at maximum weight, is too fast today. Enter the published VA, the weight it is for, and your weight now.",
    limitations: "It applies the square-root rule that follows from the stall speed changing with weight; it is not a certified value, and the flight manual governs where it lists VA by weight. It works only at or below the maximum weight. Full control deflections can still overstress the airplane at or below VA in some cases, so treat it as a limit, not a target.",
    references: &[CFR_23_335],
    examples: &[Example {
        id: "primary",
        title: "VA 105 kt at 2,550 lb, flying at 2,000 lb",
        input: r#"{"va_max":"105 kt","max_weight":"2550 lb","weight":"2000 lb"}"#,
        source: "add-aviation-practical-additions design A7: 92.99 kt",
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
            id: "aviation.airspeed.cas-to-tas",
            reason: "next",
        },
        Related {
            id: "aviation.performance.turn",
            reason: "alternative",
        },
    ],
    sentence: "At this weight, maneuvering speed is {va}.",
    limits: &[("batchRows", 10_000)],
    run: run_va_for_weight,
    ..ToolDef::BLANK
};

fn run_va_for_weight(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let kt = unit(QT::Speed, "kt");
    let lb = unit(QT::Mass, "lb");
    let va = ctx.req_quantity("va_max")?.to(kt);
    let wmax = ctx.req_quantity("max_weight")?.to(lb);
    let w = ctx.req_quantity("weight")?.to(lb);
    if va.is_nan() || va <= 0.0 {
        return Err(ToolError::invalid("/va_max", "VA must be more than zero."));
    }
    if wmax.is_nan() || wmax <= 0.0 || w.is_nan() || w <= 0.0 {
        return Err(ToolError::invalid(
            "/weight",
            "Weights must be more than zero.",
        ));
    }
    if w > wmax {
        return Err(ToolError::new(ErrorCode::OutOfDomain, "The weight is above the maximum weight; the formula applies at or below maximum weight.").at("/weight"));
    }
    let ratio = w / wmax;
    let mut out = vec![
        ("va", ctx.out("va", knots(va * sqrt(ratio)))),
        ("ratio", Json::Num(ratio)),
    ];
    if let Some(vs) = ctx.quantity("vs")? {
        out.push((
            "va_rule_of_thumb",
            ctx.out("va_rule_of_thumb", knots(1.7 * vs.to(kt))),
        ));
    }
    Ok(obj(out))
}

pub static HYDROPLANING_SPEED: ToolDef = ToolDef {
    id: "aviation.performance.hydroplaning-speed",
    title: "Tire hydroplaning speed",
    summary: "The groundspeed at which a tire starts to hydroplane on a flooded runway, from the tire pressure: 9 × √pressure knots (NASA TN D-2056), with the airplane handbook's 8.6 × √pressure as an option.",
    aliases: &[
        "hydroplaning speed calculator",
        "aquaplaning speed",
        "9 root p",
    ],
    keywords: &[
        "hydroplaning",
        "aquaplaning",
        "tire",
        "pressure",
        "wet",
        "runway",
    ],
    inputs: &[
        Field::new(
            "pressure",
            "Tire pressure",
            "Inflation pressure, like 36 psi",
            Kind::Quantity {
                q: QT::Pressure,
                unit: "psi",
            },
        )
        .required()
        .core(),
        Field::new(
            "constant",
            "Formula",
            "nasa (9 √p, the default) or afh (8.6 √p)",
            Kind::Choice(&["nasa", "afh"]),
        )
        .core(),
    ],
    outputs: &[
        kt_field("speed", "Hydroplaning speed", "Groundspeed").precision(Precision::Decimals(1)),
        kt_field("speed_nasa", "NASA, 9 √p", "NASA TN D-2056 eq. 4")
            .precision(Precision::Decimals(1)),
        kt_field(
            "speed_afh",
            "Handbook, 8.6 √p",
            "Airplane Flying Handbook variant",
        )
        .precision(Precision::Decimals(1)),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "V_P = 9 √p knots, p in psi (NASA TN D-2056 eq. 4), or 8.6 √p (Airplane Flying Handbook); valid for 24 to 150 psi",
    accuracy: "The report found reasonable agreement with tests on tires from 925 to 22,000 lb load and 24 to 150 psi; it is an estimate, not a guarantee",
    when_to_use: "Use this to know the speed above which a tire can ride up on standing water and lose braking and steering, before landing or taking off on a wet runway. Enter the tire's inflation pressure; the answer is a groundspeed.",
    limitations: "The formula holds for smooth and closed-pattern treads, and for rib treads only where the water is deeper than the tread grooves; it ignores tread wear, water viscosity, and runway texture. It was correlated for 24 to 150 psi, and values outside that range are refused. Once a tire is hydroplaning it can keep doing so below this speed, so slow down well before it.",
    references: &[TN_D_2056],
    examples: &[Example {
        id: "primary",
        title: "A 36 psi main tire",
        input: r#"{"pressure":"36 psi"}"#,
        source: "NASA TN D-2056 eq. 4: 9 × √36 = 54 kt",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.wind.runway-components",
            reason: "alternative",
        },
        Related {
            id: "aviation.weather.metar-decode",
            reason: "alternative",
        },
        Related {
            id: "aviation.performance.climb-gradient",
            reason: "next",
        },
    ],
    sentence: "The tires can hydroplane from about {speed} groundspeed.",
    limits: &[("batchRows", 10_000)],
    run: run_hydroplaning_speed,
    ..ToolDef::BLANK
};

fn run_hydroplaning_speed(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let p = ctx.req_quantity("pressure")?.to(unit(QT::Pressure, "psi"));
    if !(24.0..=150.0).contains(&p) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "NASA TN D-2056 correlated the formula for 24 to 150 psi.",
        )
        .at("/pressure"));
    }
    let (nasa, afh) = (9.0 * sqrt(p), 8.6 * sqrt(p));
    let v = if ctx.choice("constant")? == Some("afh") {
        afh
    } else {
        nasa
    };
    Ok(obj(vec![
        ("speed", ctx.out("speed", knots(v))),
        ("speed_nasa", ctx.out("speed_nasa", knots(nasa))),
        ("speed_afh", ctx.out("speed_afh", knots(afh))),
    ]))
}
