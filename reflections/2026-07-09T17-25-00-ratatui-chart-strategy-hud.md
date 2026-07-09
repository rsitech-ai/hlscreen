# Reflection Entry

## Task
- **ID/Title:** Ratatui chart strategy HUD
- **Date:** 2026-07-09
- **Scope:** multi-file

## Plan and Risks
- **Planned approach:** Add a focused-chart behavior test, then render a compact strategy HUD in the normal focused chart that synthesizes trend, flow, liquidity gate, confidence, and read-only safety without requiring expanded zoom mode.
- **Top failure hypotheses:** The HUD may imply trading advice if labels are careless; extra chart lines may crowd candle rendering; color assertions may become brittle.
- **Success criteria:** A focused 160-column chart renders `STRATEGY HUD`, signal/bias context, liquidity/flow gates, confidence, and explicit `watch only`, `no orders`, and `not advice` language. Existing expanded chart intelligence stays intact.

## Candidate Attempts
| Candidate | Summary | Outcome | Signals | Why selected / rejected |
|---|---|---|---|---|
| A | Put more strategy text only inside expanded chart zoom | Rejected | Would hide the cockpit-level strategy context behind `z` | The normal focused chart should feel like the primary workstation |
| B | Add a compact focused-chart strategy HUD | Selected | Keeps the chart tactical without changing ingestion or execution boundaries | Advances next-gen visual UX while staying read-only |

## Reflection
- **Failure modes observed:** The initial red test correctly failed on missing `STRATEGY HUD`. A setup retry showed the internal venue symbol `@107` must be used for candle matching even though the display label is `HYPE/USDC`. The first implementation also clipped the `not advice` safety text until it was split onto its own line.
- **Root cause:** The focused chart had edge/candle context but no compact synthesis layer, and the renderer separates venue identifiers from display symbols for metadata-backed rows.
- **Fix that resolved it:** Added a focused-chart strategy HUD at the existing chart rail breakpoint, using venue-matched candles, signal/bias synthesis, liquidity and flow gates, and a separate safety line.
- **What improved score/quality:** The normal focused chart now carries the hedge-fund-workstation signal context without requiring zoom, while explicitly staying in `watch only`, `no orders`, `not advice` territory.
- **Useful command-level evidence:** `cargo fmt --check`; `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit focused_chart_renders_strategy_hud_without_execution_language -- --nocapture`; `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit chart -- --nocapture`; `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction --test ratatui_cockpit -j 1`; `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`; `cargo test -p hls-cli --test live_mock`; `cargo build -p hls-cli`; fixture-backed `hls live --once --tui --color always` smoke; `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`; `git diff --check`.
- **Branch comparison insight (if multiple attempts):** Not applicable.

## Reusable Lesson
- **Pattern that worked:** Keep compact synthesis lanes short and put safety language on its own line when the terminal layout is dense.
- **Pattern to avoid:** Testing display labels where the renderer intentionally matches on venue identifiers for live market events.
- **Where to apply next:** Add similar compact synthesis lanes to book/tape if they still require zoom for high-level interpretation.

## Decision
- **Final chosen approach:** Focused chart strategy HUD for normal cockpit mode, with expanded chart intelligence left intact.
- **Commit/rollback decision:** Commit and push after clean ancestry check.
- **Next step / follow-up:** Continue adding compact synthesis lanes to book/tape/status where they improve live interpretation without expanding into execution.
