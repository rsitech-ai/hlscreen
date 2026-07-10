# Tasks: Advanced TUI Workstation

**Input**: Design documents from `specs/004-advanced-tui-workstation/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [quickstart.md](quickstart.md)

**Tests**: Required for terminal layout, keyboard flow, command validation, and live smoke safety.

## Phase 1: Setup

- [x] T001 Confirm active Ratatui branch diff and preserve existing non-TTY deterministic renderer in `crates/hls-tui/src/app.rs`
- [x] T002 [P] Add viewport fixture helpers for 80/120/160 columns in `crates/hls-tui/tests/ratatui_cockpit.rs`

## Phase 2: Foundational

- [x] T003 Extend `WorkstationUiState` with focused pane and command palette mode in `crates/hls-tui/src/interaction.rs`
- [x] T004 Add command input validation model in `crates/hls-tui/src/interaction.rs`
- [x] T005 Wire screen DSL/preset/sort validation without duplicating parser logic in `crates/hls-cli/src/commands/live.rs`

## Phase 3: User Story 1 - Operate A Full-Screen Live Cockpit (Priority: P1)

- [x] T006 [P] [US1] Add Ratatui viewport regression tests in `crates/hls-tui/tests/ratatui_cockpit.rs`
- [x] T007 [US1] Render adaptive watchlist/detail/chart/book/tape/status panes in `crates/hls-tui/src/ratatui_app.rs`
- [x] T008 [US1] Verify display pause does not pause ingestion in `crates/hls-cli/src/commands/live.rs`

## Phase 4: User Story 2 - Edit Filters And Presets In The TUI (Priority: P1)

- [x] T009 [P] [US2] Add command palette tests for valid and invalid filters in `crates/hls-tui/tests/ratatui_cockpit.rs`
- [x] T010 [US2] Implement editable filter entry and validation in `crates/hls-tui/src/interaction.rs`
- [x] T011 [US2] Apply valid filter/preset/sort changes to live rows in `crates/hls-cli/src/commands/live.rs`
- [x] T012 [US2] Render validation errors without replacing the last valid screen in `crates/hls-tui/src/ratatui_app.rs`

## Phase 5: User Story 3 - Navigate Panels Efficiently (Priority: P2)

- [x] T013 [P] [US3] Add focused-panel keyboard tests in `crates/hls-tui/tests/interactive_tui.rs`
- [x] T014 [US3] Implement pane focus cycling and contextual help in `crates/hls-tui/src/interaction.rs`
- [x] T015 [US3] Add optional mouse event handling guarded by keyboard parity in `crates/hls-cli/src/commands/live.rs`

## Phase 6: Polish

- [x] T016 Regenerate screenshots with `python3 scripts/generate-screenshots.py`
- [x] T017 Run full validation from `quickstart.md`
- [x] T018 Record live smoke evidence in `docs/reports/`
- [x] T019 Persist local TUI display preferences in `crates/hls-tui/src/interaction.rs` and `crates/hls-cli/src/commands/live.rs`
