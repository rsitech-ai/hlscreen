## Task
- **ID/Title:** Ratatui expanded Status portfolio risk terminal
- **Date:** 2026-07-09
- **Scope:** multi-file

## Plan and Risks
- **Planned approach:** Add one behavior-first cockpit test for a zoomed Status pane that renders a read-only portfolio-risk terminal, then implement it from existing cross-pair screen rows.
- **Top failure hypotheses:** The new copy could imply real account positions, crowd the expanded Status pane, or duplicate the existing regime/quality lines without adding a clearer workstation affordance.
- **Success criteria:** Expanded Status shows breadth, flow skew, concentration, degraded/stale gates, and explicit screen-only/no-position/no-order labels; existing Ratatui and live CLI checks stay green.

## Candidate Attempts
| Candidate | Summary | Outcome | Signals | Why selected / rejected |
|---|---|---|---|---|
| A | Add expanded-only portfolio risk terminal to Status | Selected | Brings a hedge-fund workstation affordance without changing data contracts | Best fit for the user-visible portfolio/workstation target |
| B | Add real portfolio/PnL fields | Rejected | No wallet/private streams/positions exist or should exist in this read-only TUI | Would violate the capital boundary |

## Reflection
- **Failure modes observed:** The first targeted test failed because the expanded Status pane did not render `PORTFOLIO RISK TERMINAL`; after implementation, no validation failures remained.
- **Root cause:** Status already exposed regime, latency, signal, quality, color, and ops gates, but lacked an explicit cross-pair risk/exposure terminal aligned with the workstation target.
- **Fix that resolved it:** Added an expanded Status-only portfolio risk terminal sourced from existing screened rows: breadth, signed-flow skew, top-book depth concentration, confidence/staleness/spread degradation, and a screen-only gate.
- **What improved score/quality:** The Status pane now feels more like a command workstation while preserving the read-only boundary with `screen exposure only`, `no positions`, `no orders`, and `not advice` copy.
- **Useful command-level evidence:** Red test: `cargo test -p hls-tui --test ratatui_cockpit expanded_status_renders_portfolio_risk_terminal -- --nocapture`; green checks: `cargo fmt --check`; `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction --test ratatui_cockpit -j 1`; `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`; `cargo test -p hls-cli --test live_mock`; fixture `hls live --fixture-file ... --once --tui --color never` smoke; `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`.
- **Branch comparison insight (if multiple attempts):** Not applicable.

## Reusable Lesson
- **Pattern that worked:** Use expanded panes for high-density hedge-fund-style affordances and pair the visual language with explicit provenance/safety labels.
- **Pattern to avoid:** Do not use portfolio language without explicit screen-only/no-position/no-order labels.
- **Where to apply next:** Any dashboard surface that borrows hedge-fund terminal language while remaining public-data only.

## Decision
- **Final chosen approach:** Expanded-only Status portfolio risk terminal with screen-only/no-position/no-order labeling.
- **Commit/rollback decision:** Commit after final diff and remote-drift checks; validation is green.
- **Next step / follow-up:** Continue with another visible Ratatui workstation slice, likely either more adaptive command control or richer chart/tape expanded surfaces.
