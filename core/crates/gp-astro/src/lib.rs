//! Shared astronomy for the time and navigation modules (add-celestial-and-
//! moon, design C1): the IAU 2006/2000A precession-nutation quantities and
//! sidereal time from the IERS Conventions (2010) tables, and the apparent
//! places of the Sun and the navigational planets from VSOP87, of the Moon
//! from ELP 2000-82B, and of the navigational stars from Hipparcos. Library only.

pub mod apparent;
mod elp_tables;
mod iers_tables;
pub mod moon;
pub mod nutation;
pub mod sidereal;
mod star_tables;
pub mod stars;
mod vsop_tables;
