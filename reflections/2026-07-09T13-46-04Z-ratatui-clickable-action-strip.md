# Ratatui clickable action strip

## Success Criteria

- The wide and medium bottom `ACTION STRIP` is clickable for the commands it visibly advertises.
- Clicking action labels maps to the same safe display-only actions as keyboard shortcuts.
- Narrow terminals keep the existing compact status behavior without pretending unsupported controls are clickable.
- Command entry continues to ignore mouse events.
- Read-only boundaries stay visible and unchanged.

## Failure Hypotheses

1. The mouse row for the status bar could be off by one because the rendered block uses only a top border.
2. The wide and medium action copy differs, so one coordinate helper could silently mismatch one layout.
3. Adding hit detection after generic pane fallback would make it unreachable.

## Candidate Approaches

- Reuse a label-hit helper with explicit action labels matching the rendered status text.
- Infer actions from individual characters in the status line.

## Chosen Approach

Use explicit labels in the same order as the rendered `ACTION STRIP`, and check the bottom action row before the generic pane fallback.

