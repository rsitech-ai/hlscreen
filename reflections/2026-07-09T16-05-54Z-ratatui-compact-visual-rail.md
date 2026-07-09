# Ratatui Compact Visual Rail

## Success Criteria

- Medium and narrow headers expose visual/color mode instead of hiding it behind wide-only `VISUAL` copy.
- Colored compact layouts show semantic swatches for up, down, and alert states.
- No-color compact layouts stay readable and do not spend narrow-width budget on decorative swatches.
- Narrow status mode labels remain complete, including zoomed pane hints.
- Existing market data, keyboard routing, read-only language, chart panes, and CLI entrypoints remain unchanged.

## Failure Hypotheses

1. Adding visual-mode text to compact headers could push mode/filter labels past the terminal edge.
2. No-color swatches could waste narrow columns without helping diagnose terminal color support.
3. Colored assertions need to account for ANSI escape sequences between visible tokens.

## Attempt Result

- Added compact `VIS plain` / `VIS ansi` header rail below the wide-only `VISUAL ...` rail.
- Added truecolor semantic swatches only when the color path is active.
- Shortened narrow mode labels to `v:ov p:watch d:bal c:15m` so color diagnostics and pane/zoom hints both fit.
- Verified medium, narrow, and colored fixture smokes, full Ratatui snapshots, interaction tests, CLI live TUI tests, live mock integration, formatting, diff hygiene, build, and clippy.
