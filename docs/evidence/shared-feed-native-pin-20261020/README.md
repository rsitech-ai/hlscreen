# Checked directory backend pin migration

Both direct dependencies and all five resolved marketfeed packages now pin
cryptofeed 4c2bd1520d16573fd3cdc2fd7a818d126f28a12d, including the new filesystem
helper. The previous consumer head was bcfc9f4 at producer 410c3ca. This migration
changes no registry package versions, checksums or dependency vectors: all 419
registry records match the baseline. The only graph addition is recording's
filesystem edge and the new git package using existing windows-sys 0.61.2.
The parent independently parsed and compared the full records and verified
all changed-file, historical-evidence and command-log hashes.

Exact scripts/check.sh pr passes 543 unique workspace tests (0 failed, 0 ignored),
Clippy, formatting, release build, docs/screenshots, packaging and smoke.
Packaging reruns 23 already-counted Rust tests; 566 summed passes are not 566
unique tests. Eight Python candidate-binding regressions pass. Full-lock
cargo-audit 0.22.2 --deny warnings passes; cargo-deny 0.20.2 licenses/sources and
cargo-about 0.9.1 attribution/notices pass. The generated license file adds one
Apache attribution line; the NOTICE describes the new helper. Original 0.19
exploratory policy output remains separate from exact-tool qualification.

The new debug CLI supplied an invalid exchange URL produces exactly the same
saved 1,055-event summary, saved clock, cursor, seal and complete output hash as
the preceding 410c security receipt. Archive file hashes and all earlier report/
evidence bytes are unchanged. No new capture or soak is implied. The receipt
records source/binary/command/log identities and retained helper/tool corrections.

Producer native writer qualification belongs to cryptofeed's evidence: the
focused Windows job for 4c2bd15 passes 81 tests on an administrative-token NTFS
runner, with its PR-merge tree verified equal to that source head. It does not
establish ordinary-user ancestor access, actual rejection on other filesystems
or hardware power-loss persistence. These hlscreen checks establish consumer
compatibility and saved-prefix parity only. Strict research and sustained
operational gates remain open; remote verification of this migration is pending.
