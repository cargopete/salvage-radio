mod cache;
mod http_guard;
mod registry;
mod runtime;
mod scheduler;
mod tui;

use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

use cache::Cache;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("salvage_radio=info".parse()?),
        )
        .without_time()
        .init();

    tracing::info!("Salvage Radio — warming the valves");

    // Open the persistent sled cache.
    // macOS: ~/Library/Application Support/salvage-radio/cache
    // Linux: ~/.local/share/salvage-radio/cache
    let cache_path = directories::ProjectDirs::from("", "", "salvage-radio")
        .map(|d| d.data_dir().join("cache"))
        .unwrap_or_else(|| PathBuf::from(".salvage-cache"));
    std::fs::create_dir_all(&cache_path)?;
    let cache = Arc::new(Cache::open(&cache_path)?);
    tracing::info!(path = %cache_path.display(), "cache open");

    let engine = Arc::new(runtime::build_engine()?);
    scheduler::spawn_epoch_ticker(Arc::clone(&engine));
    let linker = registry::build_linker(&engine)?;

    // Headless mode: `salvage-radio <path.wasm>` — used by M1/M2 acceptance tasks.
    if let Some(path) = std::env::args().nth(1).map(PathBuf::from) {
        let mut station = registry::load_station(Arc::clone(&engine), &linker, &path, Arc::clone(&cache)).await?;
        scheduler::headless_tune(&mut station).await;
        return Ok(());
    }

    // M3+: TUI mode — load all available stations and hand off to the TUI.
    // Sorted by frequency so the dial reads left-to-right.
    let wasm_paths: Vec<PathBuf> = vec![
        "crates/stations/bg-pol/target/wasm32-wasip1/release/station_bg_pol.wasm",
        "crates/stations/pastoral/target/wasm32-wasip1/release/station_pastoral.wasm",
        "crates/stations/ai/target/wasm32-wasip1/release/station_ai.wasm",
        "crates/stations/tech/target/wasm32-wasip1/release/station_tech.wasm",
        "crates/stations/web3/target/wasm32-wasip1/release/station_web3.wasm",
        "crates/stations/on-this/target/wasm32-wasip1/release/station_on_this.wasm",
        "crates/stations/world-pol/target/wasm32-wasip1/release/station_world_pol.wasm",
    ]
    .into_iter()
    .map(PathBuf::from)
    .filter(|p| p.exists())
    .collect();

    if wasm_paths.is_empty() {
        anyhow::bail!(
            "no station .wasm files found — run `yatr build` first, \
             or pass a wasm path as an argument for headless mode"
        );
    }

    // Load all stations.
    let mut loaded: Vec<registry::LoadedStation> = Vec::new();
    for path in &wasm_paths {
        match registry::load_station(Arc::clone(&engine), &linker, path, Arc::clone(&cache)).await {
            Ok(station) => loaded.push(station),
            Err(e) => tracing::warn!("skipping {}: {e:#}", path.display()),
        }
    }

    // Reset last_seen for every station before the drain so the list populates
    // with current content on each startup, regardless of previous runs.
    for station in &loaded {
        let _ = cache.set(&station.callsign, "last_seen", b"[]");
    }

    let (tx, rx) = mpsc::channel::<scheduler::Event>(256);
    let mut meta: Vec<tui::StationMeta> = Vec::new();

    for station in loaded {
        meta.push(tui::StationMeta {
            callsign:  station.callsign.clone(),
            name:      station.name.clone(),
            frequency: station.frequency.clone(),
        });
        scheduler::spawn_station_task(station, tx.clone());
    }

    if meta.is_empty() {
        anyhow::bail!("all station loads failed — check logs above");
    }

    // Sort by frequency (ascending) so dial order matches the aesthetic layout.
    meta.sort_by(|a, b| {
        let fa: f32 = a.frequency.parse().unwrap_or(0.0);
        let fb: f32 = b.frequency.parse().unwrap_or(0.0);
        fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal)
    });

    tui::run(rx, meta).await
}
