# Reflection Entry

## Task
- **ID/Title:** Compact Workstation TUI Mock Alignment
- **Date:** 2026-07-08
- **Scope:** multi-file

## Plan and Risks
- **Planned approach:** Update golden tests first, then replace the verbose board/card renderer with a compact framed table and selected-pair pane that uses existing `FeatureSnapshot` fields.
- **Top failure hypotheses:** Existing CLI tests depend on old `PAIR DETAIL CARDS` markers; the mock includes metrics not available as exact fields; fixed-width box drawing can drift in screenshots if line lengths are inconsistent.
- **Success criteria:** `hls live`/screen output starts with `Hyperliquid Spot Microstructure Workstation`, shows ranked compact rows, renders one selected pair detail pane, preserves read-only/no-order wording, and passes focused plus workspace validation.

## Candidate Attempts
| Candidate | Summary | Outcome | Signals | Why selected / rejected |
|---|---|---|---|---|
| A | Patch labels inside the existing dashboard/card layout. | Rejected | Would not satisfy the requested command shape. | Too small for the visual contract. |
| B | Replace the main render body with compact table plus selected-pair details. | Selected | Matches the mock and can reuse real feature fields. | Best fit for the operator-visible output. |

## Reflection
- **Failure modes observed:** Existing integration tests still asserted old dashboard/card strings (`PAIR DETAIL CARDS`, `UNIVERSE`, `fresh`, and old `READ-ONLY ...` titles). The first renderer pass also left dead helper warnings from the retired card layout.
- **Root cause:** The requested mock changed the user-visible renderer contract from broad dashboard plus every-row cards to compact table plus selected-pair pane, but test coverage spanned CLI live/replay/screen/full-pipeline paths.
- **Fix that resolved it:** Replaced the renderer with a compact box table, passed `ScreenRequest` context into TUI rendering, trimmed dead helpers, updated all affected golden/integration tests, and regenerated screenshots.
- **What improved score/quality:** The table is denser, request-aware, screenshot-stable, and easier to scan; metric names now avoid false precision (`flow30` and `amihud` proxy) while retaining the requested workstation feel.
- **Useful command-level evidence:** `cargo test -p hls-tui --test main_table_golden`; `cargo test -p hls-cli --test live_mock`; `cargo test --workspace --all-features`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `python3 scripts/generate-screenshots.py`; fixture TUI smoke with zero stderr.
- **Branch comparison insight (if multiple attempts):** Not applicable.

## Reusable Lesson
- **Pattern that worked:** Start with visible contract tests, inspect actual CLI output, then update downstream integration tests only after the renderer semantics are stable.
- **Pattern to avoid:** Do not copy mock metric labels when the feature engine does not compute the exact measurement.
- **Where to apply next:** Future TUI polish slices, especially if adding true 1m flow sigma or Amihud-style liquidity impact.

## Decision
- **Final chosen approach:** Compact workstation table with selected-pair details and request-derived filter/mode line.
- **Commit/rollback decision:** Keep; full workspace validation is green and screenshots are regenerated.
- **Next step / follow-up:** Add real 1m signed-flow sigma or true Amihud as a separate feature-engine task before exposing those exact labels.
