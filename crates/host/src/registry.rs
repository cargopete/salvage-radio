//! Station registry: loads .wasm components, instantiates them, calls describe().
//!
//! Failures during load are logged but don't abort startup — a single broken
//! station shouldn't take the radio off the air.

use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use wasmtime::{Engine, Store};
use wasmtime::component::{Component, Linker};
use wasmtime::component::HasSelf;

use crate::cache::Cache;
use crate::runtime::{RadioStation, Signal, StationData};

/// A loaded, instantiated station ready to be handed to the scheduler.
pub struct LoadedStation {
    pub callsign:       String,
    pub name:           String,
    pub description:    String,
    pub frequency:      String,
    pub cadence_secs:   u32,
    pub declared_hosts: Vec<String>,
    // wasmtime runtime state — kept alive so tune() shares in-memory cache
    store:    Store<StationData>,
    instance: RadioStation,
}

impl LoadedStation {
    /// Call the station's tune() export.
    /// The store (and therefore the in-memory cache) persists between calls.
    pub async fn tune(&mut self) -> Result<Signal> {
        self.store.set_epoch_deadline(50); // reset: 50 × 100ms = 5s budget
        let exports = self.instance.radio_station_station();
        let sig = exports.call_tune(&mut self.store).await?;
        Ok(sig)
    }
}

/// Build a shared linker for all stations.
/// Called once at startup; the linker is then reused for every component load.
pub fn build_linker(engine: &Engine) -> Result<Linker<StationData>> {
    let mut linker = Linker::new(engine);
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
    wasmtime_wasi_http::p2::add_only_http_to_linker_async(&mut linker)?;
    crate::runtime::radio::station::host_cache::add_to_linker::<_, HasSelf<_>>(&mut linker, |s| s)?;
    Ok(linker)
}

/// Load a single .wasm station component, instantiate it, call describe().
pub async fn load_station(
    engine: Arc<Engine>,
    linker: &Linker<StationData>,
    wasm_path: &Path,
    cache: Arc<Cache>,
) -> Result<LoadedStation> {
    let bytes = std::fs::read(wasm_path)
        .map_err(|e| anyhow::anyhow!("reading {}: {e}", wasm_path.display()))?;

    let component = Component::from_binary(&engine, &bytes)?;

    let mut store = Store::new(&engine, StationData::new("unknown", cache));
    store.set_epoch_deadline(50);

    let instance = RadioStation::instantiate_async(&mut store, &component, linker).await?;
    let exports = instance.radio_station_station();
    let meta = exports.call_describe(&mut store).await?;

    // Update callsign and declared_hosts now that we know them.
    // The guard is empty until this point — describe() must not make HTTP calls.
    store.data_mut().callsign = meta.callsign.clone();
    store.data_mut().guard.declared_hosts = meta.declared_hosts.clone();

    tracing::info!(
        callsign = %meta.callsign,
        freq     = %meta.frequency,
        "station loaded",
    );

    Ok(LoadedStation {
        callsign:       meta.callsign,
        name:           meta.name,
        description:    meta.description,
        frequency:      meta.frequency,
        cadence_secs:   meta.cadence_seconds,
        declared_hosts: meta.declared_hosts,
        store,
        instance,
    })
}

