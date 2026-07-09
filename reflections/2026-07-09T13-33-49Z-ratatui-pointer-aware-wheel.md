# Ratatui pointer-aware wheel controls

## Success Criteria

- Mouse wheel behavior follows the pane under the pointer instead of the previously focused pane.
- Watchlist/table wheel scrolling still moves selected rows.
- Detail and chart wheel scrolling switches views/timeframes without affecting the selected symbol.
- Command entry remains protected from accidental mouse input.
- Help copy explains pane-aware wheel support while keeping read-only safety visible on compact terminals.

## Failure Hypotheses

1. Pointer hit testing could disagree with the rendered adaptive layout at wide, medium, or narrow widths.
2. A new interaction action could bypass selected-row clamping or symbol-stickiness behavior.
3. Longer help copy could push the read-only safety line out of compact help overlays.

## Candidate Approaches

- Add a pane-scoped scroll action to the interaction state machine, then map mouse wheel location to that action.
- Emit `FocusPane` plus existing `Up`/`Down` as multiple actions from the CLI event mapper.

## Chosen Approach

Use one pane-scoped scroll action. It preserves the single-action event loop and keeps all UI behavior in `WorkstationUiState::apply`.

