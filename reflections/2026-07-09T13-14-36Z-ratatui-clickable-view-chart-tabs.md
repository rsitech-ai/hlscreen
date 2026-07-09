# Ratatui Clickable View And Chart Tabs Slice

## Success Criteria
- Clicking a visible detail `VIEWS` tab jumps directly to that view.
- Clicking a visible chart `WINDOWS`/`WIN` tab jumps directly to that chart window.
- Wide, medium, and narrow layout geometry stays aligned with rendered panel positions.
- Command-entry mode continues to ignore mouse actions.
- No read-only market-data, wallet, private-stream, or order-route boundaries change.

## Failure Hypotheses
- Panel block borders shift tab rows by one line, causing generic pane focus instead of direct tab selection.
- Compact labels (`ov`, `ql`, `30`) use different widths from full labels and can misroute narrow clicks.
- Adding direct actions could bypass existing row-count clamping or command-palette protections.

## Candidate Approaches
- Add direct `SetView` and `SetChartWindow` actions, then route mouse clicks through layout-aware tab hit zones before generic pane focus.
- Reuse only existing cycle actions from mouse clicks, but that makes direct tab selection unpredictable and weaker than the visible UI suggests.

## Attempt Log
- Starting with state and mouse-mapper tests for direct view/window tab selection.
- Red confirmed: direct `SetView` and `SetChartWindow` actions were missing.
- Green focused: state direct-tab actions, wide/narrow mouse view-tab clicks, wide/narrow mouse chart-window clicks, and full/compact help assertions.
