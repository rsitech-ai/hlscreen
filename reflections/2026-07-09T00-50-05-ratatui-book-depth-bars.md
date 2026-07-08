# Ratatui Book Depth Bars

## Intent

Continue the unified next-gen Ratatui live workstation by making the `BOOK` pane more visually useful without changing data ingestion, screening, recording, or safety boundaries.

## Success Criteria

- `BOOK` pane renders bid/ask share and public top-book notional bars.
- Narrow/focused layouts still show the core book signal instead of clipping important context.
- Fixture and short public live top-10 runs exercise the actual `hls live --tui` path.

## Failure Hypotheses

1. Extra book lines clip existing imbalance/OFI context in compact terminals.
2. Bars accidentally imply private depth or order-book levels that are not present.
3. Styling changes regress deterministic no-color output or keyboard-focused pane tests.

## Approach

- Add a behavior-first Ratatui cockpit test for share and notional bars.
- Derive bars only from existing public bid/ask price and size fields.
- Make `book_lines` height-aware so compact panes prioritize bid/ask, share bars, imbalance/OFI, and the public/proxy label.

## Evidence

- Red test first failed on missing `share bid`.
- First green attempt exposed compact-layout clipping; `narrow_cockpit_renders_focused_hidden_pane_as_drilldown` failed until the book renderer became height-aware.
- Passed: `cargo test -p hls-tui --test ratatui_cockpit --test interactive_tui`.
- Passed: `cargo clippy -p hls-tui --all-targets --all-features -- -D warnings`.
- Passed: `cargo build -p hls-cli`.
- Passed: `cargo test --workspace --all-features`.
- Passed: `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Fixture proof rendered `share bid 43% / ask 57%`, `BID notional`, `ASK notional`, and `BOOK proxy only | public top-book`.
- Short public live top-10 smoke completed with 10 symbols, 40 subscriptions, 226 WS messages, 478 market events, 0 reconnects, and 0 data gaps.

## Reuse

For future book/tape upgrades, add layout assertions before adding panel density. Compact terminal behavior is the main regression risk.
