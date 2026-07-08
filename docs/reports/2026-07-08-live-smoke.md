# 2026-07-08 Live Smoke Report

## Scope

- Branch: `feat/andrzej_hlscreen_live_smoke_tui`
- Mode: read-only public Hyperliquid spot market data.
- Run ID: `allpairs-15m-20260708-084527`
- Data directory: `/tmp/hlscreen-allpairs-15m-20260708-084527`

No wallet, private-key, user-specific stream, order, signing, or exchange-action surface was used.

## Official Docs Checked

- WebSocket endpoint: `wss://api.hyperliquid.xyz/ws`
- Subscription envelope: `{"method":"subscribe","subscription":...}`
- Heartbeat: client sends `{"method":"ping"}` and server responds on `pong`
- Subscription limit: 1,000 subscriptions per IP
- Spot metadata source: `POST /info` with `type=spotMetaAndAssetCtxs`

## Implementation Notes

- Live mode selects public spot symbols from `spotMetaAndAssetCtxs`.
- `--all-symbols` selected 308 spot markets during this run.
- Four streams per symbol would require 1,232 subscriptions, so all-symbol mode used three public streams per symbol: `trades`, `bbo`, and `activeAssetCtx`.
- Total public subscriptions: 924, below the configured 980 headroom and official 1,000 limit.
- Runtime payload hardening added support for `activeSpotAssetCtx` channel aliases and string-encoded numeric fields in spot asset context and candle payloads.

## Command

```bash
./target/debug/hls live \
  --all-symbols \
  --duration-secs 900 \
  --refresh-secs 60 \
  --record \
  --raw \
  --normalized \
  --run-id allpairs-15m-20260708-084527 \
  --data-dir /tmp/hlscreen-allpairs-15m-20260708-084527
```

## Result

- `symbols=308`
- `subscriptions=924`
- `streams_per_symbol=3`
- `ws_messages=298082`
- `market_events=306140`
- `elapsed_secs=900`
- `raw_messages=298082`
- `normalized_events=306140`
- `raw_files=13`
- `normalized_files=1`
- `clean_shutdown=true`
- Final rows: `308`
- Final row states: `fresh=34`, `stale=264`, `incomplete=10`

## Replay/Registry Verification

```bash
wc -l /tmp/hlscreen-allpairs-15m-20260708-084527/normalized/events/run=allpairs-15m-20260708-084527/part-000000.ndjson
zstdcat /tmp/hlscreen-allpairs-15m-20260708-084527/raw/ws/run=allpairs-15m-20260708-084527/*.zst | wc -l
sqlite3 /tmp/hlscreen-allpairs-15m-20260708-084527/hls.sqlite \
  "select run_id, raw_enabled, normalized_enabled, clean_shutdown, gap_count from runs;
   select event_type, count(*), sum(rows) from files group by event_type;
   select count(*) from symbols;"
./target/debug/hls replay --data-dir /tmp/hlscreen-allpairs-15m-20260708-084527 --run-id allpairs-15m-20260708-084527
./target/debug/hls screen --data-dir /tmp/hlscreen-allpairs-15m-20260708-084527 --run-id allpairs-15m-20260708-084527 --sort liquidity_score:desc
```

Observed:

- Normalized JSONL rows: `306140`
- Decompressed raw rows: `298082`
- SQLite run: `clean_shutdown=1`, `gap_count=0`
- SQLite file registry: `normalized_jsonl=1/306140`, `raw_ws=13/298082`
- SQLite symbol registry: `308`
- Replay and screen commands loaded the run and rendered 308 rows.

## Validation

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `cargo build --release --workspace --all-features`
- `git diff --check`
- `python3 scripts/generate-screenshots.py`

## Remaining Limits At Time Of This Report

- This report predated the later reconnect/resubscribe hardening slice. At the time of this run, server-side disconnect recovery was not implemented.
- `--parquet` remains intentionally rejected until a real Parquet writer exists.
- Long-running localhost HTTP serving is still not implemented.
