# Implementation Plan: Hyperliquid Spot Screener

**Branch**: `001-hyperliquid-spot-screener` | **Date**: 2026-07-07 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/001-hyperliquid-spot-screener/spec.md`

## Summary

Build `hlscreen` as a read-only Rust-native terminal screener and local recorder for Hyperliquid spot markets. The implementation centers on public market-data ingestion, append-only raw capture, normalized event storage, incremental rolling features, CLI/TUI workflows, a small screening DSL, operational health reporting, and replay from local recordings. v1 explicitly excludes wallet connection, trading, execution, AI/ML prediction, full order-book depth, backtesting, full web dashboard, and multi-exchange support.

## Technical Context

**Language/Version**: Rust stable, edition 2024 with `rust-version = "1.85"` for new crates.

**Primary Dependencies**: Cargo workspace; `tokio`; `tokio-tungstenite`; `reqwest` with rustls; `serde`; `serde_json`; `toml`; `clap`; `ratatui`; `crossterm`; `tracing`; `thiserror`; `anyhow`; `rust_decimal`; `arrow`; `parquet`; `zstd`; `rusqlite`; standard library parser utilities for the first DSL implementation.

**Storage**: Local filesystem for raw compressed newline-delimited JSON and normalized Parquet files; SQLite metadata database for symbols, files, recording runs, and data gaps.

**Testing**: `cargo test`; integration tests with mock REST and WebSocket servers; fixture-based parser tests; golden CLI/TUI rendering tests; replay equivalence smoke.

**Target Platform**: Local macOS and Linux terminals. No browser or hosted service required for v1.

**Project Type**: Multi-crate Rust CLI/TUI application with optional local read-only HTTP API.

**Performance Goals**: Support at least 150 selected spot markets in a live session, render the TUI at 5 Hz by default, mark stale rows within 10 seconds, keep WebSocket reading independent from disk and terminal rendering, and preserve enough telemetry to diagnose lag.

**Constraints**: Read-only public market data only; no private keys or wallet permissions; no order placement; bounded channels between ingestion, storage, feature, and UI tasks; subscription count remains below configured headroom; replay must work without live network access.

**Scale/Scope**: v1 handles one Hyperliquid environment at a time, one or two WebSocket connections by policy, top-by-volume universe selection capped below public subscription limits, local data files under a user-controlled directory, and feature windows up to 1 hour for live display with longer baselines allowed once recordings exist.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

The local `.specify/memory/constitution.md` is still the template and does not define project-specific gates. For this feature, the controlling gates come from the repo/HQ operating contract:

- **Read-only capital boundary**: PASS. v1 excludes wallet, trading, execution, private keys, and order placement.
- **Engineering-first architecture**: PASS. The plan separates ingestion, raw capture, normalization, storage, feature computation, screening, UI, and replay.
- **Testing required**: PASS. The plan includes unit, integration, golden, and replay smoke tests before implementation is accepted.
- **Observability by default**: PASS. Health status, lag metrics, writer backlog, reconnects, and data gaps are first-class.
- **No false market claims**: PASS. Scores are transparent heuristics and not presented as signals, predictions, or profitability claims.
- **Dependency hygiene**: PASS. Dependencies are boring, established, and scoped to ingestion, storage, terminal UI, and local data handling.

## Project Structure

### Documentation (this feature)

```text
specs/001-hyperliquid-spot-screener/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── cli.md
│   ├── data-files.md
│   ├── local-http-api.md
│   └── screen-rule-dsl.md
└── tasks.md
```

### Source Code (repository root)

```text
Cargo.toml
crates/
├── hls-core/
│   └── src/
├── hls-hyperliquid/
│   └── src/
├── hls-store/
│   └── src/
├── hls-features/
│   └── src/
├── hls-screen/
│   └── src/
├── hls-tui/
│   └── src/
├── hls-cli/
│   └── src/
└── hls-server/
    └── src/
config/
└── example.toml
docs/
├── architecture.md
├── data-format.md
└── feature-definitions.md
tests/
├── fixtures/
├── integration/
└── golden/
```

**Structure Decision**: Use a Cargo workspace with narrow crates. `hls-core` owns shared types and config; `hls-hyperliquid` owns public REST/WS clients and subscription management; `hls-store` owns raw/normalized local recording; `hls-features` owns rolling feature state; `hls-screen` owns filtering, sorting, presets, and DSL; `hls-tui` owns terminal rendering; `hls-cli` owns commands; `hls-server` is an optional read-only local API adapter over current feature snapshots.

## Phase 0: Research Summary

See [research.md](research.md). Key decisions:

- Use public Hyperliquid spot metadata and asset context at startup.
- Use one WebSocket connection by default and cap selected symbols to preserve subscription headroom.
- Treat raw trades and top-of-book as feature source of truth; candles are display/validation/fallback helpers.
- Record raw frames before normalization to preserve replay/debug evidence.
- Use local compressed raw files, normalized Parquet, and SQLite metadata.
- Start with a tiny custom DSL instead of embedding a scripting engine.

## Phase 1: Design Summary

See [data-model.md](data-model.md), [contracts/](contracts/), and [quickstart.md](quickstart.md).

Post-design constitution check:

- **Read-only capital boundary**: PASS. Contracts expose no trading commands or wallet inputs.
- **Replayability**: PASS. Data model includes raw frames, normalized events, file registry, run registry, and data gaps.
- **Operator truthfulness**: PASS. Health status and stale-data handling are contractually required.
- **Top-of-book honesty**: PASS. Contracts and data model use `tob_depth_usd` and `tob_imbalance`, not full depth.

## Complexity Tracking

No constitution violations are intentionally introduced.
