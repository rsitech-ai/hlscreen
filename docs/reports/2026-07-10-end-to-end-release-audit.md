# End-to-End Release Audit - 2026-07-10

Branch: `feat/andrzej_agent_sota_lab`

Base: `origin/main` at `7ec3328401735ab25c73bb96838394f6b8790ede`

Status: local release gate passed for the read-only public-data workstation. No
wallet, private stream, signing, order, or exchange-action surface was added.

## Scope And Contracts

The audit covered all workspace crates, CLI boundaries, public REST/WebSocket
parsers, market state, feature/screen flow, recording/replay, read-only server
helpers, Ratatui rendering, terminal lifecycle, scripts, CI, packaging, and
public documentation. Runtime behavior was compared with:

- Hyperliquid [WebSocket endpoint](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket), [subscriptions](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket/subscriptions), [heartbeats](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/websocket/timeouts-and-heartbeats), [rate limits](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/rate-limits-and-user-limits), and [spot Info API](https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/info-endpoint/spot).
- Ratatui [`Terminal`](https://docs.rs/ratatui/0.30.2/ratatui/struct.Terminal.html), Crossterm [event handling](https://docs.rs/crossterm/0.29.0/crossterm/event/index.html), Tokio [`MissedTickBehavior`](https://docs.rs/tokio/1.52.3/tokio/time/enum.MissedTickBehavior.html), and Reqwest [`ClientBuilder::timeout`](https://docs.rs/reqwest/0.12.28/reqwest/struct.ClientBuilder.html#method.timeout).

The implementation remains one public-data WebSocket connection, uses the
official subscription shapes, reserves headroom below the IP-wide limits,
sends documented heartbeat messages, uses bounded HTTP timeouts, and renders
through one persistent Ratatui terminal.

## Findings Resolved

1. Explicit `--color always` was still suppressed by Crossterm when the parent
   shell exported `NO_COLOR`. The draw path now applies the resolved color
   policy immediately before each serialized terminal draw. A real-PTY
   regression proves RGB output with inherited `NO_COLOR=1`.
2. `scripts/generate-screenshots.py --check` ignored its flag and rewrote
   tracked files. Check mode is now read-only, compares all nine SVGs, rejects
   unknown arguments, and fails on drift.
3. Run IDs and SQLite file paths could influence storage paths or replace prior
   evidence. Run IDs now use a bounded ASCII grammar; writers reject symlinked
   parents and existing files; replay verifies canonical containment; run/file
   identities are append-only.
4. Public numeric strings could admit non-finite values, negative timestamps,
   prices, volume, or supply. REST, WebSocket, and screen-DSL boundaries now
   reject invalid values; observed zero price sentinels map to missing data.
5. Late events could regress current prices and live trade history was
   unbounded. Quotes/trades are ordered without regressing current state;
   context/global-mid updates are receive-time ordered; analytical history is
   capped to one hour and 100,000 trades per symbol.
6. Global-mid frames retained tracking entries outside the selected universe,
   and each TUI frame copied all retained trades/candles. Unknown symbols are
   ignored and presentation payloads are capped to the latest 64 events per
   symbol while the larger analytical window remains intact.
7. Control/ack frames could make a stalled run appear alive. Success,
   reconnect backoff reset, and the 60-second watchdog now require parsed
   market-data events. Pong write failures enter the same reconnect/gap path as
   other transport write failures.
8. All-symbol subscription fallback lost useful global price coverage. The
   planner now uses one `allMids` stream plus rich per-symbol streams when the
   budget permits, then degrades to context plus mids or mids-only.
9. Missed Tokio ticks could burst redraw work, pause did not freeze market
   presentation, recorder status was ambiguous, and a spread/depth heuristic
   was mislabeled `amihud`. Timers now skip missed ticks, pause freezes only
   displayed market data, recording status is phase-correct, and the column is
   labeled `cost`.
10. CLI validation happened after possible I/O for several contradictions.
    Invalid refresh/top/subscription values, selectors, recording flags,
    schemes, output formats, and duration overflow now fail before REST, files,
    or terminal activation.

## Verification Evidence

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`: passed with no warnings.
- `cargo test --workspace --all-features --locked`: 335 passed across 60 test/doc-test targets; 0 failed.
- `cargo build --release --workspace --all-features --locked`: passed on Rust 1.88.0.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked`: passed.
- `cargo audit --deny warnings`: 346 locked dependencies scanned; no vulnerability or warning.
- Ruff lint/format, Python bytecode compilation, and `bash -n` over tracked shell scripts: passed.
- `python3 scripts/generate-screenshots.py --check`: 9 deterministic screenshots verified without writes.
- `scripts/check-release-packaging.sh`: 4 packaging contract tests passed.
- Benchmark `gap_replay_v1`: expected hash matched; feature latency 35 us against a 100,000 us ceiling.
- Release CLI matrix: help/version/terminal doctor, health JSON, expected bare-server rejection, forced color, no-color, preflight failure without data-dir creation, record/replay, and two parity runs passed.
- Real PTY suite: 5/5 passed in three consecutive runs, including resize, signal, cleanup escalation, terminal restoration, and forced color under `NO_COLOR=1`.
- Bounded unreachable WebSocket: failed closed in 3 seconds with two reconnect attempts/data gaps and a truthful no-market-data error.

## Live Public Evidence

Top-10, 10 seconds:

- 10 symbols, 40 subscriptions, 204 WebSocket messages, 456 market events.
- 10 high-confidence rows; 0 reconnects; 0 data gaps; clean duration stop.

All-symbol, 10 seconds:

- 309 symbols, 928 subscriptions: one global stream and three per symbol.
- 2,013 WebSocket messages, 7,267 market events; 0 reconnects; 0 data gaps.
- 116 high-confidence and 193 low-confidence rows. Low confidence is expected
  for inactive/sparse pairs in a short window and is exposed rather than hidden.

Live `hls tui`, 120x40 PTY, 6 seconds:

- Default top 10 completed with 40 subscriptions and zero gaps.
- One alternate-screen enter/leave, one full-screen clear, balanced mouse state,
  655 RGB foreground sequences, and explicit ANSI theme status despite inherited
  `NO_COLOR=1`.

Logs contained no unexpected errors, warnings, panics, or reconnects. The bare
`hls server` command intentionally reports that a long-running daemon is not
implemented; `server --print-health` and all read-only API contracts passed.
After the audit, no HLS runtime or fixture-test process remained.

## Review Decision And Remaining Limits

Local review found no unresolved blocker or important correctness, security,
maintainability, readability, or performance issue in the final diff. The
branch is suitable for PR and merge only after GitHub checks pass.

Remaining product limits are explicit:

- No REST backfill after a reconnect, so any gap lowers confidence for the run.
- No true Parquet writer and no long-running HTTP daemon.
- The process respects IP-wide exchange limits, but independent concurrent HLS
  processes cannot coordinate one shared IP budget; operators should run one
  all-symbol collector per public IP.
- The live runs are bounded release smokes, not an extended high-volume soak.
  Sustained all-symbol memory/render telemetry and fault-injection storms remain
  the next resilience gate.
- Path containment assumes the local data directory is not concurrently
  rewritten by a hostile same-user process.
- Public release binaries remain unproven until a reviewed `v*` tag workflow
  builds and publishes checksummed artifacts.
