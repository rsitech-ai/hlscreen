# Ratatui Clickable Pane Rails Slice

## Success Criteria
- Clicking the wide `DESK` pane rail focuses the clicked pane.
- Clicking the medium and narrow `CONTROLS` pane rails focuses the clicked pane.
- Existing watchlist row click selection keeps priority over generic pane focus.
- Command-entry mode continues to ignore mouse actions.
- No read-only market-data boundaries change.

## Failure Hypotheses
- Header row offsets can drift from the rendered Ratatui block border.
- Active pane brackets can shift hit zones if the mapper assumes static text.
- Medium layout has a longer controls prefix than wide/narrow, so one shared column map could misroute clicks.

## Candidate Approaches
- Mirror the rendered header text prefixes in a small mouse hit-zone helper and return `FocusPane` before generic pane fallback.
- Use broad proportional zones across the header, but that would make clicks feel imprecise and fail to match the visible UI.

## Attempt Log
- Starting with mouse-mapper tests for wide, medium, and narrow pane rail clicks.
- Red confirmed: rail clicks initially fell through to generic `FocusPane(Status)`.
- Green focused: wide, medium, and narrow rail clicks now focus the clicked pane.
- Compact help regression caught and fixed: shortened the operator hint so the READ-ONLY safety line remains visible at 72x24.
