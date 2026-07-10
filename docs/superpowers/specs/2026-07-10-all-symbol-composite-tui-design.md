# All-Symbol Composite TUI Design

**Date:** 2026-07-10
**Status:** Approved
**Scope:** Public, read-only Hyperliquid spot data and the unified Ratatui workstation

## Objective

Make `hls tui` start a complete all-symbol workstation with official one-minute
OHLCV for every subscribed spot symbol, a market-wide composite candle chart,
bounded selected-symbol depth, explicit data coverage, and a quieter adaptive
interface. The ingestion path must stay inside Hyperliquid's IP-wide limits and
must never present missing, approximated, or derived data as exchange-complete.

## Non-Goals

- No order entry, wallet access, private streams, or execution controls.
- No synthetic flat candles for symbols without exchange candle data.
- No attempt to bypass IP-wide limits with additional WebSocket connections.
- No hidden fallback from the complete-universe contract to a smaller universe.
- No persistence of private or execution-related state.

## Command Contract

`hls tui` loads the complete discoverable Hyperliquid spot universe by default.

- `hls tui`: complete universe, market composite chart, selected-symbol L2.
- `hls tui --top N`: explicitly restrict collection to the top `N` symbols.
- `hls tui --symbols A,B`: explicitly restrict collection to named symbols.
- `--top`, `--symbols`, and `--all-symbols` remain mutually exclusive.
- `--all-symbols` remains accepted for compatibility and is equivalent to the
  new default.
- `hls live` retains its existing non-TUI defaults.

The startup summary and TUI must show the requested universe, acknowledged
candle streams, live candle coverage, historical coverage, BBO tier coverage,
and selected L2 symbol.

## Subscription Architecture

Hyperliquid documents IP-wide limits of 1,000 active WebSocket subscriptions,
2,000 client-sent WebSocket messages per minute, 10 WebSocket connections, and
1,200 REST weight per minute. Multiple connections do not increase the
subscription allowance.

The planner allocates subscriptions in this order:

1. Reserve one global `allMids` subscription.
2. Reserve one `candle` subscription for every symbol. This is the hard
   all-symbol OHLCV contract and is never silently reduced.
3. Reserve one selected-symbol `l2Book` subscription.
4. Allocate `trades` by descending 24-hour quote notional. At the current
   universe size all symbols receive trades.
5. Allocate `bbo` to the top 100 symbols by 24-hour quote notional, bounded by
   remaining headroom.
6. Obtain all-symbol asset contexts through the existing REST metadata endpoint
   at startup and once per minute instead of one WebSocket subscription per
   symbol.

For 309 symbols the default plan is:

```text
allMids       1
candle      309
trades      309
bbo         100
l2Book        1
----------------
total       720
```

The application-level ceiling remains 980 to preserve 20 subscriptions of
exchange-limit headroom. For a universe too large to provide candles for every
symbol, planning fails before opening a socket and reports the exact required
and available counts. Trade and BBO tiers may shrink as the universe grows;
candle coverage may not.

Selected-symbol L2 changes use an unsubscribe/subscribe pair matching the
official subscription payload. Changes are debounced for 150 ms, pass through
the existing outbound message limiter, and reserve the previous book until the
new subscription is acknowledged. Failure leaves the pane explicitly degraded
rather than clearing it to an apparently valid empty book.

## Candle Data Model

Every stored candle carries provenance and completion state:

```rust
pub enum CandleProvenance {
    WebSocket,
    RestBootstrap,
}

pub enum CandleCompletion {
    Open,
    Closed,
}
```

The existing `CandleEvent` remains the exchange candle representation. New
fields use serde defaults so old NDJSON fixtures and recordings continue to
load. A candle is uniquely identified by `(hl_coin, interval, open_ts_ms)`.
Newer exchange updates replace an existing open candle only when their receive
timestamp is not older. Closed candles may receive late corrections, which are
upserted deterministically.

Trade events do not fabricate replacement exchange candles. They provide exact
live quote notional, print count validation, and an independent consistency
signal for official candle volume. Missing exchange candles stay missing.

## History Bootstrap And Cache

The live candle WebSocket begins first. Historical bootstrap runs concurrently
without blocking the workstation:

1. Load recent validated `1m` candle rows from the local SQLite cache.
2. Request the most liquid symbols first from official `candleSnapshot`.
3. Continue through the remaining universe under a shared 900 REST-weight per
   minute bootstrap budget, leaving headroom for metadata refresh and recovery.
4. Upsert REST results into live state and cache by candle identity.
5. Refresh incomplete or stale cache tails after reconnects.

The cache is stored in the existing `hls.sqlite` database and contains only
public candles, provenance, fetch time, and schema version. Writes use bounded
transactions outside the rendering path. Cache corruption, schema mismatch, or
write failure is visible in health diagnostics and does not stop live exchange
collection.

Bootstrap status distinguishes:

- `SUBS`: acknowledged candle subscriptions / requested symbols.
- `LIVE`: symbols with a current exchange candle / requested symbols.
- `HIST`: symbols with the requested chart history / requested symbols.
- `WEIGHT`: composite liquidity-weight coverage.

## Composite Candle Definition

The composite is a chained market index with initial value `100.0`. It is not an
average of incompatible raw asset prices.

### Constituent Weights

For symbol `i`, let `q_i` be finite, positive 24-hour quote notional from the
latest asset context snapshot. The raw weight is:

```text
r_i = sqrt(q_i)
```

Raw weights are normalized, capped at 10% per symbol, and renormalized until no
constituent exceeds the cap. Symbols without valid quote notional receive no
liquidity weight and are counted in missing-weight diagnostics.

Weights are frozen for each one-minute composite bucket. Later metadata refresh
does not rewrite historical composite candles.

### Chained OHLC

For each constituent with current candle `c_i` and a previous close `p_i`, form
return relatives:

```text
o_i = c_i.open  / p_i - 1
h_i = c_i.high  / p_i - 1
l_i = c_i.low   / p_i - 1
x_i = c_i.close / p_i - 1
```

For the first usable candle of a symbol, use its current open as `p_i`, making
its opening return zero. Let `O` be the previous composite close and `w_i` the
renormalized weights of usable constituents:

```text
open  = O * (1 + sum(w_i * o_i))
high  = O * (1 + sum(w_i * h_i))
low   = O * (1 + sum(w_i * l_i))
close = O * (1 + sum(w_i * x_i))
```

The builder validates finite positive prices and enforces
`high >= max(open, close)` and `low <= min(open, close)` after floating-point
rounding. Invalid constituents are excluded and reported.

### Volume And Breadth

- Live quote volume is the sum of exact public trade notional mapped to the
  candle interval.
- Historical quote volume is `candle.volume_base * candle.close` and is labeled
  `close-approx`.
- Base volumes are never summed across different assets and called market
  volume.
- Breadth is equal-weight and separate from the liquidity-weighted index:
  advances, declines, unchanged, missing, and stale.

Each `CompositeCandle` contains:

```rust
pub struct CompositeCandle {
    pub open_ts_ms: i64,
    pub close_ts_ms: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub quote_volume: f64,
    pub quote_volume_exact: bool,
    pub contributing_symbols: usize,
    pub requested_symbols: usize,
    pub liquidity_weight_coverage: f64,
    pub advances: usize,
    pub declines: usize,
    pub unchanged: usize,
    pub stale_symbols: usize,
}
```

The current open bucket is recomputed as exchange updates arrive. Closed buckets
are deterministic for identical ordered input. A late exchange correction may
recompute that bucket and every later chained composite close in the retained
window.

### Coverage Gates

- `healthy`: at least 80% liquidity-weight coverage and no known data gap.
- `partial`: 50% to less than 80% weight coverage.
- `degraded`: below 50% coverage, stale input, or a known gap.
- No composite candle is emitted with zero valid constituents.

Coverage state is visible in the chart title, candle detail, market status, and
recorded evidence.

## Selected-Symbol L2

Add an exchange `OrderBookEvent` matching the documented `WsBook` snapshot:

```rust
pub struct OrderBookLevel {
    pub price: f64,
    pub size: f64,
    pub order_count: u64,
}

pub struct OrderBookEvent {
    pub recv_ts_ns: u64,
    pub exchange_ts_ms: i64,
    pub hl_coin: String,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
}
```

Validation rejects non-finite or non-positive prices, negative sizes, crossed
books, incorrectly ordered levels, and more than the exchange's documented 20
levels per side. State retains only the latest receive-ordered snapshot for the
selected symbol. The pane shows an actual depth ladder, cumulative quote depth,
imbalance, age, and acknowledged symbol. Before acknowledgement or on failure it
is labeled `TOP OF BOOK` and uses existing BBO evidence without implying L2.

## Prepared Render Model

Filtering and sorting run once per refresh. The prepared frame owns:

- screened rows and selected index;
- symbol-to-row, symbol-to-candles, and symbol-to-trades indexes;
- composite candles and coverage state;
- selected L2 snapshot and age;
- global health, bootstrap, recorder, and subscription status;
- precomputed market breadth, leaders, aggregate flow, and quality alert.

Panels borrow prepared data. They may not rerun `ScreenEngine`, clone the full
universe, or independently derive conflicting aggregate values.

The current monolithic renderer is split into focused modules:

```text
crates/hls-tui/src/ratatui/
  mod.rs
  prepared.rs
  layout.rs
  header.rs
  watchlist.rs
  detail.rs
  chart.rs
  book.rs
  trades.rs
  status.rs
  palette.rs
  format.rs
```

## Interaction And Rendering

- Market composite is the default chart in complete-universe mode.
- Selected-symbol chart remains available through a chart-scope toggle.
- Keyboard input redraws immediately.
- Market redraw is dirty-driven and capped at one frame per 125 ms.
- Ingestion, recording, bootstrap, and reconnect processing never wait on draw.
- Missed draw ticks are skipped, not queued.
- Display pause freezes the rendered prepared frame only.
- Selection changes update detail immediately and schedule bounded L2 switching.

## Layout And Copy

Wide layout:

- Three-line maximum global header: run health, universe/composite coverage, and
  selected symbol/filter state.
- Left 30-34%: watchlist using every row that fits the viewport.
- Center 46-50%: composite or selected chart plus compact candle/coverage rail.
- Right 18-22%: selected-symbol L2 and public trades.
- Two-line maximum footer: market alert and context-sensitive action strip.

Medium and narrow layouts collapse lower-priority panes into focusable views;
they do not wrap table rows. Persistent `COMMAND DECK`, `NEON STATE`, repeated
read-only warnings, and repeated shortcut rails are removed. Complete help stays
behind `?`.

Use neutral borders, one focused-pane accent, and semantic color only for price
direction, risk, freshness, and failures. Confidence `100` renders as `H100`;
fields must not clamp values merely to satisfy a fixed width.

Missing-data copy is precise:

- `bootstrapping`: requested and not yet historically loaded.
- `not subscribed`: intentionally outside a BBO or trade tier.
- `missing`: expected exchange data has not arrived.
- `stale`: data arrived but exceeded its freshness threshold.
- `gap`: a known disconnect or parser loss affects the interval.

## Failure Behavior

- Subscription planning fails closed before network I/O when mandatory candle
  coverage cannot fit.
- Individual candle or L2 parse errors increment parser-drop diagnostics and do
  not mutate valid prior state.
- Bootstrap 429/timeout responses back off with jitter and preserve cached/live
  data; no tight retry loop is allowed.
- A reconnect restores the baseline plan first, then selected L2, then schedules
  cache-tail repair.
- Composite coverage degrades across known gaps instead of silently chaining
  across them as complete.
- Cache failures are visible and nonfatal; exchange connection failures retain
  the existing bounded reconnect and kill-switch behavior.

## Verification Contract

### Unit And Property Tests

- Subscription budgets at 309 symbols and boundary universe sizes.
- Candle and L2 validation, ordering, deduplication, and late correction.
- Weight normalization, iterative cap, finite-input rejection, and determinism.
- Composite OHLC invariants and chained recomputation.
- Exact versus approximated quote-volume labeling.
- Coverage state boundaries at 50% and 80%.

### Integration Tests

- Fixture startup produces candle subscriptions for every requested symbol.
- Selected-symbol changes emit matching L2 unsubscribe/subscribe messages.
- Invalid command and failed L2 acknowledgement preserve the last valid frame.
- Cache load, background bootstrap, restart reuse, and schema mismatch behavior.
- Reconnect restores subscriptions and marks affected composite intervals.

### TUI And PTY Tests

- Viewports `80x24`, `120x40`, `160x48`, and `236x71` do not corrupt rows.
- All-symbol composite is visible without selected-symbol candles.
- `H100`, dynamic row capacity, coverage labels, real L2, and fallback BBO copy.
- Exactly one alternate-screen entry/exit, no repeated full clears, and balanced
  cursor/mouse state.
- Forced color, no-color, resize, rapid navigation, pause, and clean quit.

### Live Acceptance

A bounded public run must prove:

- discovered symbols equal requested candle subscriptions;
- every mandatory candle subscription is acknowledged;
- composite candles become visible with truthful coverage;
- selected L2 follows symbol changes without exceeding rate limits;
- zero render-induced data gaps;
- clean shutdown and no orphan process;
- frame p95 below 16 ms at `236x71` after warmup;
- steady-state CPU and memory remain bounded for the full universe.

## Delivery Strategy

Implement as reviewable vertical commits on `feat/andrzej_agent_sota_lab`:

1. Tiered subscription planner and official all-symbol candle contract.
2. Candle cache and rate-limited history bootstrap.
3. Pure composite builder and coverage model.
4. Selected-symbol L2 parser, state, and subscription switching.
5. Prepared frame and renderer module extraction.
6. Simplified adaptive layout and composite chart.
7. Full regression, PTY, live, benchmark, documentation, and release review.

No commit may weaken the read-only boundary or merge with failing required
checks.
