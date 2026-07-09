# Ratatui Watchlist Command Deck

## Success Criteria

- Wide and standard-wide watchlist row context renders as a deliberate command deck, not loose text under the table.
- The deck preserves selected row routing, row actions, scanner leaders, read-only context, and depth evidence.
- Long symbols such as `DOWN/USDC` do not clip scanner depth at 220 columns.
- Expanded watchlist keeps the market heatmap and command-center hotkeys visible.
- Existing read-only market-data panes, charts, keyboard routing, CLI live path, and color behavior remain unchanged.

## Failure Hypotheses

1. Switching from a top border to a full block reduces inner height and can clip scanner lines.
2. Long row/action labels can wrap inside the left rail and hide depth evidence.
3. Expanded watchlist combines row router, heatmap, and command center, so it needs a larger context budget than the default wide rail.

## Attempt Result

- Replaced the loose watchlist row context with a titled `ROW COMMAND DECK` block.
- Converted router/action/scanner context into `╞ ... ╡` command rails.
- Split verbose router and action text across compact rail lines so 220-column and 240-column layouts both retain scanner depth.
- Increased row-context height only for non-expanded and expanded watchlist layouts that use the framed deck.
- Verified focused watchlist tests, 220/240-column fixture smokes, full Ratatui snapshots, interaction tests, CLI live TUI tests, live mock integration, formatting, diff hygiene, build, and clippy.
