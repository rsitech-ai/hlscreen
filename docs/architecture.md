# Architecture

`hlscreen` is a read-only Hyperliquid spot market-data workstation. It ingests public market data, records local evidence, computes explainable screening features, renders a deterministic terminal UI, and exposes read-only health/API helpers.

It does not own private keys, wallet permissions, private user streams, order placement, leverage, liquidation, execution, or capital controls.

## System Boundary

```mermaid
flowchart LR
    HLREST["Hyperliquid public Info API"]
    HLWS["Hyperliquid public WebSocket"]
    Adapter["hls-hyperliquid\nREST/WS parser + connection"]
    Live["hls-cli live\nbounded runtime"]
    Recorder["hls-store\nraw zstd + normalized JSONL + SQLite"]
    State["hls-core LiveMarketState\nsymbols, health, confidence"]
    Features["hls-features\nrolling windows + microstructure proxies"]
    Screen["hls-screen\nDSL + presets + sorting"]
    Tui["hls-tui\nworkstation table + detail pane"]
    Server["hls-server\nread-only response helpers"]

    HLREST --> Adapter
    HLWS --> Adapter
    Adapter --> Live
    Live --> Recorder
    Live --> State
    State --> Features
    Recorder --> Features
    Features --> Screen
    Screen --> Tui
    State --> Server
    Screen --> Server
```

## Crate Ownership

```mermaid
flowchart TB
    Core["hls-core\ncontracts, symbols, state, health, confidence, metrics"]
    Hyper["hls-hyperliquid\npublic REST/WS adapters"]
    Store["hls-store\nrecording, replay, registry, benchmarks"]
    Features["hls-features\nfeature engine, resilience, tradeability"]
    Screen["hls-screen\nfilter DSL, presets, row projection"]
    Tui["hls-tui\ndeterministic terminal rendering"]
    Server["hls-server\nread-only API helpers"]
    Cli["hls-cli\ncommands and operator workflows"]

    Hyper --> Core
    Store --> Core
    Store --> Hyper
    Store --> Features
    Features --> Core
    Screen --> Core
    Tui --> Core
    Tui --> Screen
    Server --> Core
    Server --> Screen
    Cli --> Core
    Cli --> Hyper
    Cli --> Store
    Cli --> Features
    Cli --> Screen
    Cli --> Tui
    Cli --> Server
```

## Live Data Flow

```mermaid
sequenceDiagram
    participant CLI as hls live
    participant REST as Hyperliquid Info API
    participant WS as Hyperliquid WS
    participant REC as Recorder worker
    participant FEAT as Feature engine
    participant TUI as TUI renderer

    CLI->>REST: Load public spot universe and metadata
    CLI->>CLI: Budget subscriptions under public limits
    CLI->>WS: Subscribe to trades, bbo, activeAssetCtx
    CLI->>WS: Send heartbeat ping during run
    WS-->>CLI: Public frames and control messages
    CLI->>REC: Queue raw frame and normalized events
    CLI->>FEAT: Apply event to market state
    FEAT-->>CLI: FeatureSnapshot rows with confidence
    CLI->>TUI: Render table, detail, health text
    CLI->>REC: Finish and mark clean shutdown
```

Runtime rules:

- All-symbol mode budgets subscriptions before connecting. On 2026-07-08 the public spot universe had `308` symbols; `trades`, `bbo`, and `activeAssetCtx` produce `924` subscriptions, under the configured headroom and official public limit.
- Disk writes are off the WebSocket read loop through a bounded worker queue. Backpressure fails closed instead of silently dropping data.
- Reconnects resubscribe and record explicit data gaps. Automatic REST backfill after reconnect is not implemented.
- The TUI renders `p95 row age`, which is row freshness, not a compute-latency SLA.

## Replay And Screening Flow

```mermaid
flowchart LR
    Raw["raw/ws/run=<id>/*.ndjson.zst"]
    Norm["normalized/events/run=<id>/*.ndjson"]
    Sqlite["hls.sqlite\nruns, files, symbols, gaps, confidence"]
    Replay["hls-store replay"]
    Features["FeatureSnapshot rows"]
    Parity["hls replay --verify-parity"]
    Screen["hls screen / hls explain"]
    Tui["workstation output"]

    Raw --> Sqlite
    Norm --> Sqlite
    Norm --> Replay
    Replay --> Features
    Features --> Parity
    Features --> Screen
    Screen --> Tui
```

Replay rules:

- Dirty or incomplete runs are rejected.
- `hls replay --verify-parity` writes a confidence baseline on first run and fails non-zero on later drift.
- Replay parity checks confidence/data-quality state, not profitability or strategy quality.

## Current Command Surfaces

- `hls init`: create local config/data directories.
- `hls doctor`: print read-only health and low-cardinality metrics.
- `hls symbols`: inspect public spot universe metadata.
- `hls live`: bounded public live screen/recording.
- `hls record`: deterministic fixture recording path.
- `hls replay`: replay normalized local captures and verify parity.
- `hls screen`: filter/sort feature snapshots with presets or custom DSL.
- `hls explain`: show why-ranked score components for one row.
- `hls bench`: run deterministic public fixture benchmark packs.
- `hls server --print-health`: print read-only API preview JSON.

## Production-Readiness Boundary

Production-ready today means:

- Run locally or in a supervised environment as a read-only public-data process.
- Capture raw and normalized public data for replay.
- Fail closed on writer backpressure, invalid configs, parser-private channels, invalid DSL, missing fixtures, unsupported Parquet output, and replay parity drift.
- Emit deterministic terminal output, keyboard-focused live TUI state, and low-cardinality health metrics.

Not production-ready today:

- Unbounded daemon/service mode.
- Hosted web API.
- Public release binaries from a proven `v*` tag.
- Public REST backfill after reconnect.
- True Parquet output.
- Full alternate-screen widget-grid or mouse-driven TUI. Current `hls live --tui` supports keyboard row focus, view cycling, density, help, pause state, and clean quit in real terminals.
- Any live trading, wallet, private stream, or order execution behavior.

See [production-readiness.md](production-readiness.md) for the current validation evidence and deployment checklist.
