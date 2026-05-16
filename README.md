# Salvage Radio

```
╔══════════════════════════════════════════════════════════════════════════╗
║ SALVAGE RADIO                                                  1430 GMT  ║
╠══════════════╦═══════════════════════════════════════════════════════════╣
║ STATIONS     ║ NOW BROADCASTING ── TECH ── 104.7                         ║
║              ║                                                           ║
║ ● TECH       ║ Linus weighs in on the io_uring rewrite                   ║
║ ● BG-POL     ║                                                           ║
║ ○ AI         ║ A proposal to refactor the io_uring submission path       ║
║ ● WEB3       ║ landed on LKML this morning. Linus responded within four  ║
║ ● PASTORAL   ║ hours, in a tone that maintainers will recognize. The     ║
║ × WORLD-POL  ║ thread is short but instructive...                        ║
║ ● ON-THIS    ║                                                           ║
║              ║ ── LKML · 2 hours ago · 4 min read                        ║
╠══════════════╩═══════════════════════════════════════════════════════════╣
║      88.1   92.4   96.7  ▼ 104.7   108.3   112.0   116.5                 ║
║       ·      ·      ·     ╪╪╪       ·       ·       ·                    ║
║       BG    PAST   AI    TECH      W3      WPOL    OTOD                  ║
╠══════════════════════════════════════════════════════════════════════════╣
║ [←/→] tune   [↑/↓] scroll   [enter] open   [q] off                       ║
╚══════════════════════════════════════════════════════════════════════════╝
```

A terminal feed aggregator built on the [WebAssembly Component Model](https://component-model.bytecodealliance.org/) (WASIp2).

Feeds as radio stations. Each station is a `.wasm` component with its own editorial logic. You tune in, listen for a while, tune out. Missing items is normal — radio doesn't replay everything you missed while the set was off.

---

## The idea

Most feed readers are inboxes. They count unread items. They make you feel behind. They reward the compulsive and punish the absent.

Salvage Radio is a radio. You tune to a station and hear what's on. If you weren't listening, you weren't listening. The station was broadcasting either way.

The inbox model optimises for completeness. The radio model optimises for presence.

---

## How it works

The host is a single native binary: a [ratatui](https://github.com/ratatui/ratatui) TUI backed by [wasmtime](https://wasmtime.dev/) and tokio. It loads station components from a directory, calls `tune()` on each one's declared cadence, and surfaces new broadcasts to the currently-tuned panel.

Each station is a `.wasm` component built against a minimal WIT interface:

```wit
// wit/station.wit (abbreviated)

interface station {
    describe: func() -> metadata;   // called once at startup
    tune:     func() -> signal;     // called on cadence
}

variant signal {
    on-air(broadcast),   // something fresh
    %static,             // quiet but healthy
    off-air(string),     // broken; reason given
}
```

Stations get `wasi:http` for outgoing requests (scoped to their declared hosts), `wasi:clocks`, and a small namespaced key-value cache. No filesystem. No environment. No sockets. A station that misbehaves gets epoch-interrupted; it cannot affect other stations or the host.

---

## Writing a station

A basic station is under 100 lines. The reference implementation (`crates/stations/tech/`) shows the full shape:

```rust
mod bindings;

use bindings::exports::radio::station::station::{Broadcast, Guest, Metadata, Signal};
use bindings::radio::station::host_cache;
use salvage_sdk::{html_to_text, parse_rss};

struct TechStation;

impl Guest for TechStation {
    fn describe() -> Metadata {
        Metadata {
            callsign:        "TECH".to_string(),
            name:            "Tech".to_string(),
            description:     "Engineering deep-dives.".to_string(),
            frequency:       "104.7".to_string(),
            operator:        "Pete".to_string(),
            cadence_seconds: 600,
            declared_hosts:  vec!["hnrss.org".to_string()],
        }
    }

    fn tune() -> Signal {
        // deduplication via the host-supplied key-value cache
        let last_seen: Vec<String> = host_cache::get("last_seen")
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();

        let bytes = match http_get("https://hnrss.org/frontpage?points=150") {
            Ok(b) => b,
            Err(e) => return Signal::OffAir(format!("fetch failed: {e}")),
        };

        let mut items = parse_rss(&bytes).unwrap_or_default();
        items.retain(|i| !last_seen.contains(&i.id));

        match items.into_iter().next() {
            Some(item) => {
                let mut seen = last_seen;
                seen.push(item.id.clone());
                seen.truncate(200);
                host_cache::set("last_seen", &serde_json::to_vec(&seen).unwrap());
                Signal::OnAir(Broadcast {
                    id: item.id, title: item.title,
                    body: html_to_text(&item.body_html),
                    source: item.source_name, permalink: item.permalink,
                    published: item.published, tags: vec![],
                })
            }
            None => Signal::Static,
        }
    }
}

bindings::export!(TechStation with_types_in bindings);
```

Outgoing HTTP is done via `wasi:http/outgoing-handler` (synchronous polling — no async executor needed). The station gets no filesystem, no environment, no arbitrary sockets. Only the hosts listed in `declared_hosts` are reachable (enforced by the host).

Stations are distributed as single `.wasm` files. See [`docs/authoring-guide.md`](docs/authoring-guide.md) for the full process of building and registering one.

---

## The seven stations

| Callsign   | Freq  | Sources                                              |
|------------|-------|------------------------------------------------------|
| `BG-POL`   | 88.1  | Mediapool, Dnevnik, Capital, Sega                    |
| `PASTORAL` | 92.4  | Small-press essays, country-life blogs — max 2/day   |
| `AI`       | 96.7  | ArXiv cs.AI/cs.LG, Hugging Face Blog                 |
| `TECH`     | 104.7 | HN ≥150pts, lobste.rs ≥10 votes                      |
| `WEB3`     | 108.3 | Vitalik, EF Blog, Week in Ethereum                   |
| `WORLD-POL`| 116.5 | Reuters, Al Jazeera, Guardian                        |
| `ON-THIS`  | 112.0 | Wikipedia "on this day" — one event per 6 hours      |

`PASTORAL` is the soul of the project. Its dial indicator will most often read `○` — quiet. When something arrives, it was worth waiting for.

`ON-THIS` demonstrates that stations don't have to be RSS-shaped. It fetches one Wikipedia event per call and emits it as a broadcast.

---

## Status

All milestones complete. **v0.1.0.**

| Milestone | Scope                                            | Status        |
|-----------|--------------------------------------------------|---------------|
| M0        | WIT compiles; tech station builds to `.wasm`     | done          |
| M1        | Headless host; loads station; stdout output      | done          |
| M2        | Pastoral station; WIT survives heterogeneous use | done          |
| M3        | TUI shell — layout, palette, keybindings         | done          |
| M4        | Polish — warmup sequence, flicker, quit animation| done          |
| M5        | All seven stations                               | done          |
| M6        | Hardening — epoch interruption, sandboxing       | done          |
| M7        | Cache persistence across restarts                | done          |
| M8        | Station authoring guide; release                 | done          |

This is a hobby project.

---

## Requirements

- Rust stable with `wasm32-wasip1` target
- [`cargo-component`](https://github.com/bytecodealliance/cargo-component)
- [`wkg`](https://github.com/bytecodealliance/wkg)
- [`wasm-tools`](https://github.com/bytecodealliance/wasm-tools)
- [`yatr`](https://github.com/cargopete/yatr)

```sh
yatr setup
```

---

## Building

```sh
# Build host binary + all station components
yatr build

# Run the TUI (M3+)
yatr run
```

```sh
# Milestone acceptance checks
yatr m0   # build tech station, inspect exported WIT interface
yatr m1   # headless: load TECH, call tune(), print to stdout
yatr m2   # headless: load PASTORAL — same WIT contract, different editorial logic
```

---

## Repository layout

```
wit/
  station.wit           # the WIT interface — the most important file in the project
  deps/                 # WASI deps, populated by wkg (gitignored)
crates/
  host/                 # native binary (wasmtime + ratatui + tokio)
    src/
      runtime.rs        # wasmtime engine, Store factory, bindgen!
      registry.rs       # load .wasm, instantiate, describe()
      scheduler.rs      # per-station tokio task, epoch ticker
      cache.rs          # host-cache impl (sled, namespaced by callsign)
      http_guard.rs     # declared-hosts enforcement
      tui/              # ratatui widgets: dial, now_playing, station_list, theme
  station-sdk/          # helpers for station authors (wasm32 library)
  stations/
    tech/               # TECH 104.7 — reference implementation
    pastoral/           # PASTORAL 92.4
    ai/                 # AI 96.7
    bg-pol/             # BG-POL 88.1
    web3/               # WEB3 108.3
    on-this/            # ON-THIS 112.0
    world-pol/          # WORLD-POL 116.5
docs/
  rfc-001.md            # design rationale
  authoring-guide.md    # how to write a station
yatr.toml               # build system
```

---

## Keybindings

| Key       | Action                                       |
|-----------|----------------------------------------------|
| `←` / `→` | Tune to previous / next station              |
| `↑` / `↓` | Scroll within current broadcast              |
| `[` / `]` | Previous / next broadcast in station buffer  |
| `Enter`   | Open permalink in `$BROWSER`                 |
| `r`       | Force refresh current station                |
| `t`       | Toggle tag visibility                        |
| `h/j/k/l` | Vim aliases                                  |
| `q`       | Turn off the set                             |
| `?`       | Help overlay                                 |

---

## Further reading

- [`docs/rfc-001.md`](docs/rfc-001.md) — full design rationale: WIT interface decisions, capability sandbox, TUI aesthetic, station editorial philosophy
- [`docs/authoring-guide.md`](docs/authoring-guide.md) — how to write and register a new station

The short version: the aesthetic is **recovered industrial equipment**, not steampunk costume. Brass, not gold. The palette has ten colours; nine of them are earth tones. The tenth (verdigris) appears in one place. There are exactly two pieces of motion in the running application: a warm-up sequence on startup and a one-frame flicker when a new broadcast arrives on the tuned station. Both are there because they earn their keep. Everything else is still.
