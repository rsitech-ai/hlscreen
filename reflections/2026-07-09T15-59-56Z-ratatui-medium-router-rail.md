# Ratatui Medium Router Rail

## Success Criteria

- Medium layouts render the lower book/tape keyboard router as a framed command rail instead of loose text.
- The rail preserves existing hotkeys, read-only public BBO/trades language, zoom hint, and color accent behavior.
- Compact medium layouts still fit without half-word truncation.
- Ratatui snapshots, interaction tests, CLI live TUI tests, live mock integration, fixture smoke, formatting, build, and clippy remain green.

## Failure Hypotheses

1. A loose one-line router can look like an unframed rendering artifact between chart and lower panes.
2. Adding a multi-line block would steal too much vertical space from medium chart/book/tape layouts.
3. Width-aware filler could overflow or hide router labels on compact medium terminals.

## Attempt Result

- Kept the one-row layout budget and converted the router into a width-aware `╞ ... ╡` command rail.
- Added regression coverage for the framed rail markers while preserving existing public-market-data and key labels.
- Verified the real 120x40 fixture output after rebuilding `hls-cli`.
- Full validation passed for Ratatui cockpit snapshots, workstation interaction, CLI live TUI tests, live mock integration, formatting, diff hygiene, build, and clippy.
