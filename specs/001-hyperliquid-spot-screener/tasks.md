# Tasks: Hyperliquid Spot Screener

**Input**: Design documents from `specs/001-hyperliquid-spot-screener/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Tests**: Included because the `rsibot` operating contract requires tests for meaningful behavior and `quickstart.md` defines validation commands.

**Organization**: Tasks are grouped by user story so each story can be implemented and tested independently after the shared foundation.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Initialize the Rust workspace, shared config, and local docs skeleton.

- [x] T001 Create Cargo workspace manifest in `/Users/s1kor/dev/trading/rsibot/hlscreen/Cargo.toml`
- [x] T002 Create crate manifests for `hls-core`, `hls-hyperliquid`, `hls-store`, `hls-features`, `hls-screen`, `hls-tui`, `hls-cli`, and `hls-server` under `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/`
- [x] T003 [P] Create default configuration file in `/Users/s1kor/dev/trading/rsibot/hlscreen/config/example.toml`
- [x] T004 [P] Create architecture and data docs stubs in `/Users/s1kor/dev/trading/rsibot/hlscreen/docs/architecture.md`, `/Users/s1kor/dev/trading/rsibot/hlscreen/docs/data-format.md`, and `/Users/s1kor/dev/trading/rsibot/hlscreen/docs/feature-definitions.md`
- [x] T005 [P] Verify and extend local ignore rules for generated market data and build artifacts in `/Users/s1kor/dev/trading/rsibot/hlscreen/.gitignore`
- [x] T006 Create shared test fixture directories in `/Users/s1kor/dev/trading/rsibot/hlscreen/tests/fixtures/`, `/Users/s1kor/dev/trading/rsibot/hlscreen/tests/integration/`, and `/Users/s1kor/dev/trading/rsibot/hlscreen/tests/golden/`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core types, configuration, errors, CLI shell, and public Hyperliquid metadata lookup required by all user stories.

**Critical**: No user story work can begin until this phase is complete.

- [x] T007 [P] Implement shared error and result types in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-core/src/error.rs`
- [x] T008 [P] Implement configuration models and loader in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-core/src/config.rs`
- [x] T009 [P] Implement symbol metadata types and feed/display mapping in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-core/src/symbol.rs`
- [x] T010 [P] Implement timestamp and duration helpers in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-core/src/time.rs`
- [x] T011 Wire `hls-core` module exports in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-core/src/lib.rs`
- [x] T012 [P] Add unit tests for config parsing and symbol mapping in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-core/tests/config_symbol.rs`
- [x] T013 Implement public REST metadata client for spot metadata and asset contexts in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-hyperliquid/src/rest.rs`
- [x] T014 [P] Add REST metadata fixtures in `/Users/s1kor/dev/trading/rsibot/hlscreen/tests/fixtures/hyperliquid/spot_meta.json` and `/Users/s1kor/dev/trading/rsibot/hlscreen/tests/fixtures/hyperliquid/spot_meta_and_asset_ctxs.json`
- [x] T015 Add REST metadata client tests in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-hyperliquid/tests/rest_metadata.rs`
- [x] T016 Implement top-level CLI command shell in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-cli/src/main.rs`
- [x] T017 Implement `init`, `doctor`, and `symbols` command modules in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-cli/src/commands/`
- [x] T018 Add CLI smoke tests for `init`, `doctor`, and fixture-backed `symbols` in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-cli/tests/basic_commands.rs`

**Checkpoint**: Foundation ready when `cargo test -p hls-core -p hls-hyperliquid -p hls-cli` passes and `hls symbols --top 20` can run against fixtures.

---

## Phase 3: User Story 1 - Watch Live Spot Market Conditions (Priority: P1) MVP

**Goal**: Display a read-only live terminal table for selected spot markets with prices, top-of-book liquidity, returns, volatility, anomaly fields, scores, sorting, and stale-data state.

**Independent Test**: Start a mock live session for selected symbols and verify rows update, sorting works, stale state is shown, and no wallet/trading prompt exists.

### Tests for User Story 1

- [x] T019 [P] [US1] Add WebSocket parser fixture tests for trades, BBO, all-mids, active asset context, and candles in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-hyperliquid/tests/ws_parser.rs`
- [x] T020 [P] [US1] Add feature formula tests for spread, top-of-book depth, imbalance, returns, realized volatility, z-scores, and bounded scores in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-features/tests/formulas.rs`
- [x] T021 [P] [US1] Add terminal table golden test for fixed feature rows in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-tui/tests/main_table_golden.rs`
- [x] T022 [P] [US1] Add mock live integration test for one-symbol and multi-symbol updates in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-cli/tests/live_mock.rs`

### Implementation for User Story 1

- [x] T023 [US1] Implement WebSocket message types and envelope parsing in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-hyperliquid/src/ws/types.rs`
- [x] T024 [US1] Implement channel-specific WebSocket parser in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-hyperliquid/src/ws/parser.rs`
- [x] T025 [US1] Implement subscription manager and subscription budget checks in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-hyperliquid/src/ws/subscriptions.rs`
- [x] T026 [US1] Implement live market state container in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-core/src/market_state.rs`
- [x] T027 [US1] Implement rolling windows in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-features/src/windows.rs`
- [x] T028 [US1] Implement feature formulas in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-features/src/formulas.rs`
- [x] T029 [US1] Implement feature engine snapshot updates in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-features/src/engine.rs`
- [x] T030 [US1] Implement TUI main table, details pane, sorting, stale markers, and read-only banner in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-tui/src/app.rs`
- [x] T031 [US1] Implement `hls live` command orchestration in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-cli/src/commands/live.rs`
- [x] T032 [US1] Wire live command modules into `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-cli/src/main.rs`

**Checkpoint**: User Story 1 complete when mock live tests pass and the live TUI shows updating read-only rows from fixtures.

---

## Phase 4: User Story 2 - Record and Replay Market Data Locally (Priority: P1)

**Goal**: Record raw public messages and normalized events locally, then replay recorded intervals to rebuild feature snapshots without live network access.

**Independent Test**: Run a short mock recording, verify raw and normalized files plus metadata registry, then replay the same interval and compare feature snapshots.

### Tests for User Story 2

- [x] T033 [P] [US2] Add raw writer rotation and flush tests in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-store/tests/raw_writer.rs`
- [x] T034 [P] [US2] Add normalized event writer tests in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-store/tests/normalized_writer.rs`
- [x] T035 [P] [US2] Add SQLite metadata registry tests in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-store/tests/metadata_registry.rs`
- [x] T036 [P] [US2] Add replay equivalence integration test in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-cli/tests/record_replay.rs`

### Implementation for User Story 2

- [x] T037 [US2] Implement raw market message model and writer in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-store/src/raw.rs`
- [x] T038 [US2] Implement normalized event file writers in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-store/src/normalized.rs`
- [x] T039 [US2] Implement SQLite metadata registry for symbols, files, runs, and data gaps in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-store/src/metadata.rs`
- [x] T040 [US2] Implement data gap model and state propagation in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-core/src/data_gap.rs`
- [x] T041 [US2] Implement recorder task orchestration with bounded channels and clean shutdown in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-store/src/recorder.rs`
- [x] T042 [US2] Implement replay reader over raw and normalized files in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-store/src/replay.rs`
- [x] T043 [US2] Implement `hls record` command in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-cli/src/commands/record.rs`
- [x] T044 [US2] Implement `hls replay` command in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-cli/src/commands/replay.rs`
- [x] T045 [US2] Integrate optional recording flags into `hls live` in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-cli/src/commands/live.rs`

**Checkpoint**: User Story 2 complete when a fixture recording writes raw/normalized files, metadata is committed, and replay rebuilds expected screen rows.

---

## Phase 5: User Story 3 - Screen Markets with Rules and Presets (Priority: P2)

**Goal**: Filter and sort market rows using built-in presets and a small safe rule language.

**Independent Test**: Evaluate presets and custom rules over fixed feature rows and verify include/exclude behavior, sorting, and validation errors.

### Tests for User Story 3

- [ ] T046 [P] [US3] Add DSL parser tests for boolean logic, comparisons, literals, and `abs(field)` in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-screen/tests/dsl_parser.rs`
- [ ] T047 [P] [US3] Add DSL evaluator tests for fixed feature rows in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-screen/tests/dsl_evaluator.rs`
- [ ] T048 [P] [US3] Add preset golden tests for liquid momentum, volume anomaly, tight-spread movers, mean-reversion watch, and thin books in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-screen/tests/presets.rs`

### Implementation for User Story 3

- [ ] T049 [US3] Implement screen row and sort models in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-screen/src/row.rs`
- [ ] T050 [US3] Implement DSL tokenizer and parser in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-screen/src/dsl/parser.rs`
- [ ] T051 [US3] Implement DSL evaluator and type validation in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-screen/src/dsl/evaluator.rs`
- [ ] T052 [US3] Implement built-in presets in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-screen/src/presets.rs`
- [ ] T053 [US3] Implement filtering and sorting engine in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-screen/src/engine.rs`
- [ ] T054 [US3] Implement `hls screen` command in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-cli/src/commands/screen.rs`
- [ ] T055 [US3] Integrate preset selection and filter editing into the TUI in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-tui/src/app.rs`

**Checkpoint**: User Story 3 complete when presets and custom rules work over fixture rows and invalid rules do not replace the active screen.

---

## Phase 6: User Story 4 - Monitor Data Health and Safety Boundaries (Priority: P3)

**Goal**: Show connection, subscription, latency, lag, storage, reconnect, gap, and read-only safety status, with optional localhost JSON endpoints.

**Independent Test**: Simulate healthy, stale, reconnecting, writer-lag, and interrupted states and verify health output and main-screen safety behavior.

### Tests for User Story 4

- [ ] T056 [P] [US4] Add heartbeat and reconnect tests with a mock WebSocket server in `/Users/s1kor/dev/trading/rsibot/hlscreen/tests/integration/reconnect_heartbeat.rs`
- [ ] T057 [P] [US4] Add health state unit tests in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-core/tests/health_state.rs`
- [ ] T058 [P] [US4] Add optional local API contract tests in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-server/tests/read_only_api.rs`

### Implementation for User Story 4

- [ ] T059 [US4] Implement health state and telemetry models in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-core/src/health.rs`
- [ ] T060 [US4] Implement heartbeat, ping/pong handling, reconnect backoff, and resubscribe flow in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-hyperliquid/src/ws/connection.rs`
- [ ] T061 [US4] Implement latency and lag measurement in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-core/src/telemetry.rs`
- [ ] T062 [US4] Implement TUI health pane in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-tui/src/health.rs`
- [ ] T063 [US4] Extend `hls doctor --live` with read-only live checks in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-cli/src/commands/doctor.rs`
- [ ] T064 [US4] Implement optional localhost read-only API in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-server/src/lib.rs`
- [ ] T065 [US4] Add API command/config wiring in `/Users/s1kor/dev/trading/rsibot/hlscreen/crates/hls-cli/src/commands/server.rs`

**Checkpoint**: User Story 4 complete when degraded states are visible within the required window and no health/API surface exposes trading actions.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Documentation, validation, performance checks, and repo continuity.

- [x] T066 [P] Document architecture decisions in `/Users/s1kor/dev/trading/rsibot/hlscreen/docs/architecture.md`
- [x] T067 [P] Document raw and normalized data formats in `/Users/s1kor/dev/trading/rsibot/hlscreen/docs/data-format.md`
- [x] T068 [P] Document feature formulas and score interpretation in `/Users/s1kor/dev/trading/rsibot/hlscreen/docs/feature-definitions.md`
- [x] T069 [P] Create user README with read-only positioning in `/Users/s1kor/dev/trading/rsibot/hlscreen/README.md`
- [x] T070 Run formatting check with `cargo fmt --check` from `/Users/s1kor/dev/trading/rsibot/hlscreen`
- [x] T071 Run lint check with `cargo clippy --workspace --all-targets -- -D warnings` from `/Users/s1kor/dev/trading/rsibot/hlscreen`
- [x] T072 Run full test suite with `cargo test --workspace` from `/Users/s1kor/dev/trading/rsibot/hlscreen`
- [ ] T073 Run quickstart validation commands from `/Users/s1kor/dev/trading/rsibot/hlscreen/specs/001-hyperliquid-spot-screener/quickstart.md`
- [x] T074 Update durable project memory in `/Users/s1kor/dev/trading/rsibot/hlscreen/MEMORY.md`
- [x] T075 Close local planning notes in `/Users/s1kor/dev/trading/rsibot/hlscreen/PLAN.md` and `/Users/s1kor/dev/trading/rsibot/hlscreen/TODO.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies.
- **Foundational (Phase 2)**: depends on Setup; blocks all user stories.
- **US1 Live Screener (Phase 3)**: depends on Foundation; MVP target.
- **US2 Recording/Replay (Phase 4)**: depends on Foundation and can integrate with US1 live events.
- **US3 Screening Rules (Phase 5)**: depends on Foundation and feature snapshot rows from US1.
- **US4 Health/Safety (Phase 6)**: depends on Foundation and integrates with live/record/replay paths.
- **Polish (Phase 7)**: after selected user stories are complete.

### User Story Dependencies

- **US1 (P1)**: first MVP story after Foundation.
- **US2 (P1)**: can start after Foundation, but final live-record integration depends on US1 live command wiring.
- **US3 (P2)**: can start after Foundation using fixture rows, then integrate with US1 snapshots.
- **US4 (P3)**: can start after Foundation using simulated state, then integrate across US1 and US2.

### Parallel Opportunities

- T003, T004, T005 can run in parallel after T001/T002.
- T007 through T010 and T014 can run in parallel after crate skeletons exist.
- US1 parser, feature, TUI golden, and mock integration tests can be written in parallel.
- US2 raw writer, normalized writer, metadata, and replay tests can be written in parallel.
- US3 parser, evaluator, and preset tests can be written in parallel.
- US4 reconnect, health state, and API tests can be written in parallel.
- Documentation tasks T066 through T069 can run in parallel once behavior stabilizes.

## Parallel Examples

### User Story 1

```text
Task: T019 Add WebSocket parser fixture tests in crates/hls-hyperliquid/tests/ws_parser.rs
Task: T020 Add feature formula tests in crates/hls-features/tests/formulas.rs
Task: T021 Add terminal table golden test in crates/hls-tui/tests/main_table_golden.rs
```

### User Story 2

```text
Task: T033 Add raw writer rotation and flush tests in crates/hls-store/tests/raw_writer.rs
Task: T034 Add normalized event writer tests in crates/hls-store/tests/normalized_writer.rs
Task: T035 Add SQLite metadata registry tests in crates/hls-store/tests/metadata_registry.rs
```

### User Story 3

```text
Task: T046 Add DSL parser tests in crates/hls-screen/tests/dsl_parser.rs
Task: T047 Add DSL evaluator tests in crates/hls-screen/tests/dsl_evaluator.rs
Task: T048 Add preset golden tests in crates/hls-screen/tests/presets.rs
```

## Implementation Strategy

### MVP First

1. Complete Setup and Foundation.
2. Complete US1 live screener with mock server and fixture validation.
3. Stop and validate terminal usability, stale markers, read-only boundary, and feature correctness.

### Incremental Delivery

1. Add US2 recording/replay after the live path is stable.
2. Add US3 DSL/presets once feature rows are stable.
3. Add US4 health/API after the state model is shared across live and recording paths.

### Review Gates

- Do not add wallet, private-key, trading, or exchange action code.
- Do not name TOB metrics as full book depth.
- Do not treat scores as trade signals or predictions.
- Do not claim replay equivalence until fixture and recorded interval tests pass.
