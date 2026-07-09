# Reflection Entry

## Task
- **ID/Title:** Ratatui medium quality alert
- **Date:** 2026-07-09
- **Scope:** multi-file

## Plan and Risks
- **Planned approach:** Add a behavior test proving 120-column status bars surface degraded data quality with color, then add a medium-width compact alert variant without changing the wide full `QUALITY ALERT` rail or the sub-90 incident rail.
- **Top failure hypotheses:** Alert text may crowd out market ticker/action rails; wide alert behavior could regress; no-color snapshots could accidentally include ANSI escapes.
- **Success criteria:** At 120 columns the status bar shows `QALERT`, degraded confidence, ticker context, action strip, and no-wallet safety; at 240 columns the existing full `QUALITY ALERT` behavior remains intact.

## Candidate Attempts
| Candidate | Summary | Outcome | Signals | Why selected / rejected |
|---|---|---|---|---|
| A | Lower the full `QUALITY ALERT` threshold from 180 to 90 | Rejected | Full alert is verbose and risks crowding the ticker/action rail | Too much text for medium terminals |
| B | Add a compact medium alert before the ticker | Selected | Keeps incident visible while preserving existing medium rails | Matches adaptive behavior goal |

## Reflection
- **Failure modes observed:** The red test confirmed a degraded row at 120 columns did not render any `QALERT` signal even though wide and sub-90 layouts had incident indicators.
- **Root cause:** The full status-bar quality alert was gated to width >= 180, and medium mode only showed the generic `QUALITY` summary plus ticker/action rails.
- **Fix that resolved it:** Inserted compact `QALERT confNN` spans before `MARKET TICKER` for width < 180 while preserving the full `QUALITY ALERT` rail at wide widths.
- **What improved score/quality:** Medium terminals now keep degraded data quality visible with semantic color while still showing ticker context, action strip, and no-wallet safety.
- **Useful command-level evidence:** `cargo fmt --check`; `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction --test ratatui_cockpit -j 1`; `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`; `cargo test -p hls-cli --test live_mock`; `cargo build -p hls-cli`; fixture `hls live --once --tui --color always`; `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`; `git diff --check`.
- **Branch comparison insight (if multiple attempts):** Not applicable.

## Reusable Lesson
- **Pattern that worked:** Treat width bands as first-class UX contracts, not degraded versions of wide mode.
- **Pattern to avoid:** Hiding critical market-data quality signals in the medium breakpoint.
- **Where to apply next:** Medium chart/book/tape focused panes and command palette density.

## Decision
- **Final chosen approach:** Medium compact alert rail.
- **Commit/rollback decision:** Commit after validation.
- **Next step / follow-up:** Continue tightening medium chart/book/tape density so all breakpoints carry the same market and control semantics.
