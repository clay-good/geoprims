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

const PHAK_CABIN: Reference = Reference {
    title: "Pilot's Handbook of Aeronautical Knowledge",
    issuer: "FAA",
    year: 2023,
    edition: "FAA-H-8083-25C",
    locator: "Chapter 7, cabin pressurization (Figure 7-41, cabin altitude from differential)",
    url: "https://www.faa.gov/regulations_policies/handbooks_manuals/aviation/phak",
};

const fn pressure_field(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Pressure,
            unit: "psi",
        },
    )
}

const fn ft_field(name: &'static str, title: &'static str, help: &'static str) -> Field {
    Field::new(
        name,
        title,
        help,
        Kind::Quantity {
            q: QT::Length,
            unit: "ft",
        },
    )
}

pub static CABIN_ALTITUDE: ToolDef = ToolDef {
    id: "aviation.altimetry.cabin-altitude",
    title: "Cabin altitude from pressure differential",
    summary: "The cabin altitude a pressurized airplane holds at a flight altitude with its maximum pressure differential, or the differential a chosen cabin altitude needs, through the standard atmosphere.",
    aliases: &[
        "cabin altitude calculator",
        "pressure differential calculator",
        "cabin pressure altitude",
    ],
    keywords: &[
        "cabin",
        "pressurization",
        "differential",
        "psid",
        "pressurized",
        "altitude",
    ],
    inputs: &[
        ft_field(
            "pressure_altitude",
            "Pressure altitude",
            "Of the airplane, like 28,000 ft",
        )
        .required()
        .core(),
        pressure_field(
            "max_differential",
            "Maximum differential",
            "From the flight manual, like 6.1 psi",
        )
        .required()
        .core(),
        ft_field(
            "cabin_selected",
            "Cabin altitude selected",
            "Optional, like 8,000 ft, to find the differential it needs",
        )
        .core(),
    ],
    outputs: &[
        ft_field(
            "cabin_altitude",
            "Cabin altitude",
            "The pressure altitude inside",
        )
        .precision(Precision::Decimals(0)),
        pressure_field(
            "cabin_pressure",
            "Cabin pressure",
            "Outside pressure plus the differential in use",
        )
        .precision(Precision::Decimals(2)),
        pressure_field(
            "outside_pressure",
            "Outside pressure",
            "Standard atmosphere at the pressure altitude",
        )
        .precision(Precision::Decimals(2)),
        pressure_field(
            "differential",
            "Differential in use",
            "Cabin minus outside pressure",
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "within_max",
            "Within the maximum",
            "Whether the selected cabin altitude is held",
            Kind::Text { max_len: 4 },
        )
        .optional(),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "p_cabin = p_ISA(pressure altitude) + differential; cabin altitude = the ISA altitude of p_cabin, by the site's standard-atmosphere engine (ICAO Doc 7488); a selected cabin altitude needs p_ISA(cabin) − p_ISA(airplane), up to the maximum",
    accuracy: "Exact through the standard atmosphere; real cabins follow the controller's schedule and the airplane's limits",
    when_to_use: "Use this to see what cabin altitude a pressurized airplane can hold at a planned flight altitude with its maximum differential, or how much differential a chosen cabin altitude takes. It helps plan the cruise altitude that keeps the cabin below the oxygen thresholds.",
    limitations: "It works in pressure altitude through the standard atmosphere and assumes the cabin holds the full differential. The airplane's controller, its schedule, and its limits govern; the maximum differential comes from its flight manual. It says nothing about oxygen rules, which depend on the cabin altitude this returns.",
    references: &[PHAK_CABIN],
    examples: &[Example {
        id: "primary",
        title: "6.1 psi differential at 28,000 ft",
        input: r#"{"pressure_altitude":"28000 ft","max_differential":"6.1 psi"}"#,
        source: "PHAK (FAA-H-8083-25C) Figure 7-41: about 10.9 psi in the cabin, 8,000 ft",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.physiology.oxygen-requirement",
            reason: "next",
        },
        Related {
            id: "aviation.atmosphere.isa",
            reason: "parent",
        },
        Related {
            id: "aviation.altimetry.pressure-altitude",
            reason: "alternative",
        },
    ],
    sentence: "The cabin holds {cabin_altitude}, at {cabin_pressure}.",
    limits: &[("batchRows", 10_000)],
    run: run_cabin_altitude,
    ..ToolDef::BLANK
};

fn run_cabin_altitude(ctx: &mut Ctx) -> Result<Json, ToolError> {
    use crate::atmosphere::{altitude_for_pressure, at};
    let m = unit(QT::Length, "m");
    let pa = unit(QT::Pressure, "Pa");
    let h = ctx.req_quantity("pressure_altitude")?.to(m);
    let dp = ctx.req_quantity("max_differential")?.to(pa);
    let range = |h: f64| (-2_000.0 * 0.3048..=65_000.0 * 0.3048).contains(&h);
    if !range(h) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The pressure altitude is from -2,000 to 65,000 ft.",
        )
        .at("/pressure_altitude"));
    }
    if dp.is_nan() || dp < 0.0 || dp > 100_000.0 {
        return Err(ToolError::invalid(
            "/max_differential",
            "The differential is from 0 to 14.5 psi.",
        ));
    }
    let outside = at(h).p;
    let (cabin_p, within) = match ctx.quantity("cabin_selected")? {
        Some(c) => {
            let c = c.to(m);
            if !range(c) {
                return Err(ToolError::new(
                    ErrorCode::OutOfDomain,
                    "The selected cabin altitude is from -2,000 to 65,000 ft.",
                )
                .at("/cabin_selected"));
            }
            // A cabin at or above the airplane needs no differential.
            let need = (at(c).p - outside).max(0.0);
            if need <= dp {
                (outside + need, Some("yes"))
            } else {
                (outside + dp, Some("no"))
            }
        }
        None => (outside + dp, None),
    };
    let q = |v: f64, u| gp_base::tool::Q { value: v, unit: u };
    let mut out = vec![
        (
            "cabin_altitude",
            ctx.out("cabin_altitude", q(altitude_for_pressure(cabin_p), m)),
        ),
        ("cabin_pressure", ctx.out("cabin_pressure", q(cabin_p, pa))),
        (
            "outside_pressure",
            ctx.out("outside_pressure", q(outside, pa)),
        ),
        (
            "differential",
            ctx.out("differential", q(cabin_p - outside, pa)),
        ),
    ];
    if let Some(w) = within {
        out.push(("within_max", Json::str(w)));
    }
    Ok(obj(out))
}
