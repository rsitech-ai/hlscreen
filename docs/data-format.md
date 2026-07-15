# Data Format

Raw records preserve exact public market-data payloads with local receive timestamps, connection identity, sequence number, and channel.

Normalized records are derived from raw records and cover trades, top-of-book quotes, asset contexts, all-market mids, candles, data gaps, and recording runs.

Committed examples follow the [test fixture policy](../tests/fixtures/README.md),
which classifies fixture groups and prohibits secrets, real accounts, private
streams, and unredacted user data.

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

Each line is a serialized `MarketEvent`. This is the canonical replay source
used by `hls replay` unless an operator explicitly selects exported
normalized-event Parquet with `--input parquet`.

`run-id` values are unique recording identities. They are limited to 128 ASCII
bytes and the characters `A-Z`, `a-z`, `0-9`, `.`, `-`, and `_`; path
components and `.`/`..` are rejected before the data directory is created.
Existing run IDs cannot be replaced. Registry file paths must remain relative
normal paths beneath the configured data directory, and replay rejects
absolute or parent-traversing entries.

The local SQLite registry lives at:

```text
hls.sqlite
```

It tracks `runs`, `files`, `symbols`, `data_gaps`, `backfill_attempts`,
`confidence_snapshots`, and the schema-versioned `public_candle_cache` used by
`hls tui`. Cached candles are keyed by `(symbol, interval, open_ts_ms)` and
retain receive timestamp, `websocket`/`rest_bootstrap` provenance, and
`open`/`closed` completion state. Receive-older updates cannot replace newer
rows. Confidence snapshots are keyed
by recording run, replay timestamp, and symbol so `hls replay --verify-parity`
can compare recomputed data-quality state against a persisted local baseline.
The registry is local-only and stores no secrets. A symlinked cache or registry
database file is rejected during open rather than followed.

## Backfill Attempt Metadata

Detected data gaps are recorded in `data_gaps`. Public repair attempts are
recorded separately in `backfill_attempts` so detection provenance does not get
mixed with repair provenance.

`backfill_attempts` stores:

| Column | Notes |
| --- | --- |
| `attempt_id` | Unique append-only identifier for one attempt. |
| `run_id` | Recording run that observed the gap. |
| `gap_id` | Gap being repaired or inspected. |
| `source` | Public source used for the attempt, such as a REST trade or quote endpoint. |
| `requested_start_ns` / `requested_end_ns` | Requested public-data repair window. |
| `attempted_at_ms` | Local attempt timestamp. |
| `status` | `repaired`, `partially_repaired`, or `unrepaired`. |
| `rows_written` | Number of recovered rows written by that attempt. |
| `confidence_impact` | `restored`, `partial`, or `degraded`. |
| `notes` | Optional operator/debug note. |

`hls backfill` runs the public candle source over recorded gaps. An operator can
also opt into the same clean-closeout path with
`hls live --record --backfill-gaps`; normalized output is required. Returned
candles are appended to a normalized JSONL evidence file,
but they cannot reconstruct missing trades or BBO updates. Any non-empty candle
result is therefore recorded as `partially_repaired` with
`confidence_impact = partial`; the original gap stays unrecovered and replay
keeps the reconnect-gap confidence penalty. Empty responses remain `unrepaired`
with `confidence_impact = degraded`. Public REST failures are also persisted as
unrepaired attempts with per-symbol failure notes, and the command exits non-zero
after writing that evidence. Repeating the same gap/source/interval skips the
existing attempt by default; `hls backfill --retry` explicitly requests another.

```bash
hls backfill --data-dir /var/tmp/hlscreen-data --run-id <run-id> --interval 1m

hls live --all-symbols --duration-secs 900 --record --normalized \
  --backfill-gaps --backfill-interval 1m \
  --data-dir /var/tmp/hlscreen-data --run-id <run-id>
```

The backfill file row and attempt row are committed in one SQLite transaction.
The candidate evidence file is created without replacement and remains pending
until that transaction commits; any write or registry failure removes the
unregistered candidate file. This prevents a failed attempt from leaving a
partial registry claim or an orphaned final artifact during normal error
handling.

The original `data_gaps.recovered` flag remains false after candle coverage.

## Public Backfill Source Boundary

The current public source adapter covers Hyperliquid `candleSnapshot` responses
for coarse historical candle coverage evidence. It parses documented public candle
fields into local `CandleEvent` rows and accepts empty public responses as valid
unrepaired-attempt input.

This does not reconstruct missing WebSocket trades or top-of-book updates.
Hyperliquid's public [`l2Book` info endpoint](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/info-endpoint)
is a current snapshot, not historical BBO replay. The official
[historical-data archive](https://hyperliquid.gitbook.io/hyperliquid-docs/historical-data)
publishes L2 snapshots roughly monthly, warns that updates may be untimely or
missing, and requires the requester to pay transfer costs. Separate node
trade/fill archives are not a bounded,
low-latency reconnect endpoint either. Therefore exact automatic live trade/BBO
reconstruction is not supportable from the documented public API. A future
offline archive importer may add best-effort research evidence, but it must not
restore live-gap confidence or be described as lossless repair.

## Parquet Event Export

`hlscreen` exports every `normalized_jsonl` part registered for a run, in
registry path order, into one analytical Parquet file:

```text
parquet/events/run=<run-id>/part-000000.parquet
```

The initial Parquet schema is `hls_normalized_events_v1`:

| Column | Type | Notes |
| --- | --- | --- |
| `row_index` | `INT64` | Zero-based row order from the normalized event stream. |
| `event_type` | `BINARY UTF8` | One of `trade`, `top_of_book`, `asset_context`, `all_mids`, or `candle`. |
| `recv_ts_ns` | `INT64` | Local receive timestamp from the normalized event. |
| `hl_coin` | optional `BINARY UTF8` | Hyperliquid feed identifier when the event is symbol-scoped; null for all-market mids. |
| `event_json` | `BINARY UTF8` | Full serialized `MarketEvent` for lossless first-slice parity with JSONL. |

Export existing normalized events:

```bash
./target/debug/hls export-parquet --data-dir "$tmpdir" --run-id smoke --dataset events
```

`hls record --parquet` and `hls live --record --parquet` imply normalized capture and export this Parquet file after the bounded recording closes cleanly. Raw compressed capture and normalized JSONL remain the canonical recording formats, but exported normalized-event Parquet can be replayed explicitly:

```bash
./target/debug/hls replay --data-dir "$tmpdir" --run-id smoke --input parquet
./target/debug/hls replay --data-dir "$tmpdir" --run-id smoke --input parquet --verify-parity
```

Each normalized-event Parquet dataset also writes a machine-readable schema
manifest:

```text
parquet/events/run=<run-id>/schema.json
```

Current manifest shape:

```json
{
  "manifest_version": 1,
  "normalized_event_schema_version": 1,
  "sqlite_schema_version": 1,
  "parquet_event_schema_version": 1
}
```

`hls-store` validates this manifest fail-closed. Unsupported manifest,
normalized-event, SQLite, or Parquet event versions return an actionable
unsupported-version error instead of guessing a migration.
`hls replay --input parquet` also requires this manifest; if the manifest or
registered `normalized_parquet` file is missing, replay fails instead of
falling back to JSONL.
The run and all JSONL inputs must already be registered. Existing Parquet or
schema evidence is never replaced; a failed export removes files created by
that failed attempt instead of registering orphan evidence.
`--dataset all` prepares both event and feature datasets before registering
either one, then commits both registry rows in one SQLite transaction. A
failure therefore leaves neither dataset partially committed.

## Parquet Feature/Confidence Export

`hlscreen` can also replay a normalized run and export the resulting
`FeatureSnapshot` rows with embedded data-confidence state:

```text
parquet/features/run=<run-id>/part-000000.parquet
```

Export feature/confidence rows:

```bash
./target/debug/hls export-parquet --data-dir "$tmpdir" --run-id smoke --dataset features
```

Export both normalized events and feature/confidence rows:

```bash
./target/debug/hls export-parquet --data-dir "$tmpdir" --run-id smoke --dataset all
```

The first feature schema is `hls_feature_snapshots_v1`:

| Column | Type | Notes |
| --- | --- | --- |
| `row_index` | `INT64` | Zero-based row order from the replayed snapshot set. |
| `snapshot_ts_ms` | `INT64` | Replay snapshot timestamp used for all rows in the export. |
| `symbol` | `BINARY UTF8` | Hyperliquid feed identifier for the feature row. |
| `confidence_score` | `INT64` | Data confidence score from `0` to `100`. |
| `confidence_level` | `BINARY UTF8` | `high`, `medium`, `low`, or `untrusted`. |
| `confidence_reasons_json` | `BINARY UTF8` | JSON array of confidence reason codes. |
| `price`, `mid_px`, `spread_bps`, `tob_depth_usd`, `tob_imbalance` | optional `DOUBLE` | Common analytical feature columns. |
| `liquidity_score`, `momentum_score`, `mean_reversion_score` | `DOUBLE` | Screening scores from the feature engine. |
| `tradeability_state` | `BINARY UTF8` | Public-data tradeability state. |
| `resilience_state` | `BINARY UTF8` | Liquidity resilience state. |
| `snapshot_json` | `BINARY UTF8` | Full serialized `FeatureSnapshot` for parity/debugging. |

Each feature dataset writes:

```text
parquet/features/run=<run-id>/schema.json
```

Current feature manifest shape:

```json
{
  "manifest_version": 1,
  "normalized_event_schema_version": 1,
  "sqlite_schema_version": 1,
  "parquet_feature_schema_version": 1
}
```

DuckDB smoke:

```bash
duckdb -c "select event_type, count(*) from read_parquet('$tmpdir/parquet/events/run=smoke/*.parquet') group by event_type order by event_type;"
duckdb -c "select symbol, confidence_level, spread_bps from read_parquet('$tmpdir/parquet/features/run=smoke/*.parquet') order by symbol;"
```

If the standalone DuckDB CLI is not installed, the same validation can run
through Python DuckDB:

```bash
python3 - <<'PY'
import duckdb
print(duckdb.sql("select event_type, count(*) from read_parquet('/tmp/hlscreen-run/parquet/events/run=smoke/*.parquet') group by event_type order by event_type").fetchall())
print(duckdb.sql("select symbol, confidence_level, spread_bps from read_parquet('/tmp/hlscreen-run/parquet/features/run=smoke/*.parquet') order by symbol").fetchall())
PY
```

Still planned: actual schema migrations when a v2 schema exists, optional
best-effort offline historical archive import, and
DuckDB validation in CI/release gates. Feature/confidence Parquet is an
analytical export; normalized-event Parquet is the replayable dataset.

## Benchmark Manifest Format

Benchmark packs are small public fixture manifests. They are designed to catch
feature and replay drift in CI without using live network calls.

```json
{
  "schema_version": 1,
  "fixture_id": "gap_replay_v1",
  "description": "Public reconnect-gap replay benchmark over BBO plus trades.",
  "input_files": ["tests/fixtures/microstructure/gap_replay.ndjson"],
  "expected_hash": "sha256:99ab7c75a7bdb03865307dcf0e6181d0901d672638eb33a8fb7351415d0364d6",
  "max_feature_latency_us": 100000,
  "tags": ["public", "gap", "replay", "microstructure"]
}
```

Rules:

- `schema_version` is currently `1`.
- `input_files` must be relative paths under `tests/fixtures/microstructure/`.
- Absolute paths, `..`, private/account naming, and `private` tags are rejected.
- `expected_hash` is the SHA-256 hash of canonical benchmark output built from
  parsed public events and computed feature snapshots.
- Expected hashes should only be updated with reviewed evidence because they are
  the drift guard.

Run:

```bash
./target/debug/hls bench \
  --manifest tests/fixtures/microstructure/benchmark_gap_replay.json \
  --repo-root . \
  --json
```
