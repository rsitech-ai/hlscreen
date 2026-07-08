# Quickstart: Hyperliquid Microstructure Workstation

This quickstart defines the validation path for the future microstructure workstation implementation. Commands that do not exist yet are marked as target commands and are converted into tasks in `tasks.md`.

## Prerequisites

- Rust 1.88 or newer.
- Current `hlscreen` checkout.
- Public network access only for live smoke commands.
- No private keys, wallet, or Hyperliquid account credentials.

## Baseline v1 Gate

Run before starting the feature:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
```

Expected:
- All commands pass.
- Existing read-only live/replay/screen behavior remains compatible.

## Story 1: Confidence and Replay Parity

Target fixture validation:

```bash
cargo test -p hls-core --test confidence_state
cargo test -p hls-store --test replay_parity
./target/debug/hls replay \
  --data-dir tests/fixtures/microstructure/gap_replay \
  --run-id gap-replay \
  --verify-parity \
  --show-score-breakdown
```

Expected:
- Gap fixtures produce degraded confidence.
- Duplicate trades are not double-counted.
- Replay parity failures return non-zero with mismatched fields.

## Story 2: Liquidity Resilience

Target fixture validation:

```bash
cargo test -p hls-features --test resilience
./target/debug/hls screen \
  --fixture-file tests/fixtures/hyperliquid/resilience_shock.ndjson \
  --preset liquidity_resilience \
  --sort spread_recovery_ms:asc
```

Expected:
- Spread shocks and recoveries are classified deterministically.
- Thin/stale books do not appear as fully tradeable.
- Metrics are labeled as BBO/top-of-book proxies where applicable.

## Story 3: Why-Ranked Explanations

Target fixture validation:

```bash
cargo test -p hls-core --test score_breakdown
cargo test -p hls-tui --test why_ranked_pane
./target/debug/hls explain \
  --data-dir tests/fixtures/microstructure/explainable_replay \
  --run-id explainable \
  --symbol @107 \
  --json
```

Expected:
- Output includes named score components.
- Confidence penalties and unavailable evidence are visible.
- Replay and live-recorded explanations match within documented tolerances.

## Story 4: Hyperliquid Metadata Enrichment

Target fixture validation:

```bash
cargo test -p hls-hyperliquid --test metadata_enrichment
cargo test -p hls-screen --test metadata_presets
./target/debug/hls screen \
  --fixture-file tests/fixtures/hyperliquid/metadata_enriched.ndjson \
  --preset new_listings
```

Expected:
- Public metadata fields are attached where available.
- Missing metadata is shown as unknown, not as a pipeline failure.
- Metadata polling respects REST budget tests.

## Story 5: OSS Operations, Metrics, Packaging, and Extension Contract

Target validation:

```bash
cargo test -p hls-core --test metrics_contract
cargo test -p hls-cli --test bench_command
cargo test -p hls-core --test extension_contract
./target/debug/hls bench --pack tests/fixtures/microstructure/canonical_pack --json
./target/debug/hls doctor --live --json
```

Expected:
- Metrics definitions reject high-cardinality labels.
- Benchmark pack validation emits replay parity and latency summaries.
- Extension contract rejects network/filesystem/private/execution capabilities.

Packaging target validation:

```bash
cargo build --release --workspace --all-features
# target once configured:
# dist plan
```

Expected:
- Release packaging can build signed-off artifacts without requiring secrets in the repository.

## Short Public Live Smoke

After confidence and resilience fields exist:

```bash
./target/debug/hls live \
  --symbols @107 \
  --duration-secs 30 \
  --refresh-secs 5 \
  --tui \
  --show-confidence \
  --show-resilience
```

Expected:
- Command uses public WebSocket data only.
- It exits cleanly.
- It reports WebSocket message count, market event count, reconnect count, data gap count, and confidence summary.

## Definition of Done for This Feature

- All story-specific tests pass.
- Full workspace fmt/clippy/tests/build pass.
- Benchmark fixture packs validate parity and confidence behavior.
- Docs explain metric formulas, caveats, and confidence states.
- No private streams, wallet, signing, order, or execution surface exists.
- Quickstart path gets a first-time user to live screen and health verification in under five minutes on a supported machine.
