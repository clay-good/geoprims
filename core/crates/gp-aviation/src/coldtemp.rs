//! Cold-temperature altitude correction (add-aviation-suite, altimetry,
//! "Cold-temperature altitude correction"): the ICAO Doc 8168 (2020) equation
//! for each procedure altitude, beside the 4%-per-10 °C rule and the ICAO
//! cold temperature error table as the AIM prints it, both labeled
//! approximations. No correction is applied when the air is at or above ISA.

use crate::refs::*;
use crate::{obj, unit};
use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::ToolError;
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::Quantity as QT;
use libm::log;

const fn qty(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    q: QT,
    u: &'static str,
) -> Field {
    Field::new(name, title, help, Kind::Quantity { q, unit: u })
}

const PANS_OPS: Reference = Reference {
    title: "Procedures for Air Navigation Services, Aircraft Operations (PANS-OPS), Doc 8168",
    issuer: "International Civil Aviation Organization",
    year: 2020,
    edition: "Volume II, 7th edition",
    locator: "Temperature correction of procedure altitudes below ISA (the 2020 equation, as cited in Transport Canada AC 500-020 section 4.8)",
    url: "https://store.icao.int/en/procedures-for-air-navigation-services-pans-aircraft-operations-volume-i-flight-procedures-doc-8168",
};

const AIM_COLD: Reference = Reference {
    locator: "Chapter 7, Section 3 (cold temperature barometric altimeter errors, Table 7-3-1 ICAO Cold Temperature Error Table, and the Cold Temperature Airports list)",
    url: "https://www.faa.gov/air_traffic/publications/atpubs/aim_html/chap7_section_3.html",
    ..AIM
};

/// Lapse rate L0 in K per ft (ICAO: −0.0019812).
const L0: f64 = -0.001_981_2;
const T0: f64 = 288.15;

/// AIM Table 7-3-1: heights above the airport (ft) across, reported
/// temperatures (°C) down, corrections in ft.
const TABLE_H: [f64; 14] = [
    200.0, 300.0, 400.0, 500.0, 600.0, 700.0, 800.0, 900.0, 1000.0, 1500.0, 2000.0, 3000.0, 4000.0,
    5000.0,
];
const TABLE_T: [f64; 7] = [10.0, 0.0, -10.0, -20.0, -30.0, -40.0, -50.0];
const TABLE: [[f64; 14]; 7] = [
    [
        10.0, 10.0, 10.0, 10.0, 20.0, 20.0, 20.0, 20.0, 20.0, 30.0, 40.0, 60.0, 80.0, 90.0,
    ],
    [
        20.0, 20.0, 30.0, 30.0, 40.0, 40.0, 50.0, 50.0, 60.0, 90.0, 120.0, 170.0, 230.0, 280.0,
    ],
    [
        20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0, 150.0, 200.0, 290.0, 390.0, 490.0,
    ],
    [
        30.0, 50.0, 60.0, 70.0, 90.0, 100.0, 120.0, 130.0, 140.0, 210.0, 280.0, 420.0, 570.0, 710.0,
    ],
    [
        40.0, 60.0, 80.0, 100.0, 120.0, 140.0, 150.0, 170.0, 190.0, 280.0, 380.0, 570.0, 760.0,
        950.0,
    ],
    [
        50.0, 80.0, 100.0, 120.0, 150.0, 170.0, 190.0, 220.0, 240.0, 360.0, 480.0, 720.0, 970.0,
        1210.0,
    ],
    [
        60.0, 90.0, 120.0, 150.0, 180.0, 210.0, 240.0, 270.0, 300.0, 450.0, 590.0, 890.0, 1190.0,
        1500.0,
    ],
];

/// The table read by bilinear interpolation, or None outside it (colder than
/// −50 °C, or a height outside 200 to 5,000 ft). At or above +10 °C it is
/// the +10 °C row, the warmest the table prints.
pub fn table_correction(temp: f64, height: f64) -> Option<f64> {
    if !(TABLE_H[0]..=TABLE_H[13]).contains(&height) || temp < TABLE_T[6] {
        return None;
    }
    let t = temp.min(TABLE_T[0]);
    let j = (0..13).find(|&j| height <= TABLE_H[j + 1]).unwrap_or(12);
    let fh = (height - TABLE_H[j]) / (TABLE_H[j + 1] - TABLE_H[j]);
    let i = (0..6).find(|&i| t >= TABLE_T[i + 1]).unwrap_or(5);
    let ft = (TABLE_T[i] - t) / (TABLE_T[i] - TABLE_T[i + 1]);
    let row = |r: usize| TABLE[r][j] + fh * (TABLE[r][j + 1] - TABLE[r][j]);
    Some(row(i) + ft * (row(i + 1) - row(i)))
}

/// The ICAO 2020 correction (ft, to add) for a height above the altimeter
/// source, with the ISA deviation at the aerodrome; zero when not colder than ISA.
pub fn icao_correction(isa_dev: f64, height: f64, aerodrome_ft: f64) -> f64 {
    if isa_dev >= 0.0 {
        return 0.0;
    }
    (-isa_dev / L0) * log(1.0 + L0 * height / (T0 + L0 * aerodrome_ft))
}

const ALT_ROW: &[Field] = &[
    Field::new(
        "name",
        "Segment",
        "Like FAF or MDA",
        Kind::Text { max_len: 40 },
    ),
    qty(
        "altitude",
        "Published altitude",
        "Like 3500 ft MSL",
        QT::Length,
        "ft",
    )
    .required(),
];

const OUT_ROW: &[Field] = &[
    Field::new(
        "name",
        "Segment",
        "As entered, or its number",
        Kind::Text { max_len: 40 },
    ),
    qty(
        "altitude",
        "Published altitude",
        "As entered",
        QT::Length,
        "ft",
    )
    .precision(Precision::Decimals(0)),
    qty(
        "correction",
        "Correction",
        "ICAO equation, to add",
        QT::Length,
        "ft",
    )
    .precision(Precision::Decimals(0)),
    qty(
        "corrected",
        "Altitude to fly",
        "Published + correction",
        QT::Length,
        "ft",
    )
    .precision(Precision::Decimals(0)),
    qty(
        "rule",
        "4% rule",
        "4% of the height per 10 °C below ISA",
        QT::Length,
        "ft",
    )
    .precision(Precision::Decimals(0)),
    qty(
        "table",
        "From the table",
        "AIM Table 7-3-1, interpolated",
        QT::Length,
        "ft",
    )
    .precision(Precision::Decimals(0))
    .optional(),
];

pub static COLD_TEMPERATURE: ToolDef = ToolDef {
    id: "aviation.altimetry.cold-temperature",
    title: "Cold temperature altitude correction",
    summary: "How much to add to each procedure altitude when the airport is colder than standard, by the ICAO equation, with the 4% rule and the ICAO table beside it.",
    aliases: &[
        "cold temperature correction",
        "cold weather altimeter correction",
        "cold temperature airports",
        "CTA correction",
        "temperature correction altitude",
    ],
    keywords: &[
        "cold",
        "temperature",
        "altimeter error",
        "correction",
        "approach",
        "minimums",
        "ISA",
        "8168",
    ],
    inputs: &[
        qty(
            "elevation",
            "Airport elevation",
            "Of the altimeter-setting source, like 2000 ft",
            QT::Length,
            "ft",
        )
        .required()
        .core(),
        qty(
            "temperature",
            "Reported airport temperature",
            "Like -30 degC",
            QT::Temperature,
            "degC",
        )
        .required()
        .core(),
        Field::new(
            "altitudes",
            "Procedure altitudes",
            "Each published altitude to correct, like FAF, 3500 ft",
            Kind::List {
                items: ALT_ROW,
                min: 1,
                max: 20,
            },
        )
        .required()
        .core(),
    ],
    outputs: &[
        qty(
            "correction",
            "Correction to add",
            "For the first altitude, by the ICAO equation",
            QT::Length,
            "ft",
        )
        .precision(Precision::Decimals(0)),
        qty(
            "corrected",
            "Altitude to fly",
            "The first altitude plus its correction",
            QT::Length,
            "ft",
        )
        .precision(Precision::Decimals(0)),
        qty(
            "isa_deviation",
            "ISA deviation at the airport",
            "Reported temperature − ISA temperature at the elevation",
            QT::TemperatureDifference,
            "degC",
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "note",
            "Note",
            "When no correction applies, why",
            Kind::Text { max_len: 200 },
        )
        .optional(),
        Field::new(
            "altitudes",
            "Every altitude",
            "Corrections by the equation, the rule, and the table",
            Kind::List {
                items: OUT_ROW,
                min: 1,
                max: 20,
            },
        ),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "ΔH = (−ΔT_std ÷ L0) · ln(1 + L0·H ÷ (T0 + L0·H_aerodrome)), with L0 = −0.0019812 K/ft, T0 = 288.15 K, ΔT_std the airport temperature minus ISA at its elevation, and H the height above the airport (ICAO Doc 8168 Vol II, 2020; not the 2018 Vol III form). Approximations shown beside it: 4% of H per 10 °C below ISA, and AIM Table 7-3-1 read by bilinear interpolation",
    accuracy: "The equation is the ICAO standard for procedure design corrections. Transport Canada AC 500-020 (Issue 04, 2025) prints the same equation with T0 written as 273 + 15, which moves a correction by about 0.1 ft. The 4% rule and the table (built for a sea-level airport) run higher. Use the correction method your procedure and the FAA Cold Temperature Airports list in AIM 7-3 call for; this is a planning aid",
    references: &[PANS_OPS, AIM_COLD],
    examples: &[Example {
        id: "primary",
        title: "A 2,000 ft airport at -30 °C, with a 3,500 ft final approach fix",
        input: r#"{"elevation":"2000 ft","temperature":"-30 degC","altitudes":[{"name":"FAF","altitude":"3500 ft"},{"name":"MDA","altitude":"2500 ft"}]}"#,
        source: "add-aviation-suite cold-temperature scenario: 1,500 ft above the airport at -30 °C corrects by about +218 ft, the 4% rule by about +246 ft",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.altimetry.isa-temperature",
            reason: "parent",
        },
        Related {
            id: "aviation.altimetry.pressure-altitude",
            reason: "alternative",
        },
    ],
    sentence: "{if correction > 0}Add {correction}: fly {corrected}.{/if}{if correction < 1}No cold-temperature correction applies.{/if}",
    limits: &[("batchRows", 10_000)],
    run: run_cold,
    ..ToolDef::BLANK
};

fn run_cold(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let (ft, c) = (unit(QT::Length, "ft"), unit(QT::Temperature, "degC"));
    let elev = ctx.req_quantity("elevation")?.to(ft);
    let temp = ctx.req_quantity("temperature")?.to(c);
    if !(-2_000.0..=15_000.0).contains(&elev) {
        return Err(ToolError::invalid(
            "/elevation",
            "Give an airport elevation between -2,000 and 15,000 ft.",
        ));
    }
    if !(-80.0..=60.0).contains(&temp) {
        return Err(ToolError::invalid(
            "/temperature",
            "Give an airport temperature between -80 °C and 60 °C.",
        ));
    }
    let isa = 15.0 + L0 * elev; // °C, L0 in K per ft
    let dev = temp - isa;
    let rows = ctx.rows("altitudes")?;
    let mut out_rows = Vec::with_capacity(rows.len());
    let mut first: Option<(f64, f64)> = None;
    for (i, r) in rows.iter().enumerate() {
        let alt = ctx
            .row_quantity("altitudes", i, r, "altitude")?
            .expect("required")
            .to(ft);
        let h = alt - elev;
        if h < 0.0 {
            return Err(ToolError::invalid(
                &format!("/altitudes/{i}/altitude"),
                "Each altitude must be at or above the airport elevation.",
            ));
        }
        let name = ctx
            .row_text("altitudes", i, r, "name")?
            .unwrap_or_else(|| format!("{}", i + 1));
        let corr = icao_correction(dev, h, elev);
        let rule = if dev < 0.0 {
            0.04 * (-dev / 10.0) * h
        } else {
            0.0
        };
        let table = if dev < 0.0 {
            table_correction(temp, h)
        } else {
            Some(0.0)
        };
        first.get_or_insert((corr, alt + corr));
        let f = |v: f64| Q { value: v, unit: ft }.to_json();
        let mut row = vec![
            ("name", Json::str(name)),
            ("altitude", f(alt)),
            ("correction", f(corr)),
            ("corrected", f(alt + corr)),
            ("rule", f(rule)),
        ];
        if let Some(t) = table {
            row.push(("table", f(t)));
        }
        out_rows.push(Json::obj(row));
    }
    let (corr, corrected) = first.expect("at least one altitude");
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "ISA deviation at the airport",
            "ΔT = temperature − (15 − 0.0019812 × elevation)",
            format!("{} − {}", n(temp, 1), n(isa, 2)),
            format!("{} °C", n(dev, 1)),
        );
        ctx.step(
            "Correction for the first altitude",
            "ΔH = (−ΔT ÷ L0) · ln(1 + L0·H ÷ (T0 + L0·H_aerodrome))",
            format!(
                "(−{} ÷ −0.0019812) · ln(1 − 0.0019812 × H ÷ (288.15 − 0.0019812 × {}))",
                n(dev, 1),
                n(elev, 0)
            ),
            display::quantity(corr, "ft", Precision::Decimals(0), fmt),
        );
    }
    let fq = |v: f64| Q { value: v, unit: ft };
    let mut out = vec![
        ("correction", ctx.out("correction", fq(corr))),
        ("corrected", ctx.out("corrected", fq(corrected))),
        (
            "isa_deviation",
            ctx.out(
                "isa_deviation",
                Q {
                    value: dev,
                    unit: unit(QT::TemperatureDifference, "degC"),
                },
            ),
        ),
    ];
    if dev >= 0.0 {
        out.push((
            "note",
            Json::str(
                "The airport is at or above ISA temperature, so no correction applies: warm-temperature corrections are not applied to procedure altitudes.",
            ),
        ));
    }
    out.push(("altitudes", Json::Arr(out_rows)));
    Ok(obj(out))
}

// ---------------------------------------------------------------- true altitude

pub static TRUE_ALTITUDE: ToolDef = ToolDef {
    id: "aviation.altimetry.true-altitude",
    title: "True altitude",
    summary: "Your true height above sea level from the altimeter reading when the air is colder or warmer than standard: high to low, look out below.",
    aliases: &[
        "true altitude calculator",
        "indicated to true altitude",
        "high to low look out below",
        "altimeter temperature error",
    ],
    keywords: &[
        "true altitude",
        "indicated altitude",
        "temperature error",
        "ISA deviation",
        "cold",
        "warm",
        "altimeter",
    ],
    inputs: &[
        qty(
            "indicated",
            "Indicated altitude",
            "With the local altimeter setting, like 8000 ft",
            QT::Length,
            "ft",
        )
        .required()
        .core(),
        qty(
            "isa_deviation",
            "ISA deviation",
            "Temperature minus standard, like -20 degC",
            QT::TemperatureDifference,
            "degC",
        )
        .required()
        .core(),
        qty(
            "station_elevation",
            "Altimeter-setting station elevation",
            "Where the setting comes from, like 0 ft (the default)",
            QT::Length,
            "ft",
        )
        .core(),
    ],
    outputs: &[
        qty(
            "true_altitude",
            "True altitude",
            "Height above sea level",
            QT::Length,
            "ft",
        )
        .precision(Precision::Decimals(0)),
        qty(
            "error",
            "True minus indicated",
            "Negative: you are lower than the altimeter shows",
            QT::Length,
            "ft",
        )
        .precision(Precision::Decimals(0)),
        qty(
            "rule_error",
            "By the 4% rule",
            "4% of the height above the station per 10 °C of deviation",
            QT::Length,
            "ft",
        )
        .precision(Precision::Decimals(0)),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "True − indicated = (ΔT ÷ L0) · ln(1 + L0·H ÷ (T0 + L0·H_station)) with L0 = −0.0019812 K/ft, T0 = 288.15 K, ΔT the ISA deviation, and H the indicated height above the altimeter-setting station: the ICAO Doc 8168 temperature relation, applied both colder and warmer than ISA. The 4% rule is shown beside it",
    accuracy: "Assumes the deviation holds from the station to the aircraft, as the ICAO relation does; real temperature profiles vary. For procedure altitudes, use the cold-temperature correction tool",
    references: &[PANS_OPS, PHAK],
    examples: &[Example {
        id: "primary",
        title: "8,000 ft indicated in air 20 °C colder than standard",
        input: r#"{"indicated":"8000 ft","isa_deviation":"-20 degC"}"#,
        source: "add-aviation-suite true-altitude scenario: in colder air the true altitude is below the indicated one, and the difference is reported",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.altimetry.cold-temperature",
            reason: "alternative",
        },
        Related {
            id: "aviation.altimetry.isa-temperature",
            reason: "parent",
        },
    ],
    sentence: "You are at {true_altitude}, {abs(error)} {if error < 0}below{else}above{/if} what the altimeter shows.",
    limits: &[("batchRows", 10_000)],
    run: run_true_altitude,
    ..ToolDef::BLANK
};

fn run_true_altitude(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let ft = unit(QT::Length, "ft");
    let ind = ctx.req_quantity("indicated")?.to(ft);
    let dev = ctx
        .req_quantity("isa_deviation")?
        .to(unit(QT::TemperatureDifference, "degC"));
    let stn = ctx.quantity("station_elevation")?.map_or(0.0, |q| q.to(ft));
    if !(-80.0..=60.0).contains(&dev) {
        return Err(ToolError::invalid(
            "/isa_deviation",
            "Give an ISA deviation between -80 °C and +60 °C.",
        ));
    }
    if !(-2_000.0..=15_000.0).contains(&stn) {
        return Err(ToolError::invalid(
            "/station_elevation",
            "Give a station elevation between -2,000 and 15,000 ft.",
        ));
    }
    let h = ind - stn;
    if !(0.0..=60_000.0).contains(&h) || ind > 60_000.0 {
        return Err(ToolError::invalid(
            "/indicated",
            "The indicated altitude must be at or above the station and at most 60,000 ft.",
        ));
    }
    let err = (dev / L0) * log(1.0 + L0 * h / (T0 + L0 * stn));
    let rule = 0.04 * (dev / 10.0) * h;
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Temperature error",
            "(ΔT ÷ L0) · ln(1 + L0·H ÷ (T0 + L0·H_station))",
            format!(
                "({} ÷ −0.0019812) · ln(1 − 0.0019812 × {} ÷ (288.15 − 0.0019812 × {}))",
                n(dev, 1),
                n(h, 0),
                n(stn, 0)
            ),
            display::quantity(err, "ft", Precision::Decimals(0), fmt),
        );
        ctx.step(
            "True altitude",
            "indicated + error",
            format!("{} + {}", n(ind, 0), n(err, 0)),
            display::quantity(ind + err, "ft", Precision::Decimals(0), fmt),
        );
    }
    let f = |v: f64| Q { value: v, unit: ft };
    Ok(obj(vec![
        ("true_altitude", ctx.out("true_altitude", f(ind + err))),
        ("error", ctx.out("error", f(err))),
        ("rule_error", ctx.out("rule_error", f(rule))),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_reads_its_printed_cells_and_between_them() {
        assert_eq!(table_correction(-30.0, 1500.0), Some(280.0));
        assert_eq!(table_correction(-50.0, 5000.0), Some(1500.0));
        assert_eq!(table_correction(10.0, 200.0), Some(10.0));
        // Halfway between -20 and -30 °C at 1,500 ft: (210 + 280) / 2.
        assert_eq!(table_correction(-25.0, 1500.0), Some(245.0));
        assert_eq!(table_correction(-55.0, 1500.0), None);
        assert_eq!(table_correction(-20.0, 6000.0), None);
    }

    #[test]
    fn the_spec_scenario() {
        let dev = -30.0 - (15.0 + L0 * 2000.0);
        let c = icao_correction(dev, 1500.0, 2000.0);
        assert!((c - 218.0).abs() < 1.0, "{c}");
        assert_eq!(icao_correction(0.5, 1500.0, 2000.0), 0.0);
    }
}
