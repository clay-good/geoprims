//! GeoTIFF decoding shared by the web and MCP raster surfaces.
//!
//! This module has no file-system or browser assumptions. A host supplies one
//! bounded byte slice; the decoder returns numeric samples, a no-data mask,
//! and the grid definition needed to compare or preserve georeferencing.

use std::io::Cursor;

use tiff::decoder::{Decoder, DecodingResult, Limits};
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

fn geokeys(decoder: &mut Decoder<Cursor<&[u8]>>) -> Result<Vec<u16>, Error> {
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

fn transform(decoder: &mut Decoder<Cursor<&[u8]>>) -> Result<Option<Transform>, Error> {
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

fn metadata(decoder: &mut Decoder<Cursor<&[u8]>>) -> Result<Metadata, Error> {
    let orientation = decoder
        .find_tag_unsigned::<u16>(Tag::Orientation)
        .map_err(invalid)?
        .unwrap_or(1);
    if orientation != 1 {
        return Err(Error::Invalid(format!(
            "TIFF orientation {orientation} is not supported; write rows from top to bottom."
        )));
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

/// Reads dimensions, band count, no-data, CRS, and the affine transform
/// without allocating a pixel buffer.
pub fn inspect(bytes: &[u8]) -> Result<Metadata, Error> {
    let mut decoder = Decoder::new(Cursor::new(bytes)).map_err(invalid)?;
    metadata(&mut decoder)
}

/// Decodes one TIFF image after enforcing the caller's sample allocation cap.
/// Large rasters will use the tiled path rather than this whole-image helper.
pub fn decode(bytes: &[u8], max_samples: usize) -> Result<Raster, Error> {
    let mut decoder = Decoder::new(Cursor::new(bytes)).map_err(invalid)?;
    let metadata = metadata(&mut decoder)?;
    let count = u64::from(metadata.grid.width)
        .checked_mul(u64::from(metadata.grid.height))
        .and_then(|n| n.checked_mul(u64::from(metadata.bands)))
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
    let mut limits = Limits::default();
    limits.decoding_buffer_size = max_samples.saturating_mul(std::mem::size_of::<f64>());
    decoder = decoder.with_limits(limits);
    let samples = match decoder.read_image().map_err(invalid)? {
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
    };
    if samples.len() as u64 != count {
        return Err(Error::Invalid(
            "Decoded sample count does not match the TIFF dimensions.".into(),
        ));
    }
    let valid = samples
        .iter()
        .map(|sample| match metadata.nodata {
            Some(nodata) if nodata.is_nan() => !sample.is_nan(),
            Some(nodata) => *sample != nodata,
            None => true,
        })
        .collect();
    Ok(Raster {
        metadata,
        samples,
        valid,
    })
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
