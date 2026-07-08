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
- Deterministic public fixture benchmark packs.
- Low-cardinality metrics snapshots with Prometheus text output.
- Read-only extension manifest contracts.
- Draft cargo-dist packaging config and tag-gated packaging workflow.

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
4. Advanced interactive TUI beyond the current row/view keyboard controls.
   - Keyboard-driven filter editing.
   - Preset switching.
   - Health panel and recording status.
5. First public release.
   - Run `dist plan` and `dist build` with the pinned cargo-dist version.
   - Review the first `v*` tag packaging workflow output.
   - Publish checksums and installation docs after the tag run is proven.

## Explicitly Out Of Scope

- Trading execution.
- Wallet integration.
- Private account streams.
- Profitability claims.
- Automated strategy recommendations.
