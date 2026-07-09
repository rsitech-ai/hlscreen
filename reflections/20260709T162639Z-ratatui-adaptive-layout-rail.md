# Ratatui Adaptive Layout Rail Reflection

## Success criteria
- Header controls expose the active adaptive profile for narrow, medium, and wide terminal sizes.
- The rail names viewport size and visible/hidden panes without adding vertical space.
- Existing fit-to-width rails and read-only safety wording remain intact.

## Failure hypotheses
- Extra copy may wrap in medium terminals and reintroduce half-word artifacts.
- Wide status/action rails may crowd theme or risk copy.
- Narrow status focus may already be dense enough that adaptive copy must stay compact.

## Candidate approaches
- Add a compact `ADAPT` segment to existing layout control lines.
- Add a separate header line for layout diagnostics.

## Attempt 1
- Added behavior tests for wide, medium, compact-medium, and narrow status focus.
- Signal: `ratatui_cockpit` fails only the four new adaptive-profile expectations; existing behavior remains green.

## Attempt 2
- A verbose inline adaptive rail clipped existing 72-120 column hotkeys.
- Revised labels to `w/m/n + viewport + visible/hidden counts`, kept narrow header controls unchanged, and surfaced the exact narrow viewport marker only when status focus has room.
- Signal: `ratatui_cockpit` passes all 111 tests.

## Closeout
- `ratatui_cockpit`: 111 passed.
- `workstation_interaction`: 11 passed.
- `hls-cli live_tui`: 18 passed.
- `live_mock`: 3 passed.
- `cargo fmt --check`, `git diff --check`, `cargo build -p hls-cli`, and strict clippy passed.
