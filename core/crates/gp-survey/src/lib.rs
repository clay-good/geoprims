//! Survey: COGO, traverse, reductions, earthwork, curves.

use gp_base::tool::Registry;

pub static REGISTRY: Registry = Registry {
    module: "survey",
    tools: &[],
};

gp_base::export_module!("survey", REGISTRY);
