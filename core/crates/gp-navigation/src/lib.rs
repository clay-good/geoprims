//! Navigation: geodesics, routes, line of sight, 3D vectors.

use gp_base::tool::Registry;

pub static REGISTRY: Registry = Registry {
    module: "navigation",
    tools: &[],
};

gp_base::export_module!("navigation", REGISTRY);
