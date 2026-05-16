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
#[station]
struct TechStation;

impl Station for TechStation {
    fn describe() -> Metadata {
        Metadata {
            callsign:        "TECH".into(),
            name:            "Tech".into(),
            description:     "Engineering deep-dives. HN >= 150 points, lobste.rs >= 10 votes.".into(),
            frequency:       "104.7".into(),
            operator:        "Pete".into(),
            cadence_seconds: 600,
            declared_hosts:  vec!["hnrss.org".into(), "lobste.rs".into()],
        }
    }

    fn tune() -> Signal {
        let cache = Cache::new();
        let last_seen: Vec<String> = cache.get_json("last_seen").unwrap_or_default();

        let items = match fetch_all() {
            Ok(items) => items,
            Err(e) => return Signal::OffAir(format!("fetch failed: {e}")),
        };

        match items.into_iter().find(|i| !last_seen.contains(&i.id)) {
            Some(item) => {
                // update cache, return broadcast
                Signal::OnAir(item.into())
            }
            None => Signal::Static,
        }
    }
}
```

Stations are distributed as single `.wasm` files. Drop one into the directory, add a line to `stations.toml`, and it's on the dial.

---

## The seven stations

| Callsign   | Freq  | Sources                                              |
|------------|-------|------------------------------------------------------|
| `BG-POL`   | 88.1  | Mediapool, Dnevnik, Capital, Sega                    |
| `PASTORAL` | 92.4  | Small-press essays, country-life blogs — max 2/day   |
| `AI`       | 96.7  | Anthropic, DeepMind, Meta AI, ArXiv cs.LG            |
| `TECH`     | 104.7 | HN ≥150pts, lobste.rs ≥10 votes                      |
| `WEB3`     | 108.3 | Graph Foundation, Vitalik, Paradigm, EF               |
| `WORLD-POL`| 116.5 | Reuters, Al Jazeera, FT, Guardian                    |
| `ON-THIS`  | 112.0 | Wikipedia "on this day" — one event per 6 hours      |

`PASTORAL` is the soul of the project. Its dial indicator will most often read `○` — quiet. When something arrives, it was worth waiting for.

`ON-THIS` demonstrates that stations don't have to be RSS-shaped. It fetches one Wikipedia event per call and emits it as a broadcast.

---

## Status

Currently working toward **M0**.

| Milestone | Scope                                            | Status        |
|-----------|--------------------------------------------------|---------------|
| M0        | WIT compiles; tech station builds to `.wasm`     | in progress   |
| M1        | Headless host; loads station; stdout output      | todo          |
| M2        | Pastoral station; WIT survives heterogeneous use | todo          |
| M3        | TUI shell — layout, palette, keybindings         | todo          |
| M4        | Polish — warmup sequence, flicker, quit animation| todo          |
| M5        | All seven stations                               | todo          |
| M6        | Hardening — epoch interruption, sandboxing       | todo          |
| M7        | Cache persistence across restarts                | todo          |
| M8        | Station authoring guide; release                 | todo          |

No deadlines. This is a hobby project.

---

## Requirements

- Rust stable with `wasm32-wasip2` target
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
# First time after cloning — fetches WASI WIT deps into wit/deps/
yatr fetch-wit

# Build host binary + all station components
yatr build

# Run
yatr run
```

```sh
# M0 acceptance: build tech station and inspect its exported interface
yatr m0
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
stations.toml           # dial order and .wasm paths
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

## Design notes

The full design rationale — the WIT interface decisions, capability sandbox, TUI aesthetic, station editorial philosophy — is in [`docs/rfc-001.md`](docs/rfc-001.md).

The short version: the aesthetic is **recovered industrial equipment**, not steampunk costume. Brass, not gold. The palette has ten colours; nine of them are earth tones. The tenth (verdigris) appears in one place. There are exactly two pieces of motion in the running application: a warm-up sequence on startup and a one-frame flicker when a new broadcast arrives on the tuned station. Both are there because they earn their keep. Everything else is still.
