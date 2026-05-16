//! Per-station tokio task: calls tune() on cadence, forwards signals to the TUI.
//!
//! One task per station. Seen-set per station (capped at 1000 IDs, persisted via sled).
//! Epoch interruption: scheduler bumps engine epoch every 100ms.
//! Stations get 50 ticks (5s) per tune() call before being killed.

use tokio::sync::mpsc;

use crate::registry::LoadedStation;
use crate::runtime::Signal;

/// Events sent from the scheduler to the TUI layer.
#[derive(Debug)]
pub enum Event {
    Broadcast {
        callsign:  String,
        id:        String,
        title:     String,
        body:      String,
        source:    String,
        permalink: String,
        published: u64,
        tags:      Vec<String>,
    },
    /// Station is healthy but has nothing new.
    Quiet(String),
    /// Station is failing.
    OffAir { callsign: String, reason: String },
    /// Internal error calling tune().
    Error { callsign: String, message: String },
}

/// M1 headless mode: call tune() once and print the result to stdout.
pub async fn headless_tune(station: &mut LoadedStation) {
    println!("[{}] {} @ {} MHz — calling tune()…", station.callsign, station.name, station.frequency);
    match station.tune().await {
        Ok(Signal::OnAir(b)) => {
            println!();
            println!("  ── ON AIR ──────────────────────────────────");
            println!("  {}", b.title);
            println!("  {} · {}", b.source, b.permalink);
            println!();
            let body_preview: String = b.body.chars().take(280).collect();
            for line in body_preview.lines().take(5) {
                println!("  {line}");
            }
            println!();
        }
        Ok(Signal::Static) => {
            println!("  [static] — nothing new");
        }
        Ok(Signal::OffAir(reason)) => {
            println!("  [off-air] {reason}");
        }
        Err(e) => {
            println!("  [error] tune() failed: {e:#}");
        }
    }
}

/// Spawn a tokio task that calls tune() on cadence and sends Events to the TUI.
pub fn spawn_station_task(mut station: LoadedStation, tx: mpsc::Sender<Event>) {
    let cadence = std::time::Duration::from_secs(station.cadence_secs as u64);
    tokio::spawn(async move {
        loop {
            let event = match station.tune().await {
                Ok(Signal::OnAir(b)) => Event::Broadcast {
                    callsign:  station.callsign.clone(),
                    id:        b.id,
                    title:     b.title,
                    body:      b.body,
                    source:    b.source,
                    permalink: b.permalink,
                    published: b.published,
                    tags:      b.tags,
                },
                Ok(Signal::Static) => Event::Quiet(station.callsign.clone()),
                Ok(Signal::OffAir(reason)) => Event::OffAir {
                    callsign: station.callsign.clone(),
                    reason,
                },
                Err(e) => Event::Error {
                    callsign: station.callsign.clone(),
                    message:  e.to_string(),
                },
            };

            if tx.send(event).await.is_err() {
                break; // TUI has dropped the receiver — we're shutting down
            }

            tokio::time::sleep(cadence).await;
        }
    });
}

/// Start the epoch bump loop. Call once after building the Engine.
/// Each tick allows 100ms of station execution; stations get 50 ticks (5s) per call.
pub fn spawn_epoch_ticker(engine: std::sync::Arc<wasmtime::Engine>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
        loop {
            interval.tick().await;
            engine.increment_epoch();
        }
    });
}
