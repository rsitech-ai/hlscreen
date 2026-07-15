# Microstructure Fixtures

This directory stores small, deterministic public-market-data fixtures for the
Hyperliquid Microstructure Workstation feature set.

Fixture rules:

- Use public market-data shapes only.
- Keep files small enough for fast CI.
- Include gap, sparse-trade, duplicate-event, resilience, metadata, and benchmark
  cases as separate files.
- Do not include wallet, account, private stream, or order data.
- Prefer raw WebSocket NDJSON when the test is about replay parity; use JSON
  manifests when the test is about metadata or expected outputs.
- Keep canonical metric contracts beside expected values in
  `canonical_metric_benchmark.json`; runtime and fixture contracts must match
  exactly before tolerance comparison.
