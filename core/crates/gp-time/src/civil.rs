//! Civil dates, instants, and leap seconds. The code lives in `gp_geo::civil`
//! so the GNSS planning tools share the same leap-second table; this module
//! re-exports it and keeps the issuer file the table was transcribed from.

pub use gp_geo::civil::*;

/// The issuer file recorded in the asset registry. Keeping it in the module
/// makes the exact table input available to both release surfaces.
pub static LEAP_TABLE_SOURCE: &[u8] = include_bytes!("../data/leap-seconds.list");
