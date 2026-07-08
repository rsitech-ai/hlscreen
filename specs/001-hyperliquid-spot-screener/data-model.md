# Data Model: Hyperliquid Spot Screener

## Entity: Market Symbol

Represents one Hyperliquid spot market selected for display, recording, or replay.

Fields:

- `symbol_id`: internal stable integer identifier
- `display_name`: user-facing name, e.g. `HYPE/USDC`
- `hl_coin`: public feed identifier, e.g. `@107` or `PURR/USDC`; this may differ from the display name for most spot markets
- `spot_index`: spot pair index from metadata
- `base_token_index`: base token index
- `quote_token_index`: quote token index
- `sz_decimals`: size decimals
- `wei_decimals`: token wei decimals
- `is_canonical`: whether the pair is canonical
- `first_seen_ms`: first observed metadata time
- `last_seen_ms`: latest metadata refresh time

Validation rules:

- `hl_coin` must be unique.
- `display_name` must not be empty.
- `spot_index`, token indexes, and decimals must be non-negative.
- Display names are derived from `spotMeta.universe[].tokens` and `spotMeta.tokens[].name`; `spotMeta.universe[].name` is often the feed ID, not the readable pair.
- If the market is `PURR/USDC`, the literal coin may be used as the feed ID; otherwise spot feeds usually use the `@{spot_index}` form.

Relationships:

- Referenced by every market event and feature snapshot.
- Stored in the metadata database.

## Entity: Raw Market Message

Append-only exact public market-data message as received.

Fields:

- `recv_ts_ns`: local receive timestamp
- `conn_id`: connection identity
- `seq`: monotonic local sequence per connection
- `channel`: received channel name
- `payload`: exact JSON payload
- `raw_file_path`: local raw file path after flush

Validation rules:

- `recv_ts_ns` must be assigned before parsing.
- `seq` must increase per connection.
- `channel` must not be empty.
- Payload must be preserved even if parsing later fails.

Relationships:

- May produce zero or more normalized market events.
- Belongs to one recording run.

## Entity: Trade Event

Normalized trade update.

Fields:

- `recv_ts_ns`
- `exchange_ts_ms`
- `symbol_id`
- `side`
- `price`
- `size`
- `notional`
- `hash`
- `tid`
- `unique_trade_id`

Validation rules:

- Price and size must be positive.
- Notional must equal price times size within decimal tolerance.
- Trade deduplication key is `(exchange_ts_ms, symbol_id, tid)`.
- Events with duplicate trade IDs must not be counted twice in rolling features.

Relationships:

- Updates trade windows, returns, realized volatility, volume buckets, trade count buckets, and last price.

## Entity: Top-of-Book Event

Normalized best bid/best ask update.

Fields:

- `recv_ts_ns`
- `exchange_ts_ms`
- `symbol_id`
- `bid_price`
- `bid_size`
- `bid_order_count`
- `ask_price`
- `ask_size`
- `ask_order_count`

Validation rules:

- Bid and ask may be missing independently.
- If both prices exist, bid must be less than or equal to ask.
- Sizes and order counts must be non-negative.
- Top-of-book dedupe key includes symbol, timestamp, bid fields, and ask fields.

Relationships:

- Updates spread bps, top-of-book depth, top-of-book imbalance, and mid price.

## Entity: Asset Context Event

Normalized spot market metadata/context update.

Fields:

- `recv_ts_ns`
- `symbol_id`
- `day_ntl_vlm`
- `prev_day_px`
- `mark_px`
- `mid_px`
- `circulating_supply`

Validation rules:

- Numeric fields may be missing but must be non-negative when present.
- `day_ntl_vlm` and `mark_px` are display/reference fields, not a substitute for trade-derived features.

Relationships:

- Updates 24h volume, mark price, context mid, and universe ranking.

## Entity: All-Mids Event

Normalized all-market mids update.

Fields:

- `recv_ts_ns`
- `mids_by_hl_coin`

Validation rules:

- Unknown coin identifiers are retained for diagnostics but not displayed until metadata mapping exists.
- Mid values must be positive.

Relationships:

- Updates fallback mid references across symbols.

## Entity: Candle Event

Exchange-provided aggregate candle, used for display, validation, and fallback only.

Fields:

- `recv_ts_ns`
- `open_ts_ms`
- `close_ts_ms`
- `symbol_id`
- `interval`
- `open`
- `high`
- `low`
- `close`
- `volume_base`
- `trade_count`

Validation rules:

- `interval` must match configured supported intervals.
- `open_ts_ms` must be before or equal to `close_ts_ms`.
- OHLC values must be positive and internally consistent.
- Candles must not replace trade/BBO as the feature source of truth.

Relationships:

- Feeds mini chart and validation checks.

## Entity: Feature Snapshot

Latest screen row for one symbol.

Fields:

- `symbol_id`
- `symbol`
- `price`
- `mid_px`
- `mark_px`
- `day_ntl_vlm`
- `bid_px`
- `bid_sz`
- `ask_px`
- `ask_sz`
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
- `staleness_state`
- `incomplete_window_reason`

Validation rules:

- Scores must be bounded from 0 to 100.
- `tob_imbalance` must be between -1 and 1 when available.
- `spread_bps` must be non-negative when available.
- Freshness must be derived from event timestamps, not render time alone.

Relationships:

- Consumed by TUI, CLI screen command, local API, and replay validation.

## Entity: Screen Rule

Filter and sort definition for a screen.

Fields:

- `rule_id`
- `source`: `preset` or `custom`
- `where_expr`
- `sort_fields`
- `created_at_ms`
- `last_validated_at_ms`

Validation rules:

- All identifiers must belong to the supported screen field set.
- Comparisons must use compatible types.
- Invalid rules must not replace the last valid active rule.

Relationships:

- Evaluated against feature snapshots.

## Entity: Recording Run

One live or replay data session.

Fields:

- `run_id`
- `started_at_ms`
- `ended_at_ms`
- `config_hash`
- `app_version`
- `git_sha`
- `raw_enabled`
- `normalized_enabled`
- `clean_shutdown`
- `gap_count`

Validation rules:

- `run_id` must be unique.
- `ended_at_ms` may be absent while active.
- Clean shutdown is true only after writers flush and metadata is committed.

Relationships:

- Owns raw files, normalized files, data gaps, and recording metrics.

## Entity: File Registry Entry

Metadata about one local raw or normalized file.

Fields:

- `path`
- `event_type`
- `symbol_id`
- `start_ts_ms`
- `end_ts_ms`
- `rows`
- `bytes`
- `created_at_ms`
- `run_id`

Validation rules:

- `path` must be unique.
- Time range must be present for normalized event files.
- Row and byte counts must be non-negative.

Relationships:

- Used by replay and inspection commands.

## Entity: Data Gap

Explicit interval where live data may be missing or incomplete.

Fields:

- `gap_id`
- `run_id`
- `conn_id`
- `started_at_ns`
- `ended_at_ns`
- `reason`
- `affected_symbols`
- `recovered`

Validation rules:

- Start must be before end once closed.
- Reason must be operator-readable.
- Affected symbols may be empty when the gap is connection-wide.

Relationships:

- Referenced by health status and feature snapshots.

## State Transitions

Recording run:

```text
configured -> starting -> live -> reconnecting -> live -> stopping -> clean_shutdown
                                      |                       |
                                      v                       v
                                  degraded              interrupted
```

Connection:

```text
disconnected -> connecting -> connected -> stale -> ping_sent -> reconnecting -> connected
```

Screen rule:

```text
draft -> valid -> active
draft -> invalid -> rejected
active -> replaced_by_valid_rule
```
