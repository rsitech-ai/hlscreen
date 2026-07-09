# Ratatui Clickable Command Controls Slice

## Success Criteria
- Clicking visible top-bar command labels opens search/help without relying on keyboard shortcuts.
- Clicking visible compact command controls maps to filter, preset, sort, chart window, zoom, density, and help actions.
- Command-entry mode still ignores mouse actions so typed commands are not mutated by clicks.
- No wallet, private-stream, order-route, or execution behavior is introduced.

## Failure Hypotheses
- Header row numbers can drift between wide, medium, and narrow render modes.
- Short compact labels such as `/pstzh?` can be too dense for reliable hit zones unless mapped deliberately.
- Top-bar pane labels can conflict with the separate DESK pane rail unless row matching is precise.

## Candidate Approaches
- Add layout-aware command hit zones before pane/tab/watchlist fallbacks, mirroring rendered header text prefixes.
- Only document keyboard shortcuts, but that leaves visible command chrome non-interactive and below the target workstation quality.

## Attempt Log
- Starting with mouse-mapper tests for top-bar and compact-control command clicks.
- Red confirmed: top-bar `SEARCH` clicked through to generic status focus.
- Green focused: top-bar search/help, compact `/pstzh?` command cluster, and command-entry mouse protection.
- Help copy was kept short enough that full and compact overlays still show the read-only safety line.
