## Task
- **ID/Title:** Ratatui expanded Detail quote terminal
- **Date:** 2026-07-09
- **Scope:** multi-file

## Plan and Risks
- **Planned approach:** Add one behavior-first test for a zoomed Detail pane that renders a selected-pair quote terminal using existing public BBO, flow, score, and confidence fields, then implement the smallest rendering helper behind the existing Detail surface.
- **Top failure hypotheses:** The new deck could duplicate existing alpha/liquidity lines, crowd adaptive layouts, or blur the read-only/no-orders boundary.
- **Success criteria:** Expanded Detail shows a workstation-style selected-instrument terminal with bid/ask, spread, top-book depth, flow, score, confidence, and explicit public/read-only labels; existing Ratatui and live CLI tests remain green.

## Candidate Attempts
| Candidate | Summary | Outcome | Signals | Why selected / rejected |
|---|---|---|---|---|
| A | Extend existing Detail pane only when zoomed/focused | Selected | Keeps normal layouts stable while improving the screenshot-like selected-instrument surface | Best fit for adaptive UI and current pane architecture |
| B | Add a new dedicated Quote pane | Rejected | Would add navigation and state surface before proving value | Too much interface churn for one slice |

## Reflection
- **Failure modes observed:** The first focused test expected a hardcoded top-book notional that did not match the generated fixture snapshot.
- **Root cause:** The fixture-derived selected row computes depth from current mock market state, so the UI contract should assert the top-book field is present rather than pinning a stale amount.
- **Fix that resolved it:** Kept the behavior assertion on `top book $` and verified the quote terminal labels for bid, ask, spread, flow, confidence, public-data provenance, and no-order safety.
- **What improved score/quality:** Zoomed Detail now behaves like a selected-instrument terminal instead of only a compact explanatory panel.
- **Useful command-level evidence:** `cargo test -p hls-tui --test ratatui_cockpit expanded_detail_renders_quote_terminal_deck -- --nocapture`; `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction --test ratatui_cockpit -j 1`; `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`; `cargo test -p hls-cli --test live_mock`; `cargo fmt --check`; `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`; fixture `hls live --once --tui` smoke.
- **Branch comparison insight (if multiple attempts):** Not applicable.

## Reusable Lesson
- **Pattern that worked:** Add quote-workstation affordances inside the existing focused/expanded pane path, then keep normal/narrow layouts covered by existing adaptive tests.
- **Pattern to avoid:** Duplicating live-data semantics in a new pane when an existing Detail drilldown can carry the selected-instrument workflow.
- **Where to apply next:** Expanded panes that map directly to the user-visible workstation screenshot.

## Decision
- **Final chosen approach:** Extend expanded Detail with a quote terminal deck.
- **Commit/rollback decision:** Commit after full Ratatui, CLI, clippy, and fixture smoke validation.
- **Next step / follow-up:** Continue hardening expanded panes and generated screenshot truth for the final workstation-level audit.
