# Roadmap

`hlscreen` is intentionally scoped as read-only market-data infrastructure.

## Current V1

- Public REST metadata parsing.
- Fixture-backed public WebSocket parsing.
- Bounded public WebSocket live mode with heartbeat pings, reconnect/resubscribe, duration-based shutdown, optional raw/normalized recording, and fail-closed writer backpressure.
- Local compressed raw recording.
- Normalized replay JSONL.
- SQLite metadata registry.
- Feature snapshots and screening DSL.
- Terminal table rendering.
- Health snapshots and read-only local API helpers.

## Next Candidate Slices

1. Public data backfill after reconnect.
   - Gap-aware public REST backfill where docs provide a matching info request.
   - Feature-window invalidation until enough fresh post-gap data arrives.
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
