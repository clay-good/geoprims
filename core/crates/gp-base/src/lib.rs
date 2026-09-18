//! gp-base: shared by every geoprims Wasm module. Units, angles, the error and
//! warning model, deterministic JSON, and the result envelope.

pub mod abi;
pub mod angle;
pub mod envelope;
pub mod error;
pub mod json;
pub mod num;
pub mod parse;
pub mod profile;
pub mod units;

pub use error::{ErrorCode, ToolError, Warning};
pub use json::Json;
