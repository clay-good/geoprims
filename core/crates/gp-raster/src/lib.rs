//! Raster: spectral indices and terrain analysis.

pub mod indices;

use gp_base::tool::Registry;

pub static REGISTRY: Registry = Registry {
    module: "raster",
    tools: indices::TOOLS,
};

gp_base::export_module!("raster", REGISTRY);
