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
