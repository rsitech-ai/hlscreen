# Reflection: Ratatui Internals Rail Mouse

## Success Criteria

- The market internals rail is clickable in narrow, medium, standard-wide, and ultra-wide header layouts.
- Clicks route to display-only panes: rows/heat/up/down to watchlist, tradeability/staleness to status, flow to tape, and depth to book.
- Existing quote rail, command rail, pane rail, and command-palette capture behavior remain intact.
- No market-data ingestion, scoring, wallet, private stream, or order-route code changes.

## Failure Hypotheses

- Header row numbers differ by breakpoint because quote and command dock rows appear only on wider layouts.
- Broad column bands may steal unrelated header clicks if row matching is wrong.
- Narrow columns are compressed enough that wide-layout bands would route incorrectly.

## Candidate Approaches

- Add exact hit testing based on rendered label text.
- Add breakpoint-specific broad column bands for the internals row.
- Make the whole internals rail focus status only.

## Chosen Approach

Use breakpoint-specific row detection and broad, stable column bands. The routing remains display-only and mirrors the semantics users expect from the visible labels rather than coupling mouse input to exact rendered metric values.

## Validation

- Red check: `cargo test -p hls-cli live_tui_mouse_clicks_market_internals_rail -j 1 -- --nocapture` failed before implementation because internals clicks fell through to generic `FocusPane(Watchlist)`.
- Focused green: `cargo test -p hls-cli live_tui_mouse_clicks_market_internals_rail -j 1 -- --nocapture`.
- Regression green: `cargo test -p hls-cli live_tui_mouse_clicks_standard_wide_selected_quote_rail -j 1 -- --nocapture`.
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
