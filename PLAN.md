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
