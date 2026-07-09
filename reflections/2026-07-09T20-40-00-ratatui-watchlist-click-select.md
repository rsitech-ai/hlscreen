# Ratatui Watchlist Click Select Slice

## Success Criteria
- Clicking a visible watchlist row selects that row and focuses the watchlist.
- Mouse hit testing respects wide/medium/narrow adaptive layout geometry.
- Command-entry mode remains protected: mouse actions do not mutate command input or market focus.
- Read-only market-data and no-order boundaries are unchanged.

## Failure Hypotheses
- Row hit testing could drift from rendered watchlist table offsets and select the wrong row.
- Mouse mapping could need current row count to avoid selecting invisible rows.
- Adding click selection could accidentally override pane focus clicks in other regions.

## Candidate Approaches
- Add `WorkstationAction::SelectRow(index)` and compute visible-row click indices in the live mouse mapper with the current screened row count.
- Keep mouse clicks as pane-focus only and document keyboard navigation better, but that does not move the TUI toward a richer interactive workstation.

## Attempt Log
- Starting with state and mouse-mapper tests, then threading row count into mouse event handling.
- Focused green: `workstation_state_selects_clicked_watchlist_row`,
  `live_tui_mouse_events_map_to_keyboard_parity_actions`,
  `help_overlay_color_mode_renders_operator_keyboard_map`, and
  `narrow_help_overlay_renders_compact_operator_map`.
