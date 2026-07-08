# Data Files Contract

All paths are relative to the configured local data directory.

## Raw Messages

Planned v1 path pattern:

```text
raw/ws/date=YYYY-MM-DD/hour=HH/part-NNNNNN.ndjson.zst
```

Line shape:

```json
{
  "recv_ts_ns": 1783435200123456789,
  "conn_id": 0,
  "seq": 123,
  "channel": "trades",
  "payload": {}
}
```

Requirements:

- One JSON object per line before compression.
- Raw payload is preserved exactly enough to replay parser behavior.
- Files rotate by configured time or size.
- Raw writer must report dropped messages if its bounded queue overflows.

Current US2 implementation note:

- The first record/replay slice writes raw files at `raw/ws/run=<run-id>/part-NNNNNN.ndjson.zst`.

## Normalized Events

Planned v1 Parquet path patterns:

```text
parquet/trades/date=YYYY-MM-DD/hour=HH/coin=<coin>/part-NNNNNN.parquet
parquet/bbo/date=YYYY-MM-DD/hour=HH/coin=<coin>/part-NNNNNN.parquet
parquet/asset_ctx/date=YYYY-MM-DD/hour=HH/coin=<coin>/part-NNNNNN.parquet
parquet/mids/date=YYYY-MM-DD/hour=HH/part-NNNNNN.parquet
parquet/candles/interval=1m/date=YYYY-MM-DD/coin=<coin>/part-NNNNNN.parquet
parquet/gaps/date=YYYY-MM-DD/part-NNNNNN.parquet
```

Requirements:

- Event files include receive timestamps and exchange timestamps where available.
- Numeric market values preserve storage correctness and can be converted to feature calculations.
- File registry records path, event type, symbol, time range, row count, byte count, creation time, and run ID.

Current US2 implementation note:

- The first record/replay slice writes normalized `MarketEvent` rows as deterministic JSONL at `normalized/events/run=<run-id>/part-000000.ndjson`.
- The file registry marks these rows with `event_type = "normalized_jsonl"`.
- `--parquet` is intentionally rejected until a real Parquet writer exists.

## SQLite Metadata

Default path:

```text
hls.sqlite
```

Required tables:

- `symbols`
- `files`
- `runs`
- `data_gaps`

Requirements:

- The metadata database does not store secrets.
- `files.path` is unique.
- `runs.run_id` is unique.
- Recording shutdown commits final run and file metadata before reporting clean shutdown.
