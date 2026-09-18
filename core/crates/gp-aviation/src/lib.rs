//! Aviation: atmosphere, airspeed, altimetry, wind, performance, loading.

use gp_base::tool::Registry;

pub static REGISTRY: Registry = Registry {
    module: "aviation",
    tools: &[],
};

gp_base::export_module!("aviation", REGISTRY);
