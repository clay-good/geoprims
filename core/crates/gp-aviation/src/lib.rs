//! Aviation: atmosphere, altimetry, and wind (add-aviation-suite). The first
//! slice is the preflight hero set: ISA, pressure and density altitude, ISA
//! temperature, runway wind components, and the wind triangle.

pub mod airspeed;
pub mod atmosphere;
pub mod ifr;
pub mod loading;
pub mod performance;
pub mod weather;
pub mod wind;

use atmosphere as isa;
use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Registry, Related, ToolDef};
use gp_base::units::{self, Quantity as QT, Unit};
use serde_json::Value;

pub mod refs {
    use gp_base::tool::Reference;

    pub const ICAO_7488: Reference = Reference {
        title: "Manual of the ICAO Standard Atmosphere (extended to 80 kilometres (262 500 feet))",
        issuer: "International Civil Aviation Organization",
        year: 1993,
        edition: "3rd edition, Doc 7488/3",
        locator: "Section 2 (defining constants and equations) and Table 1",
        url: "https://store.icao.int/en/manual-of-the-icao-standard-atmosphere-extended-to-80-kilometres-262500-feet-doc-7488",
    };
    pub const US76: Reference = Reference {
        title: "U.S. Standard Atmosphere, 1976",
        issuer: "NOAA, NASA, and U.S. Air Force",
        year: 1976,
        edition: "NOAA-S/T 76-1562",
        locator: "Part 1, equations 23 and 33a, and Table I",
        url: "https://ntrs.nasa.gov/citations/19770009539",
    };
    pub const PHAK: Reference = Reference {
        title: "Pilot's Handbook of Aeronautical Knowledge, FAA-H-8083-25C",
        issuer: "Federal Aviation Administration",
        year: 2023,
        edition: "FAA-H-8083-25C",
        locator: "Aircraft Performance and Navigation chapters (pressure and density altitude, wind triangle)",
        url: "https://www.faa.gov/regulations_policies/handbooks_manuals/aviation/phak",
    };
    pub const WEATHER_HANDBOOK: Reference = Reference {
        title: "Aviation Weather Handbook, FAA-H-8083-28",
        issuer: "Federal Aviation Administration",
        year: 2022,
        edition: "FAA-H-8083-28",
        locator: "Altimetry chapter (altimeter setting, pressure and density altitude)",
        url: "https://www.faa.gov/regulations_policies/handbooks_manuals/aviation/faa-h-8083-28-aviation-weather-handbook",
    };
    pub const ALDUCHOV: Reference = Reference {
        title: "Improved Magnus form approximation of saturation vapor pressure",
        issuer: "Alduchov, O. A., and Eskridge, R. E., Journal of Applied Meteorology",
        year: 1996,
        edition: "Vol. 35, No. 4",
        locator: "pp. 601-609, equation 21 (AERK coefficients)",
        url: "https://doi.org/10.1175/1520-0450(1996)035%3C0601:IMFAOS%3E2.0.CO;2",
    };
    pub const AC_150_5340: Reference = Reference {
        title: "Standards for Airport Markings, AC 150/5340-1M",
        issuer: "Federal Aviation Administration",
        year: 2019,
        edition: "AC 150/5340-1M",
        locator: "Runway designation marking (whole number nearest one-tenth of the magnetic azimuth)",
        url: "https://www.faa.gov/airports/resources/advisory_circulars/index.cfm/go/document.current/documentNumber/150_5340-1",
    };
    pub const AIM: Reference = Reference {
        title: "Aeronautical Information Manual",
        issuer: "Federal Aviation Administration",
        year: 2026,
        edition: "Current edition",
        locator: "Chapter 4 (tower and ATIS winds, magnetic) and Chapter 7 (METAR and TAF winds, true)",
        url: "https://www.faa.gov/air_traffic/publications/atpubs/aim_html/",
    };
    pub const GRACEY: Reference = Reference {
        title: "Measurement of Aircraft Speed and Altitude, NASA Reference Publication 1046",
        issuer: "William Gracey, NASA Langley Research Center",
        year: 1980,
        edition: "NASA RP-1046",
        locator: "Chapter 2 (airspeed equations: subsonic and Rayleigh supersonic pitot relations) and Chapter 16 (temperature)",
        url: "https://ntrs.nasa.gov/citations/19800015804",
    };
    pub const AFH: Reference = Reference {
        title: "Airplane Flying Handbook, FAA-H-8083-3C",
        issuer: "Federal Aviation Administration",
        year: 2021,
        edition: "FAA-H-8083-3C",
        locator: "Chapter 3 (turns and load factor), Chapter 6 (ground reference maneuvers, pivotal altitude), and Chapter 18 (emergency glides)",
        url: "https://www.faa.gov/regulations_policies/handbooks_manuals/aviation/airplane_handbook",
    };
    pub const IPH: Reference = Reference {
        title: "Instrument Procedures Handbook, FAA-H-8083-16B",
        issuer: "Federal Aviation Administration",
        year: 2017,
        edition: "FAA-H-8083-16B",
        locator: "Chapter 2 (descent planning) and Chapter 4 (visual descent point, vertical descent angle)",
        url: "https://www.faa.gov/regulations_policies/handbooks_manuals/aviation/instrument_procedures_handbook",
    };
    pub const WB_HANDBOOK: Reference = Reference {
        title: "Aircraft Weight and Balance Handbook, FAA-H-8083-1B",
        issuer: "Federal Aviation Administration",
        year: 2016,
        edition: "FAA-H-8083-1B",
        locator: "Chapter 2 (weight and balance theory, fuel weights) and Chapter 4 (CG envelope, percent MAC)",
        url: "https://www.faa.gov/regulations_policies/handbooks_manuals/aviation/faa-h-8083-1",
    };
}

use refs::*;

const TABLE: &[Layer] = &[Layer {
    kind: "table-only",
    map: &[],
}];

pub(crate) fn unit(q: QT, s: &str) -> &'static Unit {
    units::by_symbol(q, s).expect("registered unit")
}

pub(crate) fn m(v: f64) -> Q {
    Q {
        value: v,
        unit: unit(QT::Length, "m"),
    }
}

pub(crate) fn kelvin(v: f64) -> Q {
    Q {
        value: v,
        unit: unit(QT::Temperature, "K"),
    }
}

pub(crate) fn pa(v: f64) -> Q {
    Q {
        value: v,
        unit: unit(QT::Pressure, "Pa"),
    }
}

pub(crate) fn knots(v: f64) -> Q {
    Q {
        value: v,
        unit: unit(QT::Speed, "kt"),
    }
}

pub(crate) fn deg(v: f64) -> Q {
    Q {
        value: v,
        unit: unit(QT::Angle, "deg"),
    }
}

pub(crate) fn obj(pairs: Vec<(&str, Json)>) -> Json {
    Json::obj(pairs)
}

// ---------------------------------------------------------------- atmosphere

pub static ISA: ToolDef = ToolDef {
    id: "aviation.atmosphere.isa",
    stability: gp_base::tool::Stability::Stable,
    title: "Standard atmosphere",
    summary: "Temperature, pressure, density, speed of sound, and viscosity of the ICAO or US 1976 standard atmosphere at any altitude, on a standard or non-standard day.",
    aliases: &[
        "ISA calculator",
        "international standard atmosphere",
        "US standard atmosphere 1976",
    ],
    keywords: &[
        "ISA",
        "atmosphere",
        "air density",
        "speed of sound",
        "pressure at altitude",
        "US76",
    ],
    inputs: &[
        Field::new(
            "altitude",
            "Altitude",
            "Like 10000 ft",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .required()
        .core(),
        Field::new(
            "altitude_type",
            "Altitude type",
            "geopotential (the aviation default) or geometric",
            Kind::Choice(&["geopotential", "geometric"]),
        )
        .core(),
        Field::new(
            "temperature_deviation",
            "ISA deviation",
            "For a non-standard day, like +20 degC",
            Kind::Quantity {
                q: QT::TemperatureDifference,
                unit: "degC",
            },
        )
        .core(),
        Field::new(
            "model",
            "Model",
            "icao (to 80 km) or us76 (to 86 km geometric)",
            Kind::Choice(&["icao", "us76"]),
        ),
    ],
    outputs: &[
        Field::new(
            "temperature",
            "Temperature",
            "Air temperature",
            Kind::Quantity {
                q: QT::Temperature,
                unit: "degC",
            },
        )
        .precision(Precision::Decimals(3)),
        Field::new(
            "pressure",
            "Pressure",
            "Static pressure",
            Kind::Quantity {
                q: QT::Pressure,
                unit: "hPa",
            },
        )
        .precision(Precision::Significant(6)),
        Field::new(
            "density",
            "Density",
            "Air density",
            Kind::Quantity {
                q: QT::Density,
                unit: "kg/m3",
            },
        )
        .precision(Precision::Significant(6)),
        Field::new(
            "theta",
            "Temperature ratio θ",
            "T / T0",
            Kind::Number { min: 0.0, max: 2.0 },
        )
        .precision(Precision::Significant(6)),
        Field::new(
            "delta",
            "Pressure ratio δ",
            "p / p0",
            Kind::Number { min: 0.0, max: 2.0 },
        )
        .precision(Precision::Significant(6)),
        Field::new(
            "sigma",
            "Density ratio σ",
            "ρ / ρ0",
            Kind::Number { min: 0.0, max: 2.0 },
        )
        .precision(Precision::Significant(6)),
        Field::new(
            "speed_of_sound",
            "Speed of sound",
            "√(γRT)",
            Kind::Quantity {
                q: QT::Speed,
                unit: "kt",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "dynamic_viscosity",
            "Dynamic viscosity",
            "Sutherland's law",
            Kind::Quantity {
                q: QT::DynamicViscosity,
                unit: "Pa*s",
            },
        )
        .precision(Precision::Significant(5)),
        Field::new(
            "kinematic_viscosity",
            "Kinematic viscosity",
            "μ / ρ",
            Kind::Quantity {
                q: QT::KinematicViscosity,
                unit: "m2/s",
            },
        )
        .precision(Precision::Significant(5)),
        Field::new(
            "gravity",
            "Gravity",
            "g at this geometric altitude",
            Kind::Quantity {
                q: QT::Acceleration,
                unit: "m/s2",
            },
        )
        .precision(Precision::Significant(6)),
        Field::new(
            "geopotential_altitude",
            "Geopotential altitude",
            "H = r0·z/(r0 + z)",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "geometric_altitude",
            "Geometric altitude",
            "z = r0·H/(r0 − H)",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "layer",
            "Layer",
            "The atmosphere layer",
            Kind::Text { max_len: 40 },
        ),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &["EXPERIMENTAL_TOOL", "UNIT_ASSUMED"],
    model: "ICAO Standard Atmosphere (Doc 7488/3) layer equations on geopotential altitude; US 1976 optional",
    accuracy: "Exact evaluation of the standard's defining equations to double precision; matches the printed tables within their rounding",
    references: &[ICAO_7488, US76],
    examples: &[Example {
        id: "primary",
        title: "The standard atmosphere at 10,000 ft",
        input: r#"{"altitude":"10000 ft"}"#,
        source: "ICAO Doc 7488/3: 268.338 K, 696.82 hPa, 0.904637 kg/m³ at 3,048 m geopotential",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "profile-chart",
        map: &[("value", "temperature")],
    }],
    related: &[
        Related {
            id: "aviation.altimetry.density-altitude",
            reason: "next",
        },
        Related {
            id: "aviation.altimetry.isa-temperature",
            reason: "alternative",
        },
    ],
    sentence: "At {geopotential_altitude} in the standard atmosphere it is {temperature} with pressure {pressure} and density {density}, in the {layer}.",
    limits: &[("batchRows", 10_000)],
    run: run_isa,
    ..ToolDef::BLANK
};

fn run_isa(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let alt = ctx.req_quantity("altitude")?.base();
    let geometric = ctx.choice("altitude_type")? == Some("geometric");
    let model = match ctx.choice("model")? {
        Some("us76") => isa::Model::Us76,
        _ => isa::Model::Icao,
    };
    let h = if geometric {
        isa::geometric_to_geopotential(alt)
    } else {
        alt
    };
    if h < isa::H_MIN || h > model.h_max() {
        let err = ToolError::new(
            ErrorCode::OutOfDomain,
            format!(
                "{} covers {} to {} geopotential; this altitude is outside it.",
                model.name(),
                display::quantity(
                    isa::H_MIN / 1000.0,
                    "km",
                    Precision::Decimals(0),
                    ctx.options.format
                ),
                display::quantity(
                    model.h_max() / 1000.0,
                    "km",
                    Precision::Decimals(3),
                    ctx.options.format
                ),
            ),
        )
        .at("/altitude");
        return Err(if model == isa::Model::Icao {
            err.hint("US Standard Atmosphere 1976 extends to 86 km geometric in this tool: set model to us76.")
        } else {
            err
        });
    }
    let s = isa::at(h);
    let z = if geometric {
        alt
    } else {
        isa::geopotential_to_geometric(h)
    };
    // US 1976 reports kinetic temperature: TM × M/M0 above 80 km geometric.
    let kinetic = match model {
        isa::Model::Us76 => s.t * isa::us76_molecular_weight_ratio(z),
        isa::Model::Icao => s.t,
    };
    let dt = ctx
        .quantity("temperature_deviation")?
        .map_or(0.0, |d| d.base());
    if dt.abs() > 150.0 {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The ISA deviation must be within ±150 °C.",
        )
        .at("/temperature_deviation"));
    }
    let t = kinetic + dt;
    if t <= 0.0 {
        return Err(ToolError::invalid(
            "/temperature_deviation",
            "The deviation makes the temperature fall below absolute zero.",
        ));
    }
    // Density follows the molecular-scale temperature (identical to ISA on a standard day).
    let rho = s.p / (isa::R * (s.t + dt));
    let mu = isa::dynamic_viscosity(t);
    Ok(obj(vec![
        ("temperature", ctx.out("temperature", kelvin(t))),
        ("pressure", ctx.out("pressure", pa(s.p))),
        (
            "density",
            ctx.out(
                "density",
                Q {
                    value: rho,
                    unit: unit(QT::Density, "kg/m3"),
                },
            ),
        ),
        ("theta", Json::Num(t / isa::T0)),
        ("delta", Json::Num(s.p / isa::P0)),
        ("sigma", Json::Num(rho / isa::RHO0)),
        (
            "speed_of_sound",
            ctx.out(
                "speed_of_sound",
                Q {
                    value: isa::speed_of_sound(t),
                    unit: unit(QT::Speed, "m/s"),
                },
            ),
        ),
        (
            "dynamic_viscosity",
            ctx.out(
                "dynamic_viscosity",
                Q {
                    value: mu,
                    unit: unit(QT::DynamicViscosity, "Pa*s"),
                },
            ),
        ),
        (
            "kinematic_viscosity",
            ctx.out(
                "kinematic_viscosity",
                Q {
                    value: mu / rho,
                    unit: unit(QT::KinematicViscosity, "m2/s"),
                },
            ),
        ),
        (
            "gravity",
            ctx.out(
                "gravity",
                Q {
                    value: isa::gravity(z),
                    unit: unit(QT::Acceleration, "m/s2"),
                },
            ),
        ),
        (
            "geopotential_altitude",
            ctx.out("geopotential_altitude", m(h)),
        ),
        ("geometric_altitude", ctx.out("geometric_altitude", m(z))),
        ("layer", Json::str(s.layer)),
    ]))
}

// ---------------------------------------------------------------- altimetry

/// Reads the altimeter setting: a METAR group (`A2980`, `Q1009`) or a pressure.
/// Flags values outside 26.00–32.00 inHg as `SUSPECT_VALUE` without rejecting them.
fn altimeter(ctx: &mut Ctx) -> Result<Q, ToolError> {
    let raw = ctx
        .raw("altimeter")
        .and_then(Value::as_str)
        .map(|s| s.trim().to_ascii_uppercase());
    let q = match raw.as_deref() {
        Some(s)
            if s.len() == 5 && s.starts_with('A') && s[1..].bytes().all(|b| b.is_ascii_digit()) =>
        {
            Q {
                value: s[1..].parse::<f64>().unwrap_or(0.0) / 100.0,
                unit: unit(QT::Pressure, "inHg"),
            }
        }
        Some(s)
            if (4..=5).contains(&s.len())
                && s.starts_with('Q')
                && s[1..].bytes().all(|b| b.is_ascii_digit()) =>
        {
            Q {
                value: s[1..].parse().unwrap_or(0.0),
                unit: unit(QT::Pressure, "hPa"),
            }
        }
        _ => ctx.req_quantity("altimeter")?,
    };
    let in_hg = q.to(unit(QT::Pressure, "inHg"));
    // Sea-level pressure has never been recorded below 25.7 or above 32.1 inHg;
    // 10 to 40 inHg rejects only the impossible (a typo like 0.29 or 2980).
    if !(10.0..=40.0).contains(&in_hg) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The altimeter setting must be between 10 and 40 inHg (339 to 1,355 hPa).",
        )
        .at("/altimeter"));
    }
    if !(26.0..=32.0).contains(&in_hg) {
        ctx.warnings.push(
            Warning::new(
                "SUSPECT_VALUE",
                format!(
                    "An altimeter setting of {} is outside the usual 26.00 to 32.00 inHg (880 to 1,085 hPa). Check it.",
                    display::quantity(q.value, q.unit.symbol, Precision::Significant(6), ctx.options.format)
                ),
            )
            .at("/altimeter"),
        );
    }
    if in_hg <= 0.0 {
        return Err(ToolError::invalid(
            "/altimeter",
            "The altimeter setting must be positive.",
        ));
    }
    Ok(q)
}

/// The field elevation, within -1,000 m to 11,000 m (the lowest and highest
/// airfields sit well inside; the ISA troposphere ends at 11 km).
fn field_elevation(ctx: &mut Ctx) -> Result<Q, ToolError> {
    let e = ctx.req_quantity("elevation")?;
    if !(-1_000.0..=11_000.0).contains(&e.base()) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "Field elevation must be between -3,281 ft and 36,089 ft (-1 km to 11 km).",
        )
        .at("/elevation"));
    }
    Ok(e)
}

/// Station pressure (Pa) from an altimeter setting and field elevation (m),
/// through the ISA pressure-height relation (the definition of QNH).
fn station_pressure(qnh: Q, elevation_m: f64) -> f64 {
    qnh.base() * isa::at(elevation_m).p / isa::P0
}

const ELEVATION: Field = Field::new(
    "elevation",
    "Field elevation",
    "Like 5000 ft",
    Kind::Quantity {
        q: QT::Length,
        unit: "ft",
    },
)
.required()
.core();
const ALTIMETER: Field = Field::new(
    "altimeter",
    "Altimeter setting",
    "Like 29.80 inHg, 1009 hPa, A2980, or Q1009",
    Kind::Quantity {
        q: QT::Pressure,
        unit: "inHg",
    },
)
.required()
.core();

pub static PRESSURE_ALTITUDE: ToolDef = ToolDef {
    id: "aviation.altimetry.pressure-altitude",
    title: "Pressure altitude",
    summary: "Pressure altitude from field elevation and the altimeter setting, exact from the standard atmosphere, with the 1,000 ft per inch rule of thumb beside it.",
    aliases: &["PA calculator", "pressure altitude calculator"],
    keywords: &[
        "pressure altitude",
        "PA",
        "altimeter setting",
        "QNH",
        "29.92",
    ],
    inputs: &[ELEVATION, ALTIMETER],
    outputs: &[
        Field::new(
            "pressure_altitude",
            "Pressure altitude",
            "Altitude in the standard atmosphere with this station pressure",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "station_pressure",
            "Station pressure",
            "Pressure at the field",
            Kind::Quantity {
                q: QT::Pressure,
                unit: "inHg",
            },
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "rule_of_thumb",
            "Rule of thumb",
            "Elevation + (29.92 − setting) × 1,000 ft",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "rule_error",
            "Rule-of-thumb error",
            "Rule of thumb minus the exact value",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .precision(Precision::Decimals(0)),
    ],
    warnings: &["SUSPECT_VALUE", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "ISA pressure-height relation (ISA-derived constants 145,442.16 ft and 0.190263), layered above 36,089 ft",
    accuracy: "Exact to the ISA definition. The NWS constant set (145,366.45 ft, 0.190284) differs by up to about 12 ft. Planning aid, not certified for navigation.",
    references: &[ICAO_7488, WEATHER_HANDBOOK, PHAK],
    examples: &[Example {
        id: "primary",
        title: "A 5,000 ft field with the altimeter at 29.80 inHg",
        input: r#"{"elevation":"5000 ft","altimeter":"29.80 inHg"}"#,
        source: "add-aviation-suite altimetry scenario: 5,108 ft (±1 ft); rule of thumb 5,120 ft",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "gauge",
        map: &[("value", "pressure_altitude")],
    }],
    related: &[
        Related {
            id: "aviation.altimetry.density-altitude",
            reason: "next",
        },
        Related {
            id: "units.pressure.inhg-to-hpa",
            reason: "alternative",
        },
    ],
    sentence: "Pressure altitude is {pressure_altitude}. The 1,000 ft per inch rule gives {rule_of_thumb}, off by {abs(rule_error)}.",
    limits: &[("batchRows", 10_000)],
    run: run_pressure_altitude,
    ..ToolDef::BLANK
};

fn run_pressure_altitude(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let elev = field_elevation(ctx)?;
    let qnh = altimeter(ctx)?;
    let p = station_pressure(qnh, elev.base());
    let pa_m = isa::altitude_for_pressure(p);
    let ft = unit(QT::Length, "ft");
    let rule_ft = elev.to(ft) + (29.92 - qnh.to(unit(QT::Pressure, "inHg"))) * 1000.0;
    let rule_m = Q {
        value: rule_ft,
        unit: ft,
    }
    .base();
    Ok(obj(vec![
        ("pressure_altitude", ctx.out("pressure_altitude", m(pa_m))),
        ("station_pressure", ctx.out("station_pressure", pa(p))),
        ("rule_of_thumb", ctx.out("rule_of_thumb", m(rule_m))),
        ("rule_error", ctx.out("rule_error", m(rule_m - pa_m))),
    ]))
}

pub static DENSITY_ALTITUDE: ToolDef = ToolDef {
    id: "aviation.altimetry.density-altitude",
    title: "Density altitude",
    summary: "How high the airplane feels: density altitude from field elevation, altimeter setting, and temperature, with optional dew point, and the rules of thumb beside it.",
    aliases: &["DA calculator", "density altitude calculator"],
    keywords: &[
        "density altitude",
        "DA",
        "hot and high",
        "takeoff performance",
        "humidity",
    ],
    inputs: &[
        ELEVATION,
        ALTIMETER,
        Field::new(
            "temperature",
            "Outside air temperature",
            "Like 30 degC",
            Kind::Quantity {
                q: QT::Temperature,
                unit: "degC",
            },
        )
        .required()
        .core(),
        Field::new(
            "dew_point",
            "Dew point",
            "Optional, like 20 degC; leave empty for dry air",
            Kind::Quantity {
                q: QT::Temperature,
                unit: "degC",
            },
        )
        .core(),
    ],
    outputs: &[
        Field::new(
            "density_altitude",
            "Density altitude",
            "Altitude in the standard atmosphere with this air density",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "above_field",
            "Above the field",
            "Density altitude minus field elevation",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .precision(Precision::Significant(2)),
        Field::new(
            "pressure_altitude",
            "Pressure altitude",
            "From the altimeter setting",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "isa_temperature",
            "ISA temperature",
            "Standard temperature at the pressure altitude",
            Kind::Quantity {
                q: QT::Temperature,
                unit: "degC",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "isa_deviation",
            "ISA deviation",
            "Outside air temperature minus ISA temperature",
            Kind::Quantity {
                q: QT::TemperatureDifference,
                unit: "degC",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "rule_118_8",
            "118.8 ft per °C rule",
            "PA + 118.8 × ISA deviation",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "rule_118_8_error",
            "118.8 rule error",
            "Rule minus the exact value",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "rule_120",
            "120 ft per °C rule",
            "PA + 120 × ISA deviation",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "rule_120_error",
            "120 rule error",
            "Rule minus the exact value",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "density",
            "Air density",
            "Including humidity when a dew point is given",
            Kind::Quantity {
                q: QT::Density,
                unit: "kg/m3",
            },
        )
        .precision(Precision::Significant(5)),
    ],
    warnings: &[
        "DRY_AIR_ASSUMED",
        "SUSPECT_VALUE",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Air density from station pressure and (virtual) temperature, inverted through the layered ISA density profile; Magnus vapor pressure (Alduchov and Eskridge 1996)",
    accuracy: "Exact to the ISA definition for dry air; with humidity, vapor pressure is within 0.4% from -40 to 50 °C. Planning aid, not certified for navigation.",
    references: &[ICAO_7488, PHAK, WEATHER_HANDBOOK, ALDUCHOV],
    examples: &[Example {
        id: "primary",
        title: "A 5,000 ft field at 30 °C with the altimeter at 29.80 inHg",
        input: r#"{"elevation":"5000 ft","altimeter":"29.80 inHg","temperature":"30 degC"}"#,
        source: "add-aviation-suite altimetry scenario: PA 5,108 ft, ISA 4.88 °C, DA 7,932 ft (±5 ft), 118.8 rule 8,093 ft",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "gauge",
        map: &[("value", "density_altitude")],
    }],
    related: &[
        Related {
            id: "aviation.altimetry.pressure-altitude",
            reason: "alternative",
        },
        Related {
            id: "aviation.atmosphere.isa",
            reason: "alternative",
        },
    ],
    sentence: "Density altitude is {density_altitude}, about {abs(above_field)} {if above_field < 0}lower{else}higher{/if} than the field.{if above_field > 1000} Expect a longer takeoff roll and weaker climb.{/if}{warn DRY_AIR_ASSUMED} Assumes dry air.{/warn}",
    limits: &[("batchRows", 10_000)],
    run: run_density_altitude,
    ..ToolDef::BLANK
};

fn run_density_altitude(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let elev = field_elevation(ctx)?;
    let qnh = altimeter(ctx)?;
    let t = ctx.req_quantity("temperature")?.base();
    // Surface air ranges from about -90 °C to +60 °C; the Magnus fit holds within this span.
    let met = |k: f64| (173.15..=353.15).contains(&k);
    if !met(t) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The temperature must be between -100 °C and +80 °C.",
        )
        .at("/temperature"));
    }
    if let Some(dp) = ctx.quantity("dew_point")?
        && !met(dp.base())
    {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The dew point must be between -100 °C and +80 °C.",
        )
        .at("/dew_point"));
    }
    let p = station_pressure(qnh, elev.base());
    let pa_m = isa::altitude_for_pressure(p);
    let tv = match ctx.quantity("dew_point")? {
        Some(dp) => {
            let td = dp.base();
            if td > t + 1e-9 {
                return Err(ToolError::invalid(
                    "/dew_point",
                    "The dew point cannot be higher than the air temperature.",
                ));
            }
            let e = isa::vapor_pressure(td - 273.15);
            if e >= 0.5 * p {
                return Err(ToolError::new(
                    ErrorCode::OutOfDomain,
                    "That dew point is not possible at this pressure.",
                )
                .at("/dew_point"));
            }
            isa::virtual_temperature(t, e, p)
        }
        None => {
            ctx.warnings.push(Warning::new("DRY_AIR_ASSUMED", "Assumes dry air. Add a dew point to include humidity, which raises density altitude."));
            t
        }
    };
    let rho = p / (isa::R * tv);
    let da_m = isa::altitude_for_density(rho);
    let isa_t = isa::at(pa_m).t;
    let ft = unit(QT::Length, "ft");
    let dev = t - isa_t;
    let pa_ft = m(pa_m).to(ft);
    let rule = |k: f64| {
        Q {
            value: pa_ft + k * dev,
            unit: ft,
        }
        .base()
    };
    let (r1, r2) = (rule(118.8), rule(120.0));
    Ok(obj(vec![
        ("density_altitude", ctx.out("density_altitude", m(da_m))),
        ("above_field", ctx.out("above_field", m(da_m - elev.base()))),
        ("pressure_altitude", ctx.out("pressure_altitude", m(pa_m))),
        ("isa_temperature", ctx.out("isa_temperature", kelvin(isa_t))),
        (
            "isa_deviation",
            ctx.out(
                "isa_deviation",
                Q {
                    value: dev,
                    unit: unit(QT::TemperatureDifference, "K"),
                },
            ),
        ),
        ("rule_118_8", ctx.out("rule_118_8", m(r1))),
        (
            "rule_118_8_error",
            ctx.out("rule_118_8_error", m(r1 - da_m)),
        ),
        ("rule_120", ctx.out("rule_120", m(r2))),
        ("rule_120_error", ctx.out("rule_120_error", m(r2 - da_m))),
        (
            "density",
            ctx.out(
                "density",
                Q {
                    value: rho,
                    unit: unit(QT::Density, "kg/m3"),
                },
            ),
        ),
    ]))
}

pub static ISA_TEMPERATURE: ToolDef = ToolDef {
    id: "aviation.altimetry.isa-temperature",
    title: "ISA temperature and deviation",
    summary: "The standard temperature at a pressure altitude (15 °C falling 1.98 °C per 1,000 ft to -56.5 °C), and the ISA deviation of an outside air temperature.",
    aliases: &["ISA deviation calculator", "standard temperature"],
    keywords: &[
        "ISA temperature",
        "ISA deviation",
        "standard lapse rate",
        "OAT",
    ],
    inputs: &[
        Field::new(
            "pressure_altitude",
            "Pressure altitude",
            "Like 41000 ft (FL410)",
            Kind::Quantity {
                q: QT::Length,
                unit: "ft",
            },
        )
        .required()
        .core(),
        Field::new(
            "temperature",
            "Outside air temperature",
            "Optional, like -50 degC",
            Kind::Quantity {
                q: QT::Temperature,
                unit: "degC",
            },
        )
        .core(),
    ],
    outputs: &[
        Field::new(
            "isa_temperature",
            "ISA temperature",
            "Standard temperature at this pressure altitude",
            Kind::Quantity {
                q: QT::Temperature,
                unit: "degC",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "isa_deviation",
            "ISA deviation",
            "Outside air temperature minus ISA",
            Kind::Quantity {
                q: QT::TemperatureDifference,
                unit: "degC",
            },
        )
        .precision(Precision::Decimals(1))
        .optional(),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "ICAO Standard Atmosphere layer temperatures",
    accuracy: "Exact to the ISA definition",
    references: &[ICAO_7488, PHAK],
    examples: &[Example {
        id: "primary",
        title: "ISA temperature at FL410",
        input: r#"{"pressure_altitude":"41000 ft"}"#,
        source: "add-aviation-suite altimetry scenario: -56.5 °C above 36,089 ft",
    }],
    primary_example: "primary",
    visualization: TABLE,
    related: &[Related {
        id: "aviation.atmosphere.isa",
        reason: "alternative",
    }],
    sentence: "The standard temperature at {pressure_altitude} is {isa_temperature}.{if isa_deviation > -1000} The outside air is ISA {isa_deviation}.{/if}",
    limits: &[("batchRows", 10_000)],
    run: run_isa_temperature,
    ..ToolDef::BLANK
};

fn run_isa_temperature(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let h = ctx.req_quantity("pressure_altitude")?.base();
    if !(isa::H_MIN..=isa::Model::Icao.h_max()).contains(&h) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "The ICAO standard atmosphere covers -5 km to 80 km.",
        )
        .at("/pressure_altitude"));
    }
    let t = isa::at(h).t;
    let mut out = vec![("isa_temperature", ctx.out("isa_temperature", kelvin(t)))];
    if let Some(oat) = ctx.quantity("temperature")? {
        let d = Q {
            value: oat.base() - t,
            unit: unit(QT::TemperatureDifference, "K"),
        };
        out.push(("isa_deviation", ctx.out("isa_deviation", d)));
    }
    Ok(obj(out))
}

// ---------------------------------------------------------------- wind

const REFS: &[&str] = &["magnetic", "true"];

/// Reads the wind from a METAR group (`wind`) or direction and speed fields.
/// Returns the wind and its reference (METAR groups are always true).
fn read_wind(
    ctx: &mut Ctx,
    default_ref: &'static str,
) -> Result<(wind::Wind, &'static str), ToolError> {
    let group = ctx.text("wind")?;
    let stated_ref = ctx.choice("wind_reference")?;
    let (mut w, reference) = match group {
        Some(g) => {
            if ctx.is_set("wind_direction") || ctx.is_set("wind_speed") {
                return Err(ToolError::invalid(
                    "/wind",
                    "Give the wind once: a METAR group or a direction and speed.",
                ));
            }
            if stated_ref == Some("magnetic") {
                return Err(ToolError::invalid(
                    "/wind_reference",
                    "METAR wind groups are always true north.",
                ));
            }
            (wind::parse_metar_wind(&g, "/wind")?, "true")
        }
        None => {
            let speed = ctx
                .quantity("wind_speed")?
                .ok_or_else(|| {
                    ToolError::invalid(
                        "/wind_speed",
                        "Give the wind speed, like 15 kt, or a METAR group such as 30015KT.",
                    )
                })?
                .to(unit(QT::Speed, "kt"));
            let dir = ctx
                .quantity("wind_direction")?
                .map(|d| gp_base::angle::wrap_azimuth(d.to(unit(QT::Angle, "deg"))));
            if dir.is_none() && speed > 0.0 {
                return Err(ToolError::invalid(
                    "/wind_direction",
                    "Give the wind direction (where it blows from), like 300 deg.",
                ));
            }
            let gust = if ctx.declares("gust") {
                ctx.quantity("gust")?.map(|g| g.to(unit(QT::Speed, "kt")))
            } else {
                None
            };
            (
                wind::Wind {
                    dir: dir.or(Some(0.0)),
                    speed,
                    gust,
                    range: None,
                },
                stated_ref.unwrap_or(default_ref),
            )
        }
    };
    if w.speed < 0.0 || w.gust.is_some_and(|g| g < w.speed) {
        return Err(ToolError::invalid(
            "/gust",
            "Gusts must be at least the steady wind speed, and speeds cannot be negative.",
        ));
    }
    if ctx.declares("variable")
        && let Some(r) = ctx.text("variable")?
    {
        w.range = Some(wind::parse_range(&r, "/variable")?);
    }
    Ok((w, reference))
}

/// Converts a direction between references with a variation (east positive).
fn to_reference(
    dir: f64,
    from: &str,
    to: &str,
    variation: Option<f64>,
    field: &str,
) -> Result<f64, ToolError> {
    if from == to {
        return Ok(dir);
    }
    let var = variation.ok_or_else(|| {
        ToolError::invalid(
            field,
            format!("This mixes a {from} direction with a {to} one. Give the magnetic variation, or use one reference."),
        )
        .hint("In US practice, METAR, TAF, and winds aloft are true; ATIS and tower winds and runway numbers are magnetic.")
    })?;
    // magnetic = true − variation (east positive)
    Ok(gp_base::angle::wrap_azimuth(if to == "magnetic" {
        dir - var
    } else {
        dir + var
    }))
}

fn variation(ctx: &mut Ctx) -> Result<Option<f64>, ToolError> {
    Ok(ctx
        .quantity("variation")?
        .map(|v| v.to(unit(QT::Angle, "deg"))))
}

const WIND_FIELDS: [Field; 6] = [
    Field::new(
        "wind_direction",
        "Wind direction",
        "Where the wind blows from, like 300 deg",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    )
    .core()
    .angle_range("[0,360)"),
    Field::new(
        "wind_speed",
        "Wind speed",
        "Like 15 kt",
        Kind::Quantity {
            q: QT::Speed,
            unit: "kt",
        },
    )
    .core(),
    Field::new(
        "gust",
        "Gust",
        "Optional, like 25 kt",
        Kind::Quantity {
            q: QT::Speed,
            unit: "kt",
        },
    ),
    Field::new(
        "wind",
        "METAR wind group",
        "Instead of direction and speed, like 30015G25KT or VRB05KT (true north)",
        Kind::Text { max_len: 16 },
    ),
    Field::new(
        "variable",
        "Variable range",
        "Like 280V340 from the METAR",
        Kind::Text { max_len: 8 },
    ),
    Field::new(
        "variation",
        "Magnetic variation",
        "East positive, like -12 for 12° W; needed to mix true and magnetic",
        Kind::Quantity {
            q: QT::Angle,
            unit: "deg",
        },
    ),
];

pub static RUNWAY_COMPONENTS: ToolDef = ToolDef {
    id: "aviation.wind.runway-components",
    title: "Runway wind components",
    summary: "Crosswind and headwind or tailwind on a runway, with gusts, variable winds, and your personal limits.",
    aliases: &[
        "crosswind calculator",
        "headwind calculator",
        "crosswind component",
    ],
    keywords: &[
        "crosswind",
        "headwind",
        "tailwind",
        "runway",
        "gust",
        "components",
    ],
    inputs: &[
        Field::new(
            "runway",
            "Runway",
            "A designator like 27, 09L, or 36T (true)",
            Kind::Text { max_len: 12 },
        )
        .required()
        .core(),
        WIND_FIELDS[0],
        WIND_FIELDS[1],
        WIND_FIELDS[2],
        Field::new(
            "max_crosswind",
            "Your crosswind limit",
            "Optional, like 15 kt",
            Kind::Quantity {
                q: QT::Speed,
                unit: "kt",
            },
        )
        .core(),
        Field::new(
            "max_tailwind",
            "Your tailwind limit",
            "Optional, like 5 kt",
            Kind::Quantity {
                q: QT::Speed,
                unit: "kt",
            },
        ),
        Field::new(
            "runway_heading",
            "Runway heading",
            "Optional precise heading, like 268 deg, instead of the designator's",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .angle_range("[0,360)"),
        Field::new(
            "wind_reference",
            "Wind reference",
            "magnetic (ATIS and tower, the default) or true (METAR)",
            Kind::Choice(REFS),
        ),
        WIND_FIELDS[3],
        WIND_FIELDS[4],
        WIND_FIELDS[5],
    ],
    outputs: &[
        Field::new(
            "crosswind",
            "Crosswind",
            "Crosswind component",
            Kind::Quantity {
                q: QT::Speed,
                unit: "kt",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "crosswind_from",
            "Crosswind from",
            "left, right, none, or either side",
            Kind::Text { max_len: 12 },
        ),
        Field::new(
            "headwind",
            "Headwind",
            "Positive headwind, negative tailwind",
            Kind::Quantity {
                q: QT::Speed,
                unit: "kt",
            },
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "gust_crosswind",
            "Gust crosswind",
            "Crosswind at the gust speed",
            Kind::Quantity {
                q: QT::Speed,
                unit: "kt",
            },
        )
        .precision(Precision::Decimals(1))
        .optional(),
        Field::new(
            "gust_headwind",
            "Gust headwind",
            "Headwind at the gust speed",
            Kind::Quantity {
                q: QT::Speed,
                unit: "kt",
            },
        )
        .precision(Precision::Decimals(1))
        .optional(),
        Field::new(
            "crosswind_status",
            "Crosswind limit",
            "Within, near, or beyond your limit",
            Kind::Text { max_len: 80 },
        )
        .optional(),
        Field::new(
            "tailwind_status",
            "Tailwind limit",
            "Within, near, or beyond your limit",
            Kind::Text { max_len: 80 },
        )
        .optional(),
        Field::new(
            "runway_heading",
            "Runway heading used",
            "In the wind's reference",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(0))
        .angle_range("[0,360)"),
    ],
    warnings: &[
        "RUNWAY_HEADING_APPROXIMATE",
        "VARIABLE_WIND",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "Vector components: headwind = W·cos(θ), crosswind = W·sin(θ)",
    accuracy: "Exact for the entered wind and heading. Runway numbers round the heading to 10°, so components can be off by up to W·sin 5°. Planning aid, not certified for navigation.",
    references: &[PHAK, AC_150_5340, AIM],
    examples: &[Example {
        id: "primary",
        title: "Runway 27 with the wind 300° at 15 gusting 25 kt",
        input: r#"{"runway":"27","wind_direction":"300 deg","wind_speed":"15 kt","gust":"25 kt","max_crosswind":"15 kt"}"#,
        source: "add-aviation-suite wind scenarios: headwind 13.0 kt, crosswind 7.5 kt from the right, gust crosswind 12.5 kt",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[
            ("crosswind", "crosswind"),
            ("headwind", "headwind"),
            ("heading", "runway_heading"),
        ],
    }],
    related: &[Related {
        id: "aviation.wind.heading-groundspeed",
        reason: "next",
    }],
    sentence: "{if crosswind > 0}{crosswind} crosswind from the {crosswind_from}{else}No crosswind{/if} and {abs(headwind)} {if headwind < 0}tailwind{else}headwind{/if} on runway {runway}{if gust_crosswind > 0}, {gust_crosswind} crosswind in gusts{/if}.{warn VARIABLE_WIND} These are the worst case for the variable wind.{/warn}",
    limits: &[("batchRows", 10_000)],
    run: run_runway_components,
    ..ToolDef::BLANK
};

fn run_runway_components(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let designator = ctx.text("runway")?.expect("required");
    let rwy = wind::parse_designator(&designator, "/runway")?;
    let rwy_ref = if rwy.true_ref { "true" } else { "magnetic" };
    let heading = match ctx.quantity("runway_heading")? {
        Some(h) => gp_base::angle::wrap_azimuth(h.to(unit(QT::Angle, "deg"))),
        None => {
            ctx.warnings.push(
                Warning::new(
                    "RUNWAY_HEADING_APPROXIMATE",
                    format!("Runway {designator} was taken as {}°. Actual runway headings can differ by up to 5°; enter the published heading for exact components.", rwy.heading),
                )
                .at("/runway"),
            );
            rwy.heading
        }
    };
    let (w, wind_ref) = read_wind(ctx, "magnetic")?;
    let var = variation(ctx)?;
    // Work in the runway's reference.
    let rwy_heading = heading;
    let kt = |v: f64| knots(v);
    let mut out: Vec<(&str, Json)> = Vec::new();
    let (cross, head, from, gust) = match (w.dir, w.range) {
        (Some(d), None) if w.speed > 0.0 => {
            let d = to_reference(d, wind_ref, rwy_ref, var, "/wind_reference")?;
            let (h, x) = wind::components(rwy_heading, d, w.speed);
            let side = if x.abs() < 1e-9 {
                "none"
            } else if x > 0.0 {
                "right"
            } else {
                "left"
            };
            let gust = w.gust.map(|g| wind::components(rwy_heading, d, g));
            (x.abs(), h, side, gust.map(|(gh, gx)| (gx.abs(), gh)))
        }
        (_, _) if w.speed == 0.0 => (0.0, 0.0, "none", None),
        (dir, range) => {
            // Variable wind: the worst case over the range, or over all directions (VRB).
            let range = match (dir, range) {
                (_, Some((a, b))) => Some((
                    to_reference(a, wind_ref, rwy_ref, var, "/wind_reference")?,
                    to_reference(b, wind_ref, rwy_ref, var, "/wind_reference")?,
                )),
                _ => None,
            };
            ctx.warnings.push(Warning::new("VARIABLE_WIND", "The wind direction varies, so the components shown are the worst case over its range."));
            let (x, t) = wind::worst_case(rwy_heading, w.speed, range);
            let gust = w.gust.map(|g| {
                let (gx, gt) = wind::worst_case(rwy_heading, g, range);
                (gx, -gt)
            });
            (x, -t, "either side", gust)
        }
    };
    out.push(("crosswind", ctx.out("crosswind", kt(cross))));
    out.push(("crosswind_from", Json::str(from)));
    out.push(("headwind", ctx.out("headwind", kt(head))));
    if let Some((gx, gh)) = gust {
        out.push(("gust_crosswind", ctx.out("gust_crosswind", kt(gx))));
        out.push(("gust_headwind", ctx.out("gust_headwind", kt(gh))));
    }
    let worst_cross = gust.map_or(cross, |(gx, _)| gx.max(cross));
    let worst_tail = (-head).max(gust.map_or(0.0, |(_, gh)| -gh)).max(0.0);
    for (field, name, value, label) in [
        (
            "max_crosswind",
            "crosswind_status",
            worst_cross,
            "crosswind",
        ),
        ("max_tailwind", "tailwind_status", worst_tail, "tailwind"),
    ] {
        if let Some(limit) = ctx.quantity(field)? {
            let lim_kt = limit.to(unit(QT::Speed, "kt"));
            let text = format!(
                "{} {label} limit",
                display::quantity(
                    limit.value,
                    limit.unit.symbol,
                    Precision::Significant(4),
                    ctx.options.format
                )
            );
            out.push((name, Json::str(wind::status(value, lim_kt, 0.10, &text))));
        }
    }
    out.push((
        "runway_heading",
        ctx.out("runway_heading", deg(rwy_heading)),
    ));
    Ok(obj(out))
}

pub static HEADING_GROUNDSPEED: ToolDef = ToolDef {
    id: "aviation.wind.heading-groundspeed",
    title: "Wind triangle: heading and groundspeed",
    summary: "The heading to fly and the groundspeed you will get for a course, true airspeed, and wind: the E6B wind side, exact.",
    aliases: &["E6B", "wind triangle", "WCA", "wind correction angle"],
    keywords: &[
        "wind correction",
        "heading",
        "groundspeed",
        "course",
        "E6B",
        "flight computer",
    ],
    inputs: &[
        Field::new(
            "course",
            "Course",
            "Like 090 deg",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core()
        .angle_range("[0,360)"),
        Field::new(
            "tas",
            "True airspeed",
            "Like 120 kt",
            Kind::Quantity {
                q: QT::Speed,
                unit: "kt",
            },
        )
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
        Field::new(
            "wind_reference",
            "Wind reference",
            "true (the default: METAR, TAF, winds aloft) or magnetic",
            Kind::Choice(REFS),
        ),
        WIND_FIELDS[3],
        WIND_FIELDS[5],
    ],
    outputs: &[
        Field::new(
            "heading",
            "Heading",
            "The heading to fly, in the course's reference",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(1))
        .angle_range("[0,360)"),
        Field::new(
            "groundspeed",
            "Groundspeed",
            "Speed over the ground",
            Kind::Quantity {
                q: QT::Speed,
                unit: "kt",
            },
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
    errors: &[ErrorCode::NoSolution],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Wind triangle: WCA = asin(W/TAS · sin(WD − TC)), GS = TAS·cos(WCA) − W·cos(WD − TC)",
    accuracy: "Exact for steady wind and flat-earth geometry over a leg. Planning aid, not certified for navigation.",
    references: &[PHAK, AIM],
    examples: &[Example {
        id: "primary",
        title: "Course 090°, 120 kt, wind 030° at 20 kt",
        input: r#"{"course":"90 deg","tas":"120 kt","wind_direction":"30 deg","wind_speed":"20 kt"}"#,
        source: "add-aviation-suite wind scenario: WCA -8.30°, heading 081.7°, groundspeed 108.7 kt",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[("heading", "heading"), ("groundspeed", "groundspeed")],
    }],
    related: &[
        Related {
            id: "aviation.wind.find-wind",
            reason: "inverse",
        },
        Related {
            id: "aviation.wind.runway-components",
            reason: "alternative",
        },
    ],
    sentence: "Fly heading {heading} for a groundspeed of {groundspeed}, a wind correction of {abs(wind_correction_angle)} to the {if wind_correction_angle < 0}left{else}right{/if}.",
    limits: &[("batchRows", 10_000)],
    run: run_heading_groundspeed,
    ..ToolDef::BLANK
};

fn run_heading_groundspeed(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let dg = unit(QT::Angle, "deg");
    let course = gp_base::angle::wrap_azimuth(ctx.req_quantity("course")?.to(dg));
    let tas = ctx.req_quantity("tas")?.to(unit(QT::Speed, "kt"));
    if tas <= 0.0 {
        return Err(ToolError::invalid(
            "/tas",
            "True airspeed must be greater than zero.",
        ));
    }
    let course_ref = ctx.choice("course_reference")?.unwrap_or("true");
    let (w, wind_ref) = read_wind(ctx, "true")?;
    let var = variation(ctx)?;
    let wd = match w.dir {
        Some(d) => to_reference(d, wind_ref, course_ref, var, "/wind_reference")?,
        None => {
            return Err(ToolError::invalid(
                "/wind",
                "The wind triangle needs a steady wind direction, not a variable one.",
            ));
        }
    };
    let Some((wca, gs)) = wind::heading_groundspeed(course, tas, wd, w.speed) else {
        return Err(ToolError::new(
            ErrorCode::NoSolution,
            "The crosswind is stronger than the true airspeed, so this course cannot be held.",
        )
        .hint("Choose another course, fly faster, or wait for the wind to ease."));
    };
    if gs <= 0.0 {
        return Err(ToolError::new(
            ErrorCode::NoSolution,
            "The headwind is at least the airspeed, so the aircraft makes no progress along the course.",
        ));
    }
    Ok(obj(vec![
        (
            "heading",
            ctx.out("heading", deg(gp_base::angle::wrap_azimuth(course + wca))),
        ),
        ("groundspeed", ctx.out("groundspeed", knots(gs))),
        (
            "wind_correction_angle",
            ctx.out("wind_correction_angle", deg(wca)),
        ),
    ]))
}

pub static FIND_WIND: ToolDef = ToolDef {
    id: "aviation.wind.find-wind",
    title: "Wind triangle: find the wind",
    summary: "The wind aloft from your heading, true airspeed, track, and groundspeed.",
    aliases: &["find wind", "wind from groundspeed"],
    keywords: &["wind", "E6B", "wind triangle", "track", "groundspeed"],
    inputs: &[
        Field::new(
            "heading",
            "Heading",
            "Like 081.7 deg",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core()
        .angle_range("[0,360)"),
        Field::new(
            "tas",
            "True airspeed",
            "Like 120 kt",
            Kind::Quantity {
                q: QT::Speed,
                unit: "kt",
            },
        )
        .required()
        .core(),
        Field::new(
            "track",
            "Track",
            "Like 090 deg (same reference as the heading)",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .required()
        .core()
        .angle_range("[0,360)"),
        Field::new(
            "groundspeed",
            "Groundspeed",
            "Like 108.7 kt",
            Kind::Quantity {
                q: QT::Speed,
                unit: "kt",
            },
        )
        .required()
        .core(),
    ],
    outputs: &[
        Field::new(
            "wind_direction",
            "Wind direction",
            "Where the wind blows from, in the heading's reference",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(0))
        .angle_range("[0,360)"),
        Field::new(
            "wind_speed",
            "Wind speed",
            "Wind speed",
            Kind::Quantity {
                q: QT::Speed,
                unit: "kt",
            },
        )
        .precision(Precision::Decimals(1)),
    ],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "Wind vector = ground vector − air vector",
    accuracy: "Exact for steady wind. Planning aid, not certified for navigation.",
    references: &[PHAK],
    examples: &[Example {
        id: "primary",
        title: "Heading 081.7°, 120 kt, track 090°, groundspeed 108.7 kt",
        input: r#"{"heading":"81.7 deg","tas":"120 kt","track":"90 deg","groundspeed":"108.7 kt"}"#,
        source: "add-aviation-suite wind scenario: wind about 030° at 20 kt",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "vector-diagram",
        map: &[("wind", "wind_direction")],
    }],
    related: &[Related {
        id: "aviation.wind.heading-groundspeed",
        reason: "inverse",
    }],
    sentence: "The wind is from {wind_direction} at {wind_speed}.",
    limits: &[("batchRows", 10_000)],
    run: run_find_wind,
    ..ToolDef::BLANK
};

fn run_find_wind(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let dg = unit(QT::Angle, "deg");
    let kt = unit(QT::Speed, "kt");
    let h = ctx.req_quantity("heading")?.to(dg);
    let tas = ctx.req_quantity("tas")?.to(kt);
    let tr = ctx.req_quantity("track")?.to(dg);
    let gs = ctx.req_quantity("groundspeed")?.to(kt);
    if tas <= 0.0 || gs < 0.0 {
        return Err(ToolError::invalid(
            "/tas",
            "True airspeed must be positive and groundspeed cannot be negative.",
        ));
    }
    let (d, s) = wind::find_wind(h, tas, tr, gs);
    let d = if s < 1e-9 {
        0.0
    } else {
        gp_base::angle::wrap_azimuth(d)
    };
    Ok(obj(vec![
        ("wind_direction", ctx.out("wind_direction", deg(d))),
        ("wind_speed", ctx.out("wind_speed", knots(s))),
    ]))
}

/// Every tool in the aviation module.
pub static TOOLS: &[&ToolDef] = &[
    &ISA,
    &PRESSURE_ALTITUDE,
    &DENSITY_ALTITUDE,
    &ISA_TEMPERATURE,
    &RUNWAY_COMPONENTS,
    &HEADING_GROUNDSPEED,
    &FIND_WIND,
    &airspeed::CAS_TO_TAS,
    &airspeed::TAS_TO_CAS,
    &airspeed::TAT_SAT,
    &performance::TURN,
    &performance::DESCENT,
    &performance::CLIMB_GRADIENT,
    &performance::VDP,
    &performance::GLIDE,
    &performance::PIVOTAL_ALTITUDE,
    &loading::FUEL_WEIGHT,
    &loading::WEIGHT_BALANCE,
    &weather::METAR,
    &weather::FB_WINDS,
    &weather::TAF,
    &ifr::HOLD_ENTRY,
    &ifr::HOLD_WIND,
    &ifr::HOLD_SPEED,
];

pub static REGISTRY: Registry = Registry {
    module: "aviation",
    tools: TOOLS,
};

gp_base::export_module!("aviation", REGISTRY);
