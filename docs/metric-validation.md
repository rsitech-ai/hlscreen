# Metric Validation

`hlscreen` separates directly observed window statistics from research
estimators. `canonical` means the formula, public inputs, exchange-time sampling
rule, observation floor, unit, benchmark, and numerical tolerance are versioned
and deterministic. It does not mean the public feed is complete through a
reconnect, nor does it imply predictive or execution value.

## Sampling Contract V1

Each canonical metric uses `MetricSamplingContract` with:

- a stable snake-case metric name and positive contract version;
- an exchange-time window and minimum observation count;
- a named sampling mode and unit;
- finite absolute and relative benchmark tolerances.

The benchmark comparison accepts a value when:

```text
abs(actual - expected) <= max(absolute_tolerance,
                              relative_tolerance * abs(expected))
```

Non-finite values, empty contracts, zero observation floors, and contracts with
both tolerances set to zero fail validation.

## Canonical Public-Trade Metrics

Both V1 metrics use public trades whose exchange timestamps are inside the
inclusive `[decision_time - 60s, decision_time]` window. Rows are ordered by
`(exchange_ts_ms, tid)` and require at least three valid observations.

| Metric | Formula | Unit | Sampling | Known bias |
| --- | --- | --- | --- | --- |
| `public_trade_vwap_1m` | `sum(price * size) / sum(size)` | `price` | rolling exchange-event window | public prints observed locally; a reconnect can omit trades |
| `public_trade_return_1m` | `last_price / first_price - 1` | `decimal_return` | rolling exchange-event window | endpoint-sensitive and not a fixed calendar-bar close return |

The deterministic benchmark is
`tests/fixtures/microstructure/canonical_metric_benchmark.json`. It contains the
contract beside the expected result so a fixture cannot silently diverge from
the runtime contract.

Run the focused validation with:

```bash
cargo test -p hls-core --test metrics_contract
cargo test -p hls-features --test canonical_metrics
```

## Research Metrics

`amihud_1m`, `roll_effective_spread`, `bipower_variation_5m`,
`bbo_ofi_proxy_30s`, `signed_flow_toxicity_proxy_30s`, and
`adverse_selection_toxicity_proxy` remain `proxy` or `unavailable`. Their
current event sampling, top-of-book visibility, or absence of private fill
evidence does not justify a canonical production claim. A future promotion
requires a separate contract, benchmark corpus, sparse/gap behavior, and
external reference comparison; changing only the support label is forbidden.

## Public Data Provenance

The runtime consumes the public `trades` and `bbo` WebSocket subscriptions
documented by Hyperliquid. The venue documents subscription payloads and
reconnect behavior, but does not promise that a reconnect replays missed public
ticks. Canonical metric output is therefore still subject to confidence and gap
state from the local recording.

- [Hyperliquid WebSocket subscriptions](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket/subscriptions)
- [Hyperliquid WebSocket behavior](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket)
