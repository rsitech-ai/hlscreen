# Ratatui medium panel fit pass

## Success criteria
- A 100x30 fixture render no longer clips the watchlist title, candle title, or lower adaptive desk router.
- The compact copy still preserves the same market data, chart data, pane navigation, and read-only safety language.
- Wider medium and wide layouts keep their richer labels.

## Failure hypotheses
- Block titles were authored for wider panes and relied on terminal cropping.
- The lower pane router did not know its available width.
- The candle title needed a compact OHLCV variant below the standard medium width.

## Result
- Added regression coverage for compact medium panel titles and router copy.
- Added compact watchlist title, compact candle title, and width-aware adaptive desk router copy.
- Verified cockpit, interaction, CLI live-TUI, live mock, build, clippy, colored fixture smoke, and explicit 100-column panel markers.
