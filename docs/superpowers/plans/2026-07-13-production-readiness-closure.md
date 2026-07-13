# Production Readiness Closure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close all repository-actionable readiness gaps without weakening hlscreen's public, read-only, fail-closed boundary.

**Architecture:** Keep exchange I/O in `hls-hyperliquid`, durable evidence in `hls-store`, pure metric/alert logic in `hls-core` and `hls-features`, and orchestration in `hls-cli`. Every recovery action remains additive: original trade/BBO gaps stay authoritative, candle coverage is labeled coarse, and unavailable private or historical evidence is never synthesized.

**Tech Stack:** Rust 1.88+, Tokio, Reqwest, SQLite/rusqlite, Ratatui, Clap, GitHub Actions, cargo-dist.

## Global Constraints

- Public market data and local files only; no orders, wallets, signing, or private-account access.
- Live paths fail closed on recorder backpressure and preserve every reconnect gap.
- No metric may be labeled canonical without a versioned sampling contract, benchmark fixture, and tolerance.
- Remote HTTP exposure remains disabled until authentication and authorization are separately designed and approved.
- Release publication and multi-day soak completion require real external evidence; configuration alone does not satisfy them.

---

### Task 1: Automatic Coarse Reconnect Repair

**Files:**
- Create: `crates/hls-cli/src/commands/backfill.rs`
- Create: `crates/hls-cli/tests/backfill_command.rs`
- Modify: `crates/hls-cli/src/main.rs`
- Modify: `crates/hls-cli/src/commands/mod.rs`
- Modify: `crates/hls-cli/src/commands/live.rs`
- Modify: `crates/hls-store/src/backfill.rs`
- Modify: `crates/hls-store/tests/backfill_gaps.rs`
- Modify: `specs/005-data-formats-and-backfill/tasks.md`
- Modify: `docs/data-format.md`

**Interfaces:**
- Produces: `hls backfill --run-id <id> --data-dir <path> --interval 1m`.
- Produces: `hls live --record --backfill-gaps` closeout behavior.
- Preserves: `DataGap.recovered == false` for candle-only coverage.

- [x] Write a failing CLI integration test using a loopback HTTP fixture that records a gap, invokes `hls backfill`, and expects one `partially_repaired` attempt plus replay `ReconnectGap` confidence.
- [x] Run `cargo test -p hls-cli --test backfill_command` and confirm the missing command fails.
- [x] Add the backfill command and a prepared REST source that fetches each pending gap/symbol through `HyperliquidRestClient::candle_snapshot`.
- [x] Record per-symbol REST failures as durable unrepaired attempt notes instead of silently dropping them.
- [x] Add `--backfill-gaps`, `--backfill-interval`, and `--rest-url` to live/TUI arguments; require normalized recording and run repair only after clean recorder closeout.
- [x] Run focused store/CLI tests, then strict Clippy and the full workspace suite.
- [x] Commit the independently reviewable recovery slice.

### Task 2: Operations And Soak Evidence

**Files:**
- Create: `scripts/run-supervised-soak.sh`
- Create: `scripts/validate-soak-report.py`
- Create: `tests/fixtures/operations/soak-report-valid.json`
- Create: `tests/fixtures/operations/soak-report-invalid.json`
- Modify: `crates/hls-core/src/telemetry.rs`
- Modify: `crates/hls-core/tests/metrics_contract.rs`
- Modify: `docs/deployment.md`
- Modify: `docs/production-readiness.md`

**Interfaces:**
- Produces: a versioned soak manifest containing commit, command, start/end, CPU/RSS samples, storage growth, reconnects, gaps, parser drops, shutdown state, and replay parity.
- Produces: non-zero validation for incomplete, unclean, or parity-drifting reports.

- [x] Add failing telemetry contract tests for reconnect attempts, parser drops, stale duration, repair latency, and unrepaired-gap duration counters without symbol labels.
- [x] Implement the bounded low-cardinality definitions and validate them through `MetricsRegistry`.
- [x] Add report validation fixtures and prove missing samples, dirty shutdown, or replay drift fail.
- [x] Implement the soak wrapper with signal forwarding, periodic process/resource sampling, disk checks, exact command capture, replay parity, and atomic report publication.
- [x] Add restart, malformed-message, REST-failure, and SIGTERM acceptance commands to deployment documentation.
- [x] Run bounded live and fail-closed fixture smokes; do not mark multi-day soak complete.

### Task 3: Canonical Metric Contracts

**Files:**
- Create: `tests/fixtures/microstructure/canonical_metric_benchmark.json`
- Create: `docs/metric-validation.md`
- Modify: `crates/hls-core/src/metrics.rs`
- Modify: `crates/hls-features/src/metrics.rs`
- Modify: `crates/hls-features/tests/canonical_metrics.rs`
- Modify: `docs/feature-definitions.md`
- Modify: `specs/006-alerts-and-analytics/tasks.md`

**Interfaces:**
- Produces: versioned `MetricSamplingContract` with window, minimum observations, sampling mode, unit, and absolute/relative tolerance.
- Produces: benchmark validation that promotes only sufficiently evidenced metrics to `MetricSupport::Canonical`.

- [ ] Add failing benchmark tests for sampling-contract validation, expected values, and tolerance rejection.
- [ ] Implement time-bucketed canonical formulas only where public trades/BBO provide sufficient observations.
- [ ] Keep Amihud, Roll, bipower variation, and toxicity as proxy/unavailable when their canonical sampling assumptions are unmet.
- [ ] Document formulas, units, provenance, sample floors, tolerances, and known bias.
- [ ] Run golden, property, sparse-data, and non-finite-input tests.

### Task 4: Bounded TUI Alerts And Plugin Ownership

**Files:**
- Modify: `crates/hls-tui/src/app.rs`
- Modify: `crates/hls-tui/src/ratatui_app.rs`
- Modify: `crates/hls-tui/tests/ratatui_cockpit.rs`
- Modify: `crates/hls-cli/src/commands/live.rs`
- Modify: `specs/006-alerts-and-analytics/tasks.md`
- Modify: `docs/feature-definitions.md`

**Interfaces:**
- Produces: bounded newest-first local alert history in the TUI with fixed row and byte limits.
- Produces: explicit plugin execution budget and timeout; no plugin runs on the WebSocket receive critical section.

- [ ] Add a failing deterministic TUI test for bounded alert rows, severity, timestamp, rule, symbol, and reason.
- [ ] Render alert history without nested cards or layout shifts across compact/wide viewports.
- [ ] Add keyboard focus/navigation and preserve pause/selection behavior.
- [ ] Define plugin worker ownership, queue capacity, timeout, failure state, and stale-annotation behavior before live enablement.
- [ ] Add overload and timeout tests proving market ingestion continues or fails closed according to the documented contract.

### Task 5: Release, CI, And Open-Source Proof

**Files:**
- Modify: `.github/workflows/ci.yml`
- Modify: `.github/workflows/release.yml`
- Modify: `scripts/check-release-packaging.sh`
- Modify: `scripts/check-public-readiness.sh`
- Modify: `docs/OPEN_SOURCE_CHECKLIST.md`
- Modify: `docs/RELEASING.md`
- Modify: `docs/production-readiness.md`
- Modify: `specs/003-production-release-and-packaging/tasks.md`

**Interfaces:**
- Produces: pinned/tested macOS runner policy, scheduled public-contract smoke, SBOM/checksum/provenance verification, and clean-runner archive installation proof.

- [ ] Add a failing static workflow contract test for runner policy, least permissions, tag-only publication, SBOM, checksums, and provenance.
- [ ] Harden CI and release workflows without enabling publication from ordinary branches.
- [ ] Verify source archive and supported binaries on clean runners; scan committed assets and docs for private paths or secrets.
- [ ] Run the full local release gate and review generated plans/artifacts.
- [ ] Open and review a PR only after all repository checks pass.
- [ ] Create a `v*` tag only after explicit approval, merged green main, and reviewed generated artifact plan.
- [ ] Keep multi-day soak and public release boxes unchecked until external evidence exists.

## Final Verification

- [ ] Run `cargo fmt --all -- --check`.
- [ ] Run `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`.
- [ ] Run `cargo test --workspace --all-features --locked`.
- [ ] Run release build, rustdoc warnings-as-errors, RustSec, deterministic screenshots, packaging, and public-readiness scans.
- [ ] Run bounded live all-symbol capture, replay parity twice, API/TUI smoke, and log review.
- [ ] Review the complete diff, resolve every valid finding, push, open PR, and wait for green CI before any merge decision.
