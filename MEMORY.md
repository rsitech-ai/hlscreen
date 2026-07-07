# MEMORY

## Repo Overview
- Workspace type: new project folder inside the `rsibot/` super-repo.
- Primary languages/frameworks: planned Rust Cargo workspace for read-only Hyperliquid spot market-data ingestion, recording, features, CLI/TUI, and optional local API.
- Repo ownership boundaries: keep `hlscreen/` self-contained; do not mutate `hummingbot/`, `hummingbot-api/`, or `quants-lab/` for this planning slice.

## Commands
- Setup/install: `cargo build --workspace`.
- Format: `cargo fmt --check`.
- Lint: `cargo clippy --workspace --all-targets -- -D warnings`.
- Typecheck: no separate typecheck configured; use `cargo build --workspace` plus clippy/tests.
- Tests: `cargo test --workspace`.
- Fixture smoke: `./target/debug/hls symbols --top 2 --asset-contexts-file tests/fixtures/hyperliquid/spot_meta_and_asset_ctxs.json`.
- Mock-live smoke: `./target/debug/hls live --symbols @107 --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --once`.
- Record/replay smoke: `./target/debug/hls record --symbols @107 --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --raw --normalized --run-id smoke --data-dir /tmp/hlscreen-smoke.<id>` then `./target/debug/hls replay --data-dir /tmp/hlscreen-smoke.<id> --run-id smoke`.
- Screen smoke: `./target/debug/hls screen --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --preset thin_books`; custom filters use `--where '<expr>'` and `--sort field:desc`.
- Local quickstart smoke: `./target/debug/hls init --data-dir /tmp/hlscreen-smoke.<id>` then `./target/debug/hls doctor --data-dir /tmp/hlscreen-smoke.<id>`.

## Architecture Notes
- This project is read-only market-data infrastructure: REST/WS ingestion, local raw capture, normalized events, rolling features, screening DSL, TUI/CLI, and replay.
- No wallet, private-key, trading, order-routing, or execution surface belongs in v1.
- New Rust crates use edition 2024 with `rust-version = "1.85"`.
- Foundation implementation covers config/symbol/time primitives, fixture-backed Hyperliquid REST metadata parsing, and CLI `init`/`doctor`/`symbols`.
- US1 mock-live implementation covers public WebSocket fixture parsing, subscription budgeting, live market state, feature snapshots, stable terminal table rendering, and fixture-backed `hls live --once`; real network connection/reconnect, DSL, and full interactive TUI are still future tasks.
- US2 record/replay implementation covers fixture-backed compressed raw `.ndjson.zst`, deterministic normalized JSONL, local SQLite metadata, bounded raw-writer channel orchestration, replay snapshots, `hls record`, `hls replay`, and `hls live --record`; true Parquet output and live network recording remain future tasks.
- US3 screening implementation covers deterministic DSL parsing/evaluation, built-in presets, filtering/sorting over `FeatureSnapshot`, `hls screen`, and fixture-backed `hls live --preset/--where/--sort`; keyboard-driven interactive TUI filter editing remains future work.

## Conventions
- Use honest top-of-book naming (`TOB depth`, `TOB imbalance`) because v1 excludes `l2Book`.
- Treat exchange candles as display/validation helpers; raw trades and BBO are the feature source of truth.

## Known Pitfalls / Sharp Edges
- Hyperliquid spot symbols require careful mapping between display names and `hl_coin` identifiers such as `@107` and `PURR/USDC`.
- Recorder and TUI work must not block the WebSocket read loop; bounded channels and clean shutdown are first-class design constraints.
- `--parquet` is intentionally rejected in the current US2 CLI; use `--normalized` for replayable JSONL until a real Parquet writer exists.
- Screen presets are row-inspection heuristics only, not signals, recommendations, predictions, or profitability claims.

## Decision Log
- 2026-07-07: Initialize Spec Kit locally in `hlscreen/` for the read-only Hyperliquid spot screener plan. This keeps planning artifacts isolated from existing dirty `rsibot` parent work.
- 2026-07-07: Active feature is `specs/001-hyperliquid-spot-screener/`. Generated artifacts: `spec.md`, `plan.md`, `research.md`, `data-model.md`, `contracts/`, `quickstart.md`, and `tasks.md`. Validation commands that worked: `.specify/scripts/bash/setup-plan.sh --json`, `.specify/scripts/bash/setup-tasks.sh --json`, and `git -C /Users/s1kor/dev/trading/rsibot diff --check -- hlscreen`.
- 2026-07-07: Keep local generated files out of Git with `hlscreen/.gitignore`: `.DS_Store`, `target/`, `.hls/`, `data/`, and `*.log`.
- 2026-07-07: Implemented and validated the foundation slice. Confirmed commands: `cargo build --workspace`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, fixture-backed `hls symbols`, and local `hls init`/`hls doctor`. Hidden CLI fixture flags are for deterministic tests only; default `symbols` uses public read-only REST metadata.
- 2026-07-07: Implemented and validated US1 mock-live slice. Confirmed commands: `cargo test -p hls-hyperliquid --test ws_parser`, `cargo test -p hls-features --test formulas`, `cargo test -p hls-tui --test main_table_golden`, `cargo test -p hls-cli --test live_mock`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo build --workspace`, and fixture-backed `hls live --symbols @107 --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --once`.
- 2026-07-07: Implemented and validated US2 fixture-backed local record/replay slice. Confirmed commands: `cargo test -p hls-store --test raw_writer`, `cargo test -p hls-store --test normalized_writer`, `cargo test -p hls-store --test metadata_registry`, `cargo test -p hls-cli --test record_replay`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo build --workspace`, fixture-backed `hls record --symbols @107 --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --raw --normalized --run-id smoke-record --data-dir <tmp>`, `hls replay --data-dir <tmp> --run-id smoke-record`, and `hls live --symbols @107 --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --record --raw --normalized --run-id smoke-live --data-dir <tmp> --once`.
