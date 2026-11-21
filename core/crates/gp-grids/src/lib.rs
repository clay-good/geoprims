//! National grids (add-international-grids, geodesy/national-grids): the
//! official transformations onto other countries' grids, each one backed by
//! the grid file its issuer publishes.
//!
//! These are `geodesy.*` tools and they read as part of the geodesy domain;
//! they are their own Wasm module because together they are large, and the
//! geodesy module reached its 400 KB compressed budget
//! (establish-platform-foundation, compute-core: "Each domain module SHALL be
//! at most 400 KB compressed"). The split is by module only: every tool keeps
//! the id, page, and search entry it had, and the host loads this module for
//! them instead of the geodesy one.
//!
//! A domain module may not depend on another domain crate, or the Wasm link
//! breaks, so everything shared sits in `gp_geo`: the projections, the grid
//! readers (`hgrid`, `vgrid`), and the per-country math (`ostn15`, `osi`,
//! `rdnap`, `chenyx06`, `beta2007`, `ch1903`).

pub mod beta2007;
pub mod ch1903;
pub mod chenyx06;
pub mod nzgd;
pub mod osi;
pub mod ostn15;
pub mod rdnap;

use gp_base::tool::{Registry, ToolDef};

pub static TOOLS: &[&ToolDef] = &[
    &ostn15::OSTN15_FORWARD,
    &ostn15::OSTN15_INVERSE,
    &osi::OSI_POLYNOMIAL,
    &osi::OSI_POLYNOMIAL_INVERSE,
    &nzgd::NZGD2000,
    &beta2007::BETA2007_FORWARD,
    &beta2007::BETA2007_INVERSE,
    &ch1903::CH1903PLUS,
    &chenyx06::CHENYX06,
    &rdnap::RDNAP_FORWARD,
    &rdnap::RDNAP_INVERSE,
];

pub static REGISTRY: Registry = Registry {
    module: "national-grids",
    tools: TOOLS,
};

gp_base::export_module!("national-grids", REGISTRY);
