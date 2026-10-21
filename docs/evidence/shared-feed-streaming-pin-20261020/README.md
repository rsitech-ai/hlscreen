# Shared-feed streaming repair pin — 2026-10-20

Both direct shared-feed dependencies and all five resolved marketfeed packages
now pin producer `244f40c7e7d84db4cbf6cb2d35e1a5b7b24d7cdf` together. The producer
adds bounded legacy replay and rejects torn MFR length words; it does not change
shared-feed schemas. All 419 registry records are unchanged. Historical capture,
soak and security receipts retain their original bytes and pins.

The full local PR gate passes: 543 unique Rust workspace tests, eight Python
binary/source-binding regressions, Clippy, release/docs, screenshot, packaging
and release-smoke checks. Packaging reruns 23 existing tests (566 summed Rust
executions). Full-lock RustSec, exact cargo-deny 0.20.2 license/source policy and
cargo-about 0.9.1 attribution/notice checks pass. Attribution files are unchanged.

The explicitly rebuilt debug CLI replays the original sealed prefix with an
invalid exchange URL. All 1,055 events, cursor, archive clock and seal remain
identical. Complete output SHA256 is
`41e9a8a430547be5e39cd5db318ad4b14224cb5b852ebd4056699bcb5e249629`.
Both archive file hashes remain unchanged. Commands and raw-log/source hashes
are in `validation.json`; local target logs are retained outside Git.

These checks were run before committing the consumer pin, as recorded by its
base head/dirty flag. This is consumer saved-prefix compatibility evidence,
not new live capture, soak, Windows writer qualification, complete engine causal
replay or strict research admission. Producer and new consumer CI are tracked
separately. Historical soak evidence stays report-only; current candidate
qualification still requires exact binary and source binding.
