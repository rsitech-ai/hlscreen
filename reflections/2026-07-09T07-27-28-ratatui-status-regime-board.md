# Reflection Entry: Ratatui Status Regime Board

## Task
- **ID/Title:** Ratatui status regime board
- **Date:** 2026-07-09
- **Scope:** focused TUI renderer/test slice

## Plan and Risks
- **Planned approach:** Add one behavior-tested adaptive status-panel deck that summarizes market regime, breadth, net flow, depth, confidence, and read-only safety from existing public rows.
- **Top failure hypotheses:** The status drilldown is height-constrained on narrow terminals; adding too many lines can clip existing safety/color/key diagnostics; wide right panes are already narrow, so this should avoid the book/tape layout.
- **Success criteria:** Existing Ratatui cockpit tests keep passing, a new test proves the regime board renders in focused status mode, and the full workspace remains formatted, clippy-clean, and tested.

## Candidate Attempts
| Candidate | Summary | Outcome | Signals | Why selected / rejected |
|---|---|---|---|---|
| A | Add another header line | Rejected | Header already uses fixed 7/6/5 row budgets | Too likely to starve body layout and repeat clipping issues. |
| B | Add focused status-panel regime board | Selected | Status pane is explicit ops surface and available through keyboard focus | Improves workstation feel without destabilizing normal watchlist/chart/book/tape views. |

## Reflection
- **Failure modes observed:** The first implementation rendered Status only in the narrow right rail on wide layouts, so it used compact labels. Adding a new line also pushed existing safety/key text out of the narrow status drilldown.
- **Root cause:** The wide layout let keyboard focus select Status without allocating a real Status workspace, and the narrow drilldown had no spare vertical budget.
- **Fix that resolved it:** On wide/medium screens, Status focus now replaces the non-watchlist workspace with the status console; the regime board renders only when the status area is wide enough.
- **What improved score/quality:** The focused Status pane now behaves like a real workspace pane and adds breadth, regime, heat, net flow, depth, confidence, and read-only portfolio-scan context without weakening compact safety labels.
- **Useful command-level evidence:** `cargo test -p hls-tui --test ratatui_cockpit`; `cargo test -p hls-tui --test interactive_tui`; `cargo build -p hls-cli`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `COLUMNS=320 LINES=52 ./target/debug/hls live --top 10 --duration-secs 5 --refresh-secs 2 --tui`.

## Reusable Lesson
- **Pattern that worked:** Treat keyboard-focused hidden panes as workspace swaps on larger layouts, then keep narrow layouts conservative and safety-first.
- **Pattern to avoid:** Adding global header/status lines without checking compact terminal budgets.
- **Where to apply next:** Future pane-specific dashboards such as latency, replay parity, or plugin health should use focus-driven workspace swaps rather than squeezing the always-on board.

## Decision
- **Final chosen approach:** Behavior-tested status panel regime board.
- **Commit/rollback decision:** Commit after validation.
- **Next step / follow-up:** Continue toward richer keyboard-driven operations dashboards and visual market-state surfaces.
