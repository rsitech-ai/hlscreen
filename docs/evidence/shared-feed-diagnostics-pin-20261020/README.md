# Shared diagnostic source pin — 2026-10-20

Both direct dependencies and all five resolved marketfeed packages now pin
producer `82a871bdb80f1c4c2df0574f05f0e45012989918` together. The producer adds
observational publication-stage timings and a bounded offline ingress diagnostic;
the shared-feed wire schema and reader behavior remain compatible. It also
contains the previously qualified catalog index guard and streaming replay.

All419 complete registry records remain unchanged. Cargo's first update changed
nine unrelated Windows dependency edges; comparison rejected that drift and its
initial PR check was interrupted. The corrected lock changes only the five Git
source identities. Locked metadata, the complete PR gate and security/license/
attribution checks were rerun on that exact corrected lock. Superseded logs and
lock bytes are retained outside Git; they are not passing migration evidence.

The complete local PR gate passes543 unique Rust tests and8 Python binding cases,
with23 Rust packaging repeats (566 summed executions), zero failures/ignored.
Clippy, release/docs, screenshots, packaging and smoke pass. Full-lock RustSec
0.22.2 with denied warnings, cargo-deny0.20.2 license/source policy and cargo-about
0.9.1 attribution checks pass. Historical evidence and attribution retain their
original bytes.

The explicitly rebuilt debug CLI replays the original sealed archive with an
invalid exchange URL: all1,055 events and complete output SHA256
`41e9a8a430547be5e39cd5db318ad4b14224cb5b852ebd4056699bcb5e249629`
remain identical. Both archive hashes are unchanged. Actual source, lock, binary,
historical-file and log hashes are recorded in `validation.json`.

This migration does not repair the producer's newly reproduced ingress failure:
its raw-only synthetic diagnostic completes4,864 frames while the durable-writer
arm reaches bounded queues after147 callbacks. That workload remains failed and
unqualified. Successful saved-prefix consumption does not establish live source
freshness, continuity, capacity, complete causal replay, strict dataset admission
or operational promotion. Strategy context/features/models remain rsibot-owned.
New source/consumer platform CI is a separate gate; no earlier run is relabelled.
