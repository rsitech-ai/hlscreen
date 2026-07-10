# Initial Parquet Export

Date: 2026-07-08

## Scope

This slice implements true Parquet output for normalized event rows only. It does not implement public reconnect backfill, schema migration, Parquet replay, feature/confidence Parquet datasets, or a completed DuckDB release gate.

## Implemented

- `hls-store::parquet::export_normalized_events_to_parquet`
- `hls export-parquet --data-dir <dir> --run-id <run-id>`
- `hls record --parquet`
- bounded `hls live --record --parquet`
- SQLite registry entry type: `normalized_parquet`
- Output path: `parquet/events/run=<run-id>/part-000000.parquet`

## Schema

Initial schema: `hls_normalized_events_v1`

| Column | Type | Notes |
| --- | --- | --- |
| `row_index` | `INT64` | Zero-based normalized row order. |
| `event_type` | `BINARY UTF8` | `trade`, `top_of_book`, `asset_context`, `all_mids`, or `candle`. |
| `recv_ts_ns` | `INT64` | Local receive timestamp. |
| `hl_coin` | optional `BINARY UTF8` | Symbol/feed identifier when scoped to one market. |
| `event_json` | `BINARY UTF8` | Full serialized `MarketEvent` for first-slice parity. |

## Validation

Passed:

```bash
cargo test -p hls-store --test parquet_export
cargo test -p hls-cli --test export_parquet_command
tmpdir="$(mktemp -d /tmp/hlscreen-parquet-smoke.XXXXXX)"
./target/debug/hls record --symbols @107 --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --parquet --run-id parquet-smoke --data-dir "$tmpdir"
test -s "$tmpdir/parquet/events/run=parquet-smoke/part-000000.parquet"
./target/debug/hls export-parquet --data-dir "$tmpdir" --run-id parquet-smoke
cargo fmt --check
cargo clippy -p hls-store -p hls-cli --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build --workspace --all-features
scripts/check-release-packaging.sh
git diff --check
```

Manual smoke output included:

```text
parquet_file=parquet/events/run=parquet-smoke/part-000000.parquet
parquet_rows=6
event_type=normalized_parquet
rows=6
```

## Remaining Work

- Add explicit schema version model.
- Add public REST backfill attempt metadata.
- Add repaired/unrepaired confidence semantics.
- Add supported/unsupported schema fixtures.
- Run and document an actual DuckDB smoke gate.
- Decide whether replay should consume Parquet directly or keep JSONL as canonical replay format.
