# PLAN

## Task
- Objective: Implement and validate the first coherent Rust slice for the read-only Hyperliquid spot screener: workspace setup, core config/symbol primitives, fixture-backed REST metadata parsing, and CLI `init`/`doctor`/`symbols`.
- Owner repo(s): standalone `hlscreen/` project folder inside the dirty `rsibot/` workspace. Do not mutate parent `rsibot/`, `hummingbot/`, `hummingbot-api/`, or `quants-lab/` work.
- Capital impact: research-only / read-only market-data infrastructure. No wallet, trading, execution, order routing, credential changes, live-service restart, or order-capable API.

## Context
- Background: `hlscreen/` now has a complete Spec Kit package for a terminal-first Hyperliquid spot market data recorder and screener. The next value slice is making the foundation compile, test, and run locally.
- Inputs: `specs/001-hyperliquid-spot-screener/{spec.md,plan.md,tasks.md,contracts/,quickstart.md}`.
- Outputs: Cargo workspace, shared crates, tests, config/docs skeleton, fixture-backed CLI commands, validation evidence, and a pushable standalone Git history if checks pass.

## Assumptions
- `hlscreen/` should be pushable to `https://github.com/s1korrrr/hlscreen.git` as a standalone repository after validation.
- The first implementation slice should stay fixture-backed where possible; live network smoke is optional and must remain read-only.
- Rust edition 2024 with `rust-version = "1.85"` is acceptable for new manifests.

## Constraints
- Technical: follow the generated task order; use TDD for meaningful behavior; keep crate boundaries explicit.
- Operational: do not touch existing dirty parent repo changes; initialize/push only the `hlscreen/` project if the slice validates.
- Risk/capital: no private keys, no wallet connection, no order placement, no trading endpoints, no market predictions, and no score-as-signal language.

## Options Considered
1. Implement only Cargo workspace scaffolding and stop.
   - Pros: tiny diff, fastest validation.
   - Cons: not enough product behavior to audit or justify a push beyond planning.
2. Implement setup plus foundation CLI/metadata parsing from fixtures.
   - Pros: creates a real, testable vertical slice with no live-capital risk.
   - Cons: does not yet deliver the full live TUI, recording, replay, or DSL stories.

## Chosen Approach
- Choice: option 2.
- Why: it gives the project a tested backbone while keeping the blast radius small and read-only.

## Execution Plan
1. Create Cargo workspace and crate skeletons.
2. Add config/docs/test fixture skeleton.
3. Add failing tests for config loading, symbol mapping, REST metadata parsing, and CLI basics.
4. Implement `hls-core`, fixture-backed `hls-hyperliquid` metadata parsing, and `hls-cli` commands.
5. Update `tasks.md` as completed tasks become true.
6. Run `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`.
7. Audit read-only boundaries, diff scope, and pushable Git state.
8. Update memory and close local plan/TODO.

## Test Plan
- Unit: `cargo test -p hls-core`; `cargo test -p hls-hyperliquid`.
- Integration/smoke: `cargo test -p hls-cli`; `cargo run -p hls-cli -- symbols --top 2 --metadata-file tests/fixtures/hyperliquid/spot_meta.json --asset-contexts-file tests/fixtures/hyperliquid/spot_meta_and_asset_ctxs.json`.
- Regression/audit: `cargo fmt --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace`; `git diff --check`.

## Risks and Rollback
- Risks: CLI/metadata contracts may need adjustment when live Hyperliquid smoke is added; dependency compile time may be non-trivial; this slice does not prove WebSocket/TUI/recording behavior.
- Rollback: revert the `hlscreen/` implementation files and keep the Spec Kit artifacts, or reset the standalone `hlscreen` repo before pushing if validation fails.

## Memory Impact
- Add/update in `MEMORY.md`: confirmed Rust commands, fixture-backed CLI usage, and durable read-only/project boundaries.

## Final Notes
- What changed: Created a Rust 2024 Cargo workspace with all planned crates; implemented `hls-core` errors/config/symbol/time helpers; implemented fixture-backed Hyperliquid REST metadata parsing and public REST client methods; implemented CLI `init`, `doctor`, and `symbols`; added config/docs/fixtures/README; kept wallet/trading/order surfaces unavailable.
- Validation run: `cargo metadata --format-version 1 --no-deps`; red/green `cargo test -p hls-core --test config_symbol`; red/green `cargo test -p hls-hyperliquid --test rest_metadata`; red/green `cargo test -p hls-cli --test basic_commands`; `cargo fmt --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace`; `cargo build --workspace`; `git diff --check -- hlscreen`; fixture-backed `./target/debug/hls symbols --top 2 --asset-contexts-file tests/fixtures/hyperliquid/spot_meta_and_asset_ctxs.json`; temp-dir `hls init` and `hls doctor`.
- Follow-ups: Pushed commit `705f000` to `origin/feat/andrzej_hlscreen_foundation`. US1 remains open: WebSocket parser/subscription manager, live market state, feature formulas, TUI table, and `hls live`. US2 recording/replay, US3 rules/DSL, and US4 health/API are not implemented yet.

## 2026-07-07 US1 Live Screener Slice

### Task
- Objective: Implement the next US1 mock-live slice: Hyperliquid WebSocket fixture parsing, subscription budget checks, live market state updates, feature formulas/snapshots, terminal table rendering, and fixture-backed `hls live`.
- Owner repo(s): standalone `hlscreen/` repository only.
- Capital impact: research-only / read-only public market data. No wallet, private-key, order, exchange, or live trading surfaces.

### Context
- Background: Foundation tasks T001-T018 are complete and pushed. US1 starts at T019 with parser, feature, TUI, and mock-live tests before implementation.
- Inputs: `specs/001-hyperliquid-spot-screener/tasks.md`, `data-model.md`, `contracts/cli.md`, and current official Hyperliquid WebSocket docs.
- Outputs: Completed T019-T032 where feasible, validation evidence, and pushed commit(s).

### Assumptions
- Fixture-backed `hls live --fixture-file ...` is acceptable for this slice because the task checkpoint calls for mock live tests before real live TUI/network hardening.
- The live command remains read-only and will not subscribe to private/user streams.
- Top-of-book metrics use only `bbo`, not `l2Book`, and must be labeled honestly.

### Constraints
- Technical: maintain crate boundaries; avoid blocking read/parsing paths on rendering or storage in future-compatible API shape.
- Operational: keep parent `rsibot` untouched; push only the standalone `hlscreen` branch after validation.
- Risk/capital: no exchange endpoint, wallet, key, order, leverage, or predictive-signal semantics.

### Options Considered
1. Implement only WebSocket parser fixtures.
   - Pros: minimal and low-risk.
   - Cons: does not move the visible live screener toward the US1 checkpoint.
2. Implement a fixture-backed mock live pipeline through parser -> market state -> feature snapshot -> table -> CLI.
   - Pros: proves the main US1 data path without live network fragility.
   - Cons: real WebSocket connection/reconnect behavior remains for later US4/T060 work.

### Chosen Approach
- Choice: option 2.
- Why: it creates a testable vertical slice of the actual user value while preserving read-only safety and deterministic validation.

### Execution Plan
1. Add US1 fixture messages and parser tests for `trades`, `bbo`, `allMids`, `activeAssetCtx`, and `candle`.
2. Implement core event/state/snapshot types plus WebSocket parser/subscription budget helpers.
3. Add and implement feature formula tests for spread, TOB depth/imbalance, returns, z-scores, and bounded scores.
4. Add and implement terminal table golden rendering.
5. Add fixture-backed mock live integration through `hls live`.
6. Run fmt, clippy, tests, smoke commands, and read-only audit.
7. Update tasks/memory/notes, commit, and push.

### Test Plan
- Unit: `cargo test -p hls-hyperliquid --test ws_parser`; `cargo test -p hls-features --test formulas`; `cargo test -p hls-tui --test main_table_golden`.
- Integration/smoke: `cargo test -p hls-cli --test live_mock`; `./target/debug/hls live --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --once`.
- Regression/audit: `cargo fmt --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace`; `rg` read-only boundary scan.

### Risks and Rollback
- Risks: public WebSocket payload shape can drift; fixture-backed live does not prove network reconnection or throughput; simple table rendering is not the final interactive TUI.
- Rollback: revert this slice commit while keeping the foundation branch usable.

### Memory Impact
- Add/update in `MEMORY.md`: confirmed mock-live command, supported WS fixture channels, and any known drift between docs and implementation.

### Final Notes
- What changed: Implemented US1 mock-live pipeline from public WebSocket fixture envelopes through typed events, subscription budget checks, live market state, feature snapshots, stable terminal table rendering, and `hls live --fixture-file ... --once`.
- Validation run: `cargo test -p hls-hyperliquid --test ws_parser`; `cargo test -p hls-features --test formulas`; `cargo test -p hls-tui --test main_table_golden`; `cargo test -p hls-cli --test live_mock`; `cargo fmt --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace`; `cargo build --workspace`; `./target/debug/hls live --symbols @107 --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --once`; read-only boundary scan.
- Follow-ups: Pushed commit `40c8718` to `origin/feat/andrzej_hlscreen_foundation`. US2 recording/replay starts at T033. Real WebSocket connection/reconnect and health behavior remain open for US4/T060 rather than being claimed by this fixture-backed slice.

## 2026-07-07 US2 Recording/Replay Slice

### Task
- Objective: Implement the next US2 fixture-backed local recording and replay path: compressed raw public messages, normalized event files, SQLite metadata registry, recorder orchestration, replay snapshots, and `hls record`/`hls replay`.
- Owner repo(s): standalone `hlscreen/` repository only.
- Capital impact: research-only / read-only public market data. No wallet, private-key, order, exchange, or live trading surfaces.

### Context
- Background: US1 mock-live is complete and pushed. US2 starts at T033 with local storage tests and ends when a fixture recording writes raw/normalized files, registry metadata, and replay rebuilds screen rows.
- Inputs: `contracts/data-files.md`, `data-model.md`, `quickstart.md`, and the existing `ws_mock_live.ndjson` fixture.
- Outputs: Store crate modules, CLI record/replay commands, tests, docs/memory updates, validation evidence, and pushed commit(s).

### Assumptions
- The first normalized writer can use deterministic newline-delimited JSON for replayable `MarketEvent` rows while the Parquet-specific writer remains a storage-hardening follow-up. Do not claim Parquet completion unless real Parquet writing exists.
- Raw writer uses compressed `.ndjson.zst` files to match the raw capture contract.
- SQLite registry is local-only and stores no secrets.

### Constraints
- Technical: preserve exact enough raw payloads for parser replay; keep file metadata unique and committed before reporting clean shutdown.
- Operational: keep parent `rsibot` untouched; push only the standalone `hlscreen` branch after validation.
- Risk/capital: no private/user streams, wallet inputs, order parameters, or execution actions.

### Options Considered
1. Implement only raw writer and registry.
   - Pros: small, low-risk.
   - Cons: does not satisfy the US2 replay checkpoint.
2. Implement fixture-backed record and replay across raw, normalized events, metadata, and CLI.
   - Pros: proves US2 operator workflow without live network.
   - Cons: normalized Parquet is deferred and must be documented honestly.

### Chosen Approach
- Choice: option 2.
- Why: it creates a user-visible recording/replay flow while keeping storage semantics reproducible and read-only.

### Execution Plan
1. Add failing tests for raw writer rotation/flush, normalized event writer, SQLite metadata registry, and replay equivalence.
2. Implement `hls-core::data_gap` and `hls-store` raw/normalized/metadata/recorder/replay modules.
3. Wire `hls record` and `hls replay` with fixture-backed deterministic commands.
4. Add optional record integration to fixture-backed `hls live`.
5. Run fmt, clippy, tests, record/replay smoke, and read-only audit.
6. Update task markers, docs, memory, commit, and push.

### Test Plan
- Unit: `cargo test -p hls-store --test raw_writer`; `cargo test -p hls-store --test normalized_writer`; `cargo test -p hls-store --test metadata_registry`.
- Integration/smoke: `cargo test -p hls-cli --test record_replay`; `./target/debug/hls record --symbols @107 --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --raw --normalized --data-dir <tmp>`; `./target/debug/hls replay --data-dir <tmp> --run-id <id>`.
- Regression/audit: `cargo fmt --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace`; read-only boundary scan.

### Risks and Rollback
- Risks: normalized files are JSONL rather than Parquet in this slice; replay from compressed raw and normalized files must not hide parsing errors; SQLite bundled dependency can add compile time.
- Rollback: revert the US2 commit while preserving the US1 branch.

### Memory Impact
- Add/update in `MEMORY.md`: raw file format command, registry path, record/replay smoke commands, and Parquet deferral.

### Final Notes
- What changed: Implemented fixture-backed US2 local recording/replay. Added compressed raw public message writing, deterministic normalized replay JSONL, SQLite metadata for runs/files/symbols/data gaps, bounded raw-writer channel orchestration with clean shutdown, replay snapshot rebuilding, `hls record`, `hls replay`, and fixture-backed `hls live --record`.
- Validation run: `cargo test -p hls-store --test raw_writer`; `cargo test -p hls-store --test normalized_writer`; `cargo test -p hls-store --test metadata_registry`; `cargo test -p hls-cli --test record_replay`; `cargo fmt --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace`; `cargo build --workspace`; record smoke under `<tmp>`; replay smoke under `<tmp>`; live-record smoke under `<tmp>`; `git diff --check`; read-only boundary scan.
- Tradeoffs: Normalized events are JSONL in this slice, not Parquet. The CLI rejects `--parquet` until a real Parquet writer exists.
- Rollback: revert the US2 implementation commit(s); the pushed US1 branch remains usable.
- Follow-ups: Pushed commit `764bacc` to `origin/feat/andrzej_hlscreen_foundation`. US3 screening DSL/presets or a storage-hardening slice for true Parquet and live network recording remain open.

## 2026-07-07 US3 Screening Rules Slice

### Task
- Objective: Implement US3 filtering and sorting over feature snapshots with a small safe DSL, built-in presets, CLI `hls screen`, and live table screening integration.
- Owner repo(s): standalone `hlscreen/` repository only.
- Capital impact: research-only / read-only public market-data screening. No wallet, private-key, order, exchange, or live trading surfaces.

### Context
- Background: US1 live fixture rows and US2 replay snapshots are complete and pushed. US3 starts at T046 and should make those rows screenable by preset or custom rule.
- Inputs: `contracts/screen-rule-dsl.md`, `data-model.md`, `tasks.md`, existing `FeatureSnapshot` rows, and existing record/replay/live fixture paths.
- Outputs: `hls-screen` parser/evaluator/presets/engine, CLI `screen` command, live command screening integration, tests, docs/memory updates, validation evidence, and pushed commit(s).

### Assumptions
- The first `hls screen` command can operate over local replay data and hidden deterministic fixtures; live network screening remains covered by `hls live` and later real-network work.
- Presets are deterministic heuristics for row inspection only and must not be described as trading signals or predictions.
- Invalid rule expressions should return clear errors and must not mutate the library's active screen state.

### Constraints
- Technical: keep DSL deterministic, small, and dependency-light; use public `FeatureSnapshot` fields; keep TUI/CLI as adapters over `hls-screen`.
- Operational: keep parent `rsibot` untouched; push only the standalone `hlscreen` branch after validation.
- Risk/capital: no exchange actions, no predictive edge claims, no private/user streams.

### Options Considered
1. Implement only preset names as hard-coded filters in the CLI.
   - Pros: fastest visible result.
   - Cons: misses the DSL contract and makes TUI/CLI duplicate behavior.
2. Implement `hls-screen` as the shared parser/evaluator/preset engine and call it from CLI/live rendering.
   - Pros: one tested behavior boundary for CLI and TUI surfaces.
   - Cons: larger slice than CLI-only filtering.

### Chosen Approach
- Choice: option 2.
- Why: US3 is explicitly about reusable rules and presets; a shared library avoids false parity between commands.

### Execution Plan
1. Add failing tests for DSL parsing, DSL evaluation, and built-in presets.
2. Implement screen row/field/sort models and parser/evaluator modules.
3. Implement built-in presets and filtering/sorting engine.
4. Add `hls screen` over replayed or fixture events.
5. Route `hls live --preset/--where/--sort` through the shared screen engine and TUI renderer.
6. Run fmt, clippy, targeted tests, workspace tests, smoke commands, and read-only audit.
7. Update task markers, docs, memory, commit, and push.

### Test Plan
- Unit: `cargo test -p hls-screen --test dsl_parser`; `cargo test -p hls-screen --test dsl_evaluator`; `cargo test -p hls-screen --test presets`.
- Integration/smoke: `cargo test -p hls-cli --test screen_command`; fixture-backed `hls screen --fixture-file ...`; fixture-backed `hls live --preset ...`.
- Regression/audit: `cargo fmt --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace`; read-only boundary scan.

### Risks and Rollback
- Risks: DSL parser can grow too broad; missing numeric fields must not accidentally match; preset names could be mistaken for strategy recommendations.
- Rollback: revert the US3 commit(s) while preserving US1/US2.

### Memory Impact
- Add/update in `MEMORY.md`: supported DSL syntax, built-in preset command examples, and rule/preset boundary as screening heuristics only.

### Final Notes
- What changed: Implemented US3 screening rules with `hls-screen` DSL AST/parser/evaluator, row field extraction, sort models, built-in presets, filtering/sorting engine, session state that keeps active rows when invalid expressions fail, `hls screen`, and fixture-backed `hls live --preset/--where/--sort`.
- Validation run: `cargo test -p hls-screen --test dsl_parser`; `cargo test -p hls-screen --test dsl_evaluator`; `cargo test -p hls-screen --test presets`; `cargo test -p hls-cli --test screen_command`; `cargo test -p hls-cli --test live_mock`; `cargo fmt --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace`; `cargo build --workspace`; custom screen smoke; preset screen smoke; replay-backed screen smoke; live preset smoke; `git diff --check`; read-only/no-prediction boundary scan.
- Tradeoffs: `hls screen` is fixture/replay backed in this slice. Real live network screening waits on the later network connection work, and interactive keyboard filter editing remains future TUI work.
- Rollback: revert the US3 commit(s); US1/US2 remain usable.
- Follow-ups: Pushed commit `9c478f8` to `origin/feat/andrzej_hlscreen_foundation`. US4 health/safety, real network connection/reconnect, interactive TUI editing, or true Parquet storage remain open.

## 2026-07-07 US4 Health/Safety Slice

### Task
- Objective: Implement US4 health and safety monitoring: health/telemetry models, deterministic reconnect/backoff behavior, TUI health rendering, `doctor --live` health output, and a read-only localhost API surface.
- Owner repo(s): standalone `hlscreen/` repository only.
- Capital impact: research-only / read-only public market-data health reporting. No wallet, private-key, order, exchange action, or live trading surface.

### Context
- Background: US1-US3 are implemented and pushed. US4 starts at T056 and should surface degraded connection, stale data, writer lag, gaps, and read-only safety status in tests, CLI/TUI, and local API contracts.
- Inputs: `tasks.md` T056-T065, `contracts/local-http-api.md`, `spec.md` health requirements, `research.md` heartbeat/backoff decision, existing `LiveMarketState`/`FeatureSnapshot` paths.
- Outputs: health/telemetry modules, connection health simulator, TUI health pane rendering, server response helpers, CLI `server` command, `doctor --live` health details, tests, docs/memory updates, validation evidence, and pushed commit(s).

### Assumptions
- This slice can validate heartbeat/reconnect behavior with deterministic state transitions instead of opening a real external WebSocket; live exchange network checks stay read-only and bounded to existing public REST behavior.
- The local API can be implemented as a read-only response/router module plus a CLI endpoint preview in this slice; no long-running public service or non-local bind is required for proof.
- Health metrics are operational status and must not imply trading safety or profitability.

### Constraints
- Technical: keep models serializable and dependency-light; avoid blocking async handlers; keep all API routes read-only.
- Operational: keep parent `rsibot` untouched; push only the standalone `hlscreen` branch after validation.
- Risk/capital: no private/user streams, wallet inputs, order parameters, or exchange actions.

### Options Considered
1. Add only CLI `doctor --live` text fields.
   - Pros: small.
   - Cons: misses the shared health model, TUI pane, reconnect/backoff, and API contract tasks.
2. Implement shared health/telemetry/reconnect models with thin TUI/CLI/API adapters.
   - Pros: one testable boundary for all US4 surfaces.
   - Cons: larger diff than a CLI-only health readout.

### Chosen Approach
- Choice: option 2.
- Why: US4 is about consistent operational truth; shared models prevent each surface from inventing its own degraded-state semantics.

### Execution Plan
1. Add failing tests for health state transitions, telemetry/lag, reconnect/backoff simulation, and read-only API responses.
2. Implement `hls-core::health` and `hls-core::telemetry`.
3. Implement `hls-hyperliquid::ws::connection` deterministic heartbeat/reconnect helpers.
4. Implement `hls-tui::health` rendering.
5. Implement `hls-server` read-only API response/router helpers and `hls server` command wiring.
6. Extend `hls doctor --live` with health output.
7. Run fmt, clippy, tests, quickstart/smoke commands, read-only audit, update tasks/memory/docs, commit, and push.

### Test Plan
- Unit: `cargo test -p hls-core --test health_state`; `cargo test -p hls-server --test read_only_api`; `cargo test -p hls-hyperliquid --test reconnect_heartbeat`.
- Integration/smoke: `cargo test -p hls-cli --test health_commands`; `hls doctor --live --json`; `hls server --print-health`.
- Regression/audit: `cargo fmt --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace`; quickstart commands; read-only boundary scan.

### Risks and Rollback
- Risks: simulated reconnect behavior might be mistaken for a real live WebSocket client; local API preview might be mistaken for a full web dashboard.
- Rollback: revert the US4 commit(s); US1-US3 remain usable.

### Memory Impact
- Add/update in `MEMORY.md`: health model command examples, API route helpers, and the boundary that US4 health simulation is read-only and deterministic.

### Final Notes
- What changed: Implemented US4 health/safety with serializable health snapshots, latency percentile telemetry, deterministic heartbeat/reconnect/resubscribe simulation, TUI health rendering, read-only local API response helpers, `hls doctor --live` health output, and `hls server --print-health`.
- Validation run: red/green focused tests for `hls-core --test health_state`, `hls-hyperliquid --test reconnect_heartbeat`, `hls-server --test read_only_api`, `hls-tui --test health_pane`, and `hls-cli --test health_commands`; `cargo fmt --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace`; `cargo build --workspace`; `git diff --check`; quickstart smokes under `/tmp/hlscreen-quickstart-us4.Phs2mp`; public read-only REST metadata smoke; `hls doctor --live --json`; `hls server --print-health`; read-only boundary scan.
- Tradeoffs: US4 validates deterministic reconnect behavior and pure read-only API response helpers. A long-running localhost HTTP server loop and real external WebSocket I/O remain future work.
- Rollback: revert the US4 implementation commit(s); US1-US3 stay usable because the new surfaces are additive.
- Follow-ups: Future work can add the real WebSocket connection loop and long-running localhost API process using the shared health/router contracts.

## 2026-07-08 End-to-End Audit / PR Merge Gate

### Task
- Objective: Audit the full implementation against official Hyperliquid documentation, Spec Kit contracts, repo standards, Rust best practices, runtime behavior, and security/read-only boundaries; fix any findings; create and review a PR; merge/push to `main` only if stable.
- Owner repo(s): standalone `hlscreen/` repository only.
- Capital impact: research-only / read-only market-data infrastructure. No wallet, private-key, order, exchange action, live trading, or credential changes.

### Context
- Background: All Spec Kit tasks T001-T075 are currently marked complete on `feat/andrzej_hlscreen_foundation`, but the remote has no `main` branch yet. GitHub reports `feat/andrzej_hlscreen_foundation` as the default branch.
- Inputs: Official Hyperliquid docs for `POST /info`, spot symbol naming, WebSocket subscriptions, heartbeat/pong behavior, and rate limits; Spec Kit plan/spec/contracts/tasks; current Rust code and CLI behavior.
- Outputs: Audit evidence, any bug fixes, docs/memory updates, PR, and a stable `main` branch if all gates pass.

### Assumptions
- If `origin/main` is absent, the merge flow must first establish a sane `main` baseline before opening a PR. Do not silently force-push or discard history.
- Official docs are the source of truth when fixtures or assumptions differ.
- Runtime verification should use read-only public REST and deterministic fixture flows only.

### Constraints
- Technical: do not add unrelated features during audit; prefer small targeted fixes and tests for any discovered bug.
- Operational: parent `rsibot/` is dirty and must remain untouched; all GitHub writes stay inside `s1korrrr/hlscreen`.
- Risk/capital: no exchange endpoint, signing, wallet, private/user streams, or order-capable API may be introduced.

### Options Considered
1. Treat current green tests as enough and open/merge immediately.
   - Pros: fastest.
   - Cons: does not satisfy the requested end-to-end audit or official-doc verification.
2. Perform a full requirement/doc/code/runtime review, fix findings, then PR/merge.
   - Pros: gives defensible merge evidence and catches hidden contract drift.
   - Cons: slower and may require an audit-only commit.

### Chosen Approach
- Choice: option 2.
- Why: the user explicitly asked to go beyond tests and merge only when confirmed stable.

### Execution Plan
1. Establish base/default branch state and PR constraints.
2. Compare implementation to official Hyperliquid docs and Spec Kit contracts.
3. Review code path by path: REST, WebSocket parsing/subscriptions, state/features, store/replay, DSL, TUI, health/API, CLI.
4. Run expanded validation: fmt, clippy, tests, build, runtime smokes, edge-case probes, output/log checks, security/read-only scans, dependency/license sanity.
5. Fix any findings with focused tests and rerun affected/full gates.
6. Write audit report and close local memory/lesson artifacts.
7. Create PR to `main`, review it, resolve issues, merge/push `main` if all gates pass.

### Test Plan
- Static: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo build --release --workspace`; `git diff --check`; read-only/security scans.
- Runtime: fresh temp-dir `hls init`/`doctor`; live public `hls symbols --top 20`; fixture-backed live, record, replay, screen, health/API commands; invalid input probes.
- GitHub: inspect PR diff and checks before merge.

### Risks and Rollback
- Risks: no existing remote `main`; live public REST could fail due to network; hidden fixture mismatch with current docs; audit-only files can create noise.
- Rollback: keep fixes in branch until validation passes; if merged, revert merge commit on `main` if a post-merge issue appears.

### Memory Impact
- Add/update `MEMORY.md`, `docs/agent-memory/*.jsonl`, and daily memory with durable audit findings and confirmed commands.

### Final Notes
- What changed: Completed official-doc/spec/code/runtime audit and fixed eight audit findings: explicit REST timeout, subscription budget aligned with the documented 1,000 WebSocket subscription limit, monotonic health severity, duplicate trade idempotency, timestamp-bounded feature windows, candle anomaly baselines, fail-closed doctor handling for invalid existing configs, and robust local API percent decoding/400 responses. Added regression tests and `docs/reports/2026-07-08-pre-merge-audit.md`.
- Validation run: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo build --release --workspace`; `git diff --check`; `cargo tree -d`; read-only/security scan; temp-dir init/doctor; live read-only REST `hls symbols --top 5`; live doctor JSON; fixture-backed live/record/replay/screen; invalid rule probe; `hls server --print-health`; negative live-network probe.
- Tradeoffs: `cargo-audit` and `cargo-deny` are not installed, so advisory/license gates could not be claimed. Real live WebSocket network mode and long-running HTTP service remain intentionally fail-closed/out of scope for this slice.
- PR strategy: `origin/main` was absent and GitHub default branch was `feat/andrzej_hlscreen_foundation`; establish `main` from a reviewed baseline before opening the PR, then merge only after PR diff/check review.
- Rollback: before merge, revert the audit commit on the feature branch; after merge, revert the merge commit on `main`.
- PR/Merge result: PR #1 (`https://github.com/s1korrrr/hlscreen/pull/1`) was reviewed as mergeable with no configured checks, merged to `main` at `73ebdaa`, and the GitHub default branch was changed to `main`.

## 2026-07-08 Open Source Readiness

### Task
- Objective: Make `hlscreen` ready to be public/open source later by adding professional repository docs, community files, CI/dependency hygiene, screenshots, and clear public-safety positioning.
- Owner repo(s): standalone `hlscreen/` repository only.
- Capital impact: research-only / read-only market-data infrastructure. No live trading, wallet, credential, order, or exchange-action surfaces.

### Context
- Background: `main` now contains the validated v1 implementation and audit closeout. The repo still needs the public-facing OSS package layer: license file, contribution/security/support docs, GitHub templates, CI, screenshots, and a stronger README.
- Inputs: Current README/docs, audit report, CLI smoke output, project memory, and user request to make the repo fully professional and public-ready.
- Outputs: OSS docs/community files, README refresh with screenshots, generated screenshot assets, CI/dependency automation, memory/TODO updates, validation evidence, and a PR to `main`.

### Assumptions
- Use MIT because `Cargo.toml` already declares `license = "MIT"`.
- Screenshots can be deterministic terminal-style SVG assets generated from current fixture-backed CLI output.
- The repo may stay private for now; public-readiness files should not require repo-public state.

### Constraints
- Technical: do not change runtime logic unless validation exposes a packaging blocker.
- Operational: keep generated screenshots deterministic and committed; avoid secrets, personal data, or private endpoints.
- Risk/capital: preserve explicit read-only/no-financial-advice/no-order-surface messaging.

### Options Considered
1. Only add a LICENSE and a short README note.
   - Pros: small.
   - Cons: not enough for a professional public repo.
2. Add full OSS readiness package: README, license, contributing, security, conduct, support, release, issue/PR templates, CI, dependabot, screenshots, and docs index.
   - Pros: makes the repo public-ready and reviewable.
   - Cons: larger docs/config diff.

### Chosen Approach
- Choice: option 2.
- Why: the user explicitly asked for a professional open-source package with screenshots and everything needed before making the repo public.

### Execution Plan
1. Add OSS/community files and GitHub templates/workflows.
2. Generate deterministic terminal screenshot assets from current CLI output.
3. Rewrite README around public positioning, install/build, safety, screenshots, commands, docs, roadmap, and contribution path.
4. Add or update supporting docs for release, security/privacy/threat model, examples, and open-source checklist.
5. Run formatting, tests/builds, README/screenshot link checks, and git diff checks.
6. Commit, push, open PR, review, and merge if stable.

### Test Plan
- Static: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `git diff --check`.
- Docs/assets: verify screenshot files exist, README links resolve locally, and `.github` YAML files parse structurally where feasible.
- Runtime: rerun fixture-backed commands used for screenshots.

### Risks and Rollback
- Risks: docs could overclaim live WebSocket/server readiness; screenshots can drift from CLI output; CI can fail if it assumes unavailable tooling.
- Rollback: revert the OSS-readiness commit/PR; no runtime behavior should be affected.

### Memory Impact
- Add/update `MEMORY.md`, daily memory, and agent lessons with public-readiness files and validation commands.

### Final Notes
- What changed: Added the public open-source package: MIT license, contribution/security/support/conduct/changelog docs, GitHub CI/dependabot/templates, release/privacy/threat-model/roadmap/open-source checklist docs, example screen rules, docs index, deterministic screenshot generator, committed SVG screenshots, README refresh, package metadata, and truthful Rust 1.88 MSRV/toolchain/CI configuration.
- Validation run: `python3 scripts/generate-screenshots.py`; `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo build --release --workspace --all-features`; `git diff --check`; local required-file check; local Markdown link check; YAML parse check for `.github/**/*.yml`.
- Tradeoffs: Screenshots are deterministic SVG terminal captures rather than bitmap desktop screenshots. This keeps them reproducible and GitHub-renderable without adding image tooling.
- Follow-ups: After merge, confirm GitHub Actions status on `main` once Actions run for the now-public-ready repo.
- PR/Merge result: PR #3 (`https://github.com/s1korrrr/hlscreen/pull/3`) passed GitHub Actions and merged to `main` at `1f93af8`.

## 2026-07-08 Live Smoke / TUI Screenshot Slice

### Task
- Objective: Add a deterministic full-pipeline smoke test, improve the terminal/TUI table enough for professional screenshots, regenerate screenshot assets, and run a bounded 15-minute read-only live public-market-data pipeline across the selected Hyperliquid spot universe.
- Owner repo(s): standalone `hlscreen/` repository only.
- Capital impact: research-only / read-only public market-data ingestion. No wallet, private key, private/user stream, order, exchange action, or execution-capable route.

### Context
- Background: The merged v1 is fixture-backed for live behavior and explicitly fails closed for real external WebSocket mode. The user now wants a smoke test, better TUI evidence, screenshots, and a 15-minute full-pipeline run for all pairs.
- Inputs: Official Hyperliquid WebSocket, subscription, rate-limit, heartbeat, and spot metadata docs; Spec Kit CLI/data-file contracts; current CLI/TUI/store implementation.
- Outputs: Live public WebSocket capture path, tests, smoke helper or integration coverage, refreshed screenshots, validation evidence, and a 15-minute run artifact if the public network path is stable.

### Assumptions
- "All pairs" means the public spot universe returned by `spotMetaAndAssetCtxs` after local selection and budget validation, not private/user account feeds.
- If all pairs multiplied by requested public streams exceeds the official 1,000-subscription cap, the command must fail closed with a clear message or reduce scope only when explicitly requested by flags.
- A bounded 15-minute run should use a temp/local data directory and produce raw plus normalized replayable files.

### Constraints
- Technical: keep ingestion async and non-blocking relative to recording/rendering; preserve raw frames before normalization; honor Hyperliquid ping/pong heartbeat behavior; keep subscription count below configured headroom.
- Operational: keep parent `rsibot/` untouched; do not mutate Dependabot PR branches; commit only after validation and live evidence.
- Risk/capital: no Exchange endpoint, signing, wallet addresses, user-specific streams, or trading recommendations.

### Options Considered
1. Add only fixture smoke and screenshots.
   - Pros: deterministic, small diff, no network dependency.
   - Cons: does not satisfy the requested 15-minute all-pairs live pipeline.
2. Add a minimal public WebSocket live client over existing parser/state/store paths plus fixture smoke and screenshots.
   - Pros: proves the actual read-only pipeline while reusing existing tested boundaries.
   - Cons: requires live-network validation and careful subscription/heartbeat handling.

### Chosen Approach
- Choice: option 2.
- Why: the repo is already designed for public WebSocket ingestion; adding the narrow live read loop is the smallest honest path to the requested 15-minute full-pipeline proof.

### Execution Plan
1. Inspect current live, parser, subscription, store, TUI, screenshot, and CLI tests.
2. Add or update tests for subscription message construction, control-message handling, deterministic full-pipeline smoke, and TUI output.
3. Implement a bounded public WebSocket read loop with heartbeat, public subscription sends, raw frame preservation, normalized event handling, and clean duration-based shutdown.
4. Add `--all-symbols` and duration/refresh controls if missing, with budget validation before connecting.
5. Polish terminal rendering and regenerate committed screenshot assets.
6. Run fmt, clippy, workspace tests/builds, smoke commands, link/screenshot checks, and read-only scans.
7. Run the 900-second all-pairs pipeline, inspect raw/normalized/metadata outputs, replay/screen the run, and record final evidence.
8. Review, commit, push, open PR, review/merge only if stable.

### Test Plan
- Unit/integration: WebSocket subscription/control-message tests; CLI full-pipeline smoke test; TUI golden/screenshot command validation.
- Runtime: fixture-backed live/record/replay/screen/health; public REST symbol selection; live WebSocket dry run; 900-second all-pairs run with raw and normalized outputs.
- Regression/audit: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `git diff --check`; read-only/no-secrets scan; screenshot/link check.

### Risks and Rollback
- Risks: public WebSocket disconnects during the run; spot universe size can approach subscription headroom; payload drift can expose parser gaps; GitHub Actions can fail after merge if live-only behavior is not exercised in CI.
- Rollback: revert this branch before merge; after merge, revert the merge commit. Runtime artifacts stay in temp/local data dirs and are not required for rollback.

### Memory Impact
- Add/update in `MEMORY.md`: confirmed live command, all-pairs subscription behavior, smoke command, screenshot generation command, and any live-run caveats.

### Final Notes
- What changed: Added bounded public WebSocket live mode with duration shutdown, heartbeat ping handling, public REST universe selection, `--all-symbols`, subscription budget adaptation, streaming raw/normalized recording, tolerant live payload parsing, a full-pipeline fixture smoke test, polished terminal table output, refreshed screenshots, and `docs/reports/2026-07-08-live-smoke.md`.
- Validation run: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo build --release --workspace --all-features`; `git diff --check`; `python3 scripts/generate-screenshots.py`; screenshot asset existence check; read-only/no-secret diff scan.
- Live run: `allpairs-15m-20260708-084527` ran for 900 seconds with 308 symbols, 924 public subscriptions, three public streams per symbol, 298,082 raw WebSocket messages, 306,140 normalized events, 13 raw files, one normalized file, and `clean_shutdown=true`. File counts, SQLite registry, replay, screen, and health preview checks passed.
- Rollback: revert this branch before merge, or revert the merge commit after merge. Runtime data lives under `/tmp/hlscreen-allpairs-15m-20260708-084527` and is not required for source rollback.
- PR/Merge result: PR #9 (`https://github.com/s1korrrr/hlscreen/pull/9`) passed the GitHub `Rust workspace` check and merged to `main` at `456911d`.

## 2026-07-08 Live Production Hardening

### Task
- Objective: Remove the remaining production blockers in the public live-data path: reconnect/resubscribe on server disconnect, receive-timestamp propagation, non-blocking recording, operator-visible live TUI refresh, live-first docs/report updates, full validation, PR review, and merge to `main` only if stable.
- Owner repo(s): standalone `hlscreen/` repository only.
- Capital impact: research-only / read-only public market-data ingestion. No wallet, private key, private/user stream, order, exchange action, execution route, or trade recommendation.

### Context
- Background: The previous live smoke proved a 15-minute all-pairs bounded capture, but `MEMORY.md` and the live report still record automatic reconnect/resubscribe as missing. Hyperliquid’s current WebSocket docs state automated users should handle server-side disconnects and reconnect gracefully. The current code also writes raw/normalized output synchronously inside the WebSocket read loop and leaves normalized event `recv_ts_ns` at zero.
- Inputs: Official Hyperliquid WebSocket, subscription, heartbeat, rate-limit, and spot metadata docs; current `crates/hls-cli/src/commands/live.rs`; store metadata/data-gap support; Spec Kit US1/US2/US4 contracts.
- Outputs: Hardened live network path, targeted tests, docs/report/memory updates, screenshot evidence where practical, fresh live smoke, PR, and merge if all gates pass.

### Assumptions
- "No mocks/workarounds" means production commands and docs must use real public REST/WebSocket data by default. Hidden fixture flags can remain for deterministic tests/CI only and must not be presented as production evidence.
- "All pairs" means the public spot universe returned by `spotMetaAndAssetCtxs` while staying under Hyperliquid’s documented per-IP WebSocket subscription cap.
- Four streams per symbol across today’s full public spot universe exceeds the 1,000-subscription cap, so all-symbol live mode must use the explicit real-time stream set (`trades`, `bbo`, `activeAssetCtx`) rather than silently violating the exchange limit.

### Constraints
- Technical: do not block the WebSocket read loop on disk writes or rendering; fail closed on writer backpressure rather than dropping frames silently; timestamp received raw frames and normalized events; persist reconnect data gaps when recording is enabled.
- Operational: keep runtime artifacts under `/tmp` or `.hls`; do not commit captured raw market data; do not touch parent `rsibot` work.
- Risk/capital: no signed endpoints, user-specific subscriptions, wallet/account identifiers, trading routes, credential handling, or profitability language.

### Options Considered
1. Document the missing reconnect behavior as an external caveat.
   - Pros: smallest diff.
   - Cons: conflicts with official docs and the user’s production-readiness requirement.
2. Add reconnect/resubscribe, receive timestamps, and a bounded writer thread while keeping the current CLI surface stable.
   - Pros: removes the real production blockers with a focused diff and preserves existing tested parser/state/store boundaries.
   - Cons: still leaves full Parquet output, long-running HTTP server, and keyboard-driven TUI editing for later slices.

### Chosen Approach
- Choice: option 2.
- Why: it aligns the runtime with official WebSocket guidance and the project’s own research note that ingestion must not be blocked by disk/TUI work.

### Execution Plan
1. Add regression coverage for receive timestamp stamping and reconnect/data-gap behavior.
2. Refactor live recording into a bounded worker thread with fail-closed backpressure.
3. Refactor the live WebSocket loop to reconnect/resubscribe until the requested duration elapses, with heartbeat pings and gap persistence.
4. Add operator-visible live TUI table refresh for real terminal sessions and a `--tui` override for captured smoke evidence.
5. Update README/docs/reports to lead with live public data and move fixtures into deterministic-test language.
6. Run focused tests, full Rust gates, read-only/security scans, and a fresh real live smoke including TUI capture.
7. Review the diff, push, open PR, inspect checks/diff, and merge only if stable.

### Test Plan
- Unit/integration: `cargo test -p hls-core --test market_state`; `cargo test -p hls-cli --test live_mock`; `cargo test -p hls-cli --test full_pipeline_smoke`; reconnect/metadata tests as added.
- Static: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo build --release --workspace --all-features`; `git diff --check`.
- Runtime: live public REST symbol probe; bounded public WebSocket run with `--tui`, raw and normalized recording; replay/screen over the run; SQLite run/gap/file inspection; log review for warnings/errors.

### Risks and Rollback
- Risks: public network instability can still interrupt a bounded run; reconnect may duplicate snapshots/trades, so state idempotency remains required; writer backpressure now fails closed instead of hiding data loss; docs can overclaim beyond the implemented bounded CLI.
- Rollback: revert this branch before merge; after merge, revert the merge commit. Live runtime artifacts remain local and are not needed for source rollback.

### Memory Impact
- Add/update in `MEMORY.md`: reconnect/resubscribe status, live recording worker behavior, confirmed live smoke command, and any remaining explicit limitations.

### Final Notes
- What changed: Added real live WebSocket reconnect/resubscribe until the run deadline, fail-closed no-message endpoint handling, bounded writer-thread recording for raw/normalized live data, non-zero live receive timestamps on normalized events, reconnect data-gap persistence, asset-context freshness updates, future timestamp age clamping, `--tui` live table refresh, live-first docs, regenerated screenshots, and `docs/reports/2026-07-08-live-production-hardening.md`.
- Validation run: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo build --release --workspace --all-features`; `git diff --check`; `python3 scripts/generate-screenshots.py`; JSONL validation; Markdown local link check; read-only/no-secret scan; negative endpoint probe with `ws://127.0.0.1:1`; 25-second single-symbol public WebSocket smoke; 900-second all-pairs public WebSocket run.
- Live run: `allpairs-15m-hardening-20260708-093507` ran for 900 seconds with 308 symbols, 924 public subscriptions, 296,492 raw WebSocket messages, 304,405 normalized events, 13 raw files, one normalized file, `clean_shutdown=true`, `reconnects=0`, `data_gaps=0`, all 304,405 normalized events carrying non-zero `recv_ts_ns`, and 308 fresh replay/screen rows.
- Tradeoffs: All-symbol mode intentionally uses three public streams per symbol (`trades`, `bbo`, `activeAssetCtx`) because four streams over 308 symbols would exceed Hyperliquid's documented 1,000-subscription per-IP cap. Automatic public REST backfill after reconnect is still not implemented; reconnect windows are explicitly recorded as data gaps. True Parquet and long-running HTTP serving remain future work.
- PR/Merge result: pending GitHub PR creation and merge after final diff review.

## 2026-07-08 Next-Gen TUI Polish

### Task
- Objective: Upgrade the terminal renderer and committed screenshots so `hlscreen` looks like a professional, modern read-only market-data TUI while preserving deterministic CLI output and the live-data safety boundary.
- Owner repo(s): standalone `hlscreen/` repository only.
- Capital impact: research-only / read-only public market-data display. No wallet, private stream, order, exchange action, execution route, or recommendation semantics.

### Context
- Background: The current TUI is functionally correct but visually plain. The user asked to make it "good", "next gen", and provide screenshots.
- Inputs: Current `hls-tui` renderer, CLI smoke tests, screenshot generator, README screenshot section, and the project read-only/open-source boundary.
- Outputs: Polished table renderer, polished health pane, updated golden tests, regenerated screenshot SVGs, README screenshot copy if needed, and validation evidence.

### Assumptions
- The production CLI should remain dependency-light and deterministic; this slice improves the stable renderer rather than adding a full interactive runtime.
- Unicode terminal glyphs are acceptable here because the explicit goal is visual TUI polish; tests will keep rendering stable.
- Screenshots remain deterministic fixture/offline captures and must not be presented as live exchange evidence.

### Constraints
- Technical: no blocking live ingestion changes, no new trading/private data surfaces, no ANSI escape output that breaks scripts, and no hidden mock path for production commands.
- Operational: keep runtime artifacts out of git; do not touch parent `rsibot/` work.
- Risk/capital: retain read-only labels and screening-heuristic language.

### Options Considered
1. Add a full `ratatui`/`crossterm` interactive app now.
   - Pros: richer long-term interaction model.
   - Cons: larger architecture slice, more input/lifecycle work, and not necessary for screenshot-grade visual polish.
2. Upgrade the existing deterministic renderer and screenshot generator.
   - Pros: small, testable, preserves CLI semantics, improves screenshots immediately.
   - Cons: keyboard-driven interactive TUI remains future work.

### Chosen Approach
- Choice: option 2.
- Why: the current code already has stable rendering tests and live refresh wiring; improving that surface is the fastest safe path to a professional v1 visual layer.

### Execution Plan
1. Update golden tests for the desired TUI layout and health pane.
2. Implement a modern renderer with header, KPI strip, state markers, aligned columns, and explicit read-only footer.
3. Improve screenshot SVG styling so generated assets look publication-ready while still using real command output.
4. Regenerate screenshots and update README copy if needed.
5. Run focused TUI/CLI tests, full Rust gates, screenshot generation, and diff checks.

### Test Plan
- Focused: `cargo test -p hls-tui --test main_table_golden`; `cargo test -p hls-tui --test health_pane`; relevant CLI smoke tests.
- Regression: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `git diff --check`; `python3 scripts/generate-screenshots.py`.

### Risks and Rollback
- Risks: Unicode glyphs may render poorly in rare terminals; screenshot generator can drift from terminal output if over-styled; wide symbols could affect alignment.
- Rollback: revert this branch; no runtime data or schema migration is involved.

### Memory Impact
- Add/update in `MEMORY.md`: confirmed TUI polish command and any durable renderer/screenshot convention.

### Final Notes
- What changed: Replaced the plain table with a polished deterministic market board using a branded header, read-only/data KPI strip, state markers, explicit units, TOB imbalance, signed return formatting, readable age formatting, and a read-only heuristic footer. Added a shared TUI panel helper, upgraded the health pane, surfaced the health pane through `hls doctor --live` text output, refreshed README screenshot copy, added `health-panel.svg`, and improved SVG screenshot styling with whitespace preservation and temp-path redaction.
- Validation run: red/green focused TUI and CLI tests; `cargo test -p hls-tui --test main_table_golden --test health_pane`; `cargo test -p hls-cli --test live_mock --test health_commands --test record_replay --test screen_command --test full_pipeline_smoke`; `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo build --release --workspace --all-features`; `git diff --check`; `python3 scripts/generate-screenshots.py`; screenshot temp-path scan; PNG preview render with `rsvg-convert`.
- Live smoke: `./target/debug/hls live --symbols @107 --duration-secs 10 --refresh-secs 5 --tui` completed against the public Hyperliquid WebSocket with 1 symbol, 4 subscriptions, 102 WebSocket messages, 133 market events, 0 reconnects, 0 data gaps, and fresh TUI rows.
- Tradeoffs: This is a deterministic renderer polish slice, not a full keyboard-driven interactive TUI runtime. It intentionally avoids ANSI color in normal table strings so captured stdout remains stable; live `--tui` still uses terminal clear codes during refresh.
- Rollback: revert this branch; no schema, data, or runtime migration is involved.

## 2026-07-08 Workstation TUI Refinement

### Task
- Objective: Push the terminal UI from a clean table into a professional workstation-style market board with stronger hierarchy, richer truthful KPIs, clearer health states, and refreshed screenshots.
- Owner repo(s): standalone `hlscreen/` repository only.
- Capital impact: research-only / read-only public market-data display. No wallet, private stream, order, exchange action, execution route, or recommendation semantics.

### Context
- Background: PR #12 already merged a solid deterministic TUI polish pass. The current ask is to make the TUI look "very very good" and provide screenshots, so this is a visual/ergonomic refinement over the existing renderer rather than a new live-data feature.
- Inputs: `crates/hls-tui/src/app.rs`, `crates/hls-tui/src/health.rs`, `crates/hls-tui/src/theme.rs`, TUI golden tests, screenshot generator, README screenshot section, and current official Ratatui guidance that rich terminal UIs are normally widget/layout-driven.
- Outputs: Updated deterministic market board, health panel, screenshot styling/assets, tests, and visual proof.

### Assumptions
- The project still needs stable stdout/stderr output for smoke tests, CI screenshots, logs, and replay; a full alternate-screen keyboard UI is a future feature rather than this refinement.
- Visual KPIs must be computed from existing `FeatureSnapshot`/`HealthSnapshot` values only. No fake market data and no made-up confidence score.
- Unicode box/glyph rendering remains acceptable for the polished docs surface.

### Constraints
- Technical: keep renderer deterministic; avoid ANSI codes in captured table strings; do not block live ingestion; avoid new dependencies unless they remove real complexity.
- Operational: regenerate screenshots from the actual rebuilt binary; do not commit runtime market-data captures.
- Risk/capital: preserve read-only labels and heuristic-only score language.

### Options Considered
1. Add `ratatui`/`crossterm` and build a full alternate-screen app immediately.
   - Pros: aligns with the dominant Rust TUI ecosystem and enables richer widgets later.
   - Cons: larger lifecycle/input slice, harder screenshot automation, and unnecessary for the requested immediate visual proof.
2. Refine the existing deterministic renderer into a workstation board and keep full interactivity on the roadmap.
   - Pros: small, verifiable, screenshot-friendly, no live-data risk, and consistent with current architecture.
   - Cons: still not a keyboard-driven terminal application.

### Chosen Approach
- Choice: option 2.
- Why: it gives the user visible quality now while preserving the low-latency live-data path and leaving the right architecture path open for a later interactive Ratatui shell.

### Execution Plan
1. Update TUI golden expectations for a richer market-board layout and health command center.
2. Implement computed KPI rows, stronger table hierarchy, clearer state labels, and readable empty-state handling.
3. Improve screenshot generator width/styling if the new layout needs it.
4. Regenerate SVG screenshots and render PNG previews for inspection.
5. Run focused TUI/CLI tests, full Rust validation, screenshot generation, diff check, and read-only scan.

### Test Plan
- Focused: `cargo test -p hls-tui --test main_table_golden`; `cargo test -p hls-tui --test health_pane`.
- CLI/screenshot: `cargo test -p hls-cli --test live_mock --test health_commands`; `python3 scripts/generate-screenshots.py`; PNG preview render with `rsvg-convert`.
- Regression: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `git diff --check`.

### Risks and Rollback
- Risks: wider tables can wrap in documentation screenshots; over-styled screenshots can diverge from actual terminal output; Unicode glyphs vary by font.
- Rollback: revert this refinement commit; no data/schema/runtime migration is involved.

### Memory Impact
- Add/update in `MEMORY.md`: durable TUI renderer convention and screenshot proof command if behavior changes.

### Final Notes
- What changed: Refined the deterministic TUI into a workstation-style market board with a wider panel, branded microstructure header, mode/universe/quality KPI strip, computed median spread/top-depth/total-depth/top-score metrics, `MARKET BOARD` separator, indexed rows, uppercase state markers, RV/liquidity/momentum columns, empty-state copy, and an explicit read-only safety footer. Updated the operations health panel into safety/ingest/storage lanes, adjusted CLI tests to assert the safety footer directly, widened/styled screenshot generation, and refreshed all committed SVG screenshots plus README live-board copy.
- Validation run: red/green `cargo test -p hls-tui --test main_table_golden --test health_pane`; `cargo test -p hls-cli --test live_mock --test health_commands`; `python3 scripts/generate-screenshots.py`; SVG-to-PNG preview render with `rsvg-convert`; `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `git diff --check`; screenshot temp-path scan; read-only surface scan.
- Live smoke: `./target/debug/hls live --symbols @107 --duration-secs 10 --refresh-secs 5 --tui` completed against the public Hyperliquid WebSocket with 1 symbol, 4 public subscriptions, 56 WebSocket messages, 81 market events, 0 reconnects, and 0 data gaps.
- Tradeoffs: This remains a deterministic renderer and live-refresh surface, not a full keyboard-driven alternate-screen Ratatui app. That larger interactive shell remains a future slice.
- Rollback: revert this refinement commit; no schema, storage, or runtime migration is involved.

## 2026-07-08 Microstructure Foundation Contracts

### Task
- Objective: Implement Spec Kit v2 foundation tasks T001-T019 for the Hyperliquid Microstructure Workstation: fixture/doc scaffolding, confidence contracts, score breakdown contracts, metrics label validation, benchmark manifest model, exports, safety regression tests, and terminology docs.
- Owner repo(s): standalone `hlscreen/` repository only.
- Capital impact: research-only / read-only public market-data contracts. No wallet, private stream, signing, order placement, exchange action, execution route, or profitability claim.

### Context
- Background: PR #14 merged `specs/002-microstructure-workstation` with 92 tasks derived from the pasted product brief. All v2 implementation tasks are still unchecked; Phase 2 blocks every user story, so the next correct slice is the shared foundation.
- Inputs: `specs/002-microstructure-workstation/tasks.md` T001-T019, `contracts/confidence-and-scoring.md`, `contracts/metrics.md`, `data-model.md`, and existing v1 health/telemetry/store patterns.
- Outputs: New contract modules under `hls-core`, a benchmark manifest module under `hls-store`, fixture/doc scaffolding, tests, checked task markers for T001-T019, validation evidence, and a PR/merge if stable.

### Assumptions
- The first confidence contract should be data-quality state and reason aggregation, not full feature-engine confidence computation; US1 owns attaching and computing it from live/replay state.
- The first score contract should define named components and confidence-adjusted totals, not change current ranking formulas yet; US3 owns full why-ranked UI/CLI.
- Metrics labels must reject high-cardinality labels such as symbol/run id at the contract boundary.
- Benchmark manifests can be JSON because fixture metadata is already serde-based and should be readable from committed test fixtures.

### Constraints
- Technical: keep contracts small, serializable, and dependency-light; use existing `serde`/`HlsError` patterns; no broad feature-engine rewrites in this slice.
- Operational: keep runtime market-data captures out of git; do not touch parent `rsibot/`.
- Risk/capital: read-only language must remain visible; no trading/execution objects or advice semantics.

### Options Considered
1. Only create placeholder files/directories.
   - Pros: completes setup tasks quickly.
   - Cons: leaves the blocking foundation unimplemented and does not move toward the user's "solve tasks one by one" request.
2. Implement T001-T019 as a single foundation PR.
   - Pros: creates real shared contracts and tests while keeping scope below US1 runtime behavior.
   - Cons: touches several crates and docs, requiring full validation.

### Chosen Approach
- Choice: option 2.
- Why: every user story depends on the foundation contracts, and they are small enough to validate coherently in one PR.

### Execution Plan
1. Add setup directories and documentation/README stubs for microstructure fixtures and golden outputs.
2. Add failing foundation tests for confidence state, score breakdowns, metrics contracts, benchmark manifest parsing, and CLI safety regression.
3. Implement `hls-core::{confidence, score, metrics}` and wire exports.
4. Implement `hls-store::benchmark` and wire exports.
5. Update feature definitions and `docs/microstructure.md` with confidence/score terminology.
6. Mark T001-T019 complete in `specs/002-microstructure-workstation/tasks.md`.
7. Run focused tests, full Rust gates, diff/read-only scans, update memory/reflection, commit, push, PR, and merge only if stable.

### Test Plan
- Focused: `cargo test -p hls-core --test confidence_state --test score_breakdown --test metrics_contract`; `cargo test -p hls-store --test benchmark_manifest`; `cargo test -p hls-cli --test microstructure_safety`.
- Regression: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `git diff --check`.
- Safety: read-only scan for order/exchange/private-key surfaces and review of docs for advice/profitability language.

### Risks and Rollback
- Risks: score/confidence names may need adjustment when US1/US3 implementation starts; benchmark manifest shape may need migration once real benchmark packs exist; tests can overfit early field names.
- Rollback: revert the foundation PR; no persisted data migration is introduced.

### Memory Impact
- Add/update in `MEMORY.md`: foundation contract scope, confirmed test commands, and contract boundaries.

### Final Notes
- What changed: Completed T001-T019 from `specs/002-microstructure-workstation/tasks.md`. Added tracked microstructure fixture/golden directories, `docs/microstructure.md`, foundation tests, `hls-core::{confidence, score, metrics}`, `hls-store::benchmark`, `hls-features::microstructure` contract reexports, a CLI command-registration boundary comment, and confidence/score/metrics terminology in `docs/feature-definitions.md`.
- Validation run: red/green focused tests for `cargo test -p hls-core --test confidence_state --test score_breakdown --test metrics_contract`; `cargo test -p hls-store --test benchmark_manifest`; `cargo test -p hls-cli --test microstructure_safety`; `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `git diff --check`; task-ID format check; read-only surface scan.
- Tradeoffs: Foundation contracts define serializable shapes and validation only. Confidence computation, replay parity flags, why-ranked rendering, resilience metrics, metadata enrichment, metrics output, extension execution, and packaging remain unchecked later tasks.
- Rollback: revert this foundation commit/PR; no runtime schema migration or recorded data migration is introduced.
