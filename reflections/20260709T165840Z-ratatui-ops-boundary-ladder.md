# Ratatui Ops Boundary Ladder

## Success criteria
- Expanded status/ops view renders an operator-facing gate ladder derived from existing public row quality, spread, depth, and flow signals.
- The ladder is explicitly read-only: no wallet, no private streams, no orders, no execution.
- The panel remains adaptive and does not remove existing status, risk, latency, color, or safety content.

## Failure hypotheses
- Adding more status lines could crowd expanded status panes and hide existing assertions.
- Gate wording could imply executable trading instead of screen-only market-data inspection.
- Width-sensitive rows could wrap and split key safety phrases in medium terminals.

## Attempt 1
- Add focused cockpit assertions first for the new operator gate ladder.
- Red validation: `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit expanded_status_renders_ops_command_center -j 1 -- --nocapture`
- Result: failed as expected on missing `EXECUTION BOUNDARY LADDER`.

## Attempt 2
- Implemented `status_execution_boundary_ladder_lines` in the expanded status pane.
- Gates are derived from existing public snapshot and stream health fields: confidence/staleness/reconnect/gap status for data, spread/depth status for liquidity.
- Kept the copy read-only and non-executable: observe-only, screen gates, no wallet, no private streams, no orders.
- Validation:
  - `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit expanded_status_renders_ops_command_center -j 1 -- --nocapture`
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
