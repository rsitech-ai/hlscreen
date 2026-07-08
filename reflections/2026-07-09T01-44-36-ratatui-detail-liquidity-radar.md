# Ratatui Detail Liquidity Radar

## Intent

Continue the unified next-gen Ratatui live workstation by making the selected-symbol detail pane read more like a trading-desk microstructure scan surface.

## Success Criteria

- Overview detail pane renders `LIQUIDITY RADAR`.
- Radar shows spread cost, top-book depth, imbalance, and signed flow as compact visual meters.
- The visible radar includes the `Public BBO/flow only` safety boundary in constrained layouts.
- No wallet, private streams, signing, orders, ingestion, recording, ranking, scoring, or screen DSL behavior changes.

## Failure Hypotheses

1. Adding more detail lines could push safety copy below the visible medium-layout detail pane.
2. Radar bars could accidentally recompute ranking/scoring instead of staying presentation-only.
3. Full-width visual strings could wrap poorly and damage the chart/book/tape layout.

## Approach

- Add one Ratatui snapshot test for the visible radar behavior.
- Render the radar from existing selected-row `FeatureSnapshot` fields only.
- Put `Public BBO/flow only` in the radar title line so the safety boundary survives constrained detail heights.

## Evidence

- Red test failed on missing `LIQUIDITY RADAR`.
- Passed: `cargo test -p hls-tui --test ratatui_cockpit detail_panel_renders_liquidity_radar`.
- Passed: `cargo fmt -p hls-tui --check`.
- Passed: `cargo test -p hls-tui --test ratatui_cockpit --test interactive_tui`.
- Passed: `cargo clippy -p hls-tui --all-targets --all-features -- -D warnings`.
- Passed: `cargo build -p hls-cli`.
- Passed: `cargo test --workspace --all-features`.
- Passed: `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Short public live top-10 smoke completed with 10 symbols, 40 subscriptions, 246 WS messages, 501 market events, 0 reconnects, and 0 data gaps while rendering `LIQUIDITY RADAR`, spread/depth bars, imbalance, flow, and `Public BBO/flow only`.

## Reuse

Future detail-pane additions should be compact first, because the medium layout gives the selected-symbol panel only a small number of reliable visible rows.
