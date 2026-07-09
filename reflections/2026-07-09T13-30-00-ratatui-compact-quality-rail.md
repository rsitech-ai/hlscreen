# Reflection Entry

## Task
- **ID/Title:** Ratatui compact quality rail
- **Date:** 2026-07-09
- **Scope:** multi-file

## Plan and Risks
- **Planned approach:** Add one behavior test for a 72-column focused pane where compact status must keep health, quality alert, read-only boundary, and color semantics visible; then update the compact status renderer.
- **Top failure hypotheses:** The compact line may wrap too aggressively; color spans may be lost if the compact renderer keeps returning a plain string; adding quality text may hide contextual pane actions.
- **Success criteria:** Narrow snapshots show live health plus QALERT/read-only state outside watchlist focus, color mode styles the alert semantically, and existing wide status behavior remains unchanged.

## Candidate Attempts
| Candidate | Summary | Outcome | Signals | Why selected / rejected |
|---|---|---|---|---|
| A | Add another wide status rail | Rejected | Wide mode already carries quality/pipeline detail | Does not address resize/adaptive complaint |
| B | Upgrade compact status line with semantic spans | Selected | Directly targets 72-column and sub-90-column paths | Smallest slice that improves real terminal behavior |

## Reflection
- **Failure modes observed:** The first compact span rail still clipped `RO no-wallet` at 72 columns, and adding health to the normal chart-focus line clipped `/ command`.
- **Root cause:** The compact status bar has one content row below the top border; health, mode, quality, action hints, and safety cannot all fit at 72 columns.
- **Fix that resolved it:** Split compact status into normal mode and alert mode. Normal non-watchlist focus preserves contextual action hints; alert mode becomes a terse incident rail with health, live state, focus, QALERT/confidence, and read-only safety.
- **What improved score/quality:** Narrow terminals now surface degraded data quality outside watchlist focus with semantic color while keeping the read-only/no-wallet boundary visible.
- **Useful command-level evidence:** `cargo fmt --check`; `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction --test ratatui_cockpit -j 1`; `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`; `cargo test -p hls-cli --test live_mock`; `cargo build -p hls-cli`; fixture `hls live --once --tui --color always`; `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`; `git diff --check`.
- **Branch comparison insight (if multiple attempts):** Not applicable.

## Reusable Lesson
- **Pattern that worked:** Start from the narrow viewport where rich TUI signals disappear.
- **Pattern to avoid:** Adding wide-only cockpit decoration while compact terminals keep losing operational truth.
- **Where to apply next:** Compact chart/book/tape drilldowns and any mode hidden behind adaptive layout thresholds.

## Decision
- **Final chosen approach:** Compact semantic status rail.
- **Commit/rollback decision:** Commit after validation.
- **Next step / follow-up:** Continue tightening compact chart/book/tape drilldowns so resize behavior keeps the same operational truth as the wide cockpit.
