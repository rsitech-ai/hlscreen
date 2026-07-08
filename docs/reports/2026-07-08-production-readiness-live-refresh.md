# Production Readiness Live Refresh - 2026-07-08

## Scope

Branch: `feat/andrzej_production_docs_live_audit`

Base: `main` at `45b9e7c`.

Boundary: read-only public Hyperliquid REST and WebSocket data only. No credentials, account streams, private streams, wallet integration, signing, order placement, cancellation, withdrawal, leverage, or trading recommendation was used or added.

Official docs checked:

- WebSocket public subscription envelope and public channels: https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket/subscriptions
- WebSocket ping/pong heartbeat behavior: https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket/timeouts-and-heartbeats
- Public WebSocket connection, subscription, and message limits: https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/rate-limits-and-user-limits
- Public Info endpoint and spot symbol mapping: https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/info-endpoint

## Current Live Validation

Command:

```bash
./target/debug/hls live --all-symbols --duration-secs 300 --refresh-secs 30 --tui --record --raw --normalized --run-id allpairs-prodreadiness-20260708-201752 --data-dir /tmp/hlscreen-prodreadiness-20260708-201752
```

Result:

- `symbols=308`
- `subscriptions=924`
- `streams_per_symbol=3`
- `ws_messages=99162`
- `market_events=106980`
- `reconnects=0`
- `data_gaps=0`
- `elapsed_secs=300`
- `raw_messages=99162`
- `normalized_events=106980`
- `raw_files=5`
- `normalized_files=1`
- `clean_shutdown=true`
- SQLite: `clean_shutdown=1`, `gap_count=0`, `symbols=308`, `data_gaps=0`

Storage evidence:

- Raw rows: `99,162`
- Normalized rows: `106,980`
- Raw bytes recorded in SQLite: `2,181,495`
- Normalized bytes recorded in SQLite: `21,249,940`

Post-fix live confirmation:

```bash
./target/debug/hls live --all-symbols --duration-secs 60 --refresh-secs 30 --tui --record --raw --normalized --run-id allpairs-prodreadiness-postfix-20260708-202420 --data-dir /tmp/hlscreen-prodreadiness-postfix-20260708-202420
```

Result:

- `symbols=308`
- `subscriptions=924`
- `ws_messages=18791`
- `market_events=26455`
- `reconnects=0`
- `data_gaps=0`
- `raw_messages=18791`
- `normalized_events=26455`
- `clean_shutdown=true`
- TUI verified `p95 row age` and `quality partial` for all-symbol output with partial quote/depth coverage.

## Replay, Screen, And Screenshot Evidence

Replay parity:

- First replay: `replay_parity=baseline_written`
- Second replay: `replay_parity=passed`
- `confidence_baseline=308`
- `confidence_replay=308`
- `confidence_drift=0`
- `confidence_missing=0`
- `confidence_extra=0`
- `confidence_summary=high:296 medium:0 low:12 untrusted:0 min:60 reasons:24`

Screen presets over the captured run:

- `thin_books`: 11 rows, clean stderr, `quality watch`
- `flow_pressure`: 3 rows, clean stderr, `quality good`

Real live-data screenshot artifacts:

- `/tmp/hlscreen-prodreadiness-20260708-201752/screenshots/flow-pressure-live.svg`
- `/tmp/hlscreen-prodreadiness-20260708-201752/screenshots/flow-pressure-live.png`

Committed deterministic screenshots were regenerated with:

```bash
python3 scripts/generate-screenshots.py
```

## Findings And Fixes

### Fixed

1. All-symbol table quality could overstate evidence coverage.
   - Before: aggregate quality used only available median spread/depth values, so broad all-symbol output could show `quality good` while many rows had no spread/depth evidence.
   - After: aggregate quality reports `PARTIAL` when any visible row lacks spread or top-of-book depth evidence.
   - Files: `crates/hls-tui/src/app.rs`, `crates/hls-tui/tests/main_table_golden.rs`, regenerated screenshots.

2. TUI freshness label was ambiguous.
   - Before: `p95 local` could be misread as compute latency.
   - After: `p95 row age` labels the value as row freshness.
   - Files: `crates/hls-tui/src/app.rs`, `crates/hls-tui/tests/main_table_golden.rs`, regenerated screenshots.

3. Architecture/open-source docs lagged current implementation.
   - Added Mermaid architecture/data-flow diagrams.
   - Added a production-readiness guide and updated README/docs index/checklist.

## Health And Negative Probes

Health:

- `./target/debug/hls doctor --live --json --data-dir /tmp/hlscreen-prodreadiness-20260708-201752`: `read_only=true`, `live_rest=true`, nested health `status=healthy`, seven low-cardinality metric samples, clean stderr.
- `./target/debug/hls server --print-health`: read-only healthy JSON payload, clean stderr.

Negative probes:

- Invalid DSL `symbol > 10`: exited `1` with type-incompatible comparison.
- Unknown preset `definitely_missing`: exited `1` with unknown preset.
- Missing fixture path: exited `1` with read error.
- Unsupported Parquet recording: exited `1` with explicit `use --normalized` guidance.

Scans:

- TODO/debug/dead-code scan over `crates`, `scripts`, and `tests` found only expected test assertion `panic!` uses.
- Read-only/private-surface scan found expected safety docs/tests and parser rejection of private channels; no production wallet/order/private route was found.

## Validation Gates

Passed:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo build --release --workspace --all-features
scripts/check-release-packaging.sh
python3 scripts/generate-screenshots.py
git diff --check
```

Additional checks:

- Local Markdown link check passed.
- `rsvg-convert docs/assets/screenshots/live-screen.svg -o /tmp/hlscreen-prodreadiness-preview-live.png` rendered successfully.
- Real live-data screenshot rendered successfully to `/tmp/hlscreen-prodreadiness-20260708-201752/screenshots/flow-pressure-live.png`.

## Code Review Decision

Local review passes with the fixes above. The current implementation is suitable for local read-only live-data deployment and open-source publication after normal PR review and CI. It is not a hosted service, not a binary release proof, and not any form of trading/execution system.

## Remaining Production Gaps

- Multi-day soak testing has not been run in this pass.
- Automatic public REST backfill after reconnect remains unimplemented.
- Long-running localhost HTTP server mode remains unimplemented.
- True Parquet output remains unimplemented.
- Advanced keyboard workflows such as in-TUI filter editing and preset switching remain future work. Basic `hls live --tui` keyboard controls for row focus, view cycling, density, help, pause state, and clean quit are now implemented.
- First public `v*` release artifacts/checksums remain unproven until a reviewed tag workflow succeeds.
