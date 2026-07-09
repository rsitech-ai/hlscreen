# Ratatui Chart Bootstrap Lens

## Success criteria
- The chart pane has a visible live-start lens when public 1m candles have not arrived yet.
- The lens exposes selected price, BBO, flow, and public candle-feed status using existing public data only.
- Color mode styles the bootstrap lens semantically, while no-color output remains ANSI-free.

## Failure hypotheses
1. The no-candle path already has many lines, so a new lens could be clipped in the conservative fixture render if placed too low.
2. A generic colored label could pass weak ANSI tests without proving market fields are styled.
3. The copy must avoid implying synthetic candles or executable order data.

## Candidate approaches
- Insert `CHART BOOTSTRAP` immediately after the timeframe rail in the no-candle branch.
- Keep existing waiting/no-synthetic-candle copy as the explicit safety boundary.

## Attempt log
- Starting with a rendered color/no-color test for the selected-pair chart bootstrap lens.
- Red test failed on the missing `CHART BOOTSTRAP` line.
- Added a two-line bootstrap lens in the no-candle chart path: feed status first, then selected px/BBO/flow from public row data.
- Split the lens into two compact lines so `public 1m feed pending` and the colored market fields survive the center-pane width.
- Targeted rendered test passed with data-driven semantic color expectations for the fixture row.
