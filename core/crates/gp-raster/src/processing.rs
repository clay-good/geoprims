//! Bounded per-window raster processing.
//!
//! Callers decode matching windows, run one kernel, and release those windows
//! before advancing. This keeps scene size out of the memory bound while the
//! same arithmetic serves local files in the web and MCP hosts.

use crate::geotiff::{GridMismatch, Metadata, Raster, same_grid};
use crate::indices::normalized_value;

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    GridMismatch(GridMismatch),
    InvalidBand { band: usize, bands: u16 },
    InvalidRaster(&'static str),
}

fn validate(raster: &Raster, band: usize) -> Result<usize, Error> {
    let bands = usize::from(raster.metadata.bands);
    if band >= bands {
        return Err(Error::InvalidBand {
            band,
            bands: raster.metadata.bands,
        });
    }
    let pixels = u64::from(raster.metadata.grid.width)
        .checked_mul(u64::from(raster.metadata.grid.height))
        .and_then(|n| usize::try_from(n).ok())
        .ok_or(Error::InvalidRaster("Raster dimensions overflow."))?;
    let samples = pixels
        .checked_mul(bands)
        .ok_or(Error::InvalidRaster("Raster sample count overflow."))?;
    if raster.samples.len() != samples || raster.valid.len() != samples {
        return Err(Error::InvalidRaster(
            "Raster samples do not match its dimensions and band count.",
        ));
    }
    Ok(pixels)
}

/// Computes `(a - b) / (a + b)` for one pair of matching raster windows.
///
/// Band indexes are zero-based. The output uses the left raster as its target
/// grid and preserves its no-data sentinel. If neither input declares a
/// sentinel, invalid output samples are stored as NaN and identified by the
/// `valid` mask.
pub fn normalized_difference(
    left: &Raster,
    left_band: usize,
    right: &Raster,
    right_band: usize,
) -> Result<Raster, Error> {
    same_grid(&left.metadata.grid, &right.metadata.grid).map_err(Error::GridMismatch)?;
    let pixels = validate(left, left_band)?;
    if validate(right, right_band)? != pixels {
        return Err(Error::InvalidRaster(
            "Matching raster grids must have the same pixel count.",
        ));
    }

    let left_bands = usize::from(left.metadata.bands);
    let right_bands = usize::from(right.metadata.bands);
    let fill = left.metadata.nodata.unwrap_or(f64::NAN);
    let mut samples = Vec::with_capacity(pixels);
    let mut valid = Vec::with_capacity(pixels);
    for pixel in 0..pixels {
        let left_index = pixel * left_bands + left_band;
        let right_index = pixel * right_bands + right_band;
        let value = if left.valid[left_index] && right.valid[right_index] {
            normalized_value(left.samples[left_index], right.samples[right_index])
                .filter(|value| value.is_finite())
        } else {
            None
        };
        samples.push(value.unwrap_or(fill));
        valid.push(value.is_some());
    }

    Ok(Raster {
        metadata: Metadata {
            grid: left.metadata.grid.clone(),
            bands: 1,
            nodata: left.metadata.nodata,
        },
        samples,
        valid,
    })
}
