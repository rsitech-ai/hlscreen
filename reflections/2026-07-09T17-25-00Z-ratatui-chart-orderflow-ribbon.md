# Reflection: Ratatui Chart Orderflow Ribbon

## Success Criteria

- Wide/focused chart renders an orderflow ribbon that aligns public trades with visible 1m candles.
- The ribbon uses only public candle and trade events already present in the model.
- Buy/sell/net notional context remains explicitly read-only and does not imply fills or advice.
- Existing tape, chart, and adaptive layout behavior remain intact.

## Failure Hypotheses

- Extra chart lines could crowd medium/wide layouts and hide the candle chart.
- Trade-to-candle alignment could be misleading if timestamps are not scoped to each candle interval.
- Adding the line beside existing print markers could duplicate information without a clearer visual signal.

## Candidate Approaches

- Add a compact per-candle orderflow ribbon to the chart when public prints exist.
- Move all trade context into the tape pane only.
- Replace print markers with a larger table, which risks crowding the chart.

## Chosen Approach

Add one compact `ORDERFLOW RIBBON` line next to the existing print markers. It is display-only, timestamp-scoped to the visible candles, and limited to layouts that already show public print markers.

## Validation

- Red check: `cargo test -p hls-tui --test ratatui_cockpit wide_chart_renders_public_print_markers_on_candles -j 1 -- --nocapture` failed before implementation because `ORDERFLOW RIBBON` was absent.
- Focused green checks: `cargo test -p hls-tui --test ratatui_cockpit wide_chart_renders_public_print_markers_on_candles -j 1 -- --nocapture`; `cargo test -p hls-tui --test ratatui_cockpit chart -j 1 -- --nocapture`.
- Broad checks: `cargo fmt --check`; `git diff --check`; `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit -j 1`; `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction -j 1`; `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`; `CARGO_INCREMENTAL=0 cargo test -p hls-cli --test live_mock -j 1`; `cargo build -p hls-cli`; `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`.
- Fixture smoke: `./target/debug/hls live --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --once --tui --color always` emitted truecolor Ratatui output with `layout narrow 80x24`, `resize-safe`, `ALGO SCAN`, `DETAIL`, `BBO`, and read-only markers.
