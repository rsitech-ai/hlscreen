# Screen Rule DSL Contract

The screen rule language filters `Feature Snapshot` rows. It is intentionally small and deterministic.

## Grammar

```text
expr      = or_expr
or_expr   = and_expr ("or" and_expr)*
and_expr  = cmp_expr ("and" cmp_expr)*
cmp_expr  = value (">" | ">=" | "<" | "<=" | "==" | "!=") value
value     = identifier | number | string | bool | function_call | "(" expr ")"
function_call = function_name "(" identifier ")"
```

## Supported Functions

- `abs(field)` for numeric fields

## Supported Fields

- `symbol`
- `price`
- `mid_px`
- `mark_px`
- `day_ntl_vlm`
- `bid_px`
- `ask_px`
- `spread_bps`
- `tob_depth_usd`
- `tob_imbalance`
- `ret_1m`
- `ret_5m`
- `ret_1h`
- `rv_1m`
- `rv_5m`
- `rv_1h`
- `volume_z_1h`
- `trade_count_z_1h`
- `liquidity_score`
- `momentum_score`
- `mean_reversion_score`
- `updated_ms_ago`

## Sort Format

```text
field:asc
field:desc
abs(field):asc
abs(field):desc
```

## Built-In Presets

```text
liquid_momentum:
  where liquidity_score > 70 and volume_z_1h > 2 and ret_5m > 0 and spread_bps < 30
  sort momentum_score:desc

volume_anomaly:
  where volume_z_1h > 3 and trade_count_z_1h > 2
  sort volume_z_1h:desc

tight_spread_movers:
  where spread_bps < 20 and abs(ret_5m) > 0.01
  sort abs(ret_5m):desc

mean_reversion_watch:
  where mean_reversion_score > 70 and liquidity_score > 60
  sort mean_reversion_score:desc

thin_books:
  where day_ntl_vlm > 100000 and tob_depth_usd < 5000
  sort tob_depth_usd:asc
```

## Validation Requirements

- Unknown fields are rejected.
- Unknown functions are rejected.
- Type-incompatible comparisons are rejected.
- Parenthesis nesting and total boolean operators are each limited to 256;
  larger filters fail validation instead of entering recursive evaluation.
- Invalid expressions do not replace the currently active valid expression.
- Missing row values evaluate as non-matches for numeric comparisons.
