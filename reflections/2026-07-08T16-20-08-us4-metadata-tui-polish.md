# Reflection Entry

## Task
- **ID/Title:** US4 metadata-backed TUI polish and screenshots
- **Date:** 2026-07-08
- **Scope:** multi-file

## Plan and Risks
- **Planned approach:** Add optional public metadata enrichment to core snapshots, parse official Hyperliquid public metadata payloads, expose metadata fields to screen rules and presets, render tags in the deterministic TUI, and regenerate screenshots from the compiled CLI.
- **Top failure hypotheses:** Metadata parsing may become too brittle for partial public payloads; row filtering may treat missing metadata as a match; TUI polish may overclaim trading readiness or rely on static screenshot text.
- **Success criteria:** Missing metadata is explicit and fail-open; new-listing/fresh-liquidity presets work from row metadata; terminal output is visibly improved without private/order surfaces; screenshot SVGs are regenerated and inspected.

## Candidate Attempts
| Candidate | Summary | Outcome | Signals | Why selected / rejected |
|---|---|---|---|---|
| A | Only restyle `hls-tui` using existing snapshot fields. | Rejected | Would improve visuals but leave US4 unchecked and metadata-discovery screenshots unavailable. | Too shallow for current product request. |
| B | Implement metadata enrichment plus targeted TUI/screenshot polish. | Selected | Aligns with Spec Kit US4 and gives the UI new real information architecture. | Best balance of product value and bounded risk. |

## Reflection
- **Failure modes observed:** Initial `cargo fmt --check` found formatting drift; the first rendered metadata line exceeded the screenshot width; the plan final notes were briefly placed in an older section and corrected before commit.
- **Root cause:** Multi-crate metadata work touched wide output lines and a long append-only plan file with repeated `Final Notes` headings.
- **Fix that resolved it:** Ran `cargo fmt`, split selected-symbol metadata into two detail lines, tightened the metadata chip column to fit 152 columns, and corrected the US4 `PLAN.md` section before staging.
- **What improved score/quality:** Metadata became a shared row contract rather than renderer-only text; TUI now shows metadata coverage at header, row, and selected-detail levels; screenshots are generated from the compiled CLI and visually previewed.
- **Useful command-level evidence:** `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `python3 scripts/generate-screenshots.py`; `rsvg-convert docs/assets/screenshots/metadata-discovery.svg -o /tmp/hlscreen-tui-preview/metadata-discovery.png`; `git diff --check`.
- **Branch comparison insight (if multiple attempts):** Not applicable.

## Reusable Lesson
- **Pattern that worked:** Put new UI facts in shared snapshot models, then expose them through CLI, screen rules, store cache, and screenshots from the same path.
- **Pattern to avoid:** Do not add screenshot-only copy or static metadata strings to make the UI look richer; it creates false parity.
- **Where to apply next:** US5 metrics/benchmark/extension work should expose shared contracts first, then render them.

## Decision
- **Final chosen approach:** Metadata-backed deterministic TUI polish.
- **Commit/rollback decision:** Commit after PR review; rollback is a normal branch revert because schema changes are additive/local-only.
- **Next step / follow-up:** Open PR, wait for CI, merge only if stable; then continue with US5 operations/benchmark/metrics/extension tasks.
