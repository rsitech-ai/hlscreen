# Ratatui Public Trade Tape

## Intent

Continue the unified next-gen Ratatui live workstation by making the right-side tape panel show actual selected-symbol public trades when trade frames are available, closer to the reference trading-terminal layout.

## Success Criteria

- The TAPE pane renders `PUBLIC TRADES` with side, price, and notional for the selected symbol.
- Flow pulse and net pressure remain visible as aggregate context.
- Fixture and short public live top-10 runs exercise the real `hls live --tui` path.
- No wallet, private streams, signing, fills, order routes, ingestion contract, recording format, scoring, or screen DSL behavior changes.

## Failure Hypotheses

1. Adding trades to `RatatuiFrameModel` could diverge fixture/final/live-progress rendering paths.
2. A recent-trades view could accidentally look like account fills unless the safety copy stays explicit.
3. Narrow or medium terminals could clip the TAPE safety label or hide the aggregate flow context.

## Approach

- Add one behavior-first Ratatui snapshot test for a focused Tape pane with fixture-backed trades.
- Mirror the existing candle presentation path: clone public trade events from `LiveMarketState` into the Ratatui model at the presentation boundary.
- Filter trades by selected symbol, sort newest first, and cap rows by panel height.
- Keep the previous flow leaderboard fallback when no selected-symbol trades exist.

## Evidence

- Red test failed on missing `with_trades`.
- Passed: `cargo test -p hls-tui --test ratatui_cockpit tape_pane_renders_public_recent_trades_when_available`.
- Passed: `cargo fmt -p hls-tui -p hls-cli --check`.
- Passed: `cargo test -p hls-tui --test ratatui_cockpit --test interactive_tui`.
- Passed: `cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -- -D warnings`.
- Passed: `cargo build -p hls-cli`.
- Passed: `cargo test --workspace --all-features`.
- Passed: `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Fixture proof rendered `PUBLIC TRADES`, `SELL`, `BUY`, price/notional rows, and `Public trades only | no fills`.
- Short public live top-10 smoke completed with 10 symbols, 40 subscriptions, 176 WS messages, 428 market events, 0 reconnects, and 0 data gaps while rendering live `PUBLIC TRADES` rows.

## Reuse

For future tape polish, keep actual public trades separate from account fills and preserve explicit no-private-stream/no-fill copy whenever the UI resembles execution tooling.
