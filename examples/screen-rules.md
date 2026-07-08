# Screen Rule Examples

`hlscreen` rules filter `FeatureSnapshot` rows. They are screening helpers, not trading signals.

## Built-In Presets

```bash
./target/debug/hls screen --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --preset thin_books
./target/debug/hls screen --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --preset liquid_momentum
./target/debug/hls screen --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --preset mean_reversion_watch
```

## Custom Filters

```bash
./target/debug/hls screen \
  --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson \
  --where 'spread_bps < 75 and tob_depth_usd > 100' \
  --sort ret_5m:desc
```

```bash
./target/debug/hls screen \
  --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson \
  --where 'symbol == "@107"' \
  --sort price:desc
```

```bash
./target/debug/hls screen \
  --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson \
  --where 'abs(ret_5m) >= 0.001' \
  --sort abs(ret_5m):desc
```

## Supported Rule Surface

- Boolean operators: `and`, `or`
- Comparisons: `>`, `>=`, `<`, `<=`, `==`, `!=`
- Literals: numbers, strings, booleans
- Function: `abs(field)` for numeric fields
- Sort syntax: `field:asc`, `field:desc`, `abs(field):asc`, `abs(field):desc`

Invalid expressions fail closed and do not replace the active screen session.
