# Implementation Plan: Data Formats And Backfill

**Branch**: `005-data-formats-and-backfill` | **Date**: 2026-07-08 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/005-data-formats-and-backfill/spec.md`

## Summary

Add durable analytical storage and gap repair: true Parquet output, DuckDB examples, public-only reconnect backfill, schema versioning, and migration/unsupported-version handling.

## Technical Context

**Language/Version**: Rust stable, edition 2024.

**Primary Dependencies**: Existing `hls-store`, `hls-hyperliquid`, `hls-cli`; `parquet` 59.1.0 with default features disabled for the initial normalized-event writer; DuckDB CLI for smoke examples.

**Storage**: Raw `.ndjson.zst`, normalized JSONL, SQLite registry, new Parquet datasets, schema manifests.

**Testing**: Parquet row-count tests, DuckDB smoke, backfill fixtures, schema migration fixtures, replay parity.

**Target Platform**: Local macOS/Linux.

**Project Type**: Rust CLI storage/export and replay compatibility feature.

**Performance Goals**: Export large local runs without blocking live ingestion; backfill should be bounded and rate-limit aware.

**Constraints**: Public data only; no silent partial files; no weakening confidence after unrepaired gaps.

**Scale/Scope**: Local recordings and all-symbol runs. Cloud data lake service is out of scope.

## Constitution Check

- **Read-only public data boundary**: PASS. Backfill uses public data only.
- **Replayable evidence before ranking**: PASS. Storage/replay evidence is the core feature.
- **Live truth over mock convenience**: PASS. Backfill status is explicit when public data is incomplete.
- **Operator safety and observability**: PASS. Partial/interrupted outputs fail clearly.
- **Open-source reproducibility**: PASS. DuckDB examples are local and public.

## Project Structure

```text
crates/hls-store/src/
crates/hls-hyperliquid/src/
crates/hls-cli/src/commands/
tests/fixtures/microstructure/
docs/data-format.md
docs/reports/
```

## Phase 0: Research Summary

- Use the Apache Arrow Rust `parquet` crate low-level writer first to keep the initial dependency footprint smaller than an Arrow-backed table writer.
- Use public REST backfill only where source semantics match the missing stream.
- Keep JSONL as the canonical recording format while also proving explicit normalized-event Parquet replay compatibility through `hls replay --input parquet`.

## Phase 1: Design Summary

Parquet export is an additional analytical output, not a replacement for raw/normalized replay evidence. The first implemented dataset is `hls_normalized_events_v1`; backfill writes explicit attempt metadata later so downstream confidence can distinguish repaired and unrepaired windows.

## Complexity Tracking

Parquet introduces a heavier dependency; it must be justified by row-count tests, DuckDB smoke, and documented schemas.
