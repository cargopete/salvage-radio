//! Station registry: loads .wasm components, instantiates them, calls describe().
//!
//! Failures during load are logged but don't abort startup — a single broken
//! station shouldn't take the radio off the air.

use anyhow::Result;

/// A loaded, instantiated station ready to be handed to the scheduler.
pub struct LoadedStation {
    pub callsign:       String,
    pub name:           String,
    pub description:    String,
    pub frequency:      String,
    pub cadence_secs:   u32,
    pub declared_hosts: Vec<String>,
    // TODO M1: wasmtime Store<StationData>, StationPre<StationData>
}

/// Config entry from stations.toml.
#[derive(serde::Deserialize)]
pub struct StationConfig {
    pub path:     String,
    pub position: u32,
}

#[derive(serde::Deserialize)]
pub struct HostConfig {
    pub default_station: String,
    pub cache_dir:       String,
    pub state_dir:       String,
    pub log_level:       String,
}

#[derive(serde::Deserialize)]
pub struct StationsToml {
    pub station: Vec<StationConfig>,
    pub host:    HostConfig,
}

/// Load and instantiate all stations declared in the config file.
pub async fn load_stations(_config_path: &std::path::Path) -> Result<Vec<LoadedStation>> {
    // TODO M1:
    //   1. parse stations.toml
    //   2. for each entry: load .wasm bytes, component::Component::new(engine, bytes)
    //   3. build Linker, register wasi:http + wasi:clocks + host-cache impls
    //   4. instantiate, call describe(), store result
    //   5. log failures, continue
    Ok(vec![])
}
