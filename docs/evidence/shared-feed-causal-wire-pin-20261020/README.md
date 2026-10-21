# Causal wire consumer pin

All seven direct and 23 resolved marketfeed dependencies use producer
`5479d733f02f1a2777579460154e144babe67b87`, source recipe
`16a538c033fb37fa10112f66a6371ab7408512a7f991e95add1640af5a8afa95`.
HLS reads MFCJ5/OwnerHistory4 observations through MFOP1. Successful wire control
groups create no market features; committed Failed/Unknown application-write
settlements immediately produce a Gap. Raw MFCJ journals remain unsupported.

All 11 local source04 checks passed on macOS aarch64 with Rust 1.88: 565 tests,
Clippy and documentation with denied warnings, formatting, pin checks, 14 Python
helper tests, RustSec audit, license/source policy and a freshly packaged release
binary consuming the synthetic fixture. See [validation.json](validation.json).
The separate [artifact diagnostics repair](artifact-diagnostics.json) passes15
Python tests and a new packaged fixture check; it preserves failures while
showing bounded nested stderr. It does not resolve the hosted Windows failure
or replace the qualified retained executable.

Normal Rustls trust and hostname checks remain active; patched Mozilla roots and
verifier floors remove the vulnerable wrapper. SHA2 dev/test optimization retains
all executable rechecks, caps and deadlines.

An earlier full run timed out in the existing PTY resize/quit test. Bounded
same-artifact single, parallel, resize/key and 20-case diagnostics did not
reproduce it. Later source-qualified full suites passed, but the original failure
remains **unexplained**; no timeout, backend, thread or coverage change is called
a fix. The real shared-helper lint and fixture-classification failures are also
retained with their narrow corrections. All 71 historical files and workflow
bytes remain unchanged. The new fixture lives under the already classified
Hyperliquid fixture folder; the original classification rule is unchanged.

The [synthetic fixture](../../../tests/fixtures/hyperliquid/mfop5-synthetic-v1/README.md)
was exported after genuine native owners, one-use wire calls, source replay and
projection comparison passed. Existing artifact scripts consume its 67 groups,
12 wire groups, two native trades and exact cursor/clock/seal summary. This is
portable structural compatibility, not native producer authentication on every
artifact platform. No workflow was edited.

The retained closed pilot04 has 1,415 groups and 271 projected items. Actual HLS
once/follow reads matched the independently counted 322 released native feature
events: 97 trades, 154 BBOs, 14 books and 57 contexts. Seven gaps are final
Withheld/unsafe Stop/Revoked source transitions, not a transport-failure claim.
Accepted, executed and released counts differ when the Stop fence withholds an
input. These checks preserve the original source, feed and executable bytes and
metadata. Prior pilot03 failed freshness with 10–11 seconds of acquisition lag;
that failed result is retained. The concurrent 120-second pilot05 reached matching final checkpoints at 6,203
groups with both HLS and Quants progressing before Seal. It nevertheless failed
timing qualification: the target file finished about 34 seconds after the source,
and sampled perps acquisition lag reached about 28 seconds. Final synchronization
does not establish timely delivery. The producer retains the detailed failed
[joint05 receipt](joint05.json).

Producer [run 39504100341](https://github.com/rsitech-ai/cryptofeed-rs/actions/runs/39504100341)
is terminal **failed**: 16 jobs succeeded and Windows failed its paused-storage
32-frame test. The exact producer source passed macOS, Linux x86, Linux ARM and
its own MSRV checks; [producer-hosted.json](producer-hosted.json) retains actual
checkout/tree/source and per-platform witnesses. This is separate producer
coverage. HLS own-source run [39507682503](https://github.com/rsitech-ai/hlscreen/actions/runs/39507682503)
passed workspace, macOS/Linux PTY, RustSec and Actions checks
([exact hosted receipt](consumer-hosted-af6.json)), but failed generated
license attribution. The local [attribution repair](attribution-repair.json) changes
only the dependency usage lines and passes the existing generator/checker. This
does not retroactively pass the failed hosted job. Native artifact run
[39507682505](https://github.com/rsitech-ai/hlscreen/actions/runs/39507682505)
is terminal failed: Linux, macOS ARM and macOS Intel packaged-artifact jobs pass;
Windows fails its nested fixture helper with the underlying reason still unknown.
The [three available native archives](consumer-native-artifacts-af6.json) match
their hosted smoke binary hashes, checksums and immutable af6 documentation.
No Windows, global or source archive was produced; publication was skipped.
The successor source requires its own hosted results.

Availability remains unknown. A structural read or seal does not establish
market continuity, usable historical time, loss, latency guarantees, strategy
feature checkpoints, registered load or 24h/7d readiness. No economic run,
release, merge, tag or operational promotion follows from these local checks.
