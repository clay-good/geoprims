//! Raster: spectral indices and terrain analysis.

use gp_base::tool::Registry;

pub static REGISTRY: Registry = Registry {
    module: "raster",
    tools: &[],
};

gp_base::export_module!("raster", REGISTRY);
