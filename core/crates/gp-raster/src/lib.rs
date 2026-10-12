//! Raster: spectral indices and terrain analysis.

pub mod bandmath;
pub mod contour;
pub mod geotiff;
pub mod indices;
pub mod processing;
pub mod scaling;
pub mod sightline;
pub mod terrain;

use gp_base::tool::Registry;

pub static REGISTRY: Registry = Registry {
    module: "raster",
    tools: TOOLS,
};

/// Every raster tool: the scaling step first, then the indices it feeds.
pub static TOOLS: &[&gp_base::tool::ToolDef] = &[
    &scaling::SCALE,
    &indices::NDVI,
    &indices::NDWI_MCFEETERS,
    &indices::NDWI_GAO,
    &indices::MNDWI,
    &indices::NDBI,
    &indices::NBR,
    &indices::EVI,
    &indices::EVI2,
    &indices::SAVI,
    &indices::DNBR,
    &bandmath::BANDMATH,
    &terrain::SLOPE,
    &terrain::RUGGEDNESS,
    &terrain::CURVATURE,
    &contour::CONTOURS,
    &sightline::LINE_OF_SIGHT,
];

gp_base::export_module!("raster", REGISTRY);
gp_base::export_file_bridge!();

#[cfg(target_arch = "wasm32")]
mod file_exports {
    use serde_json::{Value, json};

    use crate::geotiff::{
        CrsType, Error as GeotiffError, Grid, Metadata, Raster, RasterType, Window,
    };
    use crate::processing::Error as ProcessingError;
    use crate::processing::{Kernel, Resampling, TargetGrid};

    const MAX_WINDOW_SAMPLES: usize = 4 * 1024 * 1024;

    fn failure(error: GeotiffError) -> Value {
        match error {
            GeotiffError::Invalid(message) => {
                json!({"ok": false, "error": {"code": "INVALID_INPUT", "message": message}})
            }
            GeotiffError::LimitExceeded { samples, limit } => json!({
                "ok": false,
                "error": {
                    "code": "LIMIT_EXCEEDED",
                    "message": format!("The raster needs {samples} samples; this operation allows {limit}."),
                }
            }),
        }
    }

    fn grid_value(grid: &Grid) -> Value {
        let transform = grid.transform.map(
            |t| json!({"x0": t.x0, "dx": t.dx, "rx": t.rx, "y0": t.y0, "ry": t.ry, "dy": t.dy}),
        );
        json!({
            "width": grid.width,
            "height": grid.height,
            "epsg": grid.epsg,
            "crsType": grid.crs_type.map(|kind| match kind {
                CrsType::Geographic => "geographic",
                CrsType::Projected => "projected",
            }),
            "rasterType": grid.raster_type.map(|kind| match kind {
                RasterType::PixelIsArea => "area",
                RasterType::PixelIsPoint => "point",
            }),
            "transform": transform,
        })
    }

    fn metadata_value(metadata: &Metadata) -> Value {
        let grid = &metadata.grid;
        let transform = grid.transform.map(
            |t| json!({"x0": t.x0, "dx": t.dx, "rx": t.rx, "y0": t.y0, "ry": t.ry, "dy": t.dy}),
        );
        json!({
            "width": grid.width,
            "height": grid.height,
            "bands": metadata.bands,
            "nodata": metadata.nodata,
            "epsg": grid.epsg,
            "crsType": grid.crs_type.map(|kind| match kind {
                CrsType::Geographic => "geographic",
                CrsType::Projected => "projected",
            }),
            "rasterType": grid.raster_type.map(|kind| match kind {
                RasterType::PixelIsArea => "area",
                RasterType::PixelIsPoint => "point",
            }),
            "transform": transform,
        })
    }

    fn raster_value(raster: Raster) -> Value {
        let metadata = metadata_value(&raster.metadata);
        json!({"metadata": metadata, "samples": raster.samples, "valid": raster.valid})
    }

    fn processing_failure(error: ProcessingError) -> Value {
        let (code, message, details) = match error {
            ProcessingError::Decode(error) | ProcessingError::Export(error) => {
                return failure(error);
            }
            ProcessingError::GridMismatch(mismatch) => (
                "GRID_MISMATCH",
                format!("The input raster grids differ in {}.", mismatch.reason),
                Some(json!({
                    "reason": mismatch.reason,
                    "left": grid_value(&mismatch.left),
                    "right": grid_value(&mismatch.right),
                })),
            ),
            ProcessingError::InvalidBand { band, bands } => (
                "INVALID_INPUT",
                format!("Band index {band} is outside a raster with {bands} bands."),
                None,
            ),
            ProcessingError::InvalidRaster(message) => ("INVALID_INPUT", message.into(), None),
            ProcessingError::InvalidWindowSize => (
                "INVALID_INPUT",
                "The raster window must have nonzero width and height.".into(),
                None,
            ),
            ProcessingError::LimitExceeded { samples, limit } => (
                "LIMIT_EXCEEDED",
                format!("The raster needs {samples} samples; this operation allows {limit}."),
                None,
            ),
        };
        json!({"ok": false, "error": {"code": code, "message": message, "details": details}})
    }

    fn field_u32(input: &Value, name: &str) -> Result<u32, Value> {
        input
            .get(name)
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| json!({
                "ok": false,
                "error": {"code": "INVALID_INPUT", "message": format!("{name} must be a nonnegative 32-bit integer.")},
            }))
    }

    fn window(input: &Value) -> Result<Window, Value> {
        Ok(Window {
            x: field_u32(input, "x")?,
            y: field_u32(input, "y")?,
            width: field_u32(input, "width")?,
            height: field_u32(input, "height")?,
        })
    }

    fn max_samples(input: &Value) -> Result<usize, Value> {
        let max_samples = field_u32(input, "maxSamples")? as usize;
        if max_samples == 0 || max_samples > MAX_WINDOW_SAMPLES {
            return Err(json!({
                "ok": false,
                "error": {"code": "LIMIT_EXCEEDED", "message": format!("maxSamples must be from 1 to {MAX_WINDOW_SAMPLES}.")},
            }));
        }
        Ok(max_samples)
    }

    fn resampling(input: &Value) -> Result<Option<(Resampling, TargetGrid)>, Value> {
        let Some(options) = input.get("resampling") else {
            return Ok(None);
        };
        let method = match options.get("method").and_then(Value::as_str) {
            Some("nearest") => Resampling::Nearest,
            Some("bilinear") => Resampling::Bilinear,
            _ => {
                return Err(json!({
                    "ok": false,
                    "error": {"code": "INVALID_INPUT", "message": "resampling.method must be nearest or bilinear."},
                }));
            }
        };
        let target = match options.get("target").and_then(Value::as_str) {
            Some("left") => TargetGrid::Left,
            Some("right") => TargetGrid::Right,
            _ => {
                return Err(json!({
                    "ok": false,
                    "error": {"code": "INVALID_INPUT", "message": "resampling.target must be left or right."},
                }));
            }
        };
        Ok(Some((method, target)))
    }

    fn kernel(input: &Value) -> Result<Kernel, Value> {
        let tool = input.get("tool").and_then(Value::as_str).ok_or_else(|| {
            json!({"ok": false, "error": {"code": "INVALID_INPUT", "message": "tool must be a string."}})
        })?;
        match tool {
            "raster.index.ndvi"
            | "raster.index.ndwi-mcfeeters"
            | "raster.index.ndwi-gao"
            | "raster.index.mndwi"
            | "raster.index.ndbi"
            | "raster.index.nbr" => Ok(Kernel::NormalizedDifference),
            "raster.index.evi" => Ok(Kernel::Evi),
            "raster.index.evi2" => Ok(Kernel::Evi2),
            "raster.index.dnbr" => Ok(Kernel::Dnbr),
            "raster.index.savi" => {
                let soil_factor = input
                    .get("parameters")
                    .and_then(|parameters| parameters.get("soil_factor"))
                    .map_or(Some(0.5), Value::as_f64)
                    .filter(|value| (0.0..=1.0).contains(value))
                    .ok_or_else(|| {
                        json!({"ok": false, "error": {"code": "INVALID_INPUT", "message": "parameters.soil_factor must be a number from 0 to 1."}})
                    })?;
                Ok(Kernel::Savi { soil_factor })
            }
            _ => Err(json!({
                "ok": false,
                "error": {"code": "UNSUPPORTED", "message": format!("{tool} has no 2-band raster kernel.")},
            })),
        }
    }

    fn sources(input: &Value) -> Result<Option<Vec<(&str, usize)>>, Value> {
        let Some(sources) = input.get("sources") else {
            return Ok(None);
        };
        let sources = sources.as_array().ok_or_else(|| {
            json!({"ok": false, "error": {"code": "INVALID_INPUT", "message": "sources must be an array."}})
        })?;
        sources
            .iter()
            .map(|source| {
                let handle = source.get("handle").and_then(Value::as_str).ok_or_else(|| {
                    json!({"ok": false, "error": {"code": "INVALID_INPUT", "message": "Each source handle must be a string."}})
                })?;
                let band = source
                    .get("band")
                    .and_then(Value::as_u64)
                    .and_then(|value| usize::try_from(value).ok())
                    .ok_or_else(|| {
                        json!({"ok": false, "error": {"code": "INVALID_INPUT", "message": "Each source band must be a nonnegative integer."}})
                    })?;
                Ok((handle, band))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some)
    }

    fn band_math(input: &Value) -> Result<(&str, Vec<String>), Value> {
        let parameters = input.get("parameters").ok_or_else(|| {
            json!({"ok": false, "error": {"code": "INVALID_INPUT", "message": "Band math parameters are missing."}})
        })?;
        let expression = parameters
            .get("expression")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                json!({"ok": false, "error": {"code": "INVALID_INPUT", "message": "parameters.expression must be a string."}})
            })?;
        let rows = parameters
            .get("bands")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                json!({"ok": false, "error": {"code": "INVALID_INPUT", "message": "parameters.bands must be an array."}})
            })?;
        let names = rows
            .iter()
            .map(|row| {
                row.get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .map(str::to_owned)
                    .ok_or_else(|| {
                        json!({"ok": false, "error": {"code": "INVALID_INPUT", "message": "Every band-math row needs a name."}})
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok((expression, names))
    }

    unsafe fn request(input: *const u8, input_len: usize) -> Result<Value, Value> {
        let text = unsafe { gp_base::abi::read_str(input, input_len) }.map_err(|_| {
            json!({"ok": false, "error": {"code": "INVALID_INPUT", "message": "The window request must be UTF-8."}})
        })?;
        serde_json::from_str(text).map_err(|_| {
            json!({"ok": false, "error": {"code": "INVALID_INPUT", "message": "The window request must be valid JSON."}})
        })
    }

    /// # Safety
    /// The handle range must be readable UTF-8 from `gp_alloc`.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn gp_raster_file_inspect(
        handle: *const u8,
        handle_len: usize,
    ) -> *const u8 {
        let out = match unsafe { gp_base::abi::read_str(handle, handle_len) } {
            Ok(handle) => match crate::geotiff::inspect_file(handle) {
                Ok(metadata) => json!({"ok": true, "metadata": metadata_value(&metadata)}),
                Err(error) => failure(error),
            },
            Err(_) => {
                json!({"ok": false, "error": {"code": "INVALID_INPUT", "message": "The file handle must be UTF-8."}})
            }
        };
        gp_base::abi::set_out(&out.to_string())
    }

    /// # Safety
    /// The input range must be readable UTF-8 from `gp_alloc`.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn gp_raster_file_window(
        input: *const u8,
        input_len: usize,
    ) -> *const u8 {
        let out = (|| {
            let value = unsafe { request(input, input_len) }?;
            let handle = value.get("handle").and_then(Value::as_str).ok_or_else(|| {
                json!({"ok": false, "error": {"code": "INVALID_INPUT", "message": "handle must be a string."}})
            })?;
            crate::geotiff::decode_file_window(handle, window(&value)?, max_samples(&value)?)
                .map(|raster| json!({"ok": true, "raster": raster_value(raster)}))
                .map_err(failure)
        })()
        .unwrap_or_else(|error| error);
        gp_base::abi::set_out(&out.to_string())
    }

    /// # Safety
    /// The input range must be readable UTF-8 from `gp_alloc`.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn gp_raster_file_normalized_difference(
        input: *const u8,
        input_len: usize,
    ) -> *const u8 {
        let out = (|| {
            let value = unsafe { request(input, input_len) }?;
            let window = window(&value)?;
            let max_samples = max_samples(&value)?;
            if let Some(sources) = sources(&value)? {
                if value.get("resampling").is_some() {
                    return Err(json!({"ok": false, "error": {"code": "INVALID_INPUT", "message": "Multi-band raster resampling is not available yet; align all source grids first."}}));
                }
                if value.get("tool").and_then(Value::as_str) == Some("raster.index.band-math") {
                    let (expression, names) = band_math(&value)?;
                    return crate::processing::band_math_file_sources_window(
                        &sources,
                        &names,
                        expression,
                        window,
                        max_samples,
                    )
                    .map(|raster| json!({"ok": true, "raster": raster_value(raster)}))
                    .map_err(processing_failure);
                }
                let kernel = kernel(&value)?;
                return crate::processing::index_file_sources_window(
                    &sources,
                    window,
                    max_samples,
                    kernel,
                )
                .map(|raster| json!({"ok": true, "raster": raster_value(raster)}))
                .map_err(processing_failure);
            }
            let string = |name| {
                value.get(name).and_then(Value::as_str).ok_or_else(|| {
                    json!({"ok": false, "error": {"code": "INVALID_INPUT", "message": format!("{name} must be a string.")}})
                })
            };
            let left_handle = string("leftHandle")?;
            let left_band = field_u32(&value, "leftBand")? as usize;
            let right_handle = string("rightHandle")?;
            let right_band = field_u32(&value, "rightBand")? as usize;
            let kernel = kernel(&value)?;
            match resampling(&value)? {
                Some((method, target)) => crate::processing::index_resampled_file_window(
                    left_handle,
                    left_band,
                    right_handle,
                    right_band,
                    window,
                    max_samples,
                    method,
                    target,
                    kernel,
                ),
                None => crate::processing::index_file_window(
                    left_handle,
                    left_band,
                    right_handle,
                    right_band,
                    window,
                    max_samples,
                    kernel,
                ),
            }
            .map(|raster| json!({"ok": true, "raster": raster_value(raster)}))
            .map_err(processing_failure)
        })()
        .unwrap_or_else(|error| error);
        gp_base::abi::set_out(&out.to_string())
    }
}
