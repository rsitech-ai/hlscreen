# Reflection Entry: Ratatui Theme Rail

## Task
- **ID/Title:** Ratatui theme rail
- **Date:** 2026-07-09
- **Scope:** focused TUI renderer/test slice

## Plan and Risks
- **Planned approach:** Add an always-visible wide/medium cockpit theme rail that reports the active palette and gives the operator an immediate `--color always` recovery hint when a terminal renders plain output.
- **Top failure hypotheses:** The status bar can become too dense and truncate more important safety text; color-specific assertions can accidentally couple tests to unrelated panel styling; the UI must keep `NO_COLOR` and `--color never` behavior deterministic.
- **Success criteria:** A Ratatui cockpit test proves the theme rail appears in plain and color mode, color mode emits visible green/red swatches, no-color mode emits no ANSI escapes, existing compact safety tests stay green, and full validation plus live smoke pass.

## Candidate Attempts
| Candidate | Summary | Outcome | Signals | Why selected / rejected |
|---|---|---|---|---|
| A | Keep the color hint only inside focused Status | Rejected | User notices monochrome on the default cockpit, not after focusing Status | Too hidden for terminal troubleshooting. |
| B | Add a compact theme strip to the wide/medium status bar | Selected | Status bar is always visible and already carries ops/safety strips | Gives immediate feedback without changing market-data behavior. |
| C | Force color by default in all terminals | Rejected | Would violate `NO_COLOR`, `TERM=dumb`, and deterministic no-color tests | Terminal policy should stay explicit and recoverable. |

## Reflection
- **Failure modes observed:** The first focused test failed on missing `THEME plain`, proving the rail was absent. After adding the rail, the full cockpit suite caught that `ACTION STRIP` was pushed off the 240-column status bar. A color-mode assertion also needed to avoid requiring contiguous text across styled spans.
- **Root cause:** Color troubleshooting existed only in the focused Status pane, while the default cockpit could look monochrome without an immediate visible explanation or recovery hint.
- **Fix that resolved it:** Added a compact always-visible wide/medium theme rail after the action affordance: active palette label, green/red/yellow swatches, and `--color always`.
- **What improved score/quality:** Operators can instantly tell whether Ratatui is rendering `ansi` or `plain`, and the recovery command is visible in the same footer where the monochrome issue appears.
- **Useful command-level evidence:** `cargo test -p hls-tui --test ratatui_cockpit wide_status_bar_renders_theme_calibration_rail`; `cargo fmt -p hls-tui --check`; `git diff --check`; `cargo test -p hls-tui --test ratatui_cockpit`; `cargo test -p hls-tui --test interactive_tui`; `cargo build -p hls-cli`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `COLUMNS=320 LINES=52 ./target/debug/hls live --top 10 --duration-secs 5 --refresh-secs 2 --tui --color always`.

## Reusable Lesson
- **Pattern that worked:** Put terminal/theme recovery in the always-visible footer, but keep safety and action affordances earlier in the line so they survive truncation.
- **Pattern to avoid:** Appending new status text after all existing rails without checking the 240-column contract.
- **Where to apply next:** Any future runtime diagnostics should first prove they do not displace `No wallet`, quality, risk, and action labels.

## Decision
- **Final chosen approach:** Wide/medium status-bar theme calibration rail that preserves existing action/safety text and keeps no-color output deterministic.
- **Commit/rollback decision:** Commit after green focused tests, full workspace tests, clippy, build, and live top-10 color smoke.
- **Next step / follow-up:** Continue Ratatui polish with visible diagnostics that are useful during live operation and tested at realistic widths.
