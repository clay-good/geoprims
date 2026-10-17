//! Endurance and power (drone/endurance-and-power spec): battery energy and
//! C-rate, momentum-theory hover power, endurance with reserve and cold
//! derating, maximum payload, and the return-to-home energy budget. Losses,
//! reserves, and air density are explicit; heuristics are labeled.

use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{
    Assumption, Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef,
};
use gp_base::units::{self, Quantity as QT};
use libm::{pow, sqrt};

pub const LEISHMAN: Reference = Reference {
    title: "Principles of Helicopter Aerodynamics",
    issuer: "Leishman, J. G., Cambridge University Press",
    year: 2006,
    edition: "2nd edition",
    locator: "Chapter 2 (momentum theory: ideal hover power T^1.5/√(2ρA), figure of merit)",
    url: "https://doi.org/10.1017/CBO9780511809569",
};
pub const LEISHMAN_SYAL: Reference = Reference {
    title: "Figure of Merit Definition for Coaxial Rotors",
    issuer: "Leishman, J. G., and Syal, M., Journal of the American Helicopter Society",
    year: 2008,
    edition: "Volume 53, issue 3",
    locator: "Momentum theory for a coaxial pair: induced power interference factor 1.2818 for a lower rotor in the fully developed slipstream of the upper rotor",
    url: "https://doi.org/10.4050/JAHS.53.290",
};
pub const LEISHMAN_FLIGHT_VEHICLES: Reference = Reference {
    title: "Introduction to Aerospace Flight Vehicles",
    issuer: "Leishman, J. G., Embry-Riddle Aeronautical University",
    year: 2026,
    edition: "2026 webbook edition",
    locator: "Chapter 67, hover-power worked example: 45,000 lb, two 38 ft rotors, 3,940.4 kW ideal and 5,253.8 kW at a 0.75 figure of merit",
    url: "https://eaglepubs.erau.edu/introductiontoaerospaceflightvehicles/chapter/helicopters/",
};
pub const ICAO_ATM: Reference = Reference {
    title: "Manual of the ICAO Standard Atmosphere, Doc 7488/3",
    issuer: "International Civil Aviation Organization",
    year: 1993,
    edition: "3rd edition",
    locator: "Troposphere equations (air density by altitude)",
    url: "https://store.icao.int/en/manual-of-the-icao-standard-atmosphere-extended-to-80-kilometres-262500-feet-doc-7488",
};

pub const BAUERSFELD: Reference = Reference {
    title: "Range, Endurance, and Optimal Speed Estimates for Multicopters",
    issuer: "Bauersfeld, L., and Scaramuzza, D., IEEE Robotics and Automation Letters 7(2)",
    year: 2022,
    edition: "Vol. 7, No. 2, pp. 2953-2960",
    locator: "Sec. VII-E, step 6 (endurance = effective capacity × nominal voltage / electrical power)",
    url: "https://arxiv.org/abs/2109.04741",
};
pub const VAN_MULDERS: Reference = Reference {
    title: "UAV-Based Servicing of IoT Nodes: Assessment of Ecological Impact",
    issuer: "Van Mulders et al., Sensors",
    year: 2023,
    edition: "Volume 23, issue 4, article 2291",
    locator: "Section 2.4.1, equation 6 and figure 4a: measured payload-hover fit P = 10.6 W + 12.88(mg)^1.5",
    url: "https://doi.org/10.3390/s23042291",
};
pub const FAA_FOSTERING_AVIATION: Reference = Reference {
    title: "Fostering Aviation Activities",
    issuer: "Federal Aviation Administration",
    year: 1995,
    edition: "Junior High Level aviation curriculum guide",
    locator: "Air Speed Indicator, pp. 23-25: groundspeed is airspeed minus headwind; worked result 134.4 mph - 30 mph = 104.4 mph",
    url: "https://www.faa.gov/sites/faa.gov/files/education/educators/curriculum/k12/Middle_Fostering_Aviation_Activities.pdf",
};
const GEOPRIMS_ENDURANCE_SPEC: Reference = Reference {
    title: "GeoPrims endurance and power specification",
    issuer: "GeoPrims",
    year: 2026,
    edition: "repository specification, accessed 2026-10-12",
    locator: "Endurance and range; Peukert and other battery models are labeled",
    url: "https://github.com/clay-good/geoprims/blob/main/openspec/changes/add-drone-suite/specs/drone/endurance-and-power/spec.md",
};

const G0: f64 = 9.806_65;
const R: f64 = 287.052_87;
const T0: f64 = 288.15;
const P0: f64 = 101_325.0;
const L: f64 = 0.0065;
const RHO0: f64 = P0 / (R * T0);

fn q(v: f64, qt: QT, s: &str) -> Q {
    Q {
        value: v,
        unit: units::by_symbol(qt, s).expect("registered unit"),
    }
}

const fn qty(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    qt: QT,
    u: &'static str,
) -> Field {
    Field::new(name, title, help, Kind::Quantity { q: qt, unit: u })
}

const fn out(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    qt: QT,
    u: &'static str,
    d: u8,
) -> Field {
    qty(name, title, help, qt, u).precision(Precision::Decimals(d))
}

const fn num(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    min: f64,
    max: f64,
) -> Field {
    Field::new(name, title, help, Kind::Number { min, max })
}

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

/// Fraction from a percent-style number input (0-100), or the default.
fn pct(ctx: &Ctx, name: &str, default: f64) -> Result<f64, ToolError> {
    Ok(ctx.number(name)?.map_or(default, |p| p / 100.0))
}

// ---------------------------------------------------------------- density

/// ISA troposphere density (kg/m³) at geopotential altitude `h` (m), optionally
/// at a measured temperature `t` (K) instead of ISA.
pub fn density(h: f64, t: Option<f64>) -> f64 {
    let t_isa = T0 - L * h;
    let p = P0 * pow(t_isa / T0, G0 / (R * L));
    p / (R * t.unwrap_or(t_isa))
}

const DENSITY_INPUTS: [Field; 3] = [
    qty(
        "altitude",
        "Altitude",
        "Pressure altitude of the site, like 5000 ft; default sea level",
        QT::Length,
        "ft",
    ),
    qty(
        "temperature",
        "Air temperature",
        "Like 30 degC; default ISA for the altitude",
        QT::Temperature,
        "degC",
    ),
    qty(
        "density_altitude",
        "Density altitude",
        "Instead of altitude and temperature, like 8000 ft",
        QT::Length,
        "ft",
    ),
];

/// Air density from the density inputs (ISA sea level when none are given).
fn read_density(ctx: &mut Ctx) -> Result<f64, ToolError> {
    let alt = ctx.quantity("altitude")?.map(|x| x.base());
    let t = ctx.quantity("temperature")?.map(|x| x.base());
    let da = ctx.quantity("density_altitude")?.map(|x| x.base());
    let check = |h: f64, at: &str| {
        if (-1000.0..=11_000.0).contains(&h) {
            Ok(h)
        } else {
            Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "Altitude must be between -1,000 m and 11,000 m (the troposphere).",
            )
            .at(at))
        }
    };
    match (da, alt, t) {
        (Some(_), Some(_), _) | (Some(_), _, Some(_)) => Err(ToolError::invalid(
            "/density_altitude",
            "Give a density altitude, or an altitude and temperature, not both.",
        )),
        (Some(d), None, None) => Ok(density(check(d, "/density_altitude")?, None)),
        (None, a, t) => {
            if t.is_some_and(|t| t <= 0.0) {
                return Err(ToolError::invalid(
                    "/temperature",
                    "The temperature must be above absolute zero.",
                ));
            }
            Ok(density(check(a.unwrap_or(0.0), "/altitude")?, t))
        }
    }
}

// ---------------------------------------------------------------- battery

pub static BATTERY_ENERGY: ToolDef = ToolDef {
    id: "drone.power.battery-energy",
    version: "1.0.1",
    stability: gp_base::tool::Stability::Stable,
    title: "Battery energy (mAh to Wh)",
    summary: "Battery energy in watt-hours from capacity and voltage (or cell count), usable energy after a discharge limit and landing reserve, and the current and C-rate for a power draw.",
    aliases: &[
        "mAh to Wh",
        "drone battery Wh calculator",
        "LiPo energy calculator",
        "C rating calculator",
    ],
    keywords: &[
        "mAh",
        "Wh",
        "battery",
        "LiPo",
        "Li-ion",
        "cells",
        "C-rate",
        "current",
        "usable energy",
        "airline battery limit",
    ],
    inputs: &[
        qty(
            "capacity",
            "Capacity",
            "Like 5870 mAh",
            QT::ElectricCharge,
            "mAh",
        )
        .required()
        .core(),
        qty(
            "voltage",
            "Nominal voltage",
            "Like 15.4 V; or give cells instead",
            QT::ElectricPotential,
            "V",
        )
        .core(),
        num("cells", "Cells in series", "Like 4 (4S)", 1.0, 30.0).core(),
        Field::new(
            "chemistry",
            "Chemistry",
            "lipo (3.7 V per cell, default) or li-ion (3.6 V per cell)",
            Kind::Choice(&["lipo", "li-ion"]),
        ),
        num(
            "depth_of_discharge",
            "Depth-of-discharge limit (%)",
            "Share of the pack you allow yourself to use, default 100",
            1.0,
            100.0,
        ),
        num(
            "reserve",
            "Landing reserve (%)",
            "Share of the pack kept for landing, default 0",
            0.0,
            99.0,
        ),
        qty(
            "power",
            "Power draw",
            "Like 150 W, for current and C-rate",
            QT::Power,
            "W",
        )
        .core(),
        num(
            "max_c_rate",
            "Maximum continuous C-rating",
            "From the pack label, like 25",
            0.1,
            500.0,
        ),
    ],
    outputs: &[
        out(
            "energy",
            "Energy",
            "Capacity × nominal voltage",
            QT::Energy,
            "Wh",
            1,
        ),
        out(
            "usable_energy",
            "Usable energy",
            "Energy × (depth-of-discharge limit − reserve)",
            QT::Energy,
            "Wh",
            1,
        ),
        out(
            "voltage",
            "Nominal voltage",
            "Entered, or cells × volts per cell",
            QT::ElectricPotential,
            "V",
            2,
        ),
        num("current", "Current", "Power / nominal voltage", 0.0, 1e9)
            .precision(Precision::Decimals(1))
            .measure("electric_current", "A")
            .optional(),
        num("c_rate", "C-rate", "Current / capacity in Ah", 0.0, 1e9)
            .precision(Precision::Decimals(2))
            .optional(),
    ],
    warnings: &[
        "C_RATE_EXCEEDED",
        "NOMINAL_VALUE_USED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "E = capacity × nominal voltage; usable = E × (DoD limit − reserve); I = P / V; C-rate = I / capacity",
    accuracy: "Exact at the nominal voltage. Real packs deliver less when cold, old, or at high current.",
    when_to_use: "Use this when planning endurance or checking what a pack can carry: it converts capacity and voltage into watt-hours, subtracts the depth-of-discharge limit and the reserve you land with to give the energy you can actually use, and turns a power draw into a current and a C-rate you can compare against the cell's rating.",
    limitations: "It is nameplate arithmetic at the nominal voltage. A real pack delivers less when it is cold, old, or worked hard, its voltage sags under load, and the C-rate a manufacturer prints is not a promise at every state of charge. Transport and shipping rules count watt-hours the same way this does, but the limits themselves are the carrier's.",
    references: &[LEISHMAN],
    examples: &[Example {
        id: "primary",
        title: "A 5,870 mAh, 15.4 V pack",
        input: r#"{"capacity":"5870 mAh","voltage":"15.4 V"}"#,
        source: "add-drone-suite battery scenario: about 90.4 Wh",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "gauge",
        map: &[("value", "energy")],
    }],
    related: &[
        Related {
            id: "drone.power.endurance",
            reason: "next",
        },
        Related {
            id: "drone.power.hover-power",
            reason: "next",
        },
        Related {
            id: "drone.power.rth-budget",
            reason: "next",
        },
    ],
    sentence: "The pack holds {energy}, with {usable_energy} usable.{if c_rate > 0} It runs at {c_rate} C.{/if}{warn C_RATE_EXCEEDED} That is above its rating.{/warn}",
    limits: &[("batchRows", 10_000)],
    run: run_battery,
    ..ToolDef::BLANK
};

pub static C_RATE: ToolDef = ToolDef {
    id: "drone.power.c-rate",
    stability: gp_base::tool::Stability::Stable,
    title: "Battery C-rate for a power draw",
    summary: "The current a power draw pulls from a battery and the C-rate that is for the pack's capacity, checked against the pack's continuous rating.",
    aliases: &[
        "C-rate calculator",
        "battery discharge rate",
        "amps from watts and volts",
    ],
    keywords: &[
        "C-rate",
        "discharge rate",
        "current",
        "amps",
        "continuous rating",
        "LiPo",
        "burst",
    ],
    inputs: &[
        BATTERY_ENERGY.inputs[0],
        BATTERY_ENERGY.inputs[6].required().core(),
        BATTERY_ENERGY.inputs[1],
        BATTERY_ENERGY.inputs[2],
        BATTERY_ENERGY.inputs[3],
        BATTERY_ENERGY.inputs[7],
    ],
    outputs: &[
        Field::new(
            "c_rate",
            "C-rate",
            "Current ÷ capacity in amp-hours",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .precision(Precision::Significant(3)),
        Field::new(
            "current",
            "Current",
            "Power ÷ voltage, in amps",
            Kind::Number { min: 0.0, max: 1e9 },
        )
        .precision(Precision::Significant(3)),
        BATTERY_ENERGY.outputs[2],
    ],
    errors: &[],
    warnings: &["NOMINAL_VALUE_USED", "C_RATE_EXCEEDED", "EXPERIMENTAL_TOOL"],
    model: "Current I = P / V; C-rate = I ÷ capacity in amp-hours, so 1 C empties the pack in an hour",
    accuracy: "Exact at the nominal voltage; under load the voltage sags and the current for the same power rises",
    when_to_use: "Use this to check that a battery can deliver what an aircraft draws. Enter the pack's capacity, its nominal voltage or cell count, and the power draw, such as hover power or the peak in a climb. The tool returns the current and the C-rate. Add the pack's continuous C rating to be warned when the draw is over it.",
    limitations: "The current is worked at the nominal voltage, so near the end of a flight, in the cold, or under a heavy load the real current is higher. A pack's printed C rating is the maker's claim and is often optimistic, and burst ratings hold only for seconds. This checks one steady draw, not the peaks of aggressive flying, and it says nothing about connector or wire limits.",
    references: BATTERY_ENERGY.references,
    examples: &[Example {
        id: "primary",
        title: "480 W from a 6S 10,000 mAh pack rated 10 C",
        input: r#"{"capacity":"10000 mAh","cells":6,"power":"480 W","max_c_rate":10}"#,
        source: "Arithmetic: 480 W at 22.2 V is 21.6 A, which is 2.16 C of 10 Ah",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "drone.power.battery-energy",
            reason: "parent",
        },
        Related {
            id: "drone.power.hover-power",
            reason: "parent",
        },
        Related {
            id: "drone.power.endurance",
            reason: "next",
        },
    ],
    sentence: "The draw is {c_rate} C, a current of {current} A.{warn C_RATE_EXCEEDED} That is over the pack's continuous rating.{/warn}",
    limits: &[("batchRows", 10_000)],
    run: run_c_rate,
    ..ToolDef::BLANK
};

fn run_c_rate(ctx: &mut Ctx) -> Result<Json, ToolError> {
    battery(ctx, true)
}

fn run_battery(ctx: &mut Ctx) -> Result<Json, ToolError> {
    battery(ctx, false)
}

/// The battery tool. `rate_only` takes a required power draw and reports the
/// C-rate and current without the energy budget.
fn battery(ctx: &mut Ctx, rate_only: bool) -> Result<Json, ToolError> {
    let cap_c = positive(ctx, "capacity", "Capacity")?;
    let v = match (ctx.quantity("voltage")?, ctx.number("cells")?) {
        (Some(v), None) => v.base(),
        (None, Some(n)) => {
            if n.fract() != 0.0 {
                return Err(ToolError::invalid(
                    "/cells",
                    "Cells must be a whole number.",
                ));
            }
            let (per, name) = if ctx.choice("chemistry")? == Some("li-ion") {
                (3.6, "Li-ion")
            } else {
                (3.7, "LiPo")
            };
            ctx.warnings.push(Warning::new(
                "NOMINAL_VALUE_USED",
                format!("Uses the nominal {name} cell voltage of {per} V. Enter the pack's own voltage if it differs."),
            ));
            n * per
        }
        (Some(_), Some(_)) => {
            return Err(ToolError::invalid(
                "/cells",
                "Give a voltage or a cell count, not both.",
            ));
        }
        (None, None) => {
            return Err(ToolError::invalid(
                "/voltage",
                "Give the nominal voltage or the cell count.",
            )
            .hint("Example: 15.4 V, or 4 cells"));
        }
    };
    if v <= 0.0 {
        return Err(ToolError::invalid("/voltage", "Voltage must be positive."));
    }
    if rate_only {
        let p = positive(ctx, "power", "Power")?;
        let i = p / v;
        let c = i / (cap_c / 3600.0);
        c_rate_check(ctx, c)?;
        return Ok(Json::obj(vec![
            ("c_rate", Json::Num(c)),
            ("current", Json::Num(i)),
            (
                "voltage",
                ctx.out("voltage", q(v, QT::ElectricPotential, "V")),
            ),
        ]));
    }
    let dod = pct(ctx, "depth_of_discharge", 1.0)?;
    let reserve = pct(ctx, "reserve", 0.0)?;
    if reserve >= dod {
        return Err(ToolError::invalid(
            "/reserve",
            "The reserve must be less than the depth-of-discharge limit.",
        ));
    }
    let e = cap_c * v; // C·V = J
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        let (mah, ah) = (cap_c / 3.6, cap_c / 3600.0);
        // Only worth a line when the pack is not simply used in full.
        if dod < 1.0 || reserve > 0.0 {
            ctx.step(
                "Usable share",
                "usable = depth of discharge − reserve",
                format!("{}% − {}%", n(dod * 100.0, 0), n(reserve * 100.0, 0)),
                format!("{}%", n((dod - reserve) * 100.0, 0)),
            );
        }
        ctx.step(
            "Charge",
            "Ah = mAh / 1000",
            format!("{} mAh / 1000", n(mah, 0)),
            format!("{} Ah", n(ah, 3)),
        );
        ctx.step(
            "Energy in the pack",
            "Wh = Ah × V",
            format!("{} Ah × {} V", n(ah, 3), n(v, 2)),
            format!("{} Wh", n(e / 3600.0, 2)),
        );
    }
    let mut o = vec![
        ("energy", ctx.out("energy", q(e, QT::Energy, "J"))),
        (
            "usable_energy",
            ctx.out("usable_energy", q(e * (dod - reserve), QT::Energy, "J")),
        ),
        (
            "voltage",
            ctx.out("voltage", q(v, QT::ElectricPotential, "V")),
        ),
    ];
    if let Some(p) = ctx.quantity("power")? {
        let i = p.base() / v;
        let c = i / (cap_c / 3600.0);
        o.push(("current", Json::Num(i)));
        o.push(("c_rate", Json::Num(c)));
        c_rate_check(ctx, c)?;
    }
    Ok(Json::obj(o))
}

/// Warns when a C-rate is over the pack's continuous rating, if one was given.
fn c_rate_check(ctx: &mut Ctx, c: f64) -> Result<(), ToolError> {
    if let Some(max) = ctx.number("max_c_rate")?
        && c > max
    {
        ctx.warnings.push(
            Warning::new(
                "C_RATE_EXCEEDED",
                format!(
                    "The draw is {} C, {} times the pack's {} C continuous rating.",
                    display::number(c, Precision::Decimals(1), ctx.options.format),
                    display::number(c / max, Precision::Decimals(2), ctx.options.format),
                    display::number(max, Precision::Significant(3), ctx.options.format)
                ),
            )
            .at("/power"),
        );
    }
    Ok(())
}

// ---------------------------------------------------------------- hover power

const HOVER_INPUTS: [Field; 5] = [
    qty(
        "mass",
        "Takeoff mass",
        "Everything on board, like 1.4 kg",
        QT::Mass,
        "kg",
    )
    .required()
    .core(),
    num("rotors", "Rotors", "Like 4", 1.0, 32.0)
        .required()
        .core(),
    qty(
        "rotor_diameter",
        "Rotor diameter",
        "Like 9.4 in",
        QT::Length,
        "in",
    )
    .required()
    .core(),
    num(
        "figure_of_merit",
        "Figure of merit",
        "Rotor efficiency vs ideal, 0.4 to 0.8, default 0.6",
        0.4,
        0.8,
    ),
    num(
        "efficiency",
        "Motor and speed-controller efficiency",
        "0.5 to 1, default 0.85",
        0.5,
        1.0,
    ),
];

/// Electrical hover power (W) and its parts for mass `m` (kg).
struct Hover {
    area: f64,
    ideal: f64,
    electrical: f64,
}

/// Total rotor disk area (m²) of `n` rotors of diameter `d` (m).
fn disk_area(n: f64, d: f64) -> f64 {
    n * core::f64::consts::PI * d * d / 4.0
}

fn hover(m: f64, n: f64, d: f64, rho: f64, fm: f64, eta: f64) -> Hover {
    let area = disk_area(n, d);
    let t = m * G0;
    let ideal = pow(t, 1.5) / sqrt(2.0 * rho * area);
    Hover {
        area,
        ideal,
        electrical: ideal / (fm * eta),
    }
}

struct HoverIn {
    m: f64,
    n: f64,
    d: f64,
    rho: f64,
    fm: f64,
    eta: f64,
}

fn read_hover(ctx: &mut Ctx) -> Result<HoverIn, ToolError> {
    let m = positive(ctx, "mass", "Mass")?;
    let n = ctx.number("rotors")?.expect("required");
    if n.fract() != 0.0 {
        return Err(ToolError::invalid(
            "/rotors",
            "Rotors must be a whole number.",
        ));
    }
    let d = positive(ctx, "rotor_diameter", "Rotor diameter")?;
    let rho = read_density(ctx)?;
    let fm = ctx.number("figure_of_merit")?.unwrap_or(0.6);
    let eta = ctx.number("efficiency")?.unwrap_or(0.85);
    Ok(HoverIn {
        m,
        n,
        d,
        rho,
        fm,
        eta,
    })
}

pub static DISK_AREA: ToolDef = ToolDef {
    id: "drone.power.disk-area",
    stability: gp_base::tool::Stability::Stable,
    title: "Rotor disk area and disk loading",
    summary: "The total area swept by a multirotor's propellers, the area of one, and the disk loading when the takeoff mass is given.",
    aliases: &[
        "rotor disk area calculator",
        "disk loading calculator",
        "propeller swept area",
    ],
    keywords: &[
        "disk area",
        "disk loading",
        "rotor",
        "propeller",
        "swept area",
        "multirotor",
        "momentum theory",
    ],
    inputs: &[
        HOVER_INPUTS[1],
        HOVER_INPUTS[2],
        qty(
            "mass",
            "Takeoff mass",
            "For the disk loading, like 1.4 kg",
            QT::Mass,
            "kg",
        )
        .core(),
    ],
    outputs: &[
        HOVER_POWER.outputs[2],
        Field::new(
            "area_per_rotor",
            "Area of one rotor",
            "π × diameter² / 4",
            Kind::Quantity {
                q: QT::Area,
                unit: "m2",
            },
        )
        .precision(Precision::Significant(4)),
        Field::new(
            "disk_loading",
            "Disk loading",
            "Weight ÷ total disk area",
            Kind::Quantity {
                q: QT::Pressure,
                unit: "Pa",
            },
        )
        .precision(Precision::Significant(4))
        .optional(),
    ],
    errors: &[],
    warnings: &["EXPERIMENTAL_TOOL"],
    model: "Disk area A = N × π × D² / 4 for N rotors of diameter D; disk loading = m × g₀ / A, with g₀ = 9.80665 m/s² (Leishman 2006, momentum theory)",
    accuracy: "Exact for the geometry; overlapping or ducted rotors are not allowed for",
    when_to_use: "Use this when sizing propellers or comparing airframes. Enter the number of rotors and the propeller diameter to get the total swept area that momentum theory uses for hover power. Add the takeoff mass to get the disk loading, the weight carried by each square meter of disk, which sets how hard the rotors must work and how strong the downwash is.",
    limitations: "The area is that of separate, unobstructed disks. Rotors that overlap, sit in ducts, or are stacked as coaxial pairs do not act on the full area counted here, so use the hover power tool's coaxial setting for those. The diameter is the propeller's own, tip to tip. Disk loading says nothing about blade design or efficiency by itself.",
    references: HOVER_POWER.references,
    examples: &[Example {
        id: "primary",
        title: "Six 15 in rotors carrying 6.2 kg",
        input: r#"{"rotors":6,"rotor_diameter":"15 in","mass":"6.2 kg"}"#,
        source: "Geometry: six 0.381 m disks are 0.684 m², carrying 60.8 N at 88.9 Pa",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "drone.power.hover-power",
            reason: "next",
        },
        Related {
            id: "drone.power.max-payload",
            reason: "next",
        },
        Related {
            id: "drone.power.calibrate-hover",
            reason: "alternative",
        },
    ],
    assumptions: &[HOVER_POWER.assumptions[2]],
    sentence: "The rotors sweep {disk_area} in all.",
    limits: &[("batchRows", 10_000)],
    run: run_disk_area,
    ..ToolDef::BLANK
};

fn run_disk_area(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let n = ctx.number("rotors")?.expect("required");
    if n.fract() != 0.0 {
        return Err(ToolError::invalid(
            "/rotors",
            "Rotors must be a whole number.",
        ));
    }
    let d = positive(ctx, "rotor_diameter", "Rotor diameter")?;
    let area = disk_area(n, d);
    let m2 = |v: f64| q(v, QT::Area, "m2");
    let mut out = vec![
        ("disk_area", ctx.out("disk_area", m2(area))),
        ("area_per_rotor", ctx.out("area_per_rotor", m2(area / n))),
    ];
    if ctx.is_set("mass") {
        let m = positive(ctx, "mass", "Mass")?;
        out.push((
            "disk_loading",
            ctx.out("disk_loading", q(m * G0 / area, QT::Pressure, "Pa")),
        ));
    }
    Ok(Json::obj(out))
}

pub static HOVER_POWER: ToolDef = ToolDef {
    id: "drone.power.hover-power",
    version: "1.1.0",
    stability: gp_base::tool::Stability::Stable,
    title: "Multirotor hover power",
    summary: "Electrical hover power for a multirotor from its mass, rotors, and air density, by momentum theory with a figure of merit and motor efficiency, never the ideal power alone.",
    aliases: &[
        "drone hover power calculator",
        "multirotor power calculator",
        "momentum theory hover",
    ],
    keywords: &[
        "hover power",
        "momentum theory",
        "disk loading",
        "figure of merit",
        "rotor",
        "density altitude",
        "watts",
    ],
    inputs: &[
        HOVER_INPUTS[0],
        HOVER_INPUTS[1],
        HOVER_INPUTS[2],
        HOVER_INPUTS[3].core(),
        HOVER_INPUTS[4],
        DENSITY_INPUTS[0],
        DENSITY_INPUTS[1],
        DENSITY_INPUTS[2],
        qty(
            "avionics_power",
            "Avionics and payload power",
            "Like 10 W; default 0",
            QT::Power,
            "W",
        ),
        Field::new(
            "coaxial",
            "Coaxial pairs",
            "yes when the rotors are stacked in pairs, one above the other, like an X8; no (the default) otherwise",
            Kind::Choice(&["no", "yes"]),
        ),
        num(
            "interference",
            "Coaxial interference factor",
            "Induced power of a stacked pair over two separate rotors, like 1.3; default 1.2818 (Leishman and Syal)",
            1.0,
            2.0,
        ),
    ],
    outputs: &[
        out(
            "electrical_power",
            "Electrical hover power",
            "Ideal power / (FM × η) + avionics",
            QT::Power,
            "W",
            1,
        ),
        out(
            "ideal_power",
            "Ideal induced power",
            "T^1.5 / √(2ρA): a lower bound, not the draw",
            QT::Power,
            "W",
            1,
        ),
        out(
            "disk_area",
            "Total disk area",
            "Rotors × π·D²/4",
            QT::Area,
            "m2",
            4,
        ),
        out(
            "disk_loading",
            "Disk loading",
            "Thrust / disk area",
            QT::Pressure,
            "Pa",
            1,
        ),
        out(
            "air_density",
            "Air density",
            "From altitude and temperature, or density altitude",
            QT::Density,
            "kg/m3",
            4,
        ),
        num(
            "density_factor",
            "Power factor vs sea level",
            "√(ρ0/ρ)",
            0.0,
            10.0,
        )
        .precision(Precision::Decimals(3)),
        num(
            "interference_factor",
            "Coaxial interference factor",
            "Applied to the induced power, like 1.2818",
            1.0,
            2.0,
        )
        .precision(Precision::Decimals(4))
        .optional(),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &["UNIT_ASSUMED"],
    model: "Momentum theory: P_ideal = (m·g0)^1.5 / √(2ρA); electrical P = P_ideal / (FM·η) + avionics; ISA troposphere density. Rotors in coaxial pairs: the induced power is κ times that of the same rotors apart (κ = 1.2818 for a lower rotor in the fully developed slipstream of the upper one, Leishman and Syal 2008) while the profile share the figure of merit carries is unchanged, so P = P_ideal·(κ + 1/FM − 1)/η + avionics",
    accuracy: "A first estimate. Real power depends on rotor design, frame drag, and wind; calibrate FM·η from a test flight when you can.",
    when_to_use: "Use this when you design or load a multirotor and want to know how many watts it needs just to stay in the air. Give the takeoff mass, the number of rotors, and their diameter, with the site altitude and temperature if you fly high or hot, and it gives the electrical hover power, the ideal minimum, the disk loading, and the air density. That power then feeds the flight time tools.",
    limitations: "It is a first estimate from momentum theory, with a figure of merit of 0.6 and an efficiency of 0.85 unless you set them. Real power depends on the rotor design, frame drag, and wind, and forward flight draws a different amount. Rotors count as separate disks, or as stacked pairs if you say so. A test flight run through the calibration tool gives a better figure for your drone.",
    references: &[LEISHMAN, LEISHMAN_FLIGHT_VEHICLES, ICAO_ATM, LEISHMAN_SYAL],
    examples: &[Example {
        id: "primary",
        title: "A 1.4 kg quadcopter with 9.4 in rotors at sea level",
        input: r#"{"mass":"1.4 kg","rotors":4,"rotor_diameter":"9.4 in"}"#,
        source: "add-drone-suite hover scenario: disk area 0.1791 m², ideal 76.8 W, electrical 150.6 W",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[("value", "electrical_power")],
    }],
    related: &[
        Related {
            id: "drone.power.endurance",
            reason: "next",
        },
        Related {
            id: "drone.power.max-payload",
            reason: "next",
        },
        Related {
            id: "drone.power.calibrate-hover",
            reason: "alternative",
        },
    ],
    assumptions: &[
        Assumption {
            name: "Sea-level pressure P0",
            value: "101325",
            unit: "Pa",
            source: "icao-7488",
        },
        Assumption {
            name: "Sea-level temperature T0",
            value: "288.15",
            unit: "K",
            source: "icao-7488",
        },
        Assumption {
            name: "Standard gravity g0",
            value: "9.80665",
            unit: "m/s2",
            source: "icao-7488",
        },
        Assumption {
            name: "Gas constant for dry air R",
            value: "287.05287",
            unit: "J/(kg K)",
            source: "icao-7488",
        },
        Assumption {
            name: "Tropospheric lapse rate",
            value: "-0.0065",
            unit: "K/m",
            source: "icao-7488",
        },
        Assumption {
            name: "Default induced-power interference factor for a coaxial pair",
            value: "1.2818",
            unit: "1",
            source: "leishman-syal-2008",
        },
    ],
    sentence: "Hovering takes about {electrical_power} from the battery. The ideal minimum is {ideal_power}.",
    limits: &[("batchRows", 10_000)],
    run: run_hover,
    ..ToolDef::BLANK
};

fn run_hover(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let h = read_hover(ctx)?;
    let av = ctx.quantity("avionics_power")?.map_or(0.0, |x| x.base());
    if av < 0.0 {
        return Err(ToolError::invalid(
            "/avionics_power",
            "Avionics power cannot be negative.",
        ));
    }
    let r = hover(h.m, h.n, h.d, h.rho, h.fm, h.eta);
    let kappa = if ctx.choice("coaxial")? == Some("yes") {
        if h.n % 2.0 != 0.0 {
            return Err(ToolError::invalid(
                "/rotors",
                "Coaxial rotors come in pairs, so their number is even.",
            ));
        }
        Some(ctx.number("interference")?.unwrap_or(1.2818))
    } else {
        if ctx.is_set("interference") {
            return Err(ToolError::invalid(
                "/interference",
                "The interference factor applies to coaxial pairs: set coaxial to yes, or leave it out.",
            ));
        }
        None
    };
    // Only the induced share grows: the figure of merit's profile share,
    // P_ideal (1/FM − 1), stays as it was.
    let electrical = kappa.map_or(r.electrical, |k| r.ideal * (k + 1.0 / h.fm - 1.0) / h.eta);
    let mut out = vec![
        (
            "electrical_power",
            ctx.out("electrical_power", q(electrical + av, QT::Power, "W")),
        ),
        (
            "ideal_power",
            ctx.out("ideal_power", q(r.ideal, QT::Power, "W")),
        ),
        ("disk_area", ctx.out("disk_area", q(r.area, QT::Area, "m2"))),
        (
            "disk_loading",
            ctx.out("disk_loading", q(h.m * G0 / r.area, QT::Pressure, "Pa")),
        ),
        (
            "air_density",
            ctx.out("air_density", q(h.rho, QT::Density, "kg/m3")),
        ),
        ("density_factor", Json::Num(sqrt(RHO0 / h.rho))),
    ];
    if let Some(k) = kappa {
        out.push(("interference_factor", Json::Num(k)));
    }
    Ok(Json::obj(out))
}

// ---------------------------------------------------------------- endurance

/// The energy a pack may use and what is left after the reserve, as the
/// endurance tool computes them: (usable, flyable) = (E × usable share ×
/// (1 − derating), usable × (1 − reserve)). Shares are fractions.
pub fn usable_energy(e: f64, usable: f64, derate: f64, reserve: f64) -> (f64, f64) {
    let avail = e * usable * (1.0 - derate);
    (avail, avail * (1.0 - reserve))
}

/// Heuristic cold derating: 0 at or above 20 °C, 20% at 0 °C, linear, capped at 50%.
pub fn cold_derating(t_c: f64) -> f64 {
    if t_c >= 20.0 {
        0.0
    } else {
        ((20.0 - t_c) * 0.01).min(0.5)
    }
}

pub static ENDURANCE: ToolDef = ToolDef {
    id: "drone.power.endurance",
    version: "1.1.0",
    stability: gp_base::tool::Stability::Stable,
    title: "Drone flight time",
    summary: "Hover and cruise flight time from usable battery energy and power draw, with the landing reserve and cold-battery derating shown, and range at a groundspeed or at an airspeed in a wind.",
    aliases: &[
        "drone flight time calculator",
        "drone endurance calculator",
        "multirotor flight time",
    ],
    keywords: &[
        "flight time",
        "endurance",
        "battery",
        "Wh",
        "reserve",
        "cold battery",
        "range",
    ],
    inputs: &[
        qty("energy", "Battery energy", "Like 90.4 Wh", QT::Energy, "Wh")
            .required()
            .core(),
        num(
            "usable",
            "Usable share (%)",
            "Depth-of-discharge limit, default 100",
            1.0,
            100.0,
        )
        .core(),
        qty("power", "Hover power", "Like 150.6 W", QT::Power, "W")
            .required()
            .core(),
        qty(
            "cruise_power",
            "Cruise power",
            "Optional, from a test flight, like 130 W",
            QT::Power,
            "W",
        ),
        num(
            "reserve",
            "Landing reserve (%)",
            "Share of the usable energy kept for landing, like 20; default 0",
            0.0,
            99.0,
        )
        .core(),
        qty(
            "battery_temperature",
            "Battery temperature",
            "Like 0 degC; applies the labeled cold derating",
            QT::Temperature,
            "degC",
        ),
        num(
            "derating",
            "Derating (%)",
            "Your own derating in percent, like 15; overrides the temperature heuristic",
            0.0,
            95.0,
        ),
        qty(
            "groundspeed",
            "Cruise groundspeed",
            "Like 10 m/s, for range",
            QT::Speed,
            "m/s",
        ),
        qty(
            "airspeed",
            "Cruise airspeed",
            "Instead of a groundspeed, with the wind below, like 12 m/s",
            QT::Speed,
            "m/s",
        ),
        qty(
            "course",
            "Course",
            "The direction flown over the ground, true, like 270 deg",
            QT::Angle,
            "deg",
        ),
        qty(
            "wind_direction",
            "Wind from",
            "True, like 300 deg",
            QT::Angle,
            "deg",
        ),
        qty("wind_speed", "Wind speed", "Like 6 m/s", QT::Speed, "m/s"),
        num(
            "peukert",
            "Peukert exponent",
            "Off unless entered, like 1.05. A weak model for lithium packs: use it only with your own discharge data",
            1.0,
            1.5,
        ),
        qty(
            "rated_time",
            "Rated discharge time",
            "The time the pack's rated capacity assumes, like 1 h (the default)",
            QT::Time,
            "h",
        ),
    ],
    outputs: &[
        out(
            "hover_time",
            "Hover time to the reserve",
            "Usable energy after derating and reserve / hover power",
            QT::Time,
            "min",
            1,
        ),
        out(
            "hover_time_no_reserve",
            "Hover time, no reserve",
            "Usable energy after derating / hover power",
            QT::Time,
            "min",
            1,
        ),
        out(
            "cruise_time",
            "Cruise time to the reserve",
            "Using cruise power",
            QT::Time,
            "min",
            1,
        )
        .optional(),
        out(
            "groundspeed_used",
            "Groundspeed",
            "From the wind triangle, when an airspeed and wind are given",
            QT::Speed,
            "m/s",
            1,
        )
        .optional(),
        out(
            "range",
            "Range to the reserve",
            "Cruise (or hover) time × groundspeed",
            QT::Distance,
            "km",
            2,
        )
        .optional(),
        out(
            "usable_energy",
            "Energy used",
            "After the usable share and derating",
            QT::Energy,
            "Wh",
            1,
        ),
        out(
            "reserve_energy",
            "Reserve kept",
            "Reserve share of the usable energy",
            QT::Energy,
            "Wh",
            1,
        ),
        num(
            "derating_applied",
            "Derating applied (%)",
            "Cold or user derating",
            0.0,
            100.0,
        )
        .precision(Precision::Decimals(0)),
        num(
            "peukert_factor",
            "Peukert factor at hover",
            "(rated power / hover power)^(k − 1); only when an exponent is entered",
            0.0,
            100.0,
        )
        .precision(Precision::Decimals(3))
        .optional(),
    ],
    errors: &[ErrorCode::NoSolution],
    warnings: &["HEURISTIC_DERATING", "HEURISTIC_PEUKERT", "UNIT_ASSUMED"],
    model: "Time = energy × usable share × (1 − derating) × (1 − reserve) / power; heuristic cold derating 0% at 20 °C or warmer, rising 1% per °C (20% at 0 °C), capped at 50%. Peukert, off by default: energy × (rated power / power)^(k − 1), where rated power = pack energy / rated discharge time. Range = time × groundspeed; from an airspeed, the groundspeed along the course by the wind triangle (the same code as the aviation wind tools): heading correction asin(w sin(θ_w − course) / V), groundspeed V cos(correction) − w cos(θ_w − course)",
    accuracy: "Only as good as the power figure. Wind, climbs, and aging packs shorten real flights.",
    when_to_use: "Use this when you know, or have estimated, the power a drone draws and want to know how long a pack will keep it in the air: it takes the usable share of the battery, removes a cold-weather derating and the reserve you land with, and divides by the power, for hover and cruise, and turns the cruise time into a range at your groundspeed.",
    limitations: "It is energy divided by power at a steady draw, so it is exactly as good as the power figure you give it. Climbs, wind, gusts, and payload changes raise the draw; a pack’s voltage sags as it empties and its capacity falls with age and at high current. The reserve here is a share of the usable energy, not of the whole pack, so 80% usable with a 20% reserve flies on 64%. The cold-battery derating is a rule of thumb. Range with wind assumes the same wind and power the whole way, one way along one course; for an out-and-back trip use the return-to-home tool, since a headwind costs more on the way back than a tailwind gives on the way out.",
    references: &[LEISHMAN, BAUERSFELD, GEOPRIMS_ENDURANCE_SPEC],
    examples: &[
        Example {
            id: "primary",
            title: "A 90.4 Wh pack, 80% usable, hovering at 150.6 W",
            input: r#"{"energy":"90.4 Wh","usable":80,"power":"150.6 W"}"#,
            source: "add-drone-suite endurance scenario: about 28.8 min",
        },
        Example {
            id: "mavic-3",
            title: "A DJI Mavic 3 flying for maximum endurance",
            input: r#"{"energy":"72.372 Wh","power":"89.5 W"}"#,
            source: "Bauersfeld and Scaramuzza (2022), Sec. VII-E step 6: 4.89 Ah effective at 4 × 3.7 V and 89.5 W gives 2,909 s, about 48.5 min",
        },
    ],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[("value", "hover_time")],
    }],
    related: &[
        Related {
            id: "drone.power.hover-power",
            reason: "alternative",
        },
        Related {
            id: "drone.power.battery-energy",
            reason: "alternative",
        },
        Related {
            id: "drone.power.calibrate-hover",
            reason: "alternative",
        },
        Related {
            id: "drone.power.rth-budget",
            reason: "next",
        },
        Related {
            id: "drone.power.max-payload",
            reason: "next",
        },
    ],
    sentence: "Hover time is about {hover_time}{if reserve_energy > 0}, keeping {reserve_energy} in reserve{/if}.{warn HEURISTIC_DERATING} This includes a cold-battery estimate.{/warn}",
    limits: &[("batchRows", 10_000)],
    assumptions: &[
        Assumption {
            name: "Cold-derating warm threshold (product heuristic)",
            value: "20",
            unit: "degC",
            source: "geoprims-endurance-spec",
        },
        Assumption {
            name: "Cold derating at 0 °C (product heuristic)",
            value: "20",
            unit: "%",
            source: "geoprims-endurance-spec",
        },
        Assumption {
            name: "Cold-derating slope below 20 °C (product heuristic)",
            value: "1",
            unit: "%/degC",
            source: "geoprims-endurance-spec",
        },
        Assumption {
            name: "Maximum automatic cold derating (product heuristic)",
            value: "50",
            unit: "%",
            source: "geoprims-endurance-spec",
        },
        Assumption {
            name: "Default rated discharge time when Peukert is enabled",
            value: "1",
            unit: "h",
            source: "geoprims-endurance-spec",
        },
    ],
    run: run_endurance,
    ..ToolDef::BLANK
};

/// Groundspeed along the course from an airspeed and a wind, by the wind
/// triangle the aviation tools use.
fn wind_groundspeed(ctx: &mut Ctx, v: f64) -> Result<f64, ToolError> {
    let w = ctx.quantity("wind_speed")?.map_or(0.0, |x| x.base());
    if v <= 0.0 || w < 0.0 {
        return Err(ToolError::invalid(
            "/airspeed",
            "The airspeed must be positive and the wind speed not negative.",
        ));
    }
    if w == 0.0 {
        return Ok(v);
    }
    let deg = units::by_symbol(QT::Angle, "deg").expect("deg");
    let (Some(course), Some(wd)) = (ctx.quantity("course")?, ctx.quantity("wind_direction")?)
    else {
        return Err(ToolError::invalid(
            "/course",
            "With a wind, give the course and the direction the wind blows from.",
        ));
    };
    match gp_geo::wind::heading_groundspeed(course.to(deg), v, wd.to(deg), w) {
        Some((_, gs)) if gs > 0.0 => Ok(gs),
        Some(_) => Err(ToolError::new(
            ErrorCode::NoSolution,
            "The headwind along this course is at least the airspeed: the drone makes no progress.",
        )
        .at("/wind_speed")),
        None => Err(ToolError::new(
            ErrorCode::NoSolution,
            "The crosswind is stronger than the airspeed: the drone cannot hold this course.",
        )
        .at("/wind_speed")),
    }
}

/// The derating fraction: the one entered, or the cold rule of thumb from the
/// battery temperature (with its warning), or none.
fn read_derating(ctx: &mut Ctx) -> Result<f64, ToolError> {
    Ok(
        match (
            ctx.number("derating")?,
            ctx.quantity("battery_temperature")?,
        ) {
            (Some(d), _) => d / 100.0,
            (None, Some(t)) => cold_derating_warned(ctx, t.base() - 273.15),
            (None, None) => 0.0,
        },
    )
}

/// The cold rule of thumb at `t_c` °C, warning that it is one when it applies.
fn cold_derating_warned(ctx: &mut Ctx, t_c: f64) -> f64 {
    let d = cold_derating(t_c);
    if d > 0.0 {
        ctx.warnings.push(
            Warning::new(
                "HEURISTIC_DERATING",
                format!(
                    "Cold battery: usable energy reduced by {}% (a rule of thumb: 0% at 20 °C, 20% at 0 °C). Enter your own derating if you have test data.",
                    display::number(d * 100.0, Precision::Decimals(0), ctx.options.format)
                ),
            )
            .at("/battery_temperature"),
        );
    }
    d
}

const WH: Kind = Kind::Quantity {
    q: QT::Energy,
    unit: "Wh",
};

pub static USABLE_ENERGY: ToolDef = ToolDef {
    id: "drone.power.usable-energy",
    title: "Usable battery energy",
    summary: "The battery energy left to fly on after a usable share, a cold derating, and a landing reserve are taken off the pack's rated energy.",
    aliases: &[
        "usable battery energy",
        "battery reserve calculator",
        "flyable watt hours",
    ],
    keywords: &[
        "usable energy",
        "reserve",
        "depth of discharge",
        "watt hours",
        "cold battery",
        "derating",
        "landing reserve",
    ],
    inputs: &[
        ENDURANCE.inputs[0],
        ENDURANCE.inputs[1].core(),
        ENDURANCE.inputs[4].core(),
        ENDURANCE.inputs[5],
        ENDURANCE.inputs[6],
    ],
    outputs: &[
        Field::new(
            "flyable_energy",
            "Energy to fly on",
            "Usable energy less the reserve",
            WH,
        )
        .precision(Precision::Decimals(1)),
        ENDURANCE.outputs[5],
        ENDURANCE.outputs[6],
        ENDURANCE.outputs[7],
    ],
    errors: &[],
    warnings: &["HEURISTIC_DERATING", "EXPERIMENTAL_TOOL"],
    model: "Usable = rated energy × usable share × (1 − derating); energy to fly on = usable × (1 − reserve). Cold derating, when a battery temperature is given instead of a figure: 0% at or above 20 °C, 20% at 0 °C, linear, capped at 50%",
    accuracy: "Exact arithmetic on the shares entered; the cold derating is a rule of thumb, flagged when used",
    when_to_use: "Use this to budget a battery before working out a flight time. Enter the pack's rated energy in watt-hours, the share you are willing to discharge, and the reserve you want left when you land. Add the battery temperature for a cold day, or your own derating if you have test data. The tool returns the energy you can plan to fly on, the usable energy before the reserve, and the reserve itself.",
    limitations: "The shares are your policy, not a property of the pack, so use the maker's discharge limit and your own landing rule. The cold derating is a rule of thumb that real packs depart from, and a pack that is aged, puffed, or drawn hard gives less than its label. It does not model voltage sag, which can end a flight before the energy is gone.",
    references: ENDURANCE.references,
    examples: &[Example {
        id: "primary",
        title: "A 99 Wh pack, 85% usable, 25% reserve",
        input: r#"{"energy":"99 Wh","usable":85,"reserve":25}"#,
        source: "Arithmetic: 99 × 0.85 is 84.15 Wh usable, of which 21.04 Wh is held back, leaving 63.11 Wh",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "drone.power.endurance",
            reason: "next",
        },
        Related {
            id: "drone.power.battery-energy",
            reason: "parent",
        },
        Related {
            id: "drone.power.temperature-derating",
            reason: "alternative",
        },
    ],
    sentence: "You can plan to fly on {flyable_energy}, with {reserve_energy} held in reserve.",
    limits: &[("batchRows", 10_000)],
    run: run_usable_energy,
    ..ToolDef::BLANK
};

fn run_usable_energy(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let e = positive(ctx, "energy", "Battery energy")?;
    let usable = pct(ctx, "usable", 1.0)?;
    let reserve = pct(ctx, "reserve", 0.0)?;
    let derate = read_derating(ctx)?;
    let (avail, flyable) = usable_energy(e, usable, derate, reserve);
    let wh = |v: f64| q(v, QT::Energy, "J");
    Ok(Json::obj(vec![
        ("flyable_energy", ctx.out("flyable_energy", wh(flyable))),
        ("usable_energy", ctx.out("usable_energy", wh(avail))),
        (
            "reserve_energy",
            ctx.out("reserve_energy", wh(avail * reserve)),
        ),
        ("derating_applied", Json::Num(derate * 100.0)),
    ]))
}

pub static TEMPERATURE_DERATING: ToolDef = ToolDef {
    id: "drone.power.temperature-derating",
    title: "Cold battery derating",
    summary: "A rule-of-thumb reduction in usable battery energy for a cold pack, from the battery temperature, with the share and the energy that remain.",
    aliases: &[
        "cold battery derating",
        "battery capacity in cold weather",
        "winter drone battery",
    ],
    keywords: &[
        "cold",
        "temperature",
        "derating",
        "winter",
        "battery capacity",
        "lithium",
        "rule of thumb",
    ],
    inputs: &[
        ENDURANCE.inputs[5].required().core(),
        Field::new(
            "energy",
            "Battery energy",
            "To get what is left of it, like 99 Wh",
            WH,
        ),
    ],
    outputs: &[
        ENDURANCE.outputs[7],
        Field::new(
            "share_remaining",
            "Share remaining",
            "100% less the derating",
            Kind::Number {
                min: 0.0,
                max: 100.0,
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "energy_remaining",
            "Energy remaining",
            "The pack's energy after the derating",
            WH,
        )
        .precision(Precision::Decimals(1))
        .optional(),
    ],
    errors: &[],
    warnings: &["HEURISTIC_DERATING", "EXPERIMENTAL_TOOL"],
    model: "A rule of thumb, not a battery model: derating = 0% at or above 20 °C, rising 1 point per degree below it to 20% at 0 °C, capped at 50%",
    accuracy: "A planning heuristic; real packs vary with chemistry, age, load, and how warm they were at takeoff",
    when_to_use: "Use this for a first allowance when flying in the cold and you have no test data for your packs. Enter the battery's own temperature, not the air's, since a pack kept warm until takeoff behaves better than the weather suggests. Add the pack's energy to see what is left. The result is the reduction to plan for, and it carries a warning that it is a rule of thumb.",
    limitations: "This is a straight-line rule of thumb, not a measurement or a model of any pack. Lithium packs lose capacity and sag in voltage in the cold by amounts that depend on chemistry, age, discharge rate, and self-heating in flight, and some aircraft refuse to take off below a set temperature. Use your maker's guidance or your own logged flights when you have them.",
    references: ENDURANCE.references,
    examples: &[Example {
        id: "primary",
        title: "A 99 Wh pack at 5 °C",
        input: r#"{"battery_temperature":"5 degC","energy":"99 Wh"}"#,
        source: "The rule of thumb: 15 degrees under 20 °C is 15%, leaving 84.15 Wh",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "drone.power.endurance",
            reason: "next",
        },
        Related {
            id: "drone.power.usable-energy",
            reason: "next",
        },
        Related {
            id: "drone.power.battery-energy",
            reason: "parent",
        },
    ],
    sentence: "Plan for {derating_applied}% less battery energy.",
    limits: &[("batchRows", 10_000)],
    run: run_temperature_derating,
    ..ToolDef::BLANK
};

fn run_temperature_derating(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let t_c = ctx.req_quantity("battery_temperature")?.base() - 273.15;
    let d = cold_derating_warned(ctx, t_c);
    let mut out = vec![
        ("derating_applied", Json::Num(d * 100.0)),
        ("share_remaining", Json::Num((1.0 - d) * 100.0)),
    ];
    if ctx.is_set("energy") {
        let e = positive(ctx, "energy", "Battery energy")?;
        out.push((
            "energy_remaining",
            ctx.out("energy_remaining", q(e * (1.0 - d), QT::Energy, "J")),
        ));
    }
    Ok(Json::obj(out))
}

pub static RANGE: ToolDef = ToolDef {
    id: "drone.power.range",
    title: "Drone range",
    summary: "How far a drone can fly on a battery: the flight time at its power draw times the groundspeed, with usable share, reserve, cold derating, and wind along the course.",
    aliases: &[
        "drone range calculator",
        "how far can a drone fly",
        "flight distance on a battery",
    ],
    keywords: &[
        "range",
        "distance",
        "battery",
        "groundspeed",
        "headwind",
        "cruise power",
        "reserve",
    ],
    inputs: &[
        ENDURANCE.inputs[0],
        Field::new(
            "power",
            "Power in forward flight",
            "Average electrical draw at the planned speed, like 180 W",
            Kind::Quantity {
                q: QT::Power,
                unit: "W",
            },
        )
        .required()
        .core(),
        ENDURANCE.inputs[7].core(),
        ENDURANCE.inputs[1],
        ENDURANCE.inputs[4],
        ENDURANCE.inputs[8],
        ENDURANCE.inputs[9],
        ENDURANCE.inputs[10],
        ENDURANCE.inputs[11],
        ENDURANCE.inputs[5],
        ENDURANCE.inputs[6],
    ],
    outputs: &[
        ENDURANCE.outputs[4],
        Field::new(
            "flight_time",
            "Flight time",
            "Energy to fly on ÷ power",
            Kind::Quantity {
                q: QT::Time,
                unit: "min",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "groundspeed_used",
            "Groundspeed",
            "As entered, or from the airspeed and wind",
            Kind::Quantity {
                q: QT::Speed,
                unit: "m/s",
            },
        )
        .precision(Precision::Decimals(1)),
        ENDURANCE.outputs[5],
    ],
    errors: &[ErrorCode::NoSolution],
    warnings: &["HEURISTIC_DERATING", "EXPERIMENTAL_TOOL"],
    model: "Range = flight time × groundspeed, with flight time = rated energy × usable share × (1 − derating) × (1 − reserve) ÷ power; groundspeed from the airspeed and wind by the wind triangle when not entered",
    accuracy: "As good as the power figure; the arithmetic is exact",
    when_to_use: "Use this to check whether a battery covers a one-way distance, such as a transit to a site or a linear inspection. Enter the pack's energy, the average power in forward flight from a logged flight or the maker's figures, and the groundspeed. Or give the airspeed, the course, and the wind, and the tool finds the groundspeed. Set a usable share and a reserve so the answer leaves the margin you plan to land with.",
    limitations: "This is distance flown in one direction at a steady speed and power. An out-and-back flight in wind does not simply halve it, since the upwind leg takes longer; the return-to-home budget tool handles that. Power in forward flight differs from hover power and changes with speed, payload, and air density. Climb, descent, and hovering on task are not included.",
    references: ENDURANCE.references,
    examples: &[Example {
        id: "primary",
        title: "A 99 Wh pack at 180 W and 14 m/s, 85% usable, 25% reserve",
        input: r#"{"energy":"99 Wh","power":"180 W","groundspeed":"14 m/s","usable":85,"reserve":25}"#,
        source: "Arithmetic: 63.11 Wh at 180 W is 21.04 min, and 17.67 km at 14 m/s",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "drone.power.endurance",
            reason: "parent",
        },
        Related {
            id: "drone.power.rth-budget",
            reason: "alternative",
        },
        Related {
            id: "drone.power.usable-energy",
            reason: "parent",
        },
    ],
    assumptions: &[
        ENDURANCE.assumptions[0],
        ENDURANCE.assumptions[1],
        ENDURANCE.assumptions[2],
        ENDURANCE.assumptions[3],
    ],
    sentence: "The drone can fly about {range} in {flight_time}.",
    limits: &[("batchRows", 10_000)],
    run: run_range,
    ..ToolDef::BLANK
};

fn run_range(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let e = positive(ctx, "energy", "Battery energy")?;
    let p = positive(ctx, "power", "Power")?;
    let usable = pct(ctx, "usable", 1.0)?;
    let reserve = pct(ctx, "reserve", 0.0)?;
    let derate = read_derating(ctx)?;
    let (avail, flyable) = usable_energy(e, usable, derate, reserve);
    let gs = match (ctx.quantity("groundspeed")?, ctx.quantity("airspeed")?) {
        (Some(_), Some(_)) => {
            return Err(ToolError::invalid(
                "/airspeed",
                "Give a groundspeed, or an airspeed with the wind, not both.",
            ));
        }
        (Some(g), None) if g.base() <= 0.0 => {
            return Err(ToolError::invalid(
                "/groundspeed",
                "The groundspeed must be positive.",
            ));
        }
        (Some(g), None) => g.base(),
        (None, Some(v)) => wind_groundspeed(ctx, v.base())?,
        (None, None) => {
            return Err(ToolError::invalid(
                "/groundspeed",
                "Give the groundspeed, or the airspeed with the wind.",
            ));
        }
    };
    let t = flyable / p;
    Ok(Json::obj(vec![
        ("range", ctx.out("range", q(t * gs, QT::Distance, "m"))),
        ("flight_time", ctx.out("flight_time", q(t, QT::Time, "s"))),
        (
            "groundspeed_used",
            ctx.out("groundspeed_used", q(gs, QT::Speed, "m/s")),
        ),
        (
            "usable_energy",
            ctx.out("usable_energy", q(avail, QT::Energy, "J")),
        ),
    ]))
}

fn run_endurance(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let e = positive(ctx, "energy", "Battery energy")?;
    let usable = pct(ctx, "usable", 1.0)?;
    let p = positive(ctx, "power", "Hover power")?;
    let reserve = pct(ctx, "reserve", 0.0)?;
    let derate = read_derating(ctx)?;
    let (avail, flyable) = usable_energy(e, usable, derate, reserve);
    let k = ctx.number("peukert")?;
    let rated_time = match ctx.quantity("rated_time")? {
        Some(_) if k.is_none() => {
            return Err(ToolError::invalid(
                "/rated_time",
                "The rated discharge time only matters with a Peukert exponent; enter one or leave this out.",
            ));
        }
        Some(t) if t.base() <= 0.0 => {
            return Err(ToolError::invalid(
                "/rated_time",
                "The rated discharge time must be positive.",
            ));
        }
        Some(t) => t.base(),
        None => 3600.0,
    };
    // Effective-energy factor at a steady draw: 1 when Peukert is off.
    let peukert = |pw: f64| k.map_or(1.0, |k| pow(e / rated_time / pw, k - 1.0));
    if k.is_some_and(|k| k > 1.0) {
        ctx.warnings.push(
            Warning::new(
                "HEURISTIC_PEUKERT",
                "Peukert's law was fitted to lead-acid cells and describes lithium packs poorly; the times use it only because you entered an exponent.",
            )
            .at("/peukert"),
        );
    }
    let wh = |v: f64| q(v, QT::Energy, "J");
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| gp_base::display::number(x, Precision::Decimals(d), fmt);
        let mins = |j: f64| n(j / p / 60.0, 1);
        ctx.step(
            "Energy you may use",
            "usable = pack energy × usable share × (1 − cold derating)",
            format!(
                "{} Wh × {}% × (1 − {}%)",
                n(e / 3600.0, 1),
                n(usable * 100.0, 0),
                n(derate * 100.0, 0)
            ),
            format!("{} Wh", n(avail / 3600.0, 1)),
        );
        ctx.step(
            "After the reserve",
            "flyable = usable × (1 − reserve)",
            format!(
                "{} Wh × (1 − {}%)",
                n(avail / 3600.0, 1),
                n(reserve * 100.0, 0)
            ),
            format!("{} Wh", n(flyable / 3600.0, 1)),
        );
        if let Some(k) = k {
            ctx.step(
                "Peukert factor",
                "(pack energy / rated time / hover power)^(k − 1)",
                format!(
                    "({} W / {} W)^{}",
                    n(e / rated_time, 1),
                    n(p, 1),
                    n(k - 1.0, 3)
                ),
                n(peukert(p), 3),
            );
        }
        ctx.step(
            "Hover time",
            "time = flyable energy × Peukert factor / hover power",
            format!(
                "{} Wh × {} / {} W",
                n(flyable / 3600.0, 1),
                n(peukert(p), 3),
                n(p, 0)
            ),
            format!("{} min", mins(flyable * peukert(p))),
        );
    }
    let mut o = vec![
        (
            "hover_time",
            ctx.out("hover_time", q(flyable * peukert(p) / p, QT::Time, "s")),
        ),
        (
            "hover_time_no_reserve",
            ctx.out(
                "hover_time_no_reserve",
                q(avail * peukert(p) / p, QT::Time, "s"),
            ),
        ),
    ];
    let cruise = ctx.quantity("cruise_power")?.map(|x| x.base());
    if let Some(cp) = cruise {
        if cp <= 0.0 {
            return Err(ToolError::invalid(
                "/cruise_power",
                "Cruise power must be positive.",
            ));
        }
        o.push((
            "cruise_time",
            ctx.out("cruise_time", q(flyable * peukert(cp) / cp, QT::Time, "s")),
        ));
    }
    let gs = match (ctx.quantity("groundspeed")?, ctx.quantity("airspeed")?) {
        (Some(_), Some(_)) => {
            return Err(ToolError::invalid(
                "/airspeed",
                "Give a groundspeed, or an airspeed with the wind, not both.",
            ));
        }
        (Some(g), None) => Some(g.base()),
        (None, Some(v)) => {
            let gs = wind_groundspeed(ctx, v.base())?;
            o.push((
                "groundspeed_used",
                ctx.out("groundspeed_used", q(gs, QT::Speed, "m/s")),
            ));
            Some(gs)
        }
        (None, None) => None,
    };
    if let Some(gs) = gs {
        let pw = cruise.unwrap_or(p);
        let t = flyable * peukert(pw) / pw;
        o.push(("range", ctx.out("range", q(t * gs, QT::Distance, "m"))));
    }
    o.push(("usable_energy", ctx.out("usable_energy", wh(avail))));
    o.push((
        "reserve_energy",
        ctx.out("reserve_energy", wh(avail * reserve)),
    ));
    o.push(("derating_applied", Json::Num(derate * 100.0)));
    if k.is_some() {
        o.push(("peukert_factor", Json::Num(peukert(p))));
    }
    Ok(Json::obj(o))
}

// ---------------------------------------------------------------- max payload

pub static MAX_PAYLOAD: ToolDef = ToolDef {
    id: "drone.power.max-payload",
    version: "1.1.0",
    stability: gp_base::tool::Stability::Stable,
    title: "Maximum payload for a flight time",
    summary: "The most payload a multirotor can carry and still hover for a target time, from its mass, rotors, battery energy, and air density by momentum theory.",
    aliases: &["drone payload calculator", "max payload for flight time"],
    keywords: &[
        "payload",
        "flight time",
        "hover",
        "endurance",
        "mass",
        "momentum theory",
    ],
    inputs: &[
        HOVER_INPUTS[0],
        HOVER_INPUTS[1],
        HOVER_INPUTS[2],
        HOVER_INPUTS[3],
        HOVER_INPUTS[4],
        qty(
            "usable_energy",
            "Usable energy",
            "After reserve, like 72 Wh",
            QT::Energy,
            "Wh",
        )
        .required()
        .core(),
        qty(
            "target_time",
            "Target hover time",
            "Like 20 min",
            QT::Time,
            "min",
        )
        .required(),
        qty(
            "avionics_power",
            "Avionics and payload power",
            "Like 10 W; default 0",
            QT::Power,
            "W",
        ),
        DENSITY_INPUTS[0],
        DENSITY_INPUTS[1],
        DENSITY_INPUTS[2],
    ],
    outputs: &[
        out(
            "max_payload",
            "Maximum payload",
            "Extra mass that still meets the target time",
            QT::Mass,
            "kg",
            3,
        ),
        out(
            "max_mass",
            "Maximum takeoff mass",
            "For the target time",
            QT::Mass,
            "kg",
            3,
        ),
        out(
            "base_hover_time",
            "Hover time with no payload",
            "At the entered mass",
            QT::Time,
            "min",
            1,
        ),
    ],
    errors: &[
        ErrorCode::InvalidInput,
        ErrorCode::NoSolution,
        ErrorCode::OutOfDomain,
    ],
    warnings: &["UNIT_ASSUMED"],
    model: "Solve (m·g0)^1.5 / √(2ρA) / (FM·η) + avionics = usable energy / target time for m",
    accuracy: "Momentum-theory estimate; it ignores the motors' thrust limit, so check the maximum takeoff mass in the manual.",
    when_to_use: "Use this when a job needs a camera, sensor, or delivery load and you must know how much the drone can carry and still hover for the time the job takes. Give its mass, rotors, rotor size, the usable battery energy after your reserve, and the target time, and it gives the most payload, the takeoff mass that implies, and the hover time with no payload at all.",
    limitations: "It answers by power alone. It does not know the motors' maximum thrust, the frame's strength, or the maker's maximum takeoff mass, and any of these can be the real limit, so the manual governs. It assumes a steady hover; forward flight, wind, and climbs change the draw. The usable energy is taken as given, so a cold or worn pack will fall short. Power the payload draws goes in as avionics power.",
    references: &[LEISHMAN, VAN_MULDERS, ICAO_ATM],
    examples: &[
        Example {
            id: "primary",
            title: "The 1.4 kg quad, 72 Wh usable, 20 minutes",
            input: r#"{"mass":"1.4 kg","rotors":4,"rotor_diameter":"9.4 in","usable_energy":"72 Wh","target_time":"20 min"}"#,
            source: "Momentum theory (Leishman 2006, chapter 2) solved for mass",
        },
        Example {
            id: "published-payload-fit-inverse",
            title: "Recover the 50 g payload in the published hover-power fit",
            input: r#"{"mass":"0.2 kg","rotors":4,"rotor_diameter":"0.054872655656933694 m","usable_energy":"20 Wh","target_time":"19.985598289204738 min","avionics_power":"10.6 W"}"#,
            source: "Van Mulders et al. (2023), equation 6 and figure 4a: inverse of the measured payload-hover fit from 0.20 kg to 0.25 kg",
        },
    ],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[("value", "max_payload")],
    }],
    related: &[
        Related {
            id: "drone.power.hover-power",
            reason: "parent",
        },
        Related {
            id: "drone.power.payload-impact",
            reason: "alternative",
        },
        Related {
            id: "drone.power.endurance",
            reason: "alternative",
        },
    ],
    assumptions: &[
        Assumption {
            name: "Sea-level pressure P0",
            value: "101325",
            unit: "Pa",
            source: "icao-7488",
        },
        Assumption {
            name: "Sea-level temperature T0",
            value: "288.15",
            unit: "K",
            source: "icao-7488",
        },
        Assumption {
            name: "Standard gravity g0",
            value: "9.80665",
            unit: "m/s2",
            source: "icao-7488",
        },
        Assumption {
            name: "Gas constant for dry air R",
            value: "287.05287",
            unit: "J/(kg K)",
            source: "icao-7488",
        },
        Assumption {
            name: "Tropospheric lapse rate",
            value: "-0.0065",
            unit: "K/m",
            source: "icao-7488",
        },
    ],
    sentence: "It can carry about {max_payload} more and still hover for the target time.",
    limits: &[("batchRows", 10_000)],
    run: run_max_payload,
    ..ToolDef::BLANK
};

fn run_max_payload(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let h = read_hover(ctx)?;
    let e = positive(ctx, "usable_energy", "Usable energy")?;
    let t = positive(ctx, "target_time", "Target hover time")?;
    let av = ctx.quantity("avionics_power")?.map_or(0.0, |x| x.base());
    if av < 0.0 {
        return Err(ToolError::invalid(
            "/avionics_power",
            "Avionics and payload power cannot be negative.",
        ));
    }
    let area = h.n * core::f64::consts::PI * h.d * h.d / 4.0;
    let budget = e / t - av; // W available for lift
    let base = hover(h.m, h.n, h.d, h.rho, h.fm, h.eta).electrical + av;
    // (m g)^1.5 = budget · FM · η · √(2ρA)
    let m_max = if budget > 0.0 {
        pow(budget * h.fm * h.eta * sqrt(2.0 * h.rho * area), 2.0 / 3.0) / G0
    } else {
        0.0
    };
    if m_max <= h.m {
        return Err(ToolError::new(
            ErrorCode::NoSolution,
            format!(
                "Even with no payload it hovers for only {}, short of the target.",
                display::quantity(
                    e / base / 60.0,
                    "min",
                    Precision::Decimals(1),
                    ctx.options.format
                )
            ),
        )
        .at("/target_time"));
    }
    Ok(Json::obj(vec![
        (
            "max_payload",
            ctx.out("max_payload", q(m_max - h.m, QT::Mass, "kg")),
        ),
        ("max_mass", ctx.out("max_mass", q(m_max, QT::Mass, "kg"))),
        (
            "base_hover_time",
            ctx.out("base_hover_time", q(e / base, QT::Time, "s")),
        ),
    ]))
}

// ---------------------------------------------------------------- calibration

pub static CALIBRATE_HOVER: ToolDef = ToolDef {
    id: "drone.power.calibrate-hover",
    version: "1.1.0",
    stability: gp_base::tool::Stability::Stable,
    title: "Calibrate hover power from a test flight",
    summary: "Back-solve a multirotor's real figure of merit × efficiency from a measured hover: the energy it drew and for how long, so the hover-power and flight-time tools match your aircraft.",
    aliases: &[
        "calibrate drone hover power",
        "figure of merit from test flight",
        "measured hover power",
    ],
    keywords: &[
        "calibration",
        "test flight",
        "figure of merit",
        "efficiency",
        "hover power",
        "momentum theory",
    ],
    inputs: &[
        HOVER_INPUTS[0],
        HOVER_INPUTS[1],
        HOVER_INPUTS[2],
        qty(
            "energy_used",
            "Energy used",
            "Drawn during the steady hover, from the flight log or recharge, like 40 Wh",
            QT::Energy,
            "Wh",
        )
        .required()
        .core(),
        qty(
            "hover_time",
            "Hover time",
            "How long the steady hover lasted, like 15 min",
            QT::Time,
            "min",
        )
        .required()
        .core(),
        qty(
            "avionics_power",
            "Avionics and payload power",
            "Drawn apart from the motors, like 10 W; default 0",
            QT::Power,
            "W",
        ),
        num(
            "efficiency",
            "Motor and speed-controller efficiency",
            "To split out the figure of merit, 0.5 to 1, default 0.85",
            0.5,
            1.0,
        ),
        DENSITY_INPUTS[0],
        DENSITY_INPUTS[1],
        DENSITY_INPUTS[2],
    ],
    outputs: &[
        num(
            "fm_eta",
            "Figure of merit × efficiency",
            "Ideal power / measured lift power; the defaults give 0.51",
            0.0,
            1.0,
        )
        .precision(Precision::Decimals(3)),
        num(
            "figure_of_merit",
            "Figure of merit",
            "At the entered or default efficiency",
            0.0,
            2.0,
        )
        .precision(Precision::Decimals(3)),
        out(
            "measured_power",
            "Measured hover power",
            "Energy used / hover time",
            QT::Power,
            "W",
            1,
        ),
        out(
            "ideal_power",
            "Ideal induced power",
            "T^1.5 / √(2ρA)",
            QT::Power,
            "W",
            1,
        ),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["UNIT_ASSUMED"],
    model: "Measured power = energy used / hover time; lift power = measured − avionics; FM·η = (m·g0)^1.5 / √(2ρA) / lift power; FM = FM·η / η",
    accuracy: "As good as the measurement: use a steady hover in calm air, and the energy the log or charger reports for that hover alone, not the whole flight.",
    when_to_use: "Use this after a test flight to make the hover power and flight time tools match your own drone. Give its mass, rotors, and rotor size, plus the energy it drew and how long it hovered, and it works back to the combined figure of merit and motor efficiency, and to the figure of merit alone at the efficiency you assume. Enter those in the hover power tool in place of its defaults.",
    limitations: "The answer is only as good as the test. Use a steady hover in calm air, and the energy for that hover alone, not for takeoff, climb, and landing too. The split between figure of merit and efficiency rests on the efficiency you enter, 0.85 unless you set one. It uses momentum theory, so a value found at one mass and air density may not hold exactly at another.",
    references: &[LEISHMAN, LEISHMAN_FLIGHT_VEHICLES, ICAO_ATM],
    examples: &[
        Example {
            id: "primary",
            title: "The 1.4 kg quad drew 40 Wh in a 15-minute hover",
            input: r#"{"mass":"1.4 kg","rotors":4,"rotor_diameter":"9.4 in","energy_used":"40 Wh","hover_time":"15 min"}"#,
            source: "Momentum theory (Leishman 2006, chapter 2) solved for the figure of merit",
        },
        Example {
            id: "embry-riddle",
            title: "Back-solve Embry-Riddle's 45,000 lb tiltrotor example",
            input: r#"{"mass":"45000 lb","rotors":2,"rotor_diameter":"38 ft","energy_used":"5253.8 kWh","hover_time":"1 h","efficiency":1}"#,
            source: "Embry-Riddle Introduction to Aerospace Flight Vehicles (2026), chapter 67: 5,253.8 kW at a figure of merit of 0.75",
        },
    ],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "drone.power.hover-power",
            reason: "next",
        },
        Related {
            id: "drone.power.endurance",
            reason: "next",
        },
        Related {
            id: "drone.power.battery-energy",
            reason: "next",
        },
    ],
    assumptions: &[
        Assumption {
            name: "Sea-level pressure P0",
            value: "101325",
            unit: "Pa",
            source: "icao-7488",
        },
        Assumption {
            name: "Sea-level temperature T0",
            value: "288.15",
            unit: "K",
            source: "icao-7488",
        },
        Assumption {
            name: "Standard gravity g0",
            value: "9.80665",
            unit: "m/s2",
            source: "icao-7488",
        },
        Assumption {
            name: "Gas constant for dry air R",
            value: "287.05287",
            unit: "J/(kg K)",
            source: "icao-7488",
        },
        Assumption {
            name: "Tropospheric lapse rate",
            value: "-0.0065",
            unit: "K/m",
            source: "icao-7488",
        },
    ],
    sentence: "Your flight shows a figure of merit × efficiency of {fm_eta}.",
    limits: &[("batchRows", 10_000)],
    run: run_calibrate,
    ..ToolDef::BLANK
};

fn run_calibrate(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let m = positive(ctx, "mass", "Mass")?;
    let n = ctx.number("rotors")?.expect("required");
    if n.fract() != 0.0 {
        return Err(ToolError::invalid(
            "/rotors",
            "Rotors must be a whole number.",
        ));
    }
    let d = positive(ctx, "rotor_diameter", "Rotor diameter")?;
    let e = positive(ctx, "energy_used", "Energy used")?;
    let t = positive(ctx, "hover_time", "Hover time")?;
    let av = ctx.quantity("avionics_power")?.map_or(0.0, |x| x.base());
    let eta = ctx.number("efficiency")?.unwrap_or(0.85);
    let rho = read_density(ctx)?;
    let measured = e / t;
    let lift = measured - av;
    if av < 0.0 || lift <= 0.0 {
        return Err(ToolError::invalid(
            "/avionics_power",
            "The avionics power must be at least 0 and below the measured power.",
        ));
    }
    let ideal = hover(m, n, d, rho, 1.0, 1.0).ideal;
    let fm_eta = ideal / lift;
    if fm_eta >= 1.0 {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "That hover would beat an ideal rotor, which is impossible; check the mass, the energy used, and the time.",
        )
        .at("/energy_used"));
    }
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let x = move |v: f64, dp: u8| display::number(v, Precision::Decimals(dp), fmt);
        ctx.step(
            "Measured power",
            "energy used / hover time",
            format!("{} Wh / {} min", x(e / 3600.0, 1), x(t / 60.0, 1)),
            format!("{} W", x(measured, 1)),
        );
        ctx.step(
            "Figure of merit × efficiency",
            "ideal power / (measured − avionics)",
            format!("{} W / {} W", x(ideal, 1), x(lift, 1)),
            x(fm_eta, 3),
        );
    }
    Ok(Json::obj(vec![
        ("fm_eta", Json::Num(fm_eta)),
        ("figure_of_merit", Json::Num(fm_eta / eta)),
        (
            "measured_power",
            ctx.out("measured_power", q(measured, QT::Power, "W")),
        ),
        (
            "ideal_power",
            ctx.out("ideal_power", q(ideal, QT::Power, "W")),
        ),
    ]))
}

// ---------------------------------------------------------------- payload impact

pub static PAYLOAD_IMPACT: ToolDef = ToolDef {
    id: "drone.power.payload-impact",
    version: "1.1.0",
    stability: gp_base::tool::Stability::Stable,
    title: "What a payload costs in flight time",
    summary: "The extra hover power and the flight time lost when a multirotor carries a payload, from its mass and any electrical draw, by momentum theory.",
    aliases: &[
        "payload flight time loss",
        "drone payload impact",
        "how much flight time does a payload cost",
    ],
    keywords: &[
        "payload",
        "flight time",
        "hover power",
        "endurance",
        "gimbal",
        "sensor",
        "momentum theory",
    ],
    inputs: &[
        HOVER_INPUTS[0],
        HOVER_INPUTS[1],
        HOVER_INPUTS[2],
        qty(
            "payload_mass",
            "Payload mass",
            "Added to the takeoff mass, like 0.3 kg",
            QT::Mass,
            "kg",
        )
        .required()
        .core(),
        qty(
            "usable_energy",
            "Usable energy",
            "After reserve, like 72 Wh",
            QT::Energy,
            "Wh",
        )
        .required()
        .core(),
        qty(
            "payload_power",
            "Payload electrical power",
            "What the payload draws from the flight battery, like 8 W; default 0",
            QT::Power,
            "W",
        ),
        HOVER_INPUTS[3],
        HOVER_INPUTS[4],
        qty(
            "avionics_power",
            "Avionics power",
            "Flight controller, radios, and so on, like 10 W; default 0",
            QT::Power,
            "W",
        ),
        DENSITY_INPUTS[0],
        DENSITY_INPUTS[1],
        DENSITY_INPUTS[2],
    ],
    outputs: &[
        out(
            "hover_time_with",
            "Hover time with the payload",
            "Usable energy / power with the payload",
            QT::Time,
            "min",
            1,
        ),
        out(
            "hover_time_without",
            "Hover time without it",
            "Usable energy / power at the takeoff mass alone",
            QT::Time,
            "min",
            1,
        ),
        out(
            "time_lost",
            "Flight time lost",
            "Without − with",
            QT::Time,
            "min",
            1,
        ),
        out(
            "power_with",
            "Hover power with the payload",
            "Lift power at the new mass + payload and avionics power",
            QT::Power,
            "W",
            1,
        ),
        out(
            "power_increase",
            "Power added",
            "With − without",
            QT::Power,
            "W",
            1,
        ),
        num(
            "power_increase_percent",
            "Power added (%)",
            "Power added / power without",
            0.0,
            100_000.0,
        )
        .precision(Precision::Decimals(1)),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["UNIT_ASSUMED"],
    model: "Lift power = (m·g0)^1.5 / √(2ρA) / (FM·η), so it grows with mass to the power 1.5; hover power = lift power + avionics (+ payload draw when carried); time = usable energy / power",
    accuracy: "Momentum-theory estimate. It ignores the motors' thrust limit and any drag the payload adds in forward flight, so check the maximum takeoff mass in the manual.",
    when_to_use: "Use this before you mount a new gimbal, sensor, or light on a multirotor to see what it costs in hover time. Give the drone's mass, rotors, and rotor size, the payload's mass and any power it draws from the flight battery, and the usable energy, and it gives the hover time with and without the payload, the minutes lost, and the extra power in watts and as a percent.",
    limitations: "Lift power grows with mass to the power 1.5 by momentum theory, at a figure of merit and motor efficiency you can set. It covers hover only, not the drag a bulky payload adds in forward flight. It cannot tell when the motors run out of thrust or when the load passes the maker's maximum takeoff mass, so check the manual before you fly with it.",
    references: &[LEISHMAN, VAN_MULDERS, ICAO_ATM],
    examples: &[
        Example {
            id: "primary",
            title: "The 1.4 kg quad with a 0.3 kg, 8 W payload on 72 Wh",
            input: r#"{"mass":"1.4 kg","rotors":4,"rotor_diameter":"9.4 in","payload_mass":"0.3 kg","usable_energy":"72 Wh","payload_power":"8 W"}"#,
            source: "Momentum theory (Leishman 2006, chapter 2) at both masses",
        },
        Example {
            id: "published-payload-fit",
            title: "A 50 g payload on the published 200 g test-aircraft model",
            input: r#"{"mass":"0.2 kg","rotors":4,"rotor_diameter":"0.054872655656933694 m","payload_mass":"0.05 kg","usable_energy":"20 Wh","avionics_power":"10.6 W"}"#,
            source: "Van Mulders et al. (2023), equation 6 and figure 4a: payload-hover fit P = 10.6 W + 12.88(mg)^1.5",
        },
    ],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[("value", "hover_time_with")],
    }],
    related: &[
        Related {
            id: "drone.power.max-payload",
            reason: "alternative",
        },
        Related {
            id: "drone.power.endurance",
            reason: "next",
        },
        Related {
            id: "drone.power.hover-power",
            reason: "parent",
        },
    ],
    assumptions: &[
        HOVER_POWER.assumptions[0],
        HOVER_POWER.assumptions[1],
        HOVER_POWER.assumptions[2],
        HOVER_POWER.assumptions[3],
        HOVER_POWER.assumptions[4],
    ],
    sentence: "With the payload it hovers for about {hover_time_with}, {time_lost} less than without it.",
    limits: &[("batchRows", 10_000)],
    run: run_payload_impact,
    ..ToolDef::BLANK
};

fn non_negative(ctx: &mut Ctx, name: &str, what: &str) -> Result<f64, ToolError> {
    let v = ctx.quantity(name)?.map_or(0.0, |x| x.base());
    if v < 0.0 {
        return Err(ToolError::invalid(
            &format!("/{name}"),
            format!("{what} cannot be negative."),
        ));
    }
    Ok(v)
}

fn run_payload_impact(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let h = read_hover(ctx)?;
    let e = positive(ctx, "usable_energy", "Usable energy")?;
    let pm = non_negative(ctx, "payload_mass", "Payload mass")?;
    let pp = non_negative(ctx, "payload_power", "Payload power")?;
    let av = non_negative(ctx, "avionics_power", "Avionics power")?;
    let lift0 = hover(h.m, h.n, h.d, h.rho, h.fm, h.eta).electrical;
    let lift1 = hover(h.m + pm, h.n, h.d, h.rho, h.fm, h.eta).electrical;
    let (p0, p1) = (lift0 + av, lift1 + av + pp);
    let (t0, t1) = (e / p0, e / p1);
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Lift power with the payload",
            "lift power grows with mass^1.5",
            format!(
                "{} W × ({} kg / {} kg)^1.5",
                n(lift0, 1),
                n(h.m + pm, 3),
                n(h.m, 3)
            ),
            format!("{} W", n(lift1, 1)),
        );
        ctx.step(
            "Hover power with the payload",
            "lift + avionics + payload draw",
            format!("{} W + {} W + {} W", n(lift1, 1), n(av, 1), n(pp, 1)),
            format!("{} W", n(p1, 1)),
        );
        ctx.step(
            "Hover time with the payload",
            "time = usable energy / power",
            format!("{} Wh / {} W", n(e / 3600.0, 1), n(p1, 1)),
            format!("{} min", n(t1 / 60.0, 1)),
        );
    }
    Ok(Json::obj(vec![
        (
            "hover_time_with",
            ctx.out("hover_time_with", q(t1, QT::Time, "s")),
        ),
        (
            "hover_time_without",
            ctx.out("hover_time_without", q(t0, QT::Time, "s")),
        ),
        ("time_lost", ctx.out("time_lost", q(t0 - t1, QT::Time, "s"))),
        ("power_with", ctx.out("power_with", q(p1, QT::Power, "W"))),
        (
            "power_increase",
            ctx.out("power_increase", q(p1 - p0, QT::Power, "W")),
        ),
        ("power_increase_percent", Json::Num((p1 / p0 - 1.0) * 100.0)),
    ]))
}

// ---------------------------------------------------------------- return to home

pub static RTH_BUDGET: ToolDef = ToolDef {
    id: "drone.power.rth-budget",
    version: "1.1.0",
    stability: gp_base::tool::Stability::Stable,
    title: "Return-to-home energy budget",
    summary: "Whether the battery can bring the drone home against the wind: return groundspeed, time, energy, the margin left, and the farthest round trip that keeps the reserve.",
    aliases: &[
        "return to home calculator",
        "RTH battery calculator",
        "drone headwind return",
    ],
    keywords: &[
        "return to home",
        "RTH",
        "headwind",
        "energy",
        "battery",
        "range",
        "margin",
    ],
    inputs: &[
        qty(
            "distance",
            "Distance home",
            "Like 1.5 km",
            QT::Distance,
            "km",
        )
        .required()
        .core(),
        qty("airspeed", "Airspeed", "Like 15 m/s", QT::Speed, "m/s")
            .required()
            .core(),
        qty(
            "headwind",
            "Headwind on the way home",
            "Like 10 m/s; negative for a tailwind",
            QT::Speed,
            "m/s",
        )
        .core(),
        qty("power", "Cruise power", "Like 180 W", QT::Power, "W")
            .required()
            .core(),
        qty(
            "remaining_energy",
            "Remaining energy",
            "Like 40 Wh",
            QT::Energy,
            "Wh",
        )
        .required()
        .core(),
        qty(
            "reserve_energy",
            "Reserve to keep",
            "Like 10 Wh; default 0",
            QT::Energy,
            "Wh",
        ),
    ],
    outputs: &[
        out(
            "return_groundspeed",
            "Return groundspeed",
            "Airspeed − headwind",
            QT::Speed,
            "m/s",
            1,
        ),
        out(
            "return_time",
            "Return time",
            "Distance / groundspeed",
            QT::Time,
            "min",
            1,
        ),
        out(
            "return_energy",
            "Energy to return",
            "Power × time",
            QT::Energy,
            "Wh",
            1,
        ),
        out(
            "margin",
            "Margin",
            "Remaining − reserve − return energy",
            QT::Energy,
            "Wh",
            1,
        ),
        out(
            "max_round_trip_distance",
            "Farthest out-and-back",
            "From a full remaining charge, keeping the reserve",
            QT::Distance,
            "km",
            2,
        ),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::NoSolution],
    warnings: &["CANNOT_RETURN_INTO_WIND", "UNIT_ASSUMED"],
    model: "Steady wind along the track: return GS = V − w, outbound GS = V + w; energy = power × time; round trip d = E / (P·(1/(V+w) + 1/(V−w)))",
    accuracy: "Exact for a steady along-track wind and constant power. Gusts, climbs, and cold packs need more margin.",
    when_to_use: "Use this while planning, or during a long flight, to check that the battery can bring the drone home against the wind. Give the distance home, airspeed, headwind, cruise power, the energy left, and the reserve you want to land with, and it gives the groundspeed and time home, the energy that takes, and the margin left. It also gives how far out you could fly on the energy left and still return with the reserve.",
    limitations: "It uses one steady wind straight along the track and the same power both ways, so it ignores gusts, crosswind, climbs, the descent, and the landing. The trip home is a straight line at constant airspeed. If the wind is at least as fast as the airspeed, it gives no answer, since the drone could not make headway on one leg. Cold packs, aging cells, and voltage sag can leave less energy than the figure you enter.",
    references: &[FAA_FOSTERING_AVIATION, BAUERSFELD],
    examples: &[
        Example {
            id: "primary",
            title: "1.5 km out, 15 m/s airspeed, 10 m/s headwind home",
            input: r#"{"distance":"1.5 km","airspeed":"15 m/s","headwind":"10 m/s","power":"180 W","remaining_energy":"40 Wh","reserve_energy":"10 Wh"}"#,
            source: "add-drone-suite return scenario: return groundspeed 5 m/s",
        },
        Example {
            id: "faa-headwind",
            title: "FAA's 134.4 mph airspeed into a 30 mph headwind",
            input: r#"{"distance":"10 mi","airspeed":"134.4 mph","headwind":"30 mph","power":"200 W","remaining_energy":"100 Wh","reserve_energy":"20 Wh"}"#,
            source: "FAA Fostering Aviation Activities, Air Speed Indicator: 134.4 mph - 30 mph = 104.4 mph groundspeed",
        },
    ],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[("value", "margin")],
    }],
    related: &[
        Related {
            id: "drone.power.endurance",
            reason: "parent",
        },
        Related {
            id: "drone.mission.sorties",
            reason: "next",
        },
        Related {
            id: "drone.power.payload-impact",
            reason: "alternative",
        },
    ],
    sentence: "Coming home at {return_groundspeed} takes {return_time} and {return_energy}, leaving {margin} of margin.",
    limits: &[("batchRows", 10_000)],
    run: run_rth,
    ..ToolDef::BLANK
};

/// The trip home against a steady headwind `w` (negative for a tailwind), as
/// the return-to-home tool computes it: (groundspeed, time, energy) =
/// (V − w, d / (V − w), P × time). The caller checks that V exceeds |w|.
pub fn return_home(d: f64, v: f64, w: f64, p: f64) -> (f64, f64, f64) {
    let gs = v - w;
    let t = d / gs;
    (gs, t, p * t)
}

fn run_rth(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let d = positive(ctx, "distance", "Distance")?;
    let v = positive(ctx, "airspeed", "Airspeed")?;
    let w = ctx.quantity("headwind")?.map_or(0.0, |x| x.base());
    let p = positive(ctx, "power", "Cruise power")?;
    let e = positive(ctx, "remaining_energy", "Remaining energy")?;
    let res = ctx.quantity("reserve_energy")?.map_or(0.0, |x| x.base());
    if res < 0.0 {
        return Err(ToolError::invalid(
            "/reserve_energy",
            "Reserve energy cannot be negative.",
        ));
    }
    if w.abs() >= v {
        ctx.warnings.push(Warning::new(
            "CANNOT_RETURN_INTO_WIND",
            "The wind is at least as fast as the airspeed, so the drone cannot make progress against it on one leg.",
        ));
        return Err(ToolError::new(
            ErrorCode::NoSolution,
            "The wind is at least as fast as the airspeed: the drone cannot fly home against it.",
        )
        .at("/headwind"));
    }
    let (gs, t, need) = return_home(d, v, w, p);
    let j = |x: f64| q(x, QT::Energy, "J");
    let round = (e - res).max(0.0) / (p * (1.0 / (v + w) + 1.0 / (v - w)));
    Ok(Json::obj(vec![
        (
            "return_groundspeed",
            ctx.out("return_groundspeed", q(gs, QT::Speed, "m/s")),
        ),
        ("return_time", ctx.out("return_time", q(t, QT::Time, "s"))),
        ("return_energy", ctx.out("return_energy", j(need))),
        ("margin", ctx.out("margin", j(e - res - need))),
        (
            "max_round_trip_distance",
            ctx.out("max_round_trip_distance", q(round, QT::Distance, "m")),
        ),
    ]))
}
