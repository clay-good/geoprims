//! Latitude and longitude inputs (numeric-determinism angle rules): latitude
//! validated to [-90, 90], longitude normalized to [-180, 180) with
//! `INPUT_NORMALIZED`. Each field accepts decimal degrees, a unit-tagged angle,
//! or a DMS/DDM string with hemisphere letters (`40°26'46"N`).

use gp_base::error::ToolError;
use gp_base::tool::{Ctx, Field, Kind};
use gp_base::units::{self, Quantity};

use crate::dms::{self, Axis};

pub const fn lat_field(name: &'static str, title: &'static str) -> Field {
    Field::new(
        name,
        title,
        "Decimal degrees, north positive, like 40.6413",
        Kind::Quantity {
            q: Quantity::Angle,
            unit: "deg",
        },
    )
    .required()
    .core()
    .angle_range("[-90,90]")
}

pub const fn lon_field(name: &'static str, title: &'static str) -> Field {
    Field::new(
        name,
        title,
        "Decimal degrees, east positive, like -73.7781",
        Kind::Quantity {
            q: Quantity::Angle,
            unit: "deg",
        },
    )
    .required()
    .core()
    .angle_range("[-180,180)")
}

/// Reads one angle field: a number or unit-tagged angle, or else DMS/DDM
/// notation with hemisphere letters.
fn angle(ctx: &mut Ctx, name: &str, axis: Axis) -> Result<f64, ToolError> {
    let deg = units::by_symbol(Quantity::Angle, "deg").expect("deg");
    match ctx.req_quantity(name) {
        Ok(q) => Ok(q.to(deg)),
        Err(unit_err) => {
            let Some(s) = ctx.raw(name).and_then(|v| v.as_str()).map(str::to_owned) else {
                return Err(unit_err);
            };
            dms::parse_angle(&s, axis).map_err(|e| {
                let looks_dms = s.chars().any(|c| {
                    matches!(c, '°' | '\'' | '"' | '′' | '″' | ':') || "NSEWnsew".contains(c)
                });
                if looks_dms {
                    ToolError::invalid(&format!("/{name}"), e)
                } else {
                    unit_err
                }
            })
        }
    }
}

/// Reads a (lat, lon) pair in degrees.
pub fn read(ctx: &mut Ctx, lat: &str, lon: &str) -> Result<(f64, f64), ToolError> {
    let la = angle(ctx, lat, Axis::Lat)?;
    let la = gp_base::angle::check_lat(la, &format!("/{lat}"))?;
    let lo = angle(ctx, lon, Axis::Lon)?;
    let (lo, w) = gp_base::angle::accept_lon(lo, &format!("/{lon}"))?;
    ctx.warnings.extend(w);
    Ok((la, lo))
}

/// Reads an angle field that may be a number, a unit-tagged angle, or plain
/// DMS/DDM like `30°00'00"` (no hemisphere): for deflections and azimuths.
pub fn plain_angle(ctx: &mut Ctx, name: &str) -> Result<Option<f64>, ToolError> {
    if !ctx.is_set(name) {
        return Ok(None);
    }
    let deg = units::by_symbol(Quantity::Angle, "deg").expect("deg");
    match ctx.quantity(name) {
        Ok(q) => Ok(q.map(|q| q.to(deg))),
        Err(unit_err) => match ctx.raw(name).and_then(|v| v.as_str()).map(str::to_owned) {
            Some(s) => dms::parse_plain(&s).map(Some).map_err(|_| unit_err),
            None => Err(unit_err),
        },
    }
}
