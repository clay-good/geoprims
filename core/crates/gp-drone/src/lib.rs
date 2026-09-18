//! Drone: photogrammetry, mission patterns, endurance, operations references.

use gp_base::tool::Registry;

pub static REGISTRY: Registry = Registry {
    module: "drone",
    tools: &[],
};

gp_base::export_module!("drone", REGISTRY);
