# Reflection: End-to-End Release Audit

## Task
- **ID/Title:** hlscreen end-to-end release audit and ship gate
- **Date:** 2026-07-10
- **Scope:** repo-wide

## Plan and Risks
- **Planned approach:** Establish deterministic static and fixture baselines, verify version-specific contracts against primary documentation, exercise service, PTY, failure, and bounded live paths, then repair only reproduced defects with behavior-level tests before PR and post-merge proof.
- **Top failure hypotheses:** (1) display pause or render scheduling does not preserve a stable presentation while ingest continues; (2) terminal capability, resize, or shutdown paths can leak terminal state or cause visible redraw instability; (3) CLI/docs/runtime contracts have drifted, leaving supported workflows undiscoverable or incorrect.
- **Success criteria:** All required gates pass without unexplained warnings; live top-10 data and degraded-network paths behave safely; logs and process state are clean; every validated defect has regression coverage; the reviewed PR and post-merge `main` state are green.

## Candidate Attempts
| Candidate | Summary | Outcome | Signals | Why selected / rejected |
|---|---|---|---|---|
| A | Evidence-first vertical repairs: reproduce one behavior, add one failing test, apply the smallest fix, and rerun parent gates. | Selected and complete | Deterministic tests plus independent live/runtime evidence. | Selected because it preserves causality and reviewability. |
| B | Broad rewrite or styling pass before diagnosis. | Rejected | Large diff would obscure regressions and weaken rollback. | Rejected because the current product already has a mature runtime and the task is validation/hardening. |

## Reflection
- **Failure modes observed:** Explicit color was suppressed by Crossterm under inherited `NO_COLOR`; screenshot check mode rewrote rather than compared; run/file identities and registry paths could replace or escape evidence; non-finite and semantically invalid market values crossed parser boundaries; late/control events could regress state or mask a stalled feed; render payloads and trade history could grow without useful bounds; all-symbol fallback lost global price coverage; one compact-table test still expected the obsolete `liq` label.
- **Root cause:** Policy was resolved in one layer but overridden later by dependency-global state; CI trusted a command-line flag the script never parsed; local persistence treated operator-controlled identifiers as benign strings; liveness was measured as transport activity rather than usable data; presentation copied analytical history wholesale; docs and tests had drifted from current semantics.
- **Fix that resolved it:** Applied the resolved color policy inside the serialized draw; implemented read-only screenshot comparison; added strict run/path validation, canonical containment, create-new files, and append-only registry inserts; rejected invalid numeric inputs; ordered and bounded state; required market events for liveness; added tiered `allMids` planning and rolling outbound limits; capped TUI payloads; corrected tests/docs.
- **What improved score/quality:** Regression-first repairs kept each cause visible, while real PTY, bounded outage, top-10, all-symbol, and live TUI evidence closed gaps that unit tests could not.
- **Useful command-level evidence:** 335 workspace tests; strict Clippy, release build, warning-free rustdoc, RustSec scan over 346 locked dependencies, 9 screenshot comparisons, 4 packaging tests, three consecutive 5/5 PTY runs, 35 us deterministic benchmark, top-10 204/456 messages/events, all-symbol 2,013/7,267, and one-clear live TUI restoration proof.
- **Branch comparison insight (if multiple attempts):** The clean branch starts exactly at current `origin/main`; unrelated primary-checkout changes remain isolated.

## Reusable Lesson
- **Pattern that worked:** Test terminal policy at the final byte-emission boundary, and distinguish protocol activity from domain-valid market data in every liveness decision.
- **Pattern to avoid:** Treating a clean screenshot or one short live run as proof of terminal correctness.
- **Where to apply next:** Future TUI release and market-data integration gates.

## Decision
- **Final chosen approach:** Candidate A, with explicit deterministic, runtime, live, PR, and post-merge gates.
- **Commit/rollback decision:** Commit only reviewed audit fixes; rollback remains the unchanged base `7ec3328` and isolated worktree removal.
- **Next step / follow-up:** Open and review the PR, require green CI, merge, then reinstall and rerun the bounded post-merge command/live proof from `main`.
