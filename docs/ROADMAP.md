# Roadmap

`hlscreen` is intentionally scoped as read-only market-data infrastructure.

## Current V1

- Public REST metadata parsing.
- Fixture-backed public WebSocket parsing.
- Bounded public WebSocket live mode with heartbeat pings, duration-based shutdown, and optional raw/normalized recording.
- Local compressed raw recording.
- Normalized replay JSONL.
- SQLite metadata registry.
- Feature snapshots and screening DSL.
- Terminal table rendering.
- Health snapshots and read-only local API helpers.

## Next Candidate Slices

1. Live WebSocket recovery hardening.
   - Automatic reconnect and resubscribe after server-side disconnects.
   - Gap recording and public REST backfill where docs provide a matching info request.
   - Bounded writer queues under sustained all-symbol load.
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
