# Contract: CLI and TUI Workflows

## Scope

This contract defines user-facing commands and terminal surfaces for the microstructure workstation feature. All commands are read-only.

## Existing Commands Extended

### `hls live`

New optional flags:

```text
--show-confidence
--show-resilience
--show-score-breakdown
--preset <existing-or-new-preset>
--where <screen-rule>
--sort <field:asc|field:desc>
```

Expected behavior:
- Rows include confidence state and score component summary when requested.
- Resilience metrics are available only when BBO/trade windows are sufficient.
- Low-confidence rows must be visibly degraded.
- The command must fail before connecting if requested streams exceed subscription budget.

### `hls replay`

New optional flags:

```text
--verify-parity
--benchmark-pack <pack-id-or-path>
--at <timestamp>
--show-score-breakdown
```

Expected behavior:
- Replay can validate expected confidence, feature, and score-breakdown outputs.
- Parity failures return non-zero and print mismatched fields.
- Unsupported fixture schema returns a clear unsupported-version error.

### `hls screen`

New fields available to `--where` and `--sort`:

```text
confidence_score
confidence_state
spread_shock_bps
spread_recovery_ms
tradeability_state
adverse_selection_proxy
signed_notional_flow_30s
bbo_ofi_proxy_30s
listing_age_ms
seeded_usdc
cohort_tag
score_total
score_component.<name>
```

Expected behavior:
- Unknown fields produce validation errors.
- Missing metadata fields do not match numeric comparisons.
- Presets using new fields must document required data windows.

### `hls doctor --live`

Expected additions:
- Low-confidence symbol count.
- Parse/feature/render lag summary.
- Recorder queue depth.
- Data gap summary.
- Metrics export readiness.

### New `hls bench`

Candidate command:

```text
hls bench --pack <path-or-pack-id> [--json] [--update-expected]
```

Expected behavior:
- Runs replay parity and performance checks over benchmark packs.
- Defaults to read-only verification.
- `--update-expected` is allowed only for fixture maintenance and must be documented as a review action.

### New `hls explain`

Candidate command:

```text
hls explain --data-dir <dir> --run-id <id> --symbol <symbol> [--at <timestamp>] [--json]
```

Expected behavior:
- Prints the score breakdown, confidence reasons, and unavailable evidence for a symbol.
- Does not require live network access when run against recorded data.

## TUI Contract

Required surfaces:
- Main market board with confidence and resilience indicators.
- Symbol detail pane with BBO/tradeability metrics.
- Why-ranked pane with score components and confidence penalties.
- Health pane with lag, queues, gaps, reconnects, and low-confidence count.
- Preset/filter surface for confidence, resilience, tradeability, and metadata cohorts.

Keyboard behavior is intentionally future-facing in this feature. If a Ratatui runtime is added, it must preserve:
- `q` quits cleanly.
- `/` opens filter entry.
- `p` opens preset selector.
- `space` pauses display refresh without stopping ingestion.
- `h` opens health.
- `enter` opens symbol details.

## Non-Goals

- No wallet/private-key prompt.
- No account/user feed subscriptions.
- No order placement, cancellation, leverage, withdrawal, or exchange action.
- No profitability or trade-signal language.
