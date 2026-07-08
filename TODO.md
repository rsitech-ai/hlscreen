# TODO

- [x] Confirm owning repo(s) and affected files
  DoD: File/module scope is listed in `PLAN.md`.

- [x] Initialize local Spec Kit scaffold
  DoD: `hlscreen/.specify/` exists and scripts/templates are available.

- [x] Generate feature specification
  DoD: `spec.md` and its quality checklist exist with no unresolved clarification markers.

- [x] Generate implementation plan and design artifacts
  DoD: `plan.md`, `research.md`, `data-model.md`, contracts, and `quickstart.md` exist.

- [x] Generate dependency-ordered tasks
  DoD: `tasks.md` uses the required task checklist format and maps work by user story.

- [x] Run relevant validation commands
  DoD: Markdown and diff checks pass; task format and Spec Kit active feature pointer are verified.

- [x] Update `MEMORY.md`
  DoD: Durable local project convention is recorded without secrets.

- [x] Final close-out
  DoD: `PLAN.md` final notes complete, `TODO.md` fully checked or explicitly deferred, reflection is closed.

## 2026-07-07 Implementation Slice

- [x] Create Rust workspace and crate skeletons
  DoD: `cargo metadata` can load all planned crates.

- [x] Add config/docs/fixture skeleton
  DoD: default config, docs stubs, and Hyperliquid REST fixtures exist.

- [x] Implement and test `hls-core`
  DoD: config parsing, symbol mapping, and time helpers have passing tests.

- [x] Implement and test fixture-backed REST metadata parsing
  DoD: `hls-hyperliquid` parses spot metadata plus asset contexts from fixtures.

- [x] Implement and test CLI `init`, `doctor`, and `symbols`
  DoD: CLI tests pass and `symbols` can run against fixtures without live credentials.

- [x] Run validation and audit
  DoD: fmt, clippy, tests, diff check, and read-only boundary audit are recorded.

- [x] Push coherent validated slice if checks pass
  DoD: standalone `hlscreen` Git repo is committed and pushed to `s1korrrr/hlscreen.git` without unrelated parent changes.

- [x] Close implementation notes
  DoD: `PLAN.md`, `TODO.md`, `MEMORY.md`, and Spec Kit `tasks.md` reflect actual state.

## 2026-07-07 US1 Live Screener Slice

- [x] Add US1 parser fixtures and tests
  DoD: `trades`, `bbo`, `allMids`, `activeAssetCtx`, and `candle` fixture messages are covered by failing-then-passing tests.

- [x] Implement WS event parsing and subscription budget checks
  DoD: `hls-hyperliquid` returns typed public market events and rejects unsafe subscription budgets.

- [x] Implement live market state and feature snapshots
  DoD: fixed fixture events produce price, TOB, return, volatility/anomaly, score, and stale-state fields.

- [x] Implement TUI table rendering and fixture-backed live command
  DoD: golden table and CLI mock-live tests pass with a visible read-only banner.

- [x] Run validation and read-only audit
  DoD: fmt, clippy, workspace tests, live fixture smoke, and boundary scan pass.

- [x] Commit and push US1 slice
  DoD: validated commits are pushed to `origin/feat/andrzej_hlscreen_foundation`.

## 2026-07-07 US2 Recording/Replay Slice

- [x] Add storage/replay tests
  DoD: raw writer, normalized writer, SQLite registry, and CLI record/replay tests fail before implementation and pass after.

- [x] Implement raw, normalized, metadata, data-gap, recorder, and replay modules
  DoD: fixture recording produces compressed raw files, normalized event files, SQLite metadata, and replay snapshots.

- [x] Implement `hls record` and `hls replay`
  DoD: CLI smoke commands record fixture data and replay rows without network.

- [x] Integrate fixture-backed recording flags into `hls live`
  DoD: `hls live --fixture-file ... --record --raw --normalized --data-dir ... --once` writes files and still renders the table.

- [x] Run validation and read-only audit
  DoD: fmt, clippy, workspace tests, record/replay smoke, and boundary scan pass.

- [x] Commit and push US2 slice
  DoD: validated commits are pushed to `origin/feat/andrzej_hlscreen_foundation`.

## 2026-07-07 US3 Screening Rules Slice

- [x] Add screening tests
  DoD: DSL parser, evaluator, preset, and CLI screen tests fail before implementation and pass after.

- [x] Implement `hls-screen` parser, evaluator, presets, and engine
  DoD: fixed `FeatureSnapshot` rows can be filtered and sorted by built-in presets and custom expressions.

- [x] Implement `hls screen` and live screening integration
  DoD: CLI screen over fixture/replay data and `hls live --preset/--where/--sort` both use the shared engine.

- [x] Run validation and read-only audit
  DoD: fmt, clippy, workspace tests, screen smoke, live preset smoke, and boundary scan pass.

- [x] Commit and push US3 slice
  DoD: validated commits are pushed to `origin/feat/andrzej_hlscreen_foundation`.

## 2026-07-07 US4 Health/Safety Slice

- [x] Add health/API/reconnect tests
  DoD: health state, reconnect simulation, local API, and CLI health tests fail before implementation and pass after.

- [x] Implement shared health, telemetry, and reconnect helpers
  DoD: degraded states and reconnect backoff are represented deterministically and serializably.

- [x] Implement TUI health pane, doctor live health output, and local API wiring
  DoD: health status is visible through TUI text, `hls doctor --live`, and read-only API route helpers/CLI preview.

- [x] Run validation, quickstart, and read-only audit
  DoD: fmt, clippy, workspace tests, quickstart smokes, API/doctor smokes, and boundary scan pass.

- [x] Commit and push US4 slice
  DoD: validated commits are pushed to `origin/feat/andrzej_hlscreen_foundation`.

## 2026-07-08 End-to-End Audit / PR Merge Gate

- [x] Confirm GitHub/base-branch state and audit scope
  DoD: Remote branch/default state and PR strategy are recorded in `PLAN.md`.

- [x] Review code against official docs and Spec Kit contracts
  DoD: REST, WebSocket, heartbeat, rate-limit, CLI, API, data-file, and read-only contracts have explicit pass/fail notes.

- [x] Run expanded static/runtime validation
  DoD: fmt, clippy, tests, build, smokes, edge probes, and scans pass or findings are fixed.

- [x] Fix findings and rerun gates
  DoD: Any correctness, security, maintainability, or docs gaps have focused fixes/tests and fresh green evidence.

- [x] Create PR, review, and merge to `main` if stable
  DoD: PR #1 diff/checks were reviewed, no blocking findings remained, and `main` was merged at `73ebdaa`.

- [x] Final close-out
  DoD: Audit report, `PLAN.md`, `TODO.md`, `MEMORY.md`, daily memory, and lesson stores reflect actual state.

## 2026-07-08 Open Source Readiness

- [x] Add OSS/community files and GitHub automation
  DoD: License, contributing/security/conduct/support/release docs, issue/PR templates, CI, and dependency automation exist.

- [x] Add deterministic screenshots
  DoD: README embeds committed screenshot assets generated from current CLI output.

- [x] Refresh README and supporting docs
  DoD: Public-facing docs explain value, scope, install/build, safety, screenshots, commands, roadmap, and contribution path without overclaiming live trading readiness.

- [x] Run validation and link checks
  DoD: Rust checks pass, fixture screenshot commands run, and local Markdown/image links resolve.

- [x] PR, review, merge, and close out
  DoD: OSS-readiness branch is reviewed/merged to `main`; plan, TODO, memory, and daily notes reflect final state.

## 2026-07-08 Live Smoke / TUI Screenshot Slice

- [x] Confirm official docs, scope, and affected files
  DoD: `PLAN.md` records WebSocket endpoint, heartbeat, subscription budget, live-run scope, and read-only boundary.

- [x] Implement bounded read-only live WebSocket pipeline
  DoD: Public subscriptions, heartbeat, duration shutdown, raw capture, normalized event handling, and budget validation work without private/order surfaces.

- [x] Add deterministic full-pipeline smoke coverage
  DoD: A test or smoke helper exercises fixture live -> record -> replay -> screen -> health and fails on parser/store/output regressions.

- [x] Polish TUI/table output and screenshots
  DoD: Terminal output is screenshot-ready, screenshot assets are regenerated from current CLI output, and README/docs still reference existing files.

- [x] Run validation gates
  DoD: fmt, clippy, tests, build, diff check, link/screenshot checks, and read-only scans pass or have explicit blockers.

- [x] Run 15-minute all-pairs pipeline
  DoD: `hls live --all-symbols --duration-secs 900 ...` exits cleanly or records an exact external/network blocker, then replay/screen evidence is inspected.

- [x] Review, PR, merge, and close out
  DoD: Diff is reviewed, stable branch is pushed, PR is merged only if checks and live evidence are acceptable, and plan/TODO/memory/reflection are complete.

## 2026-07-08 Live Production Hardening

- [x] Confirm official docs, runtime gaps, and live-only production boundary
  DoD: `PLAN.md` records WebSocket reconnect, heartbeat, subscription-cap, low-latency, and no-private-surface constraints.

- [x] Harden WebSocket reconnect/resubscribe and gap handling
  DoD: Server close/read failure reconnects until duration ends, subscriptions are resent, and recording persists explicit data gaps.

- [x] Move live recording off the WebSocket read loop
  DoD: Raw/normalized writes use a bounded worker queue and fail closed on backpressure instead of silently dropping or blocking ingestion.

- [x] Stamp receive timestamps and improve TUI live refresh
  DoD: Normalized events preserve non-zero receive timestamps in live mode, and `--tui`/TTY sessions render live table refreshes.

- [x] Update docs/reports/screenshots for live-first readiness
  DoD: Public docs lead with real public live data, fixture language is clearly test-only, and live smoke evidence is current.

- [x] Run validation and live smoke gates
  DoD: Rust gates, diff/security scans, live public smoke, replay/screen checks, and log review pass or exact blockers are recorded.

- [x] Review, PR, merge, and close out
  DoD: Diff is reviewed, PR checks pass, merge to `main` occurs only if stable, and plan/TODO/memory/reflection are complete.

## 2026-07-08 Next-Gen TUI Polish

- [x] Confirm scope and renderer contract
  DoD: `PLAN.md` records TUI-only scope, read-only boundary, and validation plan.

- [x] Update TUI golden expectations
  DoD: Tests describe the new professional layout and health pane before implementation.

- [x] Implement polished terminal renderer
  DoD: Main table and health pane render a modern, aligned, read-only UI without ANSI escape codes or trading/private surfaces.

- [x] Regenerate and inspect screenshots
  DoD: Committed SVG screenshots are regenerated from current CLI output and show the updated TUI.

- [x] Run validation gates
  DoD: Focused TUI tests, relevant CLI smoke tests, full Rust checks, screenshot generation, and `git diff --check` pass.

- [x] Close out notes
  DoD: `PLAN.md`, `TODO.md`, `MEMORY.md`, daily memory, and lesson stores reflect the final state.

## 2026-07-08 Workstation TUI Refinement

- [x] Confirm renderer scope and visual contract
  DoD: `PLAN.md` records deterministic TUI scope, read-only boundary, and validation commands.

- [x] Upgrade market-board layout
  DoD: The main renderer shows truthful KPI rows, stronger hierarchy, clear units, and an empty state without ANSI-only styling or fake data.

- [x] Upgrade health panel layout
  DoD: Health output separates safety, ingest, and storage state with clear degraded reasons and no private/order wording.

- [x] Regenerate and inspect screenshots
  DoD: SVG screenshots are rebuilt from the current binary and PNG previews are inspected locally.

- [x] Run validation gates
  DoD: Focused TUI/CLI tests, full workspace checks, screenshot generation, `git diff --check`, and read-only scan pass or exact blockers are recorded.

- [x] Close out notes
  DoD: `PLAN.md`, `TODO.md`, `MEMORY.md`, daily memory, and lesson stores reflect the final state.

## 2026-07-08 Microstructure Foundation Contracts

- [x] Confirm v2 task scope and branch
  DoD: `PLAN.md` identifies T001-T019 as the active foundation slice on a fresh feature branch.

- [x] Add setup scaffolding
  DoD: Microstructure fixture/golden directories, fixture README, and `docs/microstructure.md` exist without changing runtime behavior.

- [x] Add failing foundation tests
  DoD: Confidence, score, metrics, benchmark manifest, and CLI safety tests describe public behavior before implementation.

- [x] Implement foundation contracts
  DoD: `hls-core` exposes confidence, score, and metrics modules; `hls-store` exposes benchmark manifests; tests pass.

- [x] Update terminology docs and Spec Kit task markers
  DoD: `docs/feature-definitions.md`, `docs/microstructure.md`, and `specs/002-microstructure-workstation/tasks.md` reflect T001-T019 completion only.

- [x] Run validation gates
  DoD: Focused tests, full Rust gates, build, `git diff --check`, JSONL validation, and read-only scan pass.

- [x] Review, PR, merge, and close out
  DoD: Diff is reviewed, stable branch is pushed, PR checks pass, merge to `main` occurs only if stable, and memory/reflection notes are complete.

## 2026-07-08 Next-Gen Workstation TUI Polish

- [x] Confirm renderer scope and visual contract
  DoD: `PLAN.md` records deterministic TUI scope, read-only boundary, official TUI standard check, and validation commands.

- [x] Update focused TUI golden expectations
  DoD: Market-board and health-pane tests assert the new workstation hierarchy before implementation is complete.

- [x] Implement polished market-board and health rendering
  DoD: The renderer shows stronger KPIs, scan-friendly rows, selected-row detail, clear degraded states, and no fake confidence/advice/order surface.

- [x] Regenerate and inspect screenshots
  DoD: SVG screenshots are rebuilt from the current binary and rendered previews are inspected locally.

- [x] Run validation gates
  DoD: Focused TUI tests, relevant CLI tests, full Rust gates, screenshot generation, `git diff --check`, and read-only scan pass.

- [x] Close out notes
  DoD: `PLAN.md`, `TODO.md`, `MEMORY.md`, daily memory, lesson store, and reflection entry reflect the final state.

## 2026-07-08 US1 Confidence and Replay Parity

- [x] Confirm US1 scope and branch
  DoD: `PLAN.md` identifies T020-T033 as the active slice on `feat/andrzej_microstructure_confidence_replay`.

- [x] Add US1 fixtures and failing tests
  DoD: Gap/sparse fixtures and focused tests for duplicate confidence, replay parity, CLI parity flag, and TUI confidence rendering describe behavior before implementation.

- [x] Implement confidence on feature snapshots
  DoD: `FeatureSnapshot` carries `DataConfidenceSnapshot`, feature engine computes state-derived and runtime-input confidence, and duplicate events are counted without applying duplicate trades.

- [x] Implement persistence and replay parity
  DoD: SQLite stores confidence snapshots and replay parity detects persisted baseline drift with clear reports.

- [x] Wire CLI/TUI/docs and Spec Kit task markers
  DoD: `hls replay --verify-parity`, live/replay confidence summaries, TUI confidence row rendering, `docs/microstructure.md`, and T020-T033 markers reflect real behavior.

- [x] Run validation gates
  DoD: Focused tests, full Rust gates, screenshot generation, `git diff --check`, and read-only scan pass.

- [x] Review, PR, merge, and close out
  DoD: Diff is reviewed, branch is pushed, PR checks pass, merge to `main` occurs only if stable, and memory/reflection notes are complete.

## 2026-07-08 US2 Liquidity Resilience TUI

- [x] Confirm scope and branch
  DoD: `PLAN.md` identifies T034-T045 as the active slice on `feat/andrzej_microstructure_resilience_tui`.

- [x] Add US2 fixtures and failing tests
  DoD: Spread-shock and thin-book fixtures plus resilience, tradeability, and preset tests describe expected behavior.

- [x] Implement resilience and tradeability features
  DoD: Feature snapshots carry BBO-only spread shock, recovery, tradeability, BBO OFI proxy, and signed flow fields computed from public events.

- [x] Wire screen DSL, presets, TUI, and docs
  DoD: New fields are filterable/sortable where specified, presets exist, TUI renders the fields, and docs explain BBO-only caveats.

- [x] Regenerate and inspect screenshots
  DoD: SVG screenshots are rebuilt from the current binary and visually inspected, with no fake production data path.

- [x] Run validation gates
  DoD: Focused tests, full Rust gates, screenshot generation, `git diff --check`, read-only scan, and bounded public live smoke pass or exact blockers are recorded.

- [x] Review, PR, merge, and close out
  DoD: Diff is reviewed, branch is pushed, PR checks pass, merge to `main` occurs only if stable, and memory/reflection notes are complete.

## 2026-07-08 US3 Why-Ranked TUI Detail

- [x] Confirm scope and renderer contract
  DoD: `PLAN.md` identifies T046-T057 as the active slice on `feat/andrzej_microstructure_explainability`.

- [x] Add US3 fixtures and failing tests
  DoD: score fixture, core aggregation tests, TUI why-ranked golden, and CLI explain tests describe expected behavior.

- [x] Generate score breakdowns from public feature evidence
  DoD: `FeatureSnapshot` carries deterministic score breakdowns with positive, negative, confidence-adjusted, and unavailable-evidence fields.

- [x] Wire screen DSL, TUI detail pane, CLI explain, and docs
  DoD: score fields are filterable/sortable, `render_why_ranked_pane` exists, `hls explain` works over replay/fixtures, and docs explain caveats.

- [x] Regenerate and inspect screenshots
  DoD: SVG screenshots include a why-ranked pane generated from the current binary and are visually inspected.

- [x] Run validation gates
  DoD: Focused tests, full Rust gates, screenshot generation, `git diff --check`, and read-only scan pass or exact blockers are recorded.

- [x] Review, PR, merge, and close out
  DoD: Diff is reviewed, branch is pushed, PR checks pass, merge to `main` occurs only if stable, and memory/reflection notes are complete.

## 2026-07-08 US4 Metadata-Backed TUI Polish

- [x] Confirm US4 scope and renderer contract
  DoD: `PLAN.md` identifies T058-T068 plus screenshot polish as the active slice on `feat/andrzej_microstructure_metadata`.

- [x] Add US4 fixtures and failing tests
  DoD: Complete, partial, and missing metadata payloads are covered by core, adapter, store, and screen preset tests.

- [x] Implement metadata enrichment and cache support
  DoD: Core snapshots can carry optional metadata, public REST parsing tolerates partial fields, and SQLite records freshness.

- [x] Wire screen fields, presets, TUI, and screenshots
  DoD: Metadata fields are filterable/sortable, TUI shows metadata tags/details, and generated SVG screenshots include the updated workstation surface.

- [x] Run validation gates
  DoD: Focused tests, full Rust gates, screenshot generation, PNG preview, `git diff --check`, and read-only scan pass or exact blockers are recorded.

- [x] Review, PR, merge, and close out
  DoD: Diff is reviewed, branch is pushed, PR checks pass, merge to `main` occurs only if stable, and memory/reflection notes are complete.
