//! RDNAPTRANS™2018: ETRS89 to the Netherlands' Rijksdriehoeksmeting (RD)
//! coordinates and Normaal Amsterdams Peil (NAP) heights, and back, as NSGI
//! defines them (RDNAPTRANS™2018, 27 June 2022).
//!
//! The horizontal path is NSGI's implementation variant 2: a correction grid
//! of latitude and longitude offsets, indexed by real RD Bessel coordinates,
//! that carries real Bessel to ETRS89 and so carries the datum difference and
//! the correction together; and then the RD map projection, an oblique
//! stereographic double projection on Bessel 1841 (gp_geo::proj::Sterea).
//! ETRS89 to RD therefore iterates the grid, and RD to ETRS89 reads it once,
//! which is what sections 2.3.3 and 3.2.1 prescribe.
//!
//! The vertical path is variant 1: the NLGEO2018 quasi-geoid, indexed by
//! ETRS89 coordinates, with H = h − N (section 2.5.2).

use crate::hgrid::Grids;
use crate::proj::{Grid, Sterea};
use crate::vgrid;

/// The RD map projection: Bessel 1841, the central point Amersfoort at
/// 52°9′22.178″N 5°23′15.500″E, scale 0.9999079, false origin
/// 155,000 m east and 463,000 m north (RDNAPTRANS™2018, section 2.4.1; the
/// same parameters EPSG registers for 28992).
pub fn projection() -> Sterea {
    Sterea::new(
        6_377_397.155,
        1.0 / 299.152_812_8,
        52.0 + 9.0 / 60.0 + 22.178 / 3600.0,
        5.0 + 23.0 / 60.0 + 15.5 / 3600.0,
        0.999_907_9,
        155_000.0,
        463_000.0,
    )
}

/// What one transformation needed to say for itself.
pub struct Step {
    /// The point in real RD Bessel, degrees.
    pub bessel: (f64, f64),
    /// The correction grid's offsets there, arcseconds, east positive.
    pub offsets: (f64, f64),
    /// Iterations the grid took, 0 when it was read once.
    pub iterations: usize,
}

/// ETRS89 latitude and longitude (degrees) to RD, with the projection's
/// factors and the steps taken. None outside the correction grid.
pub fn to_rd(grids: &Grids, lat: f64, lon: f64) -> Option<(Grid, Step)> {
    let (blat, blon, iterations) = grids.inverse(lat, lon)?;
    let offsets = grids.offsets(blat, blon)?;
    Some((
        projection().forward(blat, blon),
        Step {
            bessel: (blat, blon),
            offsets,
            iterations,
        },
    ))
}

/// RD easting and northing (m) to ETRS89 latitude and longitude (degrees),
/// with the steps taken. None outside the correction grid.
pub fn to_etrs89(grids: &Grids, x: f64, y: f64) -> Option<((f64, f64), Step)> {
    let (blat, blon) = projection().inverse(x, y);
    let offsets = grids.offsets(blat, blon)?;
    let (lat, lon) = grids.forward(blat, blon)?;
    Some((
        (lat, lon),
        Step {
            bessel: (blat, blon),
            offsets,
            iterations: 0,
        },
    ))
}

/// The NLGEO2018 quasi-geoid height (m) at an ETRS89 latitude and longitude,
/// or None outside the grid.
pub fn quasi_geoid(grid: &vgrid::Grid, lat: f64, lon: f64) -> Option<f64> {
    grid.at(lat, lon)
}
