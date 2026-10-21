# Shared producer pin migration — 2026-10-19

Both direct shared-feed packages and all transitive marketfeed packages resolve
to cryptofeed 410c3cae95f17c3b2b8ee88a9809c5fc1d7ad28b. The lockfile changes only
those source revisions and the adapter API's already-locked serde_json dependency;
no registry package version changed. Canonical archives and historical receipts
retain their original identities.

All 542 workspace/all-target tests pass on Rust 1.88, with no failures or ignored
tests. Clippy, formatting, immutable pin resolution, third-party licenses/notices
and source/license policy pass. The targeted Hyperliquid consumer checks on 1.88.
Commands, log hashes and exact source input hashes are in validation.json.

The newly built CLI reads one existing sealed Hyperliquid capture with an invalid
exchange URL supplied and no exchange connection. Its entire saved summary matches
the old-pin summary: 1,055 events (270 trades, 512 BBO, 37 books, 58 candles, 178 contexts),
zero reported gaps and the identical saved clock, cursor and seal. The exact
committed prefix and binary/log hashes are in saved-prefix-replay.json. This is
saved-prefix consumer compatibility, not exact financial archive rewriting or a
new live-forward/soak result.

The producer's Windows durable-publication defect remains open. These local
reader checks do not qualify a Windows writer, source continuity, strict research
datasets, strategy decisions or operational promotion. Earlier live
samples and 15-minute standalone soak retain their original source/version scope.
