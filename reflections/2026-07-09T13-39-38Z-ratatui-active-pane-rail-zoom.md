# Ratatui active pane rail zoom

## Success Criteria

- Clicking an inactive pane rail still focuses that pane.
- Clicking the already-active pane rail toggles pane expansion, matching the keyboard `z` control.
- The behavior works across wide command strips, desk rails, and narrow compact rails.
- Command entry continues to ignore mouse input.
- Help copy makes the active-pane click behavior discoverable without hiding the read-only safety line.

## Failure Hypotheses

1. Hit testing for the wide top command strip and the desk/controls rails could diverge because they use different helper paths.
2. Adding words to the help overlay could break narrow 72-column rendering.
3. Mapping active-pane clicks to zoom could accidentally affect non-pane actions such as search/help/quit.

## Candidate Approaches

- Add a new action for "focus or zoom pane" and let state decide.
- Keep the state model unchanged and make the mouse mapper choose either `FocusPane` or `TogglePaneZoom`.

## Chosen Approach

Use the existing `TogglePaneZoom` action from the mouse mapper. This keeps the state machine small and avoids duplicating zoom behavior.

