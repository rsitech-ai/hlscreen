# Quickstart: Data Formats And Backfill

## Parquet Proof

```bash
cargo test -p hls-store --test parquet_export
cargo test -p hls-store --test replay_parity replay_can_use_normalized_event_parquet_as_input
cargo test -p hls-cli --test export_parquet_command
cargo test -p hls-cli --test replay_parity_command replay_command_can_read_normalized_event_parquet
./target/debug/hls export-parquet --data-dir /tmp/hlscreen-run --run-id <run-id>
./target/debug/hls replay --data-dir /tmp/hlscreen-run --run-id <run-id> --input parquet --verify-parity
./target/debug/hls export-parquet --data-dir /tmp/hlscreen-run --run-id <run-id> --dataset features
./target/debug/hls export-parquet --data-dir /tmp/hlscreen-run --run-id <run-id> --dataset all
duckdb -c "select count(*) from read_parquet('/tmp/hlscreen-run/parquet/**/*.parquet');"
```

If the standalone `duckdb` binary is unavailable but the Python package is installed, the
same smoke can be run with:

```bash
python3 - <<'PY'
import duckdb
print(duckdb.sql("select count(*) from read_parquet('/tmp/hlscreen-run/parquet/**/*.parquet')").fetchone()[0])
PY
```

## Backfill Proof

```bash
cargo test -p hls-store --test backfill_gaps
./target/debug/hls replay --data-dir /tmp/hlscreen-run --run-id <run-id> --verify-parity
```

Expected: candle-covered gaps remain `partially_repaired`, empty responses remain
`unrepaired`, and replay keeps the reconnect-gap confidence penalty because
trades/BBO were not reconstructed. Automatic invocation from `hls live` remains
open in T018.
