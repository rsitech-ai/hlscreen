# Feature Definitions

The v1 feature set is a transparent screener surface, not a prediction engine.

- `spread_bps`: best-ask minus best-bid divided by mid price, in basis points.
- `tob_depth_usd`: top-of-book bid notional plus ask notional.
- `tob_imbalance`: top-of-book bid notional versus ask notional, bounded to `[-1, 1]`.
- Return and volatility windows are computed from local trades whose exchange timestamps fall inside the requested decision-time window.
- Score fields are bounded heuristic ranks from `0` to `100`, not trade signals.

## Current Formula Definitions

- `spread_bps = (ask_px - bid_px) / ((bid_px + ask_px) / 2) * 10_000`
- `tob_depth_usd = bid_px * bid_sz + ask_px * ask_sz`
- `tob_imbalance = (bid_notional - ask_notional) / (bid_notional + ask_notional)`
- `ret_1m`, `ret_5m`, and `ret_1h` are timestamp-bounded trade returns for the last 1 minute, 5 minutes, and 1 hour.
- `rv_1m`, `rv_5m`, and `rv_1h` are population standard deviations over trade-to-trade returns inside each timestamp window, or `0` when fewer than three trades are available.
- `volume_z_1h` and `trade_count_z_1h` compare the latest candle with the prior candle baseline; they return `0` when there is not enough baseline variation.
- `liquidity_score = clamp(tob_depth_usd / 100, 0, 100)`
- `momentum_score = clamp(50 + selected_return * 100, 0, 100)`, where `selected_return` prefers `ret_5m`, then `ret_1m`, then `ret_1h`.
- `mean_reversion_score = clamp(50 - selected_return * 100, 0, 100)`, using the same selected return.

These scores are screen ordering aids only. They are not predictions, recommendations, or profitability claims.

## Data Confidence

The microstructure workstation uses `DataConfidenceSnapshot` as the shared
contract for row-level data quality. It starts at `100` and degrades when the
pipeline observes evidence that a row should not be fully trusted.

Confidence levels:

- `high`: score `90..100`
- `medium`: score `70..89`
- `low`: score `30..69`
- `untrusted`: score below `30`

Current foundation reason codes:

- `reconnect_gap`: a reconnect or missed interval affected the row.
- `stale_quote`: quote freshness is outside the accepted window.
- `sparse_trades`: there are not enough trades for a windowed calculation.
- `duplicate_events`: duplicate events were observed or deduped.
- `parser_drops`: parser failures affected available evidence.
- `writer_backlog`: local recording pressure could affect durability.
- `incomplete_window`: one or more feature windows are not valid yet.

User-facing ranking work must keep this confidence state visible next to the
score. A low-confidence row must not silently look equivalent to a fully trusted
row.

## Score Breakdowns

The microstructure score contract stores named components rather than a single
opaque number. `ScoreBreakdown` records:

- raw component total, clamped to `0..100`
- confidence-adjusted total, also clamped to `0..100`
- confidence score used for adjustment
- named components such as liquidity, momentum, spread cost, signed flow,
  resilience, metadata, or custom components

This supports why-ranked views and replay parity. Score breakdowns remain screen
heuristics; they are not orders, trade recommendations, or performance proof.

## Metrics Contract

Metric definitions use `hls_`-prefixed names and low-cardinality labels. Labels
such as `symbol`, `run_id`, account, wallet, address, transaction hash, and
trade id are rejected by the foundation contract. Symbol-level detail belongs in
TUI rows, structured logs, replay artifacts, or top-N summaries, not in every
histogram label.

## Screening Rules

`hls-screen` supports a small deterministic rule language over `FeatureSnapshot` fields:

- Boolean operators: `and`, `or`
- Comparisons: `>`, `>=`, `<`, `<=`, `==`, `!=`
- Literals: numbers, strings, booleans
- Function: `abs(field)` for numeric fields
- Sort syntax: `field:asc`, `field:desc`, `abs(field):asc`, `abs(field):desc`

Built-in presets:

- `liquid_momentum`
- `volume_anomaly`
- `tight_spread_movers`
- `mean_reversion_watch`
- `thin_books`

Missing numeric values do not match numeric comparisons. Invalid expressions are rejected and do not replace the active screen session.
