# Reflection: Ratatui Quote Rail Mouse

## Success Criteria

- Standard-wide selected quote rail clicks route to useful focused panes: symbol/quote to detail, bid/ask/top-book to book, and flow to tape.
- Existing standard-wide controls remain clickable after the quote rail shifts them down one rendered row.
- Ultra-wide row geometry remains unchanged.
- Command palette capture still blocks quote-rail and controls clicks.
- The behavior stays read-only and only changes display focus.

## Failure Hypotheses

- The standard-wide quote rail row could collide with existing controls-row mouse geometry.
- Adding broad row matching could steal clicks from command docks or pane rails on ultra-wide layouts.
- Exact column hit zones could be brittle if they depend on per-row market values.

## Candidate Approaches

- Make the whole quote rail focus detail only.
- Map broad column bands on the quote row to detail/book/tape.
- Parse rendered text positions dynamically, but that would couple CLI input handling to renderer output.

## Chosen Approach

Use stable row geometry and broad, value-independent column bands for the selected quote rail. Preserve command capture and route only to focus actions, never trading or private-data paths.

## Validation

- Red check: `cargo test -p hls-cli live_tui_mouse_clicks_standard_wide_selected_quote_rail -j 1 -- --nocapture` failed before implementation because row 3 fell through to `FocusPane(Watchlist)`.
- Focused green: `cargo test -p hls-cli live_tui_mouse_clicks_standard_wide_selected_quote_rail -j 1 -- --nocapture`.
- Regression green: `cargo test -p hls-cli live_tui_mouse_clicks_visible_pane_rails -j 1 -- --nocapture`.
- Regression green: `cargo test -p hls-cli live_tui_mouse_clicks_visible_command_controls -j 1 -- --nocapture`.
- `cargo fmt --check`
- `git diff --check`
- `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit -j 1`
- `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction -j 1`
- `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`
- `CARGO_INCREMENTAL=0 cargo test -p hls-cli --test live_mock -j 1`
- `cargo build -p hls-cli`
- `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`
- `./target/debug/hls live --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --once --tui --color always --data-dir <tmp>` fixture smoke: `fixture tui smoke ok 24 lines`
