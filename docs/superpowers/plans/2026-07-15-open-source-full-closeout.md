# hlscreen Open-Source Full Closeout Implementation Plan

> **For Codex:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to execute this plan task-by-task. Use superpowers:test-driven-development for behavior and contract changes and superpowers:verification-before-completion before claiming any gate is green.

**Goal:** Close every repository-controlled blocker in private, prove a release candidate locally and in hosted CI, then publish the repository with protections and security features enabled before creating the first `v0.1.0` release.

**Architecture:** Keep runtime code unchanged unless a validation failure proves it necessary. Add one canonical validation entrypoint, deterministic third-party attribution, fail-closed public-readiness and GitHub-surface gates, and explicit distribution/governance policy. Separate repository proof from owner/account gates so Actions billing, visibility, protection, security settings, and the release tag cannot be mistaken for code completion.

**Tech Stack:** Rust 1.88 workspace, Bash, Python 3, cargo-dist 0.32.0, cargo-about 0.9.1, cargo-deny 0.20.2, cargo-audit 0.22.2, zizmor 1.26.1, GitHub CLI/API.

---

## Hard boundaries and completion bar

- Work only on `feat/andrzej_oss_full_closeout` in `/private/tmp/hlscreen-oss-full-closeout`; preserve the user's checkout and stale feature branch.
- Keep the GitHub repository private until every repository-controlled check passes, a redacted full-history secret scan covers remote branches and pull-request heads, and hosted CI succeeds on the exact candidate SHA.
- Do not expose secret values in logs. Inventory secret/variable names only; secret scanning output may contain detector, ref, commit, path, and line number, never matched content.
- Do not delete branches, close PRs, change visibility/settings, or create a tag/release until their preceding gate explicitly passes.
- The first public distribution policy is GitHub binary releases only. Every workspace crate must be non-publishable to crates.io.
- Public completion means: local release gate green; clean candidate commit; private PR green; billing blocker cleared; public surface scan green; visibility public; ruleset/protection and security features verified; `v0.1.0` release artifacts, checksums, SBOMs, and attestations verified.

## Task 1: Lock down distribution policy and repository contracts

**Files:**

- Modify: `Cargo.toml`
- Modify: `crates/hls-core/Cargo.toml`
- Modify: `crates/hls-hyperliquid/Cargo.toml`
- Modify: `crates/hls-store/Cargo.toml`
- Modify: `crates/hls-features/Cargo.toml`
- Modify: `crates/hls-screen/Cargo.toml`
- Modify: `crates/hls-tui/Cargo.toml`
- Modify: `crates/hls-cli/Cargo.toml`
- Modify: `crates/hls-server/Cargo.toml`
- Modify: `tests/integration/release_packaging.rs`

1. Add a failing integration test that runs `cargo metadata --no-deps --format-version 1` or checks all manifests and proves every package reports `publish == []`.
2. Run `cargo test --test release_packaging distributable_crates_are_not_publishable -- --exact` and capture the expected failure.
3. Add `publish = false` to `[workspace.package]` and `publish.workspace = true` to every member manifest.
4. Re-run the focused test, then `cargo metadata --no-deps --format-version 1` and assert all eight package entries are non-publishable.
5. Run `git diff --check`.

## Task 2: Add deterministic dependency and vendored-tool attribution

**Files:**

- Create: `about.toml`
- Create: `about.hbs`
- Create: `THIRD_PARTY_LICENSES.txt`
- Create: `THIRD_PARTY_NOTICES.md`
- Create: `third_party/spec-kit/LICENSE`
- Create: `scripts/check-third-party-licenses.sh`
- Modify: `dist-workspace.toml`
- Modify: `scripts/local-release-artifact-smoke.sh`
- Modify: `.github/workflows/ci.yml`
- Modify: `tests/integration/release_packaging.rs`
- Modify: `docs/RELEASING.md`

1. Add failing release-packaging tests for:
   - `dist-workspace.toml` including both `THIRD_PARTY_LICENSES.txt` and `THIRD_PARTY_NOTICES.md`;
   - local archives copying and validating both files;
   - CI pinning `cargo-about 0.9.1` and calling the deterministic checker;
   - `about.toml` covering all four release targets without mutable ClearlyDefined enrichment;
   - Spec Kit `0.11.1`, source URL, covered paths, `Copyright GitHub, Inc.`, and its complete MIT license;
   - the local archive containing project and third-party licenses/notices.
2. Run the focused tests and confirm they fail for missing files/contracts.
3. Configure `cargo-about` with the allow-list aligned to `deny.toml`, `ignore-dev-dependencies = true`, `no-clearly-defined = true`, all release targets, and only necessary built-in workarounds.
4. Create a deterministic template listing crate name/version and selected full license text, followed by the fixed Spec Kit section. Preserve upstream notices verbatim where license obligations require them.
5. Install the pinned generator with `cargo install cargo-about --version 0.9.1 --locked --features cli`; generate and commit `THIRD_PARTY_LICENSES.txt`.
6. Make `scripts/check-third-party-licenses.sh` generate to a temporary file using `--workspace --all-features --locked --fail --config about.toml`, compare with `cmp`, and clean up safely. The script must not mutate the committed notice.
7. Add both notice files to cargo-dist `include`, local staging, post-unpack assertions, and release documentation. Add a pinned install/check step to the CI dependency-policy job.
8. Run:
   - `scripts/check-third-party-licenses.sh`
   - `cargo deny check licenses sources`
   - `scripts/local-release-artifact-smoke.sh`
   - `tar -tzf target/local-release-smoke/*.tar.gz | rg 'LICENSE|THIRD_PARTY'`
   - focused integration tests.
9. Manually review packages declared under Apache-2.0 or license exceptions for package-specific `NOTICE` obligations; record the review in `docs/OPEN_SOURCE_AUDIT.md`.

## Task 3: Create one canonical local validation entrypoint

**Files:**

- Create: `scripts/check.sh`
- Modify: `.github/workflows/ci.yml`
- Modify: `README.md`
- Modify: `CONTRIBUTING.md`
- Modify: `docs/RELEASING.md`
- Modify: `tests/integration/release_packaging.rs`

1. Add failing static contract tests proving `scripts/check.sh` supports exactly `fast`, `pr`, and `release`, rejects unknown modes, uses locked dependencies, and that docs point to it.
2. Implement the executable script:
   - `fast`: `cargo fmt --all -- --check`, locked workspace check/test suitable for iteration, and `git diff --check`;
   - `pr` (default): fmt, locked all-target/all-feature clippy, locked workspace tests, locked release build, rustdoc with warnings denied, deterministic screenshots, release-packaging check, diff hygiene;
   - `release`: all `pr` checks plus pinned cargo-audit policy, cargo-deny license/source policy, cargo-about drift check, and pinned zizmor workflow audit.
3. Preserve CI parallelism but make the Rust workspace job invoke `scripts/check.sh pr`, with separate security/dependency jobs invoking the same pinned commands used by `release`. Assert parity in static tests to prevent drift.
4. Replace duplicated contributor/release README command blocks with canonical mode invocations and explain the cost/scope of each mode.
5. Run `scripts/check.sh fast`, the focused contract tests, and `shellcheck` if available.

## Task 4: Correct public documentation, governance, and provenance

**Files:**

- Modify: `README.md`
- Modify: `CONTRIBUTING.md`
- Modify: `CHANGELOG.md`
- Modify: `SECURITY.md`
- Modify: `CODE_OF_CONDUCT.md`
- Modify: `SUPPORT.md`
- Modify: `.github/ISSUE_TEMPLATE/config.yml`
- Create: `tests/fixtures/README.md`
- Modify: `docs/data-format.md`
- Create: `docs/DEVELOPMENT_TOOLING.md`
- Modify: `docs/README.md`
- Modify: `docs/RELEASING.md`
- Modify: `tests/integration/release_packaging.rs`

1. Add failing public-contract tests for non-affiliation language, inbound MIT contribution licensing, build prerequisites, actionable reporting/support routes, fixture provenance, development-tooling trust policy, GitHub-only binary distribution, and truthful unreleased status.
2. Add platform prerequisites: Git, Python 3, rustup with rustfmt/clippy, Xcode Command Line Tools on macOS, build-essential/pkg-config on Linux, and MSVC C++ Build Tools on Windows.
3. Add the inbound MIT contribution statement and explicitly state no CLA is required.
4. Move dated `0.1.0` entries back under `[Unreleased]`; state that `0.1.0` is the intended first public release and has not been published. Only date it in the final release commit.
5. Add independent-project/non-affiliation language for Hyperliquid. Do not imply Hyperliquid endorsement or trademark ownership.
6. Use GitHub private vulnerability reporting as the primary security route. Before publication, confirm a monitored fallback security address and a separate conduct route; do not commit placeholders. Add response targets as expectations, not guarantees.
7. Enable a questions route through Discussions Q&A and add it to SUPPORT and issue-template configuration. Keep reproducible defects in Issues and security reports out of public issues.
8. Document all fixture groups as synthetic/minimized, derived output, or validation-report fixtures; prohibit credentials, real accounts/wallets, private streams, and unredacted user data. Link from the data-format guide.
9. Explain that pinned Spec Kit files are developer-only, identify manifests as integrity inventories, identify project-authored content, and require reviewed pinned updates with shell/PowerShell inspection.
10. Update the docs index to the July 13 proof and active `specs/004-advanced-tui-workstation`, retaining `002` as historical.
11. Run focused tests, `scripts/check-public-readiness.sh`, and `git diff --check`.

## Task 5: Harden the public-readiness and hosted-surface gates

**Files:**

- Modify: `scripts/check-public-readiness.sh`
- Create: `scripts/check-public-surface.sh`
- Create: `docs/OPEN_SOURCE_AUDIT.md`
- Modify: `docs/OPEN_SOURCE_CHECKLIST.md`
- Modify: `tests/integration/release_packaging.rs`

1. Add failing tests proving public readiness requires the new attribution, development-tooling, fixture-provenance, canonical-check, and audit files; rejects developer-specific paths, credential patterns, placeholders, obsolete private-contact wording, unsafe trading claims, and a dated `0.1.0` before a matching tag exists.
2. Extend local readiness to scan credentials through all reachable history and private/developer paths in the current tree, while avoiding self-matches and never printing matched secret values.
3. Implement a read-only authenticated surface gate with `private-candidate` and `public` modes. It must verify:
   - expected clean SHA and origin relationship;
   - visibility/default branch and branch/tag inventory;
   - Actions success at the exact candidate SHA;
   - no unexpected collaborators, hooks, deploy keys, secrets/variables, environments, Pages, deployments, releases, artifacts, or packages;
   - sanitized PR/comment/review text checks;
   - no unresolved human PRs and an explicitly recorded decision for dependency PRs/stale branches;
   - public mode additionally requires ruleset/protection and enabled security features.
4. Keep billing and private-advisory/package UI confirmation as explicit owner checklist items because the current token cannot read them conclusively.
5. Add a pinned redacted full-history scanner command covering fetched `refs/heads/*` and `refs/pull/*/head`. Store only a pass/fail summary and tool/version/ref coverage in the audit; never store findings containing secret material.
6. Run the local gate against the private candidate and record exact SHA, timestamp, inventories, exceptions, and owner-only blockers in `docs/OPEN_SOURCE_AUDIT.md`.

## Task 6: Regenerate release workflow and run the full private local matrix

**Files:**

- Modify if generated: `.github/workflows/release.yml`
- Update: `docs/OPEN_SOURCE_AUDIT.md`

1. Run `python3 scripts/harden-generated-release-workflow.py --regenerate`, review every generated diff, then run `--check`.
2. Run `dist plan --output-format=json`; confirm four targets, source tarball, checksums, SBOM, cargo-auditable metadata, attestations, tag gating, and both notice files.
3. Run `scripts/check.sh release` from a clean tree.
4. Run the PTY integration test on the available host and all existing release-packaging/public-readiness tests.
5. Inspect the actual local archive with `tar -tzf`, unpack it, run `hls --help`, `hls doctor`, and the bounded fixture-backed `hls live --once` smoke.
6. Run `git diff --check`, `git status --short`, and review `git diff --stat` plus all security/release/script changes.
7. Update the audit with exact commands and outputs. Any failure returns to its owning task; do not label the candidate ready while a required command is red.

## Task 7: Review, commit, and prove the candidate while private

**Files:**

- All intentional files from Tasks 1-6 only.

1. Use `andrzej-pr-hardening` to inspect base/head, status, diff, workflow pins, test coverage, generated artifacts, and secret/path output safety.
2. Run a fresh review subagent against the complete diff and fix every high-confidence issue.
3. Re-run the smallest affected checks, then the full release gate after the final fix.
4. Commit only intentional files, push `feat/andrzej_oss_full_closeout`, and open a private PR against `main`.
5. Fix the account billing/spending blocker in GitHub owner settings. This owner action cannot be automated with the current token and is a hard hosted-CI blocker.
6. Rerun CI and require every required job to succeed on the exact candidate SHA with non-zero executed steps. Run the cargo-dist pull-request plan/upload workflow and inspect its artifacts.
7. Merge only after the review and hosted gates are green. Re-run the private surface gate on the exact merged `main` SHA.

## Task 8: Clean the hosted surface and publish with immediate protections

1. Resolve Dependabot PRs `#28` and `#42`-`#46` one-by-one through update/merge/close decisions with green checks; document each decision.
2. Compare all seven stale feature branches against `main`, preserve wanted unique history, and delete only branches confirmed merged or intentionally retired. Treat cleanup as hygiene, not secret purging.
3. Confirm no private advisory drafts, packages, unexpired artifacts, Pages, deployments, or unexpected refs remain through owner UI/API.
4. Set repository metadata while private:
   - description: `Read-only Rust TUI for Hyperliquid spot market-data recording, replay, analysis, and screening.`
   - topics: `rust`, `hyperliquid`, `ratatui`, `tui`, `terminal`, `market-data`, `screener`, `cli`;
   - enable Discussions with Q&A;
   - keep homepage blank until a distinct site exists.
5. Prepare the exact `main` ruleset/protection payload before changing visibility. Require pull requests, required CI checks, conversation resolution, no force pushes/deletions, and admin inclusion.
6. Change visibility to public, immediately apply the ruleset/protection, then enable dependency graph, Dependabot alerts/security updates, private vulnerability reporting, secret scanning with push protection, and code scanning/default setup where GitHub makes them available.
7. Restrict Actions to the audited, SHA-pinned actions used by the repository while preserving required release functionality.
8. Run `scripts/check-public-surface.sh public <merged-sha>` and manually confirm the owner-only settings. If protection/security enablement fails, treat the public state as an incident and finish hardening before any release/tag announcement.

## Task 9: Create and verify the first public release

1. In a dedicated release PR, turn `[Unreleased]` into `[0.1.0] - 2026-07-15` (or actual release date), restore an empty `[Unreleased]`, and update the audit/checklist.
2. Run `scripts/check.sh release`, public surface gate, cargo-dist plan, and full review at the exact release commit.
3. Merge with green required checks; create annotated tag `v0.1.0` only from protected `main`.
4. Verify the tag-triggered release run, then inspect every macOS/Linux/Windows archive, checksums, source tarball, SBOM, audit metadata, and GitHub attestations. Confirm both third-party notice files are present.
5. Run the installed/smoke path on the available host and verify `hls --version` reports `0.1.0`.
6. Mark the release checklist complete only after the public repository, protection, security features, release page, artifacts, checksums, SBOMs, and attestations are all live and exact-SHA verified.

## Verification command set

```bash
cargo metadata --no-deps --format-version 1
cargo test --test release_packaging --locked
scripts/check-third-party-licenses.sh
scripts/check-public-readiness.sh
scripts/check.sh release
python3 scripts/harden-generated-release-workflow.py --check
dist plan --output-format=json
cargo test -p hls-cli --test pty_tui --locked
scripts/check-public-surface.sh private-candidate "$(git rev-parse HEAD)"
git diff --check
git status --short --branch
```

## Review checkpoints

- After Task 2: attribution/legal-engineering review.
- After Task 4: docs/governance/product-claims review.
- After Task 5: privacy and output-safety review.
- After Task 6: full local release-candidate review.
- After Task 7: private hosted-CI/artifact review.
- After Task 8: public settings/protection/security review.
- After Task 9: final release-artifact verification.

## Rollback and recovery

- Local changes remain isolated on the feature branch; revert only the failing task's intentional commit and preserve evidence in the audit.
- If an actual secret is found, rotate/revoke first, stop publication, then use a separately reviewed history-rewrite/purge plan. Branch deletion is insufficient.
- If the visibility change succeeds but protection/security setup fails, do not tag or announce a release. Apply the prepared settings immediately; if they cannot be applied safely, return the repository to private until the blocker is understood.
- If release artifacts are wrong, delete the draft release before publication or mark the release unusable; correct the source and cut a new version rather than silently replacing published immutable artifacts.

## Current known external blocker

At plan creation, GitHub Actions runs `29411491370` and `29411552416` fail before any job steps due an account payment/spending-limit condition. Repository work can continue, but private hosted proof, merge, public visibility, and release remain blocked until the owner clears billing and a fresh run succeeds on the candidate SHA.
