## Task
- **ID/Title:** Ratatui expanded Detail instrument dossier
- **Date:** 2026-07-09
- **Scope:** multi-file

## Plan and Risks
- **Planned approach:** Add one behavior-first cockpit test for a zoomed Detail pane that renders an instrument dossier, then implement it from existing public metadata and screen fields.
- **Top failure hypotheses:** The dossier could imply account/private metadata, duplicate the existing quote terminal, or crowd the expanded Detail pane.
- **Success criteria:** Expanded Detail shows an instrument dossier with cohort/tags, listing/seeded/source/feed id, confidence/freshness, and explicit public metadata/no-wallet/no-order labels; validation remains green.

## Candidate Attempts
| Candidate | Summary | Outcome | Signals | Why selected / rejected |
|---|---|---|---|---|
| A | Add expanded-only instrument dossier to Detail overview | Selected | Adds a terminal-style symbol profile without changing data contracts | Best fit for long-form pair detail |
| B | Add private account/position profile | Rejected | No private streams, wallet, or account state exist or belong in the read-only TUI | Violates safety boundary |

## Reflection
- **Failure modes observed:** The targeted test first failed because expanded Detail did not render `INSTRUMENT DOSSIER`; after implementation, formatting, TUI, CLI, smoke, and clippy gates stayed green.
- **Root cause:** Expanded Detail already exposed quotes, factor stack, liquidity radar, and alpha stack, but lacked a terminal-style symbol profile tying public metadata to the selected instrument.
- **Fix that resolved it:** Added an expanded Detail-only instrument dossier from existing metadata, feed id, listing/seeded/source, confidence, freshness, and display symbol fields.
- **What improved score/quality:** The Detail pane now reads more like a professional instrument profile while retaining screen-only/public metadata/no-wallet/no-order labels.
- **Useful command-level evidence:** Red test: `cargo test -p hls-tui --test ratatui_cockpit expanded_detail_renders_instrument_dossier -- --nocapture`; green checks: `cargo fmt --check`; `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction --test ratatui_cockpit -j 1`; `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`; `cargo test -p hls-cli --test live_mock`; fixture `hls live --fixture-file ... --once --tui --color never` smoke; `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`.
- **Branch comparison insight (if multiple attempts):** Not applicable.

## Reusable Lesson
- **Pattern that worked:** Add high-density symbol profile detail only in expanded mode, and make the provenance labels part of the pane copy.
- **Pattern to avoid:** Do not let "dossier" language imply private account or exchange action state.
- **Where to apply next:** Symbol detail surfaces that combine public metadata with market microstructure evidence.

## Decision
- **Final chosen approach:** Expanded-only Detail instrument dossier using public metadata and selected-row screen fields.
- **Commit/rollback decision:** Commit after final diff and remote-drift checks; validation is green.
- **Next step / follow-up:** Continue improving adaptive polish and high-density terminal surfaces while keeping normal layouts stable.
