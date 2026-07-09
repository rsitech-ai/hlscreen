# Ratatui Footer Separator Cleanup

## Success Criteria

- Wide `hls live --tui` status footer renders one separator between operational quality and risk strip.
- Quality-alert footer path still renders the alert block when degraded data exists.
- Existing Ratatui cockpit, interaction, CLI live, fixture smoke, formatting, build, and clippy checks remain green.

## Failure Hypotheses

1. Separator ownership is split across helper labels and parent span composition, causing duplicated pipes at wide widths.
2. Removing helper-owned separators could accidentally collapse the quality-alert block into adjacent text.
3. Snapshot tests may pass while real fixture output still wraps or drops key footer content.

## Attempt Result

- Moved wide footer separator ownership into `market_status_bar_line`.
- Kept quality-alert separators conditional on the alert block being present.
- Added a regression assertion for `QUALITY ... | RISK STRIP` and against `QUALITY ... |  | RISK STRIP`.
- Verified with focused footer tests, full Ratatui cockpit snapshots, interaction tests, CLI live TUI tests, live mock integration, colored and wide fixture smokes, formatting, build, diff hygiene, and clippy.
