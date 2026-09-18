//! Time: sun position, twilight, time scales.

use gp_base::tool::Registry;

pub static REGISTRY: Registry = Registry {
    module: "time",
    tools: &[],
};

gp_base::export_module!("time", REGISTRY);
