# Reflection Entry

## Task
- **ID/Title:** Ratatui status bar quality alert
- **Date:** 2026-07-09
- **Scope:** multi-file

## Plan and Risks
- **Planned approach:** Promote the status drilldown's data-quality idea into the always-visible wide status bar by showing the worst degraded/stale row as a compact `QUALITY ALERT` segment.
- **Top failure hypotheses:** The status bar may wrap too aggressively on medium widths; alert ranking could be unstable; forced-color checks could match unrelated warning text.
- **Success criteria:** Wide status bar exposes the worst degraded/stale symbol in plain mode, semantic alert styling is proven in forced-color snapshots, medium compact status remains stable, and standard TUI/CLI validation passes.

## Candidate Attempts
| Candidate | Summary | Outcome | Signals | Why selected / rejected |
|---|---|---|---|---|
| A | Keep degraded symbols only in the status focus pane. | Rejected. | The operator must focus status before seeing the concrete row. | Too hidden for a live cockpit. |
| B | Add a compact alert to the wide always-visible status bar. | Selected. | Builds on existing aggregate `RISK STRIP` and status ticker. | Improves live monitoring without changing ingestion or trading behavior. |

## Reflection
- **Failure modes observed:** The first full cockpit run failed because the alert fired on row age alone and crowded out the existing `degraded00` risk-strip assertion in a normal ticker snapshot.
- **Root cause:** The wide status bar has limited horizontal budget; an always-visible alert should be reserved for low-confidence or stale rows, while age-only diagnostics belong in the focused status panel.
- **Fix that resolved it:** Tightened the status-bar alert predicate to confidence below 70 or non-fresh staleness only, preserving existing risk-strip visibility.
- **What improved score/quality:** The default live surface now exposes concrete stale/low-confidence symbols without requiring status focus, while normal healthy rows do not create noisy alerts.
- **Useful command-level evidence:** `cargo fmt --check`; `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction --test ratatui_cockpit -j 1`; `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`; `cargo test -p hls-cli --test live_mock`; `cargo build -p hls-cli`; forced-color fixture smoke; `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`; `git diff --check`.
- **Branch comparison insight (if multiple attempts):** Not applicable.

## Reusable Lesson
- **Pattern that worked:** Promote critical drilldown diagnostics to the always-visible bar only when the condition is actionable enough to spend horizontal budget.
- **Pattern to avoid:** Triggering top-level alerts from age-only heuristics when aggregate quality is otherwise clean.
- **Where to apply next:** Any future global alert strip or command bar status indicator.

## Decision
- **Final chosen approach:** Width-gated status-bar quality alert for stale or low-confidence rows, with deterministic worst-row selection and semantic colors.
- **Commit/rollback decision:** Commit and push after final git hygiene.
- **Next step / follow-up:** Continue improving live surfaces that are visible without pane focus.
