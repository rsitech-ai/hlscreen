# Ratatui Focus Action Rail Reflection

## Success criteria
- Wide and standard-wide layouts show the active pane and its primary keyboard action.
- Existing generic hotkeys, adaptive layout copy, and read-only safety wording remain visible.
- No new mode or trading action is introduced.

## Failure hypotheses
- Adding focus copy could crowd the medium action strip.
- The label could duplicate existing status text without improving keyboard orientation.
- The chart-focused view could accidentally imply execution instead of read-only analysis.

## Candidate approaches
- Add a `FOCUS <pane action>` segment to the existing action strip.
- Add a new focus line in the header.

## Attempt 1
- Added behavior tests for default watchlist focus and chart focus.
- Signal: `ratatui_cockpit` fails only the new `FOCUS` expectations; existing behavior remains green.

## Attempt 2
- Adding focus to the 240-column action or market rails crowded neon, risk, and quality details.
- Moving focus to the desk rail crowded visible/hidden pane diagnostics.
- Final shape: focused pane titles carry the focus key hint, and standard-wide action strips add the focus key hint where there is room.
- Signal: `ratatui_cockpit` passes all 111 tests.

## Closeout
- `ratatui_cockpit`: 111 passed.
- `workstation_interaction`: 11 passed.
- `hls-cli live_tui`: 18 passed.
- `live_mock`: 3 passed.
- `cargo fmt --check`, `git diff --check`, `cargo build -p hls-cli`, and strict clippy passed.
