//! Wasmtime engine and per-station Store factory.
//!
//! One Engine is built at startup and shared (it's thread-safe).
//! One Store per station instance, kept warm between tune() calls.

use anyhow::Result;
use wasmtime::{Config, Engine};

/// Build the shared Wasmtime engine.
///
/// - async support: required for tokio integration
/// - component model: we load .wasm components
/// - epoch interruption: kills runaway stations (bumped every 100ms by scheduler)
/// - fuel: secondary CPU cap per tune() call (set generously; tune later)
pub fn build_engine() -> Result<Engine> {
    let mut config = Config::new();
    config
        .async_support(true)
        .wasm_component_model(true)
        .epoch_interruption(true)
        .consume_fuel(true);
    Engine::new(&config)
}

// TODO M1: uncomment once wit/deps/ is populated by `wkg wit fetch`
//
// wasmtime::component::bindgen!({
//     world: "radio-station",
//     path:  "../../wit",
//     async: true,
//     with: {
//         "wasi:http/types@0.2.3":   wasmtime_wasi_http::bindings::wasi::http::types,
//         "wasi:io/streams@0.2.3":   wasmtime_wasi::bindings::wasi::io::streams,
//         "wasi:io/poll@0.2.3":      wasmtime_wasi::bindings::wasi::io::poll,
//     },
//     trappable_imports: true,
// });
