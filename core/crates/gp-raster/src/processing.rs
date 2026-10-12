//! Bounded per-window raster processing.
//!
//! Callers decode matching windows, run one kernel, and release those windows
//! before advancing. This keeps scene size out of the memory bound while the
//! same arithmetic serves local files in the web and MCP hosts.

use std::io::{Read, Seek, Write};

use tiff::encoder::{TiffEncoder, colortype};

use crate::geotiff::{
    self, Grid, GridMismatch, Metadata, Raster, RasterType, Window, WindowDecoder, same_grid,
};
use crate::indices::normalized_value;

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    Decode(geotiff::Error),
    GridMismatch(GridMismatch),
    InvalidBand { band: usize, bands: u16 },
    InvalidRaster(&'static str),
    InvalidWindowSize,
    LimitExceeded { samples: u64, limit: usize },
    Export(geotiff::Error),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resampling {
    Nearest,
    Bilinear,
}

fn validate_band(metadata: &Metadata, band: usize) -> Result<(), Error> {
    if band >= usize::from(metadata.bands) {
        return Err(Error::InvalidBand {
            band,
            bands: metadata.bands,
        });
    }
    Ok(())
}

fn validate(raster: &Raster, band: usize) -> Result<usize, Error> {
    if raster.metadata.grid.width == 0 || raster.metadata.grid.height == 0 {
        return Err(Error::InvalidRaster(
            "Raster dimensions must be greater than zero.",
        ));
    }
    validate_band(&raster.metadata, band)?;
    let bands = usize::from(raster.metadata.bands);
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

/// Processes a complete pair of matching TIFFs in bounded row-major windows.
/// The callback owns each output window and can export, summarize, or discard
/// it before the next window is allocated.
#[allow(clippy::too_many_arguments)]
pub fn normalized_difference_scene<L, R, F>(
    left_reader: L,
    left_band: usize,
    right_reader: R,
    right_band: usize,
    window_width: u32,
    window_height: u32,
    max_samples: usize,
    accept: F,
) -> Result<(), Error>
where
    L: Read + Seek,
    R: Read + Seek,
    F: FnMut(Window, Raster) -> Result<(), Error>,
{
    let mut left = WindowDecoder::new(left_reader, max_samples).map_err(Error::Decode)?;
    let mut right = WindowDecoder::new(right_reader, max_samples).map_err(Error::Decode)?;
    process_scene(
        &mut left,
        left_band,
        &mut right,
        right_band,
        window_width,
        window_height,
        accept,
    )
}

/// Decodes and processes one window from two private local-file handles.
pub fn normalized_difference_file_window(
    left_handle: &str,
    left_band: usize,
    right_handle: &str,
    right_band: usize,
    window: Window,
    max_samples: usize,
) -> Result<Raster, Error> {
    let missing = || {
        Error::Decode(geotiff::Error::Invalid(
            "The selected local file is no longer loaded.".into(),
        ))
    };
    let left_reader = gp_base::files::reader(left_handle).ok_or_else(missing)?;
    let right_reader = gp_base::files::reader(right_handle).ok_or_else(missing)?;
    let mut left = WindowDecoder::new(left_reader, max_samples).map_err(Error::Decode)?;
    let mut right = WindowDecoder::new(right_reader, max_samples).map_err(Error::Decode)?;
    same_grid(&left.metadata().grid, &right.metadata().grid).map_err(Error::GridMismatch)?;
    validate_band(left.metadata(), left_band)?;
    validate_band(right.metadata(), right_band)?;
    let left = left.decode(window).map_err(Error::Decode)?;
    let right = right.decode(window).map_err(Error::Decode)?;
    normalized_difference(&left, left_band, &right, right_band)
}

#[allow(clippy::too_many_arguments)]
fn process_scene<L, R, F>(
    left: &mut WindowDecoder<L>,
    left_band: usize,
    right: &mut WindowDecoder<R>,
    right_band: usize,
    window_width: u32,
    window_height: u32,
    mut accept: F,
) -> Result<(), Error>
where
    L: Read + Seek,
    R: Read + Seek,
    F: FnMut(Window, Raster) -> Result<(), Error>,
{
    if window_width == 0 || window_height == 0 {
        return Err(Error::InvalidWindowSize);
    }
    same_grid(&left.metadata().grid, &right.metadata().grid).map_err(Error::GridMismatch)?;
    validate_band(left.metadata(), left_band)?;
    validate_band(right.metadata(), right_band)?;
    let grid = left.metadata().grid.clone();

    let mut y = 0;
    while y < grid.height {
        let height = window_height.min(grid.height - y);
        let mut x = 0;
        while x < grid.width {
            let width = window_width.min(grid.width - x);
            let window = Window {
                x,
                y,
                width,
                height,
            };
            let left_window = left.decode(window).map_err(Error::Decode)?;
            let right_window = right.decode(window).map_err(Error::Decode)?;
            let output = normalized_difference(&left_window, left_band, &right_window, right_band)?;
            accept(window, output)?;
            x += width;
        }
        y += height;
    }
    Ok(())
}

/// Writes a complete normalized-difference result to a seekable GeoTIFF sink,
/// one full-width strip at a time. The returned writer is available only after
/// every strip and the final directory have been written successfully.
#[allow(clippy::too_many_arguments)]
pub fn write_normalized_difference_geotiff<L, R, W>(
    left_reader: L,
    left_band: usize,
    right_reader: R,
    right_band: usize,
    rows_per_strip: u32,
    max_samples: usize,
    mut output: W,
) -> Result<W, Error>
where
    L: Read + Seek,
    R: Read + Seek,
    W: Write + Seek,
{
    if rows_per_strip == 0 {
        return Err(Error::InvalidWindowSize);
    }
    let mut left = WindowDecoder::new(left_reader, max_samples).map_err(Error::Decode)?;
    let mut right = WindowDecoder::new(right_reader, max_samples).map_err(Error::Decode)?;
    same_grid(&left.metadata().grid, &right.metadata().grid).map_err(Error::GridMismatch)?;
    validate_band(left.metadata(), left_band)?;
    validate_band(right.metadata(), right_band)?;
    let grid = left.metadata().grid.clone();
    let nodata = Some(left.metadata().nodata.unwrap_or(f64::NAN));
    let metadata = Metadata {
        grid: grid.clone(),
        bands: 1,
        nodata,
    };

    {
        let mut encoder = TiffEncoder::new(&mut output)
            .map_err(geotiff::invalid)
            .map_err(Error::Export)?;
        let mut image = encoder
            .new_image::<colortype::Gray64Float>(grid.width, grid.height)
            .map_err(geotiff::invalid)
            .map_err(Error::Export)?;
        image
            .rows_per_strip(rows_per_strip)
            .map_err(geotiff::invalid)
            .map_err(Error::Export)?;
        geotiff::write_metadata_tags(&mut image, &metadata, nodata).map_err(Error::Export)?;
        process_scene(
            &mut left,
            left_band,
            &mut right,
            right_band,
            grid.width,
            rows_per_strip,
            |_, raster| {
                geotiff::validate_nodata(&raster, nodata).map_err(Error::Export)?;
                image
                    .write_strip(&raster.samples)
                    .map_err(geotiff::invalid)
                    .map_err(Error::Export)
            },
        )?;
        image
            .finish()
            .map_err(geotiff::invalid)
            .map_err(Error::Export)?;
    }
    Ok(output)
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

fn raster_offset(raster_type: Option<RasterType>) -> f64 {
    match raster_type {
        Some(RasterType::PixelIsPoint) => 0.0,
        Some(RasterType::PixelIsArea) | None => 0.5,
    }
}

/// Resamples every band onto a chosen target grid in the same CRS.
pub fn resample(
    source: &Raster,
    target: Grid,
    method: Resampling,
    max_samples: usize,
) -> Result<Raster, Error> {
    validate(source, 0)?;
    if target.width == 0 || target.height == 0 {
        return Err(Error::InvalidRaster(
            "Target grid dimensions must be greater than zero.",
        ));
    }
    if source.metadata.grid.epsg != target.epsg || source.metadata.grid.crs_type != target.crs_type
    {
        return Err(Error::GridMismatch(GridMismatch {
            reason: "CRS",
            left: Box::new(source.metadata.grid.clone()),
            right: Box::new(target),
        }));
    }
    let source_transform = source
        .metadata
        .grid
        .transform
        .ok_or(Error::InvalidRaster("Source grid has no affine transform."))?;
    let target_transform = target
        .transform
        .ok_or(Error::InvalidRaster("Target grid has no affine transform."))?;
    let determinant =
        source_transform.dx * source_transform.dy - source_transform.rx * source_transform.ry;
    if determinant == 0.0 || !determinant.is_finite() {
        return Err(Error::InvalidRaster(
            "Source grid affine transform is not invertible.",
        ));
    }
    let count = u64::from(target.width)
        .checked_mul(u64::from(target.height))
        .and_then(|n| n.checked_mul(u64::from(source.metadata.bands)))
        .ok_or(Error::LimitExceeded {
            samples: u64::MAX,
            limit: max_samples,
        })?;
    if count > max_samples as u64 {
        return Err(Error::LimitExceeded {
            samples: count,
            limit: max_samples,
        });
    }

    let bands = usize::from(source.metadata.bands);
    let fill = source.metadata.nodata.unwrap_or(f64::NAN);
    let mut samples = Vec::with_capacity(count as usize);
    let mut valid = Vec::with_capacity(count as usize);
    let target_offset = raster_offset(target.raster_type);
    let source_offset = raster_offset(source.metadata.grid.raster_type);
    for row in 0..target.height {
        for column in 0..target.width {
            let target_x = f64::from(column) + target_offset;
            let target_y = f64::from(row) + target_offset;
            let model_x = target_transform.x0
                + target_x * target_transform.dx
                + target_y * target_transform.rx;
            let model_y = target_transform.y0
                + target_x * target_transform.ry
                + target_y * target_transform.dy;
            let delta_x = model_x - source_transform.x0;
            let delta_y = model_y - source_transform.y0;
            let source_x = (source_transform.dy * delta_x - source_transform.rx * delta_y)
                / determinant
                - source_offset;
            let source_y = (-source_transform.ry * delta_x + source_transform.dx * delta_y)
                / determinant
                - source_offset;
            for band in 0..bands {
                let value = match method {
                    Resampling::Nearest => nearest(source, source_x, source_y, band),
                    Resampling::Bilinear => bilinear(source, source_x, source_y, band),
                };
                samples.push(value.unwrap_or(fill));
                valid.push(value.is_some());
            }
        }
    }
    Ok(Raster {
        metadata: Metadata {
            grid: target,
            bands: source.metadata.bands,
            nodata: source.metadata.nodata,
        },
        samples,
        valid,
    })
}

fn source_sample(source: &Raster, x: u32, y: u32, band: usize) -> Option<f64> {
    let bands = usize::from(source.metadata.bands);
    let pixel =
        usize::try_from(u64::from(y) * u64::from(source.metadata.grid.width) + u64::from(x))
            .ok()?;
    let index = pixel.checked_mul(bands)?.checked_add(band)?;
    (source.valid[index] && source.samples[index].is_finite()).then_some(source.samples[index])
}

fn nearest(source: &Raster, x: f64, y: f64, band: usize) -> Option<f64> {
    if !x.is_finite() || !y.is_finite() {
        return None;
    }
    let x = x.round();
    let y = y.round();
    if x < 0.0
        || y < 0.0
        || x >= f64::from(source.metadata.grid.width)
        || y >= f64::from(source.metadata.grid.height)
    {
        return None;
    }
    source_sample(source, x as u32, y as u32, band)
}

fn bilinear(source: &Raster, x: f64, y: f64, band: usize) -> Option<f64> {
    if !x.is_finite()
        || !y.is_finite()
        || x < 0.0
        || y < 0.0
        || x > f64::from(source.metadata.grid.width.saturating_sub(1))
        || y > f64::from(source.metadata.grid.height.saturating_sub(1))
    {
        return None;
    }
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(source.metadata.grid.width - 1);
    let y1 = (y0 + 1).min(source.metadata.grid.height - 1);
    let x_weight = x - f64::from(x0);
    let y_weight = y - f64::from(y0);
    let top = source_sample(source, x0, y0, band)? * (1.0 - x_weight)
        + source_sample(source, x1, y0, band)? * x_weight;
    let bottom = source_sample(source, x0, y1, band)? * (1.0 - x_weight)
        + source_sample(source, x1, y1, band)? * x_weight;
    Some(top * (1.0 - y_weight) + bottom * y_weight)
}
