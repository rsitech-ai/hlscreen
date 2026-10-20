# Shared operation-owner producer pin

Hlscreen now pins all five resolved shared packages to producer
`8ba199a46f9a1e2e840d104c3da5610235ef753e`. This producer changes shared writer
code, so the earlier unchanged-shared-source proof cannot qualify this pin.

[Validation](validation.json) records the actual full local PR check: 544 unique
Rust tests (567 executions including 23 packaging repeats), 12 Python tests,
workspace lint/format/release/docs, screenshots and packaging checks. Exact
cargo-audit 0.22.2 reports zero vulnerabilities/warnings; cargo-deny 0.20.2 and
cargo-about 0.9.1 policy/attribution checks pass. All 419 complete registry lock
records and all lock bytes outside the five Git source identities are unchanged.
The actual Cargo checkout's 52 shared files match the pinned producer Git blobs.

The rebuilt, fingerprinted and preserved CLI replays all 1,055 saved market
events with byte-identical full output to the preceding consumer. The sealed
cursor, saved clock and per-kind counts agree. Both capture files and all 25
historical evidence files preserve bytes and metadata. Older receipts retain
their original pins and qualifications.

The producer now has a private operation codec and persistent commit/release
owner. Owner visibility/handoff/admission history, committed projection, bound
external I/O, actual-engine replay and live migration remain unfinished.
Hlscreen still consumes MFSF1 and rejects causal journals. The registered ingress
workload remains failed; this pin does not rerun it or qualify strict datasets.
New consumer main/native artifact CI and source-specific producer CI remain
required. This is a draft compatibility change, with no operational promotion.
