//! Raster: spectral indices and terrain analysis.

pub mod bandmath;
pub mod contour;
pub mod indices;
pub mod scaling;
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
    &contour::CONTOURS,
];

gp_base::export_module!("raster", REGISTRY);
