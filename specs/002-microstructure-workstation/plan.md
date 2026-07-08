# Implementation Plan: Hyperliquid Microstructure Workstation

**Branch**: `002-microstructure-workstation` | **Date**: 2026-07-08 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/002-microstructure-workstation/spec.md`

## Summary

Evolve `hlscreen` from a read-only live screener/recorder into a venue-native Hyperliquid spot microstructure workstation. The implementation builds on the existing v1 public WebSocket, recorder, replay, feature, screen, TUI, CLI, and health crates. The first priority is data trust: confidence-aware live/replay semantics, gap-aware feature validity, replay parity, and explainable score components. The second priority is BBO-plus-trade microstructure analytics: liquidity resilience, tradeability, and top-of-book adverse-selection proxies. Later slices add Hyperliquid metadata enrichment, low-cardinality observability, release packaging, benchmark packs, and a read-only extension contract.

## Technical Context

**Language/Version**: Rust stable, edition 2024, `rust-version = "1.88"`.

**Primary Dependencies**: Existing workspace crates (`hls-core`, `hls-hyperliquid`, `hls-store`, `hls-features`, `hls-screen`, `hls-tui`, `hls-cli`, `hls-server`); existing `tokio`, `tokio-tungstenite`, `reqwest`, `serde`, `serde_json`, `rusqlite`, `zstd`, `clap`. Candidate future dependencies are `ratatui`/`crossterm` for full keyboard TUI, `parquet`/Arrow or Polars-compatible output for analytical storage, OpenTelemetry/tracing metrics, cargo-dist for releases, and an Extism/WASM-like runtime only after the extension contract is proven.

**Storage**: Existing local raw `.ndjson.zst`, normalized JSONL, and SQLite registry remain authoritative. This feature adds benchmark fixture packs, replay parity manifests, confidence snapshots, score breakdown snapshots, metadata cache records, and eventually Parquet-compatible analytics output.

**Testing**: `cargo test --workspace --all-features`, crate-specific golden/fixture tests, replay parity tests, metrics contract tests, CLI/TUI smoke tests, packaging dry-run checks, and short bounded public WebSocket smoke tests where external network proof is required.

**Target Platform**: Local macOS and Linux terminals first; GitHub Actions on Linux; future GitHub Release binaries for macOS/Linux.

**Project Type**: Rust Cargo workspace CLI/TUI application with local storage, read-only market-data adapters, optional localhost read-only API helpers, and future extension contracts.

**Performance Goals**: Maintain WebSocket read-loop independence from disk/TUI work; expose local hot-path timing from receive timestamp to feature update; use benchmark fixtures to enforce a documented p95 feature-update budget before optimizing further. The design target is sub-millisecond p95 local feature update for normal fixture loads, but the first implementation must measure before hard-enforcing broad production SLAs.

**Constraints**: Public market data only; no wallets, private streams, signing, order placement, execution routes, or profitability claims. Respect Hyperliquid WebSocket connection/subscription/message limits. Treat BBO-only metrics as top-of-book proxies, not full order-book analytics. Avoid high-cardinality labels in metrics. Preserve v1 CLI/replay compatibility unless a migration is documented and tested.

**Scale/Scope**: Support the current all-symbol public spot mode under subscription headroom, top-50/top-150 user workflows, and deterministic benchmark packs. Full node-backed ingestion, L2/L4 order-book reconstruction, cloud collaboration, and executable trading are out of scope for this feature.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

The local `.specify/memory/constitution.md` remains a template, so controlling gates come from repo/HQ operating rules and the v1 Spec Kit package:

- **Read-only capital boundary**: PASS. This feature explicitly excludes private streams, signing, wallets, orders, execution, and profitability claims.
- **Replayability and evidence**: PASS. Confidence, score explanations, and benchmark fixtures are designed around local replay evidence.
- **Operator truthfulness**: PASS. Low-confidence, gap, sparse-data, and unavailable-evidence states are first-class outputs.
- **Testing required**: PASS. Each user story has independent fixture/golden/contract tests before implementation tasks.
- **Observability by default**: PASS. Metrics and latency separation are explicit, with low-cardinality constraints.
- **Dependency hygiene**: PASS with staged adoption. New dependencies are deferred to tasks where their value is proven by a focused slice.

## Project Structure

### Documentation (this feature)

```text
specs/002-microstructure-workstation/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── cli-tui.md
│   ├── confidence-and-scoring.md
│   ├── metrics.md
│   └── extension.md
└── tasks.md
```

### Source Code (repository root)

```text
crates/
├── hls-core/
│   └── src/
│       ├── confidence.rs          # new confidence contracts
│       ├── score.rs               # new score breakdown contracts
│       ├── health.rs              # extend with low-confidence / metrics links
│       └── market_state.rs        # extend feature snapshot linkage
├── hls-hyperliquid/
│   └── src/
│       └── rest.rs                # extend public metadata adapters where documented
├── hls-store/
│   └── src/
│       ├── replay.rs              # replay parity and snapshot loading
│       ├── metadata.rs            # schema version / confidence manifest tracking
│       └── benchmark.rs           # new canonical fixture pack helpers
├── hls-features/
│   └── src/
│       ├── resilience.rs          # new liquidity resilience metrics
│       ├── tradeability.rs        # new cost/depth/flow classification
│       └── engine.rs              # include confidence and score breakdown outputs
├── hls-screen/
│   └── src/
│       ├── row.rs                 # add confidence/resilience/metadata fields
│       └── presets.rs             # add confidence/resilience/listing presets
├── hls-tui/
│   └── src/
│       ├── app.rs                 # render confidence and why-ranked summaries
│       └── detail.rs              # future detail/why-ranked pane
├── hls-cli/
│   └── src/commands/
│       ├── bench.rs               # new benchmark fixture command
│       ├── explain.rs             # optional score explanation command
│       ├── live.rs                # surface confidence/resilience fields
│       └── replay.rs              # replay parity verification flags
└── hls-server/
    └── src/lib.rs                 # optional read-only metrics/screen output helpers

tests/
├── fixtures/hyperliquid/
│   ├── resilience_*.ndjson
│   ├── gap_*.ndjson
│   └── metadata_*.json
└── golden/
    └── microstructure/
```

**Structure Decision**: Extend the existing workspace with narrow modules rather than introducing a new application. Keep pure contracts in `hls-core`, public exchange adapters in `hls-hyperliquid`, local/replay evidence in `hls-store`, feature math in `hls-features`, rule/preset exposure in `hls-screen`, presentation in `hls-tui`, and commands in `hls-cli`.

## Phase 0: Research Summary

See [research.md](research.md). Key decisions:

- Keep the production surface on public WebSocket and Info endpoints.
- Prioritize confidence and replay parity before adding new ranking weights.
- Use BBO-plus-trade metrics as explicitly labeled top-of-book proxies.
- Add observability with low-cardinality metrics and timestamp separation.
- Add cargo-dist/GitHub Releases/Homebrew as packaging work after core validation.
- Define a read-only extension contract before adding any plugin runtime.

## Phase 1: Design Summary

See [data-model.md](data-model.md), [contracts/](contracts/), and [quickstart.md].

Post-design constitution check:

- **Read-only capital boundary**: PASS. Contracts add no wallet, private stream, or execution route.
- **Replayability and confidence**: PASS. Data model includes confidence snapshots, score breakdowns, benchmark fixture packs, and replay manifests.
- **Operator truthfulness**: PASS. CLI/TUI contracts require degraded confidence and unavailable evidence to be visible.
- **Observability**: PASS. Metrics contract explicitly prevents high-cardinality symbol labels on broad histograms.
- **Incremental delivery**: PASS. Tasks are separable by confidence/replay, resilience, explainability, metadata, and OSS operations.

## Complexity Tracking

No constitution violations are intentionally introduced.
