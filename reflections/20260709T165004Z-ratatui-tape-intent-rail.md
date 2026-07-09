# Ratatui Tape Intent Rail

## Success criteria
- Tape pane renders an explicit `TAPE INTENT` rail in the standard cockpit view.
- The rail states the data boundary: public prints/flow only, no fills, no private streams.
- Existing tape telemetry, recent public trades, flow pressure, quality diagnostics, and adaptive layout behavior remain intact.

## Failure hypotheses
- Adding a line could crowd compact tape views and hide existing safety copy.
- The rail could regress no-color snapshots if semantic styles leak into text expectations.
- Existing recent-trade rendering might lose required rows if the tape line budget is too tight.

## Attempt 1
- Added focused cockpit assertions for `TAPE INTENT`, `public prints/flow only`, `no private streams`, and `no fills`.
- Validation: `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit -j 1 -- --nocapture`
- Result: red as expected, 109 passed and 2 failed on missing `TAPE INTENT`.

## Attempt 2
- Added a tape intent rail and iterated on narrow-pane wrapping after the first implementation displaced existing compact tape rows.
- Final shape keeps the tape row budget stable: compact panes render `TAPE INTENT Selected flow`, wider panes render `TAPE INTENT | TAPE RAIL ...`, and the former `flow30` row becomes a width-aware public-flow scope row when needed.
- Validation:
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
