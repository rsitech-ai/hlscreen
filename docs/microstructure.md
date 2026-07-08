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

The contract is intentionally separate from the feature engine. The engine and
replay code will attach and compute confidence in the US1 implementation slice,
while this foundation slice defines the serializable shape and deterministic
penalty semantics.

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
