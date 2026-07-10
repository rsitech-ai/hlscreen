# Implementation Plan: Advanced TUI Workstation

**Branch**: `004-advanced-tui-workstation` | **Date**: 2026-07-08 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/004-advanced-tui-workstation/spec.md`

## Summary

Complete the move from deterministic terminal table to a full Ratatui workstation: adaptive panes, live chart/book/tape/status surfaces, command palette, editable filters, preset/sort/timeframe controls, focus management, and optional mouse support.

## Technical Context

**Language/Version**: Rust stable, edition 2024.

**Primary Dependencies**: Existing `ratatui`, `crossterm`, `tokio`, workspace crates, and deterministic TUI tests.

**Storage**: No new durable storage required except optional persisted user UI preferences if explicitly added later.

**Testing**: Ratatui snapshot tests, interaction state tests, CLI fixture tests, bounded public live smoke, screenshot generation.

**Target Platform**: Local macOS/Linux terminals.

**Project Type**: Rust CLI/TUI application.

**Performance Goals**: Rendering must not block WebSocket ingestion; bounded live smoke must show no render-induced gaps.

**Constraints**: Public data only; deterministic non-TTY output remains; no invented fields; optional mouse only.

**Scale/Scope**: All visible rows/panes for top-N and all-symbol modes. Hosted GUI/web dashboard is out of scope.

## Constitution Check

- **Read-only public data boundary**: PASS. Controls affect display state only.
- **Replayable evidence before ranking**: PASS. TUI consumes existing screen/replay evidence.
- **Live truth over mock convenience**: PASS. Chart/book/tape must use real events or render missing states.
- **Operator safety and observability**: PASS. Health and recorder panels are required.
- **Open-source reproducibility**: PASS. Snapshot tests and screenshots remain required.

## Project Structure

```text
crates/hls-tui/src/ratatui_app.rs
crates/hls-tui/src/interaction.rs
crates/hls-cli/src/commands/live.rs
crates/hls-tui/tests/
crates/hls-cli/tests/
docs/assets/screenshots/
```

## Phase 0: Research Summary

- Keep Ratatui as the primary full-screen terminal UI layer.
- Keep command parsing backed by the existing screen DSL instead of adding a second filter language.
- Preserve old deterministic output for docs and CI where a real TTY is unavailable.

## Phase 1: Design Summary

The TUI runtime is a presentation layer over existing live market state. It maintains UI state, validates commands through existing screen/preset logic, and renders errors without mutating ingestion or recorder behavior.

## Complexity Tracking

No constitution violations are intentionally introduced.
