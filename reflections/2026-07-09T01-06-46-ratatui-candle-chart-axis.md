# Ratatui Candle Chart Axis

## Intent

Continue the unified next-gen Ratatui live workstation by making the candle chart read like a real trading panel: price ladder, OHLC context, volume, and explicit public-data provenance.

## Success Criteria

- Chart renders price-axis labels beside candle glyphs.
- Chart footer names the axis range, candle count/window, OHLC values, volume, and public 1m candle source.
- Fixture and short public live top-10 runs exercise the actual `hls live --tui` path.
- No synthetic candles, private streams, wallet, signing, orders, ingestion, recording, or screen DSL behavior changes.

## Failure Hypotheses

1. Extra chart footer lines clip chart content or break compact layouts.
2. Snapshot tests accidentally assert an implementation detail instead of visible chart semantics.
3. Live short smoke may not receive a 1m candle quickly enough; if so, the waiting state still needs to remain truthful.

## Approach

- Add a behavior-first Ratatui snapshot test for focused chart axis/footer text.
- Rework `candle_chart_lines` to budget plot rows plus three footer rows.
- Prefix each plotted row with a compact price-axis label.
- Keep the existing `CandleEvent` selection and public 1m candle source unchanged.

## Evidence

- Red test failed on missing `px axis`.
- Passed: `cargo test -p hls-tui --test ratatui_cockpit cockpit_chart`.
- Passed: `cargo test -p hls-tui --test ratatui_cockpit --test interactive_tui`.
- Passed: `cargo clippy -p hls-tui --all-targets --all-features -- -D warnings`.
- Passed: `cargo build -p hls-cli`.
- Passed: `cargo test --workspace --all-features`.
- Passed: `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Fixture proof rendered `px axis`, `OHLC`, and `Public 1m candles only`.
- Short public live top-10 smoke rendered the public candle axis/footer and completed with 10 symbols, 40 subscriptions, 236 WS messages, 486 market events, 0 reconnects, and 0 data gaps.

## Reuse

For future chart polish, keep chart data provenance visible in the panel. If a richer chart is added, it must still distinguish public 1m candles from any derived or missing data state.
