# hlscreen

[![CI](https://github.com/s1korrrr/hlscreen/actions/workflows/ci.yml/badge.svg)](https://github.com/s1korrrr/hlscreen/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust 1.88+](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](rust-toolchain.toml)

`hlscreen` is a read-only Rust workspace for Hyperliquid spot market-data recording, replay, feature calculation, and terminal screening.

It is built for operators and researchers who want a local-first way to inspect public Hyperliquid spot microstructure without touching wallets, private keys, account streams, or order endpoints.

## Status

Current state: v0.1 live public-data hardening with a next-generation deterministic terminal market board, why-ranked detail pane, and health panel.

Implemented today:

- Public Hyperliquid REST metadata parsing for `spotMeta` and `spotMetaAndAssetCtxs`.
- Public WebSocket parsing for trades, BBO, all-mids, active asset context, and candles, with deterministic fixtures kept for tests.
- Bounded public WebSocket live screen with duration-based shutdown, heartbeat pings, reconnect/resubscribe, optional raw/normalized recording, and all-symbol subscription budgeting.
- Bounded live recording through a fail-closed writer queue so disk I/O does not silently drop or stall market-data ingestion.
- Live terminal refresh for TTY sessions and `--tui` smoke captures.
- Modern deterministic terminal rendering for market rows, scan KPIs, selected-symbol microstructure detail, read-only safety state, and operations health.
- Confidence-aware feature snapshots and TUI rows for fresh, sparse, duplicate, and explicit gap/parser/backlog quality inputs.
- Persisted confidence baselines plus `hls replay --verify-parity` drift detection for local replay checks.
- Deterministic score breakdowns, screen-rule score fields, and `hls explain` why-ranked output for replayed or fixture-backed rows.
- Compressed raw public message recording, normalized replay JSONL, and local SQLite metadata.
- Deterministic screening DSL and built-in screen presets.
- Health snapshots, reconnect simulation, TUI health rendering, and read-only local API helpers.

Not implemented yet:

- Automatic REST backfill for missed public data after a reconnect. Reconnect gaps are recorded explicitly.
- Long-running localhost HTTP server loop.
- True Parquet writer.
- Release binaries.

## Screenshots

These committed SVGs are deterministic terminal captures generated from the current binary and used for documentation regression. Real public WebSocket smoke evidence is tracked in the dated reports under [docs/reports](docs/reports/).

### Live Market Board

![Live market board](docs/assets/screenshots/live-screen.svg)

### Data Confidence Pane

![Data confidence pane](docs/assets/screenshots/confidence-degraded.svg)

### Liquidity Resilience Board

![Liquidity resilience board](docs/assets/screenshots/resilience-screen.svg)

### Why Ranked Detail

![Why ranked detail](docs/assets/screenshots/why-ranked.svg)

### Record And Replay

![Record and replay](docs/assets/screenshots/record-replay.svg)

### Health JSON

![Read-only health JSON](docs/assets/screenshots/health-json.svg)

### Health Panel

![Operations health panel](docs/assets/screenshots/health-panel.svg)

### Symbol Metadata

![Spot symbol metadata](docs/assets/screenshots/symbols.svg)

Regenerate these assets with:

```bash
python3 scripts/generate-screenshots.py
```

## Safety Boundary

`hlscreen` is read-only market-data infrastructure.

It does not provide:

- Wallet connection.
- Private-key handling.
- Order placement.
- Cancel/withdrawal/exchange-action routes.
- Leverage or execution controls.
- Financial advice.
- Profitability claims.

Scores and presets are screening heuristics only. They are not signals, recommendations, or strategy proof.

## Quick Start

Requirements:

- Rust 1.88 or newer.
- A network connection for public REST metadata and live public WebSocket commands.

Build:

```bash
cargo build --workspace --all-features
```

Run the local validation gate:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Initialize a local data directory:

```bash
./target/debug/hls init --data-dir /tmp/hlscreen-smoke
./target/debug/hls doctor --data-dir /tmp/hlscreen-smoke
```

Fetch read-only public spot metadata:

```bash
./target/debug/hls symbols --top 5
```

Run bounded public live screen for the current spot universe:

```bash
tmpdir="$(mktemp -d /tmp/hlscreen-live.XXXXXX)"
./target/debug/hls live \
  --all-symbols \
  --duration-secs 900 \
  --refresh-secs 60 \
  --tui \
  --record \
  --raw \
  --normalized \
  --run-id allpairs-15m \
  --data-dir "$tmpdir"
./target/debug/hls replay --data-dir "$tmpdir" --run-id allpairs-15m
./target/debug/hls replay --data-dir "$tmpdir" --run-id allpairs-15m --verify-parity
```

Run a short public live smoke for one symbol:

```bash
./target/debug/hls live \
  --symbols @107 \
  --duration-secs 30 \
  --refresh-secs 5 \
  --tui \
  --record \
  --raw \
  --normalized \
  --run-id one-symbol-live \
  --data-dir "$(mktemp -d /tmp/hlscreen-live.XXXXXX)"
```

Run deterministic fixture commands for tests or offline docs:

```bash
./target/debug/hls live \
  --symbols @107 \
  --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson \
  --preset thin_books \
  --once
```

Record and replay deterministic fixture data:

```bash
tmpdir="$(mktemp -d /tmp/hlscreen-smoke.XXXXXX)"
./target/debug/hls record \
  --symbols @107 \
  --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson \
  --raw \
  --normalized \
  --run-id smoke \
  --data-dir "$tmpdir"
./target/debug/hls replay --data-dir "$tmpdir" --run-id smoke
./target/debug/hls replay --data-dir "$tmpdir" --run-id smoke --verify-parity
```

Screen deterministic fixture rows:

```bash
./target/debug/hls screen \
  --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson \
  --where 'spread_bps < 75 and tob_depth_usd > 100' \
  --sort ret_5m:desc
```

Explain why a replayed or fixture-backed symbol ranked:

```bash
./target/debug/hls explain \
  --fixture-file tests/fixtures/microstructure/resilience_shock.ndjson \
  --symbol @107
```

Print health JSON:

```bash
./target/debug/hls doctor --live --json
./target/debug/hls server --print-health
```

## Architecture

Workspace crates:

- `hls-core`: shared config, symbols, errors, state, health, and telemetry contracts.
- `hls-hyperliquid`: public Hyperliquid REST/WebSocket parsing and connection helpers.
- `hls-store`: compressed raw capture, normalized replay data, metadata registry, and replay readers.
- `hls-features`: rolling feature windows and formulas.
- `hls-screen`: screening DSL, presets, and row filtering/sorting.
- `hls-tui`: terminal rendering.
- `hls-server`: read-only local API response helpers.
- `hls-cli`: command routing and operator workflows.

See [docs/architecture.md](docs/architecture.md).

## Data Files

Local recording writes under the configured data directory:

- `raw/ws/run=<run-id>/part-*.ndjson.zst`
- `normalized/events/run=<run-id>/part-*.ndjson`
- `hls.sqlite`

These files are local artifacts and should not be committed.

See [docs/data-format.md](docs/data-format.md) and [docs/PRIVACY.md](docs/PRIVACY.md).

## Screening Rules

The screening DSL supports:

- Boolean operators: `and`, `or`
- Comparisons: `>`, `>=`, `<`, `<=`, `==`, `!=`
- Literals: numbers, strings, booleans
- Function: `abs(field)` for numeric fields
- Sort syntax: `field:asc`, `field:desc`, `abs(field):asc`, `abs(field):desc`

Examples are in [examples/screen-rules.md](examples/screen-rules.md).

## Documentation

- [Architecture](docs/architecture.md)
- [Data format](docs/data-format.md)
- [Feature definitions](docs/feature-definitions.md)
- [Threat model](docs/THREAT_MODEL.md)
- [Privacy](docs/PRIVACY.md)
- [Roadmap](docs/ROADMAP.md)
- [Release checklist](docs/RELEASING.md)
- [Open source checklist](docs/OPEN_SOURCE_CHECKLIST.md)
- [Live production hardening report](docs/reports/2026-07-08-live-production-hardening.md)
- [Live smoke report](docs/reports/2026-07-08-live-smoke.md)
- [Pre-merge audit](docs/reports/2026-07-08-pre-merge-audit.md)

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a PR.

The short version:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
git diff --check
```

Security issues should follow [SECURITY.md](SECURITY.md). General support guidance is in [SUPPORT.md](SUPPORT.md).

## License

MIT. See [LICENSE](LICENSE).
