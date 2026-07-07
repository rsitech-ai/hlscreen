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
