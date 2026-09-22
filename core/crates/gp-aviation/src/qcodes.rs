//! Q-codes and flight levels (add-aviation-suite, altimetry, "Q-code
//! conversions" and "Flight levels and transition"): QNH, QFE, and QNE for an
//! aerodrome, the altimeter setting from station pressure (ISA or the NWS
//! formula), flight level to altitude on a QNH, and the US lowest usable
//! flight level from 14 CFR 91.121.

use crate::atmosphere as isa;
use crate::refs::*;
use crate::{altimeter, field_elevation, m, obj, pa, station_pressure, unit};
use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::regulation::rule;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use libm::pow;

const fn qty(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    q: QT,
    u: &'static str,
) -> Field {
    Field::new(name, title, help, Kind::Quantity { q, unit: u })
}

const NWS_ALTIMETER: Reference = Reference {
    title: "Altimeter Setting (WxCalc)",
    issuer: "National Weather Service, El Paso",
    year: 2024,
    edition: "Web calculator documentation",
    locator: "The altimeter setting formula from station pressure (hPa) and elevation (m)",
    url: "https://www.weather.gov/media/epz/wxcalc/altimeterSetting.pdf",
};

const CFR_91_121: Reference = Reference {
    title: "14 CFR Part 91, General Operating and Flight Rules",
    issuer: "Federal Aviation Administration",
    year: 1990,
    edition: "eCFR, current",
    locator: "§ 91.121 (altimeter settings; paragraph (b), lowest usable flight level)",
    url: "https://www.ecfr.gov/current/title-14/chapter-I/subchapter-F/part-91/subpart-B/subject-group-ECFRe4c59b5f5506932/section-91.121",
};

/// NWS altimeter setting (hPa) from station pressure (hPa) and elevation (m).
pub fn nws_altimeter(p_hpa: f64, h_m: f64) -> f64 {
    let n = 0.190_284;
    let ps = p_hpa - 0.3;
    ps * pow(
        1.0 + (pow(1013.25, n) * 0.0065 / 288.0) * (h_m / pow(ps, n)),
        1.0 / n,
    )
}

pub static Q_CODES: ToolDef = ToolDef {
    id: "aviation.altimetry.q-codes",
    title: "QNH, QFE, and QNE",
    summary: "Converts between the altimeter setting (QNH) and station pressure (QFE) at an airport, gives the field's pressure altitude (QNE), and works out the altimeter setting from a barometer reading.",
    aliases: &[
        "QNH to QFE",
        "QFE to QNH",
        "altimeter setting from station pressure",
        "station pressure",
        "Q codes",
    ],
    keywords: &[
        "QNH",
        "QFE",
        "QNE",
        "altimeter setting",
        "station pressure",
        "barometer",
        "field elevation",
    ],
    inputs: &[
        crate::ELEVATION,
        qty(
            "altimeter",
            "QNH (altimeter setting)",
            "Like 1013 hPa, 29.92 inHg, A2992, or Q1013; or give QFE instead",
            QT::Pressure,
            "inHg",
        )
        .core(),
        qty(
            "station_pressure",
            "QFE (station pressure)",
            "The barometer at field elevation, like 979.3 hPa; or give QNH instead",
            QT::Pressure,
            "hPa",
        )
        .core(),
    ],
    outputs: &[
        qty(
            "qfe",
            "QFE",
            "Pressure at the field; set it and the altimeter reads 0 on the ground",
            QT::Pressure,
            "hPa",
        )
        .precision(Precision::Decimals(1)),
        qty(
            "qnh",
            "QNH",
            "Set it and the altimeter reads field elevation on the ground",
            QT::Pressure,
            "inHg",
        )
        .precision(Precision::Decimals(2)),
        qty(
            "qne",
            "QNE (field pressure altitude)",
            "What the altimeter reads on the ground at 29.92 inHg (1013.25 hPa)",
            QT::Length,
            "ft",
        )
        .precision(Precision::Decimals(0)),
        qty(
            "qnh_nws",
            "QNH by the NWS formula",
            "From QFE, as US weather stations compute it",
            QT::Pressure,
            "inHg",
        )
        .precision(Precision::Decimals(2))
        .optional(),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["SUSPECT_VALUE", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "QFE = QNH × p_ISA(elevation) ÷ 1013.25 hPa, the ISA pressure-height relation that defines QNH, and QNH from QFE the other way; QNE is the ISA altitude of QFE. From QFE the NWS formula is also shown: (P − 0.3) × (1 + (1013.25^0.190284 × 0.0065 ÷ 288) × h ÷ (P − 0.3)^0.190284)^(1/0.190284), P in hPa and h in m",
    accuracy: "Exact for the ISA relation. The NWS formula subtracts 0.3 hPa for the barometer's height and differs from the ISA form by a few hundredths of an inch at high fields",
    references: &[ICAO_7488, NWS_ALTIMETER],
    examples: &[Example {
        id: "primary",
        title: "QNH 1013 hPa at a 1,000 ft airport",
        input: r#"{"elevation":"1000 ft","altimeter":"1013 hPa"}"#,
        source: "add-aviation-suite QFE scenario: QFE by the ISA pressure-height relation; with QFE set the altimeter reads 0 ft on the ground",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.altimetry.pressure-altitude",
            reason: "parent",
        },
        Related {
            id: "aviation.altimetry.flight-level",
            reason: "next",
        },
    ],
    sentence: "QFE is {qfe} and QNH is {qnh}; the field's pressure altitude is {qne}.",
    limits: &[("batchRows", 10_000)],
    run: run_q_codes,
    ..ToolDef::BLANK
};

fn run_q_codes(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let elev = field_elevation(ctx)?;
    let e_m = elev.base();
    let ratio = isa::at(e_m).p / isa::P0;
    let (qnh_pa, qfe_pa, from_qfe) = match (
        ctx.is_set("altimeter"),
        ctx.quantity("station_pressure")?,
    ) {
        (true, None) => {
            let qnh = altimeter(ctx)?;
            (qnh.base(), station_pressure(qnh, e_m), false)
        }
        (false, Some(p)) => {
            let p = p.base();
            if !(10_000.0..=110_000.0).contains(&p) {
                return Err(ToolError::new(
                    ErrorCode::OutOfDomain,
                    "Station pressure must be between 100 and 1,100 hPa.",
                )
                .at("/station_pressure"));
            }
            (p / ratio, p, true)
        }
        _ => {
            return Err(ToolError::invalid(
                "/altimeter",
                "Give either QNH (the altimeter setting) or QFE (the station pressure), not both.",
            ));
        }
    };
    let qne_m = isa::altitude_for_pressure(qfe_pa);
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "ISA pressure ratio at the field",
            "p_ISA(elevation) ÷ 1013.25 hPa",
            format!("p_ISA({} m) ÷ 1013.25", n(e_m, 1)),
            n(ratio, 5),
        );
        ctx.step(
            "QFE",
            "QFE = QNH × ratio",
            format!("{} hPa × {}", n(qnh_pa / 100.0, 2), n(ratio, 5)),
            display::quantity(qfe_pa / 100.0, "hPa", Precision::Decimals(1), fmt),
        );
    }
    let mut out = vec![
        ("qfe", ctx.out("qfe", pa(qfe_pa))),
        ("qnh", ctx.out("qnh", pa(qnh_pa))),
        ("qne", ctx.out("qne", m(qne_m))),
    ];
    if from_qfe {
        let nws = nws_altimeter(qfe_pa / 100.0, e_m) * 100.0;
        out.push(("qnh_nws", ctx.out("qnh_nws", pa(nws))));
    }
    Ok(obj(out))
}

/// The US lowest usable flight level for an altimeter setting in inHg
/// (14 CFR 91.121(b)), or None below the table (26.92 inHg).
pub fn lowest_usable_fl(inhg: f64) -> Option<u32> {
    // Settings are reported to hundredths; the table's bands are 0.50 inHg wide.
    let below = (2992 - (inhg * 100.0).round() as i64).max(0);
    let k = (below + 49) / 50;
    (k <= 6).then(|| 180 + 5 * k as u32)
}

pub static FLIGHT_LEVEL: ToolDef = ToolDef {
    id: "aviation.altimetry.flight-level",
    title: "Flight level and altitude",
    summary: "The altitude on the local altimeter setting that a flight level puts you at, or the other way round, with the US lowest usable flight level and transition altitude.",
    aliases: &[
        "lowest usable flight level",
        "flight level to altitude",
        "transition altitude",
        "FL to feet",
    ],
    keywords: &[
        "flight level",
        "transition",
        "QNH",
        "29.92",
        "91.121",
        "altimeter setting",
        "LUFL",
    ],
    inputs: &[
        qty(
            "altimeter",
            "Altimeter setting (QNH)",
            "Like 29.42 inHg, A2942, or Q996",
            QT::Pressure,
            "inHg",
        )
        .required()
        .core(),
        Field::new(
            "flight_level",
            "Flight level",
            "Like 185, to find its altitude on this QNH",
            Kind::Number {
                min: -20.0,
                max: 600.0,
            },
        )
        .core(),
        qty(
            "altitude",
            "Altitude on QNH",
            "Like 17500 ft, to find its flight level",
            QT::Length,
            "ft",
        )
        .core(),
    ],
    outputs: &[
        Field::new(
            "lowest_usable_fl",
            "Lowest usable flight level",
            "In the US, from the 14 CFR 91.121(b) table",
            Kind::Number {
                min: 180.0,
                max: 210.0,
            },
        )
        .precision(Precision::Decimals(0)),
        qty(
            "transition_altitude",
            "Transition altitude (US)",
            "At and above it, set 29.92 inHg",
            QT::Length,
            "ft",
        )
        .precision(Precision::Decimals(0)),
        Field::new(
            "rule",
            "Rule",
            "The regulation and the date these rules were checked",
            Kind::Text { max_len: 240 },
        ),
        qty(
            "altitude_of_fl",
            "Altitude of the flight level",
            "On this QNH",
            QT::Length,
            "ft",
        )
        .precision(Precision::Decimals(0))
        .optional(),
        Field::new(
            "fl_of_altitude",
            "Flight level of the altitude",
            "Pressure altitude ÷ 100",
            Kind::Number {
                min: -100.0,
                max: 1000.0,
            },
        )
        .precision(Precision::Decimals(1))
        .optional(),
    ],
    errors: &[ErrorCode::InvalidInput, ErrorCode::OutOfDomain],
    warnings: &["SUSPECT_VALUE", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "A flight level is a pressure altitude in hundreds of feet. On a QNH the same pressure sits at the ISA altitude of p_FL × 1013.25 ÷ QNH (the ISA pressure-height relation that defines QNH). The US lowest usable flight level is FL180 at 29.92 inHg or higher and 500 ft higher for each 0.50 inHg lower, to FL210 at 26.92 (14 CFR 91.121(b))",
    accuracy: "Exact for ISA; real temperature moves the true altitude (see the cold-temperature tool). The US table covers settings down to 26.92 inHg",
    references: &[CFR_91_121, ICAO_7488],
    examples: &[Example {
        id: "primary",
        title: "QNH 29.42 inHg, FL185",
        input: r#"{"altimeter":"29.42 inHg","flight_level":185}"#,
        source: "add-aviation-suite lowest-usable-flight-level scenario: FL185 at 29.42 inHg, with the FAA table cited",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.altimetry.q-codes",
            reason: "alternative",
        },
        Related {
            id: "aviation.altimetry.pressure-altitude",
            reason: "parent",
        },
    ],
    sentence: "The lowest usable flight level is FL{lowest_usable_fl}.",
    limits: &[("batchRows", 10_000)],
    run: run_flight_level,
    ..ToolDef::BLANK
};

fn run_flight_level(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let qnh = altimeter(ctx)?;
    let inhg = qnh.to(unit(QT::Pressure, "inHg"));
    let lufl = lowest_usable_fl(inhg).ok_or_else(|| {
        ToolError::new(
            ErrorCode::OutOfDomain,
            "The 14 CFR 91.121(b) table stops at 26.92 inHg; below that, ask ATC for the lowest usable flight level.",
        )
        .at("/altimeter")
    })?;
    let r = rule("faa-91-121");
    let ft = unit(QT::Length, "ft");
    let fq = |v: f64| Q { value: v, unit: ft };
    let mut out = vec![
        ("lowest_usable_fl", Json::Num(f64::from(lufl))),
        (
            "transition_altitude",
            ctx.out("transition_altitude", fq(r.value)),
        ),
        (
            "rule",
            Json::str(format!(
                "{}(b), rules as of {}. Not legal advice: {}",
                r.citation, r.reviewed, r.url
            )),
        ),
    ];
    // Pressure-altitude offset of the QNH datum, through the ISA relation.
    let scale = isa::P0 / qnh.base();
    if let Some(fl) = ctx.number("flight_level")? {
        let p = isa::at(fq(fl * 100.0).base()).p;
        let h = isa::altitude_for_pressure(p * scale);
        out.push(("altitude_of_fl", ctx.out("altitude_of_fl", m(h))));
    }
    if let Some(a) = ctx.quantity("altitude")? {
        let a_m = a.base();
        if !(-1_000.0..=20_000.0).contains(&a_m) {
            return Err(ToolError::new(
                ErrorCode::OutOfDomain,
                "Give an altitude between -3,281 and 65,617 ft.",
            )
            .at("/altitude"));
        }
        let p = isa::at(a_m).p / scale;
        let pa_ft = m(isa::altitude_for_pressure(p)).to(ft);
        out.push(("fl_of_altitude", Json::Num(pa_ft / 100.0)));
    }
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| display::number(x, Precision::Decimals(d), fmt);
        let below = (2992.0 - (inhg * 100.0).round()).max(0.0);
        ctx.step(
            "Below standard",
            "29.92 − setting, in hundredths of an inch",
            format!("29.92 − {}", n(inhg, 2)),
            n(below, 0),
        );
        ctx.step(
            "Lowest usable flight level",
            "FL180 + 5 for each started 0.50 inHg below 29.92",
            format!("180 + 5 × ⌈{} ÷ 50⌉", n(below, 0)),
            n(f64::from(lufl), 0),
        );
    }
    Ok(obj(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_91_121_table() {
        for (s, fl) in [
            (30.10, 180),
            (29.92, 180),
            (29.91, 185),
            (29.42, 185),
            (29.41, 190),
            (28.92, 190),
            (28.91, 195),
            (27.92, 200),
            (27.42, 205),
            (26.92, 210),
        ] {
            assert_eq!(lowest_usable_fl(s), Some(fl), "{s}");
        }
        assert_eq!(lowest_usable_fl(26.91), None);
    }

    #[test]
    fn nws_formula_at_sea_level_returns_the_reading_less_its_offset() {
        assert!((nws_altimeter(1013.55, 0.0) - 1013.25).abs() < 1e-9);
    }
}
