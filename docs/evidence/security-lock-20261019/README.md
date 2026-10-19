# Full-lock RustSec repair — 2026-10-19

Remote RustSec job 118361654247 on source d40d413 failed with five quinn-proto
vulnerabilities, an unsound lru warning and a yanked chacha20 warning. This is a
real failure retained separately from the earlier source/license policy pass.
Those policy checks do not replace the full-lock RustSec audit.

The patch retains the producer pin 410c3ca and changes only three registry package
versions/checksums: quinn-proto 0.11.16→0.11.18, lru 0.18.1→0.18.2 and
chacha20 0.10.1→0.10.2. All dependency vectors and other package records remain
unchanged, independently parsed and checked. The generated license file changes
its lru version attribution. No ignore policy, target filter or MSRV is weakened.

The patched full lock passes cargo-audit 0.22.2 with --deny warnings: zero
vulnerabilities and zero warnings on advisory database commit 999ec89d5d3f10f20be14750ec8c2d8b44e0ac9b.
Exact lock/tool hashes and audit scope are retained in rustsec-audit.json.
All three patched packages declare Rust 1.85 or 1.85.0, below hlscreen's 1.88 minimum.
Quinn/ChaCha are inactive in the current host feature tree; that does not waive
the full-lock audit or justify retaining their vulnerable/yanked lock records.

Cargo's targeted updates also changed unrelated tempfile/Windows resolution
edges. Those edges were restored from the preceding lock, preserving only the
three patched package records; cargo metadata --locked accepts the frozen result.
An exploratory test run compiled while resolution changed and is not final-source
qualification. A later test run exposed an existing bounded PTY cleanup failure:
one test-owned sleep process survived the single group kill. This is consistent
with a group-kill/fork-delivery race; the exact kernel interleaving was not
instrumented. The failed run is retained. Cleanup now repeats force signaling
only while the owned group is observed live, interleaving child-status/group
probes under the unchanged absolute deadline. An observed absent group is never
signaled again. Existing watchdog/assertions remain, and a fixed three-session
challenge requires every session to pass without retrying failed cases.
Final-source verification passes all 543 workspace/all-target tests on Rust 1.88,
with zero failures/ignored cases; Clippy, formatting, source/license policy and
regenerated third-party licenses/notices pass. The complete six-test PTY file
passes, including the original regression and fixed three-session challenge.
Validation commands, source/log hashes and original failures are in validation.json.
Saved-prefix replay still matches the identical 1,055-event summary, cursor, seal
and archive clock; no exchange connection or new live-soak qualification is implied.

Earlier pin-migration and saved-prefix receipts keep their source d40d413 scope.
The Windows producer durability defect, source continuity, strict datasets and
sustained operational qualification remain open.

Primary sources:
- [Original CI failure](https://github.com/rsitech-ai/hlscreen/actions/runs/39340825480/job/118361654247)
- [Quinn panic fix>=0.11.18](https://rustsec.org/advisories/RUSTSEC-2026-0338.html)
- [ACK-frequency fix>=0.11.18](https://rustsec.org/advisories/RUSTSEC-2026-0340.html)
- [Earlier Quinn bounds fixes](https://rustsec.org/advisories/RUSTSEC-2026-0341.html)
- [LRU panic-safety fix>=0.18.2](https://rustsec.org/advisories/RUSTSEC-2026-0253.html)
- [ChaCha registry version/yank status](https://crates.io/api/v1/crates/chacha20)
