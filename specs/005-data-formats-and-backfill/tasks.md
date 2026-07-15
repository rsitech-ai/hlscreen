# Tasks: Data Formats And Backfill

**Input**: Design documents from `specs/005-data-formats-and-backfill/`

## Phase 1: Setup

- [x] T001 Research Parquet crate choice and MSRV impact in `specs/005-data-formats-and-backfill/research.md`
- [x] T002 [P] Add storage schema documentation outline in `docs/data-format.md`

## Phase 2: Foundational

- [x] T003 Add schema version model in `crates/hls-store/src/schema.rs`
- [x] T004 Add backfill attempt model in `crates/hls-store/src/metadata.rs`
- [x] T005 Add Parquet writer module placeholder and tests in `crates/hls-store/tests/parquet_export.rs`

## Phase 3: User Story 1 - Write Analytical Parquet (Priority: P1)

- [x] T006 [P] [US1] Add JSONL-to-Parquet parity fixture in `tests/fixtures/microstructure/`
- [x] T007 [US1] Implement normalized-event and feature/confidence Parquet export in `crates/hls-store/src/parquet.rs`
- [x] T008 [US1] Add CLI export command and dataset selector in `crates/hls-cli/src/commands/export_parquet.rs`
- [x] T009 [US1] Add DuckDB smoke documentation in `docs/data-format.md`
- [x] T019 [US1] Implement explicit normalized-event Parquet replay input in `crates/hls-store/src/replay.rs` and `crates/hls-cli/src/commands/replay.rs`

## Phase 4: User Story 2 - Backfill Public Gaps After Reconnect (Priority: P1)

- [x] T010 [P] [US2] Add partial-coverage and unrepaired public candle fixtures in `tests/fixtures/microstructure/`
- [x] T011 [US2] Implement public `candleSnapshot` backfill adapter in `crates/hls-hyperliquid/src/rest.rs`
- [x] T012 [US2] Record backfill attempt metadata in `crates/hls-store/src/metadata.rs`
- [x] T013 [US2] Keep reconnect-gap confidence degraded when coarse candle evidence is appended
- [x] T018 [US2] Wire live closeout candle coverage without marking tick gaps recovered
- [x] T020 [US2] Document that the official real-time public API has no historical trade/BBO reconnect endpoint; delayed, requester-paid, potentially incomplete archives remain best-effort offline research inputs only

## Phase 5: User Story 3 - Preserve Schema Compatibility (Priority: P2)

- [x] T014 [P] [US3] Add supported and unsupported schema fixtures in `tests/fixtures/microstructure/`
- [x] T015 [US3] Implement explicit schema migration or unsupported-version errors in `crates/hls-store/src/schema.rs`

## Phase 6: Polish

- [x] T016 Run full validation and DuckDB smoke from `quickstart.md`
- [x] T017 Update docs, reports, and memory with evidence
