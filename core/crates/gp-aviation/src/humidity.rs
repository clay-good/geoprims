//! Humidity and moist air (add-aviation-suite, atmosphere, "Humidity and
//! moist-air effects"): dew point, relative humidity, and vapor pressure from
//! each other by the Magnus form with Alduchov and Eskridge (1996)
//! coefficients, the mixing ratio, virtual temperature, and air density with
//! and without the water vapor.

use crate::atmosphere as isa;
use crate::{kelvin, obj, pa, unit};
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

const ALDUCHOV: Reference = Reference {
    title: "Improved Magnus Form Approximation of Saturation Vapor Pressure",
    issuer: "Alduchov, O. A., and R. E. Eskridge, Journal of Applied Meteorology",
    year: 1996,
    edition: "Vol. 35, No. 4",
    locator: "pp. 601-609, equation 21 (AERK coefficients 6.1094 hPa, 17.625, 243.04 °C; within 0.4% from -40 to 50 °C)",
    url: "https://doi.org/10.1175/1520-0450(1996)035%3C0601:IMFAOS%3E2.0.CO;2",
};

const A: f64 = 17.625;
const B: f64 = 243.04;
/// Ratio of the gas constants of water vapor and dry air, Rd / Rv.
const EPS: f64 = 0.622;

/// Dew point (°C) for a vapor pressure (Pa), the Magnus form inverted.
pub fn dew_point(e: f64) -> f64 {
    let g = log(e / 610.94);
    B * g / (A - g)
}

pub static HUMIDITY: ToolDef = ToolDef {
    id: "aviation.atmosphere.humidity",
    version: "1.0.1",
    title: "Humidity and moist air",
    summary: "Dew point, relative humidity, and vapor pressure from each other, with the mixing ratio, virtual temperature, and how much lighter the moist air is than dry air.",
    aliases: &[
        "relative humidity calculator",
        "dew point calculator",
        "vapor pressure",
        "virtual temperature",
        "moist air density",
    ],
    keywords: &[
        "humidity",
        "dew point",
        "relative humidity",
        "vapor pressure",
        "mixing ratio",
        "virtual temperature",
        "density",
        "Magnus",
    ],
    inputs: &[
        qty(
            "temperature",
            "Temperature",
            "Like 30 degC",
            QT::Temperature,
            "degC",
        )
        .required()
        .core(),
        qty(
            "dew_point",
            "Dew point",
            "Like 24 degC; or give the relative humidity",
            QT::Temperature,
            "degC",
        )
        .core(),
        Field::new(
            "relative_humidity",
            "Relative humidity",
            "Percent, like 70; or give the dew point",
            Kind::Number {
                min: 0.0,
                max: 100.0,
            },
        )
        .core(),
        qty(
            "pressure",
            "Air pressure",
            "Station pressure, like 1000 hPa (1013.25 by default)",
            QT::Pressure,
            "hPa",
        )
        .core(),
    ],
    outputs: &[
        qty(
            "moist_density",
            "Moist air density",
            "p ÷ (Rd × virtual temperature)",
            QT::Density,
            "kg/m3",
        )
        .precision(Precision::Significant(5)),
        qty(
            "dry_density",
            "Dry air density",
            "p ÷ (Rd × T), for comparison",
            QT::Density,
            "kg/m3",
        )
        .precision(Precision::Significant(5)),
        qty(
            "dew_point",
            "Dew point",
            "From the vapor pressure",
            QT::Temperature,
            "degC",
        )
        .precision(Precision::Decimals(1)),
        Field::new(
            "relative_humidity",
            "Relative humidity",
            "Percent: vapor pressure ÷ saturation vapor pressure",
            Kind::Number {
                min: 0.0,
                max: 100.0,
            },
        )
        .precision(Precision::Decimals(1)),
        qty(
            "vapor_pressure",
            "Vapor pressure",
            "The Magnus form at the dew point",
            QT::Pressure,
            "hPa",
        )
        .precision(Precision::Decimals(2)),
        qty(
            "saturation_vapor_pressure",
            "Saturation vapor pressure",
            "The Magnus form at the temperature",
            QT::Pressure,
            "hPa",
        )
        .precision(Precision::Decimals(2)),
        Field::new(
            "mixing_ratio",
            "Mixing ratio",
            "Grams of water vapor per kilogram of dry air",
            Kind::Number { min: 0.0, max: 1e6 },
        )
        .measure("mixing_ratio", "g/kg")
        .precision(Precision::Decimals(2)),
        qty(
            "virtual_temperature",
            "Virtual temperature",
            "The dry-air temperature with the same density",
            QT::Temperature,
            "degC",
        )
        .precision(Precision::Decimals(2)),
    ],
    errors: &[ErrorCode::InvalidInput],
    warnings: &["UNIT_ASSUMED", "EXPERIMENTAL_TOOL"],
    model: "e_s(T) = 6.1094 hPa × exp(17.625·T ÷ (T + 243.04)) (Alduchov and Eskridge 1996, over water); e = e_s(dew point) or RH × e_s(T); dew point by inverting the same form; mixing ratio = 622 × e ÷ (p − e) g/kg; virtual temperature T ÷ (1 − 0.378·e ÷ p); densities p ÷ (287.05287 × T)",
    accuracy: "The saturation pressure is within 0.4% from -40 °C to 50 °C, over liquid water; below freezing over ice it reads high. The densities follow from the ideal-gas law",
    references: &[ALDUCHOV, crate::refs::ICAO_7488],
    examples: &[Example {
        id: "primary",
        title: "30 °C, dew point 24 °C, 1,000 hPa",
        input: r#"{"temperature":"30 degC","dew_point":"24 degC","pressure":"1000 hPa"}"#,
        source: "add-aviation-suite humid-density scenario: moist air is lighter than dry air at the same temperature and pressure, and both are reported",
    }],
    primary_example: "primary",
    visualization: &[Layer {
        kind: "table-only",
        map: &[],
    }],
    related: &[
        Related {
            id: "aviation.altimetry.density-altitude",
            reason: "next",
        },
        Related {
            id: "aviation.atmosphere.cloud-base",
            reason: "alternative",
        },
    ],
    sentence: "Moist air here weighs {moist_density}, against {dry_density} if it were dry; relative humidity is {relative_humidity}%.",
    limits: &[("batchRows", 10_000)],
    run: run_humidity,
    ..ToolDef::BLANK
};

fn run_humidity(ctx: &mut Ctx) -> Result<Json, ToolError> {
    let c = unit(QT::Temperature, "degC");
    let t = ctx.req_quantity("temperature")?.to(c);
    if !(-40.0..=50.0).contains(&t) {
        return Err(ToolError::invalid(
            "/temperature",
            "The Magnus form holds from -40 °C to 50 °C; give a temperature in that range.",
        ));
    }
    let p = ctx.quantity("pressure")?.map_or(isa::P0, |q| q.base());
    if !(10_000.0..=110_000.0).contains(&p) {
        return Err(ToolError::invalid(
            "/pressure",
            "Give an air pressure between 100 and 1,100 hPa.",
        ));
    }
    let es = isa::vapor_pressure(t);
    let e = match (ctx.quantity("dew_point")?, ctx.number("relative_humidity")?) {
        (Some(td), None) => {
            let td = td.to(c);
            if td > t {
                return Err(ToolError::invalid(
                    "/dew_point",
                    "The dew point cannot be above the temperature.",
                ));
            }
            if td < -60.0 {
                return Err(ToolError::invalid(
                    "/dew_point",
                    "Give a dew point of -60 °C or more.",
                ));
            }
            isa::vapor_pressure(td)
        }
        (None, Some(rh)) if rh > 0.0 => rh / 100.0 * es,
        (None, Some(_)) => {
            return Err(ToolError::invalid(
                "/relative_humidity",
                "Relative humidity must be above 0%.",
            ));
        }
        _ => {
            return Err(ToolError::invalid(
                "/dew_point",
                "Give either the dew point or the relative humidity, not both.",
            ));
        }
    };
    let tk = t + 273.15;
    let tv = isa::virtual_temperature(tk, e, p);
    let dry = p / (isa::R * tk);
    let moist = p / (isa::R * tv);
    let w = 1000.0 * EPS * e / (p - e);
    if ctx.explaining() {
        let fmt = ctx.options.format;
        let n = move |x: f64, d: u8| display::number(x, Precision::Decimals(d), fmt);
        ctx.step(
            "Virtual temperature",
            "Tv = T ÷ (1 − 0.378 × e ÷ p)",
            format!(
                "{} K ÷ (1 − 0.378 × {} ÷ {})",
                n(tk, 2),
                n(e / 100.0, 2),
                n(p / 100.0, 2)
            ),
            format!("{} K", n(tv, 2)),
        );
        ctx.step(
            "Moist air density",
            "ρ = p ÷ (287.05287 × Tv)",
            format!("{} Pa ÷ (287.05287 × {} K)", n(p, 0), n(tv, 2)),
            display::quantity(moist, "kg/m3", Precision::Significant(5), fmt),
        );
    }
    let dens = |v: f64| Q {
        value: v,
        unit: unit(QT::Density, "kg/m3"),
    };
    Ok(obj(vec![
        ("moist_density", ctx.out("moist_density", dens(moist))),
        ("dry_density", ctx.out("dry_density", dens(dry))),
        (
            "dew_point",
            ctx.out("dew_point", kelvin(dew_point(e) + 273.15)),
        ),
        ("relative_humidity", Json::Num(100.0 * e / es)),
        ("vapor_pressure", ctx.out("vapor_pressure", pa(e))),
        (
            "saturation_vapor_pressure",
            ctx.out("saturation_vapor_pressure", pa(es)),
        ),
        ("mixing_ratio", Json::Num(w)),
        (
            "virtual_temperature",
            ctx.out("virtual_temperature", kelvin(tv)),
        ),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The NWS WxCalc vapor-pressure sheet uses the Tetens coefficients
    /// (6.11 × 10^(7.5T ÷ (237.3 + T)) hPa); the two agree within 0.5% from
    /// 0 to 50 °C, a cross-check on the constants (below freezing they part,
    /// by about 3% at -40 °C).
    #[test]
    fn agrees_with_the_nws_tetens_form() {
        for t in [0.0, 10.0, 20.0, 30.0, 40.0, 50.0] {
            let nws = 611.0 * libm::pow(10.0, 7.5 * t / (237.3 + t));
            let ours = isa::vapor_pressure(t);
            assert!((ours / nws - 1.0).abs() < 5e-3, "{t}: {ours} vs {nws}");
        }
    }

    #[test]
    fn dew_point_inverts_the_vapor_pressure() {
        for td in [-30.0, 0.0, 15.5, 24.0] {
            assert!((dew_point(isa::vapor_pressure(td)) - td).abs() < 1e-9);
        }
    }
}
