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
use crate::indices::{dnbr_value, evi_value, evi2_value, normalized_value, savi_value};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetGrid {
    Left,
    Right,
}

/// A fixed 2-band index formula exposed by both scalar and raster tools.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kernel {
    NormalizedDifference,
    Evi,
    Evi2,
    Savi { soil_factor: f64 },
    Dnbr,
}

impl Kernel {
    fn value(self, values: &[f64]) -> Option<f64> {
        match self {
            Self::NormalizedDifference => normalized_value(values[0], values[1]),
            Self::Evi => evi_value(values[0], values[1], values[2]),
            Self::Evi2 => evi2_value(values[0], values[1]),
            Self::Savi { soil_factor } => savi_value(values[0], values[1], soil_factor),
            Self::Dnbr
                if (-1.0..=1.0).contains(&values[0]) && (-1.0..=1.0).contains(&values[1]) =>
            {
                Some(dnbr_value(values[0], values[1]))
            }
            Self::Dnbr => None,
        }
    }

    fn bands(self) -> usize {
        match self {
            Self::Evi => 3,
            _ => 2,
        }
    }
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
    index_file_window(
        left_handle,
        left_band,
        right_handle,
        right_band,
        window,
        max_samples,
        Kernel::NormalizedDifference,
    )
}

/// Decodes and processes one 2-band index window from private local-file handles.
#[allow(clippy::too_many_arguments)]
pub fn index_file_window(
    left_handle: &str,
    left_band: usize,
    right_handle: &str,
    right_band: usize,
    window: Window,
    max_samples: usize,
    kernel: Kernel,
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
    index(&left, left_band, &right, right_band, kernel)
}

/// Decodes and processes one fixed-formula index window from ordered private
/// local-file band sources.
pub fn index_file_sources_window(
    sources: &[(&str, usize)],
    window: Window,
    max_samples: usize,
    kernel: Kernel,
) -> Result<Raster, Error> {
    if sources.len() != kernel.bands() {
        return Err(Error::InvalidRaster(
            "The raster kernel received the wrong number of band sources.",
        ));
    }
    let missing = || {
        Error::Decode(geotiff::Error::Invalid(
            "The selected local file is no longer loaded.".into(),
        ))
    };
    let mut decoders = sources
        .iter()
        .map(|(handle, _)| {
            WindowDecoder::new(
                gp_base::files::reader(handle).ok_or_else(missing)?,
                max_samples,
            )
            .map_err(Error::Decode)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let target = decoders[0].metadata().grid.clone();
    for (decoder, (_, band)) in decoders.iter().zip(sources) {
        same_grid(&target, &decoder.metadata().grid).map_err(Error::GridMismatch)?;
        validate_band(decoder.metadata(), *band)?;
    }
    let rasters = decoders
        .iter_mut()
        .map(|decoder| decoder.decode(window).map_err(Error::Decode))
        .collect::<Result<Vec<_>, _>>()?;
    let bands = rasters
        .iter()
        .zip(sources)
        .map(|(raster, (_, band))| (raster, *band))
        .collect::<Vec<_>>();
    index_sources(&bands, kernel)
}

/// Decodes one target window and only the source pixels needed to resample the
/// other input onto it. The normalized-difference output inherits the chosen
/// target's grid and no-data value.
#[allow(clippy::too_many_arguments)]
pub fn normalized_difference_resampled_window<L, R>(
    left_reader: L,
    left_band: usize,
    right_reader: R,
    right_band: usize,
    window: Window,
    max_samples: usize,
    method: Resampling,
    target: TargetGrid,
) -> Result<Raster, Error>
where
    L: Read + Seek,
    R: Read + Seek,
{
    index_resampled_window(
        left_reader,
        left_band,
        right_reader,
        right_band,
        window,
        max_samples,
        method,
        target,
        Kernel::NormalizedDifference,
    )
}

/// Decodes a target window and only the source pixels required to evaluate a
/// fixed 2-band kernel after local resampling.
#[allow(clippy::too_many_arguments)]
pub fn index_resampled_window<L, R>(
    left_reader: L,
    left_band: usize,
    right_reader: R,
    right_band: usize,
    window: Window,
    max_samples: usize,
    method: Resampling,
    target: TargetGrid,
    kernel: Kernel,
) -> Result<Raster, Error>
where
    L: Read + Seek,
    R: Read + Seek,
{
    let mut left = WindowDecoder::new(left_reader, max_samples).map_err(Error::Decode)?;
    let mut right = WindowDecoder::new(right_reader, max_samples).map_err(Error::Decode)?;
    validate_band(left.metadata(), left_band)?;
    validate_band(right.metadata(), right_band)?;
    if left.metadata().grid.epsg != right.metadata().grid.epsg
        || left.metadata().grid.crs_type != right.metadata().grid.crs_type
    {
        return Err(Error::GridMismatch(GridMismatch {
            reason: "CRS",
            left: Box::new(left.metadata().grid.clone()),
            right: Box::new(right.metadata().grid.clone()),
        }));
    }

    let (target_raster, source_raster, target_nodata, source_is_left) = match target {
        TargetGrid::Left => {
            let target_raster = left.decode(window).map_err(Error::Decode)?;
            let source_raster =
                decode_resampling_source(&mut right, &target_raster.metadata.grid, method)?;
            (target_raster, source_raster, left.metadata().nodata, false)
        }
        TargetGrid::Right => {
            let target_raster = right.decode(window).map_err(Error::Decode)?;
            let source_raster =
                decode_resampling_source(&mut left, &target_raster.metadata.grid, method)?;
            (target_raster, source_raster, right.metadata().nodata, true)
        }
    };
    let resampled = resample(
        &source_raster,
        target_raster.metadata.grid.clone(),
        method,
        max_samples,
    )?;
    let mut output = if source_is_left {
        index(&resampled, left_band, &target_raster, right_band, kernel)?
    } else {
        index(&target_raster, left_band, &resampled, right_band, kernel)?
    };
    let fill = target_nodata.unwrap_or(f64::NAN);
    for (sample, valid) in output.samples.iter_mut().zip(&output.valid) {
        if !valid {
            *sample = fill;
        }
    }
    output.metadata.nodata = target_nodata;
    Ok(output)
}

#[allow(clippy::too_many_arguments)]
pub fn normalized_difference_resampled_file_window(
    left_handle: &str,
    left_band: usize,
    right_handle: &str,
    right_band: usize,
    window: Window,
    max_samples: usize,
    method: Resampling,
    target: TargetGrid,
) -> Result<Raster, Error> {
    index_resampled_file_window(
        left_handle,
        left_band,
        right_handle,
        right_band,
        window,
        max_samples,
        method,
        target,
        Kernel::NormalizedDifference,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn index_resampled_file_window(
    left_handle: &str,
    left_band: usize,
    right_handle: &str,
    right_band: usize,
    window: Window,
    max_samples: usize,
    method: Resampling,
    target: TargetGrid,
    kernel: Kernel,
) -> Result<Raster, Error> {
    let missing = || {
        Error::Decode(geotiff::Error::Invalid(
            "The selected local file is no longer loaded.".into(),
        ))
    };
    index_resampled_window(
        gp_base::files::reader(left_handle).ok_or_else(missing)?,
        left_band,
        gp_base::files::reader(right_handle).ok_or_else(missing)?,
        right_band,
        window,
        max_samples,
        method,
        target,
        kernel,
    )
}

fn decode_resampling_source<R: Read + Seek>(
    source: &mut WindowDecoder<R>,
    target: &Grid,
    method: Resampling,
) -> Result<Raster, Error> {
    let window = resampling_source_window(&source.metadata().grid, target, method)?;
    source.decode(window).map_err(Error::Decode)
}

fn resampling_source_window(
    source: &Grid,
    target: &Grid,
    method: Resampling,
) -> Result<Window, Error> {
    let source_transform = source
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
    let target_offset = raster_offset(target.raster_type);
    let source_offset = raster_offset(source.raster_type);
    let mut xs = [0.0; 4];
    let mut ys = [0.0; 4];
    for (i, (column, row)) in [
        (0, 0),
        (target.width.saturating_sub(1), 0),
        (0, target.height.saturating_sub(1)),
        (
            target.width.saturating_sub(1),
            target.height.saturating_sub(1),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let x = f64::from(column) + target_offset;
        let y = f64::from(row) + target_offset;
        let model_x = target_transform.x0 + x * target_transform.dx + y * target_transform.rx;
        let model_y = target_transform.y0 + x * target_transform.ry + y * target_transform.dy;
        let delta_x = model_x - source_transform.x0;
        let delta_y = model_y - source_transform.y0;
        xs[i] = (source_transform.dy * delta_x - source_transform.rx * delta_y) / determinant
            - source_offset;
        ys[i] = (-source_transform.ry * delta_x + source_transform.dx * delta_y) / determinant
            - source_offset;
    }
    if !xs.iter().chain(&ys).all(|value| value.is_finite()) {
        return Err(Error::InvalidRaster(
            "The raster transforms do not produce finite coordinates.",
        ));
    }
    let padding = match method {
        Resampling::Nearest => 0.5,
        Resampling::Bilinear => 0.0,
    };
    let bounds = |values: &[f64; 4], limit: u32| {
        let minimum = values.iter().copied().fold(f64::INFINITY, f64::min);
        let maximum = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let start = (minimum.floor() - padding)
            .max(0.0)
            .min(f64::from(limit - 1)) as u32;
        let end = (maximum.ceil() + padding + 1.0)
            .max(1.0)
            .min(f64::from(limit)) as u32;
        (start, end.max(start + 1))
    };
    if source.width == 0 || source.height == 0 || target.width == 0 || target.height == 0 {
        return Err(Error::InvalidRaster(
            "Raster dimensions must be greater than zero.",
        ));
    }
    let (x, x_end) = bounds(&xs, source.width);
    let (y, y_end) = bounds(&ys, source.height);
    Ok(Window {
        x,
        y,
        width: x_end - x,
        height: y_end - y,
    })
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
    index(
        left,
        left_band,
        right,
        right_band,
        Kernel::NormalizedDifference,
    )
}

/// Applies one fixed 2-band kernel to matching raster windows.
pub fn index(
    left: &Raster,
    left_band: usize,
    right: &Raster,
    right_band: usize,
    kernel: Kernel,
) -> Result<Raster, Error> {
    index_sources(&[(left, left_band), (right, right_band)], kernel)
}

/// Applies one fixed-formula kernel to matching raster windows.
pub fn index_sources(sources: &[(&Raster, usize)], kernel: Kernel) -> Result<Raster, Error> {
    if sources.len() != kernel.bands() {
        return Err(Error::InvalidRaster(
            "The raster kernel received the wrong number of band sources.",
        ));
    }
    let (target, target_band) = sources[0];
    let pixels = validate(target, target_band)?;
    for (raster, band) in &sources[1..] {
        same_grid(&target.metadata.grid, &raster.metadata.grid).map_err(Error::GridMismatch)?;
        if validate(raster, *band)? != pixels {
            return Err(Error::InvalidRaster(
                "Matching raster grids must have the same pixel count.",
            ));
        }
    }
    let fill = target.metadata.nodata.unwrap_or(f64::NAN);
    let mut samples = Vec::with_capacity(pixels);
    let mut valid = Vec::with_capacity(pixels);
    let mut values = Vec::with_capacity(sources.len());
    for pixel in 0..pixels {
        values.clear();
        let mut all_valid = true;
        for (raster, band) in sources {
            let index = pixel * usize::from(raster.metadata.bands) + band;
            all_valid &= raster.valid[index];
            values.push(raster.samples[index]);
        }
        let value = all_valid
            .then(|| kernel.value(&values))
            .flatten()
            .filter(|value| value.is_finite());
        samples.push(value.unwrap_or(fill));
        valid.push(value.is_some());
    }
    Ok(Raster {
        metadata: Metadata {
            grid: target.metadata.grid.clone(),
            bands: 1,
            nodata: target.metadata.nodata,
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
