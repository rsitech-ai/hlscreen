# Tasks: Hyperliquid Microstructure Workstation

**Input**: Design documents from `specs/002-microstructure-workstation/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Tests**: Included because the feature spec and repo operating contract require tests for meaningful behavior, replay parity, metrics contracts, and safety boundaries.

**Organization**: Tasks are grouped by independently testable user story. Each story can be implemented as a separate PR after the shared foundation.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Prepare fixture locations, docs, and module placeholders without changing runtime behavior.

- [x] T001 Create microstructure fixture directories in `/Users/s1kor/dev/trading/rsibot/hlscreen/tests/fixtures/microstructure/`
- [x] T002 [P] Create microstructure golden output directory in `/Users/s1kor/dev/trading/rsibot/hlscreen/tests/golden/microstructure/`
- [x] T003 [P] Create microstructure documentation stub in `/Users/s1kor/dev/trading/rsibot/hlscreen/docs/microstructure.md`
- [x] T004 [P] Create benchmark fixture README in `/Users/s1kor/dev/trading/rsibot/hlscreen/tests/fixtures/microstructure/README.md`
- [x] T005 Add microstructure module exports placeholders in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-core/src/lib.rs`
- [x] T006 Add microstructure module exports placeholders in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-features/src/lib.rs`
- [x] T007 Add CLI command placeholder registration comments in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-cli/src/commands/mod.rs`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Define shared contracts used by every story: confidence, score breakdowns, benchmark manifests, metrics metadata, and fixture schemas.

**Critical**: No user story implementation should begin until this phase is complete.

### Tests for Foundation

- [x] T008 [P] Add confidence model unit tests in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-core/tests/confidence_state.rs`
- [x] T009 [P] Add score breakdown unit tests in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-core/tests/score_breakdown.rs`
- [x] T010 [P] Add metrics contract unit tests in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-core/tests/metrics_contract.rs`
- [x] T011 [P] Add benchmark manifest parsing tests in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-store/tests/benchmark_manifest.rs`
- [x] T012 [P] Add read-only safety regression tests for new CLI fields in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-cli/tests/microstructure_safety.rs`

### Implementation for Foundation

- [x] T013 Implement `DataConfidenceSnapshot` and reason codes in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-core/src/confidence.rs`
- [x] T014 Implement `ScoreBreakdown` and score component contracts in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-core/src/score.rs`
- [x] T015 Implement metric definition registry and label validation in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-core/src/metrics.rs`
- [x] T016 Implement benchmark fixture manifest model in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-store/src/benchmark.rs`
- [x] T017 Wire new foundation modules into `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-core/src/lib.rs`
- [x] T018 Wire benchmark module into `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-store/src/lib.rs`
- [x] T019 Update feature definitions docs for confidence and score terminology in `/Users/s1kor/dev/trading/rsibot/hlscreen/docs/feature-definitions.md`

**Checkpoint**: Foundation ready when `cargo test -p hls-core --test confidence_state --test score_breakdown --test metrics_contract` and `cargo test -p hls-store --test benchmark_manifest` pass.

---

## Phase 3: User Story 1 - Trust Data Quality During Live and Replay Sessions (Priority: P1) MVP

**Goal**: Make confidence-aware data quality and replay parity visible and testable.

**Independent Test**: Run gap, duplicate, sparse-trade, and writer-lag fixtures; verify confidence states and replay parity without live network access.

### Tests for User Story 1

- [x] T020 [P] [US1] Add reconnect-gap fixture in `/Users/s1kor/dev/trading/rsibot/hlscreen/tests/fixtures/microstructure/gap_replay.ndjson`
- [x] T021 [P] [US1] Add sparse-trade fixture in `/Users/s1kor/dev/trading/rsibot/hlscreen/tests/fixtures/microstructure/sparse_trades.ndjson`
- [x] T022 [P] [US1] Add duplicate-event confidence tests in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-core/tests/confidence_state.rs`
- [x] T023 [P] [US1] Add replay parity tests for confidence snapshots in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-store/tests/replay_parity.rs`
- [x] T024 [P] [US1] Add CLI replay parity test in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-cli/tests/replay_parity_command.rs`
- [x] T025 [P] [US1] Add TUI confidence rendering golden test in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-tui/tests/confidence_pane.rs`

### Implementation for User Story 1

- [x] T026 [US1] Attach confidence snapshots to feature snapshots in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-core/src/market_state.rs`
- [x] T027 [US1] Compute confidence from gaps, freshness, sparse data, duplicates, parser drops, and writer backlog in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-features/src/engine.rs`
- [x] T028 [US1] Persist confidence snapshot metadata in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-store/src/metadata.rs`
- [x] T029 [US1] Implement replay parity checker in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-store/src/replay.rs`
- [x] T030 [US1] Add `--verify-parity` replay flag in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-cli/src/commands/replay.rs`
- [x] T031 [US1] Render confidence state in market board rows in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-tui/src/app.rs`
- [x] T032 [US1] Include confidence summary in live/replay command output in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-cli/src/commands/live.rs`
- [x] T033 [US1] Document confidence states and replay parity in `/Users/s1kor/dev/trading/rsibot/hlscreen/docs/microstructure.md`

**Checkpoint**: User Story 1 complete when gap/replay fixtures degrade confidence correctly and replay parity detects drift.

---

## Phase 4: User Story 2 - Analyze Liquidity Resilience and Tradeability (Priority: P1)

**Goal**: Add BBO-plus-trade metrics for spread shocks, recovery, tradeability, and top-of-book adverse-selection proxies.

**Independent Test**: Feed deterministic BBO/trade shock fixtures and verify resilience labels and ordering.

### Tests for User Story 2

- [x] T034 [P] [US2] Add spread-shock fixture in `/Users/s1kor/dev/trading/rsibot/hlscreen/tests/fixtures/microstructure/resilience_shock.ndjson`
- [x] T035 [P] [US2] Add brittle-thin-book fixture in `/Users/s1kor/dev/trading/rsibot/hlscreen/tests/fixtures/microstructure/thin_brittle_book.ndjson`
- [x] T036 [P] [US2] Add liquidity resilience formula tests in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-features/tests/resilience.rs`
- [x] T037 [P] [US2] Add tradeability classifier tests in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-features/tests/tradeability.rs`
- [x] T038 [P] [US2] Add screen preset tests for resilience filters in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-screen/tests/microstructure_presets.rs`

### Implementation for User Story 2

- [x] T039 [US2] Implement liquidity resilience state machine in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-features/src/resilience.rs`
- [x] T040 [US2] Implement spread shock and recovery metrics in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-features/src/resilience.rs`
- [x] T041 [US2] Implement tradeability classifier in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-features/src/tradeability.rs`
- [x] T042 [US2] Add BBO OFI proxy and signed flow fields to feature rows in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-screen/src/row.rs`
- [x] T043 [US2] Add resilience and tradeability presets in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-screen/src/presets.rs`
- [x] T044 [US2] Render resilience columns or detail lines in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-tui/src/app.rs`
- [x] T045 [US2] Document BBO-only metric caveats in `/Users/s1kor/dev/trading/rsibot/hlscreen/docs/feature-definitions.md`

**Checkpoint**: User Story 2 complete when fixture rows classify shock/recovery/thin states with expected labels and no full-book claims.

---

## Phase 5: User Story 3 - Explain Why a Symbol Ranked (Priority: P2)

**Goal**: Store and render named score components for each ranked row.

**Independent Test**: Run fixed score fixtures and verify why-ranked output matches expected components and confidence penalties.

### Tests for User Story 3

- [ ] T046 [P] [US3] Add score component fixture in `/Users/s1kor/dev/trading/rsibot/hlscreen/tests/fixtures/microstructure/explainable_scores.json`
- [ ] T047 [P] [US3] Add score aggregation tests in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-core/tests/score_breakdown.rs`
- [ ] T048 [P] [US3] Add why-ranked TUI golden test in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-tui/tests/why_ranked_pane.rs`
- [ ] T049 [P] [US3] Add `hls explain` CLI test in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-cli/tests/explain_command.rs`

### Implementation for User Story 3

- [ ] T050 [US3] Generate score breakdowns in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-features/src/engine.rs`
- [ ] T051 [US3] Add score component fields to screen rows in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-screen/src/row.rs`
- [ ] T052 [US3] Add score component filtering and sorting support in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-screen/src/dsl/evaluator.rs`
- [ ] T053 [US3] Implement why-ranked rendering in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-tui/src/detail.rs`
- [ ] T054 [US3] Export detail module from `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-tui/src/lib.rs`
- [ ] T055 [US3] Implement `hls explain` command in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-cli/src/commands/explain.rs`
- [ ] T056 [US3] Register explain command in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-cli/src/main.rs`
- [ ] T057 [US3] Document score components and caveats in `/Users/s1kor/dev/trading/rsibot/hlscreen/docs/microstructure.md`

**Checkpoint**: User Story 3 complete when top rows have deterministic score breakdowns in CLI, TUI, and replay.

---

## Phase 6: User Story 4 - Discover Hyperliquid-Native Listing and Token Events (Priority: P2)

**Goal**: Enrich screen rows with public Hyperliquid metadata while tolerating missing or partial fields.

**Independent Test**: Use metadata fixtures with complete, partial, and missing values; verify row enrichment and presets.

### Tests for User Story 4

- [ ] T058 [P] [US4] Add metadata enrichment fixtures in `/Users/s1kor/dev/trading/rsibot/hlscreen/tests/fixtures/microstructure/metadata_enrichment.json`
- [ ] T059 [P] [US4] Add public metadata adapter tests in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-hyperliquid/tests/metadata_enrichment.rs`
- [ ] T060 [P] [US4] Add metadata model tests in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-core/tests/metadata_enrichment.rs`
- [ ] T061 [P] [US4] Add metadata preset tests in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-screen/tests/metadata_presets.rs`

### Implementation for User Story 4

- [ ] T062 [US4] Implement metadata enrichment model in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-core/src/metadata.rs`
- [ ] T063 [US4] Add public metadata fetch/cache adapter in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-hyperliquid/src/rest.rs`
- [ ] T064 [US4] Persist metadata cache freshness in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-store/src/metadata.rs`
- [ ] T065 [US4] Add metadata fields to screen rows in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-screen/src/row.rs`
- [ ] T066 [US4] Add new-listing and metadata cohort presets in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-screen/src/presets.rs`
- [ ] T067 [US4] Surface metadata tags in terminal rows or details in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-tui/src/app.rs`
- [ ] T068 [US4] Document metadata source stability and unknown-field behavior in `/Users/s1kor/dev/trading/rsibot/hlscreen/docs/microstructure.md`

**Checkpoint**: User Story 4 complete when metadata fields enrich rows without introducing private/account data or live-ingestion failures.

---

## Phase 7: User Story 5 - Operate, Package, and Extend the Workstation as OSS (Priority: P3)

**Goal**: Add benchmark, observability, packaging, and read-only extension contracts that make the project professional and contributor-friendly.

**Independent Test**: Run benchmark packs, metrics contract tests, packaging dry-run checks, and extension contract tests from a clean checkout.

### Tests for User Story 5

- [ ] T069 [P] [US5] Add benchmark command tests in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-cli/tests/bench_command.rs`
- [ ] T070 [P] [US5] Add metrics output tests in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-cli/tests/metrics_output.rs`
- [ ] T071 [P] [US5] Add extension contract tests in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-core/tests/extension_contract.rs`
- [ ] T072 [P] [US5] Add release packaging check script tests in `/Users/s1kor/dev/trading/rsibot/hlscreen/tests/integration/release_packaging.rs`

### Implementation for User Story 5

- [ ] T073 [US5] Implement benchmark pack runner in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-store/src/benchmark.rs`
- [ ] T074 [US5] Implement `hls bench` command in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-cli/src/commands/bench.rs`
- [ ] T075 [US5] Register bench command in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-cli/src/main.rs`
- [ ] T076 [US5] Implement metrics snapshot output helpers in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-core/src/metrics.rs`
- [ ] T077 [US5] Add metrics output to `hls doctor --live --json` in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-cli/src/commands/doctor.rs`
- [ ] T078 [US5] Implement read-only extension contract models in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-core/src/extension.rs`
- [ ] T079 [US5] Add release packaging configuration draft in `/Users/s1kor/dev/trading/rsibot/hlscreen/dist-workspace.toml`
- [ ] T080 [US5] Add release packaging workflow draft in `/Users/s1kor/dev/trading/rsibot/hlscreen/.github/workflows/release.yml`
- [ ] T081 [US5] Update release documentation in `/Users/s1kor/dev/trading/rsibot/hlscreen/docs/RELEASING.md`
- [ ] T082 [US5] Add plugin/extension documentation in `/Users/s1kor/dev/trading/rsibot/hlscreen/docs/extensions.md`

**Checkpoint**: User Story 5 complete when benchmark, metrics, release, and extension contracts can be validated without secrets.

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Documentation, validation, screenshots, reports, and repo continuity.

- [ ] T083 [P] Update README roadmap and status in `/Users/s1kor/dev/trading/rsibot/hlscreen/README.md`
- [ ] T084 [P] Update architecture documentation in `/Users/s1kor/dev/trading/rsibot/hlscreen/docs/architecture.md`
- [ ] T085 [P] Update data format documentation in `/Users/s1kor/dev/trading/rsibot/hlscreen/docs/data-format.md`
- [x] T086 [P] Update screenshot generator for new confidence/resilience output in `/Users/s1kor/dev/trading/rsibot/hlscreen/scripts/generate-screenshots.py`
- [ ] T087 [P] Add dated implementation report in `/Users/s1kor/dev/trading/rsibot/hlscreen/docs/reports/2026-07-08-microstructure-workstation.md`
- [ ] T088 Run full validation gate and record results in `/Users/s1kor/dev/trading/rsibot/hlscreen/PLAN.md`
- [ ] T089 Update durable repo memory in `/Users/s1kor/dev/trading/rsibot/hlscreen/MEMORY.md`
- [ ] T090 Update daily memory and lesson stores in `/Users/s1kor/dev/trading/rsibot/hlscreen/docs/agent-memory/agent_lessons.jsonl`
- [ ] T091 Review for read-only boundary regressions in `/Users/s1kor/dev/trading/rsibot/hlscreen/docs/THREAT_MODEL.md`
- [ ] T092 Close local planning notes in `/Users/s1kor/dev/trading/rsibot/hlscreen/TODO.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies.
- **Foundational (Phase 2)**: depends on Setup; blocks all user stories.
- **US1 Confidence/Replay (Phase 3)**: depends on Foundation; MVP target.
- **US2 Liquidity Resilience (Phase 4)**: depends on Foundation and can use US1 confidence once available.
- **US3 Why-Ranked (Phase 5)**: depends on Foundation and benefits from US1/US2 fields.
- **US4 Metadata Enrichment (Phase 6)**: depends on Foundation; can run in parallel with US2 after shared row contracts are stable.
- **US5 OSS Operations (Phase 7)**: depends on Foundation; benchmark tasks depend on US1 for confidence parity.
- **Polish (Phase 8)**: after selected stories are complete.

### User Story Dependencies

- **US1 (P1)**: first MVP story; confidence is required before ranking can be trusted.
- **US2 (P1)**: can start after Foundation, but final ranking must consume US1 confidence.
- **US3 (P2)**: can start after Foundation, but final explanations should include US1 confidence and US2 resilience where implemented.
- **US4 (P2)**: can start after Foundation using metadata fixtures; final presets integrate with screen rows.
- **US5 (P3)**: metrics and extension contracts can start after Foundation; packaging should wait until user-facing commands are stable.

### Parallel Opportunities

- T001 through T004 can run in parallel after the feature directory exists.
- T008 through T012 can run in parallel because they touch separate test files.
- T020 through T025 can run in parallel for US1 test scaffolding.
- T034 through T038 can run in parallel for US2 test scaffolding.
- T046 through T049 can run in parallel for US3 tests.
- T058 through T061 can run in parallel for US4 metadata tests.
- T069 through T072 can run in parallel for US5 operations tests.
- T083 through T087 can run in parallel once behavior stabilizes.

## Parallel Examples

### User Story 1

```text
Task: T020 Add reconnect-gap fixture in tests/fixtures/microstructure/gap_replay.ndjson
Task: T023 Add replay parity tests in crates/hls-store/tests/replay_parity.rs
Task: T025 Add TUI confidence rendering golden test in crates/hls-tui/tests/confidence_pane.rs
```

### User Story 2

```text
Task: T036 Add liquidity resilience formula tests in crates/hls-features/tests/resilience.rs
Task: T037 Add tradeability classifier tests in crates/hls-features/tests/tradeability.rs
Task: T038 Add screen preset tests in crates/hls-screen/tests/microstructure_presets.rs
```

### User Story 5

```text
Task: T069 Add benchmark command tests in crates/hls-cli/tests/bench_command.rs
Task: T070 Add metrics output tests in crates/hls-cli/tests/metrics_output.rs
Task: T071 Add extension contract tests in crates/hls-core/tests/extension_contract.rs
```

## Implementation Strategy

### MVP First

1. Complete Phase 1 and Phase 2.
2. Complete US1 confidence/replay parity.
3. Stop and validate that every ranked row can show confidence and replay can detect drift.

### Incremental Delivery

1. Add US2 liquidity resilience once confidence exists.
2. Add US3 why-ranked explanations over confidence plus resilience.
3. Add US4 metadata enrichment when row contracts are stable.
4. Add US5 benchmark, metrics, packaging, and extension contracts after core behavior is reliable.

### Review Gates

- Do not add wallet, signing, private stream, order, or execution surfaces.
- Do not claim BBO metrics are full order-book metrics.
- Do not let low-confidence rows appear fully trusted.
- Do not add high-cardinality metric labels.
- Do not add plugin runtime capabilities before the read-only extension contract is enforced.
- Do not update expected benchmark outputs without explicit review evidence.
