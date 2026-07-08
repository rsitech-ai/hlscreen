# Ratatui Detail Factor Stack

## Intent

Continue the unified next-gen Ratatui live workstation by making the detail pane explain the screen score visually instead of only listing raw row metrics.

## Success Criteria

- Detail and explain views render a `FACTOR STACK` line.
- The stack shows raw score, adjusted score, confidence score, and the strongest signed contributors.
- Fixture and short public live top-10 runs exercise the actual `hls live --tui` path.
- No wallet, private streams, signing, orders, ingestion, recording, screen DSL, or scoring behavior changes.

## Failure Hypotheses

1. Extra detail lines clip key flow or metadata lines at 120 columns.
2. The test could pass by matching tokens outside the visible factor row.
3. Live public data may select a different top row than the fixture, so assertions must stay semantic rather than symbol-specific.

## Approach

- Add a behavior-first Ratatui snapshot test for visible factor-stack labels and fixture contributors.
- Reuse existing `score_breakdown` data from `FeatureSnapshot`.
- Sort contributors by absolute signed contribution and render the top three as centered bars.
- Keep unavailable score breakdowns truthful with a compact fallback line.

## Evidence

- Red test failed on missing `FACTOR STACK`.
- Passed: `cargo test -p hls-tui --test ratatui_cockpit detail_panel_renders_score_factor_stack`.
- Passed: `cargo fmt -p hls-tui --check`.
- Passed: `cargo test -p hls-tui --test ratatui_cockpit --test interactive_tui`.
- Passed: `cargo clippy -p hls-tui --all-targets --all-features -- -D warnings`.
- Passed: `cargo build -p hls-cli`.
- Passed: `cargo test --workspace --all-features`.
- Passed: `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Fixture proof rendered `FACTOR STACK score raw 12.8 adj 12.8 conf 100` plus `mom`, `spread`, and `mean` signed bars.
- Short public live top-10 smoke completed with 10 symbols, 40 subscriptions, 162 WS messages, 412 market events, 0 reconnects, and 0 data gaps while rendering `FACTOR STACK`.

## Reuse

Future score UX should keep factor provenance close to the selected row and reuse `score_breakdown` rather than recalculating display-only scoring.
