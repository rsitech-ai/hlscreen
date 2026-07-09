# Ratatui Row Action Map

## Success Criteria

- The selected-row router exposes a clear keyboard action map for the selected symbol.
- The action map covers the main workstation panes: detail, chart, book, tape, filter, and expand.
- The copy remains display-only and does not imply orders, wallet state, or execution.
- Validation covers the public snapshot renderer and the live `--tui` fixture path before pushing.

## Failure Hypotheses

1. Adding a row action line overflows the non-expanded watchlist router height.
2. The action map duplicates header controls without giving selected-row context.
3. The wording could imply trade actions unless it stays explicitly display-only.

## Candidate Approaches

- Insert one compact `ROW ACTION MAP` line between selected-row quality and scanner leaders.
- Keep the map stable across market data, since it reflects keyboard routing rather than market state.

## Execution Log

- Starting with a wide watchlist snapshot test for `ROW ACTION MAP`.
- Red check confirmed the selected-row router had no action map: `wide_watchlist_renders_selected_row_router_strip` failed on missing `ROW ACTION MAP`.
- Added a compact two-line row action map:
  - `enter detail`, `3 chart`, `4 book`, `5 tape`
  - `/ filter`, `z expand`, `display only`
- Increased the wide row-router height so the existing scanner rail and depth leader remain visible. The full cockpit suite caught the initial clipping and passed after the adjustment.
- Validation passed:
  - `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit wide_watchlist_renders_selected_row_router_strip -- --nocapture`
  - `cargo fmt --check`
  - `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction --test ratatui_cockpit -j 1`
  - `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`
  - `cargo test -p hls-cli --test live_mock`
  - fixture `hls live --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --once --tui --color never`
  - `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`

## Closeout

- The selected-row router now makes keyboard workflows discoverable at the point of use.
- The change is display-only; no order, wallet, private stream, or execution path changed.
