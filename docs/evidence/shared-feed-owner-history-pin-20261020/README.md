# Shared owner-history producer pin

Hlscreen pins all five shared packages to producer
`7f383b2dba580740e5c60c3dcb572e236e89f588`. Four of the 52 actual Cargo
shared source files changed from the prior pin; all 52 match the new producer
Git blobs. All 419 complete registry records and all other lock bytes remain
unchanged. This is scoped source verification, not artifact equivalence.

[Validation](validation.json) records the fresh full local PR checks: 544 unique
Rust tests (567 executions including 23 packaging repeats), 12 Python cases,
formatting, denied-warning lint/docs, release build, screenshots and packaging.
Exact cargo-audit 0.22.2 reports zero vulnerabilities/warnings; cargo-deny 0.20.2
and cargo-about 0.9.1 policy/attribution pass. The actual rebuilt and preserved
CLI replays all 1,055 saved events with byte-identical full output, cursor,
saved clock and per-kind counts. Both capture files and all 27 historical
evidence files retain bytes and metadata; their earlier pins are unchanged.

The consumer tests both MFCJ1 and MFCJ2 header rejection before watermark
access, including a missing watermark. MFCJ2 adds typed owner-history records
and physical-order validation to the producer. Actual runtime owner history,
committed projection, bound external I/O, actual-engine replay and live migration
remain unfinished; the producer factory rejects this profile. Hlscreen still
consumes MFSF1. The registered durable-ingress workload remains failed.

cryptofeed owns acquisition/storage/replay, rsibot owns context/features/models/
ETF research and decisions through existing owners, and hlscreen presents the
shared feed. Fresh own-source producer and consumer hosted checks are required.
This draft compatibility change does not qualify strict datasets or operations.

Separate read-only review verified the actual source checkout, lock records,
retained binary, parent execution/security logs and unchanged historical data,
with no material finding in this scope. No independent execution was claimed.
