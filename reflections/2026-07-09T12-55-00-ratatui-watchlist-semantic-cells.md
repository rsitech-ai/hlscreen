# Ratatui Watchlist Semantic Cells

## Success criteria
- The watchlist scan board uses semantic cell color for price/signal/spread/flow/depth/quality instead of flattening the whole row into one color.
- The selected row keeps market-direction foregrounds in color mode while preserving selection emphasis.
- No-color snapshots keep the same readable text and remain ANSI-free.

## Failure hypotheses
1. Ratatui row style may override cell style in a way that prevents selected-row semantic colors from appearing.
2. Existing colored text elsewhere may make ANSI tests pass without proving the watchlist cells changed.
3. Styling every table branch could create noisy duplication unless small cell helpers keep the rendering readable.

## Candidate approaches
- Introduce watchlist-specific cell helpers for selected/non-selected semantic foreground and selection background.
- Start with the main enhanced/default scan board, then extend compact/quality/explain branches only where the same helpers naturally fit.

## Attempt log
- Starting with a rendered color/no-color behavior test for selected-row semantic watchlist cells.
- Red test failed because the selected-row background/semantic-cell contract was not present.
- Added watchlist cell helpers for selected background plus direction, spread, flow, depth, quality, confidence, staleness, tradeability, and resilience foregrounds.
- Targeted test passed after anchoring assertions on stable selected-row table output rather than cross-panel trend text.
