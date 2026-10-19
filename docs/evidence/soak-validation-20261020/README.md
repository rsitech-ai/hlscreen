# Historical report and candidate qualification repair

CI run39342436582/job118366958412 on82b715b failed at packaging: it compared
a retained historical macOS soak binary to the new Linux release binary.
The original failure is retained in the companion cryptofeed evidence root as
hlscreen-security-ci-rust-workspace-01.log. The old check could also pass locally
with changed sources when the old release binary remained in target/release.

Packaging now validates saved reports explicitly as historical/report-only.
It does not claim that the current candidate passed a soak. Candidate
qualification requires both --binary and --runtime-source-root; the exact binary
and runtime source hashes must match the report. Binary-only validation remains
available and labels its narrower scope. The supervised runner checks both
bindings. Report bytes, commits and recorded hashes are unchanged; the two
historical report hashes are pinned in the behavioral regression suite.

Eight subprocess regressions cover exact binding, changed binary, changed source
with retained binary, documentation-only changes, missing binary, missing paired
option, binary-only scope, and immutable historical scope. Both the author and
parent ran these tests. The exact CI entrypoint scripts/check.sh pr passed:
543 unique workspace tests, Clippy, release build, docs/screenshots, packaging
checks, supervisor/public-readiness and packaged fixture smoke. It invokes the
23 packaging tests a second time:566 summed passes are not566 unique tests.
Commands, log hashes and changed-file hashes are recorded in validation.json.

No new live/soak run was performed. This tooling repair does not qualify the
current binary for operations or replace the remaining calendar24h/7d gates.
Producer pin410c3ca, runtime source, lock, recorded events and reports remain
unchanged by this slice. Remote verification of this patch remains pending.
