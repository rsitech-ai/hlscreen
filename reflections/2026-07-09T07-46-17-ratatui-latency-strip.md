# Reflection Entry: Ratatui Latency Strip

## Task
- **ID/Title:** Ratatui latency strip
- **Date:** 2026-07-09
- **Scope:** focused TUI renderer/test slice

## Plan and Risks
- **Planned approach:** Add a wide/medium Status pane latency strip that summarizes p95 row age, low-confidence rows, stale rows, reconnects, gaps, and a read-only local-processing caveat from existing public market rows and health counters.
- **Top failure hypotheses:** Extra status lines can clip compact safety labels; row age values may be missing or negative; the UI must not imply exchange/network latency when only local row freshness is being summarized.
- **Success criteria:** A Ratatui cockpit test proves the latency strip, compact status tests stay green, and focused/broad validation plus live smoke pass.

## Candidate Attempts
| Candidate | Summary | Outcome | Signals | Why selected / rejected |
|---|---|---|---|---|
| A | Add the strip to every status panel | Rejected | Narrow status already has no spare vertical budget | Would risk clipping read-only safety and key hints. |
| B | Add the strip only when the Status area is wide enough | Selected | Matches prior regime-board pattern | Improves ops visibility without weakening compact layout. |

## Reflection
- **Failure modes observed:** The first contract test failed on the missing `LATENCY STRIP` text, which confirmed the test was exercising the real wide Status render path. Formatting needed one `cargo fmt` pass after the implementation.
- **Root cause:** The Status focus pane already had regime and quality summaries, but no single row combining row freshness, confidence degradation, stale rows, reconnects, and data gaps.
- **Fix that resolved it:** Added a non-compact Status `LATENCY STRIP` sourced from existing feature snapshots and live status counters, with p95 row age bounded to nonnegative public row freshness values.
- **What improved score/quality:** Operators get a fast live-data health read without moving away from the keyboard-focused Ratatui cockpit, and compact/narrow layouts keep their existing safety/key labels.
- **Useful command-level evidence:** `cargo fmt -p hls-tui --check`; `git diff --check`; `cargo test -p hls-tui --test ratatui_cockpit`; `cargo test -p hls-tui --test interactive_tui`; `cargo build -p hls-cli`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `COLUMNS=320 LINES=52 ./target/debug/hls live --top 10 --duration-secs 5 --refresh-secs 2 --tui`.

## Reusable Lesson
- **Pattern that worked:** Add one visible cockpit contract test first, then hang the implementation off the same non-compact Status branch used by the regime board.
- **Pattern to avoid:** Adding more text to compact Status panes, where vertical budget is already reserved for safety state and control hints.
- **Where to apply next:** Future ops-console additions should use focused pane drilldowns or non-compact mode, not the always-visible compact footer.

## Decision
- **Final chosen approach:** Wide/medium Status-only latency strip, reusing existing public row freshness and live health counters.
- **Commit/rollback decision:** Commit after green focused tests, full workspace tests, clippy, build, and live top-10 smoke.
- **Next step / follow-up:** Continue the Ratatui upgrade with richer focused drilldowns while preserving the single unified `hls live --tui` entrypoint.
