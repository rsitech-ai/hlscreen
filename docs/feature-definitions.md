# Feature Definitions

The v1 feature set is a transparent screener surface, not a prediction engine.

- `spread_bps`: best-ask minus best-bid divided by mid price, in basis points.
- `tob_depth_usd`: top-of-book bid notional plus ask notional.
- `tob_imbalance`: top-of-book bid notional versus ask notional, bounded to `[-1, 1]`.
- Return and volatility windows are computed from available local trade and quote state.
- Score fields are bounded heuristic ranks from `0` to `100`, not trade signals.
