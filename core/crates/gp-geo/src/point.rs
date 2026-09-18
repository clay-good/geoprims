//! Latitude and longitude inputs (numeric-determinism angle rules): latitude
//! validated to [-90, 90], longitude normalized to [-180, 180) with
//! `INPUT_NORMALIZED`. Coordinate notations (DMS, MGRS, UTM) arrive with the
//! geodesy suite's parser.

use gp_base::error::ToolError;
use gp_base::tool::{Ctx, Field, Kind};
use gp_base::units::{self, Quantity};

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

/// Reads a (lat, lon) pair in degrees.
pub fn read(ctx: &mut Ctx, lat: &str, lon: &str) -> Result<(f64, f64), ToolError> {
    let deg = units::by_symbol(Quantity::Angle, "deg").expect("deg");
    let la = ctx.req_quantity(lat)?.to(deg);
    let la = gp_base::angle::check_lat(la, &format!("/{lat}"))?;
    let lo = ctx.req_quantity(lon)?.to(deg);
    let (lo, w) = gp_base::angle::accept_lon(lo, &format!("/{lon}"))?;
    ctx.warnings.extend(w);
    Ok((la, lo))
}
