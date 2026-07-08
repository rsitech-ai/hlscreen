# 2026-07-08 Pre-Merge Audit

## Scope
- Branch audited: `feat/andrzej_hlscreen_foundation`.
- Capital impact: research-only, read-only public market data.
- External writes/execution checked: no wallet, private-key, private-user stream, order, cancel, withdrawal, or `/exchange` path is implemented.
- Runtime scope verified: read-only public REST metadata plus deterministic fixture-backed live, record, replay, screen, and health flows.

## Official Documentation Checked
- Hyperliquid Info endpoint: `POST https://api.hyperliquid.xyz/info` for `spotMeta` and `spotMetaAndAssetCtxs`.
- Hyperliquid spot naming: `PURR/USDC` keeps the pair name; other spot assets use `@{index}` feed identifiers.
- Hyperliquid WebSocket endpoint and public subscription envelope: `wss://api.hyperliquid.xyz/ws`, `{"method":"subscribe","subscription":...}`.
- Hyperliquid public WebSocket channels used by v1: `trades`, `bbo`, `allMids`, `activeAssetCtx`, and `candle`.
- Hyperliquid heartbeat guidance: server can close idle connections after 60 seconds; client ping/pong handling is modeled by the deterministic health machine.
- Hyperliquid rate limits: REST weight budget and WebSocket limits, including 1,000 subscriptions per IP.
- Hyperliquid exchange endpoint: reviewed only to confirm hlscreen does not call or expose order-capable APIs.

## Findings Fixed
- REST client now uses an explicit 10 second request timeout instead of the implicit default.
- Default WebSocket subscription budget now supports the product default universe of 150 symbols x 4 public streams with headroom under the documented 1,000 subscription limit.
- Health aggregation now raises status monotonically, so an interrupted safety state cannot be downgraded by a later degraded connection state.
- Duplicate `unique_trade_id` values are idempotent in market state, preventing replay/reconnect duplicates from inflating feature windows.
- Feature snapshots now calculate distinct timestamp-bounded `1m`, `5m`, and `1h` returns/volatility instead of reusing one full-history value for all windows.
- Candle volume and trade-count anomaly fields now use latest-vs-baseline z-scores, with `0.0` only when there is not enough baseline variation.
- `hls doctor` now fails closed for invalid existing configs instead of silently reporting default read-only safety.
- Local API request decoding now handles UTF-8 percent escapes correctly and returns JSON `400` responses for malformed query/path encodings.
- TUI golden output was updated to show `-` for `ret_1m` when the strict 60 second window lacks two trades.

## Validation Evidence
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `cargo build --release --workspace`
- `git diff --check`
- `cargo tree -d`
- Read-only/security scan for `/exchange`, order/cancel/withdraw, key/seed, and unsafe safety defaults.
- `./target/debug/hls init --data-dir /tmp/hlscreen-audit.ib2pq9`
- `./target/debug/hls doctor --data-dir /tmp/hlscreen-audit.ib2pq9`
- `./target/debug/hls symbols --top 5`
- `./target/debug/hls doctor --data-dir /tmp/hlscreen-audit.ib2pq9 --live --json`
- `./target/debug/hls live --symbols @107 --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --record --raw --normalized --run-id audit-live --data-dir /tmp/hlscreen-audit.ib2pq9 --once`
- `./target/debug/hls record --symbols @107 --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --raw --normalized --run-id audit-record --data-dir /tmp/hlscreen-audit.ib2pq9`
- `./target/debug/hls replay --data-dir /tmp/hlscreen-audit.ib2pq9 --run-id audit-record`
- `./target/debug/hls screen --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --preset thin_books`
- Invalid rule probe: `./target/debug/hls screen --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --where 'symbol > 10'` exits non-zero with a typed validation error.
- `./target/debug/hls server --print-health`
- Negative live-network probe: `./target/debug/hls live --symbols @107 --once` exits non-zero with the explicit current-slice message that live network mode is not implemented.

## Review Result
- Correctness: pass after fixes. Public REST/WS parsing, symbol mapping, screening, record/replay, and health paths match the intended v1 contracts.
- Maintainability/readability: pass. Crate boundaries remain clear: core types, Hyperliquid adapters, feature computation, screening, storage, TUI, local API, and CLI are separated.
- Security/read-only: pass. Source scan found no order-capable route or exchange endpoint usage; only the spec prohibition and negative test config mention unsafe trading flags.
- Performance: pass for v1 fixture/read-only scope. Known future issue: raw writer buffers by file rotation size; acceptable with current bounded defaults but should become streaming-oriented for high-throughput live capture.
- Dependency hygiene: partial pass. `cargo tree -d` shows expected transitive duplicate families; `cargo-audit` and `cargo-deny` are not installed in this environment, so no advisory scan was claimed.
- Runtime/service behavior: pass within implemented scope. There is no long-running service or real live WebSocket loop to restart in this slice; `server --print-health` and fixture-backed runtime flows start cleanly.

## Known Scope Limits
- Real live WebSocket network mode is intentionally not implemented and fails closed with a clear message.
- Long-running localhost HTTP serving is not implemented; the current API proof is read-only route helpers plus `server --print-health`.
- `--parquet` remains intentionally rejected until a real Parquet writer exists.
- No CI status exists yet on a remote `main` branch because the repository currently has only `feat/andrzej_hlscreen_foundation` on origin.

## Merge Recommendation
- Recommendation: create a `main` baseline, open a PR from `feat/andrzej_hlscreen_foundation`, inspect the PR diff/check state, and merge to `main` if GitHub accepts the PR with no new blockers.
