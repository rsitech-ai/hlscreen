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
- Follow-ups: US1 remains open: WebSocket parser/subscription manager, live market state, feature formulas, TUI table, and `hls live`. US2 recording/replay, US3 rules/DSL, and US4 health/API are not implemented yet.
