# Typed native observation consumer

This candidate adds MFOP1 consumption to HLS735784b88b06e6f75a74081fe762cf5c0db131cd,
pinning all six direct and all 23 resolved marketfeed Git packages to immutable
e812e44b9893f200a4e357e6e8ae77ae41db1c3b. Native spot/perps observations, original
product metadata, physical source order and explicit approximate feature-clock
policy replace raw-frame decoding for MFOP1. MFSF1 retains legacy compatibility.
All-mids is not admitted by this producer profile. Availability remains unknown;
structural validation alone does not authenticate financial values.

Whole-batch application precedes acknowledged cursor/clock publication. Errors
and caught callback panics refuse reuse; caller partial state must be discarded.
Native context mid/circulating supply remain separate from book mid. Independent
input/output/event/book budgets defer one complete group without exposing future
metadata. The amplification test crosses the production 8MiB output cap with
actual offline factory/owner/source-comparison operations and a synthetic long
symbol catalog. Its declared source-record capacity is 1MiB and whole-projection
test budget 60s; original small-fixture and producer 15s watchdogs, owner 10s /
finalization 2s policies and HLS production caps are unchanged. Logical limits
are not measured process-memory guarantees or persisted feature checkpoints.

All eight frozen-source local gates pass on Rust/Cargo 1.88.0: full PR checks,
580 Rust executions (556 unique target/test pairs), 12 Python tests, release/docs/
screenshot/supervisor/packaging checks, exact saved replay, full-lock RustSec,
dependency policy and attribution. [Validation](validation.json) and the complete
[283-file input inventory](source.json) bind this scope. Historical evidence
outputs are excluded from the input digest and checked separately. All 35 old
HLS evidence files, 22 frozen dataset files and two pilot files retain every
recorded hash/byte/mtime/ctime/mode/inode field; the dataset stays 0555.

The saved 1,055-event replay remains byte-identical (SHA256
41e9a8a430547be5e39cd5db318ad4b14224cb5b852ebd4056699bcb5e249629).
Retained actual executable files and the native test's embedded declared producer
recipe are bound separately. These are not loaded-image, full transitive-input,
reproducible-build, deployment or soak proofs. Native macOS tar members match
current source documents and the explicitly rebuilt host executable.

Artifact inspection caught two verification bugs: the smoke script could copy a
stale executable with a custom Cargo target/target directory, and native tar
creation included AppleDouble metadata. It now explicitly builds the host target
and target directory, checks Cargo fingerprints even when a binary exists, and
creates the archive with metadata copying disabled. The sandbox regression tests
relative/absolute overrides, conflicting Cargo environment/config, stale binaries,
failed rebuild rejection and a macOS xattr fixture. The original source01 gates
remain passed but its artifact qualification remains failed; the original tar
and validator failures are retained. Source02 reruns every gate.

The lock preserves all 419 existing registry node versions/checksums and adds 12
nodes. Dependency edges are not claimed identical: feature additions and Windows
dependency selections are listed explicitly in validation. cargo-audit 0.22.2
reports zero vulnerabilities/warnings against its recorded 1,345-advisory
database; deny 0.20.2 passes advisories/bans/licenses/sources, and about 0.9.1
verifies regenerated attribution. The recorded database and exact lock define the advisory scope.

Own-source hosted/native/archive qualification remains pending. Producer e812's
CI39462645500 has real Windows and Linux 47/48 Hyperliquid watchdog failures;
phase/cause remain undetermined. Diagnostic successor87b is separate and is not
a claimed repair. Prior HLS735/pin95 CI cannot qualify this candidate. Real
live/control/external-I/O integration, the unchanged failed ingress comparison,
strict datasets, all-mids and operational gates remain open. Strategy research
and planned continuous consumption stay in rsibot; its initial manifest boundary
is not a continuous reader. No economics, deployment, merge or release.
