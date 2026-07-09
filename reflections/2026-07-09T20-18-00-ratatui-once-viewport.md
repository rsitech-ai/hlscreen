# Ratatui Once Viewport Slice

## Success Criteria
- `hls live --fixture-file ... --once --tui` renders with the detected terminal size instead of a hardcoded 160x48 viewport.
- Non-terminal or failed terminal-size detection keeps a conservative fallback so fixture smoke remains reproducible.
- The change does not weaken read-only TUI safety labels or live TUI behavior.

## Failure Hypotheses
- Switching the fixture path to terminal size could make existing CLI tests brittle in non-TTY CI.
- A too-small fallback could hide the main workstation regions and break smoke tests.
- The change could accidentally affect the interactive alternate-screen path, which already uses Ratatui frame size directly.

## Candidate Approaches
- Add an injectable viewport resolver and use it from both the existing runtime helper and the fixture once path.
- Add a visible CLI flag for fixture viewport sizing, but that adds public API surface when terminal auto-detection is the expected behavior.

## Attempt Log
- Starting with unit coverage for viewport resolution, then replacing the hardcoded fixture `Some(160x48)` with the adaptive resolver.
- Focused signal: the viewport helper accepted a 240x64 terminal and a 160x48 fallback, but the CLI smoke exposed a zero-size non-TTY report. The helper now treats zero dimensions as invalid and falls back conservatively.
- Integration signal: the fixture TUI smoke now accepts adaptive narrow or wide Ratatui shell markers instead of requiring the old fixed 160-column header and panes.
- Final verification passed: fmt, Ratatui cockpit/workstation tests, hls-cli live TUI tests, live mock, build, adaptive ANSI fixture smoke, clippy, and diff hygiene.
