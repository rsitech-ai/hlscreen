# Production Docs And Live Readiness Refresh

## Task
- **ID/Title:** 2026-07-08 production docs and live readiness refresh
- **Date:** 2026-07-08
- **Scope:** current all-symbol live validation plus production/open-source documentation refresh

## Plan and Risks
- **Planned approach:** Validate current all-available public spot data with a bounded all-symbol live run, replay/screen the capture, generate TUI screenshot evidence, then update README/docs/architecture with diagrams and truthful production-readiness language.
- **Top failure hypotheses:** Public payloads drift from parser assumptions; docs overstate readiness beyond the read-only production boundary; architecture docs lag current crates; screenshots are deterministic but not tied clearly enough to live proof.
- **Success criteria:** Current all-symbol run exits cleanly, replay/screen/health outputs are consistent, docs include architecture diagrams and current runbooks, full gates pass, and remaining production blockers are explicit.

## Candidate Attempts
| Candidate | Summary | Outcome | Signals | Why selected / rejected |
|---|---|---|---|---|
| A | Update docs using the previous all-data audit only. | Rejected. | Prior proof is strong but stale for this request. | The operator asked to validate all available data now. |
| B | Run fresh all-symbol proof first, then rewrite docs from that evidence. | Selected. | Current live data anchors the production-readiness claim. | Best match for live/no-mock/open-source readiness. |

## Reflection
- **Failure modes observed:** The primary all-symbol run itself was stable, but the TUI quality label overclaimed visible table quality when some rows had no spread or top-of-book depth evidence. The header also used `p95 local`, which read like local compute latency even though it was row freshness.
- **Root cause:** `TableStats::quality_status` only evaluated available spread/depth values and ignored coverage gaps across the visible rows. The age label came from an older renderer naming choice, not the current metric semantics.
- **Fix that resolved it:** Track row count plus spread/depth coverage in `TableStats`, return `partial` whenever visible rows lack required quote/depth evidence, add a regression test for mixed coverage, and rename the header to `p95 row age`.
- **What improved score/quality:** Docs now label readiness as read-only local release-candidate state, diagrams show exact boundaries, and the TUI communicates data coverage without pretending sparse microstructure evidence is complete.
- **Useful command-level evidence:** 300s all-symbol live run `allpairs-prodreadiness-20260708-201752` captured 308 symbols, 924 subscriptions, 99,162 WS messages, 106,980 normalized events, clean shutdown, 0 reconnects, and 0 data gaps. Replay parity passed on the second run. Post-fix 60s all-symbol smoke showed `p95 row age` and `quality partial`.
- **Branch comparison insight (if multiple attempts):** Previous audit branches already proved the read-only data path; this branch adds current production-readiness docs, architecture diagrams, and a narrower TUI truthfulness fix rather than reworking ingestion architecture.

## Reusable Lesson
- **Pattern that worked:** Anchor production docs in a fresh live run, replay parity, screenshots, health probes, negative probes, and full validation before changing readiness language.
- **Pattern to avoid:** Do not let display aggregates imply quality from the subset of fields that happen to be present; coverage gaps are part of the quality state.
- **Where to apply next:** Future TUI metrics, Parquet support, long-running server mode, and deploy runbooks should use the same evidence-first readiness labels.

## Decision
- **Final chosen approach:** Fresh live proof, docs refresh, diagrammed architecture, and full validation.
- **Commit/rollback decision:** Commit after final focused checks pass; rollback is safe because no exchange/account state is mutated and all changes are local docs/TUI/test artifacts.
- **Next step / follow-up:** Publish the branch/PR if final checks remain green, then watch GitHub checks before any merge claim.
