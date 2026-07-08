# Data Format

Raw records preserve exact public market-data payloads with local receive timestamps, connection identity, sequence number, and channel.

Normalized records are derived from raw records and cover trades, top-of-book quotes, asset contexts, all-market mids, candles, data gaps, and recording runs.

Top-of-book metrics must be labeled as `tob_depth_usd` and `tob_imbalance`. They are not full book depth.

## WebSocket Fixture Format

US1 parser tests use newline-delimited JSON with one public Hyperliquid WebSocket envelope per line:

```json
{"channel":"trades","data":[{"coin":"@107","side":"B","px":"35.00","sz":"2.0","hash":"0xabc","time":1710000000000,"tid":11,"users":["0xbuyer","0xseller"]}]}
```

Supported fixture channels are `trades`, `bbo`, `allMids`, `activeAssetCtx`, and `candle`. Private/user channels such as `userFills`, `userEvents`, `orderUpdates`, and `openOrders` are rejected by the parser.

## Current Local Recording Format

US2 writes raw public WebSocket messages to compressed newline-delimited JSON:

```text
raw/ws/run=<run-id>/part-000000.ndjson.zst
```

Each decompressed line is a `RawMarketMessage` with `recv_ts_ns`, `conn_id`, `seq`, `channel`, and the preserved JSON `payload`.

US2 writes normalized replay events as deterministic JSONL:

```text
normalized/events/run=<run-id>/part-000000.ndjson
```

Each line is a serialized `MarketEvent`. This is the replay source used by `hls replay`.

The local SQLite registry lives at:

```text
hls.sqlite
```

It tracks `runs`, `files`, `symbols`, `data_gaps`, and
`confidence_snapshots`. Confidence snapshots are keyed by recording run, replay
timestamp, and symbol so `hls replay --verify-parity` can compare recomputed
data-quality state against a persisted local baseline. The registry is
local-only and stores no secrets.

True Parquet output is not implemented in the current slice. The CLI rejects `--parquet` and asks for `--normalized` until the Parquet writer is added.
