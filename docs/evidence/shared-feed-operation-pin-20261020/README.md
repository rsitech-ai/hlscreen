# Actual-runner capture source coordination — 2026-10-20

Both direct dependencies and all five resolved marketfeed packages move from
`c8c1378` to producer `ba512ad1a1bbd0447c49eddedd2277d23ea9191b` together.
All 419 complete registry records and every other lock field/byte remain identical;
substituting the previous Git identity reproduces the complete previous lockfile.
No automatic dependency update was used. Resolved Cargo metadata verifies all five.

The producer adds a private actual-runner offline capture seam with original
inputs, callback/action/queue outcomes, partial mutation and separate completeness.
It has no journal, durable receipt or release API, and no live caller has migrated.
The shared-feed wire format and consumer behavior remain compatible. The existing
registered durable-ingress workload still fails; no workload or new live capture
was run. Context, features, models and strategy decisions remain rsibot-owned.

Full local PR checks pass 543 unique Rust and eight Python tests, zero failures
or ignored cases; packaging repeats 23 Rust cases (566 summed executions).
Clippy, release/docs, screenshots, packaging and smoke pass. Exact audit 0.22.2
with denied warnings reports zero vulnerabilities/warnings; deny 0.20.2
licenses/sources and about 0.9.1 attribution pass. Final staged public checks
and independent evidence closure are recorded in validation.json.

The explicitly rebuilt debug CLI reads the original saved capture with an invalid
exchange URL. It separately records exit zero and preserves all 1,055 events,
archive clock, cursor/seal and complete output SHA256
`41e9a8a430547be5e39cd5db318ad4b14224cb5b852ebd4056699bcb5e249629`.
All 19 historical evidence files and both original archive files retain their
SHA256, size, mode, mtime, ctime and inode. This is saved-reader compatibility;
it does not qualify continuity or strict research admission.

Earlier consumer `99940f` main CI39364083428 and artifact CI39364083402 completed
successfully, including all four platform builds. Scheduled public API smoke,
publishing and announcement were skipped. Earlier producer `c8c1378` completed
all 17 jobs with identical complete tested/source tree. These historical results
retain their exact source scope; new sources require separate hosted CI.

Complete causal integration/replay B, unchanged failed-ingress comparison,
native semantics/units/continuity, strict datasets, rsibot live context/features,
calendar/native-runtime/hardware and operational qualification remain open.
Both PRs remain drafts. No economics, activation, merge, deployment or promotion.
