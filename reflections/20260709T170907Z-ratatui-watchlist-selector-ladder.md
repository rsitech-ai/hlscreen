# Ratatui Watchlist Selector Ladder

## Success criteria
- Expanded watchlist command deck renders a keyboard selection ladder.
- The ladder shows previous/current/next row context from the actual screened rows.
- Copy stays read-only and keyboard-focused: j/k navigation, inspect only, no orders.
- Existing scanner rail, heatmap deck, command center, and compact watchlist layouts remain intact.

## Failure hypotheses
- The expanded row-command deck is already dense; adding lines may clip existing hotkeys or leader rows.
- A selection ladder could imply action/execution unless it stays scoped to navigation and inspection.
- Wider row-router height could starve the watchlist table on shorter layouts.

## Attempt 1
- Add expanded watchlist cockpit assertions for `SELECTOR LADDER`, previous/current/next row context, and keyboard navigation copy.
- Red validation: `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit expanded_watchlist_renders_command_center_deck -j 1 -- --nocapture`
- Result: failed as expected on missing `SELECTOR LADDER`.

## Attempt 2
- Implemented `watchlist_selector_ladder_lines` in the expanded row-command deck.
- Increased expanded row-router height from 15 to 17 so the added ladder does not clip the existing heatmap or command-center lines.
- The ladder derives `prev/current/next` from the current screened row slice and selected symbol, and stays keyboard/read-only scoped.
- Validation:
  - `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit expanded_watchlist_renders_command_center_deck -j 1 -- --nocapture`
  - `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit -j 1 -- --nocapture`
  - `cargo fmt`
  - `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction -j 1`
  - `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`
  - `CARGO_INCREMENTAL=0 cargo test -p hls-cli --test live_mock -j 1`
  - `cargo fmt --check`
  - `git diff --check`
  - `cargo build -p hls-cli`
  - `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`
- Result: all checks passed.
