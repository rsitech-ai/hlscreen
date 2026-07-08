# Contract: Metrics and Observability

## Purpose

Expose enough live/replay telemetry to diagnose pipeline lag, data gaps, and confidence degradation without creating high-cardinality metric load.

## Required Metrics

| Metric | Type | Labels | Notes |
|---|---|---|---|
| `hls_ws_messages_total` | counter | `channel` | Public WebSocket frames by channel |
| `hls_ws_disconnects_total` | counter | `reason` | Server close, timeout, parse fatal, operator stop |
| `hls_parse_latency_us` | histogram | `channel` | JSON/envelope parse and normalization |
| `hls_feature_latency_us` | histogram | none | Feature update latency after normalization |
| `hls_pipeline_lag_ms` | histogram | `stage` | Receive to feature to render stage lag |
| `hls_recorder_queue_depth` | gauge | `queue` | Raw/normalized writer backlog |
| `hls_data_gaps_total` | counter | `reason` | Reconnect or source gap events |
| `hls_replay_speed_ratio` | gauge | none | Replay wall-clock ratio |
| `hls_symbols_tracked` | gauge | none | Current tracked symbol count |
| `hls_confidence_low_symbols` | gauge | none | Count of degraded/invalid confidence rows |

## Label Rules

Allowed labels:
- bounded enum labels such as `channel`, `stage`, `reason`, `queue`

Disallowed labels on broad metrics:
- `symbol`
- `run_id`
- filesystem path
- connection id
- user/account address
- arbitrary error strings

Symbol-level diagnostics should be shown in:
- TUI detail panes
- JSON health/debug outputs
- top-N degraded symbol summaries
- structured logs with bounded sampling

## Timestamp Policy

Every event path must preserve:
- `exchange_ts_ms` when provided by Hyperliquid
- `recv_ts_ns` stamped before parsing where practical
- `feature_ts_ns` when a feature snapshot is updated
- `render_ts_ns` for display/output snapshots when applicable

Derived telemetry:
- `wire_lag_ms = recv_ts_ms - exchange_ts_ms`
- `feature_lag_us = feature_ts_ns - recv_ts_ns`
- `render_age_ms = render_ts_ns - feature_ts_ns`

## CLI/JSON Outputs

`hls doctor --live --json` should eventually include:

```json
{
  "metrics": {
    "symbols_tracked": 150,
    "confidence_low_symbols": 3,
    "parse_latency_us_p95": 420,
    "feature_latency_us_p95": 650,
    "recorder_queue_depth_max": 12
  }
}
```

## Validation

- Unit tests must reject accidental high-cardinality labels in metric definitions.
- Benchmark fixtures must emit metrics snapshots suitable for regression checks.
- Live smoke reports must include message counts, gaps, reconnects, and at least one local pipeline latency summary once implemented.
