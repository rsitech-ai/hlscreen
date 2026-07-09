## Task
- **ID/Title:** Ratatui expanded Watchlist command center
- **Date:** 2026-07-09
- **Scope:** multi-file

## Plan and Risks
- **Planned approach:** Add one behavior-first test for a zoomed Watchlist pane that renders an operator-style command center over existing screened rows, then implement a small read-only deck below the current scanner/heatmap lines.
- **Top failure hypotheses:** The deck could crowd the row router, duplicate heatmap content, or imply executable trading controls.
- **Success criteria:** Expanded Watchlist shows keyboard intent, selected symbol context, market breadth, tradeable/degraded counts, flow/depth leaders, and explicit no-wallet/no-orders safety; existing adaptive and live CLI checks stay green.

## Candidate Attempts
| Candidate | Summary | Outcome | Signals | Why selected / rejected |
|---|---|---|---|---|
| A | Extend expanded Watchlist row-router footer | Selected | Uses current pane architecture and keeps normal watchlist layout stable | Best fit for screenshot-like watchlist control surface |
| B | Add portfolio tabs or simulated positions | Rejected | Would imply account/private state | Violates read-only public-data boundary |

## Reflection
- **Failure modes observed:** The first implementation rendered the deck but clipped the final hotkey/safety line because the expanded Watchlist footer was sized for the older scanner/heatmap stack.
- **Root cause:** The expanded row-router area had a fixed height of 8 lines while the new footer needed 11 lines after scanner, heatmap, and command-center content.
- **Fix that resolved it:** Increased only the expanded Watchlist footer allocation and kept non-expanded row-router sizing unchanged.
- **What improved score/quality:** The Watchlist now reads more like an operator command center with selected-row context, breadth, leaders, quality counts, and keyboard intent in one place.
- **Useful command-level evidence:** `cargo test -p hls-tui --test ratatui_cockpit expanded_watchlist_renders_command_center_deck -- --nocapture`; `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction --test ratatui_cockpit -j 1`; `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`; `cargo test -p hls-cli --test live_mock`; `cargo fmt --check`; `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`; fixture `hls live --once --tui` smoke.
- **Branch comparison insight (if multiple attempts):** Not applicable.

## Reusable Lesson
- **Pattern that worked:** Put richer command-center content behind existing expanded-pane mode and adjust layout height only in that expanded path.
- **Pattern to avoid:** Do not add portfolio/position language unless backed by real private/account data and explicit user approval.
- **Where to apply next:** Other expanded panes that need command-center affordances without changing market-data contracts.

## Decision
- **Final chosen approach:** Add read-only Watchlist command center lines in expanded Watchlist mode.
- **Commit/rollback decision:** Commit after focused, full Ratatui, live CLI, clippy, and fixture smoke validation.
- **Next step / follow-up:** Continue moving expanded panes toward a complete workstation and regenerate screenshot proof when the next visual plateau is reached.
