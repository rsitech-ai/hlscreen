# Microstructure Workstation

`hlscreen` v2 evolves the read-only screener into a local-first
microstructure workstation. The core rule remains unchanged: public market data
only, with no wallet, private stream, signing, order placement, or exchange
action surface.

## Foundation Contracts

The foundation slice defines shared contracts before runtime behavior changes:

- data-confidence snapshots and reason codes
- named score breakdowns
- metrics definitions with low-cardinality label validation
- benchmark fixture manifests

User-story implementations build on these contracts. Confidence computation,
replay parity checks, why-ranked panes, resilience analytics, metadata
enrichment, metrics output, and extension execution are implemented in later
tasks.

### Data Confidence

`DataConfidenceSnapshot` is the row-level data-quality contract. It starts at
`100` and moves through `high`, `medium`, `low`, and `untrusted` based on reason
codes such as reconnect gaps, stale quotes, sparse trades, duplicate events,
parser drops, writer backlog, and incomplete feature windows.

`FeatureSnapshot` rows now carry this confidence snapshot, and
`hls-features::FeatureEngine` computes the default row state from:

- quote freshness
- sparse trade evidence for return/volatility windows
- duplicate trade observations that were deduped before feature calculation
- explicit runtime quality inputs for reconnect gaps, parser drops, and writer
  backlog

The deterministic terminal board renders confidence in the header strip, each
market row, and the selected-symbol detail pane. Confidence is data-quality
evidence only; it is not a risk model, trade signal, or profitability claim.

Persisted confidence baselines and `hls replay --verify-parity` drift detection
are still pending in the US1 replay-parity slice.

### Score Breakdowns

`ScoreBreakdown` stores named components and a confidence-adjusted total. This
keeps ranking explainable and replayable without changing current v1 ranking
behavior yet. The later why-ranked story will render these components in CLI/TUI
surfaces.

### Metrics

Metrics definitions must use `hls_` names and low-cardinality labels. Labels
such as `symbol`, `run_id`, wallet/account identifiers, addresses, transaction
hashes, and trade ids are rejected by the foundation contract because they would
make Prometheus-style metrics expensive and noisy.

### Benchmark Manifests

Benchmark manifests describe small public fixture packs with a schema version,
relative input files, expected output hash, latency budget, and tags. They must
reference relative files under `tests/fixtures/microstructure/` and must not
describe private/account datasets.

## Safety Language

Scores and presets are screen heuristics. They are not predictions, trade
signals, recommendations, or profitability claims. Any future replay or
benchmark result must be labeled as evidence about recorded public data, not as
proof of future performance.
