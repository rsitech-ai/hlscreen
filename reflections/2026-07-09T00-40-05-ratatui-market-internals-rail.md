# Ratatui Market Internals Rail Reflection

## Success Criteria
- The Ratatui header gains a high-density market internals rail visible in fixture and live TUI output.
- The rail summarizes screened public market rows: count, up/down breadth, tradeable count, stale count, aggregate signed flow, and aggregate top-book depth.
- The slice remains presentation-only and read-only.

## Failure Hypotheses
- Header height changes could squeeze adaptive layouts and break narrow/medium snapshots.
- Aggregates could be computed from unscreened rows and contradict active filters.
- The rail could depend on private fills or synthetic values rather than existing public feature snapshots.

## Attempt Log
- Added a red snapshot test asserting `INTERNALS`, `rows 02`, `up 01`, `down 01`, aggregate `flow -$4.2K`, aggregate `depth $490`, and a `tradeable` label.
- Implemented a third header content line and increased the header allocation from four to five rows across wide, medium, and narrow layouts.
- Derived all rail values from `screened_rows(model)` and existing `FeatureSnapshot` fields.

## Verification
- Passed: `cargo test -p hls-tui --test ratatui_cockpit cockpit_header_renders_market_internals_rail -- --nocapture`.
- Passed: `cargo test -p hls-tui --test ratatui_cockpit --test interactive_tui`.
- Passed: `cargo clippy -p hls-tui --all-targets --all-features -- -D warnings`.
- Passed: `cargo build -p hls-cli`.
- Fixture smoke rendered `INTERNALS rows 01 ... flow -$35 depth $245`.
- Public live top-10 smoke completed with 10 symbols, 40 subscriptions, 292 WS messages, 542 market events, 0 reconnects, and 0 data gaps.

## Closeout
- This moves the TUI closer to a real workstation: users can read market breadth and liquidity pressure before drilling into a row.
