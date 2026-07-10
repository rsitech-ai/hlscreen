# Post-Merge Production Audit - 2026-07-10

## Verdict

The audited revision is stable for its documented scope: a local, read-only,
public Hyperliquid spot-data workstation with bounded live runs, recording,
replay, screening, a keyboard-interactive TUI, and a loopback API preview.

This is not evidence for unattended production deployment, public network
hosting, private account monitoring, or trading. The repository still labels
those boundaries explicitly in `docs/production-readiness.md` and
`docs/deployment.md`.

## Official Contract Review

The implementation was checked against Hyperliquid's official public API
documentation:

- WebSocket subscriptions and public spot feed identifiers:
  <https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket/subscriptions>
- WebSocket reconnect requirement:
  <https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket>
- IP rate limits: 1,000 subscriptions, 2,000 outbound WebSocket messages per
  minute, 30 new WebSocket connections per minute, and 1,200 REST weight per
  minute:
  <https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/rate-limits-and-user-limits>
- Public Info and spot metadata/candle endpoints:
  <https://hyperliquid.gitbook.io/Hyperliquid-docs/for-developers/api/info-endpoint>
  and
  <https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/info-endpoint/spot>

Timer behavior was checked against Tokio's documented missed-tick semantics:
<https://docs.rs/tokio/latest/tokio/time/enum.MissedTickBehavior.html>.

## Findings Resolved

1. The live API server sent up to 930 subscriptions immediately after every
   reconnect and retried after a fixed one-second delay. A reconnect storm
   could exceed both official outbound-message and connection-rate limits.
   The TUI and server now share a rolling 1,900-message/60-second limiter; the
   server counts subscriptions, heartbeats, and pongs and uses exponential
   reconnect delay capped at 30 seconds.
2. A bounded live API run could outlive `--duration-secs` while connecting or
   writing. Connection establishment, outbound writes, rate-limit waits, and
   reconnect sleeps now all observe the monotonic run deadline.
3. Backfill file registration and backfill-attempt registration were separate
   SQLite writes. An attempt insert failure left a committed file row and final
   artifact. They now commit in one transaction, and a pending-file guard
   removes the candidate artifact on failure. A SQLite trigger regression test
   proves rollback of both database and filesystem state.
4. The loopback HTTP preview had unbounded connection tasks, no request-header
   timeout, and checked the 16 KiB limit after accepting a complete header.
   It now caps concurrent connections at 64, times incomplete headers out after
   five seconds, checks size before acceptance, tracks tasks, and aborts them on
   shutdown.
5. `/symbol/HYPE%2FUSDC` returned 404 for live rows stored under feed ID `@107`.
   Detail lookup now accepts display pairs, hyphen/slash variants, and feed IDs.
6. Health derived current connection state from historical gap count, leaving a
   recovered server permanently marked `reconnecting`. Current state and gap
   history are now independent and regression-tested.
7. Deterministic screenshot generation hardcoded the checkout-local `target`
   directory. It now honors `CARGO_TARGET_DIR`, preventing stale binaries from
   another worktree from producing false failures or stale captures.

## Static And Security Gates

All commands passed from a clean isolated Cargo target:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo build --release --workspace --all-features --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked
cargo audit --deny warnings --ignore RUSTSEC-2024-0436
CARGO_TARGET_DIR=<isolated> python3 scripts/generate-screenshots.py --check
scripts/check-public-readiness.sh
git diff --check
```

RustSec scanned 428 locked dependencies with no vulnerability finding. The one
explicit exception is the documented unmaintained `paste` proc-macro pulled by
Apache Parquet 59.1.0; it is not a vulnerability exception. Both GitHub Actions
workflow files parsed successfully, every shell script passed `bash -n`, and
the screenshot generator passed Python bytecode compilation.

## Live End-To-End Evidence

Command:

```bash
hls live \
  --all-symbols \
  --duration-secs 900 \
  --refresh-secs 30 \
  --tui \
  --record \
  --raw \
  --normalized \
  --run-id audit-allpairs-20260710-15m \
  --data-dir /tmp/hlscreen-audit-20260710-15m
```

Result:

| Measure | Result |
| --- | ---: |
| Symbols | 310 |
| Subscriptions | 931 |
| WebSocket messages | 286,205 |
| Normalized events | 294,144 |
| Raw files | 13 |
| Normalized files | 1 |
| Reconnects / gaps | 0 / 0 |
| Clean shutdown | true |
| Evidence size | 60 MB |

The TUI rendered real display pairs such as `HFUN/USDC`, maintained recording
status, and restored the terminal cursor on exit. During the run RSS remained
roughly 41-60 MB and CPU roughly 0.7-2.1% in sampled checkpoints. The terminal
stream contained no panic, warning, failure, reconnect, or gap diagnostic.

A separate 30-second all-symbol live API run served 310 rows through 930
subscriptions, processed 7,731 WebSocket messages / 15,426 market events, and
ended with zero reconnects and gaps. `/health`, `/symbols`, `/screen`, and
symbol detail were probed while the process was live.

## Replay And Output Validation

The first replay wrote a 310-row confidence baseline. The second replay passed:

```text
replay_parity=passed
confidence_baseline=310
confidence_replay=310
confidence_drift=0
confidence_missing=0
confidence_extra=0
confidence_summary=high:159 medium:0 low:151 untrusted:0 min:60 reasons:302
```

`thin_books` returned 14 rows and `flow_pressure` returned one row. Their
stderr files were empty. `doctor --live --json` reached public REST, reported
`healthy`, `read_only=true`, a writable data directory, and zero reconnect/gap
metrics.

## Remaining Boundaries

- Automatic coarse REST backfill is not invoked by the live reconnect path.
- Missing trades and BBO cannot be reconstructed from candle snapshots.
- This was a 15-minute bounded run, not multi-hour or multi-day supervised soak
  evidence.
- The localhost API remains a preview, not an authenticated hosted service or
  supported daemon deployment.
- No reviewed `v*` public binary release has been published.
- Private fee-tier lookup, realized fill modeling, and trading actions remain
  outside the public read-only trust boundary.

These are readiness limits, not hidden fallbacks. Live network runs use public
REST/WebSocket data; fixtures remain explicit test-only inputs.
