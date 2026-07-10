# Release Packaging Readiness - 2026-07-09

## Status

This report proves local release-packaging mechanics only. It does not prove a
published or reviewed public binary release.

## Local Evidence

- `scripts/local-release-artifact-smoke.sh` builds or reuses `target/release/hls`.
- It creates `target/local-release-smoke/hlscreen-<host-triple>.tar.gz`.
- It writes and verifies a SHA-256 checksum.
- It unpacks the archive and runs `hls --help`, `hls doctor`, and a
  fixture-backed bounded `hls live --once` smoke.
- `scripts/check-public-readiness.sh` validates required open-source docs,
  workflows, screenshot assets, and truthful release wording.
- `scripts/check-release-packaging.sh` runs the static release contract tests,
  public-readiness scan, and local artifact smoke.

## Missing Publication Proof

- No reviewed `v*` release page has been verified.
- No published artifact/checksum matrix has been reviewed on clean supported runners.
- No public binary installation instruction is therefore considered proven.

## Safety Boundary

Release packaging requires no wallet keys, exchange credentials, private
endpoints, or order-capable functionality. All local smoke tests are read-only
and fixture-backed by default.
