## Task
- **ID/Title:** Ratatui expanded Book liquidity wall monitor
- **Date:** 2026-07-09
- **Scope:** multi-file

## Plan and Risks
- **Planned approach:** Add one behavior-first test for a zoomed Book pane that renders a bid/ask liquidity wall monitor, then implement it from existing public BBO notional, spread, imbalance, OFI, and quality fields.
- **Top failure hypotheses:** The added lines could crowd the expanded Book pane, duplicate existing depth-map text, or imply executable order-book depth beyond the top-book proxy.
- **Success criteria:** Expanded Book shows bid/ask wall shares, notional, spread, skew, OFI, quality state, and explicit public top-book/no-order labels; existing Ratatui and live CLI validation stays green.

## Candidate Attempts
| Candidate | Summary | Outcome | Signals | Why selected / rejected |
|---|---|---|---|---|
| A | Add expanded-only liquidity wall monitor below current depth map | Selected | Improves the order-book visual surface without changing normal layouts | Best fit for the screenshot-like bid/ask monitor |
| B | Simulate multi-level L2 ladder | Rejected | Current authoritative data is BBO/top-book proxy, not full public L2 reconstruction | Would overstate evidence |

## Reflection
- **Failure modes observed:** No validation failures after adding the expanded-only monitor. The main risk stayed semantic: the UI must label this as public BBO/top-book evidence, not reconstructed L2 depth.
- **Root cause:** The expanded Book pane already had depth-map and queue-share surfaces, but lacked a compact bid/ask wall readout with spread, OFI, and microprice-edge context.
- **Fix that resolved it:** Added `expanded_book_renders_liquidity_wall_monitor` and implemented an expanded Book-only `LIQUIDITY WALL` section sourced from existing BBO notional/share, spread, OFI proxy, and microprice/mid fields.
- **What improved score/quality:** The Book drilldown now has screenshot-style bid/ask wall context while preserving read-only/no-orders provenance labels and leaving normal non-expanded layouts unchanged.
- **Useful command-level evidence:** `cargo fmt --check`; `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction --test ratatui_cockpit -j 1`; `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`; `cargo test -p hls-cli --test live_mock`; fixture `hls live --fixture-file ... --once --tui --color never` smoke; `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`.
- **Branch comparison insight (if multiple attempts):** Not applicable.

## Reusable Lesson
- **Pattern that worked:** Keep high-density workstation additions expanded-pane-only, test the rendered public text, and state data provenance in the pane itself when the visual language resembles an order-book tool.
- **Pattern to avoid:** Do not render synthetic L2 depth as if it were exchange order-book depth when only BBO proxy fields are available.
- **Where to apply next:** Other order-flow surfaces where public-data provenance must stay explicit.

## Decision
- **Final chosen approach:** Add an expanded-only Book liquidity wall monitor with top-book provenance labels.
- **Commit/rollback decision:** Commit after final diff/drift checks; validation is green.
- **Next step / follow-up:** Continue the same Ratatui rewrite with another expanded-pane or adaptive-layout slice rather than widening this change.
