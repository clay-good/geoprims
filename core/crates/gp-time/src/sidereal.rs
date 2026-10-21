//! Sidereal time (add-celestial-and-moon, time/sidereal-and-calendar):
//! the Earth rotation angle, Greenwich mean and apparent sidereal time, and
//! the equation of the equinoxes by IAU 2006/2000A, as the IERS Conventions
//! (2010) chapter 5 state them, with Meeus's IAU 1982 mean sidereal time
//! beside for comparison.

use crate::civil;
use gp_astro::sidereal::sidereal;
use gp_base::ErrorCode;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef};
use gp_base::units::{self, Quantity as QT};
use std::f64::consts::TAU;

const J2000: f64 = 2_451_545.0;
const JD_UNIX: f64 = 2_440_587.5;

fn secs(q: Option<Q>) -> Option<f64> {
    q.map(|q| q.to(units::by_symbol(QT::Time, "s").expect("s")))
}

const IERS_2010: Reference = Reference {
    title: "IERS Conventions (2010), IERS Technical Note 36, Chapter 5",
    issuer: "International Earth Rotation and Reference Systems Service (Petit and Luzum, eds.)",
    year: 2010,
    edition: "Technical Note 36, chapter 5 of 10 August 2012, with Tables 5.2e and 5.3a",
    locator: "Equations 5.14 to 5.15 (ERA), 5.32 (GMST), 5.40 (mean obliquity), 5.43 and 5.44 (fundamental arguments); Table 5.3a (nutation in longitude) and Table 5.2e (GST's non-polynomial part)",
    url: "https://iers-conventions.obspm.fr/content/chapter5/icc5.pdf",
};
const MEEUS: Reference = Reference {
    title: "Astronomical Algorithms",
    issuer: "Meeus, J., Willmann-Bell",
    year: 1998,
    edition: "2nd edition",
    locator: "Chapter 12, equation 12.4 (mean sidereal time at Greenwich, IAU 1982)",
    url: "https://www.willbell.com/math/mc1.htm",
};

/// An angle as hours, minutes, and seconds of time, to the millisecond.
fn hms(rad: f64) -> String {
    let ms = (rad.rem_euclid(TAU) / TAU * 86_400_000.0).round() as i64 % 86_400_000;
    format!(
        "{}h {:02}m {:02}.{:03}s",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    )
}

const HOURS: f64 = 24.0 / TAU;

pub static SIDEREAL: ToolDef = ToolDef {
    id: "time.scale.sidereal",
    title: "Sidereal time",
    summary: "Greenwich mean and apparent sidereal time, the Earth rotation angle, and the equation of the equinoxes for a UTC instant by IAU 2006/2000A, with local sidereal time for a longitude.",
    aliases: &[
        "sidereal time calculator",
        "GMST calculator",
        "local sidereal time",
    ],
    keywords: &[
        "sidereal",
        "GMST",
        "GAST",
        "LST",
        "ERA",
        "astronomy",
        "IAU 2006",
    ],
    inputs: &[
        Field::new(
            "utc",
            "UTC time",
            "Like 2000-01-01T12:00:00Z",
            Kind::Text { max_len: 40 },
        )
        .required()
        .core(),
        Field::new(
            "longitude",
            "Longitude",
            "East positive, like -74.5 for 74°30'W, for local sidereal time",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .core(),
        Field::new(
            "dut1",
            "DUT1 (UT1 − UTC)",
            "From IERS Bulletin A, like 0.05 s",
            Kind::Quantity {
                q: QT::Time,
                unit: "s",
            },
        )
        .core(),
    ],
    outputs: &[
        Field::new(
            "gmst",
            "Greenwich mean sidereal time",
            "IAU 2006",
            Kind::Text { max_len: 24 },
        ),
        Field::new(
            "gast",
            "Greenwich apparent sidereal time",
            "IAU 2006/2000A",
            Kind::Text { max_len: 24 },
        ),
        Field::new(
            "era",
            "Earth rotation angle",
            "IAU 2000",
            Kind::Quantity {
                q: QT::Angle,
                unit: "deg",
            },
        )
        .precision(Precision::Decimals(7)),
        Field::new(
            "ee",
            "Equation of the equinoxes",
            "Apparent minus mean, in seconds of time",
            Kind::Quantity {
                q: QT::Time,
                unit: "s",
            },
        )
        .precision(Precision::Decimals(4)),
        Field::new(
            "lmst",
            "Local mean sidereal time",
            "GMST + longitude",
            Kind::Text { max_len: 24 },
        )
        .optional(),
        Field::new(
            "last",
            "Local apparent sidereal time",
            "GAST + longitude",
            Kind::Text { max_len: 24 },
        )
        .optional(),
        Field::new(
            "gmst_decimal",
            "GMST, decimal",
            "In sidereal hours, 0 to 24, like 18.697",
            Kind::Number {
                min: 0.0,
                max: 24.0,
            },
        )
        .precision(Precision::Plain(9)),
        Field::new(
            "gast_decimal",
            "GAST, decimal",
            "In sidereal hours, 0 to 24, like 18.697",
            Kind::Number {
                min: 0.0,
                max: 24.0,
            },
        )
        .precision(Precision::Plain(9)),
        Field::new(
            "gmst_meeus",
            "GMST by IAU 1982 (Meeus)",
            "For comparison",
            Kind::Text { max_len: 24 },
        ),
        Field::new(
            "meeus_difference",
            "IAU 1982 minus IAU 2006",
            "Seconds of time",
            Kind::Quantity {
                q: QT::Time,
                unit: "s",
            },
        )
        .precision(Precision::Decimals(4)),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["UT1_APPROXIMATED"],
    model: "ERA = 2π(UT1 day fraction + 0.7790572732640 + 0.00273781191135448 Tu) (IERS 2010 eq. 5.15); GMST = ERA + 0.014506″ + 4612.156534″ t + … (eq. 5.32, t in TT centuries); GAST = GMST + Δψ cos εA + the complementary terms, with Δψ from Table 5.3a (IAU 2000A_R06, 1,358 terms) and the complementary terms from Table 5.2e; LST = GST + east longitude",
    accuracy: "Within 1e-9 rad (0.2 mas, 14 µs of time) of ERFA's era00, gmst06, and gst06a; UT1 is UTC plus DUT1, so without DUT1 the result can be up to 0.9 s of UT1 off",
    when_to_use: "Use this when an astronomical or geodetic calculation needs sidereal time: pointing a telescope, reducing a star observation, checking an antenna or an Earth-orientation computation. Enter the UTC time, and DUT1 from IERS Bulletin A for full accuracy; add a longitude for local sidereal time.",
    limitations: "UT1 comes from UTC plus the DUT1 you give; without it the tool uses UTC and the result can be off by up to 0.9 s of UT1, 13.5 arcseconds. TT comes from the leap-second table (from 1972) or a dated ΔT estimate before that. The IAU 1982 value is shown only for comparison.",
    references: &[IERS_2010, MEEUS],
    examples: &[Example {
        id: "primary",
        title: "J2000.0, 2000-01-01 at 12:00 UT1",
        input: r#"{"utc":"2000-01-01T12:00:00Z","dut1":"0 s"}"#,
        source: "add-celestial-and-moon time spec scenario: GMST 18h 41m 50.549s by IAU 2006 (50.548 s by IAU 1982), ERA 280.4606°",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "time.scale.julian-date",
            reason: "alternative",
        },
        Related {
            id: "time.sun.position",
            reason: "next",
        },
        Related {
            id: "time.scale.unix-time",
            reason: "alternative",
        },
    ],
    sentence: "Greenwich mean sidereal time is {gmst}, and apparent sidereal time is {gast}.",
    limits: &[("batchRows", 10_000)],
    run: run_sidereal,
    ..ToolDef::BLANK
};

fn run_sidereal(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let s = civil::parse_stamp(&ctx.text("utc")?.expect("required"))
        .map_err(|m| ToolError::invalid("/utc", m))?;
    let (days, sec) = crate::to_utc(s);
    if !(-2_000_000..=2_000_000).contains(&days) {
        return Err(ToolError::invalid(
            "/utc",
            "Dates from about 3500 BC to AD 7400 only.",
        ));
    }
    let dut1 = secs(ctx.quantity("dut1")?);
    if let Some(v) = dut1 {
        if v.is_nan() || v.abs() > 0.9 {
            return Err(ToolError::invalid(
                "/dut1",
                "DUT1 stays within ±0.9 s by definition.",
            ));
        }
    } else {
        ctx.warnings.push(Warning::new(
            "UT1_APPROXIMATED",
            "UT1 was taken as UTC; give DUT1 from IERS Bulletin A. Without it sidereal time can be off by up to 0.9 s.",
        ));
    }
    let sec = sec.min(86_400.0);
    // UT1 as a whole Julian day number and a fraction.
    let ut1 = sec + dut1.unwrap_or(0.0);
    let jd_day = JD_UNIX + days as f64;
    let (ut1_day, ut1_frac) = (jd_day, ut1 / 86_400.0);
    // TT: TAI − UTC + 32.184 s from 1972, else the dated ΔT estimate.
    let tt_minus_ut1 = civil::tt_minus_ut1(days, dut1.unwrap_or(0.0));
    let t_tt = ((jd_day - J2000) + (ut1 + tt_minus_ut1) / 86_400.0) / 36_525.0;
    let st = sidereal(ut1_day, ut1_frac, t_tt);
    let rad = units::by_symbol(QT::Angle, "rad").expect("rad");
    let s_u = units::by_symbol(QT::Time, "s").expect("s");
    let to_s = |rad: f64| rad / TAU * 86_400.0;
    let mut out = vec![
        ("gmst", Json::str(hms(st.gmst))),
        ("gast", Json::str(hms(st.gast))),
        (
            "era",
            ctx.out(
                "era",
                Q {
                    value: st.era,
                    unit: rad,
                },
            ),
        ),
        (
            "ee",
            ctx.out(
                "ee",
                Q {
                    value: to_s(st.ee),
                    unit: s_u,
                },
            ),
        ),
        ("gmst_decimal", Json::Num(st.gmst * HOURS)),
        ("gast_decimal", Json::Num(st.gast * HOURS)),
        ("gmst_meeus", Json::str(hms(st.gmst82))),
    ];
    let diff = (st.gmst82 - st.gmst + std::f64::consts::PI).rem_euclid(TAU) - std::f64::consts::PI;
    out.push((
        "meeus_difference",
        ctx.out(
            "meeus_difference",
            Q {
                value: to_s(diff),
                unit: s_u,
            },
        ),
    ));
    if let Some(lon) = ctx.quantity("longitude")? {
        let l = lon.to(rad);
        if !(-TAU..=TAU).contains(&l) {
            return Err(ToolError::invalid(
                "/longitude",
                "The longitude is from -360 to 360 degrees.",
            ));
        }
        out.push(("lmst", Json::str(hms(st.gmst + l))));
        out.push(("last", Json::str(hms(st.gast + l))));
    }
    Ok(Json::obj(out))
}
