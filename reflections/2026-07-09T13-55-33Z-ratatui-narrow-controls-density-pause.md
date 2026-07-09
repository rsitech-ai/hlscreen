# Ratatui narrow controls density pause

## Success Criteria

- Narrow terminals expose density and pause in the top compact control rail.
- Mouse clicks on the visible `d` and `sp` compact controls map to density and pause.
- Existing compact controls still map to filter, preset, sort, chart window, zoom, status, help, and quit.
- The 72-column and 80-column layouts remain resize-safe.

## Failure Hypotheses

1. Adding labels could cause narrow header wrapping or hidden controls.
2. Dense one-character labels could drift from their mouse offsets.
3. The `sp` two-character pause label could collide with `s sort`.

## Candidate Approaches

- Use a longer descriptive narrow control rail.
- Keep the dense rail and add `d` plus `sp`.

## Chosen Approach

Use the dense `/pstdzsp h? q` rail. It fits the narrow layout and keeps every advertised control directly clickable.

