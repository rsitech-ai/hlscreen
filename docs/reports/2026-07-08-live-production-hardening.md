# 2026-07-08 Live Production Hardening Report

## Scope

- Branch: `feat/andrzej_hlscreen_live_production_hardening`
- Mode: read-only public Hyperliquid spot market data.
- Run ID: `allpairs-15m-hardening-20260708-093507`
- Data directory: `/tmp/hlscreen-allpairs-15m-hardening-20260708-093507`

No wallet, private-key, user-specific stream, order, signing, or exchange-action surface was used.

## Official Docs Checked

- WebSocket endpoint: `wss://api.hyperliquid.xyz/ws`
- Subscription envelope: `{"method":"subscribe","subscription":...}`
- Heartbeat: client sends `{"method":"ping"}` and server responds on `pong`
- Automated clients should handle server disconnects and reconnect gracefully.
- Subscription limit: 1,000 subscriptions per IP.
- Spot metadata source: `POST /info` with `type=spotMetaAndAssetCtxs`.

## Hardening Changes Verified

- Live WebSocket loop reconnects and resubscribes until the configured duration elapses.
- If no WebSocket messages are ever received after reconnect attempts, live mode fails closed instead of printing a false-green completion summary.
- Raw and normalized live recording now runs through a bounded writer worker; queue backpressure is treated as an error to avoid silent data loss.
- Raw frames and normalized events carry non-zero receive timestamps from the live read loop.
- Reconnect windows are persisted as data gaps when recording is enabled.
- TUI refresh is available in TTY sessions and with `--tui`; public live smoke logs showed no negative age values after clamping future exchange timestamps.
- Asset context frames refresh staleness using live receive time, so all-symbol rows reflect current data freshness even when a market has no recent trade or BBO update.

## Command

```bash
./target/debug/hls live \
  --all-symbols \
  --duration-secs 900 \
  --refresh-secs 60 \
  --tui \
  --record \
  --raw \
  --normalized \
  --run-id allpairs-15m-hardening-20260708-093507 \
  --data-dir /tmp/hlscreen-allpairs-15m-hardening-20260708-093507
```

## Result

- `symbols=308`
- `subscriptions=924`
- `streams_per_symbol=3`
- `ws_messages=296492`
- `market_events=304405`
- `reconnects=0`
- `data_gaps=0`
- `elapsed_secs=900`
- `raw_messages=296492`
- `normalized_events=304405`
- `raw_files=13`
- `normalized_files=1`
- `clean_shutdown=true`
- Final rows: `308`
- Final row states: `fresh=308`, `stale=0`, `incomplete=0`

All-symbol live mode used three public streams per symbol: `trades`, `bbo`, and `activeAssetCtx`. Four streams per 308 symbols would require 1,232 subscriptions, which exceeds Hyperliquid's documented 1,000-subscription per-IP cap.

## Replay/Registry Verification

```bash
wc -l /tmp/hlscreen-allpairs-15m-hardening-20260708-093507/normalized/events/run=allpairs-15m-hardening-20260708-093507/part-000000.ndjson
zstdcat /tmp/hlscreen-allpairs-15m-hardening-20260708-093507/raw/ws/run=allpairs-15m-hardening-20260708-093507/*.zst | wc -l
sqlite3 /tmp/hlscreen-allpairs-15m-hardening-20260708-093507/hls.sqlite \
  "select run_id, raw_enabled, normalized_enabled, clean_shutdown, gap_count from runs;
   select event_type, count(*), sum(rows) from files group by event_type;
   select count(*) from symbols;
   select count(*) from data_gaps;"
./target/debug/hls replay --data-dir /tmp/hlscreen-allpairs-15m-hardening-20260708-093507 --run-id allpairs-15m-hardening-20260708-093507
./target/debug/hls screen --data-dir /tmp/hlscreen-allpairs-15m-hardening-20260708-093507 --run-id allpairs-15m-hardening-20260708-093507 --sort liquidity_score:desc
```

Observed:

- Normalized JSONL rows: `304405`
- Decompressed raw rows: `296492`
- SQLite run: `clean_shutdown=1`, `gap_count=0`
- SQLite file registry: `normalized_jsonl=1/304405`, `raw_ws=13/296492`
- SQLite symbol registry: `308`
- SQLite data gaps: `0`
- Replay and screen commands loaded the run and rendered `308` fresh rows.
- Normalized receive timestamp scan: `304405/304405` events had non-zero `recv_ts_ns`.
- TUI age-column scan: no negative age values.
- Stdout/stderr log scan: zero `error`, `panic`, `failed`, or `queue is full` matches.

## Validation

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo build --workspace --all-features`
- `cargo build --release --workspace --all-features`
- `git diff --check`
- `python3 scripts/generate-screenshots.py`
- Negative endpoint probe: `hls live --ws-url ws://127.0.0.1:1` exited non-zero after reconnect attempts and reported no received WebSocket messages.

## Remaining Limits

- Automatic public REST backfill after a reconnect is not implemented. Reconnect windows are recorded as data gaps.
- `--parquet` remains intentionally rejected until a real Parquet writer exists.
- Long-running localhost HTTP serving is still not implemented.
