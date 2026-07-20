# Production-ready open-source release hardening

## Goal

- User-visible outcome: prepare `hlscreen` as far as safely possible for a professional public GitHub release, with truthful installation, security, product, packaging, and contributor-facing surfaces.
- How to see it working: the canonical local release gate, clean-artifact installation smoke, documentation checks, redacted worktree/history scans, and real TUI/runtime smoke all pass from one final commit; remaining legal, owner, hosted, and publication actions are isolated as explicit gates.

## Current State

- Relevant paths: Rust workspace under `crates/`, public docs under `README.md` and `docs/`, repository automation under `.github/`, release gates under `scripts/`, and prior evidence under `docs/evidence/`.
- Existing behavior: `hlscreen` is a read-only Hyperliquid spot-market CLI/TUI for macOS and Linux. The repository already contains community files, MIT license text, CI/release workflows, packaging scripts, notices, screenshots, and public-readiness checks.
- Baseline: `origin/main` is `a44b349fd6e62237873582f31f6dd16f6510324f`; the inherited audited candidate is `e287955c90e90fa9041b409a6fde1dc650bc7e17`, 18 commits ahead with a clean tree. Work continues on `chore/oss-release-readiness` without rewriting or discarding those commits.
- Constraints: no push, PR, merge, release, visibility change, GitHub/profile setting mutation, package publication, license selection/change, copyright decision, personal-contact publication, or history rewrite without approval. Secrets must remain redacted. No trades or private exchange credentials are in scope.

## Target State

- Desired behavior: a clean unauthenticated checkout builds with public dependencies, documented commands work, release artifacts contain only intended files and install cleanly, public workflows are least-privilege and safe for forks, security/privacy/IP findings are resolved or explicitly gated, and the real TUI/runtime remains truthful and stable.
- Non-goals: publishing externally, changing repository ownership or visibility, rewriting history, making breaking public-API changes, claiming hosted/default-branch/public-page proof before those surfaces are actually configured and observed, or inventing legal/contact/funding/adoption claims.

## Risks and Failure Modes

- Historical sensitive material or unclear third-party provenance could block publication even when the current tree is clean.
- The candidate is 18 commits ahead of `origin/main`; release proof must cover the full inherited diff, not only new documentation edits.
- Security/tooling changes can make fork CI unsafe or create an accidental publish path.
- Runtime or packaging evidence becomes stale after code changes and must be regenerated from the final commit.
- A long local green run can still leave hosted GitHub rulesets, security settings, and signed-out public-page verification blocked externally.

## Milestones

### M1. Orientation and reproducible baseline

- Goal: establish exact Git, toolchain, project, artifact, and pre-change verification state.
- Files / systems: Git metadata, repository instructions, manifests, workflows, scripts, docs, local toolchain.
- Changes: record this plan only; preserve all inherited work.
- Verification: inventory commands plus existing setup, format, lint, type/build, test, package, and gate commands where practical.
- Expected result: pre-existing failures and environmental limitations are captured before remediation.

### M2. Public-exposure, security, privacy, and provenance audit

- Goal: find publication blockers without printing sensitive values.
- Files / systems: tracked/untracked tree, reachable refs, Git metadata, workflows, dependencies, vendored/assets/docs content.
- Changes: remove or redact current-tree findings, harden unsafe boundaries, update notices/gates, or document an approval-only remediation.
- Verification: redacted secret scans, sensitive-pattern summaries, dependency advisories/licenses, static workflow/security review, and focused regression tests.
- Expected result: no confirmed active secret or sensitive current-tree data; history/legal findings are explicit approval gates.

### M3. Correctness, product truth, and repository hygiene

- Goal: close release-blocking defects and misleading public/product claims while preserving behavior.
- Files / systems: CLI/TUI runtime, config/error paths, ignore/attributes/editor conventions, docs and screenshot surfaces.
- Changes: smallest test-backed fixes and concrete copy corrections only.
- Verification: focused tests, formatter/linter, runtime interaction smoke, snapshot/screenshot validation, and `git diff --check`.
- Expected result: user-facing states and public claims match observed behavior; no unnecessary cleanup churn.

### M4. Tests, CI, packaging, and contributor experience

- Goal: make the public contribution and release path reproducible and least-privilege.
- Files / systems: test suites, `.github/`, release scripts, README/community/releasing docs, package metadata.
- Changes: repair missing checks, unsafe workflow settings, stale commands/links, packaging contents, or contributor guidance.
- Verification: exact CI-equivalent matrix, release-contract tests, archive inspection, install-from-artifact smoke, docs/link checks, and mocked publication gates.
- Expected result: safe local/fork validation and deterministic release rehearsal from public inputs.

### M5. Final runtime and clean-environment release rehearsal

- Goal: prove the final commit rather than reuse stale evidence.
- Files / systems: final release binary/archive, public Hyperliquid read-only endpoints, recorder/replay/TUI paths, fresh temporary checkout.
- Changes: evidence/report updates only after the runtime is stable.
- Verification: `scripts/check.sh pr`, `scripts/check.sh release`, final bounded public smoke/soak plus replay parity, fresh-checkout documented quickstart, package installation, and final redacted scans.
- Expected result: repo-ready/package-ready/runtime-proven status is backed by exact final-commit evidence.

### M6. Independent review, commits, and publication gate report

- Goal: produce a reviewable local candidate and one consolidated owner approval request.
- Files / systems: complete `origin/main...HEAD` diff, Git status/history, GitHub read-only metadata where accessible.
- Changes: logical local commits containing only intentional files; no push.
- Verification: independent whole-diff review, staged-diff secret check, final GO/NO-GO matrix, exact external/manual next actions.
- Expected result: either `READY AFTER LISTED APPROVALS` or an unsoftened `NOT READY FOR PUBLICATION`, never an unsupported readiness claim.

## Verification

- `scripts/check.sh fast`
- `scripts/check.sh pr`
- `scripts/check.sh release`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo test --workspace --all-targets --all-features --locked --no-fail-fast`
- `cargo build --workspace --release --all-features --locked`
- `cargo audit`
- `cargo deny check`
- `python3 scripts/test-public-surface-gate.py`
- `git diff --check`
- Redacted current-tree and reachable-history secret/privacy scans using available repository tooling.
- Fresh temporary checkout: documented build, quickstart/fixture smoke, release archive build, extracted-binary smoke, package inventory, and documentation link validation.
- Manual smoke: keyboard-driven TUI launch/exit and a bounded read-only public-data run from the final binary; no trading or credentials.

## Decision Log

- 2026-07-20: Use `e287955` as the inherited candidate and `origin/main` at `a44b349` as the comparison point. Dropping the 18 audited commits would discard relevant correctness/security work and contradict the preservation boundary.
- 2026-07-20: Use the user-specified branch `chore/oss-release-readiness` from the clean inherited candidate.
- 2026-07-20: Treat the existing MIT license as repository state to audit, not authority to select, change, or assign its copyright holder.
- 2026-07-20: Keep owner/profile migration and every GitHub/publication mutation outside local authority, even if read-only inspection is available.
- 2026-07-20: Replace remote pipe-to-shell release installers with
  version-locked Cargo registry builds; preserve the generated cargo-dist
  workflow while making the hardener enforce those deltas.
- 2026-07-20: Bind soak acceptance to a deterministic hash of tracked runtime
  inputs rather than commit ancestry, so documentation-only commits do not
  invalidate evidence while any runtime change does.
- 2026-07-20: Keep historical maintainer journals in the repository, add clear
  non-authoritative banners, and exclude them from source release archives;
  deletion would exceed the unambiguous cleanup boundary.

## Progress Log

- 2026-07-20: Completed session bootstrap, request/skill/instruction review, memory continuity pass, remote refresh, Git baseline, and task-branch creation.
- 2026-07-20: Independent documentation/product, CI/package, and
  security/privacy audits completed. Confirmed blockers were unsafe release
  installers, stale/unbound soak evidence, REST redirect/body boundaries,
  remote cleartext WebSocket acceptance, and misleading public capability/write
  claims.
- 2026-07-20: Implemented test-backed network hardening, complete public CLI
  help, short-terminal help safety, evidence schema v2, release workflow
  hardening, source-archive exclusions, privacy/config/platform documentation,
  and community-template improvements.
- 2026-07-20: `scripts/check.sh fast`, full workspace tests, clippy with denied
  warnings, release workspace build, rustdoc with denied warnings, focused
  network/TUI/CLI contracts, workflow hardener, and public-readiness checks pass
  on the uncommitted implementation tree.
- 2026-07-20: Current: review and commit the stable runtime/tooling slice, then
  run the required 15-minute exact-source soak and refresh retained evidence.

## Rollback / Recovery

- If this fails: stop the affected command, preserve its exact redacted failure log, and isolate whether the failure is pre-existing, environmental, or introduced by this branch.
- Safe fallback: revert only task-owned edits with a new patch/commit after review; do not reset, clean, stash, rewrite history, or discard the inherited 18 commits or user files.
