# Architecture

`hlscreen` is a read-only Hyperliquid spot market-data application.

The intended crate boundaries are:

- `hls-core`: shared configuration, symbol identity, errors, timestamps, health, and state contracts.
- `hls-hyperliquid`: public Hyperliquid REST/WebSocket adapters and feed-specific parsing.
- `hls-store`: raw frame capture, normalized event files, metadata registry, and replay readers.
- `hls-features`: rolling windows and feature formulas.
- `hls-screen`: preset and custom screening rules over feature rows.
- `hls-tui`: terminal rendering and interaction.
- `hls-cli`: command routing and operator-facing workflows.
- `hls-server`: optional localhost read-only API.

No crate owns private keys, wallet permissions, order placement, leverage, or execution.

## Implemented Data Paths

Current US1 live flow:

1. `hls-hyperliquid::ws::parser` parses public WebSocket envelopes for `trades`, `bbo`, `allMids`, `activeAssetCtx`, and `candle`.
2. `hls-core::market_state::LiveMarketState` applies typed market events into per-symbol state.
3. `hls-features::engine::FeatureEngine` builds `FeatureSnapshot` rows with top-of-book, return, freshness, and score fields.
4. `hls-tui::app::render_main_table` renders a stable read-only workstation board with scan KPIs, market rows, and selected-symbol microstructure detail.
5. `hls-cli live --duration-secs ...` runs a bounded public WebSocket session with heartbeat pings, reconnect/resubscribe, subscription-budget validation, optional recording, live TTY refresh, and clean shutdown.
6. `hls-cli live --fixture-file ... --once` remains a deterministic test/offline-doc path without live network access.

Current US2 recording/replay flow:

1. `hls-store::recorder` records fixture public WebSocket lines through a bounded raw-writer channel.
2. `hls-store::raw` writes compressed raw `.ndjson.zst` files under the configured local data directory.
3. `hls-store::normalized` writes deterministic replayable JSONL `MarketEvent` rows.
4. `hls-store::metadata` records runs, files, symbols, and data gaps in local `hls.sqlite`.
5. `hls-store::replay` rebuilds feature snapshots from normalized local files.
6. `hls-cli live --record` records real public WebSocket frames through a bounded writer worker that fails closed on backpressure. `hls-cli record`, `hls-cli replay`, and fixture-backed `hls live --record` expose deterministic test/offline flows.

Current US3 screening flow:

1. `hls-screen::dsl::parser` parses the small deterministic filter DSL and sort syntax.
2. `hls-screen::engine::ScreenEngine` filters and sorts `FeatureSnapshot` rows by custom expression or built-in preset.
3. `hls-screen::engine::ScreenSession` preserves the last active rows when an invalid expression is rejected.
4. `hls-tui::app::render_screened_table` applies screening before rendering.
5. `hls-cli screen` and fixture-backed `hls live --preset/--where/--sort` call the same shared engine.

Current US4 health/safety flow:

1. `hls-core::health` owns serializable read-only safety, connection, writer, recording, gap, and aggregate health state.
2. `hls-core::telemetry` owns deterministic latency percentile calculation for data, feature, and render lag.
3. `hls-hyperliquid::ws::connection` owns heartbeat timing, bounded reconnect backoff, and resubscribe state transitions for public WebSocket connections.
4. `hls-tui::health::render_health_pane` renders operator health text from the shared snapshot.
5. `hls-server::handle_get` exposes pure read-only `/health`, `/symbols`, `/screen`, and `/symbol/{symbol}` response helpers.
6. `hls-cli doctor --live` and `hls-cli server --print-health` expose health status without wallet, private, order, or trading actions.

True Parquet output, public-data backfill after reconnect, a long-running localhost HTTP server loop, and interactive keyboard filter editing remain separate later slices.
