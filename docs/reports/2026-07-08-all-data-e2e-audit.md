# All-Data Live Smoke And End-to-End Audit - 2026-07-08

## Scope

Branch: `feat/andrzej_all_data_e2e_audit`

Base: `main` at `ab01664` before this audit branch.

Capital boundary: read-only public Hyperliquid REST and WebSocket market data only. No credentials, private streams, wallet prompts, signing, order placement, cancellation, withdrawals, exchange actions, live trading, or trade recommendations were used or added.

Official docs checked:

- Hyperliquid WebSocket subscriptions: public `trades`, `bbo`, `activeAssetCtx`, `candle`, `allMids`, `subscriptionResponse`, and `pong` shapes.
- Hyperliquid WebSocket heartbeat: client ping and server pong contract; server can close idle connections after 60 seconds without sent messages.
- Hyperliquid rate limits: 10 WebSocket connections, 30 new connections/min, 1000 subscriptions, and 2000 client-sent WebSocket messages/min.
- Hyperliquid Info endpoint: `POST https://api.hyperliquid.xyz/info`, spot `PURR/USDC` exception, and `@{index}` spot feed IDs from `spotMeta`.

## Live All-Symbol Smoke

Primary current capture:

```bash
./target/debug/hls live --all-symbols --duration-secs 180 --refresh-secs 30 --tui --record --raw --normalized --run-id allpairs-e2e-20260708-195413 --data-dir /tmp/hlscreen-allpairs-e2e-20260708-195413
```

Result:

- `symbols=308`
- `subscriptions=924`
- `streams_per_symbol=3`
- `ws_messages=59384`
- `market_events=67192`
- `reconnects=0`
- `data_gaps=0`
- `elapsed_secs=180`
- `raw_messages=59384`
- `normalized_events=67192`
- `raw_files=3`
- `normalized_files=1`
- `clean_shutdown=true`
- SQLite registry: `runs.clean_shutdown=1`, `gap_count=0`, `files`: raw `3` / normalized `1`, `symbols=308`, `data_gaps=0`.

Post-fix confirmation after correcting the TUI confidence label:

```bash
./target/debug/hls live --all-symbols --duration-secs 60 --refresh-secs 30 --tui --record --raw --normalized --run-id allpairs-postfix-20260708-195842 --data-dir /tmp/hlscreen-allpairs-postfix-20260708-195842
```

Result:

- `symbols=308`
- `subscriptions=924`
- `streams_per_symbol=3`
- `ws_messages=18470`
- `market_events=26156`
- `reconnects=0`
- `data_gaps=0`
- `elapsed_secs=60`
- `raw_messages=18470`
- `normalized_events=26156`
- `raw_files=2`
- `normalized_files=1`
- `clean_shutdown=true`
- SQLite registry: `runs.clean_shutdown=1`, `gap_count=0`, `files`: raw `2` / normalized `1`, `symbols=308`, `data_gaps=0`.

The live proof used real public Hyperliquid data. Fixtures were used only for deterministic tests and committed screenshot generation.

## Replay, Screen, Health, And Screenshots

Replay over the primary capture:

```bash
./target/debug/hls replay --data-dir /tmp/hlscreen-allpairs-e2e-20260708-195413 --run-id allpairs-e2e-20260708-195413 --verify-parity
```

The first replay wrote the confidence baseline. The second replay returned `replay_parity=passed` with `confidence_drift=0`, `confidence_missing=0`, and `confidence_extra=0`.

Screen commands over the same captured run:

```bash
./target/debug/hls screen --data-dir /tmp/hlscreen-allpairs-e2e-20260708-195413 --run-id allpairs-e2e-20260708-195413 --preset thin_books
./target/debug/hls screen --data-dir /tmp/hlscreen-allpairs-e2e-20260708-195413 --run-id allpairs-e2e-20260708-195413 --preset flow_pressure
```

Both exited cleanly with zero stderr. `thin_books` returned 8 rows. `flow_pressure` returned 2 rows and rendered the compact workstation with selected-pair detail.

Health surfaces:

```bash
./target/debug/hls doctor --live --json --data-dir /tmp/hlscreen-allpairs-e2e-20260708-195413
./target/debug/hls server --print-health
```

`doctor --live --json` returned `read_only=true`, `live_rest=true`, `status=healthy`, zero reconnects, zero gaps, and low-cardinality metrics. `server --print-health` returned a read-only JSON health payload. The long-running localhost HTTP server is still intentionally out of scope.

Screenshot evidence:

- Real live-data PNG: `/tmp/hlscreen-allpairs-e2e-20260708-195413/screenshots/live-flow-pressure-real.png`
- Real live-data SVG: `/tmp/hlscreen-allpairs-e2e-20260708-195413/screenshots/live-flow-pressure-real.svg`
- Regenerated committed SVGs: `docs/assets/screenshots/live-screen.svg`, `metadata-discovery.svg`, `confidence-degraded.svg`, `resilience-screen.svg`, `record-replay.svg`, plus unchanged health/symbol/explain screenshots.

## Code Review Findings

### Fixed

1. TUI confidence counter label was misleading.
   - Before: `Confidence gap:N` counted `incomplete_windows.len()`, not actual reconnect/data gaps.
   - After: the detail line uses `window:N stale:N sparse:N reconnect:N parser_drop:N`.
   - Files: `crates/hls-tui/src/app.rs`, `crates/hls-tui/tests/main_table_golden.rs`, `crates/hls-tui/tests/confidence_pane.rs`, regenerated screenshot SVGs.

### Passed

- WebSocket subscriptions match the official public subscription envelope.
- All-symbol mode stays under the 1000-subscription limit by using `trades`, `bbo`, and `activeAssetCtx` for the 308-symbol universe, producing 924 subscriptions.
- Heartbeat pings use the documented `{ "method": "ping" }` message; `pong` and `subscriptionResponse` are control messages.
- Parser accepts documented public channels plus the live-observed `activeSpotAssetCtx` spot alias and rejects private/user/trading channels.
- REST metadata uses public Info endpoint types and the documented spot symbol mapping.
- Live recorder is off the read loop through a bounded queue and fails closed on backpressure/disconnect.
- Replay refuses dirty/incomplete recording runs and fails if normalized events are missing.
- Screen DSL rejects type-incompatible filters and unknown presets without replacing active rows.
- TUI uses real `FeatureSnapshot` fields and labels unsupported values as missing/proxy evidence.
- Health and metrics avoid symbol/run/account high-cardinality labels.
- Extension contract remains model-only and denies network, filesystem, private-data, and trading permissions.
- Release workflow remains tag-gated and packaging tests confirm no release secrets other than `GITHUB_TOKEN`.

## Negative Probes

All probes exited non-zero with clear errors:

- Invalid DSL: `symbol > 10` -> type-incompatible comparison.
- Unknown preset: `definitely_missing` -> unknown preset.
- Missing fixture path -> read error with OS cause.
- Unsupported Parquet output -> explicit not-implemented error directing use of `--normalized`.

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
cargo test -p hls-tui --test main_table_golden --test confidence_pane
```

Scans:

- TODO/debug/dead-code scan over `crates`, `scripts`, and `tests` found only expected test assertion `panic!` uses.
- Read-only/private-surface scan found expected docs, negative tests, safety wording, and parser rejection of private channels; no production order/private/wallet path was found.

## Residual Risks

- This smoke is bounded to minutes, not a multi-day soak test.
- Automatic public REST backfill after reconnect remains unimplemented; reconnect gaps are explicit instead of hidden.
- Long-running localhost HTTP server mode remains out of scope; `server --print-health` is the current read-only API preview.
- True Parquet output remains unimplemented; `--normalized` JSONL is the replayable format.
- Interactive keyboard-driven TUI remains future work; the current renderer is deterministic terminal output.
- BBO-derived OFI/adverse-selection/Amihud-like fields are top-of-book proxies, not full depth or execution-quality proof.

## Review Decision

Local audit passed after the confidence-label fix. The branch is suitable for PR once the report and continuity notes are committed. Merge to `main` is allowed only after GitHub checks pass.
