# Reflection: TUI Width Adaptive Rendering

## Success Criteria
- `hls live --tui --top 10` remains readable in terminals narrower than the fixed 107-column workstation table.
- Live redraws do not wrap table or detail lines into corrupted box fragments.
- Existing deterministic screenshot and golden renderer behavior remains stable unless explicitly using the width-aware path.

## Top Failure Hypotheses
1. The renderer assumes a fixed table width and emits lines longer than the user's terminal.
2. The selected-symbol detail pane has unbounded free-form lines that wrap even if the table is compact.
3. Passing terminal width from the CLI could make fixture/golden output nondeterministic if used in non-live paths.

## Candidate Approaches
1. Full Ratatui layout runtime. Strong long-term fit, but too much blast radius for this bug.
2. Add a width-aware compact renderer path and pass live terminal width from Crossterm. Small, testable, and keeps existing output stable.

## Chosen Approach
- Use approach 2. Keep current default rendering unchanged for deterministic artifacts, and use compact columns plus bounded detail lines only when a narrow terminal width is supplied.

## Attempt Log
- Reproduced from screenshot and code inspection: the live TUI used the fixed 107-column deterministic renderer, while terminal wrapping split table rows and unbounded detail lines into corrupted fragments.
- Added a failing `hls-tui` regression test requiring an 88-column render to keep every line within width.
- Implemented `RenderOptions::for_width`, compact/narrow/mini column sets, bounded selected-detail lines, and compact live progress text.
- Wired `hls live --tui` progress and final frames to pass Crossterm terminal width into the renderer.

## Closeout
- Worked: width-aware rendering fixes the root cause without changing existing deterministic default output.
- Validation passed: `cargo test -p hls-tui --test interactive_tui --test main_table_golden`; `cargo test -p hls-cli --test live_mock`; `cargo fmt --check`; `cargo clippy -p hls-tui -p hls-cli --all-targets --all-features`.
- Reuse: future visual upgrades should keep a max-line-width regression test for any adaptive terminal mode.

## Follow-Up Clamp
- User screenshot still showed wide live headers (`sprbp`, `flow30`, `rv5m`, `amihud`) and wrapping, which means the live terminal reported enough columns to choose the 107-column layout while the actual visible pane still wrapped.
- Added `RenderOptions::for_live_terminal_width` to cap live TTY rendering at a conservative 96 columns with an 8-column safety margin, plus a tiny fallback layout.
- Added a regression test proving a reported 180-column terminal still uses compact live headers and emits no line longer than 96 characters.
- Follow-up validation passed: `cargo test -p hls-tui --test interactive_tui`; `cargo test -p hls-cli --test live_mock`; `cargo fmt --check`; `cargo clippy -p hls-tui -p hls-cli --all-targets --all-features`; `git diff --check`.
