//! Practical performance additions (add-aviation-practical-additions,
//! aviation/performance-and-cabin): maneuvering speed at a weight and tire
//! hydroplaning speed; and (aviation/fuel-density) fuel density at a
//! temperature.

use gp_base::ErrorCode;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use libm::{exp, sqrt};

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
        let vs = vs.to(kt);
        if vs.is_nan() || vs <= 0.0 {
            return Err(ToolError::invalid("/vs", "VS must be more than zero."));
        }
        out.push((
            "va_rule_of_thumb",
            ctx.out("va_rule_of_thumb", knots(1.7 * vs)),
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

// ------------------------------------------------- fuel density by temperature

const CHEVRON_FTR: Reference = Reference {
    title: "Aviation Fuels Technical Review (FTR-3)",
    issuer: "Chevron Products Company (vendor guidance)",
    year: 2007,
    edition: "FTR-3, 2007",
    locator: "Chapter 2: Coefficient of Thermal Expansion (0.00099 per °C; 1.000 gal at 0 °C is 1.040 gal at 40 °C) and Figure 2.1, typical densities at 15 °C",
    url: "https://www.chevron.com/-/media/chevron/operations/documents/aviation-tech-review.pdf",
};

const MC_VCF: Reference = Reference {
    title: "Measurement Canada volume correction factor tables and bulletin V-18",
    issuer: "Measurement Canada",
    year: 2016,
    edition: "tables issued January 2016; V-18 modified 2017-02-01",
    locator: "V-18 Table 1 (Table 54B for Jet A and avgas, 54A for Jet B) and the Jet A, Jet B, and aviation gasoline VCF tables, −40 to 40 °C",
    url: "https://ised-isde.canada.ca/site/measurement-canada/en/node/93",
};

const ABB_API: Reference = Reference {
    title: "SpiritIT eXLerate function reference manual (API MPMS 11.1 metric constants)",
    issuer: "ABB Measurement & Analytics (vendor)",
    year: 2017,
    edition: "CM/eXL/FR-EN Rev. B, October 2017",
    locator: "API constants in metric units: K0 and K1 for tables 54A and 54B",
    url: "https://library.e.abb.com/public/1cd9d80704364862a1ddcfb915280467/CM_eXL_FR-EN_B_102017.pdf",
};

/// Typical densities at 15 °C (kg/m³), Chevron FTR-3 Figure 2.1.
const FUEL_DENSITIES: &[(&str, &str, f64)] = &[
    ("jet-a", "Jet A (kerosine type)", 810.0),
    ("wide-cut", "wide-cut (Jet B)", 762.0),
    ("100ll", "100LL avgas", 715.0),
];

/// Chevron's typical coefficient of thermal expansion for kerosine-type jet fuel, per °C.
const KEROSINE_BETA: f64 = 0.00099;

/// The thermal expansion coefficient at 15 °C (per °C) of the API MPMS 11.1
/// (1980) tables: Table 54A for crude oils, which Measurement Canada's V-18
/// assigns to Jet B, and Table 54B for refined products, whose four density
/// zones meet where α is continuous (770.5, 787.5, and 838.5 kg/m³).
fn api_alpha(rho15: f64, crude: bool) -> f64 {
    if crude {
        613.9723 / (rho15 * rho15)
    } else if rho15 < 770.5 {
        346.4228 / (rho15 * rho15) + 0.4388 / rho15
    } else if rho15 < 787.5 {
        -0.003_363_12 + 2680.3206 / (rho15 * rho15)
    } else if rho15 < 838.5 {
        594.5418 / (rho15 * rho15)
    } else {
        186.9696 / (rho15 * rho15) + 0.4862 / rho15
    }
}

/// The volume correction factor to 15 °C of the API tables: V15 = V(T) × VCF.
fn api_vcf(rho15: f64, t: f64, crude: bool) -> f64 {
    let a = api_alpha(rho15, crude);
    let d = t - 15.0;
    exp(-a * d * (1.0 + 0.8 * a * d))
}

/// The same factor for a constant coefficient: ρ(T) = ρ15 / (1 + β (T − 15)).
fn linear_vcf(beta: f64, t: f64) -> f64 {
    1.0 / (1.0 + beta * (t - 15.0))
}

pub static FUEL_DENSITY_TEMPERATURE: ToolDef = ToolDef {
    id: "aviation.loading.fuel-density-temperature",
    title: "Fuel density at a temperature",
    summary: "The density of jet fuel or avgas at the temperature it is loaded, from its density at 15 °C, and the weight of a volume of it, so a hot or cold ramp does not throw off the weight and balance.",
    aliases: &[
        "fuel density temperature correction",
        "jet fuel volume correction factor",
        "hot fuel weight",
    ],
    keywords: &[
        "fuel",
        "density",
        "temperature",
        "Jet A",
        "avgas",
        "VCF",
        "expansion",
    ],
    inputs: &[
        Field::new(
            "fuel",
            "Fuel type",
            "jet-a (810 kg/m³ typical), wide-cut (762), or 100ll (715), like jet-a",
            Kind::Choice(&["jet-a", "wide-cut", "100ll"]),
        )
        .core(),
        Field::new(
            "density_15",
            "Density at 15 °C",
            "From the fuel slip, like 804 kg/m3; overrides the typical value",
            Kind::Quantity {
                q: QT::Density,
                unit: "kg/m3",
            },
        )
        .core(),
        Field::new(
            "temperature",
            "Fuel temperature",
            "Like -10 °C",
            Kind::Quantity {
                q: QT::Temperature,
                unit: "degC",
            },
        )
        .required()
        .core(),
        Field::new(
            "volume",
            "Fuel volume",
            "Optional, like 100 gal, for its weight",
            Kind::Quantity {
                q: QT::Volume,
                unit: "galUS",
            },
        )
        .core(),
        Field::new(
            "method",
            "Method",
            "table (the API tables' formula, the default) or linear (one expansion coefficient), like table",
            Kind::Choice(&["table", "linear"]),
        ),
        Field::new(
            "expansion",
            "Expansion coefficient",
            "Per degree Celsius for the linear method, like 0.00099 (Chevron's typical value for kerosine-type fuel)",
            Kind::Number {
                min: 0.0,
                max: 0.01,
            },
        ),
    ],
    outputs: &[
        Field::new(
            "density",
            "Density at the temperature",
            "Density at 15 °C × the volume correction factor",
            Kind::Quantity {
                q: QT::Density,
                unit: "kg/m3",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "vcf",
            "Volume correction factor",
            "Volume at 15 °C / volume at the temperature",
            Kind::Number { min: 0.0, max: 2.0 },
        )
        .precision(Precision::Decimals(4)),
        Field::new(
            "density_15",
            "Density at 15 °C used",
            "Typical or entered",
            Kind::Quantity {
                q: QT::Density,
                unit: "kg/m3",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "weight",
            "Weight of the volume",
            "Volume × density at the temperature",
            Kind::Quantity {
                q: QT::Mass,
                unit: "lb",
            },
        )
        .precision(Precision::Decimals(1))
        .optional(),
        Field::new(
            "volume_15",
            "Volume at 15 °C",
            "The same fuel's volume at 15 °C",
            Kind::Quantity {
                q: QT::Volume,
                unit: "galUS",
            },
        )
        .precision(Precision::Decimals(2))
        .optional(),
        Field::new(
            "vcf_table",
            "Factor by the API tables",
            "Table 54B, or 54A for wide-cut",
            Kind::Number { min: 0.0, max: 2.0 },
        )
        .precision(Precision::Decimals(4)),
        Field::new(
            "vcf_linear",
            "Factor by one coefficient",
            "1 / (1 + coefficient × (temperature − 15 °C))",
            Kind::Number { min: 0.0, max: 2.0 },
        )
        .precision(Precision::Decimals(4))
        .optional(),
        Field::new(
            "density_basis",
            "Density basis",
            "typical or entered",
            Kind::Text { max_len: 40 },
        ),
    ],
    errors: &[ErrorCode::OutOfDomain, ErrorCode::InvalidInput],
    warnings: &[
        "NOMINAL_VALUE_USED",
        "MODEL_EXTRAPOLATED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "VCF = exp(−α ΔT (1 + 0.8 α ΔT)), ΔT = T − 15 °C, α = K0/ρ15² + K1/ρ15 (API MPMS 11.1 Table 54B by density zone, or 54A for wide-cut, as Measurement Canada's V-18 assigns); or VCF = 1 / (1 + β ΔT) with one coefficient; density = ρ15 × VCF",
    accuracy: "The tables' formula matches every printed cell of Measurement Canada's Jet A, Jet B, and avgas tables (−40 to 40 °C) within 0.00006; the result is nominal, because a real batch's density at 15 °C is on its fuel slip",
    when_to_use: "Use this when you load fuel on a hot or cold ramp and want its weight from the gallons or liters pumped. Enter the fuel's temperature and either pick the fuel type or type the density at 15 °C from the fuel slip. It returns the density at that temperature and, with a volume, its weight. Send the density on to the fuel weight tool and the weight and balance.",
    limitations: "Typical densities are vendor values; Jet A batches range from 775 to 840 kg/m³ at 15 °C (ASTM D1655), so use the slip's density when you have it. The tables' formula is computed without the standard's intermediate rounding, so a factor can differ from a printed table in the fourth decimal. The single coefficient is Chevron's typical value for kerosine-type fuel; for other fuels enter your own. Temperatures from −50 to 60 °C only.",
    references: &[CHEVRON_FTR, MC_VCF, ABB_API],
    examples: &[Example {
        id: "primary",
        title: "Jet A at 800 kg/m³ on a −20 °C ramp",
        input: r#"{"fuel":"jet-a","density_15":"800 kg/m3","temperature":"-20 degC"}"#,
        source: "Measurement Canada Jet A table (Table 54B, 800 kg/m³): VCF 1.0322 at −20 °C",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.loading.fuel-weight",
            reason: "next",
        },
        Related {
            id: "aviation.loading.weight-balance",
            reason: "next",
        },
        Related {
            id: "aviation.loading.fuel-plan",
            reason: "alternative",
        },
    ],
    sentence: "At {temperature}, the fuel's nominal density is {density}.{if weight > 0} That volume weighs {weight}.{/if}",
    limits: &[("batchRows", 10_000)],
    run: run_fuel_density_temperature,
    assumptions: &[
        gp_base::tool::Assumption {
            name: "Typical density of kerosine-type jet fuel at 15 °C",
            value: "810",
            unit: "kg/m3",
            source: "chevron-ftr3",
        },
        gp_base::tool::Assumption {
            name: "Typical density of wide-cut jet fuel at 15 °C",
            value: "762",
            unit: "kg/m3",
            source: "chevron-ftr3",
        },
        gp_base::tool::Assumption {
            name: "Typical density of avgas at 15 °C",
            value: "715",
            unit: "kg/m3",
            source: "chevron-ftr3",
        },
        gp_base::tool::Assumption {
            name: "Coefficient of thermal expansion of kerosine-type jet fuel",
            value: "0.00099",
            unit: "1/degC",
            source: "chevron-ftr3",
        },
    ],
    ..ToolDef::BLANK
};

fn run_fuel_density_temperature(ctx: &mut Ctx) -> Result<Json, ToolError> {
    use gp_base::error::Warning;
    let kg_m3 = unit(QT::Density, "kg/m3");
    let t = ctx
        .req_quantity("temperature")?
        .to(unit(QT::Temperature, "degC"));
    if !(-50.0..=60.0).contains(&t) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The fuel temperature is from -50 to 60 °C.",
        )
        .at("/temperature"));
    }
    let fuel = ctx.choice("fuel")?;
    let (rho15, basis) = match (ctx.quantity("density_15")?, fuel) {
        (Some(d), _) => (d.to(kg_m3), "entered"),
        (None, Some(f)) => {
            let (_, name, rho) = FUEL_DENSITIES
                .iter()
                .find(|(id, _, _)| *id == f)
                .expect("choice");
            let batch = if f == "jet-a" {
                " Jet A batches range from 775 to 840 kg/m³ (ASTM D1655);"
            } else {
                ""
            };
            ctx.warnings.push(Warning::new(
                "NOMINAL_VALUE_USED",
                format!(
                    "Uses a typical {name} density of {rho} kg/m³ at 15 °C (Chevron).{batch} enter the density on your fuel slip for close loading."
                ),
            ));
            (*rho, "typical")
        }
        (None, None) => {
            return Err(ToolError::invalid(
                "/fuel",
                "Choose a fuel type or enter the density at 15 °C.",
            )
            .hint("Example: jet-a"));
        }
    };
    if !(653.0..=1075.0).contains(&rho15) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The API tables cover densities at 15 °C from 653 to 1,075 kg/m³.",
        )
        .at("/density_15"));
    }
    if !(-40.0..=40.0).contains(&t) {
        ctx.warnings.push(Warning::new(
            "MODEL_EXTRAPOLATED",
            "Measurement Canada's tables, which check this formula, stop at -40 and 40 °C.",
        ));
    }
    let table = api_vcf(rho15, t, fuel == Some("wide-cut"));
    // Chevron's coefficient is for kerosine-type fuel: a jet fuel by type, or
    // an entered density in the jet fuel zone.
    let kerosine = match fuel {
        Some(f) => f == "jet-a",
        None => (787.5..838.5).contains(&rho15),
    };
    let beta = ctx
        .number("expansion")?
        .or(kerosine.then_some(KEROSINE_BETA));
    let linear = beta.map(|b| linear_vcf(b, t));
    let vcf = if ctx.choice("method")? == Some("linear") {
        linear.ok_or_else(|| {
            ToolError::invalid(
                "/expansion",
                "Chevron's coefficient is for kerosine-type jet fuel; enter one for this fuel.",
            )
            .hint("Example: 0.00099")
        })?
    } else {
        table
    };
    let rho = rho15 * vcf;
    let q = |v: f64, u| gp_base::tool::Q { value: v, unit: u };
    let mut out = vec![
        ("density", ctx.out("density", q(rho, kg_m3))),
        ("vcf", Json::Num(vcf)),
        ("density_15", ctx.out("density_15", q(rho15, kg_m3))),
        ("vcf_table", Json::Num(table)),
        ("density_basis", Json::str(basis)),
    ];
    if let Some(l) = linear {
        out.push(("vcf_linear", Json::Num(l)));
    }
    if let Some(v) = ctx.quantity("volume")? {
        let m3 = v.base();
        if m3 < 0.0 {
            return Err(ToolError::invalid("/volume", "Fuel cannot be negative."));
        }
        out.push((
            "weight",
            ctx.out("weight", q(m3 * rho, unit(QT::Mass, "kg"))),
        ));
        out.push((
            "volume_15",
            ctx.out("volume_15", q(m3 * vcf, unit(QT::Volume, "m3"))),
        ));
    }
    Ok(obj(out))
}
