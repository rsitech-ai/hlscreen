# Roadmap

`hlscreen` is intentionally scoped as read-only market-data infrastructure.

## Current V1

- Public REST metadata parsing.
- Fixture-backed public WebSocket parsing.
- Local compressed raw recording.
- Normalized replay JSONL.
- SQLite metadata registry.
- Feature snapshots and screening DSL.
- Terminal table rendering.
- Health snapshots and read-only local API helpers.

## Next Candidate Slices

1. Real public WebSocket live mode.
   - Public market-data subscriptions only.
   - Heartbeat, ping/pong, reconnect, resubscribe, and gap recording.
   - Bounded writer queues and clean shutdown.
2. True Parquet writer.
   - Stable schemas.
   - Local replay compatibility.
   - Optional DuckDB query examples.
3. Long-running localhost API.
   - Bind to localhost only by default.
   - Read-only routes only.
   - Health and screen endpoints backed by live/replay state.
4. Interactive TUI.
   - Keyboard-driven filter editing.
   - Preset switching.
   - Health panel and recording status.
5. Public packaging.
   - Signed release binaries.
   - Checksums.
   - Installation docs for macOS and Linux.

## Explicitly Out Of Scope

- Trading execution.
- Wallet integration.
- Private account streams.
- Profitability claims.
- Automated strategy recommendations.
