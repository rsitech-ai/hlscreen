# Shared writer source coordination — 2026-10-20

Both direct dependencies and all five resolved marketfeed packages move together
from `82a871b` to producer `c8c13781d257d0d6e252144a8a70a07f700e1764`. No
registry package or other lock field changes: all 419 complete registry records
remain identical. No automatic dependency update was used.

The producer adds a bounded writer actor with separate admission and durable
receipts. It is not yet installed in the live engine. The registered durable
ingress workload still fails; complete causal replay and strict data admission
remain unfinished. The shared-feed wire format and consumer behavior stay the
same. Strategy context and decisions remain rsibot-owned.

Full local PR checks pass 543 unique Rust and eight Python cases, with zero
failures/ignored. Packaging repeats 23 Rust cases (566 summed executions).
Clippy, release/docs, screenshots, packaging and smoke pass. Cargo-audit 0.22.2
with denied warnings reports zero vulnerabilities/warnings; exact cargo-deny
0.20.2 licenses/sources and cargo-about 0.9.1 attribution pass. The final staged
public check and independent closure are recorded in `validation.json`.

The explicitly rebuilt debug CLI replays the original capture with an invalid
exchange URL: all 1,055 events, archive clock, cursor and seal are identical.
Complete output SHA256 remains
`41e9a8a430547be5e39cd5db318ad4b14224cb5b852ebd4056699bcb5e249629`.
Both original archive files and all 17 historical evidence files retain their
bytes. The first shell replay retained complete matching output but did not
capture a separate CLI exit before metadata; it remains preserved. The qualifying
second replay records exit zero explicitly and has identical output. This is
saved-prefix reader compatibility, not a new capture or continuity qualification.

New producer/consumer sources need their own CI. Earlier timestamp producer
`7bef521` completed all 17 jobs with its identical tested merge tree. Earlier
consumer `1e33c1e` main and artifact workflows also completed successfully with
publishing/promotion skipped. These historical results retain their exact scope.
Native/runtime, calendar soak, strict datasets, economics and operational
promotion remain unqualified; both integration PRs remain drafts.
