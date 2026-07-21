# Production Readiness

`hlscreen` is a read-only local live-data preview with bounded validation evidence. It is not production-ready for unattended operation, live trading, private account monitoring, hosted service operation, or capital-touching automation.

## Readiness Label

**Status:** local read-only live-data preview; production readiness is not yet proven.

Supported bounded local use:

- Build from source with Rust 1.88+.
- Run bounded public WebSocket sessions over all currently available spot symbols.
- Record raw `.ndjson.zst`, normalized JSONL, and SQLite registry metadata.
- Replay captured runs and verify confidence parity.
- Screen captured rows with deterministic presets or DSL filters.
- Render deterministic terminal output and health JSON.
- Preview read-only localhost HTTP routes over current in-memory state.
- Run a bounded localhost API preview backed by public live market-data snapshots with `hls server --live`.
- Stop loopback servers through the shared SIGINT/SIGTERM Unix and CTRL-C
  Windows lifecycle. The isolated process smoke proves plain-server Unix
  SIGTERM cleanup and same-port restart locally. Shared live cancellation and
  signal mapping are unit-tested; a live process and Windows CTRL-C were not
  runtime-proven by that smoke.
- Evaluate validated local-only alert playbooks in the TUI with bounded in-memory
  history and no external delivery or exchange action.

Not included:

- Wallets, private keys, private streams, signed actions, orders, cancels, withdrawals, leverage, liquidation, or execution.
- Trading advice, recommendations, or profitability claims.
- A supported long-running daemon, hosted multi-user API, public network exposure, production supervisor deployment, or completed multi-day soak proof.
- Public release binaries/checksums from a reviewed `v*` tag.
- Full tick-level public-data repair after live reconnect. Coarse public candle rows may be appended, but missing trades/BBO are not reconstructed and the original gap remains degraded.
- A production alert engine, validated canonical production microstructure metric suite, private account fee-tier lookup, realized fill model, or service-backed historical analog search.
- Parquet replay for feature/confidence datasets; those datasets can be exported
  today, but replay currently supports normalized-event Parquet only.

Current resource boundaries are finite but intentionally conservative: public
REST backfill uses at most 1,100 weighted units per rolling minute; live/server
WebSocket clients use at most 1,900 outbound messages and 29 new connections per
rolling minute; analog replay samples every five minutes and retains at most 288
candidates per symbol. The hosted-surface gate bounds each `gh` read to 120
seconds and its local Git SHA read to 10 seconds by default; validated test
overrides are limited to 1–600 and 1–60 seconds respectively. These controls
provide headroom and bounded failure, not unattended availability, complete
historical analog coverage, or production-service readiness.

## Release Engineering State

The repository now has a least-privilege candidate release pipeline: fixed
runners, full-SHA action pins, disabled checkout credential persistence,
workflow concurrency, a pinned zizmor gate, disabled release caches/containers,
shell-expression isolation, version-pinned build tooling,
an all-target cargo-deny license/source policy,
pull-request artifact builds, source archives, SHA-256 checksums, CycloneDX
SBOM generation, cargo-auditable binaries, and tag-only artifact attestations.
Local `dist plan`, packaging contract checks, and unpacked binary smoke are
repository-owned gates. Clean-runner candidate uploads, provenance, and a public
release remain unproven until the feature PR and a later explicitly approved
tag workflow complete.

Because GitHub artifact attestations for private repositories require Enterprise
Cloud, the current private-repository state is an external release blocker. It
does not block source development or ordinary PR CI.

## Latest Live Validation

Run: `oss-release-v012-20260721`

Fresh supervised evidence at commit
`96ade27499240a5d956e459497e97f57eacf0922`:

- Measured duration: `902` seconds (`--duration-secs 900`)
- Symbols: `314`
- Public subscriptions: `943`
- Raw WebSocket messages: `300,147`
- Normalized market events: `308,104`
- Reconnects / data gaps / parser drops: `0 / 0 / 0`
- Failed public backfill requests: `0`
- SQLite clean shutdown: `true`
- Replay status: `baseline_written`, then `passed`
- Replay drift / missing / extra: `0 / 0 / 0`
- Peak settled RSS: `37,371,904` bytes
- Final evidence size: `66,330,624` bytes

The exact command, timestamps, resource samples, limits, counters, and replay
results are committed in the
[machine-readable 15-minute soak report](evidence/soak/sota-allpairs-20260720-15m.json).
Its live stderr contained only expected 30-second progress records; both replay
stderr logs were empty. This is fresh bounded all-symbol evidence, not multi-day
or unattended production proof.

Previous 15-minute evidence follows for comparison.

Run: `audit-allpairs-20260710-15m`

Current post-merge evidence:

- Duration: `899` measured seconds (`--duration-secs 900`)
- Symbols: `310`
- Public subscriptions: `931` (`allMids` plus per-symbol trades, BBO, and active asset context)
- Raw WebSocket messages: `286,205`
- Normalized market events: `294,144`
- Raw files: `13`
- Normalized files: `1`
- Registered files: `14`
- Reconnects: `0`
- Data gaps: `0`
- SQLite clean shutdown: `true`
- Replay confidence rows: `310`
- Replay drift/missing/extra: `0 / 0 / 0`

The exact command, official-document checks, code-review findings, and gate
results were recorded in the maintainer's internal post-merge production audit
for that run; the machine-validated evidence lives in
[`docs/evidence/soak/`](evidence/soak/sota-allpairs-20260720-15m.json).

Opt-in closeout repair is now available with `--backfill-gaps` on a normalized
recorded live run, plus the standalone `hls backfill` command. This is coarse
candle coverage only. REST failures are recorded as unrepaired attempts and
produce a non-zero command result; repeat attempts are skipped unless `--retry`
is explicit. No current readiness claim treats this as tick-level recovery.

Machine-validated supervised soak evidence is available through
`scripts/run-supervised-soak.sh`. Its versioned report records the exact commit
and command, CPU/RSS/storage samples, reconnect and data-quality counters,
shutdown state, and two replay-parity outcomes. The validator rejects dirty or
short runs, missing samples, parser/backfill failures, unrepaired tick gaps,
memory growth beyond the declared limit, and replay drift. This closes the
repository tooling gap; no multi-day run has yet been completed and the
deployment status remains experimental.

The runner itself was exercised against the public feed on 2026-07-13 with a
15-second requested capture (`ops-smoke-live-20260713`): 310 symbols, 931
subscriptions, 2,829 WebSocket messages, 10,515 normalized events, zero
reconnects/gaps/parser drops, clean closeout, and zero replay
drift/missing/extra. This validates the bounded runner path only; it does not
replace the required multi-day evidence.

Earlier five-minute evidence remains below for comparison.

Run: `allpairs-prodreadiness-20260708-201752`

Command:

```bash
./target/debug/hls live \
  --all-symbols \
  --duration-secs 300 \
  --refresh-secs 30 \
  --tui \
  --record \
  --raw \
  --normalized \
  --run-id allpairs-prodreadiness-20260708-201752 \
  --data-dir /tmp/hlscreen-prodreadiness-20260708-201752
```

Result:

- Symbols: `308`
- Public subscriptions: `924`
- Streams per symbol: `3` (`trades`, `bbo`, `activeAssetCtx`)
- Duration: `300` seconds
- Raw WebSocket messages: `99,162`
- Normalized market events: `106,980`
- Raw files: `5`
- Normalized files: `1`
- SQLite run clean shutdown: `true`
- Reconnects: `0`
- Data gaps: `0`

Post-fix renderer confirmation:

- Run: `allpairs-prodreadiness-postfix-20260708-202420`
- Duration: `60` seconds
- Symbols: `308`
- Public subscriptions: `924`
- Raw WebSocket messages: `18,791`
- Normalized market events: `26,455`
- Reconnects: `0`
- Data gaps: `0`
- TUI header verified: `p95 row age` and `quality partial` for broad all-symbol rows with partial quote/depth evidence.

## Replay And Screen Evidence

Replay parity over the 300-second capture:

```bash
./target/debug/hls replay \
  --data-dir /tmp/hlscreen-prodreadiness-20260708-201752 \
  --run-id allpairs-prodreadiness-20260708-201752 \
  --verify-parity
```

Evidence:

- First replay: `replay_parity=baseline_written`
- Second replay: `replay_parity=passed`
- Confidence baseline rows: `308`
- Confidence replay rows: `308`
- Drift: `0`
- Missing: `0`
- Extra: `0`
- Confidence summary: `high:296 medium:0 low:12 untrusted:0 min:60 reasons:24`

Screen presets over captured data:

- `thin_books`: 11 rows, clean stderr, `quality watch`
- `flow_pressure`: 3 rows, clean stderr, `quality good`

The original live-data captures were local audit evidence and are not distributed
as repository assets. Committed README screenshots remain deterministic
fixture/replay assets generated by:

```bash
python3 scripts/generate-screenshots.py
```

## Deployment Checklist

Use this checklist for bounded local validation:

1. Build and test:

   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
   cargo test --workspace --all-features --locked
   cargo build --release --workspace --all-features --locked
   RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked
   cargo audit --deny warnings --ignore RUSTSEC-2024-0436
   cargo deny check licenses sources
   scripts/check-release-packaging.sh
   ```

   The single ignored advisory is the unmaintained `paste` proc-macro pulled
   transitively by Apache Parquet 59.1.0. It is not a vulnerability exception;
   all vulnerabilities and all other dependency warnings remain denied.

2. Create a local data directory outside the repo:

   ```bash
   export HLS_DATA_DIR=/var/tmp/hlscreen-data
   mkdir -p "$HLS_DATA_DIR"
   ```

3. Run a bounded all-symbol capture:

   ```bash
   ./target/release/hls live \
     --all-symbols \
     --duration-secs 900 \
     --refresh-secs 60 \
     --tui \
     --record \
     --raw \
     --normalized \
     --run-id allpairs-$(date +%Y%m%d-%H%M%S) \
     --data-dir "$HLS_DATA_DIR"
   ```

   For an evidence package with resource samples and automatic parity checks,
   use:

   ```bash
   scripts/run-supervised-soak.sh \
     --duration-secs 900 \
     --sample-interval-secs 30 \
     --data-dir "$HLS_DATA_DIR"
   ```

4. Replay and verify:

   ```bash
   ./target/release/hls replay --data-dir "$HLS_DATA_DIR" --run-id <run-id> --verify-parity
   ./target/release/hls replay --data-dir "$HLS_DATA_DIR" --run-id <run-id> --verify-parity
   ```

5. Screen captured rows:

   ```bash
   ./target/release/hls screen --data-dir "$HLS_DATA_DIR" --run-id <run-id> --preset thin_books
   ./target/release/hls screen --data-dir "$HLS_DATA_DIR" --run-id <run-id> --preset flow_pressure
   ```

6. Check health:

   ```bash
   ./target/release/hls doctor --live --json --data-dir "$HLS_DATA_DIR"
   ./target/release/hls server --print-health
   ./target/release/hls server --bind 127.0.0.1:8787
   ./target/release/hls server --live --symbols hype-usdc --duration-secs 300 --bind 127.0.0.1:8787
   ```

The HTTP commands above are bounded local previews. They are not a production
daemon or deployment procedure. See [deployment.md](deployment.md) for the
missing production-service gates.

## Operational Signals

Treat these as fail-closed or investigate-now signals:

- Non-zero reconnects or data gaps in a run where continuous coverage matters.
- `clean_shutdown=false` in SQLite.
- Replay parity drift after a baseline exists.
- Writer backlog or backpressure health warnings.
- Unknown or unsupported Hyperliquid public channel payloads.
- TUI `quality partial`, `watch`, or `check` when the workflow requires complete quote/depth evidence.

## Open-Source Release State

Ready:

- MIT license, contribution guide, support policy, security policy, code of conduct.
- GitHub issue/PR templates, CI, Dependabot policy, and release packaging dry-run checks.
- SHA-pinned least-privilege workflows, PR candidate artifact configuration,
  source/checksum/SBOM generation, and tag-only provenance configuration.
- Deterministic screenshots and diagrammed architecture docs.
- Threat model, privacy doc, data format doc, feature definitions, and release checklist.

Still required before a public binary release:

- Review and tag a `v*` release.
- Make the repository public or use an eligible Enterprise Cloud repository so
  the required artifact attestations can run.
- Verify the feature PR candidate artifacts on every supported runner.
- Verify generated GitHub release artifacts, checksums, and installers.
- Publish installation instructions only after the tag workflow succeeds.
