# Feature Specification: Hyperliquid Microstructure Workstation

**Feature Branch**: `002-microstructure-workstation`

**Created**: 2026-07-08

**Status**: Draft

**Input**: User-provided strategy brief: evolve `hlscreen` from a read-only Hyperliquid spot screener into a standout open-source, venue-native, local-first microstructure workstation with replayable data, confidence-aware analytics, low-latency terminal UX, packaging, and extensibility.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Trust Data Quality During Live and Replay Sessions (Priority: P1)

A trader or researcher can see whether every ranked symbol and feature row is trustworthy under reconnects, sparse trades, stale quotes, storage pressure, and replay gaps.

**Why this priority**: The brief's central product principle is that a symbol should not rank highly unless its data can be replayed, explained, and trusted under gaps. Confidence and replay parity are prerequisites for every higher-level metric.

**Independent Test**: Run deterministic live/replay fixtures with clean data, sparse data, reconnect gaps, duplicate trades, and writer lag; verify confidence scores, incomplete-window states, and degradation reasons match expected outputs without live network access.

**Acceptance Scenarios**:

1. **Given** a live session with no reconnects or stale feeds, **When** feature snapshots are published, **Then** each row includes a high confidence state with no hidden gap, sparse-data, or stale-quote warnings.
2. **Given** a reconnect gap or missing stream interval, **When** ranking is computed, **Then** affected rows show degraded confidence and features depending on the missing interval are marked incomplete until enough fresh data arrives.
3. **Given** a recorded interval, **When** the user replays it, **Then** the replayed confidence state, feature values, and ranking explanations are reproducible from local raw/normalized data within documented tolerances.
4. **Given** duplicate trades or repeated BBO messages after reconnect, **When** the replay or live pipeline applies them, **Then** confidence and feature windows do not double-count the duplicate events.

---

### User Story 2 - Analyze Liquidity Resilience and Tradeability (Priority: P1)

A discretionary trader can identify whether a live move is supported by resilient top-of-book liquidity or is brittle, stale, or expensive to trade.

**Why this priority**: Liquidity resilience is the fastest differentiator beyond generic screeners because it uses existing `bbo`, `trades`, and recorder data while answering a practical microstructure question.

**Independent Test**: Feed fixed BBO/trade sequences containing spread shocks, quote recovery, aggressive flow, and thin books; verify the resilience metrics and row ordering match expected classifications.

**Acceptance Scenarios**:

1. **Given** a spread shock followed by quick recovery, **When** the resilience radar updates, **Then** the symbol shows recovery duration, spread shock magnitude, and a healthy or recovering state.
2. **Given** aggressive trade flow with deteriorating BBO, **When** the toxicity/adverse-selection lens updates, **Then** the symbol is flagged as high-risk or brittle rather than simply "momentum".
3. **Given** a thin top-of-book and high spread, **When** a move appears in the screener, **Then** tradeability ranking reflects quoted cost, top-of-book depth, and confidence instead of raw return alone.

---

### User Story 3 - Explain Why a Symbol Ranked (Priority: P2)

A user can inspect the top-ranked symbols and see the score components, evidence, and confidence caveats that caused each ranking.

**Why this priority**: Explainable ranking turns the tool from a passive table into a research workstation and prevents opaque score worship.

**Independent Test**: Run fixture rows with known score components and verify that the why-ranked pane reports the exact positive, negative, and confidence-adjusted contributions.

**Acceptance Scenarios**:

1. **Given** a ranked row, **When** the user opens the explanation view, **Then** the UI shows named score components such as liquidity resilience, signed flow, spread cost, volatility, confidence, and venue metadata tags.
2. **Given** a score is reduced by low confidence, **When** the explanation is rendered, **Then** the confidence penalty and underlying reason are visible next to the score.
3. **Given** a replayed interval, **When** the user inspects the same symbol at the same replay timestamp, **Then** the score breakdown matches the live-recorded explanation.

---

### User Story 4 - Discover Hyperliquid-Native Listing and Token Events (Priority: P2)

A researcher can monitor venue-specific spot metadata such as token details, deployer, deploy time, seeded USDC, supply, and new-listing cohorts alongside live market data.

**Why this priority**: Venue-native metadata is a defensible niche that generic crypto screeners do not own.

**Independent Test**: Use fixed `spotMeta`, `spotMetaAndAssetCtxs`, `tokenDetails`, and deploy-auction fixtures; verify that new-listing/deployer tags and metadata filters appear without requiring private account data.

**Acceptance Scenarios**:

1. **Given** a newly discovered spot asset, **When** metadata enrichment runs, **Then** the row includes listing age, deployer, seeded USDC, supply fields, and a new-listing cohort tag where data is available.
2. **Given** a metadata endpoint returns missing or partial fields, **When** the screen renders, **Then** the row shows unknown metadata explicitly and does not fail the live data pipeline.
3. **Given** a user filters for new listings or fresh liquidity, **When** the metadata cache is populated, **Then** matching rows are discoverable through presets and custom screen rules.

---

### User Story 5 - Operate, Package, and Extend the Workstation as OSS (Priority: P3)

A new user or contributor can install the tool quickly, verify it, inspect metrics, reproduce benchmark fixtures, and understand how to add read-only feature panels safely.

**Why this priority**: The brief calls out open-source adoption, release packaging, benchmark packs, observability, and plugin/extensibility as key to standing out.

**Independent Test**: From a clean checkout or release artifact, run documented validation commands, install/package checks, metrics/health commands, benchmark replay fixtures, and contribution checks without hidden local state.

**Acceptance Scenarios**:

1. **Given** a first-time user on macOS or Linux, **When** they follow the release quickstart, **Then** they can install or build, run `hls live --top 50`, and inspect health within five minutes.
2. **Given** maintainers run benchmark fixtures, **When** a PR changes feature logic, **Then** output hashes and performance budgets detect regressions in replay parity or local hot-path latency.
3. **Given** a developer proposes a custom feature or panel, **When** they use the extension contract, **Then** the extension remains read-only, row-scoped, and unable to access network or filesystem by default.
4. **Given** users need observability, **When** the tool runs in live or replay mode, **Then** it exposes bounded metrics for message counts, parse/feature latency, data gaps, queue depth, and low-confidence symbols without high-cardinality labels on every histogram.

### Edge Cases

- Hyperliquid public WebSocket sends no updates for an inactive symbol while other streams remain healthy.
- A reconnect produces a data gap that affects some feature windows but not metadata-only fields.
- A BBO feed has quote silence because best bid/ask did not change, not because the connection failed.
- A symbol has very sparse trades, causing return/volatility windows to be mathematically invalid.
- A metadata endpoint is unavailable, paginated, or missing token detail fields for a subset of assets.
- A user enables all-symbol mode and new streams exceed the configured subscription budget.
- A plugin or custom feature tries to access private data, filesystem paths, network APIs, or mutation hooks.
- Prometheus/metrics labels are accidentally made high-cardinality by symbol or run ID.
- A replay fixture created on an older schema must either migrate explicitly or fail with a clear unsupported-version message.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST remain read-only by default and MUST NOT add wallet connection, private stream subscription, signing, order placement, cancellation, withdrawals, or execution routes.
- **FR-002**: The system MUST compute a per-symbol data-confidence state that accounts for reconnect gaps, quote staleness, sparse trades, duplicate events, parser drops, writer backlog, and incomplete feature windows.
- **FR-003**: The system MUST apply confidence state to ranking so degraded rows cannot silently appear as fully trusted top-ranked opportunities.
- **FR-004**: The system MUST persist enough replay metadata to reproduce feature values, confidence state, and why-ranked explanations for recorded intervals.
- **FR-005**: The system MUST add liquidity resilience metrics using public BBO and trade data, including spread shock magnitude, recovery time, top-of-book depth, top-of-book imbalance, and quote freshness.
- **FR-006**: The system MUST add tradeability metrics that combine quoted spread, top-of-book depth, aggressive flow, realized volatility, and confidence without claiming profitability.
- **FR-007**: The system MUST label any best-level-only order-flow or adverse-selection metric as a BBO/top-of-book proxy, not full order-book depth.
- **FR-008**: The system MUST represent ranking scores as named components so UI, logs, and replay can explain the total score.
- **FR-009**: The system MUST expose a why-ranked view or output that shows positive contributors, negative contributors, confidence penalties, and unavailable evidence for each selected symbol.
- **FR-010**: The system MUST enrich market rows with Hyperliquid-native metadata where publicly available, including token details, deployer, deploy time, seeded USDC, supply fields, and listing cohort.
- **FR-011**: The system MUST tolerate missing or partial metadata by surfacing unknown fields without stopping live ingestion.
- **FR-012**: The system MUST provide presets or screen-rule fields for confidence, liquidity resilience, tradeability, new-listing, and metadata-based discovery.
- **FR-013**: The system MUST expose operator metrics for WebSocket messages, disconnects, parse latency, feature latency, pipeline lag, recorder queue depth, data gaps, replay speed, tracked symbols, and low-confidence symbol counts.
- **FR-014**: The system MUST avoid high-cardinality metric labels such as symbol on broad histograms; symbol-level detail belongs in TUI detail panes, logs, or top-N offender summaries.
- **FR-015**: The system MUST provide canonical benchmark/replay fixtures that validate output parity, confidence behavior, and performance budgets.
- **FR-016**: The system MUST document metric formulas, validity conditions, confidence caveats, replay semantics, and known biases in user-facing docs.
- **FR-017**: The system MUST include release packaging tasks for reviewed release artifacts and supported desktop binary targets.
- **FR-018**: The system MUST define a read-only extension/plugin contract before enabling third-party features or panels.
- **FR-019**: Any extension/plugin surface MUST be read-only and MUST NOT receive network, filesystem, credential, private account, or state-mutation capabilities by default.
- **FR-020**: The system MUST provide a clean quickstart path that gets a new user from install/build to live screen and health verification in under five minutes on supported platforms.
- **FR-021**: The system MUST keep local hot-path processing measurable by separating receive timestamp, feature timestamp, and render timestamp.
- **FR-022**: The system MUST preserve v1 CLI and recording compatibility unless a migration is explicitly documented and tested.

### Key Entities *(include if feature involves data)*

- **Data Confidence Snapshot**: Per-symbol trust state with confidence score, degraded reasons, affected feature windows, source stream freshness, gap state, and sparse-data flags.
- **Liquidity Resilience Snapshot**: Public BBO/trade-derived metrics for spread shocks, recovery timing, top-of-book depth, imbalance, quote freshness, and tradeability classification.
- **Score Breakdown**: Named score components, weights, signed contributions, total score, confidence multiplier or penalty, and unavailable evidence notes.
- **Metadata Enrichment**: Hyperliquid public metadata for token details, deployer, deploy time, seeded USDC, supply fields, listing age, and cohort tags.
- **Benchmark Fixture Pack**: Curated raw/normalized replay bundle with expected output hashes, feature snapshots, confidence states, and performance budgets.
- **Operator Metrics Snapshot**: Low-cardinality counters, gauges, and histograms for ingestion, parsing, feature computation, replay, recorder, gaps, and health.
- **Read-Only Extension Contract**: Versioned input/output schema for future custom features or panels, including capability restrictions and validation rules.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Reconnect-gap fixtures mark affected feature windows incomplete and reduce confidence within one snapshot refresh.
- **SC-002**: Replay of a canonical 10-minute fixture reproduces feature values, confidence states, and score breakdowns within documented tolerances.
- **SC-003**: Liquidity resilience fixtures classify spread-shock recovery and brittle-thin-book cases with 100% expected labels.
- **SC-004**: Every top-ranked row in the enhanced screen has at least three named score components or explicit unavailable-evidence notes.
- **SC-005**: Metadata enrichment fixtures expose new-listing/deployer fields where available and keep ingestion healthy when metadata is partial.
- **SC-006**: Local hot-path metrics report parse and feature latency separately; benchmark fixtures enforce a p95 local feature-update budget documented in `quickstart.md`.
- **SC-007**: Release/quickstart validation proves a first-time local build can start a live top-50 screen and inspect health in under five minutes on a supported machine.
- **SC-008**: Repository scans and tests confirm no wallet, signing, private stream, order, or execution surface is introduced by the feature.

## Assumptions

- Existing v1 live ingestion, recording, replay, screening, TUI rendering, and health commands remain the foundation.
- Public Hyperliquid WebSocket and Info endpoints are the only production data sources for this feature.
- Full order-book depth and node-backed data are future advanced modes, not required for this feature.
- Metrics should be useful locally and for Prometheus-style exporters but must remain low-cardinality by default.
- Plugin/extensibility work may stop at a versioned contract and test harness before a full WASM runtime is implemented.
- Scores are operational heuristics and research evidence, not financial advice, trading signals, or profitability claims.
