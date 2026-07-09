## Task
- **ID/Title:** Ratatui fixture TUI single-frame output
- **Date:** 2026-07-09
- **Scope:** multi-file

## Plan and Risks
- **Planned approach:** Add a command-level regression test that fixture-backed `hls live --once --tui` emits exactly one Ratatui frame marker, then add a visible adaptive layout readout to the cockpit header.
- **Top failure hypotheses:** The marker appears in nested labels and makes counting brittle; the layout readout crowds compact terminals; ANSI color assertions hide duplicate visible content.
- **Success criteria:** The fixture TUI path prints one workstation frame, keeps ANSI color by default, wide/medium/narrow renders expose their layout class and terminal dimensions, and existing Ratatui/CLI tests stay green.

## Candidate Attempts
| Candidate | Summary | Outcome | Signals | Why selected / rejected |
|---|---|---|---|---|
| A | Count `WATCHLIST` occurrences in the integration test. | Rejected | Watchlist can appear in help/pane rails. | Too coupled to panel copy and future labels. |
| B | Count the frame title and expose a layout/dimensions readout. | Selected | The frame title is stable; layout class/dimensions make resize behavior visible. | Directly catches duplicate frame output and moves adaptive UX forward. |

## Reflection
- **Failure modes observed:** A suspected duplicate-frame defect was not present on current `origin/main`; the initial value was still useful as regression coverage. The actual UI gap was that resize mode was implicit, not visible to the operator.
- **Root cause:** The renderer had adaptive branches, but the cockpit did not expose which breakpoint and dimensions were active.
- **Fix that resolved it:** Pass the full viewport into the header renderer and include `layout <wide|medium|narrow> <cols>x<rows>` in the header title.
- **What improved score/quality:** Operators can now see at a glance whether the TUI is in wide, medium, or narrow mode and what dimensions Ratatui is rendering against; fixture TUI output also has regression coverage for one frame per invocation.
- **Useful command-level evidence:** `cargo fmt -p hls-tui -p hls-cli --check`; `cargo test -p hls-tui --test ratatui_cockpit`; `cargo test -p hls-tui --test interactive_tui`; `cargo test -p hls-cli --test live_mock`; `cargo build -p hls-cli`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; direct binary smoke confirmed `layout wide 160x48` and exactly one workstation frame.
- **Branch comparison insight (if multiple attempts):** Not applicable.

## Reusable Lesson
- **Pattern that worked:** Make adaptive behavior visible in the rendered artifact and prove it at each breakpoint.
- **Pattern to avoid:** Treating resize support as complete just because layout branches exist internally.
- **Where to apply next:** Fixture/non-TTY TUI smoke paths where duplicated frames can look like layout corruption.

## Decision
- **Final chosen approach:** Candidate B.
- **Commit/rollback decision:** Commit after final diff review.
- **Next step / follow-up:** Continue toward persisted TUI preferences and deeper pane-specific interactions.
