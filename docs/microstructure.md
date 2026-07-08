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
replay parity are implemented for US1. Liquidity resilience and tradeability
analytics are implemented for US2. Why-ranked score explanations are
implemented for US3. Public metadata enrichment is implemented for US4. Metrics
output and extension execution are implemented in later tasks.

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
market row, and the per-row detail cards. Confidence is data-quality
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

### Liquidity Resilience

US2 adds BBO/trade-derived fields to `FeatureSnapshot` and the terminal market
board:

- `spread_shock_bps`
- `spread_recovery_ms`
- `resilience_state`
- `tradeability_state`
- `adverse_selection_proxy`
- `signed_notional_flow_30s`
- `bbo_ofi_proxy_30s`

These values are computed from public `bbo` and `trades` events in the same
state path used by live, replay, screen, and screenshots. `bbo_ofi_proxy_30s`
and `adverse_selection_proxy` are explicitly top-of-book/BBO-only proxies. They
do not claim full order-book depth, hidden liquidity, fill quality, or trading
edge.

### Score Breakdowns

`ScoreBreakdown` stores named components and a confidence-adjusted total. The
feature engine generates the current score explanation from public row evidence:

- `liquidity_resilience`: top-of-book depth and resilience context
- `momentum`: available return windows
- `mean_reversion_context`: return-window context for contrarian screens
- `signed_flow`: public trade-side signed notional over the recent window
- `spread_cost`: latest BBO spread cost penalty

Rows expose these values through `score_total`, `score_raw_total`,
`score_confidence_penalty`, and `score_component.<name>` in `hls-screen`. The
same breakdown is rendered by `hls explain` and the TUI why-ranked pane.

Example:

```bash
./target/debug/hls explain \
  --data-dir /tmp/hlscreen-run \
  --run-id allpairs-15m \
  --symbol @107
```

Missing evidence is surfaced as `unavailable_evidence` instead of being silently
imputed. Score breakdowns remain screen heuristics; they are not orders, trade
recommendations, execution simulations, fill-quality estimates, or performance
proof.

### Public Metadata Enrichment

US4 adds optional metadata enrichment from Hyperliquid public `spotMeta`,
`spotMetaAndAssetCtxs`, and `tokenDetails` responses. The enrichment model is
attached to `FeatureSnapshot` rows by adapter code, not by the hot feature
calculation path, so WebSocket ingestion does not depend on token-detail
availability.

Rows can carry:

- display name and feed identifier
- spot pair index and base/quote token indices
- metadata source and fetch timestamp
- listing age when deploy time is available
- deployer, seeded USDC, max supply, and circulating supply when public detail
  fields are available
- cohort tags such as `new_listing`, `fresh_liquidity`, `low_float`, and
  `unknown_metadata`

Missing and partial metadata are intentional states. Unknown fields are recorded
in `unknown_fields`, exposed as the `unknown_metadata` cohort, and rendered in
the terminal detail pane. Missing deployer, supply, or seeded-liquidity fields
must not stop live ingestion, replay, or screen rendering.

Metadata screen fields now include `metadata_state`, `metadata_source`,
`metadata_fetched_at_ms`, `listing_age_ms`, `deployer`, `deploy_time_ms`,
`seeded_usdc`, `max_supply`, `circulating_supply`, and `cohort_tag`. Built-in
presets include `new_listings`, `fresh_liquidity`, and `metadata_unknown`.
These are discovery filters only; they are not listing-quality claims or trade
recommendations.

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
