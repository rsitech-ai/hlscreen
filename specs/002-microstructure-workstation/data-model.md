# Data Model: Hyperliquid Microstructure Workstation

## Data Confidence Snapshot

Represents whether a symbol row can be trusted at a specific snapshot time.

Fields:
- `symbol`: feed identifier such as `@107`.
- `display_name`: optional metadata-backed display pair such as `HYPE/USDC`; renderer surfaces should prefer this when available.
- `snapshot_ts_ns`: local snapshot publication timestamp.
- `confidence_score`: bounded numeric confidence from `0.0..1.0`.
- `state`: `trusted`, `degraded`, or `invalid`.
- `reasons`: ordered list of reason codes.
- `affected_windows`: feature windows affected by gaps or sparse data.
- `last_trade_age_ms`: optional age of latest trade.
- `last_bbo_age_ms`: optional age of latest BBO.
- `last_asset_ctx_age_ms`: optional age of latest asset context.
- `gap_count_active`: active reconnect/data-gap count.
- `duplicate_event_count`: duplicate events ignored for the snapshot interval.
- `parser_drop_count`: invalid public messages dropped before feature update.
- `writer_backlog`: recorder backlog observed near snapshot time.

Validation:
- `confidence_score` must be finite and clamped to `0.0..1.0`.
- `invalid` state must include at least one reason.
- Any feature window listed in `affected_windows` must be marked incomplete in the row or score breakdown.
- Future timestamps must be clamped at display time but still counted as clock-skew evidence.

## Liquidity Resilience Snapshot

Represents public BBO/trade-derived liquidity and recovery state for one symbol.

Fields:
- `symbol`
- `snapshot_ts_ns`
- `spread_bps`
- `spread_shock_bps`: optional shock magnitude versus local baseline.
- `spread_recovery_ms`: optional time from shock start to recovery threshold.
- `tob_depth_usd`
- `tob_imbalance`
- `quote_freshness_ms`
- `signed_notional_flow_5s`
- `signed_notional_flow_30s`
- `signed_notional_flow_1m`
- `trade_intensity_30s`
- `bbo_ofi_proxy_30s`
- `adverse_selection_proxy`: `normal`, `watch`, `brittle`, or `unknown`.
- `tradeability_state`: `tradeable`, `costly`, `thin`, `stale`, or `unknown`.

Validation:
- Values derived only from BBO must be labeled top-of-book/BBO proxy.
- `tradeability_state = tradeable` requires sufficient confidence and fresh BBO.
- `spread_recovery_ms` is only valid after a detected shock and recovery.
- Sparse trade windows must output `unknown` rather than zero.

## Score Breakdown

Explains why a symbol ranked where it did.

Fields:
- `symbol`
- `snapshot_ts_ns`
- `total_score`
- `confidence_score`
- `components`: list of named score components.
- `unavailable_evidence`: list of missing inputs.
- `version`: scoring model version.

Component fields:
- `name`
- `raw_value`
- `normalized_value`
- `weight`
- `signed_contribution`
- `direction`: `positive`, `negative`, or `neutral`.
- `evidence_window`

Validation:
- `total_score` must equal the documented aggregation of component contributions after confidence adjustment.
- Every top-ranked row must include at least three components or explicit unavailable evidence.
- Score names must not imply financial advice or execution recommendations.

## Metadata Enrichment

Public Hyperliquid metadata attached to a market row.

Fields:
- `symbol`
- `display_name`
- `feed_identifier`
- `spot_index`
- `base_token_index`
- `quote_token_index`
- `metadata_source`
- `metadata_fetched_at_ms`
- `listing_age_ms`: optional.
- `deployer`: optional public address if available from the chosen public source.
- `deploy_time_ms`: optional.
- `seeded_usdc`: optional.
- `max_supply`: optional.
- `circulating_supply`: optional.
- `cohort_tags`: list such as `new_listing`, `fresh_seed`, `thin_float`, `unknown_metadata`.

Validation:
- Missing optional fields must produce `unknown_metadata` or field-level `None`, not ingestion failure.
- Public metadata fetches must respect REST rate budgets.
- Any source outside official public API docs must be marked experimental in docs and tests.

## Benchmark Fixture Pack

Curated replay bundle used for parity and performance checks.

Fields:
- `pack_id`
- `schema_version`
- `description`
- `source`: `fixture`, `recorded_public_live`, or `synthetic_public_shape`.
- `raw_files`
- `normalized_files`
- `expected_snapshots`
- `expected_score_breakdowns`
- `expected_confidence`
- `expected_metrics`
- `performance_budget`
- `created_at_ms`

Validation:
- Expected files must include hashes.
- Any fixture derived from live public data must contain no private/account data.
- Schema version mismatches must either migrate explicitly or fail with a clear error.

## Operator Metrics Snapshot

Machine-readable low-cardinality metrics emitted by live/replay runs.

Fields:
- `ws_messages_total_by_channel`
- `ws_disconnects_total_by_reason`
- `parse_latency_us`
- `feature_latency_us`
- `pipeline_lag_ms_by_stage`
- `recorder_queue_depth_by_queue`
- `data_gaps_total_by_reason`
- `replay_speed_ratio`
- `symbols_tracked`
- `confidence_low_symbols`

Validation:
- Broad histograms must not use `symbol`, `run_id`, or unbounded path labels.
- Symbol-level diagnostics belong in TUI/log/top-N outputs.
- Metrics names must be stable once documented.

## Read-Only Extension Contract

Versioned schema for future custom features or panels.

Input fields:
- `contract_version`
- `row`
- `confidence`
- `recent_trades`
- `recent_bbo`
- `metadata`
- `capabilities`: always explicit and denied by default.

Output fields:
- `contract_version`
- `panel_lines`
- `feature_fields`
- `warnings`
- `requested_capabilities`

Validation:
- Extensions cannot mutate core state.
- Filesystem, network, credentials, private account data, and order/execution capabilities are denied by default.
- Outputs must be bounded in size and deterministic for a given input fixture.

## State Transitions

### Confidence state

```text
trusted -> degraded: data gap, stale stream, sparse required input, writer lag, parser drops
degraded -> trusted: required windows repopulated and degradation reasons expire
degraded -> invalid: required feature inputs unavailable beyond configured threshold
invalid -> degraded: enough input returns for partial output
invalid -> trusted: all required windows and freshness checks recover
```

### Liquidity resilience state

```text
unknown -> normal: enough fresh BBO and trade observations exist
normal -> shock: spread or depth crosses shock threshold
shock -> recovering: spread/depth starts reverting toward baseline
recovering -> normal: recovery threshold met within configured timeout
shock/recovering -> brittle: recovery timeout exceeded or adverse flow persists
any -> unknown: confidence invalid or BBO stale
```

### Replay parity state

```text
not_checked -> matched: replay output equals expected hash/tolerances
not_checked -> drifted: replay output differs from expected output
drifted -> matched: expected output intentionally regenerated with review
any -> unsupported_schema: fixture schema cannot be loaded
```
