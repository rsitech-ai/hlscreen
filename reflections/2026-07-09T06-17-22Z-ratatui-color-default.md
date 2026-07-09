## Task
- **ID/Title:** Ratatui TUI color default
- **Date:** 2026-07-09
- **Scope:** multi-file

## Plan and Risks
- **Planned approach:** Make `hls live --tui` default to the colored Ratatui theme while preserving explicit `--color auto` and `--color never` overrides.
- **Top failure hypotheses:** Default parsing remains `auto`; docs imply the old behavior; deterministic no-color tests lose their explicit override path.
- **Success criteria:** CLI parsing proves the default is `always`; color resolution still supports `auto` and `never`; docs tell operators how to force environment-sensitive or monochrome output.

## Candidate Attempts
| Candidate | Summary | Outcome | Signals | Why selected / rejected |
|---|---|---|---|---|
| A | Change only README/run instructions to recommend `--color always`. | Rejected | User has repeatedly seen monochrome by default. | Documentation alone does not make the requested TUI beautiful out of the box. |
| B | Change `--color` default to `always`, keep explicit overrides. | Selected | Small, testable, preserves safety and deterministic controls. | Directly addresses the visible color mismatch without changing data/trading behavior. |

## Reflection
- **Failure modes observed:** The red parser test proved plain `--tui` still defaulted to `Auto`; the first manual smoke used a bad grep pattern for ESC `[`, then the corrected smoke proved ANSI output.
- **Root cause:** The TUI renderer and color palette were implemented, but the CLI default still allowed environment-driven monochrome output.
- **Fix that resolved it:** Change the `--color` default to `always`, document that `--color auto` is the opt-in environment-sensitive mode, and add parser/integration coverage.
- **What improved score/quality:** The TUI now presents the colored workstation by default while preserving explicit `auto` and `never` modes for operators and deterministic tests.
- **Useful command-level evidence:** `cargo test -p hls-cli`; `cargo test -p hls-tui --test ratatui_cockpit`; `cargo test -p hls-tui --test interactive_tui`; `cargo build -p hls-cli`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; fixture smoke with `TERM=dumb` and no `--color` emitted ANSI and rendered `THEME ansi` after stripping escape codes.
- **Branch comparison insight (if multiple attempts):** Not applicable.

## Reusable Lesson
- **Pattern that worked:** Treat visual defaults as product behavior and cover them through both parser-level tests and command-level smoke.
- **Pattern to avoid:** Relying on docs to tell users to force the desired mode when the product target expects that mode by default.
- **Where to apply next:** Ratatui defaults and diagnostics that affect what the operator actually sees.

## Decision
- **Final chosen approach:** Candidate B.
- **Commit/rollback decision:** Commit after final diff review.
- **Next step / follow-up:** Continue deeper UI polish after this default-color slice lands.
