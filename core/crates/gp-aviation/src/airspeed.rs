//! Airspeed (aviation/airspeed spec): CAS ↔ impact pressure ↔ Mach ↔ TAS ↔ EAS
//! through the exact compressible-flow relations (Gracey, NASA RP-1046), with
//! the Rayleigh pitot formula above Mach 1, POH calibration tables for IAS, and
//! total ↔ static air temperature.

use crate::atmosphere as isa;
use crate::refs::*;
use crate::{knots, obj, pa, unit};
use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Related, ToolDef};
use gp_base::units::Quantity as QT;
use libm::{pow, sqrt};

/// Sea-level speed of sound (m/s).
pub fn a0() -> f64 {
    isa::speed_of_sound(isa::T0)
}

/// (γ+1)/2)^(γ/(γ−1)) · ((γ+1)/(2γ))^(1/(γ−1)) in its γ = 1.4 form: 1.2^3.5 · 6^2.5.
fn rayleigh_c() -> f64 {
    pow(1.2, 3.5) * pow(6.0, 2.5)
}

/// Impact pressure ratio qc/p for Mach `m`: isentropic below 1, Rayleigh pitot above.
pub fn qc_ratio(m: f64) -> f64 {
    if m <= 1.0 {
        pow(1.0 + 0.2 * m * m, 3.5) - 1.0
    } else {
        rayleigh_c() * pow(m, 7.0) / pow(7.0 * m * m - 1.0, 2.5) - 1.0
    }
}

/// Mach for an impact-pressure ratio qc/p. Above Mach 1 the Rayleigh formula
/// is solved by fixed-point iteration to 1e-12 relative.
pub fn mach_for_qc_ratio(r: f64) -> f64 {
    let m = sqrt(5.0 * (pow(r + 1.0, 2.0 / 7.0) - 1.0));
    if m <= 1.0 {
        return m;
    }
    // r + 1 = (C/7^2.5) · M² / (1 − 1/(7M²))^2.5, so M = K·√((r+1)(1 − 1/(7M²))^2.5).
    let k = sqrt(pow(7.0, 2.5) / rayleigh_c());
    let mut m = m;
    for _ in 0..200 {
        let next = k * sqrt((r + 1.0) * pow(1.0 - 1.0 / (7.0 * m * m), 2.5));
        let done = (next - m).abs() <= 1e-12 * next;
        m = next;
        if done {
            break;
        }
    }
    m
}

/// Impact pressure (Pa) for a calibrated airspeed (m/s): the sea-level relation.
pub fn qc_from_cas(vc: f64) -> f64 {
    isa::P0 * qc_ratio(vc / a0())
}

/// Calibrated airspeed (m/s) for an impact pressure (Pa).
pub fn cas_from_qc(qc: f64) -> f64 {
    a0() * mach_for_qc_ratio(qc / isa::P0)
}

/// Every airspeed at one flight condition.
#[derive(Clone, Copy, Debug)]
pub struct Speeds {
    pub cas: f64,
    pub qc: f64,
    pub mach: f64,
    pub eas: f64,
    /// True airspeed; `None` when the temperature is unknown (Mach input).
    pub tas: Option<f64>,
    pub a: Option<f64>,
    /// Static pressure (Pa).
    pub p: f64,
}

pub fn from_mach(mach: f64, p: f64, t: Option<f64>) -> Speeds {
    let qc = p * qc_ratio(mach);
    let a = t.map(isa::speed_of_sound);
    Speeds {
        cas: cas_from_qc(qc),
        qc,
        mach,
        // EAS = TAS·√σ = M·a0·√(p/P0): it needs no temperature.
        eas: mach * a0() * sqrt(p / isa::P0),
        tas: a.map(|a| mach * a),
        a,
        p,
    }
}

pub fn from_cas(cas: f64, p: f64, t: Option<f64>) -> Speeds {
    let qc = qc_from_cas(cas);
    let mut s = from_mach(mach_for_qc_ratio(qc / p), p, t);
    // Keep the caller's CAS exactly rather than its round trip.
    s.cas = cas;
    s.qc = qc;
    s
}

/// Linear interpolation in a strictly increasing (x, y) table; `None` outside it.
pub fn interpolate(table: &[(f64, f64)], x: f64) -> Option<f64> {
    let (first, last) = (table.first()?, table.last()?);
    if x < first.0 || x > last.0 {
        return None;
    }
    let i = table
        .windows(2)
        .position(|w| x <= w[1].0)
        .unwrap_or(table.len() - 2);
    let ((x0, y0), (x1, y1)) = (table[i], table[i + 1]);
    Some(y0 + (y1 - y0) * (x - x0) / (x1 - x0))
}

// ---------------------------------------------------------------- shared fields

const fn qty(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    q: QT,
    u: &'static str,
) -> Field {
    Field::new(name, title, help, Kind::Quantity { q, unit: u })
}

const PRESSURE_ALTITUDE: Field = qty(
    "pressure_altitude",
    "Pressure altitude",
    "Like 10000 ft (altimeter set to 29.92 inHg or 1013.25 hPa)",
    QT::Length,
    "ft",
)
.required()
.core();

const TEMPERATURE: Field = qty(
    "temperature",
    "Outside air temperature",
    "Static air temperature, like -5 degC",
    QT::Temperature,
    "degC",
)
.core();

const TEMPERATURE_SOURCE: Field = Field::new(
    "temperature_source",
    "Temperature source",
    "measured (default) or isa to assume the standard temperature",
    Kind::Choice(&["measured", "isa"]),
);

const CAL_ROW: &[Field] = &[
    qty("indicated", "Indicated", "KIAS, like 60", QT::Speed, "kt").required(),
    qty("calibrated", "Calibrated", "KCAS, like 62", QT::Speed, "kt").required(),
];

const fn speed_out(name: &'static str, title: &'static str, help: &'static str) -> Field {
    qty(name, title, help, QT::Speed, "kt").precision(Precision::Decimals(1))
}

const MACH_OUT: Field = Field::new(
    "mach",
    "Mach",
    "TAS / speed of sound",
    Kind::Number {
        min: 0.0,
        max: 10.0,
    },
)
.precision(Precision::Decimals(4));

const QC_OUT: Field = qty(
    "impact_pressure",
    "Impact pressure qc",
    "Pitot minus static pressure",
    QT::Pressure,
    "hPa",
)
.precision(Precision::Significant(5));

const FLOW_OUT: Field = Field::new(
    "flow",
    "Flow branch",
    "subsonic (isentropic) or supersonic (Rayleigh pitot)",
    Kind::Text { max_len: 40 },
);

const MIN_P: f64 = 1.0; // Pa: the ISA at 80 km is about 0.9 Pa; any real flight is far above.

/// Static pressure (Pa) at the pressure altitude, checked against the ISA range.
fn static_pressure(ctx: &mut Ctx) -> Result<f64, ToolError> {
    let h = ctx.req_quantity("pressure_altitude")?.base();
    if !(isa::H_MIN..=30_000.0).contains(&h) {
        return Err(ToolError::new(
            ErrorCode::OutOfDomain,
            "Pressure altitude must be between -16,404 ft and 98,425 ft (-5 km to 30 km).",
        )
        .at("/pressure_altitude"));
    }
    Ok(isa::at(h).p.max(MIN_P))
}

/// The static air temperature (K): measured, or ISA at the pressure altitude when
/// the caller chose it (with `ISA_TEMPERATURE_ASSUMED`). `needed` makes it required.
fn static_temperature(ctx: &mut Ctx, needed: bool) -> Result<Option<f64>, ToolError> {
    let isa_chosen = ctx.choice("temperature_source")? == Some("isa");
    let t = ctx.quantity("temperature")?;
    match (t, isa_chosen) {
        (Some(_), true) => Err(ToolError::invalid(
            "/temperature_source",
            "Choose either a measured temperature or the ISA temperature, not both.",
        )),
        (Some(t), false) => {
            let k = t.base();
            if k <= 0.0 {
                return Err(ToolError::invalid(
                    "/temperature",
                    "The temperature must be above absolute zero.",
                ));
            }
            Ok(Some(k))
        }
        (None, true) => {
            let h = ctx.req_quantity("pressure_altitude")?.base();
            let t = isa::at(h).t;
            ctx.warnings.push(Warning::new(
                "ISA_TEMPERATURE_ASSUMED",
                format!(
                    "Uses the standard temperature at this pressure altitude, {}. True airspeed depends on the real outside air temperature.",
                    display::quantity(t - 273.15, "degC", Precision::Decimals(1), ctx.options.format)
                ),
            ));
            Ok(Some(t))
        }
        (None, false) if needed => Err(ToolError::invalid(
            "/temperature",
            "Outside air temperature is required for true airspeed.",
        )
        .hint("Enter the outside air temperature, like -5 degC, or set temperature_source to isa to assume the standard temperature.")),
        (None, false) => Ok(None),
    }
}

fn flow_text(mach: f64) -> &'static str {
    if mach > 1.0 {
        "supersonic (Rayleigh pitot)"
    } else {
        "subsonic (isentropic)"
    }
}

fn speeds_json(ctx: &mut Ctx, s: &Speeds, out: &mut Vec<(&'static str, Json)>) {
    out.push(("mach", Json::Num(s.mach)));
    if let Some(tas) = s.tas {
        out.push(("tas", ctx.out("tas", mps(tas))));
    }
    out.push(("eas", ctx.out("eas", mps(s.eas))));
    out.push(("impact_pressure", ctx.out("impact_pressure", pa(s.qc))));
    if let Some(a) = s.a {
        out.push(("speed_of_sound", ctx.out("speed_of_sound", mps(a))));
    }
    // q = ½ρV² = 0.7·p·M² (γ = 1.4)
    out.push((
        "dynamic_pressure",
        ctx.out("dynamic_pressure", pa(0.7 * s.p * s.mach * s.mach)),
    ));
    out.push((
        "compressibility_correction",
        ctx.out("compressibility_correction", mps(s.cas - s.eas)),
    ));
    out.push(("flow", Json::str(flow_text(s.mach))));
}

fn mps(v: f64) -> Q {
    Q {
        value: v,
        unit: unit(QT::Speed, "m/s"),
    }
}

const COMMON_OUT: [Field; 6] = [
    speed_out("eas", "Equivalent airspeed", "TAS·√σ"),
    QC_OUT,
    speed_out(
        "speed_of_sound",
        "Speed of sound",
        "√(γ·R·T) at the outside air temperature",
    )
    .optional(),
    qty(
        "dynamic_pressure",
        "Dynamic pressure q",
        "½ρV² = 0.7·p·M²",
        QT::Pressure,
        "hPa",
    )
    .precision(Precision::Significant(5)),
    speed_out(
        "compressibility_correction",
        "Compressibility correction",
        "CAS − EAS, never negative",
    ),
    FLOW_OUT,
];

// ---------------------------------------------------------------- CAS → TAS

pub static CAS_TO_TAS: ToolDef = ToolDef {
    id: "aviation.airspeed.cas-to-tas",
    title: "True airspeed from indicated or calibrated airspeed",
    summary: "True airspeed, Mach, and equivalent airspeed from IAS or CAS, pressure altitude, and outside air temperature, by the exact compressible-flow path, with the 2% per 1,000 ft rule beside it.",
    aliases: &[
        "TAS calculator",
        "true airspeed calculator",
        "CAS to TAS",
        "IAS to TAS",
        "Mach calculator",
    ],
    keywords: &[
        "TAS",
        "CAS",
        "IAS",
        "KTAS",
        "Mach",
        "EAS",
        "airspeed",
        "E6B",
        "compressibility",
    ],
    inputs: &[
        qty("airspeed", "Airspeed", "Like 250 kt", QT::Speed, "kt")
            .required()
            .core(),
        Field::new(
            "airspeed_type",
            "Airspeed type",
            "calibrated (default) or indicated with a calibration table",
            Kind::Choice(&["calibrated", "indicated"]),
        )
        .core(),
        PRESSURE_ALTITUDE,
        TEMPERATURE,
        TEMPERATURE_SOURCE,
        Field::new(
            "calibration",
            "Calibration table",
            "From the POH/AFM: indicated and calibrated airspeed pairs, in increasing order, like 70 kt indicated and 73 kt calibrated",
            Kind::List {
                items: CAL_ROW,
                min: 2,
                max: 100,
            },
        ),
        V_SPEEDS[0],
        V_SPEEDS[1],
        V_SPEEDS[2],
        V_SPEEDS[3],
        V_SPEEDS[4],
    ],
    outputs: &[
        speed_out("tas", "True airspeed", "Mach × speed of sound"),
        MACH_OUT,
        speed_out(
            "cas",
            "Calibrated airspeed",
            "IAS corrected by the calibration table, or the input CAS",
        ),
        COMMON_OUT[0],
        COMMON_OUT[1],
        COMMON_OUT[2],
        COMMON_OUT[3],
        COMMON_OUT[4],
        COMMON_OUT[5],
        speed_out(
            "rule_of_thumb",
            "2% per 1,000 ft rule",
            "CAS × (1 + 0.02 × pressure altitude / 1,000 ft)",
        ),
        speed_out("rule_error", "Rule error", "Rule minus the exact TAS"),
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &[
        "ABOVE_VNE",
        "CAUTION_RANGE",
        "BELOW_STALL_SPEED",
        "CALIBRATION_ASSUMED",
        "ISA_TEMPERATURE_ASSUMED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "CAS → impact pressure (sea-level ISA relation) → Mach at the static pressure of the pressure altitude → TAS at the outside air temperature; Rayleigh pitot formula above Mach 1",
    accuracy: "Exact compressible-flow relations for γ = 1.4 air to double precision; the Rayleigh branch is iterated to 1e-12. Planning aid, not certified for navigation.",
    references: &[GRACEY, ICAO_7488, PHAK],
    examples: &[
        Example {
            id: "primary",
            title: "250 KCAS at 10,000 ft and -5 °C",
            input: r#"{"airspeed":"250 kt","pressure_altitude":"10000 ft","temperature":"-5 degC"}"#,
            source: "add-aviation-suite airspeed scenario: Mach 0.4523, TAS 288.6 kt, EAS 248.1 kt",
        },
        Example {
            id: "high-altitude",
            title: "300 KCAS at FL350 and -54.3 °C",
            input: r#"{"airspeed":"300 kt","pressure_altitude":"35000 ft","temperature":"-54.3 degC"}"#,
            source: "add-aviation-suite airspeed scenario: Mach 0.8736, TAS 503.6 kt",
        },
    ],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "gauge",
        map: &[("value", "tas")],
    }],
    related: &[
        Related {
            id: "aviation.airspeed.tas-to-cas",
            reason: "inverse",
        },
        Related {
            id: "aviation.airspeed.tat-sat",
            reason: "next",
        },
        Related {
            id: "aviation.wind.heading-groundspeed",
            reason: "next",
        },
    ],
    sentence: "True airspeed is {tas}, Mach {mach}. The 2% rule gives {rule_of_thumb}, off by {abs(rule_error)}.{warn ABOVE_VNE} Above VNE.{/warn}{warn CAUTION_RANGE} In the caution range.{/warn}{warn CALIBRATION_ASSUMED} Treats IAS as CAS.{/warn}{warn ISA_TEMPERATURE_ASSUMED} Assumes ISA temperature.{/warn}",
    limits: &[("batchRows", 10_000)],
    run: run_cas_to_tas,
    ..ToolDef::BLANK
};

/// The V-speeds of the aircraft, from its POH: the speeds an airspeed
/// indicator's arcs begin and end at. All optional; each one given is checked
/// against the airspeed, and the gauge draws the arcs the set can support.
static V_SPEEDS: &[Field] = &[
    qty(
        "vs0",
        "VS0, stall speed in the landing configuration",
        "Bottom of the white arc, like 48 kt",
        QT::Speed,
        "kt",
    ),
    qty(
        "vs1",
        "VS1, stall speed clean",
        "Bottom of the green arc, like 55 kt",
        QT::Speed,
        "kt",
    ),
    qty(
        "vfe",
        "VFE, maximum flaps extended",
        "Top of the white arc, like 95 kt",
        QT::Speed,
        "kt",
    ),
    qty(
        "vno",
        "VNO, maximum structural cruising",
        "Top of the green arc, like 130 kt",
        QT::Speed,
        "kt",
    ),
    qty(
        "vne",
        "VNE, never exceed",
        "The red line, like 160 kt",
        QT::Speed,
        "kt",
    ),
];

/// Reads the V-speeds, checks they are ordered as an indicator's arcs are, and
/// says where this calibrated airspeed falls among them.
fn check_v_speeds(ctx: &mut Ctx, cas: f64) -> Result<(), ToolError> {
    let mut v = [None; 5];
    for (i, f) in V_SPEEDS.iter().enumerate() {
        v[i] = ctx.quantity(f.name)?.map(|q| q.base());
        if v[i].is_some_and(|x| x <= 0.0) {
            return Err(ToolError::invalid(
                &format!("/{}", f.name),
                "A V-speed must be positive.",
            ));
        }
    }
    let [vs0, vs1, vfe, vno, vne] = v;
    // The arcs an indicator is marked with only make sense in this order.
    for (lo, hi, why) in [
        (
            vs0,
            vs1,
            "VS0 is the stall speed with the flaps down, so it is below VS1, the clean stall speed.",
        ),
        (
            vs1,
            vno,
            "VS1 is the bottom of the green arc and VNO its top.",
        ),
        (
            vs0,
            vfe,
            "VS0 is the bottom of the white arc and VFE its top.",
        ),
        (
            vno,
            vne,
            "VNO is the bottom of the yellow arc and VNE the red line at its top.",
        ),
    ] {
        if let (Some(a), Some(b)) = (lo, hi)
            && a >= b
        {
            return Err(ToolError::invalid("/vne", why)
                .hint("Check the V-speeds against the POH/AFM airspeed indicator markings."));
        }
    }
    let fmt = ctx.options.format;
    let show = |x: f64| {
        display::quantity(
            mps(x).to(knots(0.0).unit),
            "kt",
            Precision::Decimals(0),
            fmt,
        )
    };
    if let Some(x) = vne
        && cas > x
    {
        let m = format!(
            "This calibrated airspeed is above VNE ({}), past the red line.",
            show(x)
        );
        ctx.warnings
            .push(Warning::new("ABOVE_VNE", m).at("/airspeed"));
    } else if let (Some(a), Some(b)) = (vno, vne)
        && cas > a
    {
        let m = format!(
            "This calibrated airspeed is in the caution range, between VNO ({}) and VNE ({}): smooth air only.",
            show(a),
            show(b)
        );
        ctx.warnings
            .push(Warning::new("CAUTION_RANGE", m).at("/airspeed"));
    }
    if let Some(a) = vs0
        && cas < a
    {
        let m = format!(
            "This calibrated airspeed is below VS0 ({}), the stall speed in the landing configuration.",
            show(a)
        );
        ctx.warnings
            .push(Warning::new("BELOW_STALL_SPEED", m).at("/airspeed"));
    }
    Ok(())
}

fn run_cas_to_tas(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let v = ctx.req_quantity("airspeed")?.base();
    if v <= 0.0 {
        return Err(ToolError::invalid(
            "/airspeed",
            "Airspeed must be positive.",
        ));
    }
    let indicated = ctx.choice("airspeed_type")? == Some("indicated");
    let rows = ctx.rows("calibration")?;
    let cas = if !indicated {
        if !rows.is_empty() {
            return Err(ToolError::invalid(
                "/calibration",
                "A calibration table applies only to indicated airspeed.",
            )
            .hint("Set airspeed_type to indicated, or remove the table."));
        }
        v
    } else if rows.is_empty() {
        ctx.warnings.push(Warning::new(
            "CALIBRATION_ASSUMED",
            "No calibration table was given, so indicated airspeed is treated as calibrated airspeed. Add the POH/AFM airspeed calibration table for your flap setting.",
        ).at("/calibration"));
        v
    } else {
        let mut table = Vec::with_capacity(rows.len());
        for (i, r) in rows.iter().enumerate() {
            let x = ctx
                .row_quantity("calibration", i, r, "indicated")?
                .expect("required")
                .base();
            let y = ctx
                .row_quantity("calibration", i, r, "calibrated")?
                .expect("required")
                .base();
            if table.last().is_some_and(|(px, _): &(f64, f64)| x <= *px) {
                return Err(ToolError::invalid(
                    &format!("/calibration/{i}/indicated"),
                    "Calibration rows must be in increasing order of indicated airspeed.",
                ));
            }
            table.push((x, y));
        }
        interpolate(&table, v).ok_or_else(|| {
            let show = |x: f64| {
                display::quantity(mps(x).to(knots(0.0).unit), "kt", Precision::Decimals(1), ctx.options.format)
            };
            ToolError::new(
                ErrorCode::OutOfDomain,
                format!(
                    "The calibration table covers {} to {} indicated; this airspeed is outside it. The tool does not extrapolate.",
                    show(table[0].0),
                    show(table[table.len() - 1].0)
                ),
            )
            .at("/airspeed")
        })?
    };
    check_v_speeds(ctx, cas)?;
    let p = static_pressure(ctx)?;
    let t = static_temperature(ctx, true)?;
    let s = from_cas(cas, p, t);
    let tas = s.tas.expect("temperature is known");
    let pa_ft = ctx
        .req_quantity("pressure_altitude")?
        .to(unit(QT::Length, "ft"));
    let rule = cas * (1.0 + 0.02 * pa_ft / 1000.0);
    let mut out = vec![("cas", ctx.out("cas", mps(cas)))];
    speeds_json(ctx, &s, &mut out);
    out.push(("rule_of_thumb", ctx.out("rule_of_thumb", mps(rule))));
    out.push(("rule_error", ctx.out("rule_error", mps(rule - tas))));
    Ok(obj(out))
}

// ---------------------------------------------------------------- TAS/Mach → CAS

pub static TAS_TO_CAS: ToolDef = ToolDef {
    id: "aviation.airspeed.tas-to-cas",
    title: "Calibrated airspeed from true airspeed or Mach",
    summary: "The calibrated airspeed to fly for a true airspeed or Mach number at a pressure altitude and temperature, with Mach, EAS, and impact pressure.",
    aliases: &["TAS to CAS", "Mach to CAS", "Mach to TAS"],
    keywords: &["CAS", "TAS", "Mach", "EAS", "airspeed", "crossover"],
    inputs: &[
        qty(
            "tas",
            "True airspeed",
            "Like 288.6 kt; or give Mach instead",
            QT::Speed,
            "kt",
        )
        .core(),
        Field::new(
            "mach",
            "Mach",
            "Like 0.78; or give TAS instead",
            Kind::Number { min: 0.0, max: 5.0 },
        )
        .core(),
        PRESSURE_ALTITUDE,
        TEMPERATURE,
        TEMPERATURE_SOURCE,
        V_SPEEDS[0],
        V_SPEEDS[1],
        V_SPEEDS[2],
        V_SPEEDS[3],
        V_SPEEDS[4],
    ],
    outputs: &[
        speed_out(
            "cas",
            "Calibrated airspeed",
            "From impact pressure through the sea-level relation",
        ),
        MACH_OUT,
        speed_out("tas", "True airspeed", "Mach × speed of sound").optional(),
        COMMON_OUT[0],
        COMMON_OUT[1],
        COMMON_OUT[2],
        COMMON_OUT[3],
        COMMON_OUT[4],
        COMMON_OUT[5],
    ],
    errors: &[ErrorCode::OutOfDomain],
    warnings: &[
        "ABOVE_VNE",
        "CAUTION_RANGE",
        "BELOW_STALL_SPEED",
        "ISA_TEMPERATURE_ASSUMED",
        "UNIT_ASSUMED",
        "EXPERIMENTAL_TOOL",
    ],
    model: "TAS → Mach at the outside air temperature → impact pressure at the static pressure → CAS by the sea-level relation; Rayleigh pitot formula above Mach 1",
    accuracy: "Exact compressible-flow relations for γ = 1.4 air to double precision. Planning aid, not certified for navigation.",
    references: &[GRACEY, ICAO_7488],
    examples: &[Example {
        id: "primary",
        title: "Mach 0.78 at FL350",
        input: r#"{"mach":0.78,"pressure_altitude":"35000 ft","temperature":"-54.3 degC"}"#,
        source: "Round trip of the add-aviation-suite FL350 scenario (Gracey RP-1046 equations)",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "gauge",
        map: &[("value", "cas")],
    }],
    related: &[Related {
        id: "aviation.airspeed.cas-to-tas",
        reason: "inverse",
    }],
    sentence: "Fly {cas} calibrated for Mach {mach}.{if tas > 0} That is {tas} true.{/if}{warn ABOVE_VNE} Above VNE.{/warn}{warn CAUTION_RANGE} In the caution range.{/warn}{warn ISA_TEMPERATURE_ASSUMED} Assumes ISA temperature.{/warn}",
    limits: &[("batchRows", 10_000)],
    run: run_tas_to_cas,
    ..ToolDef::BLANK
};

fn run_tas_to_cas(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let tas = ctx.quantity("tas")?;
    let mach = ctx.number("mach")?;
    let p = static_pressure(ctx)?;
    let s = match (tas, mach) {
        (Some(v), None) => {
            let v = v.base();
            if v <= 0.0 {
                return Err(ToolError::invalid(
                    "/tas",
                    "True airspeed must be positive.",
                ));
            }
            let t = static_temperature(ctx, true)?.expect("required");
            from_mach(v / isa::speed_of_sound(t), p, Some(t))
        }
        (None, Some(mm)) => {
            if mm <= 0.0 {
                return Err(ToolError::invalid("/mach", "Mach must be positive."));
            }
            let t = static_temperature(ctx, false)?;
            from_mach(mm, p, t)
        }
        _ => {
            return Err(ToolError::invalid(
                "/tas",
                "Give either true airspeed or Mach, not both and not neither.",
            ));
        }
    };
    check_v_speeds(ctx, s.cas)?;
    let mut out = vec![("cas", ctx.out("cas", mps(s.cas)))];
    speeds_json(ctx, &s, &mut out);
    Ok(obj(out))
}

// ---------------------------------------------------------------- TAT ↔ SAT

pub static TAT_SAT: ToolDef = ToolDef {
    id: "aviation.airspeed.tat-sat",
    title: "Total and static air temperature",
    summary: "Static (outside) air temperature from the total air temperature a probe reads, or the reverse, with the ram rise at your Mach number.",
    aliases: &["TAT to SAT", "SAT to TAT", "ram rise", "RAT"],
    keywords: &[
        "TAT",
        "SAT",
        "OAT",
        "ram rise",
        "recovery factor",
        "temperature probe",
    ],
    inputs: &[
        qty(
            "temperature",
            "Temperature",
            "Like -20 degC",
            QT::Temperature,
            "degC",
        )
        .required()
        .core(),
        Field::new(
            "temperature_kind",
            "Temperature kind",
            "total (the probe reading, default) or static",
            Kind::Choice(&["total", "static"]),
        )
        .core(),
        Field::new(
            "mach",
            "Mach",
            "Like 0.8",
            Kind::Number { min: 0.0, max: 5.0 },
        )
        .required()
        .core(),
        Field::new(
            "recovery_factor",
            "Recovery factor",
            "The probe's recovery factor r, default 1.0",
            Kind::Number { min: 0.5, max: 1.0 },
        ),
    ],
    outputs: &[
        qty(
            "sat",
            "Static air temperature",
            "The true outside air temperature",
            QT::Temperature,
            "degC",
        )
        .precision(Precision::Decimals(2)),
        qty(
            "tat",
            "Total air temperature",
            "What the probe reads",
            QT::Temperature,
            "degC",
        )
        .precision(Precision::Decimals(2)),
        qty(
            "ram_rise",
            "Ram rise",
            "TAT − SAT",
            QT::TemperatureDifference,
            "K",
        )
        .precision(Precision::Decimals(2)),
    ],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "SAT = TAT / (1 + 0.2·r·M²)",
    accuracy: "Exact for the stated recovery factor; real probes have r from about 0.75 to 1.0",
    references: &[GRACEY],
    examples: &[Example {
        id: "primary",
        title: "TAT -20 °C at Mach 0.8",
        input: r#"{"temperature":"-20 degC","mach":0.8}"#,
        source: "add-aviation-suite airspeed scenario: SAT -48.73 °C, ram rise 28.73 K",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[Related {
        id: "aviation.airspeed.cas-to-tas",
        reason: "next",
    }],
    sentence: "The static air temperature is {sat}. Ram heating adds {ram_rise}.",
    limits: &[("batchRows", 10_000)],
    run: run_tat_sat,
    ..ToolDef::BLANK
};

fn run_tat_sat(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let t = ctx.req_quantity("temperature")?.base();
    if t <= 0.0 {
        return Err(ToolError::invalid(
            "/temperature",
            "The temperature must be above absolute zero.",
        ));
    }
    let mach = ctx.number("mach")?.expect("required");
    let r = ctx.number("recovery_factor")?.unwrap_or(1.0);
    let f = 1.0 + 0.2 * r * mach * mach;
    let (sat, tat) = if ctx.choice("temperature_kind")? == Some("static") {
        (t, t * f)
    } else {
        (t / f, t)
    };
    let k = |v: f64| Q {
        value: v,
        unit: unit(QT::Temperature, "K"),
    };
    Ok(obj(vec![
        ("sat", ctx.out("sat", k(sat))),
        ("tat", ctx.out("tat", k(tat))),
        (
            "ram_rise",
            ctx.out(
                "ram_rise",
                Q {
                    value: tat - sat,
                    unit: unit(QT::TemperatureDifference, "K"),
                },
            ),
        ),
    ]))
}
