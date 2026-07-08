# Threat Model

## Assets

- Local raw market-data captures.
- Local normalized replay data.
- Local SQLite metadata registry.
- Operator trust in read-only safety.
- Integrity of feature and screening outputs.
- Integrity of benchmark expected hashes.
- Integrity of extension manifests and release packaging config.

## Trust Boundaries

- Hyperliquid public REST and WebSocket payloads are external inputs.
- Local fixture and replay files can be malformed or stale.
- CLI arguments and screen-rule expressions are untrusted input.
- Local data directories are operator-controlled and can contain old or partial runs.
- Extension manifests are untrusted until validated.
- Release packaging workflows are external CI surfaces and must remain tag-gated.

## Non-Goals

- No order placement.
- No wallet integration.
- No signing.
- No private account data.
- No execution, leverage, liquidation, or position management.

## Risks And Controls

| Risk | Control |
| --- | --- |
| Accidentally adding order-capable API calls | Source scans, PR checklist, no `/exchange` usage in v1 |
| Treating screen scores as trading signals | README/docs language and issue template safety checks |
| Malformed public payloads crash the tool | Typed parsing and error-returning parser tests |
| Reconnect/replay duplicates inflate features | `unique_trade_id` idempotency |
| Full-history windows create false freshness | Timestamp-bounded feature windows |
| Invalid config appears safe | `hls doctor` fails closed on invalid existing config |
| Local API path/query decoding bugs | UTF-8 percent decoding and JSON `400` responses |
| Raw captures committed accidentally | `.gitignore` excludes `.hls/`, `data/`, compressed raw files, SQLite, and Parquet |
| Benchmark expected hashes normalize a regression | Expected hash changes require reviewed benchmark evidence |
| Extension manifests request unsafe host capabilities | v1 contract rejects network, filesystem, private-data, and trading permissions |
| Release workflow publishes unexpectedly | Release packaging is tag-gated and local checks use no release secrets |

## Security Review Checklist

- Search for `exchange`, `order`, `cancel`, `withdraw`, `private_key`, `seed`, `wallet_enabled = true`, and `trading_enabled = true`.
- Run the full Rust validation gate.
- Run fixture live/record/replay/screen smokes.
- Run `hls bench --manifest tests/fixtures/microstructure/benchmark_gap_replay.json --repo-root . --json`.
- Run `scripts/check-release-packaging.sh`.
- Inspect docs for overclaims about trading, profitability, or live execution.
- Keep dependency updates reviewable through Dependabot.
