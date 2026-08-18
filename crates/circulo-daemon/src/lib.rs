//! Local Circulo daemon HTTP API.

pub mod bind;
mod generate;
pub mod http;
mod model_catalog_cache;

pub use bind::{listen_addr, BindError, DEFAULT_ADDR};
pub use http::{router, AppState};
