# Advanced TUI Proof Report - 2026-07-09

## Scope

This report closes the local proof tasks for `specs/004-advanced-tui-workstation`.

Implemented proof:

- 80, 120, and 160 column Ratatui viewport fixture helpers.
- Regression coverage for adaptive narrow/medium/wide cockpit rendering.
- Display pause verified as a UI-only state: ingestion, market-state updates, raw recording, and normalized recording continue.
- Current screenshot SVGs regenerated from the deterministic screenshot script.
- Bounded public Hyperliquid all-symbol live smoke.

This does not claim a multi-day supervised daemon, plugin execution, node-backed ingestion, public release publication, private account integration, wallet access, or order routing.

## Focused Validation

```bash
cargo test -p hls-tui --test ratatui_cockpit --test interactive_tui
cargo test -p hls-cli commands::live::tests --bin hls
cargo test -p hls-cli --test live_mock
python3 scripts/generate-screenshots.py
```

Result:

- `ratatui_cockpit`: 12 passed.
- `interactive_tui`: 7 passed.
- `commands::live::tests`: 13 passed.
- `live_mock`: 3 passed.
- Screenshots regenerated under `docs/assets/screenshots/`.

## Live Public Smoke

Command:

```bash
rm -rf target/live-smoke-004
./target/debug/hls live \
  --all-symbols \
  --duration-secs 15 \
  --refresh-secs 5 \
  --record \
  --raw \
  --normalized \
  --data-dir target/live-smoke-004 \
  --color never
```

Observed result:

| Metric | Value |
| --- | ---: |
| Symbols | 308 |
| Subscriptions | 924 |
| Streams per symbol | 3 |
| WebSocket messages | 4,003 |
| Normalized market events | 11,638 |
| Reconnects | 0 |
| Data gaps | 0 |
| Elapsed seconds | 15 |
| Confidence summary | high:296 medium:0 low:12 untrusted:0 |
| Raw files | 1 |
| Normalized files | 1 |
| Normalized rows on disk | 11,638 |
| Clean shutdown | true |

Artifacts:

- `target/live-smoke-004/raw/ws/run=run-1783554606400/part-000000.ndjson.zst`
- `target/live-smoke-004/normalized/events/run=run-1783554606400/part-000000.ndjson`
- `target/live-smoke-004/hls.sqlite`

The live output rendered the Ratatui workstation table with real public market data, including `REC`, `LIVE`, `p95 row age`, confidence counts, read-only safety copy, and selected-symbol detail. Many thin or metadata-only rows correctly showed missing BBO/trade evidence rather than fabricated spread, imbalance, or flow values.

## Screenshot Assets

Regenerated assets:

- `docs/assets/screenshots/live-screen.svg`
- `docs/assets/screenshots/metadata-discovery.svg`
- `docs/assets/screenshots/confidence-degraded.svg`
- `docs/assets/screenshots/resilience-screen.svg`
- `docs/assets/screenshots/why-ranked.svg`
- `docs/assets/screenshots/record-replay.svg`
- `docs/assets/screenshots/health-json.svg`
- `docs/assets/screenshots/health-panel.svg`
- `docs/assets/screenshots/symbols.svg`

## Residual Risk

- The TUI is validated with deterministic snapshots and bounded public live smoke, not a 24/7 supervised deployment.
- Public all-symbol smoke verifies the current public WebSocket path and local recorder under a 15-second run. It does not prove multi-day reconnect behavior under venue or network incidents.
- Many Hyperliquid spot markets are thin. Missing BBO/trade fields are expected and must remain visible as missing evidence, not converted into false precision.
- Broader plugin ecosystem work, node-backed ingestion, and richer alert/playbook workflows remain in later roadmap work. The current plugin path is limited to bounded CLI row annotations.
