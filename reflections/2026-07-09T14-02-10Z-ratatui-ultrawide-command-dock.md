# Reflection: Ratatui Ultra-Wide Command Dock

## Success Criteria

- The ultra-wide Ratatui header exposes the same core workstation actions as the keyboard map: pane focus, symbol search, filter, preset, sort, chart window, density, zoom, pause, help, and quit.
- The visible top command dock includes live state for chart window, density, zoom/grid, and pause/live so the user can see what mode they are in.
- Mouse hit testing maps the visible ultra-wide dock labels to the same actions without changing command-entry guardrails.

## Failure Hypotheses

- The rendered command dock becomes too long and truncates important controls on common wide terminals.
- Click columns drift from the rendered labels because the top bar uses dynamic state text.
- New mouse handling steals clicks while the command palette is open.

## Candidate Approaches

- Replace the existing ultra-wide top strip with a compact stateful dock and keep the bottom action strip as a redundant command surface.
- Add a second header line only for ultra-wide terminals, but that risks crowding the selected quote and market internals rails.

## Chosen Approach

Update the existing ultra-wide top strip in place, keep labels compact, and add direct unit coverage for both rendering and mouse hit mapping.

## Validation Notes

- Targeted Ratatui render test passed for the stateful ultra-wide command dock.
- CLI mouse tests passed after preserving the existing behavior where clicking the already-active top pane toggles zoom.
- Full `ratatui_cockpit`, `workstation_interaction`, `hls-cli live_tui`, `live_mock`, `cargo build -p hls-cli`, clippy with warnings denied, and fixture TUI smoke passed.
