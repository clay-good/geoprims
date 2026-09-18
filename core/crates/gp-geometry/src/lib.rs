//! Computational geometry on the plane and the ellipsoid.

use gp_base::tool::Registry;

pub static REGISTRY: Registry = Registry {
    module: "geometry",
    tools: &[],
};

gp_base::export_module!("geometry", REGISTRY);
