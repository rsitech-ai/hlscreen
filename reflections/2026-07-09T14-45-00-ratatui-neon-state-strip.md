# Ratatui Neon State Strip

## Success Criteria

- The wide Ratatui status/action rail exposes a market-state strip with regime, heat, breadth, and read-only signal-cockpit copy.
- The strip is driven by current screened rows, not hard-coded fixture text.
- Color mode visibly styles the new strip through the existing ANSI path; no-color snapshots remain ANSI-free.
- Validation covers the public snapshot renderer and the live `--tui` fixture path before pushing.

## Failure Hypotheses

1. The wide action rail becomes too long and clips the new market-state copy.
2. The strip duplicates risk-strip information without making the terminal more visually scan-friendly.
3. The wording implies trading advice unless it remains explicitly read-only.

## Candidate Approaches

- Add the strip to the existing second status-bar line only for wide terminals.
- Keep medium/narrow status bars compact and unchanged.

## Execution Log

- Starting with a wide no-color/color snapshot test for `NEON STATE`.
- Red check confirmed the rail was missing: `wide_status_bar_renders_neon_market_state_strip` failed on missing `NEON STATE`.
- Added a wide-only `NEON STATE` strip to the existing action/status rail:
  - regime from current screened rows
  - heat from up/down breadth
  - breadth counts
  - `read-only signal cockpit` safety copy
- Used the existing cyan accent for the label so color snapshots prove a visible ANSI transition.
- Validation passed:
  - `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit wide_status_bar_renders_neon_market_state_strip -- --nocapture`
  - `cargo fmt --check`
  - `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction --test ratatui_cockpit -j 1`
  - `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`
  - `cargo test -p hls-cli --test live_mock`
  - fixture `hls live --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --once --tui --color never`
  - `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`

## Closeout

- The wide cockpit now has a more visual market-state rail in the status/action area while keeping no-color captures clean.
- The strip is informational and read-only; no order, wallet, private stream, or execution path changed.
