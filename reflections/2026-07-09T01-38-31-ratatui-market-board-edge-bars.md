# Ratatui Market Board Edge Bars

## Intent

Continue the unified next-gen Ratatui live workstation by turning the wide watchlist score column into a faster visual scan surface.

## Success Criteria

- Wide watchlist renders an `EDGE` column beside `SIG` and `BIAS`.
- `EDGE` uses the same score source as `SIG`.
- Compact layouts keep the prior adaptive market-board row shape.
- No wallet, private streams, signing, orders, ingestion, recording, ranking, scoring, or screen DSL behavior changes.

## Failure Hypotheses

1. Adding another visual column could clip existing `FLOW30` or `DEPTH` context on wide layouts.
2. The bar could silently diverge from `SIG` if it recomputes from different fields.
3. Block glyphs could make deterministic snapshot assertions brittle if tests depend on exact frame geometry.

## Approach

- Extend the existing market-board score test to require `EDGE` and block output.
- Reuse the same `score_signal_value` helper for numeric `SIG` and visual `EDGE`.
- Tighten the wide market-board column constraints only enough to preserve existing scan fields.

## Evidence

- Red test failed on missing `EDGE`.
- Passed: `cargo test -p hls-tui --test ratatui_cockpit market_board_renders_score_and_bias_columns`.
- Passed: `cargo fmt -p hls-tui --check`.
- Passed: `cargo test -p hls-tui --test ratatui_cockpit --test interactive_tui`.
- Passed: `cargo clippy -p hls-tui --all-targets --all-features -- -D warnings`.
- Passed: `cargo build -p hls-cli`.
- Passed: `cargo test --workspace --all-features`.
- Passed: `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Short public live top-10 smoke completed with 10 symbols, 40 subscriptions, 185 WS messages, 435 market events, 0 reconnects, and 0 data gaps while rendering `SIG`, `EDGE`, `BIAS`, block bars, `FLOW30`, and `DEPTH`.

## Reuse

Future market-board visual encodings should share score helpers with the detail pane and keep compact breakpoints conservative.
