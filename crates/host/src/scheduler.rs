//! Per-station tokio task: calls tune() on cadence, forwards signals to the TUI.
//!
//! One task per station. Seen-set per station (capped at 1000 IDs, persisted via sled).
//! Epoch interruption: scheduler bumps engine epoch every 100ms.
//! Stations get 50 ticks (5s) per tune() call before being killed.

use tokio::sync::mpsc;

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

// TODO M1: implement station_loop(station: LoadedStation, tx: mpsc::Sender<Event>)
//
// async fn station_loop(station: LoadedStation, tx: mpsc::Sender<Event>) {
//     let cadence = Duration::from_secs(station.cadence_secs as u64);
//     let mut interval = tokio::time::interval(cadence);
//     interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
//     let mut seen: HashSet<String> = load_seen_set(&station.callsign);
//
//     loop {
//         interval.tick().await;
//         match station.tune().await {
//             Ok(Signal::OnAir(b)) if !seen.contains(&b.id) => {
//                 seen.insert(b.id.clone());
//                 persist_seen_set(&station.callsign, &seen);
//                 tx.send(Event::Broadcast { ... }).await.ok();
//             }
//             Ok(Signal::Static)      => tx.send(Event::Quiet(...)).await.ok(),
//             Ok(Signal::OffAir(why)) => tx.send(Event::OffAir { ... }).await.ok(),
//             Err(e)                  => tx.send(Event::Error { ... }).await.ok(),
//             _ => {}
//         }
//     }
// }

/// Start the epoch bump loop. Call once after building the Engine.
pub fn spawn_epoch_ticker(engine: std::sync::Arc<wasmtime::Engine>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
        loop {
            interval.tick().await;
            engine.increment_epoch();
        }
    });
}
