//! Spatial indexing: H3, S2, geohash, tiles, Plus Codes.

use gp_base::tool::Registry;

pub static REGISTRY: Registry = Registry {
    module: "indexing",
    tools: &[],
};

gp_base::export_module!("indexing", REGISTRY);
