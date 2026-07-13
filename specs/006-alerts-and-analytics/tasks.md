# Tasks: Alerts And Analytics

**Input**: Design documents from `specs/006-alerts-and-analytics/`

## Phase 1: Setup

- [x] T001 Define alert, analog, metric, fee, and plugin ownership boundaries in `docs/feature-definitions.md`
- [x] T002 [P] Add deterministic alert and analog fixtures in `tests/fixtures/microstructure/`

## Phase 2: Foundational

- [x] T003 Add alert playbook model in `crates/hls-core/src/alerts.rs`
- [x] T004 Add metric definition validity model in `crates/hls-core/src/metrics.rs`
- [x] T005 Add fee profile model in `crates/hls-core/src/fees.rs`
- [x] T006 Add plugin runtime safety tests in `crates/hls-core/tests/extension_contract.rs`

## Phase 3: User Story 1 - Define Read-Only Alert Playbooks (Priority: P1)

- [x] T007 [P] [US1] Add alert replay tests in `crates/hls-features/tests/alerts.rs`
- [x] T008 [US1] Implement local alert evaluator in `crates/hls-features/src/alerts.rs`
- [x] T009 [US1] Add alert CLI/replay command in `crates/hls-cli/src/commands/alerts.rs`
- [x] T010 [US1] Render alert events in TUI/status surfaces in `crates/hls-tui/src/ratatui_app.rs`
- [x] T025 [US1] Add explicit local JSONL alert history persistence via `--alert-history-file` in `crates/hls-cli/src/commands/alerts.rs`
- [x] T026 [US1] Add local alert history listing via `hls alerts --history-file` in `crates/hls-cli/src/commands/alerts.rs`
- [x] T027 [US1] Render bounded local alert history in Ratatui TUI surfaces when live alert integration exists

## Phase 4: User Story 2 - Search Historical Analogs (Priority: P2)

- [x] T011 [P] [US2] Add analog fixture tests in `crates/hls-store/tests/analog_search.rs`
- [x] T012 [US2] Implement local analog search over replay windows in `crates/hls-store/src/analog.rs`
- [x] T013 [US2] Add analog command in `crates/hls-cli/src/commands/analog.rs`
- [x] T028 [US2] Add schema-versioned local analog index write/read support via `hls analog --write-index` and `--index-file`

## Phase 5: User Story 3 - Extend Metrics And Fee-Aware Tradeability (Priority: P2)

- [x] T014 [P] [US3] Add research metric formula/proxy tests in `crates/hls-features/tests/canonical_metrics.rs`
- [x] T015 [US3] Implement research proxy and unavailable states in `crates/hls-features/src/metrics.rs`
- [ ] T031 [US3] Define and validate a canonical production metric suite with benchmark data, sampling contracts, and error tolerances
- [x] T032 [US3] Add versioned sampling/tolerance contracts and benchmark two directly observed public-trade window metrics while retaining advanced estimators as proxy/unavailable
- [x] T016 [US3] Implement fee-aware tradeability using explicit fee profiles in `crates/hls-features/src/tradeability.rs`
- [x] T017 [US3] Update metric docs in `docs/feature-definitions.md`
- [x] T029 [US3] Add local maker/taker fill-mix fee economics with `taker_fill_ratio_hundredths`
- [x] T030 [US3] Add bounded public-trade signed-flow toxicity proxy metric

## Phase 6: User Story 4 - Execute Read-Only Plugins (Priority: P3)

- [x] T018 [P] [US4] Add safe and unsafe plugin runtime fixtures in `tests/fixtures/microstructure/`
- [x] T019 [US4] Select and review plugin sandbox dependency in `specs/006-alerts-and-analytics/research.md`
- [x] T020 [US4] Implement bounded plugin runtime in `crates/hls-core/src/extension.rs`
- [x] T021 [US4] Add plugin invocation command or TUI integration after safety tests pass
- [ ] T024 [US4] Add explicit live row-annotation plugin execution after defining live runtime ownership and latency limits
- [x] T033 [US4] Define and test bounded plugin worker ownership, overload, timeout, failure, and stale-annotation behavior before live enablement

## Phase 7: Polish

- [x] T022 Run full validation from `quickstart.md`
- [x] T023 Update docs, reports, and memory with actual evidence and residual risks
