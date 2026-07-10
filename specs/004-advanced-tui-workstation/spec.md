# Feature Specification: Advanced TUI Workstation

**Feature Branch**: `004-advanced-tui-workstation`

**Created**: 2026-07-08

**Status**: Draft

**Input**: Build a full next-generation terminal workstation with adaptive Ratatui layout, command palette, editable filters, preset switching, health/recording panels, richer keyboard flow, and optional mouse support.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Operate A Full-Screen Live Cockpit (Priority: P1)

An operator can run `hls live --tui` as a resize-aware full-screen workstation that preserves live ingestion and recording while the display updates.

**Why this priority**: The current string renderer and basic keyboard state are not enough for the requested hedge-fund-style workstation.

**Independent Test**: Run fixture and bounded public live sessions in TUI mode, resize the viewport, and verify watchlist, detail, chart, book, tape, health, and read-only status remain visible without line wrapping.

**Acceptance Scenarios**:

1. **Given** a live TTY session, **When** the terminal is wide, medium, or narrow, **Then** the layout adapts without corrupting market rows or hiding safety state.
2. **Given** display pause is toggled, **When** ingestion continues, **Then** recording and WebSocket counters continue while rendering pauses.

---

### User Story 2 - Edit Filters And Presets In The TUI (Priority: P1)

An operator can open a command palette or filter editor to change screen rules, sort order, presets, and timeframe without restarting live ingestion.

**Why this priority**: Real workstation use requires rapid iteration over live public data without command restarts.

**Independent Test**: In fixture-backed TUI tests, enter valid and invalid filters, switch presets, and verify rows update or errors render without replacing the last valid view.

**Acceptance Scenarios**:

1. **Given** a valid filter expression, **When** the user applies it in the TUI, **Then** the visible watchlist updates and the active filter is shown.
2. **Given** an invalid filter expression, **When** the user submits it, **Then** the previous valid screen remains active and the error is visible.

---

### User Story 3 - Navigate Panels Efficiently (Priority: P2)

An operator can move focus between watchlist, detail, chart, book, tape, alerts, and health panels with keyboard shortcuts and optional mouse support.

**Why this priority**: Dense trading workstations must be navigable without losing context.

**Independent Test**: Snapshot tests verify focused panel state, mouse/focus events where supported, and help overlay consistency.

**Acceptance Scenarios**:

1. **Given** multiple panes are visible, **When** the user changes focus, **Then** keyboard actions apply to the focused pane only.
2. **Given** mouse support is unavailable, **When** the app starts, **Then** all functionality remains accessible by keyboard.

### Edge Cases

- Terminal reports a width larger than the visible Codex/app pane.
- A filter changes while all-symbol mode is still receiving events.
- The command palette is open during reconnect or shutdown.
- `NO_COLOR` or `TERM=dumb` disables color and advanced control sequences.
- Mouse events are unsupported by the terminal.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The TUI MUST be full-screen, resize-aware, and usable at narrow, medium, and wide terminal sizes.
- **FR-002**: The TUI MUST keep live ingestion and recording independent from rendering, display pause, and command editing.
- **FR-003**: The TUI MUST show read-only safety state, connection health, recorder state, active filter/preset/sort, and selected-symbol detail.
- **FR-004**: The TUI MUST provide keyboard-accessible filter editing, preset switching, sort switching, and timeframe selection.
- **FR-005**: Invalid in-TUI commands MUST fail without replacing the last valid screen result.
- **FR-006**: Optional mouse support MUST never be the only path to a command.
- **FR-007**: Fixture/non-TTY screenshot paths MUST remain deterministic.
- **FR-008**: The TUI MUST use real `FeatureSnapshot`, candle, health, and recorder fields only.

### Key Entities *(include if feature involves data)*

- **TUI Session State**: Focused pane, selected row, active view, density, command mode, active filter, preset, sort, and pause state.
- **Command Palette Entry**: User-entered filter, preset, sort, timeframe, or action with validation status.
- **Panel Layout**: Adaptive arrangement of watchlist, detail, chart, book, tape, health, alerts, and help.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Fixture TUI snapshots pass at 80, 120, and 160 columns without wrapped table corruption.
- **SC-002**: Valid in-TUI filters update visible rows within one refresh interval.
- **SC-003**: Invalid in-TUI filters preserve the previous valid result in 100% of tests.
- **SC-004**: Bounded public live TUI smoke exits cleanly with zero data gaps introduced by rendering.

## Assumptions

- Ratatui and Crossterm remain acceptable dependencies for the advanced TUI.
- Existing deterministic string/SVG screenshot paths remain for docs regression.
- Current branch work on the Ratatui cockpit is an implementation input, not proof that all command-editing requirements are complete.
