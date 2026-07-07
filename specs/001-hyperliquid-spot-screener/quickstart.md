# Quickstart: Hyperliquid Spot Screener

This is the validation guide for the planned v1. It describes the checks implementation must satisfy; commands become runnable as tasks are completed.

## Prerequisites

- Rust toolchain installed.
- Network access to public Hyperliquid market-data endpoints for live smoke tests.
- A writable local data directory.
- No wallet, private key, trading key, or exchange account is required.

## Setup

```bash
cargo build
./target/debug/hls init --data-dir ~/.hls
./target/debug/hls doctor
```

Expected outcome:

- Config file exists.
- Data directory is writable.
- Read-only safety status is reported.

## Live Metadata Smoke

```bash
./target/debug/hls symbols --top 20
```

Expected outcome:

- Symbols print with display names and Hyperliquid feed identifiers.
- Output includes 24h volume, mark, and mid when available.
- No wallet or trading prompt appears.

## One-Symbol Live Ingestion Smoke

```bash
./target/debug/hls live --symbols @107 --record --raw --parquet
```

Expected outcome:

- TUI displays one live row.
- Health pane shows connection and subscription state.
- Raw and normalized local files are written.
- Shutdown flushes file metadata and marks the run clean.

## Preset Screen Smoke

```bash
./target/debug/hls live --top 50 --preset volume_anomaly
```

Expected outcome:

- Rows are filtered by the preset.
- Sort order matches the preset.
- Invalid filter edits are rejected without replacing the active preset.

## Recorder-Only Smoke

```bash
./target/debug/hls record --symbols @107 --raw --parquet --duration 60s
```

Expected outcome:

- A recording run ID is printed.
- Raw and normalized files are registered.
- No terminal UI is required.

## Replay Smoke

```bash
./target/debug/hls replay --from 2026-07-07T12:00:00Z --to 2026-07-07T12:10:00Z --speed 10x
```

Expected outcome:

- Feature snapshots are rebuilt from local data.
- Data gaps are reported.
- No live network access is required for replay.

## Validation Commands

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Expected outcome:

- Formatting passes.
- Lint passes without warnings.
- Unit, integration, golden, and replay tests pass for implemented slices.
