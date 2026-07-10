# Research: Hyperliquid Microstructure Workstation

## Decision: Keep production ingestion public and read-only

**Decision**: Continue to use Hyperliquid public WebSocket and Info endpoints as the only production data sources for this feature.

**Rationale**: Hyperliquid documents WebSocket URLs for mainnet/testnet and states that WebSockets are available for real-time data streaming. It also warns automated users to handle server-side disconnects and gracefully reconnect. Public subscriptions cover `trades`, `bbo`, `allMids`, `activeAssetCtx`, and `candle`, while user/private subscriptions such as fills, spot state, and order updates require user addresses and remain out of scope.

**Alternatives considered**:
- Add authenticated/user feeds now. Rejected because it violates the read-only trust boundary and creates account/privacy risk.
- Add node-backed ingestion now. Rejected for this feature; it is a later advanced mode after public feed confidence/replay is proven.

**Primary references**:
- Hyperliquid WebSocket docs: https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket
- Hyperliquid WebSocket subscriptions: https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket/subscriptions

## Decision: Treat local recording as a first-class data product

**Decision**: Make confidence-aware replay and benchmark packs central to the v2 roadmap.

**Rationale**: Hyperliquid's historical archive is useful but not a complete substitute for local spot recording. The current official historical-data page says uploaded archive data may be delayed or missing and does not provide every useful historical dataset through S3, including examples such as candles or spot asset data. The workstation becomes more defensible if it can record, replay, and explain its own public spot dataset.

**Alternatives considered**:
- Depend on official or third-party history for replay. Rejected because it weakens reproducibility and does not cover every desired spot/TUI feature.
- Keep replay only as a debugging tool. Rejected because replay parity is the strongest trust and contribution mechanism.

**Primary reference**:
- Hyperliquid historical data: https://hyperliquid.gitbook.io/hyperliquid-docs/historical-data

## Decision: Build a confidence subsystem before adding stronger rankings

**Decision**: Add per-symbol confidence state and incomplete-window propagation before new ranking formulas can influence top rows.

**Rationale**: The project thesis is that a row should not rank highly unless its data can be trusted. Confidence must account for data gaps, quote freshness, sparse trades, duplicate events, parser drops, writer backlog, and incomplete feature windows.

**Alternatives considered**:
- Add liquidity/resilience metrics directly into existing scores. Rejected because new metrics can create false precision under gaps.
- Only show confidence in health view. Rejected because trust must be visible next to ranked rows.

## Decision: Use BBO-plus-trade microstructure metrics with explicit labels

**Decision**: Implement liquidity resilience and tradeability metrics from public BBO and trade data, while labeling best-level-only order-flow and adverse-selection metrics as top-of-book proxies.

**Rationale**: Hyperliquid public `WsBbo` contains best bid/ask price, size, and order count; `WsTrade` contains side, price, size, hash, timestamp, and trade id. Those fields are enough for spread shock, top-of-book depth, imbalance, quote freshness, trade intensity, signed flow, and recovery-time analytics. They are not enough to claim full order-book depth or queue-position realism.

**Alternatives considered**:
- Add `l2Book` to v2 as mandatory. Rejected because it increases subscription and processing load, and the current brief prioritizes a BBO-plus-trade differentiator.
- Present BBO metrics as full liquidity. Rejected as misleading.

**Primary reference**:
- Hyperliquid WebSocket subscriptions: https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket/subscriptions

## Decision: Keep subscription/rate limits as hard design constraints

**Decision**: All new streams and metadata refreshes must be budgeted before connecting or polling.

**Rationale**: Hyperliquid documents per-IP public limits including REST weight, maximum WebSocket connections, maximum new WebSocket connections per minute, maximum WebSocket subscriptions, and maximum messages sent per minute across WebSocket connections. The feature must not accidentally create reconnect storms, all-symbol over-subscription, or noisy client-side command traffic.

**Alternatives considered**:
- Let the runtime fail after exceeding limits. Rejected because it creates unreliable operator behavior.
- Split every feature into separate connections. Rejected unless a budgeted plan proves the need.

**Primary reference**:
- Hyperliquid rate limits and user limits: https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/rate-limits-and-user-limits

## Decision: Add low-cardinality observability first

**Decision**: Start with counters/gauges/histograms for message counts, disconnects, parse latency, feature latency, pipeline lag, queue depth, replay speed, tracked symbols, and low-confidence counts. Avoid `symbol` labels on broad histograms.

**Rationale**: The operator needs observability, but high-cardinality metrics can create memory/CPU costs and noisy dashboards. Symbol-level diagnostics are better shown in TUI detail panes, logs, top-N summaries, or replay artifacts.

**Alternatives considered**:
- Export every metric per symbol. Rejected for cardinality and cost.
- Keep metrics only in the TUI. Rejected because CI/benchmarks and operators need machine-readable outputs.

## Decision: Stage interactive TUI and packaging work after confidence/replay

**Decision**: Keep deterministic rendering tests for the first slice, then add a Ratatui/crossterm keyboard runtime when the data model and why-ranked panes are stable. Add reviewed cargo-dist/GitHub Release packaging after the quickstart and release checks are stable.

**Rationale**: Ratatui is a Rust library for fast, lightweight, rich terminal UIs and fits the target UX, but a full keyboard runtime is a larger lifecycle change than confidence/replay. cargo-dist is built to plan, build, host, publish, and announce binary releases and fits the release goal, but release automation should follow stable user-visible commands.

**Alternatives considered**:
- Add Ratatui and cargo-dist as first tasks. Rejected because the confidence/replay contract is more fundamental.
- Avoid a full interactive TUI entirely. Rejected because the pasted brief explicitly targets a standout terminal workstation.

**Primary references**:
- Ratatui: https://ratatui.rs/
- cargo-dist: https://github.com/axodotdev/cargo-dist

## Decision: Define extension contract before runtime

**Decision**: Write and test a read-only extension input/output contract before adopting a WASM plugin runtime.

**Rationale**: A plugin system can create an exfiltration and trust boundary if it receives filesystem, network, private account, or mutable state access too early. A versioned contract lets contributors design custom features or panels while the core stays safe.

**Alternatives considered**:
- Native Rust dynamic libraries. Rejected because Rust ABI stability and safety boundaries are poor for this use.
- Full WASM runtime immediately. Deferred until row-scoped contract, fixtures, and validation are stable.
