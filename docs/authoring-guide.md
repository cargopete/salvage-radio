# Writing a Salvage Radio Station

A station is a single `.wasm` file that the host loads at startup. It implements two functions: `describe()` returns static metadata, and `tune()` returns whatever the station has to broadcast right now. Everything else — scheduling, rendering, deduplication across restarts — is handled by the host.

A basic station is under 100 lines.

---

## Prerequisites

```sh
# Rust stable (stations target wasm32-wasip1)
rustup target add wasm32-wasip1

# cargo-component: builds Rust crates into wasm components
cargo install cargo-component

# wkg: fetches WIT dependencies (run once per workspace)
cargo install wkg
```

---

## Creating a station

### 1. Add a crate

```sh
# From the salvage-radio workspace root:
cargo new --lib crates/stations/my-station
```

Edit `crates/stations/my-station/Cargo.toml` to opt in to the component model
and declare the WIT target:

```toml
[workspace]

[package]
name    = "station-my-station"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
salvage-sdk    = { path = "../../station-sdk" }
anyhow         = "1"
serde_json     = "1"
wit-bindgen-rt = { version = "0.44.0", features = ["bitflags"] }

[package.metadata.component]
package = "radio:station"

[package.metadata.component.target]
path  = "../../../wit"
world = "radio-station"

[package.metadata.component.target.dependencies]
"wasi:http"   = { path = "../../../wit/deps/wasi-http-0.2.3" }
"wasi:clocks" = { path = "../../../wit/deps/wasi-clocks-0.2.3" }
"wasi:io"     = { path = "../../../wit/deps/wasi-io-0.2.3" }
"wasi:random" = { path = "../../../wit/deps/wasi-random-0.2.3" }
"wasi:cli"    = { path = "../../../wit/deps/wasi-cli-0.2.3" }
```

The `[workspace]` key is required because `cargo-component` builds each station
as a separate workspace.

### 2. Write the station

`src/lib.rs`:

```rust
#[allow(warnings)]
mod bindings;

use bindings::exports::radio::station::station::{Broadcast, Guest, Metadata, Signal};
use bindings::radio::station::host_cache;
use salvage_sdk::{html_to_text, parse_rss};

struct MyStation;

impl Guest for MyStation {
    fn describe() -> Metadata {
        Metadata {
            callsign:        "MY-STATION".to_string(),
            name:            "My Station".to_string(),
            description:     "One sentence describing editorial focus.".to_string(),
            frequency:       "99.9".to_string(),    // aesthetic only, shown on dial
            operator:        "Your Name".to_string(),
            cadence_seconds: 3600,                  // how often tune() is called
            declared_hosts:  vec!["example.com".to_string()],
        }
    }

    fn tune() -> Signal {
        // 1. Load the last-seen deduplication list from the cache.
        let last_seen: Vec<String> = host_cache::get("last_seen")
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();

        // 2. Fetch and parse the feed.
        let bytes = match http_get("https://example.com/feed.rss") {
            Ok(b) => b,
            Err(e) => return Signal::OffAir(format!("fetch failed: {e}")),
        };

        let mut items = parse_rss(&bytes).unwrap_or_default();
        items.retain(|i| !last_seen.contains(&i.id));

        // 3. Surface one item and record it as seen.
        match items.into_iter().next() {
            Some(item) => {
                let mut seen = last_seen;
                seen.push(item.id.clone());
                seen.truncate(200);
                if let Ok(b) = serde_json::to_vec(&seen) {
                    host_cache::set("last_seen", &b);
                }
                Signal::OnAir(Broadcast {
                    id:        item.id,
                    title:     item.title,
                    body:      html_to_text(&item.body_html),
                    source:    item.source_name,
                    permalink: item.permalink,
                    published: item.published,
                    tags:      vec!["my-tag".to_string()],
                })
            }
            None => Signal::Static,
        }
    }
}

bindings::export!(MyStation with_types_in bindings);
```

### 3. Build it

Add a build task to `yatr.toml`:

```toml
[tasks.build-my-station]
desc    = "Build MY-STATION component (release)"
run     = ["cargo component build --release"]
cwd     = "crates/stations/my-station"
sources = ["src/**/*.rs", "Cargo.toml", "../../../Cargo.lock"]
outputs = ["target/wasm32-wasip1/release/station_my_station.wasm"]
```

Then build:

```sh
yatr build-my-station
# or directly:
cd crates/stations/my-station && cargo component build --release
```

### 4. Register it

Add the output path to `main.rs` in the `wasm_paths` vec (sorted by frequency):

```rust
"crates/stations/my-station/target/wasm32-wasip1/release/station_my_station.wasm",
```

Then add the station to the `[tasks.build]` `depends` list in `yatr.toml`.

---

## The WIT interface

The full interface is in `wit/station.wit`. The short version:

```wit
interface station {
    describe: func() -> metadata;
    tune:     func() -> signal;
}

variant signal {
    on-air(broadcast),   // something fresh — surface it
    %static,             // nothing new, station is healthy
    off-air(string),     // can't fetch — reason given, shown as ×
}
```

**`describe()`** is called once at startup. It must be fast — no I/O, no network.
Return your callsign, frequency (aesthetic only), cadence, and the list of
hostnames you intend to fetch from.

**`tune()`** is called on your declared cadence. Return `on-air` with one broadcast,
`%static` if there is nothing new, or `off-air` with a reason if something is
broken. The host shows `●` for on-air/static and `×` for off-air.

---

## HTTP fetching

Stations have no async executor. HTTP is done via `wasi:http/outgoing-handler`
with synchronous polling. The SDK does not yet wrap this, so copy the `http_get`
helper from `crates/stations/tech/src/lib.rs` — it is about 55 lines of
boilerplate that won't change.

Key points:

- Only hosts listed in `declared_hosts` are reachable. Requests to any other host
  are trapped by the host and turn into an `off-air` signal.
- `tune()` has a **5-second budget** (50 epoch ticks × 100 ms each). Slow feeds
  will cause the station to be interrupted and report off-air. Cache aggressively.
- Set a sensible `cadence_seconds`. Polling every 60 seconds is antisocial;
  600–3600 is typical.

---

## The host-cache

The cache survives process restarts (sled-backed). Use it for:

- **Deduplication** — the `last_seen` list pattern above
- **Per-call state** — e.g. ON-THIS uses it to track which Wikipedia events it
  has already surfaced
- **Daily budgets** — PASTORAL stores a unix-day and a count to enforce ≤2/day

```rust
// Read
let last_seen: Vec<String> = host_cache::get("last_seen")
    .and_then(|b| serde_json::from_slice(&b).ok())
    .unwrap_or_default();

// Write
if let Ok(b) = serde_json::to_vec(&updated_list) {
    host_cache::set("last_seen", &b);
}
```

Keys are automatically namespaced by your callsign — you cannot read another
station's entries.

The cache is **best-effort**: it can be wiped at any time (e.g. `rm -rf` the
data directory). Your station must handle a cold start gracefully; a missing
`last_seen` just means it may re-broadcast an item once.

---

## SDK reference

`salvage-sdk` is a `wasm32` library (no std I/O, no threads). Import it from
`../../station-sdk`.

| Item | What it does |
|------|-------------|
| `parse_rss(bytes: &[u8]) -> Result<Vec<RawItem>>` | Parses RSS and Atom feeds |
| `parse_atom(bytes: &[u8]) -> Result<Vec<RawItem>>` | Parses Atom only |
| `html_to_text(html: &str) -> String` | Strips tags, normalises whitespace |
| `RawItem` | Struct: `id`, `title`, `body_html`, `source_name`, `permalink`, `published` |

---

## Capability summary

| Capability | Available | Notes |
|-----------|-----------|-------|
| `wasi:http/outgoing-handler` | Yes | Only declared hosts. GET only is conventional. |
| `wasi:clocks/wall-clock` | Yes | Unix timestamps for date logic |
| `wasi:clocks/monotonic-clock` | Yes | Elapsed time |
| `host-cache` | Yes | Persistent k/v, namespaced by callsign |
| Filesystem | **No** | No `wasi:filesystem` in the world |
| Environment | **No** | No `wasi:cli/environment` |
| Sockets | **No** | Only HTTP via the outgoing handler |
| Spawning | **No** | No threads, no subprocesses |

---

## Editorial notes

A station is an editorial voice, not a firehose. Some things that make a good
station:

- **One item per call.** The host buffers broadcasts per station; surface one
  good thing rather than everything.
- **Earn the silence.** `%static` is a valid and expected response. A station
  that is quiet most of the time is working correctly, not broken.
- **Respect the radio metaphor.** The user was not listening while the set was
  off. You do not need to backfill. Surface what is current.
- **Cadence is editorial.** A 6-hour cadence (like ON-THIS) is a statement about
  how often the topic moves. A 10-minute cadence (like TECH) is a statement about
  how noisy the source is.

---

## Reference implementations

| Station | Source | Notes |
|---------|--------|-------|
| `crates/stations/tech/` | `hnrss.org`, `lobste.rs` | Reference: multi-source RSS merge, score filtering |
| `crates/stations/pastoral/` | Emergence Magazine, Orion | Daily budget (≤2/day), slow cadence |
| `crates/stations/on-this/` | Wikipedia REST API | Non-RSS: JSON API, date arithmetic, 6-hour windows |
| `crates/stations/bg-pol/` | Mediapool, Capital, Dnevnik | Multi-feed, partial-failure tolerant |
| `crates/stations/world-pol/` | Reuters, Al Jazeera, Guardian | High-cadence news |
