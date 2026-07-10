# Feature Specification: Data Formats And Backfill

**Feature Branch**: `005-data-formats-and-backfill`

**Created**: 2026-07-08

**Status**: Draft

**Input**: Add true Parquet output, DuckDB examples, reconnect backfill where public data permits, schema stability, and migration handling.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Write Analytical Parquet (Priority: P1)

A researcher can record public market data and query it efficiently with analytical tools without losing replay compatibility.

**Why this priority**: JSONL is replayable but inefficient for larger research workflows.

**Independent Test**: Record or replay fixture data, write Parquet, query it with DuckDB, and verify row counts and key fields match normalized JSONL.

**Acceptance Scenarios**:

1. **Given** a recorded run, **When** Parquet export runs, **Then** files use documented schemas and row counts match normalized replay data.
2. **Given** Parquet output is requested for an unsupported schema, **When** export runs, **Then** it fails clearly without partial silent output.

---

### User Story 2 - Backfill Public Gaps After Reconnect (Priority: P1)

An operator can distinguish recorded gaps with coarse public candle coverage from gaps with no public coverage.

**Why this priority**: Current reconnect gaps are honest but unrepaired, which limits long-running capture quality.

**Independent Test**: Simulate reconnect gaps, append supported public candle evidence, and verify the original trade/BBO gap remains explicit and confidence-degraded.

**Acceptance Scenarios**:

1. **Given** a reconnect gap with public candle coverage, **When** coarse rows are appended, **Then** metadata marks the attempt partially repaired and keeps the gap confidence penalty.
2. **Given** a gap cannot be publicly backfilled, **When** replay runs, **Then** confidence remains degraded and the reason is visible.

---

### User Story 3 - Preserve Schema Compatibility (Priority: P2)

A maintainer can evolve storage schemas without breaking old recordings silently.

**Why this priority**: Public users will keep local recordings across releases.

**Independent Test**: Replay older fixture schemas and verify explicit migration or unsupported-version errors.

**Acceptance Scenarios**:

1. **Given** an older supported schema, **When** replay/export runs, **Then** migration behavior is documented and tested.
2. **Given** an unsupported schema, **When** replay/export runs, **Then** the command exits non-zero with a clear version error.

### Edge Cases

- Public endpoints do not provide enough history for a missing interval.
- Backfilled data has coarser timestamps than WebSocket data.
- Parquet writer is interrupted mid-file.
- Schema evolution changes nullable fields or metric names.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST implement true Parquet output with documented schemas.
- **FR-002**: Parquet row counts and key fields MUST be validated against normalized JSONL for the same run.
- **FR-003**: DuckDB examples MUST query exported Parquet without private dependencies.
- **FR-004**: Backfill MUST use public data only and record whether each gap was repaired, partially repaired, or unrepaired.
- **FR-005**: Confidence state MUST account for repaired and unrepaired gaps differently.
- **FR-006**: Schema versions MUST be explicit for normalized, SQLite, and Parquet outputs.
- **FR-007**: Unsupported historical data MUST fail with a clear unsupported-version message.

### Key Entities *(include if feature involves data)*

- **Parquet Dataset**: Columnar export of public events, feature snapshots, confidence, and metadata.
- **Backfill Attempt**: Gap interval, source, result status, row count, and confidence impact.
- **Schema Version**: Versioned storage contract and migration status.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Fixture export writes Parquet files with row counts matching normalized JSONL.
- **SC-002**: DuckDB smoke queries complete successfully against exported Parquet.
- **SC-003**: Backfill fixture tests classify candle coverage as partial and empty coverage as unrepaired without hiding the original gap.
- **SC-004**: Unsupported schema fixtures fail non-zero with an actionable message.

## Assumptions

- Current public REST coverage is limited to coarse candles and is not tick-level repair.
- Parquet export can be a separate command or recording flag as long as replay compatibility is preserved.
