# Salvage Radio build system
# Requires: cargo, cargo-component, wkg, wasm-tools

# ── Setup ────────────────────────────────────────────────────────────────────

# Fetch WASI WIT dependencies into wit/deps/ (run once after clone)
fetch-wit:
    wkg wit fetch

# Install required cargo subcommands if missing
setup:
    cargo install cargo-component wkg wasm-tools
    rustup target add wasm32-wasip2

# ── Host ─────────────────────────────────────────────────────────────────────

build-host:
    cargo build --release -p salvage-radio

check-host:
    cargo check -p salvage-radio

run:
    cargo run -p salvage-radio

# ── Stations ─────────────────────────────────────────────────────────────────

build-tech:
    cd crates/stations/tech && cargo component build --release

check-tech:
    cd crates/stations/tech && cargo component check

# Inspect the exported WIT interface of the built tech station
inspect-tech: build-tech
    wasm-tools component wit crates/stations/tech/target/wasm32-wasip2/release/station_tech.wasm

# ── All ──────────────────────────────────────────────────────────────────────

build: build-host build-tech

# ── Dev tooling ──────────────────────────────────────────────────────────────

fmt:
    cargo fmt -p salvage-radio
    cd crates/station-sdk  && cargo fmt
    cd crates/stations/tech && cargo fmt

lint:
    cargo clippy -p salvage-radio -- -D warnings
    cd crates/stations/tech && cargo component clippy -- -D warnings

# M0 acceptance: WIT compiles, station builds, interface visible
m0: fetch-wit build-tech inspect-tech
    @echo "M0 done."
