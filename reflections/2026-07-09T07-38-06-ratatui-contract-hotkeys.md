# Reflection Entry: Ratatui Contract Hotkeys

## Task
- **ID/Title:** Ratatui contract hotkeys
- **Date:** 2026-07-09
- **Scope:** focused CLI/TUI interaction slice

## Plan and Risks
- **Planned approach:** Add test-first support for the contract hotkeys `h` and `Enter`: `h` focuses the Status/health pane, and plain `Enter` focuses the selected symbol detail pane when no command palette is open.
- **Top failure hypotheses:** Enter may already be reserved for command submission, so the implementation must preserve command-mode behavior; help text can drift from actual key behavior; key mapping changes must not alter live ingestion or read-only boundaries.
- **Success criteria:** Existing command-mode Enter behavior remains unchanged, new tests prove normal-mode `h`/`H` and Enter mapping, and focused/broad validation plus live smoke stay green.

## Candidate Attempts
| Candidate | Summary | Outcome | Signals | Why selected / rejected |
|---|---|---|---|---|
| A | Add new action variants for health/detail | Rejected | Existing `FocusPane(Status)` and `FocusPane(Detail)` already represent the behavior | Extra enum variants would add noise without new capability. |
| B | Map `h` and Enter directly to existing focus actions | Selected | Minimal, contract-aligned, preserves command-mode Enter | Moves keyboard interaction closer to the documented TUI contract. |

## Reflection
- **Failure modes observed:** The first visible legend update was too verbose for narrow terminals and clipped existing `p preset` / `s sort` hints in the status drilldown.
- **Root cause:** Wide-keyboard wording was pushed into compact rows that already run near the terminal-width budget.
- **Fix that resolved it:** Keep `Enter` and `h` mapped in the CLI, use compact `ent h` copy in narrow headers, and preserve the existing compact status command rail while adding the new keys to the safety line.
- **What improved score/quality:** The TUI now satisfies the documented `h` health/status and `Enter` detail controls without weakening command-mode Enter submission or read-only labels.
- **Useful command-level evidence:** `cargo test -p hls-cli commands::live::tests::live_tui_control_keys_map_to_screen_actions`; `cargo test -p hls-tui --test ratatui_cockpit`; `cargo test -p hls-tui --test interactive_tui`; `cargo build -p hls-cli`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `COLUMNS=320 LINES=52 ./target/debug/hls live --top 10 --duration-secs 5 --refresh-secs 2 --tui`.

## Reusable Lesson
- **Pattern that worked:** Map new keyboard affordances to existing pane-focus actions first, then update compact and wide legends separately.
- **Pattern to avoid:** Treating one wide help string as safe for narrow status/header surfaces.
- **Where to apply next:** Future hotkeys should be added through the same path: CLI key mapper test, pure state/render assertions, compact wording audit, live smoke.

## Decision
- **Final chosen approach:** Direct `Enter` and `h` mappings to existing Detail and Status pane focus actions, with width-aware legends.
- **Commit/rollback decision:** Commit after validation.
- **Next step / follow-up:** Continue improving pane-specific keyboard workflows and visual affordances without changing read-only market-data boundaries.
