# End-to-End Audit and Release Review

## Goal

- User-visible outcome: turn the current `origin/main` implementation into a production-ready candidate for its documented read-only local deployment scope, repair every confirmed audit defect, and merge an approved PR to `main` only after local, runtime, documentation, and hosted gates are green.
- How to see it working: canonical release checks pass; fixture and bounded public live flows start, operate, stop, and leave clean logs; the final PR has no unresolved review findings and its required checks pass before merge.

## Current State

- Relevant paths: Rust workspace under `crates/`, release and smoke tooling under `scripts/`, acceptance contracts under `specs/`, runtime evidence and operator documentation under `docs/`.
- Existing behavior: `origin/main` at `a44b349` contains the bounded live workstation and open-source hardening; the previous local TUI checkpoint branch is not the publication baseline and is not an ancestor of current `origin/main`.
- Constraints: read-only public Hyperliquid data only; no wallet, private stream, order, or trade actions; preserve deterministic non-TTY behavior; use fresh evidence; do not merge if any material finding or required hosted check remains unresolved.

## Target State

- Desired behavior: inputs are validated at boundaries, pure logic remains separate from I/O, rate limits and resource use are bounded, evidence remains truthful across reconnect/replay/backfill, service and recorder lifecycles terminalize cleanly, operator-visible status matches the specs, dependencies and workflows match official contracts, and all supported flows are warning-free and reproducible.
- Non-goals: adding trading/execution capability, claiming unattended production readiness, rewriting Git history, publishing a release tag, or broad feature expansion unrelated to confirmed audit findings.

## Risks and Failure Modes

- A green unit suite may miss TTY lifecycle, resize, reconnect, recording, API, signal, or port-release failures.
- Public API drift or dependency-version drift may invalidate parser, WebSocket, terminal, or workflow assumptions.
- Long-running or network tests may be flaky; each failure must be classified with exact logs and reproduced before code changes.
- The GitHub CLI is unavailable locally, so authenticated connector APIs and Git transport must be used for PR review and merge gates.
- Hosted CI or repository policy may remain an external blocker even when local validation is green.

## Milestones

### M1. Establish scope and architecture truth

- Goal: map the current implementation, contracts, risk boundaries, dependencies, entrypoints, and exact comparison point.
- Files / systems: `Cargo.toml`, crate manifests and sources, `specs/`, `README.md`, `docs/`, `.github/workflows/`, `scripts/`.
- Changes: none unless a documentation or contract defect is confirmed.
- Verification: dependency/target inventory, source and test inventory, `git diff origin/main...HEAD`, and independent architecture/security review.
- Expected result: a traceable verification matrix and prioritized findings.

### M2. Static, deterministic, and security validation

- Goal: catch compile, lint, test, documentation, packaging, dependency, workflow, dead-code, and policy defects.
- Files / systems: full workspace and release tooling.
- Changes: focused tests and minimal fixes for reproduced defects.
- Verification: `scripts/check.sh fast`, `scripts/check.sh pr`, `scripts/check.sh release`, targeted boundary tests, and diff hygiene.
- Expected result: zero warnings or failures, or an exact blocker with root cause.

#### Remediation tasks

1. Preserve unrecovered reconnect-gap evidence and bound filter-parser nesting.
2. Enforce the selected-symbol boundary and keep candle anomaly baselines interval-consistent.
3. Make recorder initialization/closeout terminalize failed runs and make TUI preference persistence symlink-safe and atomic.
4. Enforce Hyperliquid REST weight and WebSocket connection limits with cancellable waits.
5. Bound analog-index CPU and memory growth with deterministic sampling/caps.
6. Bound hosted validation subprocesses, harden supervisor smoke isolation/lifecycle proof, support clean SIGTERM, and reconcile stale design/deployment documentation.

### M3. Runtime and integration proof

- Goal: prove supported fixture, replay, server, TUI, recording, shutdown, and bounded public-data flows.
- Files / systems: CLI binaries, fixture packs, temporary data directories, loopback services, public Hyperliquid REST/WebSocket boundaries.
- Changes: minimal lifecycle/integration fixes plus regression tests when a defect is confirmed.
- Verification: quickstart commands, PTY smoke, resize/input/exit checks, loopback API health and port release, bounded public session, replay/parity, and clean log inspection.
- Expected result: clean startup, stable behavior, deterministic shutdown, no orphan services, no hidden errors, and no invented success claims.

### M4. Independent review and remediation

- Goal: review the complete candidate for correctness, maintainability, readability, security, performance, and best practices.
- Files / systems: all changes against `origin/main` plus high-risk unchanged paths found during the audit.
- Changes: resolve every material finding and add regression proof.
- Verification: focused reproductions followed by the full relevant matrix.
- Expected result: no unresolved P0-P2 findings and an evidence-backed `ready` decision.

### M5. PR, hosted gates, and merge

- Goal: publish the reviewed candidate and merge only after hosted approval.
- Files / systems: Git remote and GitHub PR/check/review surfaces.
- Changes: commit intentional files, push `feat/andrzej_full_audit_20260717`, open PR to `main`, address feedback, and merge through the PR.
- Verification: remote SHA equality, PR diff review, required check status, review-thread status, merge result, and refreshed `origin/main` ancestry.
- Expected result: PR merged and local/remote `main` contains the verified candidate; otherwise report the precise external blocker without bypassing it.

## Verification

- `scripts/check.sh fast`
- `scripts/check.sh pr`
- `scripts/check.sh release`
- Targeted crate tests and boundary probes selected from the architecture/risk inventory.
- `python3 scripts/generate-screenshots.py --check`
- Fixture-backed CLI/TUI/PTTY smoke with clean exit and captured stderr.
- Bounded public Hyperliquid metadata/WebSocket recording smoke followed by replay parity.
- Loopback service health, signal shutdown, and port-release proof when supported by the current scripts.
- GitHub PR diff, checks, reviews, threads, and merge-state verification.

## Decision Log

- 2026-07-17: Use freshly fetched `origin/main` (`a44b349`) as the audit/publication baseline because the checked-out TUI checkpoint has a deleted upstream and is not an ancestor of the current default branch.
- 2026-07-17: Keep the audit inside the read-only public-data trust boundary; no trades or private-account access are authorized or needed.
- 2026-07-17: Use generic read-only subagents for independent documentation, architecture/security, and runtime/test review; the parent remains the only writer and owns final verification.
- 2026-07-17: After validated findings expanded implementation scope, switch to serialized subagent-driven development: one assigned writer at a time with non-overlapping ownership, followed by an independent task review; the parent owns integration and final verification.
- 2026-07-17: Treat `production-ready` as the repository's documented read-only local-service scope. Wallets, private streams, orders, and live-money execution remain prohibited and out of scope.
- 2026-07-17: Reserve explicit headroom below Hyperliquid's documented IP limits: 1,100 weighted REST units/minute versus 1,200, 29 new WebSocket connections/minute versus 30, and 1,900 outbound WebSocket messages/minute versus 2,000.
- 2026-07-17: Bound analog replay to deterministic five-minute samples and the 288 newest candidates per symbol. Cache accepted replay time and snapshot revision in market state so cadence decisions remain O(1) and ignored events do not trigger all-symbol recomputation.

## Progress Log

- 2026-07-17: Completed HQ bootstrap, repository/remote inventory, plan/spec intake, baseline selection, and fresh audit branch creation.
- 2026-07-17: Baseline `scripts/check.sh fast` passed; 249 additional focused tests and deterministic fixture/service smokes passed.
- 2026-07-17: Reproduced and fixed reconnect-gap recovery truth and filter-parser stack exhaustion using failing-first regression tests; focused tests are green.
- 2026-07-17: Completed and independently approved selected-universe/candle-interval correctness, recorder/preference lifecycle hardening, and bounded public REST/WebSocket request behavior.
- 2026-07-17: Completed and independently approved bounded analog replay after closing per-event all-symbol scans, terminal duplicate sweeps, ignored-event recomputation, and immediately-evicted-history edge cases. Core plus store verification passed 100 tests with warnings denied.
- 2026-07-17: In progress: service signal lifecycle, validation timeout/isolation, supervisor process proof, and documentation truth. Next: run the full runtime/release matrix and final whole-branch review.

## Rollback / Recovery

- If this fails: stop services, preserve logs and exact temporary evidence paths, keep the candidate branch intact, and report the failing command plus root cause.
- Safe fallback: do not merge; leave `main` untouched and retain only focused reviewable commits on the audit branch. Revert any candidate change through a new commit rather than discarding user work.
