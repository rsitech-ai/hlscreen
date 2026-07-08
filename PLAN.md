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
