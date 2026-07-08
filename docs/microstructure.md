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

User-story implementations build on these contracts. Confidence computation and
replay parity are implemented for US1. Why-ranked panes, resilience analytics,
metadata enrichment, metrics output, and extension execution are implemented in
later tasks.

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

### Replay Parity

`hls replay --verify-parity` compares the replayed confidence snapshots against
a local SQLite baseline for the same recording run and replay timestamp. The
first verification for a run writes the baseline. Later verifications compare
the recomputed replay confidence against the persisted baseline and fail with a
non-zero exit when confidence drifts, a symbol is missing, or a baseline row no
longer has a replayed row.

Replay parity covers data-quality state, not profitability or trading behavior.
It is designed to detect changes such as:

- recorded reconnect gaps no longer degrading confidence
- sparse trade windows silently becoming trusted
- duplicate-event evidence disappearing from replay
- parser-drop or writer-backlog inputs changing confidence unexpectedly

The replay command prints machine-readable summary lines such as
`replay_parity=passed`, `confidence_drift=0`, and
`confidence_summary=high:1 medium:0 low:0 untrusted:0 min:100 reasons:0` before
the human-readable terminal board.

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
