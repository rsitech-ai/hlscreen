# Ratatui Market Board Score Columns

## Intent

Continue the unified next-gen Ratatui live workstation by making the watchlist itself carry score and leading-factor context, so the first scan surface feels more like a trading desk market board.

## Success Criteria

- Wide watchlist renders `SIG` and `BIAS` columns.
- `SIG` uses the existing adjusted score from `score_breakdown`.
- `BIAS` shows the strongest score component as a compact signed label such as `LIQ+` or `MOM+`.
- Compact layouts remain readable and avoid wrapping.
- No wallet, private streams, signing, orders, ingestion, recording, ranking, scoring, or screen DSL behavior changes.

## Failure Hypotheses

1. Adding columns could break the 120-column compact board that previously preserved movement and flow signals.
2. Score labels could drift from the detail factor stack if they recompute different values.
3. The CLI fixture path uses a deterministic 160-column viewport, so wide-only columns must be validated through the Ratatui renderer and a wide live smoke.

## Approach

- Add one behavior-first Ratatui snapshot test for visible `SIG`, `BIAS`, and `MOM+` market-board labels.
- Keep compact mode at narrower watchlist widths.
- Reuse existing `score_breakdown.adjusted_total` and strongest signed component; use existing liquidity/momentum score fallback only when breakdown is unavailable.

## Evidence

- Red test failed on missing `SIG`.
- Passed: `cargo test -p hls-tui --test ratatui_cockpit market_board_renders_score_and_bias_columns`.
- Passed: `cargo fmt -p hls-tui --check`.
- Passed: `cargo test -p hls-tui --test ratatui_cockpit --test interactive_tui`.
- Passed: `cargo clippy -p hls-tui --all-targets --all-features -- -D warnings`.
- Passed: `cargo build -p hls-cli`.
- Passed: `cargo test --workspace --all-features`.
- Passed: `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Short public live top-10 smoke completed with 10 symbols, 40 subscriptions, 151 WS messages, 401 market events, 0 reconnects, and 0 data gaps while rendering `SIG`, `BIAS`, `LIQ+`, `MOM+`, `FLOW30`, and `DEPTH`.

## Reuse

Future market-board changes should preserve the compact breakpoint and keep row-level scan signals derived from the same score breakdown used by the detail pane.
