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

- [-] PR, review, merge, and close out
  DoD: OSS-readiness branch is reviewed/merged to `main`; plan, TODO, memory, and daily notes reflect final state.
