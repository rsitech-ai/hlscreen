# Contract: Confidence and Score Explanation

## Purpose

Every ranked row must state both what the system observed and how trustworthy that observation is. Scores are screening heuristics only.

## Confidence Reason Codes

Required initial reason codes:

```text
gap_active
gap_recent
bbo_stale
trade_sparse
asset_ctx_stale
parser_drops
writer_backlog
clock_skew
duplicate_events
metadata_missing
unsupported_schema
```

## Confidence State Rules

```text
trusted:
  confidence_score >= 0.90
  no active gap
  required feature windows valid

degraded:
  0.30 <= confidence_score < 0.90
  one or more reason codes present
  row may render but must show caveat

invalid:
  confidence_score < 0.30
  required data unavailable
  ranking score must be suppressed or clearly marked invalid
```

Exact numeric penalties are implementation details but must be documented in `docs/feature-definitions.md` once implemented.

## Score Breakdown Shape

Human-readable row:

```text
score_total=73.2 confidence=0.91 why=+resilience +flow -spread_cost
```

JSON shape:

```json
{
  "symbol": "@107",
  "snapshot_ts_ns": 1710000066000000000,
  "total_score": 73.2,
  "confidence_score": 0.91,
  "version": "microstructure-v1",
  "components": [
    {
      "name": "liquidity_resilience",
      "raw_value": 0.84,
      "normalized_value": 84.0,
      "weight": 0.35,
      "signed_contribution": 29.4,
      "direction": "positive",
      "evidence_window": "30s"
    }
  ],
  "unavailable_evidence": []
}
```

## Required Invariants

- Ranking must use confidence-adjusted totals or visibly separate confidence from rank.
- A top-ranked row must not hide `degraded` or `invalid` state.
- Missing data cannot be treated as zero unless the metric definition explicitly says zero is the correct value.
- BBO-only metrics must include `bbo` or `top_of_book` in their description or documentation.
- Score components must be deterministic under replay for a given fixture and version.
- Score versions must change when formulas or weights change incompatibly.

## Replay Parity

Replay parity checks compare:
- feature snapshot fields
- confidence state and reason codes
- score breakdown components
- ranking order
- output hashes where exact rendering is part of the benchmark

Allowed tolerance classes:
- exact string/enum equality
- integer equality
- floating tolerance by metric
- intentionally ignored wall-clock/render timestamps

Any mismatch outside tolerance returns non-zero for benchmark validation.
