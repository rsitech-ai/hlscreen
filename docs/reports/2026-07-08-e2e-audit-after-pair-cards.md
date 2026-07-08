# End-to-End Audit After Pair Cards

Date: 2026-07-08
Branch: `feat/andrzej_e2e_audit_20260708`
Scope: `hlscreen/` standalone Rust workspace
Decision: pass after fixes below

## Summary

The current implementation was audited against the repo contracts, Spec Kit
plan, source code behavior, runtime smokes, release gates, and official
Hyperliquid documentation.

Two issues were found and fixed:

1. The TUI quality band could say `GOOD` when live rows had no spread/depth
   evidence yet. It now reports `PARTIAL` when median spread or top-of-book
   depth is unavailable, and a regression test covers that state.
2. `health-json.svg` embedded the current `generated_at_ms`, causing screenshot
   churn. The screenshot generator now normalizes that volatile value.

After those fixes, the implementation starts cleanly, uses live public data in
network mode, keeps fixture paths explicit and hidden for deterministic tests,
fails closed on invalid inputs, and keeps all v1 surfaces read-only.

## Official Documentation Alignment

Checked against:

- Hyperliquid WebSocket endpoint docs:
  `https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket`
- Hyperliquid WebSocket subscriptions:
  `https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket/subscriptions`
- Hyperliquid timeouts and heartbeats:
  `https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket/timeouts-and-heartbeats`
- Hyperliquid Info endpoint and spot metadata:
  `https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/info-endpoint`
  and
  `https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/info-endpoint/spot`
- Hyperliquid rate limits:
  `https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/rate-limits-and-user-limits`

Findings:

- Default live WebSocket URL is the official mainnet URL
  `wss://api.hyperliquid.xyz/ws`.
- Subscription messages match the documented
  `{ "method": "subscribe", "subscription": { ... } }` shape for public
  `trades`, `bbo`, `activeAssetCtx`, and `candle` streams.
- Heartbeat pings are sent every 20 seconds, below the documented 60-second
  server timeout.
- Reconnects resubscribe and are recorded as explicit local data gaps instead
  of being hidden. Automatic public REST backfill is not implemented and is
  documented as a current limitation.
- Subscription budgeting stays below the documented 1,000 subscription cap by
  defaulting to 980 max subscriptions with headroom.
- Public REST spot metadata parsing handles `PURR/USDC` style spot names,
  `@{index}` feed IDs, numeric string fields, and missing token details without
  failing ingestion.
- Private/user/trading channels are rejected by the parser and no wallet,
  signing, order, cancel, withdrawal, or account routes are present.

## Source Review

Reviewed areas:

- `hls-hyperliquid`: public REST metadata, WebSocket parser, subscription
  builder, heartbeat message.
- `hls-cli live`: fixture/live split, official WS URL, network-mode guards,
  subscription budget fallback, heartbeat, reconnect, recorder backpressure,
  raw/normalized recording, clean shutdown.
- `hls-core`: config read-only safety, market state idempotency, receive
  timestamps, health severity, metrics cardinality, extension manifest safety.
- `hls-features`: timestamp-bounded return/volatility windows, confidence
  inputs, resilience/tradeability, score breakdowns.
- `hls-store`: raw zstd writer, normalized JSONL, SQLite registry, gaps,
  replay clean-shutdown enforcement, replay parity, public benchmark manifests.
- `hls-screen`: narrow DSL parser/evaluator, known fields only, missing values
  treated as non-matches and sorted last.
- `hls-tui`: stable deterministic renderer, all-pair detail cards from
  `FeatureSnapshot`, health and why-ranked panes, safety caveats.
- `hls-server`: read-only JSON helpers, health/symbol/screen/symbol-detail
  routes only, 400s for malformed input, 404s for unknown routes.
- CI/release/docs: Rust 1.88 CI, fmt, clippy, tests, release build, packaging
  check, tag-gated release workflow, threat model, privacy, release checklist,
  deterministic screenshots.

Code-review result:

- Correctness: pass after the TUI quality-status fix.
- Maintainability: pass. Modules keep I/O, parsing, feature math, storage,
  screening, and rendering boundaries separate.
- Readability: pass. The current diff is small and localized.
- Security/safety: pass for v1 read-only scope. Scans found no active secrets,
  private stream use, wallet surface, or order-capable command.
- Performance: pass for v1. Bounded channels and duration-bounded live runs
  avoid silent recorder loss. Full all-symbol long-run evidence already exists
  in repo memory; this audit reran bounded live smokes.
- OSS readiness: pass. License/community/release/security docs and generated
  screenshots are present; screenshot generation is now more reproducible.

## Runtime Evidence

Fixture/runtime matrix:

- Temp run directory: `/tmp/hlscreen-e2e-audit.1c4dZD`
- `hls init`: clean config with `read_only=true`
- `hls doctor --json`: config readable, data dir writable, read-only true
- `hls symbols --top 2 --asset-contexts-file ...`: rendered fixture-backed
  HYPE/PURR metadata table
- Fixture `hls live --once`: rendered read-only market board and pair detail
  cards
- Fixture `hls live --record --raw --normalized`: `clean_shutdown=true`
- `hls replay --verify-parity`: first run wrote baseline, second run passed
  with `confidence_drift=0`, `missing=0`, `extra=0`
- Replay-backed `hls screen`: rendered `PAIR DETAIL CARDS` and read-only caveat
- `hls explain`: rendered why-ranked components and "not advice" caveat
- `hls bench --json`: `matched=true`, `events_read=4`,
  `feature_latency_us=30`, expected hash matched output hash
- `hls server --print-health`: healthy read-only JSON
- All captured `*.err` logs in the fixture matrix were empty.

Negative probes:

- Invalid DSL `symbol > 10`: exited 1 with typed incompatible-comparison error.
- Fixture live without `--once`: exited 1 and refused to run.
- `hls record` without fixture file: exited 1 because network recording is not
  implemented in that deterministic command.
- Private/absolute benchmark manifest path: exited 1 and was rejected as not
  under `tests/fixtures/microstructure/`.

Live public data smokes:

- Single-symbol live smoke:
  `./target/debug/hls live --symbols @107 --duration-secs 15 --refresh-secs 5 --tui`
  - Temp dir: `/tmp/hlscreen-e2e-live.U5N6mT`
  - `symbols=1`
  - `subscriptions=4`
  - `ws_messages=92`
  - `market_events=129`
  - `reconnects=0`
  - `data_gaps=0`
  - `confidence_summary=high:1 medium:0 low:0 untrusted:0 min:100 reasons:0`

- Multi-symbol live smoke after the quality-status fix:
  `./target/debug/hls live --top 3 --duration-secs 15 --refresh-secs 5 --tui`
  - Temp dir: `/tmp/hlscreen-e2e-live-multi-fix.YgEP9W`
  - `symbols=3`
  - `subscriptions=12`
  - `ws_messages=60`
  - `market_events=135`
  - `reconnects=0`
  - `data_gaps=0`
  - `confidence_summary=high:3 medium:0 low:0 untrusted:0 min:100 reasons:0`
  - Quality band correctly reported `PARTIAL` when spread/depth were missing.

Screenshot verification:

- Regenerated SVG assets with `python3 scripts/generate-screenshots.py`.
- PNG previews rendered with `rsvg-convert`:
  - `/tmp/hlscreen-e2e-screens/live-screen.png`
  - `/tmp/hlscreen-e2e-screens/metadata-discovery.png`
  - `/tmp/hlscreen-e2e-screens/resilience-screen.png`
- Visual check: no text overlap; pair detail cards, metadata, resilience, and
  safety caveats are visible.

## Final Validation

Final pass over the completed diff:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo build --release --workspace --all-features
scripts/check-release-packaging.sh
python3 scripts/generate-screenshots.py
git diff --check
```

Result: all passed.

Additional scans:

```bash
rg -n "TODO|FIXME|todo!|unimplemented!|dbg!|eprintln!\\(\\\"debug|println!\\(\\\"debug|panic!\\(" --glob '!target/**' --glob '!docs/reports/**' .
rg -n "private_key|secret|api[_-]?key|seed phrase|mnemonic|wallet_enabled\\s*=\\s*true|trading_enabled\\s*=\\s*true|withdraw|cancel|place_order|create_order|exchange" --glob '!target/**' --glob '!docs/reports/**' .
```

Result: no actionable production findings. Matches are docs/tests/planning
references, expected safety fixtures, or test-only panics.

Supply-chain tools:

- `cargo audit`, `cargo deny`, and `cargo machete` were not installed in this
  environment, so no vulnerability/dependency-pruning result is claimed.

## Residual Caveats

- Automatic public REST backfill after reconnect is still not implemented.
  Reconnects are explicit and gaps are recorded.
- The localhost API remains a print-health/helper contract, not a long-running
  production HTTP server.
- Scores are screen heuristics only. The tool remains read-only public market
  data and does not trade or provide advice.
- A future public release should add a CI dependency-audit gate once the project
  chooses `cargo audit`, `cargo deny`, or another pinned tool.

## Review Decision

Approved for merge after fixes and validation. No blocking correctness,
security, performance, or maintainability findings remain for the current v1
read-only scope.
