# Reflection Entry

## Task
- **ID/Title:** Ratatui selected-pair route deck
- **Date:** 2026-07-09
- **Scope:** multi-file

## Plan and Risks
- **Planned approach:** Add one behavior test for an expanded detail panel route deck, then implement the smallest renderer helper that ties the selected pair to keyboard routes across detail/chart/book/tape/status.
- **Top failure hypotheses:** The detail panel becomes too tall and hides existing content; route copy becomes decorative instead of tied to actual hotkeys; compact terminals regress due to long strings.
- **Success criteria:** The expanded detail pane exposes a clear pair route deck, keeps read-only boundaries visible, and existing cockpit/workstation tests still pass.

## Candidate Attempts
| Candidate | Summary | Outcome | Signals | Why selected / rejected |
|---|---|---|---|---|
| A | Add the route deck only in expanded detail overview. | selected | Preserves compact layouts and focuses on the operator deep-dive surface. | Best fit for the user's workstation target without crowding the default grid. |
| B | Add route copy to every detail view. | rejected | More visible but risks repetitive text and weaker per-view signal. | Too much surface area for this slice. |

## Reflection
- **Failure modes observed:** The initial focused test failed because the expanded detail pane had quote/instrument context but no selected-pair route deck.
- **Root cause:** Existing detail content described the instrument but did not map the selected pair to the keyboard/operator routes across chart, book, tape, and status.
- **Fix that resolved it:** Added an expanded-detail-only route deck with adaptive compact/wide labels and a read-only boundary line.
- **What improved score/quality:** The selected pair now has an explicit action map without adding execution language or crowding the default non-expanded detail panel.
- **Useful command-level evidence:** Focused red/green test, full `ratatui_cockpit`, `workstation_interaction`, `live_tui`, `live_mock`, `cargo fmt --check`, `git diff --check`, `cargo build -p hls-cli`, and clippy all passed.
- **Branch comparison insight (if multiple attempts):** Not applicable.

## Reusable Lesson
- **Pattern that worked:** Pending validation.
- **Pattern to avoid:** Pending validation.
- **Where to apply next:** Other expanded panes can use route-aware decks when they expose genuine keyboard paths.

## Decision
- **Final chosen approach:** Expanded-detail route deck behind a focused behavior test.
- **Commit/rollback decision:** Commit and push after green validation.
- **Next step / follow-up:** Continue hardening other expanded panes only where the deck maps to real keyboard actions and live data.
