# Quickstart: Hyperliquid Spot Screener

This is the validation guide for the planned v1. It describes the checks implementation must satisfy; commands become runnable as tasks are completed.

## Prerequisites

- Rust toolchain installed.
- Network access to public Hyperliquid market-data endpoints for live smoke tests.
- A writable local data directory.
- No wallet, private key, trading key, or exchange account is required.

## Setup

```bash
cargo build --workspace
./target/debug/hls init --data-dir ~/.hls
./target/debug/hls doctor --data-dir ~/.hls
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
./target/debug/hls live --symbols HYPE/USDC --duration-secs 30 --refresh-secs 5 --tui --record --raw --normalized --run-id quickstart-live --data-dir /tmp/hlscreen-quickstart
```

Expected outcome:

- Table displays one live public WebSocket row.
- Raw and normalized local files are written.
- Shutdown flushes file metadata and marks the run clean.

## Preset Screen Smoke

```bash
./target/debug/hls screen --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --preset thin_books
./target/debug/hls live --symbols hype-usdc --duration-secs 30 --refresh-secs 5 --tui --preset thin_books
```

Expected outcome:

- Rows are filtered by the preset.
- Sort order matches the preset.
- Invalid filter edits are rejected without replacing the active preset.

## Recorder-Only Smoke

```bash
./target/debug/hls record --symbols @107 --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --raw --normalized --run-id quickstart-record --data-dir /tmp/hlscreen-quickstart
```

Expected outcome:

- A recording run ID is printed.
- Raw and normalized files are registered.
- No terminal UI is required.

## Replay Smoke

```bash
./target/debug/hls replay --data-dir /tmp/hlscreen-quickstart --run-id quickstart-record
```

Expected outcome:

- Feature snapshots are rebuilt from local data.
- Data gaps are reported.
- No live network access is required for replay.

## Health And Local API Smoke

```bash
./target/debug/hls doctor --live --json
./target/debug/hls server --print-health
```

Expected outcome:

- `doctor --live --json` reports read-only safety, public REST reachability, and health status.
- `server --print-health` prints the compact `/health` JSON payload for the local read-only API contract.
- No output contains wallet prompts, private credentials, or order actions.

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
