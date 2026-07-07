# Research: Hyperliquid Spot Screener

Sources were checked against official Hyperliquid documentation on 2026-07-07:

- [WebSocket endpoint](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket)
- [WebSocket subscriptions](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket/subscriptions)
- [Info endpoint](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/info-endpoint)
- [Spot info endpoint](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/info-endpoint/spot)
- [Rate limits and user limits](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/rate-limits-and-user-limits)
- [Timeouts and heartbeats](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket/timeouts-and-heartbeats)
- [Historical data](https://hyperliquid.gitbook.io/hyperliquid-docs/historical-data)

## Decision: Use Public Spot Metadata and Context at Startup

**Rationale**: Official spot docs expose `spotMeta` for tokens/universe and `spotMetaAndAssetCtxs` for spot asset contexts. The general info endpoint docs state that spot `coin` identifiers differ from display names: `PURR/USDC` is literal, while most spot pairs use `@{index}` from the `spotMeta.universe` index. The implementation needs a durable `SymbolMeta` that preserves both display name and feed identifier.

**Alternatives considered**:

- Only parse display symbols from live messages. Rejected because the spot identifier mapping is explicit and needed before subscriptions.
- Hard-code important symbols such as HYPE. Rejected because v1 must support dynamic top-by-volume universe selection.

## Decision: Use WebSockets for Live Market Data

**Rationale**: Official docs list the mainnet WebSocket URL as `wss://api.hyperliquid.xyz/ws` and say automated users should handle server-side disconnects and reconnect gracefully. The subscription docs confirm `allMids`, `trades`, `bbo`, `activeAssetCtx`, and `candle`. These match the v1 scope.

**Alternatives considered**:

- Poll REST for live screen updates. Rejected because docs recommend WebSockets for lowest-latency realtime data and polling would burn REST weight.
- Include `l2Book` in v1. Rejected because the brief excludes it and top-of-book is enough for honest spread and TOB liquidity metrics.

## Decision: Conservative Subscription Budget

**Rationale**: Current Hyperliquid docs list per-IP limits including 10 WebSocket connections, 30 new WebSocket connections per minute, 1000 WebSocket subscriptions, and 2000 messages sent per minute across WebSocket connections. v1 will use one connection by default and enforce configured headroom below the 1000-subscription limit.

Policy:

- Default max symbols: 180
- Hard max symbols: 240
- Max configured subscriptions: 950
- Per-symbol subscription count when all streams are enabled: trades, bbo, active asset context, candle
- Global subscription: all mids

**Alternatives considered**:

- Track every spot pair by default. Rejected because it can exceed subscription limits as the universe grows and makes reconnect storms harder to control.
- Use many WebSocket connections immediately. Rejected because one connection is simpler, easier to observe, and enough for v1.

## Decision: Heartbeat and Reconnect Semantics

**Rationale**: Hyperliquid heartbeat docs state the server closes a connection if it has not sent a message in 60 seconds and supports ping/pong keepalive messages. v1 will send a ping after 30 seconds without inbound messages and reconnect with backoff if no inbound data arrives by 60 seconds.

**Alternatives considered**:

- Assume active markets keep all subscriptions alive. Rejected because sparse spot markets and quiet channels are expected.
- Hide reconnect gaps. Rejected because replay and feature trust require explicit gap records.

## Decision: Raw Trades and BBO Are Feature Source of Truth

**Rationale**: The brief requires raw data and self-aggregation. Trades support returns, realized volatility, volume buckets, and trade-count z-scores. BBO supports spread and top-of-book liquidity. `allMids` and asset contexts provide low-cost references; candles remain visual/fallback/validation helpers.

**Alternatives considered**:

- Use exchange candle data as the primary feature source. Rejected because it would not satisfy raw-data replay and top-of-book feature requirements.
- Use `l2Book` for full depth. Rejected from v1 scope and terminology; UI must say TOB, not full book depth.

## Decision: Raw Append-Only Capture Before Normalization

**Rationale**: Official historical-data docs state Hyperliquid historical market data is uploaded approximately monthly, has no guarantee of timely updates, may be missing, and users can record additional historical data themselves. Raw capture preserves evidence for replay and parser/debug work.

**Alternatives considered**:

- Store only normalized events. Rejected because parser bugs would be unrecoverable.
- Store only current feature snapshots. Rejected because replay and feature regression testing require underlying events.

## Decision: Local File Store plus SQLite Metadata

**Rationale**: Raw compressed line files are simple and replayable; Parquet is appropriate for columnar normalized events; SQLite is enough for file registry, symbols, runs, and gaps without requiring a service.

**Alternatives considered**:

- Use a server database. Rejected because v1 should remain local and easy to run.
- Use only files without metadata. Rejected because run/file discovery and replay ranges become brittle.

## Decision: Task Isolation with Bounded Channels

**Rationale**: The WebSocket reader must timestamp and enqueue frames immediately. Disk writes, parsing, feature computation, and terminal rendering must not block ingestion. Bounded channels reveal pressure instead of silently growing memory.

**Alternatives considered**:

- Single event loop for ingestion, features, storage, and TUI. Rejected because terminal or disk stalls could cause data loss.
- Unbounded channels. Rejected because they hide overload until the process becomes unstable.

## Decision: Small Custom DSL for v1

**Rationale**: The rule language only needs boolean logic, comparisons, identifiers, literals, and a few safe helpers such as `abs`. A small parser avoids embedding a general scripting engine in a read-only market-data tool.

**Alternatives considered**:

- `rhai` scripting. Rejected as too broad for v1 and harder to constrain.
- `evalexpr`. Acceptable fallback, but a tiny parser keeps the accepted grammar explicit.

## Decision: Optional Local HTTP API, No Web Dashboard

**Rationale**: A read-only local API can make future web UI cheap without delaying terminal-first v1. The API should expose health, symbols, current screen, and symbol detail only.

**Alternatives considered**:

- Build a full web dashboard in v1. Rejected by scope and because it would slow the core data engine.
- Exclude API entirely. Rejected because a small API is low risk once the feature snapshot channel exists.
