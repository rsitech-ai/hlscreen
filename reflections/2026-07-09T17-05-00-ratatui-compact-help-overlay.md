# Reflection Entry

## Task
- **ID/Title:** Ratatui compact help overlay
- **Date:** 2026-07-09
- **Scope:** multi-file

## Plan and Risks
- **Planned approach:** Add a narrow-viewport help overlay test, then route small popups to a compact operator map that keeps navigation keys, command keys, color diagnostics, and read-only safety visible.
- **Top failure hypotheses:** The compact overlay may hide wide-mode guidance; width detection could accidentally alter existing 150-column help tests; text wrapping could still exceed small terminal constraints.
- **Success criteria:** At 72 columns the help overlay renders `HELP COMPACT`, key controls, color path guidance, and `READ-ONLY public market data only`, with semantic color in color mode and no ANSI in no-color mode. Existing wide help tests stay green.

## Candidate Attempts
| Candidate | Summary | Outcome | Signals | Why selected / rejected |
|---|---|---|---|---|
| A | Rewrite the existing help copy to be shorter everywhere | Rejected | Would remove useful wide-terminal operator context | Wrong tradeoff for full desktop terminals |
| B | Add a compact help deck only below a popup-width threshold | Selected | Keeps wide help rich while making narrow shells usable | Matches adaptive workstation objective |

## Reflection
- **Failure modes observed:** The new narrow test first failed because the renderer had no compact help deck, then failed again because the color guidance line was still too dense and clipped `--color always`.
- **Root cause:** Help overlay copy was optimized for wide terminals, and the color diagnostic combined too many details into one wrapped line.
- **Fix that resolved it:** Added a popup-width breakpoint for compact help lines and split color guidance into short `color ... --color always` and `path ...` lines.
- **What improved score/quality:** Narrow terminals now expose usable keyboard controls, active pane context, color troubleshooting, and read-only safety without removing the richer wide help map.
- **Useful command-level evidence:** `cargo fmt --check`; `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit narrow_help_overlay_renders_compact_operator_map -- --nocapture`; `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit help_overlay -- --nocapture`; `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction --test ratatui_cockpit -j 1`; `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`; `cargo test -p hls-cli --test live_mock`; `cargo build -p hls-cli`; fixture-backed `hls live --once --tui --color always` smoke; `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`; `git diff --check`.
- **Branch comparison insight (if multiple attempts):** Not applicable.

## Reusable Lesson
- **Pattern that worked:** Breakpoint-specific overlays should split diagnostics into short rows, not just shorten the title.
- **Pattern to avoid:** Copying wide-mode command/help strings into a narrow popup and relying on wrapping to make it readable.
- **Where to apply next:** Narrow command/filter/preset diagnostics if they show similar wrapping.

## Decision
- **Final chosen approach:** Compact help overlay below the narrow popup threshold, with wide help behavior preserved.
- **Commit/rollback decision:** Commit and push after clean ancestry check.
- **Next step / follow-up:** Continue tightening narrow overlays and top-level diagnostics where text still wraps under 80 columns.
