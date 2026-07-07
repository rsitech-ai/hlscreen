# Feature Specification: Hyperliquid Spot Screener

**Feature Branch**: `001-hyperliquid-spot-screener`

**Created**: 2026-07-07

**Status**: Draft

**Input**: User description: "A read-only terminal-first screener and local recorder for Hyperliquid spot markets, built from public market-data feeds, with local rolling features, a small screening DSL, replay, and no trading or AI in v1."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Watch Live Spot Market Conditions (Priority: P1)

A trader or researcher opens the tool and sees a live, sortable terminal screener for selected Hyperliquid spot markets with prices, volume, spread, top-of-book liquidity, returns, volatility, anomalies, scores, and freshness indicators.

**Why this priority**: The live screener is the primary value of v1. If this works without recording, custom rules, or replay, the product is already useful as read-only market infrastructure.

**Independent Test**: Can be tested by starting the live view for a small allowlist of spot markets and verifying that rows update, stale data is marked, sorting works, and no wallet or trading action is requested.

**Acceptance Scenarios**:

1. **Given** a configured list of spot markets, **When** the user starts the live screen, **Then** the tool displays one row per tracked market with price, market metadata, spread, top-of-book liquidity, returns, volatility, volume anomaly, trade-count anomaly, score fields, and last-update freshness.
2. **Given** live data is arriving, **When** the user changes the sort field, **Then** the visible rows reorder by the selected metric without interrupting data collection.
3. **Given** a tracked market stops receiving fresh updates, **When** the freshness threshold is exceeded, **Then** the row is marked stale instead of silently presenting old values as current.

---

### User Story 2 - Record and Replay Market Data Locally (Priority: P1)

A researcher records exact incoming market-data messages and normalized market events locally, then replays a selected interval later to reproduce the same market table and debug feature behavior.

**Why this priority**: Raw capture and replay make the screener trustworthy. Without replay, live bugs and feature drift are hard to investigate.

**Independent Test**: Can be tested by running a short recording session, confirming raw and normalized files are written with a run record, then replaying the interval and comparing resulting screen rows to the original session within documented tolerances.

**Acceptance Scenarios**:

1. **Given** recording is enabled, **When** the user starts a live session, **Then** exact received messages are stored append-only with receive timestamps, connection identity, sequence numbers, channel names, and payloads.
2. **Given** normalized storage is enabled, **When** trade, top-of-book, market metadata, and candle messages are received, **Then** the tool stores validated event records that can be loaded independently of the raw capture.
3. **Given** a previous recording exists, **When** the user replays a time range, **Then** the tool rebuilds live state and feature snapshots from local data without needing live network access.
4. **Given** the live connection drops during a recording, **When** the connection is restored, **Then** the tool records a data-gap event and marks affected feature windows as incomplete until enough fresh data arrives.

---

### User Story 3 - Screen Markets with Rules and Presets (Priority: P2)

A trader or researcher applies built-in or custom screening rules to focus on liquid movers, volume anomalies, tight-spread movers, mean-reversion watchlists, or thin-book situations.

**Why this priority**: Screening rules turn the table from a passive watchlist into a useful discovery surface while staying read-only.

**Independent Test**: Can be tested with fixed sample screen rows by applying built-in presets and custom comparisons, then verifying that included rows, excluded rows, and sort order match the rule definitions.

**Acceptance Scenarios**:

1. **Given** the live table has feature rows, **When** the user selects a preset such as liquid momentum or volume anomaly, **Then** only rows matching that preset are shown and sorted by the preset's metric.
2. **Given** the user enters a custom rule over supported screen fields, **When** the rule is valid, **Then** matching rows update continuously as new feature snapshots arrive.
3. **Given** the user enters an invalid rule, **When** the rule is parsed, **Then** the tool reports a clear validation error and keeps the previous valid screen active.

---

### User Story 4 - Monitor Data Health and Safety Boundaries (Priority: P3)

An operator checks connection health, subscription count, data lag, storage lag, writer status, reconnect events, and read-only safety status from a dedicated health surface.

**Why this priority**: The tool is market infrastructure. It must reveal degraded data, storage pressure, or reconnect gaps before the user trusts the screen.

**Independent Test**: Can be tested by using simulated healthy, stale, disconnected, and writer-lag states, then verifying the health view reports the correct status and the screen does not hide degraded data.

**Acceptance Scenarios**:

1. **Given** a healthy session, **When** the user opens health status, **Then** the tool shows connection uptime, last-message age, subscription count, data lag percentiles, feature freshness, writer queue state, rows written, and recording status.
2. **Given** inbound messages stop for long enough to require a heartbeat or reconnect, **When** the session recovers, **Then** the health surface shows the reconnect and any data gap.
3. **Given** a user reviews safety status, **When** the tool is running, **Then** it explicitly remains read-only and exposes no wallet connection, private-key input, or order placement action.

### Edge Cases

- Public market-data endpoints are reachable but a subset of selected markets has sparse trades or no top-of-book update.
- Connection drops mid-session and reconnect succeeds after multiple backoff attempts.
- Connection remains disconnected long enough that all live rows become stale.
- Recording storage is temporarily slow or unavailable.
- Duplicate trade or top-of-book messages are received after reconnect.
- A custom screen rule references an unsupported field or compares incompatible value types.
- The configured universe would exceed the safe subscription budget.
- A market symbol has a display name that differs from the identifier required by the public market-data feed.
- The user tries to enable an out-of-scope v1 capability such as wallet connection, trading, full order-book depth, AI prediction, backtesting, full web dashboard, social/news data, or multi-exchange mode.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST operate as a read-only market-data tool and MUST NOT request private keys, wallet permissions, order permissions, withdrawals, or trading credentials.
- **FR-002**: The system MUST support live screening of Hyperliquid spot markets selected by a configured universe, include list, exclude list, and volume-based ranking.
- **FR-003**: The system MUST preserve both display names and feed identifiers for spot markets so users can see readable symbols while the system subscribes to the correct market-data streams.
- **FR-004**: The system MUST collect public price, trade, top-of-book, market metadata, and 1-minute candle data for selected markets, with candles treated as display, validation, or fallback inputs rather than the primary feature source.
- **FR-005**: The system MUST store exact received market-data messages append-only when raw recording is enabled.
- **FR-006**: The system MUST transform received messages into validated market events for trades, top-of-book quotes, market metadata, all-market mids, candles, data gaps, and recording runs.
- **FR-007**: The system MUST maintain live per-symbol state without requiring users to read local storage during normal live viewing.
- **FR-008**: The system MUST compute screen rows containing price, 24h notional volume, mid price, mark price, best bid, best ask, spread bps, top-of-book depth, top-of-book imbalance, 1m/5m/1h returns, realized volatility, volume z-score, trade-count z-score, liquidity score, momentum score, mean-reversion score, and freshness.
- **FR-009**: The system MUST label top-of-book metrics honestly as top-of-book depth and top-of-book imbalance, not full book depth.
- **FR-010**: The system MUST expose a terminal live view with sortable rows, selected-symbol details, filter editing, preset selection, recording status, pause/resume of display updates, and health/status navigation.
- **FR-011**: The system MUST expose command-line flows for initialization, symbol inspection, live viewing, recording, screening, replay, data inspection, and diagnostics.
- **FR-012**: The system MUST support built-in screening presets for liquid momentum, volume anomaly, tight-spread movers, mean-reversion watch, and thin-book monitoring.
- **FR-013**: The system MUST support a small screening rule language over screen-row fields with boolean logic, comparisons, supported numeric fields, supported string fields, and clear validation errors.
- **FR-014**: The system MUST support sorting screen rows by at least symbol, price, 24h volume, spread bps, top-of-book depth, returns, z-scores, and score fields.
- **FR-015**: The system MUST support local replay from recorded data for a requested time range and produce feature snapshots from replayed events.
- **FR-016**: The system MUST detect duplicate trades and repeated top-of-book or metadata messages sufficiently to avoid double-counting live features after reconnects.
- **FR-017**: The system MUST pace market-data subscriptions and refuse unsafe universe sizes that exceed the configured subscription budget.
- **FR-018**: The system MUST detect missing inbound data, send keepalive checks when appropriate, reconnect with backoff, resubscribe, and surface gaps instead of silently hiding them.
- **FR-019**: The system MUST surface health information covering connection state, subscription counts, last-message age, data lag, parser/feature/render lag, writer backlog, rows written, recording status, and data gaps.
- **FR-020**: The system MUST flush local recordings cleanly on shutdown and report whether shutdown was clean or interrupted.
- **FR-021**: The system MUST provide configuration for data directory, recording modes, network environment, heartbeat thresholds, reconnect limits, universe selection, stream toggles, feature windows, and terminal refresh behavior.
- **FR-022**: The system MUST make out-of-scope v1 capabilities unavailable: full order-book ingestion, trading/execution, wallet connection, AI/ML prediction, backtesting, full web dashboard, multi-exchange support, and social/news/sentiment ingestion.

### Key Entities *(include if feature involves data)*

- **Market Symbol**: A tracked Hyperliquid spot market with display name, feed identifier, token indexes, spot index, decimals, canonical status, and first/last seen times.
- **Raw Market Message**: An exact received public market-data message with receive timestamp, connection identity, sequence number, channel, and payload.
- **Market Event**: A validated normalized event derived from a raw message, such as trade, top-of-book quote, market metadata, all-market mid, candle, or data gap.
- **Feature Snapshot**: The current computed row for one symbol, including prices, liquidity, returns, volatility, anomaly scores, ranking scores, and freshness.
- **Screen Rule**: A user-provided or preset filter and sort definition over supported feature fields.
- **Recording Run**: A local session record with start/end time, configuration identity, storage files, row counts, gaps, and clean-shutdown status.
- **Health Status**: Connection, subscription, latency, lag, storage, recording, and read-only safety state shown to the operator.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can start a live screen for at least 150 selected spot markets and see populated rows with freshness status within 30 seconds under normal network conditions.
- **SC-002**: During a 30-minute recording smoke, raw messages and normalized event files are created, indexed by the recording run, and closed cleanly on shutdown.
- **SC-003**: Replaying a recorded 10-minute interval reproduces the same symbol set and core feature values within documented tolerances without live network access.
- **SC-004**: Built-in presets and custom rules filter fixed sample rows with 100% expected include/exclude behavior in validation fixtures.
- **SC-005**: In simulated disconnect, stale-feed, and writer-lag cases, the health surface identifies the degraded condition within 10 seconds and the main screen stops presenting stale rows as fresh.
- **SC-006**: A first-time user can initialize the tool, list symbols, start a small live watchlist, apply a preset, and stop cleanly in under 10 minutes using documented commands.
- **SC-007**: No v1 user flow exposes wallet connection, private-key entry, order creation, trading, execution, or predictive edge claims.

## Assumptions

- Users are local operators, traders, or researchers who are comfortable with terminal tools.
- v1 targets public Hyperliquid spot market data only.
- The tool is allowed to write local market-data files under a user-controlled data directory.
- The default universe is volume-ranked and capped below the public subscription budget, with user include/exclude overrides.
- Feature scores are transparent screening heuristics, not predictions, trade signals, or profitability claims.
- Full order-book depth, execution, wallet connection, AI prediction, backtesting, full web dashboard, multi-exchange support, and sentiment/news feeds are intentionally deferred.
