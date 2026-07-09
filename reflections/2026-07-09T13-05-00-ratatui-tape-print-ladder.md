## Task
- **ID/Title:** Ratatui expanded Tape public print ladder
- **Date:** 2026-07-09
- **Scope:** multi-file

## Plan and Risks
- **Planned approach:** Add one behavior-first cockpit test for a zoomed Tape pane that renders a public print ladder, then implement it from existing public trade events.
- **Top failure hypotheses:** The ladder could imply private fills, synthetic order flow, or execution recommendations; it could also crowd the expanded Tape pane if too verbose.
- **Success criteria:** Expanded Tape shows price-level print ladder, side pressure, notional, largest print, toxicity proxy, and explicit public-trades/no-fills/no-orders labels; validation remains green.

## Candidate Attempts
| Candidate | Summary | Outcome | Signals | Why selected / rejected |
|---|---|---|---|---|
| A | Add expanded-only public print ladder after time-and-sales board | Selected | Closest to screenshot trade rail while preserving public-data provenance | Best visible TUI improvement for tape pane |
| B | Add private fill/order route semantics | Rejected | No private streams or wallet data exist or belong here | Violates read-only boundary |

## Reflection
- **Failure modes observed:** The targeted test first failed because expanded Tape did not render `PUBLIC PRINT LADDER`; the first format gate then caught one formatting-only issue in the new helper.
- **Root cause:** Expanded Tape had time-and-sales summary and recent public trades, but lacked a price-level ladder that visually matched the screenshot's trade rail.
- **Fix that resolved it:** Added an expanded Tape-only print ladder from existing public `TradeEvent` rows, grouped by price level with buy/sell notional, largest print, and a toxicity proxy.
- **What improved score/quality:** The Tape pane now has a more terminal-grade public prints rail while explicitly retaining `public trades only`, `no fills`, and `no orders` labels.
- **Useful command-level evidence:** Red test: `cargo test -p hls-tui --test ratatui_cockpit expanded_tape_renders_public_print_ladder -- --nocapture`; green checks: `cargo fmt --check`; `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction --test ratatui_cockpit -j 1`; `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`; `cargo test -p hls-cli --test live_mock`; fixture `hls live --fixture-file ... --once --tui --color never` smoke; `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`.
- **Branch comparison insight (if multiple attempts):** Not applicable.

## Reusable Lesson
- **Pattern that worked:** Price-level tape summaries can be useful and visually dense as long as they are derived only from public prints and keep no-fill/no-order provenance visible.
- **Pattern to avoid:** Do not let time-and-sales visuals imply private fills or executable flow.
- **Where to apply next:** Tape and chart trade-marker surfaces where public trade provenance must remain visible.

## Decision
- **Final chosen approach:** Expanded-only public print ladder under the Tape pane's time-and-sales board.
- **Commit/rollback decision:** Commit after final diff and remote-drift checks; validation is green.
- **Next step / follow-up:** Continue with adaptive polish and richer terminal surfaces while keeping normal layouts stable.
