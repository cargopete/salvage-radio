mod cache;
mod http_guard;
mod registry;
mod runtime;
mod scheduler;
mod tui;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("salvage_radio=info".parse()?),
        )
        .without_time()
        .init();

    // M1: load stations.toml, instantiate stations, headless loop to stdout
    // M3: hand off to tui::run()
    tracing::info!("Salvage Radio — warming the valves");
    println!("Salvage Radio (scaffold — M0)");
    println!("Run `just build` to produce station .wasm files.");

    Ok(())
}
