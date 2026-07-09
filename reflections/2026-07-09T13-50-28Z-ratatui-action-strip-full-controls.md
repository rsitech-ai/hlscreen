# Ratatui action strip full controls

## Success Criteria

- The bottom action strip advertises every core live display control, including density and pause.
- Mouse clicks on the visible `d density` and `space pause` labels map to the same actions as the keyboard.
- Wide and medium status bars keep read-only/safety and theme information visible.
- Command entry still ignores action-strip mouse clicks.

## Failure Hypotheses

1. Adding labels could push useful status content off narrower medium terminals.
2. Existing coordinate tests for later labels could drift after inserting new controls.
3. Pause and density could be exposed visually but not wired to the mouse mapper.

## Candidate Approaches

- Add density and pause to the rendered action copy and mouse label table.
- Keep the visual strip unchanged and only add hidden mouse zones.

## Chosen Approach

Update both visible text and mouse hit mapping. The TUI should not have hidden controls; the user should be able to click what they can see.

