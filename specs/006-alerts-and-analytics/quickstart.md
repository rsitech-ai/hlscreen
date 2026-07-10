# Quickstart: Alerts And Analytics

## Focused Validation

```bash
cargo test -p hls-core --test alerts_contract --test metrics_contract --test extension_contract
cargo test -p hls-features --test canonical_metrics --test fee_tradeability
cargo test -p hls-store --test analog_search
cargo test -p hls-cli --test alerts_command --test analog_command --test extension_command --test live_mock --test screen_command
```

## Public Toxicity Proxy Smoke

`signed_flow_toxicity_proxy_30s` is a bounded public trade-flow concentration
proxy. It must emit `support=proxy` when public trade evidence is sufficient and
`support=unavailable` for sparse windows. It must not be documented as canonical
toxicity, private fill quality, or account execution evidence.

```bash
cargo test -p hls-features --test canonical_metrics signed_flow_toxicity_proxy_uses_public_trade_imbalance
cargo test -p hls-features --test canonical_metrics signed_flow_toxicity_proxy_is_unavailable_for_sparse_public_trades
```

## Local Fee Profile Smoke

Explicit local fee profiles must validate integer maker/taker fees and an
optional `taker_fill_ratio_hundredths` field. The fill ratio blends maker and
taker rates for screening economics only; it must not imply private account
fee-tier lookup, realized fill modeling, routing, or execution feasibility.

```bash
cargo test -p hls-core --test fees_contract
cargo test -p hls-features --test fee_tradeability blended_fee_profile_uses_maker_taker_fill_mix
cargo test -p hls-cli --test screen_command screen_applies_blended_maker_taker_fee_profile_file
```

## Local Analog Index Smoke

`hls analog --write-index <path>` must build a schema-versioned local JSON
index from normalized replay windows, and `hls analog --index-file <path>` must
reuse that index without rescanning the run. This remains local read-only
research context, not prediction, execution simulation, or advice.

```bash
cargo test -p hls-store --test analog_search analog_index_round_trips_replay_candidates
cargo test -p hls-cli --test analog_command analog_command_writes_and_reuses_local_index
```

## Live Plugin Status

Live plugin loading is not implemented. Validate only the standalone bounded
`hls extension` path; do not pass extension flags to `hls live`.

## Local Alert History Smoke

`--alert-history-file <path>` must append emitted and suppressed alert evidence
as local JSONL only, and `hls alerts --history-file <path>` must list recent
records without requiring replay/live inputs. These commands must not enable
external delivery, daemon scheduling, wallet/private data, or exchange actions.
The committed regression coverage is:

```bash
cargo test -p hls-cli --test alerts_command alerts_command_writes_local_history_jsonl
cargo test -p hls-cli --test alerts_command alerts_command_lists_local_history_jsonl
```

## Safety Proof

Every alert/plugin command must prove:

- no wallet/private/order/execution surface
- deterministic replay output
- confidence and unavailable-evidence rendering
- bounded plugin time/output if runtime execution is enabled
