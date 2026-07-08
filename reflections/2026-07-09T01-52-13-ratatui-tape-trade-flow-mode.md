# Ratatui Tape Trade Flow Mode

## Intent

Continue the unified next-gen Ratatui live workstation by making the focused `TAPE` pane expose a higher-signal public trade pressure view through existing keyboard state.

## Success Criteria

- Focusing `TAPE` and cycling to `view:flow` renders `TRADE FLOW MODE`.
- Trade flow mode shows buy pressure, sell pressure, and trade skew from selected-symbol public trades.
- The public trade tape remains visible, including `Public trades only | no fills`.
- No wallet, private streams, signing, fills, orders, ingestion, recording, ranking, scoring, key mapping, or screen DSL behavior changes.

## Failure Hypotheses

1. Extra summary rows could crowd out the public trade tape in focused layouts.
2. Aggregating public trades could accidentally imply private fills or execution state.
3. Reusing `view:flow` could change normal overview tape behavior unexpectedly.

## Approach

- Add one Ratatui snapshot test for focus TAPE plus `NextView` with fixture public trades.
- Aggregate only selected-symbol public `TradeEvent` side, count, and notional fields.
- Keep the existing trade list and read-only/no-fills safety copy below the new pressure summary.

## Evidence

- Red test failed on missing `TRADE FLOW MODE`.
- Passed: `cargo test -p hls-tui --test ratatui_cockpit tape_pane_flow_view_renders_public_trade_pressure_mode`.
- Passed: `cargo fmt -p hls-tui --check`.
- Passed: `cargo test -p hls-tui --test ratatui_cockpit --test interactive_tui`.
- Passed: `cargo clippy -p hls-tui --all-targets --all-features -- -D warnings`.
- Passed: `cargo build -p hls-cli`.
- Passed: `cargo test --workspace --all-features`.
- Passed: `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Short public live top-10 smoke completed with 10 symbols, 40 subscriptions, 225 WS messages, 475 market events, 0 reconnects, and 0 data gaps while preserving the live `BOOK`, `TAPE`, `LIQUIDITY RADAR`, `FLOW pulse`, and `PUBLIC TRADES` panels.

## Reuse

Future focused pane modes should keep safety copy visible and use the existing focus/view model until there is a stronger reason to add another key binding.
