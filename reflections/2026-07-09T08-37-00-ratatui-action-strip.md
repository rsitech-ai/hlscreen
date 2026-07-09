# Reflection Entry: Ratatui Action Strip

## Task
- **ID/Title:** Ratatui action strip
- **Date:** 2026-07-09
- **Scope:** focused TUI renderer/test slice

## Plan and Risks
- **Planned approach:** Replace the wide/medium footer's placeholder `ACTION STRIP` with a compact, always-visible keyboard command rail while preserving wallet, quality, risk, and theme diagnostics.
- **Top failure hypotheses:** Adding keys can push theme diagnostics or safety labels off the 240-column contract; color-mode spans can make contiguous text assertions brittle; the footer must not imply order-entry actions.
- **Success criteria:** A Ratatui cockpit test proves the action strip includes concrete keyboard controls at 240 columns, existing status/theme tests stay green, and focused/broad validation plus live smoke pass.

## Candidate Attempts
| Candidate | Summary | Outcome | Signals | Why selected / rejected |
|---|---|---|---|---|
| A | Leave `ACTION STRIP` as a placeholder | Rejected | It does not satisfy keyboard-interactive UX on the always-visible footer | Too decorative. |
| B | Add full prose command descriptions to the footer | Rejected | 240-column footer already carries market, risk, safety, and theme rails | Would truncate important safety/ops text. |
| C | Add a terse key rail after `ACTION STRIP` | Selected | Matches dense terminal UI conventions | Keeps the workstation keyboard-forward without changing behavior. |

## Reflection
- **Failure modes observed:** The first test failed because `ent detail` was not visible. A first implementation still failed because the footer was too dense for one line at 240 columns, confirming that the one-line footer was the wrong layout for richer keyboard UX.
- **Root cause:** The wide footer had an `ACTION STRIP` label, but no concrete commands; adding commands to the existing market/risk/theme line caused truncation pressure.
- **Fix that resolved it:** Wide and medium layouts now reserve a two-line footer: market/risk/safety on the first line, and keyboard actions plus theme diagnostics on the second line. Narrow mode stays compact.
- **What improved score/quality:** The live cockpit now has an always-visible keyboard command rail with row navigation, detail focus, view cycling, filter, preset, sort, chart window, help, and quit controls, without implying any order-entry path.
- **Useful command-level evidence:** `cargo test -p hls-tui --test ratatui_cockpit wide_status_bar_renders_action_key_rail`; `cargo fmt -p hls-tui --check`; `git diff --check`; `cargo test -p hls-tui --test ratatui_cockpit`; `cargo test -p hls-tui --test interactive_tui`; `cargo build -p hls-cli`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `COLUMNS=320 LINES=52 ./target/debug/hls live --top 10 --duration-secs 5 --refresh-secs 2 --tui --color always`.

## Reusable Lesson
- **Pattern that worked:** Split dense operational footer information into stable rows instead of forcing market data, safety, actions, and theme into one truncation-prone line.
- **Pattern to avoid:** Treating footer labels as decorative placeholders; if a rail is visible, it should carry useful live operator information.
- **Where to apply next:** Future adaptive improvements should use explicit footer/header rows at wide and medium breakpoints, while preserving the narrow compact footer.

## Decision
- **Final chosen approach:** Two-line wide/medium footer with market/risk first and action/theme second.
- **Commit/rollback decision:** Commit after green focused tests, full workspace tests, clippy, build, and live top-10 color smoke.
- **Next step / follow-up:** Continue polishing high-density keyboard affordances and focused pane drilldowns without weakening read-only boundaries.
