//! Germany's DHDN Gauss-Krüger to ETRS89 UTM and back, by AdV's BeTA2007
//! grid (add-international-grids, geodesy/national-grids, "Canada, Europe,
//! and the other national paths").
//!
//! BeTA2007 is the single national NTv2 grid AdV publishes for DHDN90 to
//! ETRS89, replacing the state-by-state transformations for ATKIS. The
//! coordinates either side are projected, not geographic: DHDN on the
//! three-degree Gauss-Krüger zones (EPSG 31466 to 31469, whose easting
//! carries the zone number in front of it) and ETRS89 on UTM (EPSG 25831 to
//! 25833).

use crate::hgrid::Grid;
use crate::proj;

/// Bessel 1841, which the Gauss-Krüger zones are defined on.
pub fn bessel() -> (f64, f64) {
    (6_377_397.155, 1.0 / 299.152_812_8)
}
/// GRS 80, which ETRS89 and its UTM zones are defined on.
pub fn grs80() -> (f64, f64) {
    (6_378_137.0, 1.0 / 298.257_222_101)
}

/// A three-degree Gauss-Krüger zone: central meridian 3n, scale 1, and a
/// false easting of n,500,000 m, so the easting states its own zone.
pub fn gk_grid(zone: u8) -> proj::TmGrid {
    let (a, f) = bessel();
    proj::TmGrid::new(
        a,
        f,
        0.0,
        3.0 * f64::from(zone),
        1.0,
        f64::from(zone) * 1_000_000.0 + 500_000.0,
        0.0,
    )
}

/// A UTM zone on GRS 80.
pub fn utm_grid(zone: u8) -> proj::TmGrid {
    let (a, f) = grs80();
    proj::TmGrid::new(
        a,
        f,
        0.0,
        6.0 * f64::from(zone) - 183.0,
        0.9996,
        500_000.0,
        0.0,
    )
}

/// The Gauss-Krüger zone an easting states, from the digits in front of the
/// 500,000 m false easting, or None if it states none in 2 to 5 or sits
/// further than 300 km from that zone's central meridian.
pub fn gk_zone_of(east: f64) -> Option<u8> {
    if !east.is_finite() || east < 1_000_000.0 {
        return None;
    }
    let zone = (east / 1_000_000.0).floor();
    if !(2.0..=5.0).contains(&zone) {
        return None;
    }
    let from_meridian = east - zone * 1_000_000.0 - 500_000.0;
    if from_meridian.abs() > 300_000.0 {
        return None;
    }
    Some(zone as u8)
}

/// The UTM zone a longitude falls in.
pub fn utm_zone_of(lon: f64) -> u8 {
    (((lon + 180.0) / 6.0).floor() as i32).clamp(0, 59) as u8 + 1
}

/// The Gauss-Krüger zone a longitude falls in, the nearest three-degree
/// meridian, clamped to the zones Germany uses.
pub fn gk_zone_for(lon: f64) -> u8 {
    ((lon / 3.0).round() as i32).clamp(2, 5) as u8
}

/// What one transformation needed to say for itself.
pub struct Step {
    /// The point on DHDN, degrees.
    pub dhdn: (f64, f64),
    /// The point on ETRS89, degrees.
    pub etrs89: (f64, f64),
    /// The grid's offsets, arcseconds, DHDN to ETRS89, east positive.
    pub offsets: (f64, f64),
    /// Iterations the grid took, 0 when it was read once.
    pub iterations: usize,
}

/// Gauss-Krüger easting and northing on its zone to UTM on `utm`. None
/// outside the grid.
pub fn to_utm(
    grid: &Grid,
    gk: u8,
    utm: u8,
    east: f64,
    north: f64,
) -> Option<((f64, f64), proj::Grid, Step)> {
    let (lat, lon) = gk_grid(gk).inverse(east, north);
    let offsets = grid.offsets(lat, lon)?;
    let (lat2, lon2) = grid.forward(lat, lon)?;
    let p = utm_grid(utm).forward(lat2, lon2);
    Some((
        (p.e, p.n),
        p,
        Step {
            dhdn: (lat, lon),
            etrs89: (lat2, lon2),
            offsets,
            iterations: 0,
        },
    ))
}

/// UTM easting and northing on its zone to Gauss-Krüger on `gk`. None
/// outside the grid.
pub fn to_gauss_kruger(
    grid: &Grid,
    utm: u8,
    gk: u8,
    east: f64,
    north: f64,
) -> Option<((f64, f64), proj::Grid, Step)> {
    let (lat, lon) = utm_grid(utm).inverse(east, north);
    let (lat2, lon2, iterations) = grid.inverse(lat, lon)?;
    let offsets = grid.offsets(lat2, lon2)?;
    let p = gk_grid(gk).forward(lat2, lon2);
    Some((
        (p.e, p.n),
        p,
        Step {
            dhdn: (lat2, lon2),
            etrs89: (lat, lon),
            offsets,
            iterations,
        },
    ))
}
