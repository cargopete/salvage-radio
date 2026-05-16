# Salvage Radio

A terminal-based feed aggregator built on the WebAssembly Component Model (WASIp2).

Feeds as radio stations. Each station is a `.wasm` component. You tune in, listen for a while, tune out. Missing items is normal — radio doesn't replay everything you missed while the set was off.

See [`docs/rfc-001.md`](docs/rfc-001.md) for full design rationale.

---

## Status

Working toward **M0**: WIT compiles, tech station builds to `.wasm`.

| Milestone | Scope                                       | Status  |
|-----------|---------------------------------------------|---------|
| M0        | WIT + tech station component builds         | in progress |
| M1        | Headless host, loads station, stdout output | todo    |
| M2        | Second station (pastoral), heterogeneous WIT| todo    |
| M3        | TUI shell — layout, palette, keybindings    | todo    |
| M4        | Polish — warmup, flicker, quit animation    | todo    |
| M5        | All seven stations                          | todo    |
| M6        | Hardening — epoch interruption, sandboxing  | todo    |
| M7        | Cache persistence across restarts           | todo    |
| M8        | Docs, station authoring guide, release      | todo    |

---

## Requirements

- Rust stable + `wasm32-wasip2` target
- [`cargo-component`](https://github.com/bytecodealliance/cargo-component)
- [`wkg`](https://github.com/bytecodealliance/wkg)
- [`wasm-tools`](https://github.com/bytecodealliance/wasm-tools) (for inspection)

```sh
just setup   # installs cargo-component, wkg, wasm-tools; adds wasm32-wasip2 target
```

## Building

```sh
just fetch-wit      # populate wit/deps/ from WASI registry (once per clone)
just build          # host binary + all station .wasm files
just run            # run the host
```

## M0 acceptance

```sh
just m0             # fetch WIT deps, build tech station, show exported interface
```

## Workspace layout

```
Cargo.toml              # workspace (host only; stations use cargo-component)
wit/
  station.wit           # the WIT interface — the most important file in the project
  deps/                 # populated by wkg (gitignored)
crates/
  host/                 # the native binary (ratatui + wasmtime + tokio)
  station-sdk/          # helpers for station authors (wasm32 library)
  stations/
    tech/               # TECH 104.7 — reference implementation
stations.toml           # which .wasm files to load and their dial positions
```

## Writing a station

See `docs/rfc-001.md` §Station SDK and Appendix A.
A basic station is under 100 lines including imports.
