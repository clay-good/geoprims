//! Geodesy: parsing, frames, datums, projections, grid references, heights, geomagnetism.

use gp_base::tool::Registry;

pub static REGISTRY: Registry = Registry {
    module: "geodesy",
    tools: &[],
};

gp_base::export_module!("geodesy", REGISTRY);
