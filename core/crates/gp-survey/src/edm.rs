//! EDM atmospheric and prism corrections (add-survey-suite, instrument
//! reductions, "EDM atmospheric correction"): the first velocity correction in
//! ppm, entered or computed from temperature, pressure, and humidity with the
//! IAG 1999 closed formula against the instrument's reference refractivity,
//! and the prism constant as its own correction.

use gp_base::ErrorCode;
use gp_base::display;
use gp_base::error::{ToolError, Warning};
use gp_base::json::Json;
use gp_base::tool::{
    Assumption, Ctx, Example, Field, Kind, Layer, Precision, Q, Reference, Related, ToolDef,
};
use gp_base::units::Quantity as QT;
use libm::exp;

use crate::unit;

const LANDGATE: Reference = Reference {
    title: "The Survey Instrumentation Calibration Technical User Manual",
    issuer: "Landgate (Western Australian Land Information Authority) with ICSM",
    year: 2025,
    edition: "Online edition, updated 24 November 2025",
    locator: "Section 4.2.3, equations 4.6 and 4.7 (the IAG 1999 Resolution 3 closed formula for group refractivity) and equation 4.1 (the first velocity correction)",
    url: "https://medjil0.lb.landgate.wa.gov.au/calibrationguide/read_manual/edmi_calibration_manual.html",
};

const fn num(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    min: f64,
    max: f64,
) -> Field {
    Field::new(name, title, help, Kind::Number { min, max })
}

const fn qty(
    name: &'static str,
    title: &'static str,
    help: &'static str,
    q: QT,
    u: &'static str,
) -> Field {
    Field::new(name, title, help, Kind::Quantity { q, unit: u })
}

/// Group refractivity at 0 °C, 1013.25 hPa, dry air, for carrier wavelength
/// `lambda` in micrometers (IAG 1999).
pub fn group_refractivity(lambda: f64) -> f64 {
    287.6155 + 4.88660 / (lambda * lambda) + 0.06800 / lambda.powi(4)
}

/// Refractivity (ppm) at temperature `t` °C, pressure `p` hPa, and vapor
/// pressure `e` hPa (IAG 1999 closed formula).
pub fn ambient_refractivity(ng: f64, t: f64, p: f64, e: f64) -> f64 {
    let tk = 273.15 + t;
    (273.15 / 1013.25) * ng * p / tk - 11.27 * e / tk
}

/// Saturation vapor pressure over water in hPa (Magnus form, Alduchov and
/// Eskridge 1996 coefficients).
fn saturation(t: f64) -> f64 {
    6.1094 * exp(17.625 * t / (t + 243.04))
}

pub static EDM_CORRECTION: ToolDef = ToolDef {
    id: "survey.reduction.edm-correction",
    title: "EDM atmospheric and prism corrections",
    summary: "Corrects a measured EDM distance for the air it traveled through (in ppm, entered or worked out from temperature, pressure, and humidity) and for the prism constant, each listed.",
    aliases: &[
        "EDM ppm correction",
        "atmospheric correction ppm",
        "prism constant correction",
        "first velocity correction",
    ],
    keywords: &[
        "EDM",
        "ppm",
        "atmospheric",
        "refractive index",
        "prism constant",
        "total station",
        "distance",
    ],
    inputs: &[
        qty(
            "distance",
            "Measured distance",
            "As the instrument displayed it, like 1000.000 m",
            QT::Length,
            "m",
        )
        .required()
        .core(),
        qty(
            "prism_constant",
            "Prism constant",
            "Added to the distance, like -30 mm",
            QT::Length,
            "mm",
        )
        .core(),
        num(
            "ppm",
            "Atmospheric correction",
            "In ppm if you have it, like 12; or give temperature and pressure",
            -1000.0,
            1000.0,
        )
        .core(),
        qty(
            "temperature",
            "Air temperature",
            "Like 28 degC",
            QT::Temperature,
            "degC",
        )
        .core(),
        qty(
            "pressure",
            "Air pressure",
            "Station pressure, like 980 hPa",
            QT::Pressure,
            "hPa",
        )
        .core(),
        num(
            "humidity",
            "Relative humidity",
            "Percent, like 60; dry air if left out",
            0.0,
            100.0,
        ),
        num(
            "wavelength",
            "Carrier wavelength",
            "Micrometers, from the instrument manual, like 0.658",
            0.3,
            2.0,
        ),
        num(
            "reference_refractivity",
            "Reference refractivity",
            "The instrument's N_ref in ppm, from its manual, like 286.34",
            200.0,
            400.0,
        ),
    ],
    outputs: &[
        qty(
            "corrected",
            "Corrected distance",
            "Measured + atmospheric + prism corrections",
            QT::Length,
            "m",
        )
        .precision(Precision::Decimals(4)),
        num(
            "ppm",
            "Atmospheric correction",
            "In ppm, positive lengthens",
            -1000.0,
            1000.0,
        )
        .precision(Precision::Decimals(2)),
        qty(
            "atmospheric_correction",
            "Atmospheric correction",
            "ppm × distance",
            QT::Length,
            "m",
        )
        .precision(Precision::Decimals(4)),
        qty(
            "prism_correction",
            "Prism correction",
            "The prism constant",
            QT::Length,
            "m",
        )
        .precision(Precision::Decimals(4)),
        num(
            "refractivity",
            "Refractivity of the air",
            "N in ppm, from the IAG 1999 formula",
            0.0,
            1000.0,
        )
        .precision(Precision::Decimals(2))
        .optional(),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["DRY_AIR_ASSUMED", "UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "N_G = 287.6155 + 4.88660 ÷ λ² + 0.06800 ÷ λ⁴ (λ in µm); N_L = (273.15 ÷ 1013.25) · N_G · p ÷ T − 11.27 · e ÷ T, with p and e in hPa and T in kelvins (IAG 1999 Resolution 3); ppm = N_ref − N_L; corrected = D + ppm · 10⁻⁶ · D + prism constant. Vapor pressure from humidity by the Magnus form",
    accuracy: "The closed formula is within 0.25 ppm of the full Ciddor and Hill procedure for ordinary survey conditions; use the instrument's own N_ref and wavelength. Instruments that apply ppm internally need no second correction",
    references: &[LANDGATE],
    examples: &[Example {
        id: "primary",
        title: "1,000 m with +12 ppm and a -30 mm prism",
        input: r#"{"distance":"1000 m","ppm":12,"prism_constant":"-30 mm"}"#,
        source: "add-survey-suite ppm scenario: 1,000.000 + 0.012 − 0.030 = 999.982 m, each correction listed",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "survey.reduction.slope",
            reason: "next",
        },
        Related {
            id: "survey.reduction.combined-factor",
            reason: "next",
        },
    ],
    assumptions: &[
        Assumption {
            name: "Group refractivity constant term (IAG 1999)",
            value: "287.6155",
            unit: "1",
            source: "landgate-edmi-manual",
        },
        Assumption {
            name: "Group refractivity coefficient on 1/wavelength squared",
            value: "4.88660",
            unit: "um2",
            source: "landgate-edmi-manual",
        },
        Assumption {
            name: "Group refractivity coefficient on 1/wavelength to the fourth",
            value: "0.06800",
            unit: "um4",
            source: "landgate-edmi-manual",
        },
        Assumption {
            name: "Reference pressure for the group refractivity",
            value: "1013.25",
            unit: "hPa",
            source: "landgate-edmi-manual",
        },
        Assumption {
            name: "Water vapor refractivity coefficient",
            value: "11.27",
            unit: "K/hPa",
            source: "landgate-edmi-manual",
        },
        Assumption {
            name: "Magnus coefficient a (saturation vapor pressure)",
            value: "17.625",
            unit: "1",
            source: "alduchov",
        },
        Assumption {
            name: "Magnus coefficient b",
            value: "243.04",
            unit: "degC",
            source: "alduchov",
        },
        Assumption {
            name: "Magnus coefficient c",
            value: "6.1094",
            unit: "hPa",
            source: "alduchov",
        },
    ],
    sentence: "The corrected distance is {corrected}: {ppm} ppm for the air and {prism_correction} for the prism.",
    limits: &[("batchRows", 10_000)],
    run: run_edm,
    ..ToolDef::BLANK
};

fn run_edm(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let m = unit(QT::Length, "m");
    let d = ctx.req_quantity("distance")?.to(m);
    if d.is_nan() || d <= 0.0 {
        return Err(ToolError::invalid(
            "/distance",
            "The measured distance must be above zero.",
        ));
    }
    let prism = ctx.quantity("prism_constant")?.map_or(0.0, |q| q.to(m));
    if prism.abs() > 1.0 {
        return Err(ToolError::invalid(
            "/prism_constant",
            "A prism constant is a few centimeters, like -30 mm.",
        ));
    }
    let given = ctx.number("ppm")?;
    let temp = ctx
        .quantity("temperature")?
        .map(|q| q.to(unit(QT::Temperature, "degC")));
    let pres = ctx
        .quantity("pressure")?
        .map(|q| q.to(unit(QT::Pressure, "hPa")));
    let (ppm, n_l) = match (given, temp, pres) {
        (Some(p), None, None) => (p, None),
        (None, Some(t), Some(p)) => {
            if !(-40.0..=60.0).contains(&t) || !(500.0..=1100.0).contains(&p) {
                return Err(ToolError::invalid(
                    "/temperature",
                    "Give survey conditions: -40 to 60 °C and 500 to 1,100 hPa.",
                ));
            }
            let lambda = ctx.number("wavelength")?.ok_or_else(|| {
                ToolError::invalid(
                    "/wavelength",
                    "Give the instrument's carrier wavelength from its manual, like 0.658 µm.",
                )
            })?;
            let n_ref = ctx.number("reference_refractivity")?.ok_or_else(|| {
                ToolError::invalid("/reference_refractivity", "Give the instrument's reference refractivity N_ref from its manual, like 286.34.")
            })?;
            let e = match ctx.number("humidity")? {
                Some(h) => h / 100.0 * saturation(t),
                None => {
                    ctx.warnings.push(Warning::new("DRY_AIR_ASSUMED", "No humidity was given, so the air is taken as dry; humidity changes the result by under 1 ppm in most conditions."));
                    0.0
                }
            };
            let nl = ambient_refractivity(group_refractivity(lambda), t, p, e);
            (n_ref - nl, Some(nl))
        }
        (None, None, None) => {
            return Err(ToolError::invalid(
                "/ppm",
                "Give the ppm correction, or the temperature and pressure to work it out.",
            ));
        }
        _ => {
            return Err(ToolError::invalid(
                "/ppm",
                "Give either the ppm correction or the temperature and pressure (both), not a mix.",
            ));
        }
    };
    let atm = ppm * 1e-6 * d;
    let corrected = d + atm + prism;
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, dp: u8| display::number(x, Precision::Decimals(dp), fmt);
        ctx.step(
            "Atmospheric correction",
            "ppm × 10⁻⁶ × measured distance",
            format!("{} × 10⁻⁶ × {}", n(ppm, 2), n(d, 4)),
            format!("{} m", n(atm, 4)),
        );
        ctx.step(
            "Corrected distance",
            "measured + atmospheric + prism",
            format!("{} + {} + {}", n(d, 4), n(atm, 4), n(prism, 4)),
            display::quantity(corrected, "m", Precision::Decimals(4), fmt),
        );
    }
    let q = |v: f64| Q { value: v, unit: m };
    let mut out = vec![
        ("corrected", ctx.out("corrected", q(corrected))),
        ("ppm", Json::Num(ppm)),
        (
            "atmospheric_correction",
            ctx.out("atmospheric_correction", q(atm)),
        ),
        ("prism_correction", ctx.out("prism_correction", q(prism))),
    ];
    if let Some(nl) = n_l {
        out.push(("refractivity", Json::Num(nl)));
    }
    Ok(Json::obj(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 0.658 µm red laser referenced to 12 °C, 1013.25 hPa, and 60%
    /// humidity has N_ref ≈ 286.34, the value such instruments print.
    #[test]
    fn reference_refractivity_of_a_red_laser() {
        let ng = group_refractivity(0.658);
        let e = 0.6 * saturation(12.0);
        let n_ref = ambient_refractivity(ng, 12.0, 1013.25, e);
        assert!((n_ref - 286.34).abs() < 0.01, "{n_ref}");
        // Warmer, thinner air slows light less: a positive correction.
        assert!(n_ref - ambient_refractivity(ng, 30.0, 950.0, e) > 0.0);
    }
}
