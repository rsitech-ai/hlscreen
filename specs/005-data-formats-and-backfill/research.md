# Research: Data Formats And Backfill

## Decision: Use `parquet` 59.1.0 Low-Level Writer For Initial Event Export

Rationale: `parquet` 59.1.0 is the official Apache Arrow Rust Parquet crate, is Apache-2.0 licensed, and declares Rust 1.85 minimum support, which is compatible with this workspace's Rust 1.88 policy. The initial implementation uses `default-features = false` and the low-level `SerializedFileWriter` API so normalized-event export does not pull the Arrow stack before a fuller analytical schema is designed.

Alternatives considered:

- Arrow-backed writer: easier table construction and richer type integration, but heavier dependency surface for this first slice.
- Polars/DuckDB integration: useful consumer examples, not the right core writer dependency for a Rust CLI library.
- Keep JSONL only: preserves replay simplicity, but does not satisfy the analytical Parquet roadmap item.

## Decision: Export Normalized Events Before Feature/Backfill Datasets

Rationale: normalized JSONL is already the replay source of truth, so a Parquet event export can be tested for row parity without changing live ingestion semantics. Feature snapshots, confidence snapshots, repaired-gap metadata, and schema migration remain separate tasks.

Alternatives considered:

- Stream Parquet directly from WebSocket ingestion: higher risk and easier to corrupt on interrupted live runs.
- Replace JSONL replay with Parquet replay immediately: too broad for this slice and would weaken existing replay parity gates.
