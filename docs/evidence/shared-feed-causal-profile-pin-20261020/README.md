# Shared causal-profile source pin

Both direct shared-feed dependencies and all five transitive marketfeed packages
are pinned to producer `6565c9428e0c97636f1c5988f4a2c22b4d17f52e`.
The 419 complete registry records and every other lockfile byte are unchanged.

Hlscreen continues reading the legacy `MFSF1` shared feed. A new regression
checks that a causal `MFCJ1` journal is rejected at negotiation, before consulting
its watermark or creating feature windows. Engine causal operations require a
deliberate projection from committed release outcomes; that integration is not
implemented by this pin update.

The complete local PR check passes **544 unique Rust tests and eight Python
tests**, including 23 packaging repeats for 567 Rust executions. Formatting,
Clippy, release build, documentation, screenshot, packaging and smoke checks
pass. The rebuilt debug binary replays all **1,055** events from the original
sealed capture with the identical complete output hash
`41e9a8a430547be5e39cd5db318ad4b14224cb5b852ebd4056699bcb5e249629`.
The receipt binds its actual executable hash and exit status.

Exact cargo-audit 0.22.2 with denied warnings, cargo-deny 0.20.2 license/source
policy and cargo-about 0.9.1 attribution checks pass. All 21 historical evidence
files and both original archive files retain hashes, sizes, modes, modification
and change timestamps and inodes against the new pre-update baseline. Earlier
receipts retain their original pins and qualification scope.

These are local consumer compatibility and boundary results. Producer storage
tests do not qualify engine replay or observed availability. The registered
ingress failure, engine DTO/bootstrap/private commit-gated owner, release
projection, actual-engine replay/live migration, native semantics/continuity,
strict dataset, rsibot context/features and calendar/hardware gates remain open.
Hosted CI for each newly published source is a separate qualification.
