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

## Implemented Data Path

Current US1 mock-live flow:

1. `hls-hyperliquid::ws::parser` parses public WebSocket envelopes for `trades`, `bbo`, `allMids`, `activeAssetCtx`, and `candle`.
2. `hls-core::market_state::LiveMarketState` applies typed market events into per-symbol state.
3. `hls-features::engine::FeatureEngine` builds `FeatureSnapshot` rows with top-of-book, return, freshness, and score fields.
4. `hls-tui::app::render_main_table` renders a stable read-only terminal table.
5. `hls-cli live --fixture-file ... --once` runs the mock-live path without live network access.

Real WebSocket connection, reconnect, recording, replay, and interactive TUI work remain separate later slices.
