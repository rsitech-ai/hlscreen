## Task
- **ID/Title:** Ratatui immediate resize redraw
- **Date:** 2026-07-09
- **Scope:** single-file

## Plan and Risks
- **Planned approach:** Add a small live-TUI event effect classifier so terminal resize events request a redraw immediately, while keyboard and mouse events keep their existing action behavior.
- **Top failure hypotheses:** Resize handling could accidentally mutate UI state; command-mode keyboard handling could regress; terminal event tests could couple too tightly to Crossterm internals.
- **Success criteria:** `Event::Resize` returns a redraw effect without an action; focused live-TUI tests pass; fixture live TUI smoke still renders the unified Ratatui cockpit; workspace checks stay green.

## Candidate Attempts
| Candidate | Summary | Outcome | Signals | Why selected / rejected |
|---|---|---|---|---|
| A | Wait for the next timed refresh to pick up resized terminal dimensions. | Rejected | Already current behavior. | Not interactive enough for the requested adaptive workstation. |
| B | Treat resize as a redraw-only TUI event. | Selected | Minimal, testable, and preserves ingestion/render boundaries. | Gives immediate adaptive feedback without adding async complexity. |

## Reflection
- **Failure modes observed:** None in implementation; the only delay was cold-build compilation in the clean worktree and a transient Cargo package-cache lock wait during clippy.
- **Root cause:** The previous event loop only treated key and mouse events as redraw-worthy. `Event::Resize` fell through to ignore, so resize awareness depended on the next scheduled refresh or user input.
- **Fix that resolved it:** Added `LiveTuiEventEffect` and `live_tui_event_effect(...)`; resize now returns `Redraw`, key/mouse events still map to workstation actions, and ignored events remain ignored. `apply_pending_tui_actions(...)` now redraws when either actions or resize effects are present.
- **What improved score/quality:** The live Ratatui workstation reacts to terminal resize events immediately without mutating selected row, focused pane, active view, ingestion state, recorder state, or screen filters.
- **Useful command-level evidence:** `cargo test -p hls-cli commands::live::tests::live_tui -- --nocapture`; `cargo fmt --check`; `cargo test -p hls-cli --test live_mock`; `cargo test -p hls-tui --test ratatui_cockpit --test workstation_interaction`; `cargo test --workspace --all-features`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo build --workspace --all-features`; `git diff --check`; fixture TUI smoke confirmed one colored workstation frame with layout/action/theme rails.
- **Branch comparison insight (if multiple attempts):** Not applicable.

## Reusable Lesson
- **Pattern that worked:** Classify terminal events into explicit input effects before applying state mutations.
- **Pattern to avoid:** Hiding resize support behind periodic refresh only.
- **Where to apply next:** Any future mouse/layout events should return explicit input effects instead of being buried in the read loop.

## Decision
- **Final chosen approach:** Redraw-only resize effect in the live TUI event classifier.
- **Commit/rollback decision:** Commit and push after focused tests, full workspace gate, and fixture smoke.
- **Next step / follow-up:** Continue runtime polish toward richer resize-aware pane behavior and visual depth.
