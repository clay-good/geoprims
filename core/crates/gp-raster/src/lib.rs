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

    use crate::geotiff::{CrsType, Error, Metadata, RasterType, Window};

    const MAX_WINDOW_SAMPLES: usize = 4 * 1024 * 1024;

    fn failure(error: Error) -> Value {
        match error {
            Error::Invalid(message) => {
                json!({"ok": false, "error": {"code": "INVALID_INPUT", "message": message}})
            }
            Error::LimitExceeded { samples, limit } => json!({
                "ok": false,
                "error": {
                    "code": "LIMIT_EXCEEDED",
                    "message": format!("The raster needs {samples} samples; this operation allows {limit}."),
                }
            }),
        }
    }

    fn metadata_value(metadata: &Metadata) -> Value {
        let transform = metadata.grid.transform.map(
            |t| json!({"x0": t.x0, "dx": t.dx, "rx": t.rx, "y0": t.y0, "ry": t.ry, "dy": t.dy}),
        );
        json!({
            "width": metadata.grid.width,
            "height": metadata.grid.height,
            "bands": metadata.bands,
            "nodata": metadata.nodata,
            "epsg": metadata.grid.epsg,
            "crsType": metadata.grid.crs_type.map(|kind| match kind {
                CrsType::Geographic => "geographic",
                CrsType::Projected => "projected",
            }),
            "rasterType": metadata.grid.raster_type.map(|kind| match kind {
                RasterType::PixelIsArea => "area",
                RasterType::PixelIsPoint => "point",
            }),
            "transform": transform,
        })
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
            let text = unsafe { gp_base::abi::read_str(input, input_len) }.map_err(|_| {
                json!({"ok": false, "error": {"code": "INVALID_INPUT", "message": "The window request must be UTF-8."}})
            })?;
            let value: Value = serde_json::from_str(text).map_err(|_| {
                json!({"ok": false, "error": {"code": "INVALID_INPUT", "message": "The window request must be valid JSON."}})
            })?;
            let handle = value.get("handle").and_then(Value::as_str).ok_or_else(|| {
                json!({"ok": false, "error": {"code": "INVALID_INPUT", "message": "handle must be a string."}})
            })?;
            let window = Window {
                x: field_u32(&value, "x")?,
                y: field_u32(&value, "y")?,
                width: field_u32(&value, "width")?,
                height: field_u32(&value, "height")?,
            };
            let max_samples = field_u32(&value, "maxSamples")? as usize;
            if max_samples == 0 || max_samples > MAX_WINDOW_SAMPLES {
                return Err(json!({
                    "ok": false,
                    "error": {"code": "LIMIT_EXCEEDED", "message": format!("maxSamples must be from 1 to {MAX_WINDOW_SAMPLES}.")},
                }));
            }
            crate::geotiff::decode_file_window(handle, window, max_samples)
                .map(|raster| json!({
                    "ok": true,
                    "raster": {
                        "metadata": metadata_value(&raster.metadata),
                        "samples": raster.samples,
                        "valid": raster.valid,
                    }
                }))
                .map_err(failure)
        })()
        .unwrap_or_else(|error| error);
        gp_base::abi::set_out(&out.to_string())
    }
}
