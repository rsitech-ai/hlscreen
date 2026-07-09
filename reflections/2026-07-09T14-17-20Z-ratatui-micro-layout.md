# Reflection: Ratatui Micro Layout

## Success Criteria

- Very short terminals render an intentional Ratatui fallback instead of squeezing the full cockpit stack into unreadable space.
- The fallback preserves read-only safety, live/pause state, current pane, layout dimensions, command keys, and color mode.
- Focused panes remain accessible: watchlist, detail, chart, book, tape, and status can each become the primary short-screen body.

## Failure Hypotheses

- A height-only fallback could hide too many market-data features if it always renders the watchlist.
- A custom fallback could lose the existing color/read-only diagnostics or command controls.
- Existing normal narrow/medium/wide layouts could regress if the height threshold is too high.

## Candidate Approaches

- Compress every existing layout proportionally, which risks unreadable charts and pane overlap.
- Add a micro layout for sub-20-row terminals, using the focused pane as the body and retaining a compact command/status shell.

## Chosen Approach

Add a dedicated micro layout for terminals under 20 rows. Keep normal behavior unchanged at 20+ rows, and use the current focused pane as the body so all existing pane features remain reachable by keyboard/mouse focus controls.

## Validation Notes

- Focused micro-layout tests passed for short watchlist and focused chart shells, including color/no-color diagnostics.
- Full `ratatui_cockpit`, `workstation_interaction`, `hls-cli live_tui`, and `live_mock` suites passed.
- Build, clippy with warnings denied, `git diff --check`, and fixture-backed `hls live --once --tui --color always` smoke passed.
