# Ratatui Pane Accent Borders

## Success criteria
- Color-mode renders distinct pane border accents for the main cockpit panes instead of one generic border color.
- No-color snapshots remain ANSI-free and usable in terminals that do not support color.
- Live fixture smoke still shows the existing cockpit features and read-only guardrails.

## Failure hypotheses
1. The ANSI snapshot helper only captures text foreground/background and not border styling, making the test too implementation-adjacent.
2. Pane accent colors collide with existing signal colors, so a test passes without proving panel hierarchy changed.
3. Focus styling loses too much contrast if the focused pane stops using the warning color.

## Candidate approaches
- Add a pane-aware border style helper used by `panel_for`, with color-mode RGB accents and no-color white fallback.
- Add visual title badges per pane, if border ANSI is not reliably captured by the test backend.

## Attempt log
- Starting with a focused snapshot test for color/no-color behavior before editing the renderer.
- Red test failed on the missing chart accent ANSI code, confirming generic border styling.
- Added pane-aware border colors for watchlist/detail/chart/book/tape/status and routed the status bar through the status-pane accent.
- Targeted test passed; removed the now-unused generic focus border helper.
