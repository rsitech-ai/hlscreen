# Ratatui Chart Timeframe Rail

## Success Criteria
- The chart pane exposes a visible `TIMEFRAME RAIL` in the normal cockpit path.
- Existing `WINDOWS 1m 5m 15m ...` keyboard window selector behavior and tests remain intact.
- No extra vertical rows are added, preserving adaptive layout behavior.

## Failure Hypotheses
- A longer rail label may clip the active window or keyboard hint at medium widths.
- Replacing `WINDOWS` would break existing snapshot contracts.
- Compact/narrow chart controls may become too noisy.

## Candidate Approaches
- Add a new timeframe rail row above the chart.
- Rename the existing `WINDOWS` rail to include `TIMEFRAME RAIL`, preserving row count.
- Only show the label in expanded chart mode.

## Chosen Approach
Rename the existing chart window rail to include the timeframe identity while preserving the `WINDOWS` token and active-window behavior.

## Validation
- `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit cockpit_chart_renders_interactive_window_tab_rail -- --nocapture` failed before implementation on missing `TIMEFRAME RAIL`.
- Added `TIMEFRAME RAIL` only in non-compact chart mode so narrow `WIN` controls stay concise.
- `cargo fmt --check` passed.
- `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction --test ratatui_cockpit -j 1` passed.
- `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture` passed.
- `cargo test -p hls-cli --test live_mock` passed.
- Fixture TUI smoke with `hls live --fixture-file ... --once --tui --color never` passed and found `TIMEFRAME RAIL`.
- `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings` passed.

## Closeout
The chart pane now presents its keyboard-cycled candle windows as a trading-terminal timeframe rail while preserving existing chart window behavior and adaptive compact controls.
