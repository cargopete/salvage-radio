//! Wasmtime engine and per-station Store factory.
//!
//! One Engine is built at startup and shared (it's thread-safe).
//! One Store per station instance, kept warm between tune() calls.

use anyhow::Result;
use std::sync::Arc;
use wasmtime::{Config, Engine};
use wasmtime::component::ResourceTable;
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};
use wasmtime_wasi_http::{WasiHttpCtx, p2::{WasiHttpCtxView, WasiHttpView}};

use crate::cache::Cache;
use crate::http_guard::DeclaredHostsGuard;

wasmtime::component::bindgen!({
    world: "radio-station",
    path:  "../../wit",
    imports: { default: trappable },
    exports: { default: async },
    require_store_data_send: true,
    with: {
        // WASI base: map entire packages to wasmtime_wasi's generated bindings
        "wasi:io":     wasmtime_wasi::p2::bindings::io,
        "wasi:clocks": wasmtime_wasi::p2::bindings::clocks,

        // HTTP resource types: map each individually to wasmtime_wasi_http types
        "wasi:http/types.fields":                    wasmtime_wasi_http::FieldMap,
        "wasi:http/types.outgoing-request":          wasmtime_wasi_http::p2::types::HostOutgoingRequest,
        "wasi:http/types.request-options":           wasmtime_wasi_http::p2::types::HostRequestOptions,
        "wasi:http/types.future-incoming-response":  wasmtime_wasi_http::p2::types::HostFutureIncomingResponse,
        "wasi:http/types.incoming-response":         wasmtime_wasi_http::p2::types::HostIncomingResponse,
        "wasi:http/types.incoming-body":             wasmtime_wasi_http::p2::body::HostIncomingBody,
        "wasi:http/types.future-trailers":           wasmtime_wasi_http::p2::body::HostFutureTrailers,
        "wasi:http/types.outgoing-body":             wasmtime_wasi_http::p2::body::HostOutgoingBody,
        "wasi:http/types.outgoing-response":         wasmtime_wasi_http::p2::types::HostOutgoingResponse,
        "wasi:http/types.incoming-request":          wasmtime_wasi_http::p2::types::HostIncomingRequest,
        "wasi:http/types.response-outparam":         wasmtime_wasi_http::p2::types::HostResponseOutparam,
    },
});

// Re-export the generated signal types so other modules can pattern-match.
pub use exports::radio::station::station::Signal;

/// Build the shared Wasmtime engine.
///
/// - async support: required for tokio integration
/// - component model: we load .wasm components
/// - epoch interruption: kills runaway stations (bumped every 100ms by scheduler)
pub fn build_engine() -> Result<Engine> {
    let mut config = Config::new();
    config
        .wasm_component_model(true)
        .epoch_interruption(true);
    Ok(Engine::new(&config)?)
}

/// Per-station Store state. One instance per loaded station.
pub struct StationData {
    pub wasi:     WasiCtx,
    pub http:     WasiHttpCtx,
    pub table:    ResourceTable,
    pub callsign: String,
    /// Persistent sled-backed cache, shared across all stations (namespaced by callsign).
    pub cache: Arc<Cache>,
    /// HTTP guard: blocks requests to hosts not in declared_hosts.
    /// Populated after describe() is called during station load.
    pub guard: DeclaredHostsGuard,
}

impl StationData {
    pub fn new(callsign: &str, cache: Arc<Cache>) -> Self {
        Self {
            wasi:     WasiCtxBuilder::new().build(),
            http:     WasiHttpCtx::new(),
            table:    ResourceTable::new(),
            callsign: callsign.to_string(),
            cache,
            guard:    DeclaredHostsGuard { declared_hosts: Vec::new() },
        }
    }
}

impl WasiView for StationData {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView { ctx: &mut self.wasi, table: &mut self.table }
    }
}

impl WasiHttpView for StationData {
    fn http(&mut self) -> WasiHttpCtxView<'_> {
        WasiHttpCtxView {
            ctx:   &mut self.http,
            table: &mut self.table,
            hooks: &mut self.guard,
        }
    }
}

/// Host-cache implementation: sled-backed persistent store, namespaced by callsign.
impl radio::station::host_cache::Host for StationData {
    fn get(&mut self, key: String) -> Result<Option<Vec<u8>>, wasmtime::Error> {
        self.cache
            .get(&self.callsign, &key)
            .map_err(wasmtime::Error::new)
    }

    fn set(&mut self, key: String, value: Vec<u8>) -> Result<(), wasmtime::Error> {
        self.cache
            .set(&self.callsign, &key, &value)
            .map_err(wasmtime::Error::new)
    }
}
