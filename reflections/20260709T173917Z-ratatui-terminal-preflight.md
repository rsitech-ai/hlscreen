## Task

Make the live Ratatui cockpit visibly self-identify as the unified colored TUI path so a stale binary, old renderer, or monochrome terminal downgrade is obvious from the first screen.

## Success Criteria

- The active cockpit includes a first-screen terminal/color preflight marker in wide and medium layouts.
- Color mode still obeys existing `--color always|auto|never` semantics.
- No wallet/private stream/order route semantics change.
- Focused TUI and live CLI tests pass.

## Failure Hypotheses

1. The user's black-and-white screenshot is from a stale installed `hls` binary rather than the current `target/debug/hls`.
2. The current cockpit contains color diagnostics, but they are too hidden in help/status panels to diagnose the active path quickly.
3. Adding more header text can overflow compact terminals if not gated by viewport width.

## Candidate Approaches

- Add a compact first-screen `TERMINAL PREFLIGHT`/`UNIFIED RATATUI` strip to the existing header/status rail.
- Add a separate diagnostic command or help-only text.

Chosen approach: reuse the existing header/status rail, because it is already adaptive and visible during normal live use.
