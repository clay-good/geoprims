//! LV03 to LV95 and back by swisstopo's CHENyx06 (add-international-grids,
//! geodesy/national-grids, "Canada, Europe, and the other national paths").
//!
//! The Swiss triangulation of 1903 was bent by the error propagation of its
//! own measurements, so LV95 is not LV03 shifted by a round 2,000 km: the
//! difference reaches 1.6 m. swisstopo publishes the difference as CHENyx06
//! and transforms with FINELTRA, a triangulated affine transformation on the
//! projection plane; its own documentation offers a grid approximation of the
//! same differences beside it, which is what this applies, through
//! gp_geo::hgrid on the geographic coordinates between the two projections.
//!
//! LV03 and LV95 are the same Swiss oblique Mercator with different false
//! origins, so LV03 is projected and unprojected at LV95's origin with the
//! round 2,000 km taken off at the end, which is exact.

use crate::hgrid::Grid;
use crate::proj::swiss;

/// The round offset between the two grids' false origins.
pub const LV03_EAST: f64 = 2_000_000.0;
pub const LV03_NORTH: f64 = 1_000_000.0;

/// What one transformation needed to say for itself.
pub struct Step {
    /// The point on CH1903, degrees, before the grid.
    pub from: (f64, f64),
    /// The point on CH1903+, degrees, after it.
    pub to: (f64, f64),
    /// The grid's offsets, arcseconds, CH1903 to CH1903+, east positive.
    pub offsets: (f64, f64),
    /// Iterations the grid took, 0 when it was read once.
    pub iterations: usize,
}

/// LV03 easting and northing (m) to LV95. None outside the grid.
pub fn to_lv95(grid: &Grid, east: f64, north: f64) -> Option<((f64, f64), Step)> {
    let (lat, lon) = swiss::inverse(east + LV03_EAST, north + LV03_NORTH);
    let (lat, lon) = (lat.to_degrees(), lon.to_degrees());
    let offsets = grid.offsets(lat, lon)?;
    let (lat2, lon2) = grid.forward(lat, lon)?;
    Some((
        swiss::forward(lat2.to_radians(), lon2.to_radians()),
        Step {
            from: (lat, lon),
            to: (lat2, lon2),
            offsets,
            iterations: 0,
        },
    ))
}

/// LV95 easting and northing (m) to LV03. None outside the grid.
pub fn to_lv03(grid: &Grid, east: f64, north: f64) -> Option<((f64, f64), Step)> {
    let (lat, lon) = swiss::inverse(east, north);
    let (lat, lon) = (lat.to_degrees(), lon.to_degrees());
    let (lat2, lon2, iterations) = grid.inverse(lat, lon)?;
    let offsets = grid.offsets(lat2, lon2)?;
    let (e, n) = swiss::forward(lat2.to_radians(), lon2.to_radians());
    Some((
        (e - LV03_EAST, n - LV03_NORTH),
        Step {
            from: (lat2, lon2),
            to: (lat, lon),
            offsets,
            iterations,
        },
    ))
}
