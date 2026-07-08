# Reflection: Next-Gen Keyboard-Interactive TUI

## Success Criteria
- `hls live --tui` gains real keyboard controls for display focus/view state without adding trading or private-data behavior.
- Stable renderer output remains deterministic enough for golden tests and SVG screenshots.
- Live WebSocket ingestion remains bounded, read-only, and non-blocking.

## Top Failure Hypotheses
1. Terminal event polling blocks or destabilizes the live WebSocket loop.
2. The UI becomes visually richer by inventing metrics that are not present in `FeatureSnapshot`.
3. Snapshot and smoke tests become flaky because interactive state leaks into non-TTY fixture runs.

## Candidate Approaches
1. Full Ratatui alternate-screen application. Strong long-term UI substrate, but too much blast radius for this repo’s deterministic screenshot and CI contract.
2. Pure interaction state in `hls-tui` with optional live keyboard polling in `hls-cli`. Smaller, testable, and compatible with current renderer.

## Chosen Approach
- Use approach 2 for this slice. Keep full widget-grid Ratatui as a later rewrite only after the state/actions and live loop semantics are stable.

## Attempt Log
- Start: branch `feat/andrzej_nextgen_interactive_tui`; scope is read-only public market-data TUI only.
- Implemented pure interaction state in `hls-tui`, renderer hooks, and live CLI key polling with direct Crossterm raw mode only for real TTYs.
- Found a live-progress symbol display issue during public smoke: progress frames showed `@107` until final output attached metadata. Fixed by passing public metadata into live progress rendering.
- Regenerated SVG screenshots and previewed the main TUI PNG at `/tmp/hlscreen-nextgen-tui-preview/live-screen.png`.

## Closeout
- Worked: state-machine-first approach kept tests deterministic and let the live loop remain the data source.
- Failed/changed: the first command rail was too verbose and truncated; fixed with compact labels. The first live smoke exposed metadata parity drift between progress and final frames.
- Reuse: keep future TUI enhancements behind pure state/action tests first, then wire terminal input in `hls-cli`.
- Validation: focused TUI/CLI tests, full workspace fmt/clippy/tests/builds, release packaging check, screenshot generation, diff check, and real public `hype-usdc` live smoke passed.
