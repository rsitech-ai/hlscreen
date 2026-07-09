# Reflection Entry

## Task
- **ID/Title:** Ratatui medium desk router
- **Date:** 2026-07-09
- **Scope:** multi-file

## Plan and Risks
- **Planned approach:** Add one behavior test proving the 120-160 column medium layout has an explicit lower-pane router for book/tape keyboard focus and public-only safety, then add a one-line adaptive rail above the book/tape split.
- **Top failure hypotheses:** The rail may starve chart height on 120x36; text may wrap or crowd; adding a medium-only line could shift existing assertions unexpectedly.
- **Success criteria:** Medium cockpit renders `ADAPTIVE DESK`, `4 book`, `5 tape`, public BBO/trades-only safety, and `z zoom`, while existing medium chart/book/tape visibility remains green.

## Candidate Attempts
| Candidate | Summary | Outcome | Signals | Why selected / rejected |
|---|---|---|---|---|
| A | Put more text inside compact book/tape content | Rejected | Those panes already use almost every content row at 120x36 | Risks hiding existing BBO/flow lines |
| B | Add a one-line medium router above the lower split | Selected | Keeps book/tape content intact and makes the adaptive layout explicit | Best fit for keyboard-interactive medium mode |

## Reflection
- **Failure modes observed:** The first implementation added the router but stole one row from the 120x36 lower stack, clipping the compact book pane's `ask pressure` evidence.
- **Root cause:** The medium body at 120x36 has exactly 27 rows; adding a one-line router while keeping a 10-row chart minimum over-constrained the vertical split.
- **Fix that resolved it:** Kept book/tape at the existing 9-row allocation and relaxed the medium chart minimum from 10 to 9, preserving compact book/tape content while making room for the router.
- **What improved score/quality:** Medium breakpoints now explicitly expose `4 book`, `5 tape`, public BBO/trades-only safety, and `z zoom`, making the adaptive lower stack keyboard-routable instead of merely visible.
- **Useful command-level evidence:** `cargo fmt --check`; `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction --test ratatui_cockpit -j 1`; `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`; `cargo test -p hls-cli --test live_mock`; `cargo build -p hls-cli`; fixture `hls live --once --tui --color always`; `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`; `git diff --check`.
- **Branch comparison insight (if multiple attempts):** Not applicable.

## Reusable Lesson
- **Pattern that worked:** Treat breakpoint-specific layout transitions as user-visible controls, not hidden renderer decisions.
- **Pattern to avoid:** Stuffing control help into already-dense data panes.
- **Where to apply next:** Medium command palette density and expanded-pane transition hints.

## Decision
- **Final chosen approach:** One-line medium lower desk router.
- **Commit/rollback decision:** Commit after validation.
- **Next step / follow-up:** Continue improving medium and narrow expanded-pane transition hints.
