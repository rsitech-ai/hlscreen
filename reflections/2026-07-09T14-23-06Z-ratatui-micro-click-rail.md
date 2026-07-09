# Reflection: Ratatui Micro Click Rail

## Success Criteria

- The sub-20-row micro layout exposes a visible command rail instead of only key prose.
- Mouse clicks on the micro command rail map to symbol jump, filter, preset, sort, chart window, density, zoom, pause, help, and quit.
- Existing normal-width, narrow, medium, wide, and ultra-wide mouse mappings remain unchanged.

## Failure Hypotheses

- Passing terminal height into header hit testing could shift existing command geometry.
- The micro rail could be too long for 80-column terminals and lose read-only status.
- Command-palette-open guardrails could regress and allow background mouse actions while editing.

## Candidate Approaches

- Reuse the narrow `/pstdzsp h? q` cluster, but it is hard to read in micro mode where the UI already has a dedicated header line.
- Render the same compact `CMD g / p s t d z sp ? q` rail used by medium/wide and add a height-aware mouse hit map.

## Chosen Approach

Use the compact `CMD g / p s t d z sp ? q` rail in the micro header and make `mouse_header_command_action` height-aware, so only sub-20-row terminals use the new micro hit map.

## Validation Notes

- Focused micro render tests passed and now assert the visible `CMD g / p s t d z sp ? q` rail.
- CLI mouse tests passed for micro rail actions and command-palette-open guardrails.
- Full `ratatui_cockpit`, `workstation_interaction`, `hls-cli live_tui`, `live_mock`, build, clippy with warnings denied, and fixture TUI smoke passed.
