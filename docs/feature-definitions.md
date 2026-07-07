# Feature Definitions

The v1 feature set is a transparent screener surface, not a prediction engine.

- `spread_bps`: best-ask minus best-bid divided by mid price, in basis points.
- `tob_depth_usd`: top-of-book bid notional plus ask notional.
- `tob_imbalance`: top-of-book bid notional versus ask notional, bounded to `[-1, 1]`.
- Return and volatility windows are computed from available local trade and quote state.
- Score fields are bounded heuristic ranks from `0` to `100`, not trade signals.

## Current Formula Definitions

- `spread_bps = (ask_px - bid_px) / ((bid_px + ask_px) / 2) * 10_000`
- `tob_depth_usd = bid_px * bid_sz + ask_px * ask_sz`
- `tob_imbalance = (bid_notional - ask_notional) / (bid_notional + ask_notional)`
- `ret_* = (latest_trade_px - first_trade_px) / first_trade_px` for the available fixture-backed trade window
- `rv_*` is population standard deviation over trade-to-trade returns, or `0` when fewer than three trades are available
- `liquidity_score = clamp(tob_depth_usd / 100, 0, 100)`
- `momentum_score = clamp(50 + ret_1m * 100, 0, 100)`
- `mean_reversion_score = clamp(50 - ret_1m * 100, 0, 100)`

These scores are screen ordering aids only. They are not predictions, recommendations, or profitability claims.

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
