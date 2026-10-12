//! GeoTIFF decoding shared by the web and MCP raster surfaces.
//!
//! This module has no file-system or browser assumptions. A host supplies one
//! bounded byte slice; the decoder returns numeric samples, a no-data mask,
//! and the grid definition needed to compare or preserve georeferencing.

use std::io::{Cursor, Read, Seek};

use tiff::decoder::{ChunkType, Decoder, DecodingResult, Limits};
use tiff::tags::Tag;

/// The GeoTIFF raster-space convention (GeoKey 1025).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RasterType {
    PixelIsArea,
    PixelIsPoint,
}

/// The affine mapping from TIFF raster coordinates to model coordinates.
///
/// `x = x0 + column * dx + row * rx`; `y = y0 + column * ry + row * dy`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    pub x0: f64,
    pub dx: f64,
    pub rx: f64,
    pub y0: f64,
    pub ry: f64,
    pub dy: f64,
}

/// The parts of a raster grid that must agree before bands can be combined.
#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    pub width: u32,
    pub height: u32,
    pub transform: Option<Transform>,
    pub epsg: Option<u16>,
    pub raster_type: Option<RasterType>,
}

/// TIFF and GeoTIFF metadata needed before any pixel allocation.
#[derive(Clone, Debug, PartialEq)]
pub struct Metadata {
    pub grid: Grid,
    pub bands: u16,
    pub nodata: Option<f64>,
}

/// A decoded image. Samples are interleaved by pixel when there is more than
/// one band. `valid` is false for a sample matching the GDAL no-data tag.
#[derive(Clone, Debug, PartialEq)]
pub struct Raster {
    pub metadata: Metadata,
    pub samples: Vec<f64>,
    pub valid: Vec<bool>,
}

/// A pixel-aligned rectangle within one TIFF image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    Invalid(String),
    LimitExceeded { samples: u64, limit: usize },
}

#[derive(Clone, Debug, PartialEq)]
pub struct GridMismatch {
    pub reason: &'static str,
    pub left: Box<Grid>,
    pub right: Box<Grid>,
}

fn invalid(error: impl std::fmt::Display) -> Error {
    Error::Invalid(error.to_string())
}

fn geokeys<R: Read + Seek>(decoder: &mut Decoder<R>) -> Result<Vec<u16>, Error> {
    decoder
        .find_tag(Tag::GeoKeyDirectoryTag)
        .map_err(invalid)?
        .map(|value| value.into_u16_vec().map_err(invalid))
        .transpose()
        .map(Option::unwrap_or_default)
}

fn inline_key(keys: &[u16], wanted: u16) -> Option<u16> {
    let count = *keys.get(3)? as usize;
    keys.get(4..4 + count * 4)?
        .as_chunks::<4>()
        .0
        .iter()
        .find_map(|entry| {
            (entry[0] == wanted && entry[1] == 0 && entry[2] == 1).then_some(entry[3])
        })
}

fn transform<R: Read + Seek>(decoder: &mut Decoder<R>) -> Result<Option<Transform>, Error> {
    if let Some(value) = decoder
        .find_tag(Tag::ModelTransformationTag)
        .map_err(invalid)?
    {
        let m = value.into_f64_vec().map_err(invalid)?;
        if m.len() != 16 {
            return Err(Error::Invalid(
                "ModelTransformationTag must hold 16 numbers.".into(),
            ));
        }
        return Ok(Some(Transform {
            x0: m[3],
            dx: m[0],
            rx: m[1],
            y0: m[7],
            ry: m[4],
            dy: m[5],
        }));
    }
    let scale = decoder
        .find_tag(Tag::ModelPixelScaleTag)
        .map_err(invalid)?
        .map(|value| value.into_f64_vec().map_err(invalid))
        .transpose()?;
    let tie = decoder
        .find_tag(Tag::ModelTiepointTag)
        .map_err(invalid)?
        .map(|value| value.into_f64_vec().map_err(invalid))
        .transpose()?;
    match (scale, tie) {
        (None, None) => Ok(None),
        (Some(scale), Some(tie)) if scale.len() >= 2 && tie.len() >= 6 => Ok(Some(Transform {
            x0: tie[3] - tie[0] * scale[0],
            dx: scale[0],
            rx: 0.0,
            y0: tie[4] + tie[1] * scale[1],
            ry: 0.0,
            dy: -scale[1],
        })),
        _ => Err(Error::Invalid(
            "GeoTIFF georeferencing needs both ModelPixelScaleTag and ModelTiepointTag.".into(),
        )),
    }
}

fn metadata<R: Read + Seek>(decoder: &mut Decoder<R>) -> Result<Metadata, Error> {
    let orientation = decoder
        .find_tag_unsigned::<u16>(Tag::Orientation)
        .map_err(invalid)?
        .unwrap_or(1);
    if orientation != 1 {
        return Err(Error::Invalid(format!(
            "TIFF orientation {orientation} is not supported; write rows from top to bottom."
        )));
    }
    let planar_configuration = decoder
        .find_tag_unsigned::<u16>(Tag::PlanarConfiguration)
        .map_err(invalid)?
        .unwrap_or(1);
    if planar_configuration != 1 {
        return Err(Error::Invalid(
            "Planar TIFF storage is not supported; write samples interleaved by pixel.".into(),
        ));
    }
    let (width, height) = decoder.dimensions().map_err(invalid)?;
    let bands = decoder.colortype().map_err(invalid)?.num_samples();
    let keys = geokeys(decoder)?;
    let epsg = inline_key(&keys, 3072).or_else(|| inline_key(&keys, 2048));
    let raster_type = match inline_key(&keys, 1025) {
        Some(1) => Some(RasterType::PixelIsArea),
        Some(2) => Some(RasterType::PixelIsPoint),
        Some(_) => {
            return Err(Error::Invalid(
                "GeoKey 1025 has an unknown raster type.".into(),
            ));
        }
        None => None,
    };
    let nodata = decoder
        .find_tag(Tag::GdalNodata)
        .map_err(invalid)?
        .map(|value| value.into_string().map_err(invalid))
        .transpose()?
        .map(|text| {
            text.trim_matches('\0')
                .trim()
                .parse::<f64>()
                .map_err(|_| Error::Invalid(format!("GDAL_NODATA is not a number: {text:?}.")))
        })
        .transpose()?;
    Ok(Metadata {
        grid: Grid {
            width,
            height,
            transform: transform(decoder)?,
            epsg,
            raster_type,
        },
        bands,
        nodata,
    })
}

fn samples_as_f64(result: DecodingResult) -> Vec<f64> {
    match result {
        DecodingResult::U8(v) => v.into_iter().map(f64::from).collect(),
        DecodingResult::U16(v) => v.into_iter().map(f64::from).collect(),
        DecodingResult::U32(v) => v.into_iter().map(f64::from).collect(),
        DecodingResult::U64(v) => v.into_iter().map(|x| x as f64).collect(),
        DecodingResult::I8(v) => v.into_iter().map(f64::from).collect(),
        DecodingResult::I16(v) => v.into_iter().map(f64::from).collect(),
        DecodingResult::I32(v) => v.into_iter().map(f64::from).collect(),
        DecodingResult::I64(v) => v.into_iter().map(|x| x as f64).collect(),
        DecodingResult::F16(v) => v.into_iter().map(|x| x.to_f64()).collect(),
        DecodingResult::F32(v) => v.into_iter().map(f64::from).collect(),
        DecodingResult::F64(v) => v,
    }
}

fn validity(samples: &[f64], nodata: Option<f64>) -> Vec<bool> {
    samples
        .iter()
        .map(|sample| match nodata {
            Some(nodata) if nodata.is_nan() => !sample.is_nan(),
            Some(nodata) => *sample != nodata,
            None => true,
        })
        .collect()
}

fn sample_count(width: u32, height: u32, bands: u16, limit: usize) -> Result<usize, Error> {
    let count = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|n| n.checked_mul(u64::from(bands)))
        .ok_or(Error::LimitExceeded {
            samples: u64::MAX,
            limit,
        })?;
    if count > limit as u64 {
        return Err(Error::LimitExceeded {
            samples: count,
            limit,
        });
    }
    Ok(count as usize)
}

fn pixel_offset(x: u32, y: u32, width: u32) -> Result<usize, Error> {
    let offset = u64::from(y)
        .checked_mul(u64::from(width))
        .and_then(|n| n.checked_add(u64::from(x)))
        .ok_or_else(|| Error::Invalid("TIFF pixel offset overflowed.".into()))?;
    usize::try_from(offset).map_err(invalid)
}

/// Reads dimensions, band count, no-data, CRS, and the affine transform
/// without allocating a pixel buffer.
pub fn inspect(bytes: &[u8]) -> Result<Metadata, Error> {
    inspect_reader(Cursor::new(bytes))
}

/// Reads metadata from any seekable source, including a range-backed reader.
pub fn inspect_reader<R: Read + Seek>(reader: R) -> Result<Metadata, Error> {
    let mut decoder = Decoder::new(reader).map_err(invalid)?;
    metadata(&mut decoder)
}

/// Inspects bytes supplied through the private local-file ABI.
pub fn inspect_file(handle: &str) -> Result<Metadata, Error> {
    let bytes = gp_base::files::get(handle)
        .ok_or_else(|| Error::Invalid("The selected local file is no longer loaded.".into()))?;
    inspect(&bytes)
}

/// Decodes one TIFF image after enforcing the caller's sample allocation cap.
/// Large rasters will use the tiled path rather than this whole-image helper.
pub fn decode(bytes: &[u8], max_samples: usize) -> Result<Raster, Error> {
    decode_reader(Cursor::new(bytes), max_samples)
}

/// Decodes one TIFF image from any seekable source.
pub fn decode_reader<R: Read + Seek>(reader: R, max_samples: usize) -> Result<Raster, Error> {
    let mut decoder = Decoder::new(reader).map_err(invalid)?;
    let metadata = metadata(&mut decoder)?;
    let count = sample_count(
        metadata.grid.width,
        metadata.grid.height,
        metadata.bands,
        max_samples,
    )?;
    let mut limits = Limits::default();
    limits.decoding_buffer_size = max_samples.saturating_mul(std::mem::size_of::<f64>());
    decoder = decoder.with_limits(limits);
    let samples = samples_as_f64(decoder.read_image().map_err(invalid)?);
    if samples.len() != count {
        return Err(Error::Invalid(
            "Decoded sample count does not match the TIFF dimensions.".into(),
        ));
    }
    let valid = validity(&samples, metadata.nodata);
    Ok(Raster {
        metadata,
        samples,
        valid,
    })
}

/// Decodes only strips or tiles that intersect a pixel-aligned window.
/// `max_samples` must fit both the returned window and one decoded source
/// chunk, so a small window cannot conceal an unexpectedly large strip.
pub fn decode_window(bytes: &[u8], window: Window, max_samples: usize) -> Result<Raster, Error> {
    decode_window_reader(Cursor::new(bytes), window, max_samples)
}

/// Decodes a window from any seekable source without reading other pixel
/// chunks. TIFF directory bytes may still be read wherever the file stores
/// them.
pub fn decode_window_reader<R: Read + Seek>(
    reader: R,
    window: Window,
    max_samples: usize,
) -> Result<Raster, Error> {
    WindowDecoder::new(reader, max_samples)?.decode(window)
}

/// A reusable decoder for processing one bounded window at a time.
pub struct WindowDecoder<R: Read + Seek> {
    decoder: Decoder<R>,
    metadata: Metadata,
    max_samples: usize,
}

impl<R: Read + Seek> WindowDecoder<R> {
    pub fn new(reader: R, max_samples: usize) -> Result<Self, Error> {
        let mut decoder = Decoder::new(reader).map_err(invalid)?;
        let metadata = metadata(&mut decoder)?;
        let mut limits = Limits::default();
        limits.decoding_buffer_size = max_samples.saturating_mul(std::mem::size_of::<f64>());
        Ok(Self {
            decoder: decoder.with_limits(limits),
            metadata,
            max_samples,
        })
    }

    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    pub fn decode(&mut self, window: Window) -> Result<Raster, Error> {
        let mut metadata = self.metadata.clone();
        let x_end = window.x.checked_add(window.width);
        let y_end = window.y.checked_add(window.height);
        let (Some(x_end), Some(y_end)) = (x_end, y_end) else {
            return Err(Error::Invalid(
                "The requested window must be non-empty and inside the TIFF dimensions.".into(),
            ));
        };
        if window.width == 0
            || window.height == 0
            || x_end > metadata.grid.width
            || y_end > metadata.grid.height
        {
            return Err(Error::Invalid(
                "The requested window must be non-empty and inside the TIFF dimensions.".into(),
            ));
        }
        let count = sample_count(
            window.width,
            window.height,
            metadata.bands,
            self.max_samples,
        )?;

        let (chunk_width, chunk_height) = self.decoder.chunk_dimensions();
        let chunks_across = metadata.grid.width.div_ceil(chunk_width);
        let chunk_count = match self.decoder.get_chunk_type() {
            ChunkType::Strip => self.decoder.strip_count().map_err(invalid)?,
            ChunkType::Tile => self.decoder.tile_count().map_err(invalid)?,
        };
        let first_chunk_x = window.x / chunk_width;
        let last_chunk_x = (x_end - 1) / chunk_width;
        let first_chunk_y = window.y / chunk_height;
        let last_chunk_y = (y_end - 1) / chunk_height;
        let bands = usize::from(metadata.bands);
        let mut samples = vec![0.0; count];

        for chunk_y in first_chunk_y..=last_chunk_y {
            for chunk_x in first_chunk_x..=last_chunk_x {
                let chunk_index = chunk_y
                    .checked_mul(chunks_across)
                    .and_then(|n| n.checked_add(chunk_x))
                    .ok_or_else(|| Error::Invalid("TIFF chunk index overflowed.".into()))?;
                if chunk_index >= chunk_count {
                    return Err(Error::Invalid("TIFF chunk index is out of range.".into()));
                }
                let (data_width, data_height) = self.decoder.chunk_data_dimensions(chunk_index);
                let expected =
                    sample_count(data_width, data_height, metadata.bands, self.max_samples)?;
                let chunk = samples_as_f64(self.decoder.read_chunk(chunk_index).map_err(invalid)?);
                if chunk.len() != expected {
                    return Err(Error::Invalid(
                        "Decoded chunk sample count does not match its TIFF dimensions.".into(),
                    ));
                }
                let chunk_origin_x = chunk_x * chunk_width;
                let chunk_origin_y = chunk_y * chunk_height;
                let copy_x0 = window.x.max(chunk_origin_x);
                let copy_y0 = window.y.max(chunk_origin_y);
                let copy_x1 = x_end.min(chunk_origin_x.saturating_add(data_width));
                let copy_y1 = y_end.min(chunk_origin_y.saturating_add(data_height));
                for image_y in copy_y0..copy_y1 {
                    for image_x in copy_x0..copy_x1 {
                        let source_pixel = pixel_offset(
                            image_x - chunk_origin_x,
                            image_y - chunk_origin_y,
                            data_width,
                        )?;
                        let target_pixel =
                            pixel_offset(image_x - window.x, image_y - window.y, window.width)?;
                        let source = source_pixel * bands;
                        let target = target_pixel * bands;
                        samples[target..target + bands]
                            .copy_from_slice(&chunk[source..source + bands]);
                    }
                }
            }
        }

        if let Some(transform) = metadata.grid.transform.as_mut() {
            transform.x0 += f64::from(window.x) * transform.dx + f64::from(window.y) * transform.rx;
            transform.y0 += f64::from(window.x) * transform.ry + f64::from(window.y) * transform.dy;
        }
        metadata.grid.width = window.width;
        metadata.grid.height = window.height;
        let valid = validity(&samples, metadata.nodata);
        Ok(Raster {
            metadata,
            samples,
            valid,
        })
    }
}

/// Decodes bytes supplied through the private local-file ABI.
pub fn decode_file(handle: &str, max_samples: usize) -> Result<Raster, Error> {
    let bytes = gp_base::files::get(handle)
        .ok_or_else(|| Error::Invalid("The selected local file is no longer loaded.".into()))?;
    decode(&bytes, max_samples)
}

/// Decodes a window from bytes supplied through the private local-file ABI.
pub fn decode_file_window(
    handle: &str,
    window: Window,
    max_samples: usize,
) -> Result<Raster, Error> {
    let bytes = gp_base::files::get(handle)
        .ok_or_else(|| Error::Invalid("The selected local file is no longer loaded.".into()))?;
    decode_window(&bytes, window, max_samples)
}

/// Refuses bands whose dimensions, CRS, pixel convention, or transform differ.
pub fn same_grid(left: &Grid, right: &Grid) -> Result<(), GridMismatch> {
    let reason = if left.width != right.width || left.height != right.height {
        Some("dimensions")
    } else if left.epsg != right.epsg {
        Some("CRS")
    } else if left.raster_type != right.raster_type {
        Some("raster type")
    } else if left.transform != right.transform {
        Some("geotransform")
    } else {
        None
    };
    match reason {
        Some(reason) => Err(GridMismatch {
            reason,
            left: Box::new(left.clone()),
            right: Box::new(right.clone()),
        }),
        None => Ok(()),
    }
}
