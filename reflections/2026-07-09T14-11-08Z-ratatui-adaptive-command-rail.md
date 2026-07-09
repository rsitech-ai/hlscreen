# Reflection: Ratatui Adaptive Command Rail

## Success Criteria

- Medium and standard-wide Ratatui layouts expose the global command controls in the header instead of relying only on the bottom action strip.
- The command rail remains resize-safe: medium keeps visible/hidden pane truth, wide keeps clickable pane tabs, and ultra-wide behavior is unchanged.
- Mouse hit testing maps the visible medium/wide command rail labels to the same read-only display actions as keyboard controls.

## Failure Hypotheses

- Adding command labels to the medium header could truncate existing visible/hidden pane labels.
- Adding command labels to the wide desk row could shift pane-tab click geometry.
- A new header command hit map could steal pane-focus clicks or command-palette clicks.

## Candidate Approaches

- Add a new header row, which would cost body space and break medium header height.
- Reuse the existing desk row as an adaptive command rail, keeping pane/layout labels and adding compact command labels where they fit.

## Chosen Approach

Reuse the existing desk row. Medium gets a compact `CMD g / p s t d z sp ? q` rail before visible/hidden pane labels. Standard-wide keeps pane tabs first, then adds the compact command rail before state/read-only text.

## Validation Notes

- Focused Ratatui header tests passed for medium layout-director visibility and standard-wide desk rails.
- CLI mouse tests passed for ultra-wide, medium, standard-wide, narrow, and bottom action-strip command controls.
- Full `ratatui_cockpit`, `workstation_interaction`, `hls-cli live_tui`, `live_mock`, build, clippy with warnings denied, and fixture TUI smoke passed.
