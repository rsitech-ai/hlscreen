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

## 2026-07-08 US5 OSS Operations Slice

### Task
- Objective: Complete the Spec Kit US5 operations layer: deterministic benchmark packs, metrics snapshots, read-only extension contract models, release packaging drafts, and supporting docs/tests.
- Owner repo(s): standalone `hlscreen/` repository only.
- Capital impact: research-only / read-only public market-data tooling. No wallet, private stream, order, execution, signing, leverage, or live-capital action.

### Context
- Background: US1-US4 and the metadata/TUI polish slices are merged to `main`. The remaining feature work is US5, which makes the project more professional for OSS operation without changing live trading boundaries.
- Inputs: `specs/002-microstructure-workstation/tasks.md` T069-T082, current `hls-core::metrics`, `hls-store::benchmark`, CLI command layout, release docs, and official Prometheus/OpenTelemetry/Extism/cargo-dist references.
- Outputs: Passing tests for benchmark command, metrics output, extension contract validation, and release packaging checks; new `hls bench`; JSON/Prometheus-style metrics in `doctor --live --json`; release/extension docs and packaging drafts.

### Assumptions
- `cargo-dist` release publishing should remain tag-gated and draft-only in this slice; no release tag will be created.
- The extension work is a contract/model only. It does not load or execute arbitrary WASM.
- Benchmark packs should use committed public fixtures and fail on expected-hash drift.

### Constraints
- Technical: keep benchmark runs deterministic; avoid high-cardinality metric labels; keep CLI data on stdout and diagnostics/errors on stderr; use existing parser/state/feature paths.
- Operational: do not mutate parent `rsibot`; do not publish packages, tags, GitHub releases, or Homebrew taps.
- Risk/capital: no private/account data, no wallet/config secrets, no plugin network/filesystem access, no order-capable APIs.

### Options Considered
1. Add docs-only release and extension notes.
   - Pros: small diff.
   - Cons: does not satisfy US5 validation or provide reproducible operator checks.
2. Implement tested contracts and local dry-run helpers without enabling external publication.
   - Pros: gives contributors real commands/tests while keeping release actions explicit and tag-gated.
   - Cons: bigger slice; requires new stable hashes and release workflow checks.

### Chosen Approach
- Choice: option 2.
- Why: US5 is about operational trust. A docs-only pass would keep hidden drift risk in benchmark fixtures, metrics naming, extension permissions, and release packaging.

### Execution Plan
1. Add failing tests for `hls bench`, `doctor --live --json` metrics, extension contract validation, and release packaging checks.
2. Implement benchmark runner over public NDJSON fixtures using the existing WebSocket parser, live state, and feature engine.
3. Add `hls bench` and register it in the CLI.
4. Extend `hls-core::metrics` with snapshot samples and Prometheus text output, then include metrics in live doctor JSON.
5. Add read-only extension manifest/invocation models with strict permission validation.
6. Add `dist-workspace.toml`, tag-gated release workflow draft, `docs/RELEASING.md`, and `docs/extensions.md`.
7. Update Spec Kit tasks and continuity docs; run focused tests plus full validation.

### Test Plan
- Unit/contract: `cargo test -p hls-core --test extension_contract --test metrics_contract`; `cargo test -p hls-store --test benchmark_manifest`.
- CLI/integration: `cargo test -p hls-cli --test bench_command --test metrics_output`; `scripts/check-release-packaging.sh`.
- Regression/audit: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `git diff --check`; read-only/no-private scan.

### Risks and Rollback
- Risks: benchmark hashes may be brittle if serialized snapshot contracts intentionally change; cargo-dist syntax can drift; release workflow is a draft until a tag-run is proven on GitHub.
- Rollback: revert the US5 commit(s); no external release, token, or package state is modified by this slice.

### Memory Impact
- Add/update in `MEMORY.md`: benchmark command, metrics output boundary, extension no-runtime/no-permissions contract, release dry-run command.

### Final Notes
- What changed: Completed US5 operations and Phase 8 polish. Added deterministic benchmark runner/`hls bench`, benchmark fixture hash gate, low-cardinality metrics snapshots with Prometheus text in `doctor --live --json`, read-only extension manifest models, draft cargo-dist config/workflow, release packaging check harness, extension/release docs, architecture/data/threat-model updates, and dated implementation report.
- Validation run: `cargo test -p hls-core --test extension_contract --test metrics_contract`; `cargo test -p hls-store --test benchmark_manifest`; `cargo test -p hls-cli --test bench_command --test metrics_output`; `scripts/check-release-packaging.sh`; `/tmp/hlscreen-dist/bin/dist plan`; `cargo metadata --no-deps --format-version 1`; `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo build --release --workspace --all-features`; `git diff --check`; read-only scan; `./target/debug/hls bench --manifest tests/fixtures/microstructure/benchmark_gap_replay.json --repo-root . --json`; `./target/debug/hls doctor --live --json --simulate-health writer-lag --data-dir /tmp/hlscreen-us5-doctor-smoke`.
- Tradeoffs: Release publication is still unproven until the first reviewed `v*` tag workflow succeeds; no release tag was created in this slice. Cargo-dist CI was regenerated with pinned 0.32.0 instead of maintaining a divergent hand-written release workflow.
- Rollback: revert the US5 commit(s); no external release, tag, package, token, plugin runtime, or live-capital state was modified.

## 2026-07-08 Next-Gen Keyboard-Interactive TUI

### Task
- Objective: Upgrade the live terminal workstation into a keyboard-interactive, adaptive operator interface while preserving the existing deterministic screenshot path and read-only public-data boundary.
- Owner repo(s): standalone `hlscreen/` repository only.
- Capital impact: research-only / read-only public Hyperliquid spot market-data UI. No wallet, private stream, order, execution, leverage, or capital-touching control.

### Context
- Background: The compact workstation renderer now matches the requested mock shape, but it is still a static table refreshed by the live loop. The operator wants a more advanced hedge-fund-style TUI with keyboard interaction, dynamic focus, richer surface hierarchy, and current live data for all visible pairs.
- Inputs: Current `hls-tui` deterministic renderer, `hls live` public WebSocket loop, existing `FeatureSnapshot` fields, screenshot generator, Ratatui app-architecture guidance, and direct Crossterm event handling.
- Outputs: Tested interaction state, richer workstation frame, keyboard controls for row focus/view mode/density/pause/help, CLI wiring for live progress rendering, regenerated screenshots, and docs that state what is implemented truthfully.

### Assumptions
- The stable documentation screenshots should remain ANSI-free SVG captures generated from deterministic CLI output.
- True terminal keyboard handling can use Crossterm-style polling while the existing public WebSocket loop remains the live data source.
- `--tui` should continue to work for automated smoke captures; interactive controls must not make non-TTY runs hang.

### Constraints
- Technical: separate pure TUI state/actions from terminal I/O; do not block the WebSocket read loop; do not introduce fake data fields; keep CLI stdout machine-readable and render live progress on stderr.
- Operational: do not publish releases or mutate parent `rsibot`; do not start long-running services beyond bounded live smokes.
- Risk/capital: UI controls may change display focus/filter/view only; they must not create trading/execution semantics or private-data integrations.

### Options Considered
1. Replace the renderer with a full Ratatui alternate-screen app.
   - Pros: strongest native terminal capabilities.
   - Cons: high blast radius, harder deterministic screenshots, and more risk to the existing live smoke/CI path.
2. Add an interaction state machine and adaptive workstation frame around the current renderer, then poll keys opportunistically during live progress.
   - Pros: preserves deterministic screenshots/tests, wires into the real live data loop, and gives immediate keyboard utility without destabilizing ingestion.
   - Cons: not yet a full widget-grid Ratatui app with mouse support or async event channel.

### Chosen Approach
- Choice: option 2.
- Why: the project already values deterministic output and live-data truth. A state-machine-first TUI adds real interaction while keeping low-latency ingestion and screenshot regression stable.

### Execution Plan
1. Add `hls-tui` interaction model with tabs/view modes, density, focused row, pause/help flags, keyboard action mapping, and deterministic rendering helpers.
2. Extend the workstation renderer so selected row and UI mode can be supplied externally, while default rendering remains compatible.
3. Wire `hls live --tui` progress rendering to maintain interactive state and poll keyboard input only for real terminals.
4. Add focused tests for keyboard actions, selected-row rendering, help overlay, and fixture/CLI smoke behavior.
5. Regenerate screenshots and update README/docs with the implemented controls and current truth boundary.
6. Run fmt, clippy, workspace tests, screenshot generation, diff check, and a bounded public live smoke.

### Test Plan
- Unit/golden: `cargo test -p hls-tui --test main_table_golden --test interactive_tui`.
- CLI/integration: `cargo test -p hls-cli --test live_mock`.
- Regression/audit: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `python3 scripts/generate-screenshots.py`; `git diff --check`.
- Smoke: bounded public `hls live --symbols hype-usdc --duration-secs 10 --refresh-secs 5 --tui` after implementation.

### Risks and Rollback
- Risks: terminal key polling can behave differently in CI/non-TTY shells; richer rendering can break line wrapping; adding controls can imply unsupported trading behavior if copy is sloppy.
- Rollback: revert this branch; the previous deterministic compact workstation remains intact on `main`.

### Memory Impact
- Add/update in `MEMORY.md`: implemented keyboard controls, deterministic screenshot command, and any terminal-interaction caveats.

### Final Notes
- What changed: Added `hls-tui::interaction` with tested row focus, view tabs, density, help, pause, and quit state; extended the deterministic workstation renderer with a command rail, selected-row marker, view-specific detail panes, and external UI state; wired `hls live --tui` to direct Crossterm raw-mode key polling only for real TTY sessions; attached public metadata to live progress frames so `HYPE/USDC` display names are consistent during refresh and final output; regenerated deterministic SVG screenshots; updated README and architecture docs.
- Validation run: `cargo test -p hls-tui --test interactive_tui --test main_table_golden`; `cargo test -p hls-cli --test live_mock`; `python3 scripts/generate-screenshots.py`; `rsvg-convert docs/assets/screenshots/live-screen.svg -o /tmp/hlscreen-nextgen-tui-preview/live-screen.png`; `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo build --release --workspace --all-features`; `scripts/check-release-packaging.sh`; `git diff --check`; public smoke `./target/debug/hls live --symbols hype-usdc --duration-secs 10 --refresh-secs 5 --tui` completed with 61 WebSocket messages, 105 market events, 0 reconnects, 0 data gaps, and display-name-correct TUI progress/final output.
- Follow-ups: A full alternate-screen Ratatui widget-grid with mouse support, async input channel, and persistent multi-pane layout remains a future enhancement. Current implementation is keyboard-interactive for real TTY `hls live --tui` while preserving deterministic stdout/screenshots.

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

## 2026-07-08 Next-Gen Workstation TUI Polish

### Task
- Objective: Upgrade the deterministic terminal UI into a more polished, next-generation workstation surface and refresh committed screenshots from the real binary.
- Owner repo(s): standalone `hlscreen/` repository only.
- Capital impact: research-only / read-only public market-data display. No wallet, private stream, signing, order placement, exchange action, or advice semantics.

### Context
- Background: The current TUI is already clean, but still reads as a flat table. The active product brief calls for an operator-grade terminal cockpit with persistent panes, clear data-quality framing, fast scanability, and screenshot-ready OSS presentation.
- Inputs: `crates/hls-tui/src/app.rs`, `crates/hls-tui/src/health.rs`, `crates/hls-tui/src/theme.rs`, TUI golden tests, screenshot generator, README screenshots, Ratatui official concepts for layout/widgets, and `specs/002-microstructure-workstation/tasks.md` T025/T031/T086.
- Outputs: Stronger market-board hierarchy, richer truthful summary/detail sections, clearer health panel, updated tests, regenerated screenshots, docs/status notes, and validation evidence.

### Assumptions
- This is a visual and information-architecture slice, not the full US1 confidence/replay parity implementation. It must not mark T020-T033 complete unless the real confidence/parity behavior exists.
- The renderer should stay deterministic and ANSI-free for CI screenshots and command output. A full alternate-screen `ratatui` event loop remains a later interactive shell slice.
- UI copy must use only real `FeatureSnapshot` and `HealthSnapshot` fields; no mock/live claims, fake confidence, or trading advice.

### Constraints
- Technical: preserve low-latency live ingestion; keep disk/network behavior unchanged; avoid new dependencies unless they remove real complexity.
- Operational: regenerate screenshots with `python3 scripts/generate-screenshots.py` from the rebuilt binary and inspect rendered assets.
- Risk/capital: read-only safety language must remain visible and no order/private terminology may be introduced.

### Options Considered
1. Add a full `ratatui`/`crossterm` interactive shell now.
   - Pros: aligns with the dominant Rust TUI ecosystem and unlocks real widgets, keyboard focus, and panes.
   - Cons: larger lifecycle/input/rendering change, higher regression risk, and not needed to provide polished screenshots now.
2. Upgrade the existing deterministic renderer into a cockpit-like market board and health panel.
   - Pros: tight scope, easy golden tests, screenshot-friendly, no ingestion risk, and immediately improves OSS presentation.
   - Cons: still not a keyboard-driven alternate-screen app.

### Chosen Approach
- Choice: option 2.
- Why: it maximizes visible quality and verification speed while keeping the live data path untouched. The plan will document the future Ratatui shell separately instead of mixing it into this polish patch.

### Execution Plan
1. Update focused TUI tests to expect the richer market board and health panel.
2. Refactor `hls-tui` formatting helpers as needed for reusable bars, status chips, and selected-row detail.
3. Implement a cockpit-style table with scan-friendly columns, truthful data-quality/read-only framing, and selected-symbol details.
4. Improve screenshot styling for the new layout and regenerate SVG screenshots.
5. Run focused tests, relevant CLI smokes, full Rust validation, screenshot generation, diff check, and read-only scans.

### Test Plan
- Focused: `cargo test -p hls-tui --test main_table_golden --test health_pane`.
- CLI/screenshot: `cargo test -p hls-cli --test live_mock --test health_commands`; `python3 scripts/generate-screenshots.py`; visual inspection of generated SVG/PNG previews.
- Regression: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `git diff --check`.

### Risks and Rollback
- Risks: wide tables may wrap in docs screenshots; glyph-heavy UI may render differently across terminal fonts; richer copy can accidentally overclaim current confidence/replay work.
- Rollback: revert this polish commit; no schema, data format, or network behavior changes are expected.

### Memory Impact
- Add/update in `MEMORY.md`: durable convention for deterministic TUI screenshot rendering if the renderer or screenshot command changes.

### Final Notes
- What changed: Upgraded the deterministic `hls-tui` market board into a wider workstation-style board with session/universe/quality/latency KPI strips, scan-friendly rows, combined score column, observation badges, and a selected-symbol microstructure detail pane. Upgraded the health panel into safety/connection/recorder/runbook lanes and carried `writer_warn_at` through `HealthSnapshot` so backlog thresholds are displayed truthfully. Refreshed tests, docs, screenshot styling, and all committed SVG screenshots from the rebuilt binary.
- Validation run: red/green `cargo test -p hls-tui --test main_table_golden --test health_pane`; `cargo test -p hls-cli --test live_mock --test health_commands`; full gate `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `python3 scripts/generate-screenshots.py`; SVG-to-PNG previews with `rsvg-convert`; `git diff --check`; read-only/safety scan. The first full gate caught stale smoke assertions, which were fixed and rerun successfully.
- Follow-ups: This remains deterministic command output rather than a full keyboard-driven alternate-screen Ratatui app. Spec Kit T020-T033 confidence/replay parity tasks remain open and should be implemented before claiming confidence-aware TUI completion.

## 2026-07-08 US4 Metadata-Backed TUI Polish

### Task
- Objective: Implement US4 public metadata enrichment and use it to make the deterministic TUI more professional, scan-friendly, and screenshot-ready.
- Owner repo(s): standalone `hlscreen/` repository only.
- Capital impact: research-only / read-only public market-data display. No wallet, private stream, signing, order placement, exchange action, execution route, or advice semantics.

### Context
- Background: US3 why-ranked explanations are merged. The next unchecked Spec Kit slice is US4 metadata enrichment (T058-T068). The latest operator request asks for a stronger next-gen TUI and screenshots, so the UI polish should be backed by real metadata row fields instead of static copy.
- Inputs: Official Hyperliquid public `spotMeta`, `spotMetaAndAssetCtxs`, and `tokenDetails` docs; `specs/002-microstructure-workstation/{spec.md,tasks.md,data-model.md}`; current screen DSL, TUI renderer, screenshot generator, and store registry.
- Outputs: Metadata fixtures/tests/model/parser/cache, metadata screen fields and presets, TUI metadata tags/details, regenerated SVG screenshots, docs/status notes, and validation evidence.

### Assumptions
- Metadata enrichment is optional and must not break live ingestion when public detail fields are missing or unavailable.
- `tokenDetails` can be parsed from public REST responses and used in fixture-backed tests; live startup should avoid turning metadata-detail availability into a hard dependency for WebSocket ingestion.
- The deterministic renderer remains ANSI-free for CI, docs, and screenshot stability. A full alternate-screen Ratatui event loop remains a separate future feature.

### Constraints
- Technical: no private/account streams, no new trading surfaces, no fake metadata, no hidden fallback that pretends missing fields are known.
- Operational: screenshots must be regenerated from the compiled `hls` binary via `python3 scripts/generate-screenshots.py` and visually inspected through PNG previews.
- Risk/capital: labels must stay as screening/metadata context only, not trade signals or recommendations.

### Options Considered
1. Restyle only the current TUI output.
   - Pros: fastest visible improvement.
   - Cons: does not advance US4 and would make the screenshot polish mostly cosmetic.
2. Implement US4 metadata enrichment and render it in the TUI.
   - Pros: improves the workstation with real venue-native information, completes a Spec Kit story, and creates a better screenshot surface.
   - Cons: touches more crates and requires careful partial-data behavior.

### Chosen Approach
- Choice: option 2.
- Why: it ties the visual upgrade to real product capability while preserving the read-only and deterministic-output constraints.

### Execution Plan
1. Add metadata fixture and focused tests for core model, public REST adapter, store cache freshness, screen fields, and presets.
2. Implement `hls-core::metadata` and attach optional metadata to `FeatureSnapshot`.
3. Extend `hls-hyperliquid::rest` parsing/fetch helpers for public token details and metadata enrichment from public payloads.
4. Persist metadata cache freshness in SQLite.
5. Expose metadata fields through `hls-screen` and add new-listing/fresh-liquidity/unknown-metadata presets.
6. Render metadata tags/details in `hls-tui`, update the screenshot generator, and regenerate assets.
7. Run focused tests, full Rust validation, screenshot generation/preview, diff check, and read-only scans.

### Test Plan
- Focused: `cargo test -p hls-core --test metadata_enrichment`; `cargo test -p hls-hyperliquid --test metadata_enrichment`; `cargo test -p hls-store --test metadata_registry`; `cargo test -p hls-screen --test metadata_presets`; `cargo test -p hls-tui --test main_table_golden`.
- CLI/screenshot: `python3 scripts/generate-screenshots.py`; PNG previews via `rsvg-convert`.
- Regression: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `git diff --check`.

### Risks and Rollback
- Risks: public metadata schemas may add or omit fields; large live token-detail fetches could add startup latency if wired too eagerly; metadata tags could be mistaken for trade recommendations if copy is sloppy.
- Rollback: revert this US4 patch; SQLite metadata cache table is additive and local-only.

### Memory Impact
- Add/update in `MEMORY.md`: metadata enrichment contract, confirmed test/screenshot commands, and partial-metadata behavior if validated.

### Final Notes
- What changed: Implemented US4 metadata-backed TUI polish. Added `MetadataEnrichment` and cohort tags in `hls-core`; parsed public Hyperliquid `spotMetaAndAssetCtxs` plus `tokenDetails` bundles in `hls-hyperliquid`; added optional metadata to `FeatureSnapshot`; persisted metadata cache freshness in SQLite; exposed metadata fields and presets through `hls-screen`; attached metadata in CLI live/screen adapters; added TUI metadata KPI/chip/detail rendering; added `metadata-discovery.svg` and refreshed screenshots/docs.
- Validation run: `cargo test -p hls-core --test metadata_enrichment`; `cargo test -p hls-hyperliquid --test metadata_enrichment --test rest_metadata`; `cargo test -p hls-screen --test metadata_presets --test presets --test dsl_evaluator`; `cargo test -p hls-store --test metadata_registry`; `cargo test -p hls-tui --test main_table_golden --test confidence_pane --test health_pane`; `cargo test -p hls-cli --test live_mock --test screen_command --test full_pipeline_smoke`; `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `python3 scripts/generate-screenshots.py`; `rsvg-convert` previews for live, metadata, and health screenshots; `git diff --check`; read-only scan.
- Tradeoffs: live startup carries partial public `spotMetaAndAssetCtxs` metadata from selected markets, while token-detail enrichment is bounded/optional and fixture-backed for deterministic screenshots; missing detail fields are explicit `unknown_metadata` instead of a hard failure.
- Merge: PR #20 passed GitHub `Rust workspace` CI and merged into `main` at `4f324ae`.
- Follow-ups: US5 operations/benchmark/metrics/extension/release tasks T069-T082 and polish/report tasks T083-T092 remain open after this PR.

## 2026-07-08 US1 Confidence and Replay Parity

### Task
- Objective: Implement Spec Kit microstructure workstation US1 tasks T020-T033: confidence-aware data quality snapshots, replay parity checks, CLI `--verify-parity`, TUI confidence rendering, persistence, fixtures, docs, and task markers.
- Owner repo(s): standalone `hlscreen/` repository only.
- Capital impact: research-only / read-only public market-data quality and replay validation. No wallet, private stream, signing, order placement, exchange action, execution route, or advice semantics.

### Context
- Background: Foundation contracts T001-T019 and next-gen deterministic TUI polish are merged. The active pasted brief prioritizes trustworthy local data, gap-aware replay, explicit confidence under stale/sparse/reconnect conditions, and deterministic research workflows before ranking complexity.
- Inputs: `specs/002-microstructure-workstation/tasks.md` T020-T033, `contracts/confidence-and-scoring.md`, `data-model.md`, current official Hyperliquid docs for WebSocket endpoint/subscriptions/reconnect guidance/rate limits, and existing `hls-core`, `hls-features`, `hls-store`, `hls-cli`, and `hls-tui` patterns.
- Outputs: Microstructure fixtures, runtime confidence on `FeatureSnapshot`, confidence metadata persistence, replay parity checker and CLI flag, TUI confidence columns/summary, docs, tests, checked task markers, validation evidence, and a PR/merge if stable.

### Assumptions
- Confidence computation can be state-derived by default and enriched through explicit runtime inputs for reconnect gaps, parser drops, and writer backlog. Hidden globals or fake live values are not acceptable.
- Replay parity should compare persisted confidence baselines to recomputed replay confidence and return a non-zero CLI exit on drift.
- Recorded baseline persistence can happen in the replay command when `--verify-parity` is requested and no baseline exists, but drift detection must use a real persisted baseline when present.
- This slice does not implement US2 resilience metrics or US3 score breakdown rendering.

### Constraints
- Technical: preserve deterministic replay; avoid broad rewrites of recorder or live ingestion; keep SQLite schema additive; no ANSI-only output in stable TUI strings.
- Operational: keep runtime market-data captures out of git; fixtures must be small and public-shape only.
- Risk/capital: confidence is data-quality evidence only, not trade safety or performance proof.

### Options Considered
1. Compute confidence only in the TUI renderer.
   - Pros: small visual diff.
   - Cons: not replayable, not persistable, and fails the US1 data-quality contract.
2. Attach confidence to `FeatureSnapshot` in `hls-core` and compute it in `hls-features`, with explicit runtime quality inputs for gaps/drops/backlog.
   - Pros: shared by live/replay/screen/TUI, deterministic, testable, and compatible with persistence.
   - Cons: touches shared snapshot contracts and requires updating downstream tests.

### Chosen Approach
- Choice: option 2.
- Why: confidence must travel with the data row so replay, CLI, TUI, and future scoring all see the same quality evidence.

### Execution Plan
1. Add reconnect-gap and sparse-trade public-shape fixtures.
2. Add failing tests for duplicate confidence, replay parity, CLI `--verify-parity`, and TUI confidence rendering.
3. Add `confidence` to `FeatureSnapshot` and track duplicate trade counts in `SymbolMarketState`.
4. Compute confidence from staleness, sparse windows, duplicate events, parser drops, writer backlog, and gap symbols in `hls-features`.
5. Add confidence metadata persistence and replay parity comparison in `hls-store`.
6. Wire `hls replay --verify-parity` and confidence summaries in replay/live output.
7. Render confidence state in TUI rows and docs, then mark T020-T033 complete only if all behavior is true.
8. Run focused tests, full Rust gates, diff/read-only scans, update memory/reflection, commit, push, PR, and merge only if stable.

### Test Plan
- Focused: `cargo test -p hls-core --test confidence_state --test market_state`; `cargo test -p hls-features --test formulas`; `cargo test -p hls-store --test replay_parity --test metadata_registry`; `cargo test -p hls-cli --test replay_parity_command`; `cargo test -p hls-tui --test confidence_pane --test main_table_golden`.
- Regression: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `python3 scripts/generate-screenshots.py`; `git diff --check`.
- Safety: read-only scan for private/order/execution surfaces and docs review for confidence-as-quality wording only.

### Risks and Rollback
- Risks: adding confidence to `FeatureSnapshot` updates many golden assertions; parity baseline semantics can become misleading if baseline generation and verification happen in the same step; duplicate tracking must not reintroduce duplicate trades.
- Rollback: revert this US1 commit/PR; SQLite schema addition is additive and only affects local metadata files created after the change.

### Memory Impact
- Add/update in `MEMORY.md`: confidence computation boundary, replay parity command, and confirmed test commands.

### Final Notes
- What changed: Completed Spec Kit US1 tasks T020-T033. Added gap/sparse fixtures, confidence on `FeatureSnapshot`, duplicate observation tracking, state-derived and explicit-input confidence computation, SQLite `confidence_snapshots`, replay parity reports, `hls replay --verify-parity`, confidence summaries in live/replay command output, TUI confidence rendering, parity-aware screenshots, and docs for confidence states and replay parity.
- Validation run: red/green `cargo test -p hls-store --test replay_parity` and `cargo test -p hls-cli --test replay_parity_command`; focused tests `cargo test -p hls-core --test confidence_state --test market_state`; `cargo test -p hls-features --test formulas`; `cargo test -p hls-store --test replay_parity --test metadata_registry`; `cargo test -p hls-cli --test replay_parity_command --test record_replay --test live_mock --test full_pipeline_smoke`; `cargo test -p hls-tui --test main_table_golden --test confidence_pane`; `python3 scripts/generate-screenshots.py`; PNG preview of `docs/assets/screenshots/record-replay.svg`; `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `git diff --check`; JSONL validation; read-only surface scan; fixture parity smoke with `replay_parity=baseline_written` then `replay_parity=passed`; and a real public WebSocket smoke `./target/debug/hls live --symbols @107 --duration-secs 10 --refresh-secs 5 --tui` with 34 WebSocket messages, 60 market events, 0 reconnects, and 0 data gaps.
- Follow-ups: US1 is complete, but the full v2 workstation objective remains active. Next unchecked slices start at US2 liquidity resilience/tradeability T034-T045, then US3 why-ranked explanations, US4 metadata enrichment, US5 OSS operations, and polish tasks T083-T092.

## 2026-07-08 US2 Liquidity Resilience TUI

### Task
- Objective: Implement Spec Kit microstructure workstation US2 tasks T034-T045 so the next-gen TUI shows real BBO/trade-derived liquidity resilience, tradeability, BBO order-flow proxy, and signed flow fields instead of cosmetic-only polish.
- Owner repo(s): standalone `hlscreen/` repository only.
- Capital impact: research-only / read-only public market-data analytics. No wallet, private stream, signing, order placement, exchange action, execution route, or advice semantics.

### Context
- Background: US1 confidence/replay parity is merged. The user now wants the TUI to look "very very good" and current product contracts say the workstation board should surface resilience and tradeability indicators. Hyperliquid's public WebSocket docs describe `trades` and `bbo` subscriptions, and note BBO updates are emitted when BBO changes on a block; these are the only production data sources for this slice.
- Inputs: `specs/002-microstructure-workstation/tasks.md` T034-T045, `contracts/cli-tui.md`, existing public WebSocket parser/state, `FeatureSnapshot`, `hls-features`, `hls-screen`, `hls-tui`, screenshot generator, and Hyperliquid public WebSocket subscription docs.
- Outputs: Spread-shock and thin-book fixtures, resilience/tradeability feature tests, new feature snapshot fields, screen DSL/preset exposure, TUI resilience/tradeability rendering, docs caveats, regenerated screenshots, validation evidence, and PR/merge if stable.

### Assumptions
- BBO-only metrics must be labeled as top-of-book proxies. They are not full order-book depth, not an execution simulator, and not a trade recommendation.
- The first state machine can use deterministic recent BBO/trade windows stored in `SymbolMarketState`; it does not need a separate streaming actor.
- Screenshots can be fixture-backed for determinism, but the screenshot command must exercise the same production parser/state/feature/TUI code path as live public data.

### Constraints
- Technical: preserve low-latency live ingestion; keep feature computation bounded; avoid external dependencies; keep CLI output deterministic and ANSI-free outside live refresh clearing.
- Operational: keep runtime captures out of git; fixtures must stay small and public-shape only; no hidden mock behavior in production commands.
- Risk/capital: tradeability is a screen heuristic only. It must account for confidence and cost proxies without implying profitability, suitability, or order execution.

### Options Considered
1. Add a visual-only TUI redesign.
   - Pros: faster screenshot improvement.
   - Cons: does not satisfy US2 or the product contract and risks fake-looking polish without new evidence.
2. Implement real US2 feature fields first, then render them in the existing deterministic workstation board.
   - Pros: adds truthful microstructure content, unlocks presets/DSL, and makes screenshots represent real computed data.
   - Cons: touches shared snapshot contracts and requires broad tests.

### Chosen Approach
- Choice: option 2.
- Why: the TUI should look next-gen because it exposes better real-time information, not because the same rows have more decoration.

### Execution Plan
1. Add fixture sequences for spread shock/recovery and brittle thin-book states.
2. Add focused tests for resilience metrics, tradeability classification, and microstructure presets.
3. Extend market state with bounded BBO observations and add resilience/tradeability fields to feature snapshots.
4. Implement BBO-only spread-shock/recovery, BBO OFI proxy, signed notional flow, and tradeability classification in `hls-features`.
5. Expose new fields through `hls-screen` row DSL and presets.
6. Render resilience/tradeability in the market board and selected-symbol detail; update screenshots from the real binary.
7. Document BBO-only caveats, mark T034-T045 complete only after behavior is implemented, then run focused and full validation.

### Test Plan
- Focused: `cargo test -p hls-features --test resilience --test tradeability`; `cargo test -p hls-screen --test microstructure_presets`; `cargo test -p hls-tui --test main_table_golden --test confidence_pane`.
- CLI/screenshot: `python3 scripts/generate-screenshots.py`; fixture live screen commands; preview generated SVG/PNG assets.
- Regression: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `git diff --check`.
- Live smoke: bounded public `hls live --symbols @107 --duration-secs 10 --refresh-secs 5 --tui` after the full gate.

### Risks and Rollback
- Risks: simple BBO windows can overstate resilience if quote updates are sparse; signed flow inferred from public trade side is a proxy; adding fields can break downstream golden tests.
- Rollback: revert this branch/PR; no storage migration or external state mutation is introduced.

### Memory Impact
- Add/update in `MEMORY.md`: US2 resilience/tradeability field definitions, BBO-only caveat, and confirmed validation/screenshot commands.

### Final Notes
- What changed: Completed Spec Kit US2 tasks T034-T045 and screenshot task T086. Added spread-shock and thin-brittle public-shape fixtures, bounded BBO history in `SymbolMarketState`, new `FeatureSnapshot` fields for spread shock, recovery, resilience state, tradeability state, adverse-selection proxy, signed notional flow, and BBO OFI proxy, pure `hls-features::{resilience,tradeability}` modules, screen DSL fields and presets, TUI resilience/tradeability columns plus selected-symbol detail, BBO-only docs caveats, and a new `docs/assets/screenshots/resilience-screen.svg` asset linked from the README.
- Validation run: red/green focused tests `cargo test -p hls-features --test resilience --test tradeability`, `cargo test -p hls-screen --test microstructure_presets`, and `cargo test -p hls-tui --test main_table_golden --test confidence_pane`; broader focused suite `cargo test -p hls-features --test formulas --test resilience --test tradeability`, `cargo test -p hls-screen --test dsl_evaluator --test presets --test microstructure_presets`, and `cargo test -p hls-tui --test main_table_golden --test confidence_pane`; `python3 scripts/generate-screenshots.py`; PNG preview render with `rsvg-convert` for `live-screen`, `confidence-degraded`, and `resilience-screen`; `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo build --release --workspace --all-features`; `git diff --check`; screenshot temp-path scan; SVG asset sanity check; read-only/safety scan.
- Live smoke: `./target/debug/hls live --symbols @107 --duration-secs 10 --refresh-secs 5 --tui` completed against the public Hyperliquid WebSocket with 1 symbol, 4 public subscriptions, 79 WebSocket messages, 106 market events, 0 reconnects, 0 data gaps, high confidence, and live resilience/tradeability/proxy fields rendered in the TUI.
- Tradeoffs: `bbo_ofi_proxy_30s` and `adverse_selection_proxy` are explicitly BBO/top-of-book proxies. They do not claim full order-book depth, hidden liquidity, fill quality, profitability, or execution safety. BBO history is bounded to 256 recent quote observations per symbol for deterministic local memory use.
- Follow-ups: The full v2 objective remains active. Next unchecked slices are US3 why-ranked explanations, US4 metadata enrichment, US5 operations/packaging/extension work, and remaining polish/report tasks.

## 2026-07-08 US3 Why-Ranked TUI Detail

### Task
- Objective: Make the TUI feel production-grade by adding deterministic why-ranked score explanations, exposing score components to screen filters/sorts, adding `hls explain`, and regenerating screenshots from the real binary.
- Owner repo(s): standalone `hlscreen/` repository only.
- Capital impact: research-only / read-only public market-data explanation. No wallet, private stream, signing, order placement, exchange action, execution route, or advice semantics.

### Context
- Background: US2 resilience/tradeability fields are merged. The next visible quality gap is explainability: ranked rows show scores but do not explain positive, negative, or confidence-adjusted contributions.
- Inputs: `specs/002-microstructure-workstation/tasks.md` T046-T057, `contracts/cli-tui.md`, `contracts/confidence-and-scoring.md`, current `FeatureSnapshot`, `hls-features`, `hls-screen`, `hls-tui`, `hls-cli`, and screenshot generator.
- Outputs: score-component fixture, score model/tests, generated score breakdowns, screen DSL fields, why-ranked pane, `hls explain`, docs, screenshots, validation evidence, and PR/merge if stable.

### Assumptions
- Score components are deterministic screen heuristics computed from public BBO/trade/context fields already present in `FeatureSnapshot`.
- Missing evidence must be surfaced explicitly instead of silently imputing fake values.
- The deterministic text renderer remains the right v1 UI contract because it is CI-testable, screenshot-friendly, and works over replay without a live terminal.

### Constraints
- Technical: preserve backward-compatible score constructors where practical; avoid hidden network calls in `hls explain`; keep filters deterministic and missing values non-matching.
- Operational: screenshots may be fixture-backed, but they must use the same parser/state/feature/TUI path as live public data.
- Risk/capital: score explanations must say "screen heuristic" and "not advice"; no order/private/user/account wording.

### Options Considered
1. Visual-only restyling of the existing table.
   - Pros: smaller diff.
   - Cons: does not explain rankings and fails the US3 contract.
2. Add score breakdowns at the feature boundary and render a dedicated why-ranked pane.
   - Pros: makes CLI, TUI, replay, screen filters, and screenshots share one truthful explanation model.
   - Cons: touches shared snapshot contracts and requires broad test updates.

### Chosen Approach
- Choice: option 2.
- Why: the TUI looks and behaves more professional when it exposes real evidence and caveats instead of only adding decoration.

### Execution Plan
1. Extend score component modeling with signed contribution metadata and unavailable evidence.
2. Attach score breakdowns to feature snapshots from real public microstructure fields.
3. Expose `score_total`, `score_raw_total`, `score_confidence_penalty`, and `score_component.<name>` through `hls-screen`.
4. Add `hls-tui::detail::render_why_ranked_pane` and use it in a new `hls explain` command.
5. Add fixtures/tests/docs and regenerate screenshots, including a why-ranked screenshot.
6. Mark T046-T057 only after behavior and validation pass.

### Test Plan
- Focused: `cargo test -p hls-core --test score_breakdown`; `cargo test -p hls-features --test formulas`; `cargo test -p hls-screen --test dsl_evaluator`; `cargo test -p hls-tui --test why_ranked_pane --test main_table_golden`; `cargo test -p hls-cli --test explain_command --test screen_command`.
- Screenshot/docs: `python3 scripts/generate-screenshots.py`; render PNG previews of updated SVGs where available.
- Regression: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `git diff --check`.
- Safety: read-only scan for private/order/execution surfaces and docs review for score-as-screen-heuristic wording only.

### Risks and Rollback
- Risks: component weights can look like trading advice if labels are careless; adding `score_breakdown` to `FeatureSnapshot` requires updating manual test rows; dynamic `score_component.<name>` parsing can break existing field parsing if not tested.
- Rollback: revert this branch/PR; no storage migration, live external mutation, or config change is expected.

### Memory Impact
- Add/update in `MEMORY.md`: score explanation boundary, `hls explain` command, and deterministic screenshot command if confirmed.

### Final Notes
- What changed: Completed Spec Kit US3 tasks T046-T057. Added a richer score component contract with direction, raw/normalized values, weights, signed contributions, evidence windows, and unavailable evidence; attached deterministic `ScoreBreakdown` values to feature snapshots; exposed `score_total`, `score_raw_total`, `score_confidence_penalty`, and `score_component.<name>` through the screen DSL; added `hls-tui::detail::render_why_ranked_pane`; added `hls explain` text/JSON output over replayed or fixture-backed data; updated docs and regenerated screenshots including `docs/assets/screenshots/why-ranked.svg`.
- Validation run: focused green `cargo test -p hls-core --test score_breakdown`, `cargo test -p hls-features --test formulas`, `cargo test -p hls-screen --test dsl_parser --test dsl_evaluator`, `cargo test -p hls-tui --test why_ranked_pane --test main_table_golden`, and `cargo test -p hls-cli --test explain_command --test screen_command`; `python3 scripts/generate-screenshots.py`; PNG visual preview of `why-ranked.svg`; `cargo fmt --check`; `git diff --check`; read-only/safety scan; fixed clippy too-many-arguments finding; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`.
- Follow-ups: PR #19 merged into `main` at `4c8b10d` after GitHub `Rust workspace` passed. US3 is complete. Remaining v2 work starts at US4 metadata enrichment T058-T068, then US5 operations/packaging/extension tasks and final polish/report tasks.

## 2026-07-08 All-Pair TUI Detail Cards

### Task
- Objective: Upgrade the deterministic workstation TUI so every rendered pair has a compact detail card matching the mock/product surface, not only the first/selected row.
- Owner repo(s): standalone `hlscreen/` repository only.
- Capital impact: research-only / read-only public market-data visualization. No wallet, private stream, signing, order placement, execution route, or trading advice.

### Context
- Background: The current market board is visually improved and uses real `FeatureSnapshot` data, but richer microstructure detail is still concentrated in a single `SELECTED SYMBOL` section. The requested mock calls for per-pair visibility into price, 24h volume, mid/mark, bid/ask, spread, TOB depth/imbalance, returns, realized volatility, activity z-scores, liquidity/momentum/mean-reversion scores, confidence, flow, resilience, and metadata.
- Inputs: `/Users/s1kor/.codex/attachments/24f4f400-2fac-4700-a096-ce4c1a31397d/pasted-text.txt`, `crates/hls-core/src/market_state.rs`, `crates/hls-tui/src/app.rs`, `crates/hls-tui/tests/main_table_golden.rs`, `crates/hls-cli/tests/live_mock.rs`, and `scripts/generate-screenshots.py`.
- Outputs: per-pair detail-card rendering from existing live snapshot fields, updated golden/CLI tests, regenerated screenshots, validation evidence, and PR/merge if stable.

### Assumptions
- The v1 TUI contract remains deterministic text/SVG-friendly output rather than a keyboard-driven ratatui app.
- Missing fields must render as `-`; no fixture-only placeholder or mock value should be invented to make the UI look fuller.
- The pair detail section should render all visible rows after any screen filter/sort is applied.

### Constraints
- Technical: keep rendering bounded and pure; avoid ANSI escape codes in golden output; preserve low-latency live ingestion by changing only display code/tests.
- Operational: screenshots can be deterministic fixture outputs, but must use the same parser/state/feature/TUI path as live data.
- Risk/capital: all score/resilience/flow text must remain screen heuristics and top-of-book proxies, not advice or execution readiness.

### Options Considered
1. Keep the board as-is and only widen row columns.
   - Pros: smallest diff.
   - Cons: still hides mock-required fields for all but one pair and does not answer the request.
2. Replace the selected-symbol detail pane with repeated compact pair detail cards for every visible pair.
   - Pros: gives each pair the same truthful field coverage and keeps deterministic screenshots professional.
   - Cons: output is taller for large universes.
3. Add a separate interactive TUI crate/runtime now.
   - Pros: could support keyboard selection later.
   - Cons: larger architecture shift than needed and not required to fix the current per-pair visibility gap.

### Chosen Approach
- Choice: option 2.
- Why: it directly matches the mock, preserves the current production data path, and avoids introducing an unproven interactive runtime.

### Execution Plan
1. Add/update tests so golden output expects `PAIR DETAIL CARDS` and verifies two rendered rows each expose price ladder, microstructure windows, activity/score fields, quality/flow, metadata, confidence, and why-ranked summary.
2. Refactor `crates/hls-tui/src/app.rs` to render a compact card per row from existing `FeatureSnapshot` fields.
3. Update CLI smoke expectations that currently look for `SELECTED SYMBOL`.
4. Regenerate screenshots from the current binary and visually inspect PNG previews.
5. Run focused TUI/CLI tests plus full workspace fmt, clippy, tests, build, release build, diff check, and a bounded public live smoke.

### Test Plan
- Focused: `cargo test -p hls-tui --test main_table_golden`; `cargo test -p hls-cli --test live_mock --test screen_command`.
- Screenshot/docs: `python3 scripts/generate-screenshots.py`; PNG preview `live-screen.svg` and `metadata-discovery.svg` with `rsvg-convert`.
- Regression: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo build --release --workspace --all-features`; `git diff --check`.
- Live smoke: bounded read-only public `./target/debug/hls live --symbols @107 --duration-secs 10 --refresh-secs 5 --tui`.

### Risks and Rollback
- Risks: all-pair cards make long all-symbol output taller; screenshot assets may need larger view boxes; manual golden assertions can become brittle if labels are too verbose.
- Rollback: revert the TUI/test/screenshot commit; no storage, network, config, or external state changes are introduced.

### Memory Impact
- Add/update in `MEMORY.md`: per-pair detail-card TUI contract and confirmed validation/screenshot commands if the slice passes.

### Final Notes
- What changed: Replaced the selected-symbol-only TUI detail pane with `PAIR DETAIL CARDS` rendered for every visible row after screening. Each card uses existing `FeatureSnapshot` fields for price, 24h notional, bid/ask, mid/mark, spread, TOB depth/imbalance, 1m/5m/1h returns and realized volatility, volume/trade z-scores, liquidity/momentum/mean-reversion scores, flow, resilience, metadata, confidence, and why-ranked summary. Updated CLI/TUI tests, screenshot highlighting, regenerated SVG assets, repo memory, daily memory, and lesson store.
- Validation run: red/green `cargo test -p hls-tui --test main_table_golden`; `cargo test -p hls-cli --test live_mock --test screen_command`; `python3 scripts/generate-screenshots.py`; PNG previews with `rsvg-convert` for `live-screen`, `metadata-discovery`, and `resilience-screen`; `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo build --release --workspace --all-features`; `git diff --check`; read-only public smoke `./target/debug/hls live --symbols @107 --duration-secs 10 --refresh-secs 5 --tui` with 57 WebSocket messages, 83 market events, 0 reconnects, and 0 data gaps.
- Follow-ups: PR #23 merged into `main` at `c7584a2` after PR checks passed; post-merge `main` CI run `28957709994` passed. Full keyboard-driven interactive TUI remains future work. This slice keeps the stable deterministic renderer and does not add trading/private/order surfaces.

## 2026-07-08 End-to-End Audit After Pair Cards

### Task
- Objective: Audit the current `main` implementation end-to-end against official Hyperliquid docs, repo/Spec Kit contracts, Rust CLI/TUI best practices, and production-readiness gates; fix any findings, publish an audit report, and merge only after PR and post-merge CI pass.
- Owner repo(s): standalone `hlscreen/` repository only.
- Capital impact: research-only / read-only public market-data tooling. No wallet, private stream, signing, order placement, execution route, account mutation, or trading action.

### Context
- Background: `main` now includes all-pair TUI detail cards. The user requested a full code, runtime, integration, service/log, edge-case, and code-review audit before further promotion.
- Inputs: Hyperliquid official API docs for WebSocket endpoint, subscriptions, heartbeat, spot Info endpoints, and rate limits; `specs/002-microstructure-workstation/*`; current source tree; CI configuration; generated screenshots; existing public live smoke evidence.
- Outputs: audit report under `docs/reports/`, updated plan/TODO/memory, any required fixes, local validation evidence, PR, merge, and post-merge CI evidence.

### Assumptions
- No long-running production service is required for this repo; service verification means running CLI/API preview commands, live public WebSocket smokes, local replay/screen/bench flows, and log/stdout/stderr review.
- A deterministic audit report is a legitimate review artifact even if no runtime code changes are required.
- External network live smokes may vary in message counts but must exit cleanly, remain read-only, and report reconnect/data-gap counts explicitly.

### Constraints
- Technical: keep all changes scoped to findings/reporting; do not weaken tests, benchmark hashes, or safety scans.
- Operational: do not mutate external systems beyond public read-only Hyperliquid REST/WebSocket calls and GitHub PR/merge operations.
- Risk/capital: no private endpoints, account addresses, keys, order-capable commands, signed actions, or advice language.

### Options Considered
1. Run only the existing CI suite and report green.
   - Pros: fast.
   - Cons: does not satisfy the requested end-to-end review, live behavior, logs, edge cases, or official-doc alignment.
2. Perform a full local audit matrix, source review, live/replay smokes, security/read-only scans, screenshot checks, report the evidence, and merge via PR.
   - Pros: creates a reviewable readiness artifact and catches docs/code drift beyond tests.
   - Cons: takes longer and may require small follow-up fixes.

### Chosen Approach
- Choice: option 2.
- Why: this is a trading-adjacent public-data tool; false confidence is more expensive than an extra validation pass.

### Execution Plan
1. Compare current REST/WS/live/runtime behavior against official docs and Spec Kit contracts.
2. Inspect source modules for parser, REST, live loop, recorder, feature engine, screen DSL, TUI, API helpers, metrics, benchmark, extension, and CI/release workflow boundaries.
3. Run focused edge tests and smokes for fixture live, full pipeline, replay parity, screen/explain/bench/doctor/server, screenshots, and live public WebSocket.
4. Run full gates: fmt, clippy, tests, build, release build, release packaging, diff check, read-only/security/dead-code scans.
5. Capture findings and fixes in `docs/reports/2026-07-08-e2e-audit-after-pair-cards.md`.
6. PR, review, merge, wait for post-merge CI, then close memory/TODO/plan.

### Test Plan
- Official-doc alignment: manual/code review of `hls-hyperliquid`, `hls-cli live`, and docs against current Hyperliquid docs.
- Focused CLI/runtime: `hls symbols`, fixture `hls live`, fixture record/replay/screen, `hls explain`, `hls bench`, `hls doctor --live --json`, `hls server --print-health`, screenshot generation and PNG preview.
- Edge/safety: invalid DSL, missing fixture, invalid benchmark path, read-only/private/order scan, TODO/debug scan, log/stdout/stderr review.
- Full gates: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo build --release --workspace --all-features`; `scripts/check-release-packaging.sh`; `git diff --check`.
- Live smoke: bounded public `./target/debug/hls live --symbols @107 --duration-secs 10 --refresh-secs 5 --tui` and a short multi-symbol/all-symbol budget probe where safe.

### Risks and Rollback
- Risks: external WebSocket may be temporarily quiet or unavailable; generated docs/screenshots could churn; audit report could overclaim if not tied to commands actually run.
- Rollback: revert any audit/fix commits on this branch; no external market/account state is modified.

### Memory Impact
- Add/update in `MEMORY.md`: confirmed audit matrix, live smoke results, and any durable sharp edge or command discovered.

### Final Notes
- What changed: Completed a repo-wide source/runtime audit and recorded it in `docs/reports/2026-07-08-e2e-audit-after-pair-cards.md`. Fixed a TUI truthfulness bug where missing spread/depth evidence could still show `QUALITY ... GOOD`; missing quote/depth evidence now reports `PARTIAL` and is covered by `missing_quote_depth_marks_quality_partial`. Fixed screenshot reproducibility by normalizing volatile `generated_at_ms` values in `scripts/generate-screenshots.py`, and regenerated `docs/assets/screenshots/health-json.svg`.
- Validation run: official-doc/code review of public Hyperliquid WS/REST integration; fixture matrix under `/tmp/hlscreen-e2e-audit.1c4dZD` with zero non-empty stderr logs; negative probes for invalid DSL, fixture live without `--once`, deterministic record without fixture, and private benchmark path; public live smoke `./target/debug/hls live --symbols @107 --duration-secs 15 --refresh-secs 5 --tui` with 92 WS messages, 129 market events, 0 reconnects, 0 data gaps; public multi-symbol live smoke after the fix with 60 WS messages, 135 market events, 0 reconnects, 0 data gaps, and `QUALITY ... PARTIAL` for missing spread/depth evidence; PNG previews with `rsvg-convert`; final `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo build --release --workspace --all-features`; `scripts/check-release-packaging.sh`; `python3 scripts/generate-screenshots.py`; `git diff --check`; read-only/private/dead-code scans.
- Follow-ups: PR #25 merged into `main` at `b6a188a` after PR checks passed; post-merge `main` CI run `28959783411` passed format, clippy, tests, release build, release packaging check, and diff hygiene. Residual caveats remain documented: no automatic REST backfill after reconnect, no long-running HTTP server loop, and no installed `cargo audit`/`cargo deny`/`cargo machete` result claimed.

## 2026-07-08 CI/CD and Dependabot PR Hygiene

### Task
- Objective: Verify current CI/CD and open PR status, identify any red PRs, and prevent known unsupported automated dependency updates from repeatedly opening failing PRs.
- Owner repo(s): standalone `hlscreen/` repository only.
- Capital impact: research-only / repository operations. No market, private-account, order, or execution behavior changes.

### Context
- Background: `main` is green after PR #25/#26, but the open PR list still includes Dependabot updates. User asked to ensure pipelines are good and PRs are green.
- Inputs: GitHub Actions run state, open PR check rollups, `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `.github/dependabot.yml`, pinned Rust `1.88`, pinned cargo-dist `0.32.0`, and GitHub Dependabot ignore documentation.
- Outputs: scoped Dependabot ignore policy for unsupported update classes, validation evidence, and PR/merge if stable.

### Assumptions
- The repo should keep Rust 1.88 MSRV unless a dedicated dependency-upgrade slice explicitly changes it.
- The generated cargo-dist release workflow should remain managed by pinned cargo-dist, not manual workflow edits that `dist plan` rejects.
- Green automated dependency PRs can remain open for normal review; unsupported red dependency PRs should be closed or superseded after the policy lands.

### Constraints
- Technical: do not weaken CI gates, release packaging checks, or `dist plan` validation.
- Operational: GitHub branch-protection API is unavailable while the repo is private on the current account tier; verify run status and workflow behavior instead.
- Risk/capital: CI changes must not touch runtime market-data behavior or any trading-capable surface.

### Options Considered
1. Rerun the red PR checks.
   - Pros: no code/config changes.
   - Cons: failures are deterministic and would waste CI minutes.
2. Raise MSRV and/or cargo-dist version immediately to satisfy Dependabot major/minor updates.
   - Pros: could make red PRs mergeable.
   - Cons: changes repo support policy and release generation in a broader slice than requested.
3. Add targeted Dependabot ignores for the unsupported update classes while preserving current CI and release gates.
   - Pros: keeps `main` green, documents why red PRs are invalid under current policy, and avoids recurring false-red PRs.
   - Cons: requires revisiting when the repo intentionally upgrades Rust MSRV or cargo-dist.

### Chosen Approach
- Choice: option 3.
- Why: the current `main` pipeline is already green; the risk is automated noise from updates that conflict with explicit repo constraints.

### Execution Plan
1. Inspect GitHub auth, workflows, current `main` Actions, open PRs, and failing PR logs.
2. Add Dependabot ignore policy for `rusqlite` semver-minor updates under Rust 1.88 and `actions/checkout` semver-major updates while cargo-dist 0.32.0 owns release workflow generation.
3. Validate YAML, release packaging, CI-equivalent local gates, and GitHub checks.
4. Close or supersede unsupported red Dependabot PRs after the policy PR lands so remaining open PRs are green.

### Test Plan
- GitHub: `gh pr list --state open --json ...`, failing-check inspection for PR #4/#7, `gh run list --branch main --limit 10 --json ...`.
- Local config: Ruby YAML parse for `.github/dependabot.yml`.
- Gates: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --release --workspace --all-features`; `scripts/check-release-packaging.sh`; `git diff --check`.

### Risks and Rollback
- Risks: overly broad ignore rules can hide useful dependency work; comments may drift when MSRV/cargo-dist policy changes.
- Rollback: remove the two ignore entries and rerun Dependabot after a reviewed Rust/cargo-dist upgrade.

### Memory Impact
- Add/update in `MEMORY.md`: record the CI/Dependabot policy and exact red PR root causes if validated.

### Final Notes
- What changed: Added scoped Dependabot ignore rules for two unsupported automated update classes: `rusqlite` semver-minor updates while the repo MSRV is Rust 1.88, and `actions/checkout` semver-major updates while pinned cargo-dist 0.32.0 owns the generated release workflow. This keeps the current CI/CD contract strict instead of allowing known-invalid automation PRs to stay red.
- Validation run: `gh auth status`; `gh repo view --json nameWithOwner,defaultBranchRef,url,isPrivate`; `gh pr list --state open --json ...`; `gh run list --branch main --limit 10 --json ...`; failing-check inspection for PR #4, #7, and #28; `.github/workflows/ci.yml`, `.github/workflows/release.yml`, and `.github/dependabot.yml` review; Ruby YAML parse; `cargo fmt --check`; `scripts/check-release-packaging.sh`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --release --workspace --all-features`; `git diff --check`.
- Follow-ups: PR #27 merged the Dependabot policy at `dcefc8a`, with post-merge main CI run `28960992598` passing. PR #4 (`actions/checkout@v7`) and PR #7 (`rusqlite@0.40.1`) were closed/superseded after the policy landed. PR #29 merged the SHA-256 compatibility fix at `1ba842c`, with post-merge main CI run `28961502684` passing. PR #28 (`sha2@0.11.0`) was updated against the fixed base and is now green. All open PRs were green by check rollup at closeout: #28, #13, #8, and #5. GitHub branch-protection API returned `403` while the repo is private on the current account tier, so enforcement could not be API-verified until the repo is public or upgraded.

## 2026-07-08 Compact Workstation TUI Mock Alignment

### Task
- Objective: Make the primary terminal renderer closely match the operator-provided Hyperliquid Spot Microstructure Workstation mock for the live/screen command surface.
- Owner repo(s): standalone `hlscreen/` repository only.
- Capital impact: research-only / read-only public market-data visualization. No market-data semantics change, private stream, wallet, order, execution, or risk-control surface.

### Context
- Background: The current TUI is professional but still uses a broad dashboard plus per-row card layout. The operator wants a compact box table with immediate ranked rows and a selected-pair detail pane similar to the provided mock.
- Inputs: `crates/hls-tui/src/app.rs`, `crates/hls-tui/tests/main_table_golden.rs`, `crates/hls-cli/tests/live_mock.rs`, current screen request fields, and existing public fixture data.
- Outputs: Golden-tested compact workstation table, selected-pair details, regenerated screenshot assets if the screenshot output changes, and validation evidence.

### Assumptions
- The renderer should stay deterministic and ANSI-free so screenshots and golden tests remain stable.
- Labels must not claim unavailable metrics as raw measurements. Existing real fields include spread bps, TOB imbalance, signed notional flow 30s, OFI proxy 30s, RV windows, liquidity score, confidence score/reasons, and screen request preset/filter/sort.
- The mock's `flow1m`/sigma and `amihud` styling can be approximated only where backed by existing fields; unsupported values should be labeled as proxies or use the closest existing 30s/score fields.

### Constraints
- Technical: keep the change inside the TUI contract unless a test requires CLI expectation updates; no new dependencies; no ANSI escape codes; no fake streaming/recording claims.
- Operational: do not mutate runtime live WebSocket behavior or CI workflow state in this slice.
- Risk/capital: keep read-only/no-wallet/no-order language and avoid advice wording.

### Options Considered
1. Patch text labels in the existing dashboard.
   - Pros: minimal diff.
   - Cons: still does not match the requested compact workstation shape.
2. Replace the primary render body with a compact framed table and selected-pair detail pane while reusing existing feature fields.
   - Pros: matches the requested command output and keeps all metrics truthfully derived.
   - Cons: requires updating golden tests and screenshot expectations.

### Chosen Approach
- Choice: option 2.
- Why: The requested outcome is a layout/flow change, not just typography. Reusing existing feature snapshots preserves the current data contract.

### Execution Plan
1. Add failing golden assertions for the compact header, filter/mode line, row columns, selected pair detail pane, confidence reason counters, and read-only footer.
2. Refactor `render_screened_table` to pass request context into the renderer and render request-derived filter/mode text.
3. Replace the verbose board/card body with the compact workstation table and one selected-pair detail pane.
4. Keep helper functions small and deterministic; add proxy labels only when needed.
5. Run focused TUI/CLI tests, fmt/clippy/workspace tests, screenshot generation if output assets changed, and diff/read-only scans.

### Test Plan
- Focused: `cargo test -p hls-tui --test main_table_golden`; relevant CLI live/screen tests if strings change.
- Regression: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `git diff --check`.
- Visual: run fixture live command and regenerate/inspect screenshot SVGs if screenshot scripts use the primary renderer.

### Risks and Rollback
- Risks: old tests/docs may depend on `PAIR DETAIL CARDS`; width assumptions can regress screenshots; the mock includes metrics not available as exact fields.
- Rollback: revert the TUI renderer/test changes; no persisted data or external market state is modified.

### Memory Impact
- Add/update in `MEMORY.md`: record the compact workstation renderer contract and any changed screenshot command if durable.

### Final Notes
- What changed: Replaced the broad dashboard/detail-card primary renderer with a compact `Hyperliquid Spot Microstructure Workstation` box table plus selected-pair detail pane. `render_screened_table` now passes screen request context so `filter:` and `mode:` reflect the active preset/custom rule/sort. CLI/TUI integration tests now assert the compact live, replay, screen, record/replay, confidence, and full-pipeline output. Screenshot styling and SVG assets were regenerated for the new box table.
- Validation run: Red tests first: `cargo test -p hls-tui --test main_table_golden` and `cargo test -p hls-cli --test live_mock` failed on the old renderer. Final green gates: `cargo test -p hls-tui --test main_table_golden`; `cargo test -p hls-cli --test live_mock`; `python3 scripts/generate-screenshots.py`; `rsvg-convert docs/assets/screenshots/live-screen.svg -o /tmp/hlscreen-preview/live-screen.png`; `rsvg-convert docs/assets/screenshots/resilience-screen.svg -o /tmp/hlscreen-preview/resilience-screen.png`; direct fixture TUI smoke with zero stderr; `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `scripts/check-release-packaging.sh`; `git diff --check`; read-only/private-surface scan reviewed with only expected docs/tests/fixtures/read-only caveats.
- Follow-ups: The compact mock uses current feature support: `flow30` is real 30s signed notional flow, and `amihud` is a liquidity-cost proxy from spread/depth/liquidity score. Add real 1m signed-flow sigma or true Amihud only as a separate feature-engine slice with tests before renaming those columns.

## 2026-07-08 All-Data Live Smoke And End-to-End Audit

### Task
- Objective: Run a fresh all-available-public-data smoke test, capture TUI screenshot evidence, audit the implementation against official Hyperliquid docs and project standards, fix any blocking findings, and merge only if local and GitHub gates are green.
- Owner repo(s): standalone `hlscreen/` repository only.
- Capital impact: research-only / read-only public market-data ingestion and visualization. No wallet, private stream, signing, order placement, live trading, or external capital action.

### Context
- Background: `main` already contains the compact workstation TUI and previous live/CI hardening. The operator requested another full smoke/audit pass with current live evidence and screenshots before PR/merge.
- Inputs: current `main` at `ab01664`, official Hyperliquid WebSocket/API docs, `specs/002-microstructure-workstation/plan.md`, CLI/TUI/store/feature/source modules, existing CI and release-packaging gates.
- Outputs: dated audit report with live-run artifacts, screenshot PNG/SVG paths, validation matrix, code-review notes, any focused fixes, PR, and post-merge CI evidence if stable.

### Assumptions
- A bounded current all-symbol live run is sufficient for this pass; prior 15-minute runs remain historical evidence but are not a substitute for current smoke proof.
- The live run must use public Hyperliquid data only and must not introduce mock/fallback paths into production evidence.
- The PR can be merged only after local validation and GitHub checks pass; if any blocker remains, the branch stays unmerged with an explicit blocker report.

### Constraints
- Technical: keep the change audit/report-focused unless a real bug is found; no fake metrics, no mock data in live proof, no unbounded background services left running.
- Operational: use `feat/andrzej_all_data_e2e_audit`; do not touch parent `rsibot` or unrelated repos; keep artifact paths explicit.
- Risk/capital: read-only public REST/WebSocket only; no credentials, account endpoints, private streams, order APIs, or trading recommendations.

### Options Considered
1. Rely on the previous all-pairs 15-minute evidence.
   - Pros: fastest and already validated.
   - Cons: does not satisfy the request for current smoke evidence and screenshots.
2. Run a fresh bounded all-symbol smoke, replay/screen it, run validation, and document the results.
   - Pros: current evidence, catches drift in live payloads, keeps the report auditable.
   - Cons: takes longer and can be affected by temporary public network conditions.

### Chosen Approach
- Choice: option 2.
- Why: The request is specifically about production readiness, live data, and no mock/workaround behavior. Fresh bounded live proof is the only defensible evidence.

### Execution Plan
1. Confirm branch/base status and official-doc references.
2. Run a bounded `hls live --all-symbols` public WebSocket smoke with raw and normalized recording.
3. Replay and screen the recorded run; inspect logs, registry, file counts, and generated TUI output.
4. Generate PNG/SVG screenshot artifacts from the current TUI output.
5. Review source modules for WebSocket/REST contracts, recording/replay, feature semantics, TUI truthfulness, read-only boundaries, dead code, and edge cases.
6. Run full validation gates and negative-input probes.
7. Write a dated audit report, update memory/reflection/TODO, commit, push, open PR, wait for checks, and merge only if stable.

### Test Plan
- Live smoke: `./target/debug/hls live --all-symbols --duration-secs <bounded> --refresh-secs 30 --tui --record --raw --normalized --run-id <id> --data-dir <tmp>`.
- Replay/screen/API: `hls replay`, `hls screen`, `hls doctor --live --json`, `hls server --print-health` against the captured data where applicable.
- Negative probes: invalid DSL, unknown preset, missing fixture/live inputs, and read-only/private-surface scan.
- Regression: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo build --release --workspace --all-features`; `scripts/check-release-packaging.sh`; `python3 scripts/generate-screenshots.py`; `git diff --check`.

### Risks and Rollback
- Risks: public market quiet periods can yield sparse trade-derived fields; temporary WebSocket/network issues can fail the smoke without proving a code regression; a short smoke cannot prove days-long service stability.
- Rollback: revert only this audit/report branch. No external exchange/account/release state is mutated by the planned work.

### Memory Impact
- Add/update in `MEMORY.md`: record current all-symbol smoke command, artifact counts, screenshot path, and any durable production-readiness caveats discovered.

### Final Notes
- What changed: Ran a fresh all-symbol public Hyperliquid smoke, replayed and screened the captured run, generated a real-data TUI PNG, audited the source against official REST/WebSocket/rate-limit/heartbeat docs, fixed the misleading TUI confidence counter label from `gap` to `window`, regenerated committed screenshots, and wrote `docs/reports/2026-07-08-all-data-e2e-audit.md`.
- Validation run: 180s all-symbol live capture `allpairs-e2e-20260708-195413` with 308 symbols, 924 subscriptions, 59,384 WS messages, 67,192 normalized events, clean shutdown, 0 reconnects, and 0 data gaps; replay parity baseline then pass; `thin_books` and `flow_pressure` screen commands over the captured run; `doctor --live --json`; `server --print-health`; negative probes for invalid DSL, unknown preset, missing fixture, and unsupported Parquet; post-fix 60s all-symbol live capture `allpairs-postfix-20260708-195842` with 18,470 WS messages, 26,156 normalized events, clean shutdown, 0 reconnects, and 0 data gaps; `cargo test -p hls-tui --test main_table_golden --test confidence_pane`; `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo build --release --workspace --all-features`; `scripts/check-release-packaging.sh`; `python3 scripts/generate-screenshots.py`; `git diff --check`; read-only/private-surface and TODO/debug scans.
- Follow-ups: Automatic public REST backfill after reconnect, true Parquet output, long-running HTTP server mode, keyboard-driven interactive TUI, and multi-day soak testing remain separate future work. Current live proof is public read-only data only and uses top-of-book proxies honestly.

## 2026-07-08 Production Docs And Live Readiness Refresh

### Task
- Objective: Refresh `hlscreen` toward truthful production/open-source readiness by validating all currently available public spot data, updating docs to match the latest implementation, adding architecture diagrams, and capturing current TUI screenshot evidence.
- Owner repo(s): standalone `hlscreen/` repository only.
- Capital impact: research-only / read-only public market-data tooling. No private streams, wallet access, signing, order placement, trading execution, or capital-changing action.

### Context
- Background: `main` already has compact workstation TUI, all-symbol live proof, recorder/replay, confidence, metadata, benchmark, metrics, release draft, and open-source docs. The operator wants the whole codebase/docs to read as production-ready according to truth, with current all-pair live validation and architecture diagrams.
- Inputs: current `main` at `45b9e7c`, official Hyperliquid public API docs, `specs/002-microstructure-workstation/plan.md`, README/docs, TUI screenshots, live all-symbol commands, CI/release gates.
- Outputs: updated README/docs, diagrammed architecture doc, production-readiness doc/report, current all-symbol smoke evidence, TUI screenshot artifact(s), validation matrix, and any focused fixes found during audit.

### Assumptions
- "Production ready" means deployable for read-only public-data recording/screening with explicit caveats, not a capital-touching trading system.
- A bounded all-symbol smoke is acceptable current proof; multi-day soak, external hosting, and release-tag publishing remain separate gates unless explicitly requested.
- Mermaid diagrams in Markdown are the right architecture-diagram format for open-source docs because they live cleanly in git and render on GitHub.

### Constraints
- Technical: no mock data in live proof; deterministic fixtures remain only for tests/docs screenshots; no fake metrics or unsupported precision in TUI copy.
- Operational: keep changes scoped to `hlscreen/`; leave no background services running; do not mutate GitHub release state or external systems beyond public read-only API calls.
- Risk/capital: no account addresses, credentials, private endpoints, order APIs, recommendations, or profitability claims.

### Options Considered
1. Only update docs from previous audit evidence.
   - Pros: fast and low risk.
   - Cons: does not validate current live data or catch drift.
2. Run fresh all-symbol validation, then update docs/architecture around the new evidence.
   - Pros: current truth, better open-source credibility, catches runtime/doc drift.
   - Cons: takes longer and depends on public network stability.

### Chosen Approach
- Choice: option 2.
- Why: The request is centered on current live-data production readiness. Docs should be updated from fresh evidence, not just memory.

### Execution Plan
1. Refresh official-doc alignment for public REST, WebSocket subscriptions, heartbeat, and rate limits.
2. Run bounded all-symbol public live capture with raw and normalized recording.
3. Inspect SQLite/file counts, replay parity, screen presets, health output, and TUI screenshot output from the captured run.
4. Audit code/docs for production-readiness gaps, dead language, stale links, and untruthful readiness claims.
5. Update README/docs with current readiness labels, deployment/runbook guidance, architecture Mermaid diagrams, and the latest validation report.
6. Run focused docs/link checks, screenshot generation, full Rust gates, release packaging, diff/read-only scans, and summarize remaining blockers honestly.

### Test Plan
- Live: `./target/debug/hls live --all-symbols --duration-secs <bounded> --refresh-secs 30 --tui --record --raw --normalized --run-id <id> --data-dir <tmp>`.
- Replay/screen: `hls replay --verify-parity`, `hls screen --preset thin_books`, `hls screen --preset flow_pressure`, and registry/file-count checks over the captured run.
- Health/docs: `hls doctor --live --json`, `hls server --print-health`, screenshot generation, Markdown/link checks.
- Regression: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo build --release --workspace --all-features`; `scripts/check-release-packaging.sh`; `git diff --check`.

### Risks and Rollback
- Risks: live public data can be temporarily quiet; a short run cannot prove multi-day uptime; diagram/doc changes can overstate readiness if not tied to evidence.
- Rollback: revert this branch's docs/report/fix changes. No external market/account/release state is modified.

### Memory Impact
- Add/update in `MEMORY.md`: current production-readiness command set, latest all-symbol smoke evidence, and durable docs/diagram caveats.

### Final Notes
- What changed: Ran fresh all-symbol public Hyperliquid validation, replayed and screened the capture, checked health and negative paths, generated a real-data TUI screenshot, and refreshed README/docs around the current read-only production boundary. Replaced the stale architecture prose with Mermaid diagrams for system boundaries, crate ownership, live/replay flows, command surfaces, and deploy-readiness gates. Fixed one audit finding in `hls-tui`: row quality now reports `partial` when any visible row is missing spread or top-of-book depth evidence, and the header says `p95 row age` instead of the misleading `p95 local`.
- Validation run: Primary 300s all-symbol live capture `allpairs-prodreadiness-20260708-201752` completed with 308 symbols, 924 subscriptions, 99,162 raw WebSocket messages, 106,980 normalized events, 5 raw files, 1 normalized file, clean shutdown, 0 reconnects, and 0 data gaps. Replay parity wrote a baseline then passed with 0 confidence drift/missing/extra rows. `thin_books` and `flow_pressure` screened the captured run with clean stderr; `doctor --live --json` and `server --print-health` reported healthy read-only state; invalid DSL, unknown preset, missing fixture, and unsupported Parquet probes failed closed. Post-fix 60s all-symbol live capture `allpairs-prodreadiness-postfix-20260708-202420` completed with 18,791 WebSocket messages, 26,455 normalized events, clean shutdown, 0 reconnects, 0 data gaps, `p95 row age`, and `quality partial` on sparse visible coverage. Full gate passed: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo build --release --workspace --all-features`; `scripts/check-release-packaging.sh`; `python3 scripts/generate-screenshots.py`; Markdown local-link check; `rsvg-convert docs/assets/screenshots/live-screen.svg -o /tmp/hlscreen-prodreadiness-preview-live.png`; `git diff --check`; read-only/private-surface and TODO/debug scans.
- Follow-ups: Multi-day soak, deploy-host supervision, long-running HTTP server mode, automatic public REST backfill after reconnect, true Parquet output, keyboard-driven interactive TUI, and tagged release artifact publication remain future gates. Current readiness is a local deployable, public-data, read-only release candidate; it is not trading execution software.

## 2026-07-08 Live Spot Symbol Display Mapping Fix

### Task
- Objective: Validate and fix Hyperliquid spot symbol display so user-facing surfaces show readable pairs such as `HYPE/USDC` while internal subscriptions still use official feed IDs such as `@107`.
- Owner repo(s): standalone `hlscreen/` repository only.
- Capital impact: research-only / read-only public metadata and WebSocket symbol mapping. No private streams, wallet access, order placement, execution, or capital-changing action.

### Context
- Background: The operator noticed live TUI/screens were showing symbols such as `@107`, which are official Hyperliquid spot feed identifiers but not the intended human-facing market labels.
- Inputs: official Hyperliquid Info endpoint docs, live `spotMeta` response, existing symbol parser/tests/fixtures, CLI `symbols`, live universe selection, TUI snapshots.
- Outputs: parser regression test, fixed live metadata display mapping, docs/README command examples updated where user-facing commands should prefer display pairs, validation evidence.

### Assumptions
- Hyperliquid `spotMeta.universe[].name` is the feed `coin` identifier for most spot markets (`@{index}`), while readable display pairs can be derived from `universe[].tokens` and `tokens[].name`.
- `PURR/USDC` remains both a feed identifier and display pair per official docs; other spot markets keep `@{index}` internally.

### Constraints
- Technical: preserve selector matching by both display name and feed ID; do not break replay files or recorded normalized events that store `@{index}` feed IDs.
- Operational: no broad ingestion/TUI refactor; keep change at REST metadata boundary plus docs/tests.
- Risk/capital: read-only public API validation only.

### Options Considered
1. Keep displaying `@{index}` everywhere and document it.
   - Pros: matches official subscription `coin` exactly.
   - Cons: poor user experience and violates the existing spec requirement to preserve display names separately from feed identifiers.
2. Derive display names from token indexes and keep `@{index}` as `hl_coin`.
   - Pros: matches user expectations, preserves official feed IDs, fixes the bug at the metadata boundary.
   - Cons: relies on token-name availability in `spotMeta`, which is part of the public response.

### Chosen Approach
- Choice: option 2.
- Why: It keeps the transport contract correct while making CLI/TUI output human-readable.

### Execution Plan
1. Reproduce with live `spotMeta`: confirm HYPE has `universe.name="@107"` and tokens `[150, 0]` deriving `HYPE/USDC`.
2. Update fixtures to use the live-shaped `@107`/`@108` universe names and run parser tests red.
3. Update parser to derive `display_name` from token names while keeping `hl_coin` from `feed_id_for_spot`.
4. Run focused parser/CLI/TUI validation and live `hls symbols` checks for `HYPE/USDC` and `@107`.
5. Update docs/memory if durable command examples or symbol semantics changed.

### Test Plan
- Focused: `cargo test -p hls-hyperliquid --test rest_metadata`; `cargo test -p hls-core --test config_symbol`.
- CLI smoke: `./target/debug/hls symbols --include HYPE/USDC --top 1`; `./target/debug/hls symbols --include @107 --top 1`; JSON output checks.
- Regression: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `git diff --check`.

### Risks and Rollback
- Risks: recorded/replay feature snapshots still carry feed IDs unless metadata/display mapping is explicitly attached later; this fix makes symbol inspection and live selection correct but may require a follow-up to propagate display names into every TUI row.
- Rollback: revert parser/fixture/docs changes. No exchange/account state is modified.

### Memory Impact
- Add/update in `MEMORY.md`: live Hyperliquid symbol semantics and confirmed commands for display-name/feed-ID selection.

### Final Notes
- What changed: Confirmed against official docs and live `spotMeta` that non-PURR spot markets often expose `spotMeta.universe[].name` as the transport feed ID (`@107`) while the readable pair is derived from token indexes (`HYPE/USDC`). Fixed `parse_spot_meta` to derive display names from `tokens[].name`; fixed `spotMetaAndAssetCtxs` parsing to join asset contexts by their explicit `coin` field instead of array position; resolved explicit live selectors such as `HYPE/USDC`, `hype-usdc`, and `@107` to the correct feed ID; updated TUI rendering to prefer metadata display names; refreshed active docs and screenshots.
- Validation run: The red regression was `cargo test -p hls-hyperliquid --test rest_metadata` after making fixtures live-shaped with `@107`/`@108` names. Final focused checks passed: `cargo test -p hls-core --test config_symbol`; `cargo test -p hls-hyperliquid --test rest_metadata`; `cargo test -p hls-cli commands::live::tests::explicit_live_symbol -- --nocapture`; `cargo test -p hls-tui --test main_table_golden`. Live public checks passed: `hls symbols --include HYPE/USDC --top 1`, `hls symbols --include @107 --top 1`, `hls symbols --include hype-usdc --top 1`, and `hls symbols --include ueth-usdc --top 1`; `ETH/USDC` correctly failed as unknown because current Hyperliquid public metadata uses `UETH/USDC`. A 5s live WebSocket TUI run with `--symbols hype-usdc` completed with 1 symbol, 4 subscriptions, 35 WS messages, 63 market events, 0 reconnects, 0 data gaps, and rendered `HYPE/USDC`. Full gates passed: `cargo fmt --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo build --release --workspace --all-features`; `scripts/check-release-packaging.sh`; `python3 scripts/generate-screenshots.py`; Markdown link check; `git diff --check`.
- Follow-ups: Replay/fixture rows without metadata still display feed IDs by design because raw WebSocket events only carry `coin`; if replay TUI should always show display names, persist and reattach symbol metadata from the registry during replay.
