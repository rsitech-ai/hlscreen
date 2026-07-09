# Ratatui Chart Structure Tape

## Success criteria
- Focused chart view renders a compact public-candle structure tape.
- The tape derives structure from existing candle data: higher highs, lower lows, range, and latest volume pulse.
- Copy stays read-only and does not imply execution, recommendations, or private data.
- Existing chart HUDs, candle plot, prints strip, order-pressure lane, and adaptive behavior remain intact.

## Failure hypotheses
- Adding a line to chart focus could crowd existing candle plot rows in medium viewports.
- Structure labels could wrap in narrow chart panes and break test-visible phrases.
- Trend wording could imply advice if not explicitly scoped to public-candle context.

## Attempt 1
- Add focused chart cockpit assertions for a `STRUCTURE TAPE` public-candle row.
- Red validation: `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit cockpit_chart_renders_selected_pair_edge_hud -j 1 -- --nocapture`
- Result: failed as expected on missing `STRUCTURE TAPE`.

## Attempt 2
- Implemented a width/height-gated chart structure tape for wide focused chart layouts.
- The row computes higher-high/lower-low counts, range, and volume pulse from public candles and starts with `public candles only` so the scope is visible before metrics.
- A full cockpit run initially showed the standard focused chart lost `VOL LANE`; the breakpoint was tightened to wide/tall chart panes and the assertion moved to the wide chart pressure-lane test.
- Validation:
  - `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit wide_chart_renders_selected_pair_order_pressure_lane -j 1 -- --nocapture`
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
