## Task
- **ID/Title:** Ratatui expanded Chart tactical matrix
- **Date:** 2026-07-09
- **Scope:** multi-file

## Plan and Risks
- **Planned approach:** Add one behavior-first cockpit test for a zoomed Chart pane that renders a tactical matrix, then implement it from existing public candles, BBO/trade metrics, confidence, and chart-window state.
- **Top failure hypotheses:** The matrix could crowd the expanded chart, overstate screen heuristics as a trade signal, or duplicate existing chart-intel/session-strip lines.
- **Success criteria:** Expanded Chart shows tactical regime, active window, trend, volatility, liquidity gate, flow gate, confidence, and explicit public/no-order/not-advice labels; normal chart layouts stay unchanged and validation remains green.

## Candidate Attempts
| Candidate | Summary | Outcome | Signals | Why selected / rejected |
|---|---|---|---|---|
| A | Add expanded-only tactical matrix above the candle chart | Selected | Strengthens the chart-centered workstation vibe while staying in public data | Best fit for the screenshot-style chart command surface |
| B | Add execution recommendations | Rejected | Would imply advice or trading instructions beyond the read-only screener boundary | Violates the capital/safety boundary |

## Reflection
- **Failure modes observed:** The targeted test first failed because expanded Chart did not render `TACTICAL MATRIX`; after adding the helper and reserving expanded-chart overhead, validation stayed green.
- **Root cause:** Expanded Chart already showed candles, prints, profile, crosshair context, and chart intel, but it lacked a compact tactical regime/gate matrix tied to the active chart window.
- **Fix that resolved it:** Added an expanded Chart-only tactical matrix sourced from public candles, BBO/depth/flow fields, confidence, and the active chart-window state.
- **What improved score/quality:** The chart pane now has a more algorithmic-trading workstation feel while explicitly labeling the output as public candles/BBO/trades only, with no orders and not-advice copy.
- **Useful command-level evidence:** Red test: `cargo test -p hls-tui --test ratatui_cockpit expanded_chart_renders_tactical_matrix -- --nocapture`; green checks: `cargo fmt --check`; `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction --test ratatui_cockpit -j 1`; `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`; `cargo test -p hls-cli --test live_mock`; fixture `hls live --fixture-file ... --once --tui --color never` smoke; `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`.
- **Branch comparison insight (if multiple attempts):** Not applicable.

## Reusable Lesson
- **Pattern that worked:** Expanded chart surfaces can carry high-density decision-support language when each line is backed by existing public fields and clearly marked as a screen heuristic.
- **Pattern to avoid:** Do not label chart heuristics as execution recommendations.
- **Where to apply next:** Other expanded chart and tape surfaces where high-density terminal language needs provenance labels.

## Decision
- **Final chosen approach:** Expanded-only Chart tactical matrix with active-window, trend, volatility, liquidity, flow, and confidence context.
- **Commit/rollback decision:** Commit after final diff and remote-drift checks; validation is green.
- **Next step / follow-up:** Continue with another visible workstation slice, likely denser tape/trades or adaptive command workflow polish.
