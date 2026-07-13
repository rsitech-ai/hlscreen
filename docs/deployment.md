# Deployment Status

`hlscreen` does not currently provide a production daemon or a supported
supervisor deployment package.

The `hls server` command exposes read-only route helpers, an operator-terminated
localhost loop over in-memory state, and a bounded live-data preview. The CLI
rejects non-loopback bind addresses. These modes are useful for tests and local
inspection, but they do not provide the supported lifecycle, durable state,
authentication, upgrade policy, or operational guarantees expected from a
production service.

The bounded live preview coalesces data-bearing WebSocket bursts into at most
one full API snapshot recomputation every 250 ms and skips timer catch-up after
idle or scheduler stalls. `--refresh-secs` remains the slower health refresh
cadence; the final state is also published at shutdown.

The preview also keeps all outbound WebSocket messages under a rolling
`1,900`-per-60-second budget, applies exponential reconnect delay from one
second to a 30-second cap, and limits connection attempts to 29 per rolling 60
seconds even when a data-bearing socket repeatedly flaps. Connection
establishment, outbound writes, and both rate-limit waits are included in the
bounded duration. The loopback HTTP listener
allows at most 64 concurrent connection tasks, limits request headers to 16
KiB, times out incomplete headers after five seconds, and aborts outstanding
connection tasks on shutdown. These are local resource and lifecycle guards;
they do not turn the preview into a supported hosted service.

Symbol detail routes accept the user-facing pair (`HYPE/USDC` or
`hype-usdc`) and Hyperliquid's transport feed identifier (`@107`). Returned
rows preserve the transport identifier in `symbol` and expose the display pair
through public metadata.

Experimental process-manager templates may exist in the repository while this
work is developed. They are not an install path, release artifact, or evidence
that unattended operation has been validated.

## Supervised Soak Evidence

The repository includes a bounded evidence runner for operator-supervised,
public-data validation:

```bash
scripts/run-supervised-soak.sh \
  --duration-secs 900 \
  --sample-interval-secs 30 \
  --data-dir /var/tmp/hlscreen-soak
```

It builds or uses the release binary, checks available disk space, records the
exact all-symbol command and Git commit, forwards termination signals, samples
CPU/RSS/storage growth, records raw and normalized public data, invokes coarse
gap backfill, and runs replay parity twice. It publishes `report.json`
atomically beside retained stdout/stderr and resource samples.

The report is accepted only when the live process exits cleanly, receives real
symbols/messages/events, has at least two monotonic resource samples, records
no parser drops or failed backfill requests, leaves no exact tick-level gaps,
stays within the configured RSS-growth bound, and passes the second replay with
zero drift/missing/extra rows. Revalidate retained evidence with:

```bash
python3 scripts/validate-soak-report.py \
  /var/tmp/hlscreen-soak/soak-reports/<run-id>/report.json \
  --minimum-duration-secs 900
```

A passing 15-minute report is bounded smoke evidence, not multi-day soak proof.
For a two-day candidate, run with `--duration-secs 172800` under direct operator
supervision and retain the entire evidence directory. The runner is not a
process supervisor and does not promote the experimental templates to a
supported deployment package.

## Operational Acceptance Matrix

Run these focused gates before a supervised candidate:

```bash
# Reconnect/gap persistence and coarse REST repair, including source failure.
cargo test -p hls-store --test backfill_gaps
cargo test -p hls-cli --test backfill_command

# Malformed/private/non-finite public messages fail closed.
cargo test -p hls-hyperliquid --test ws_parser

# SIGINT/SIGTERM restore the TUI and complete closeout.
cargo test -p hls-cli --test pty_tui fixture_tui_restores_terminal_when_signaled_during_initial_frame

# Static supervisor boundaries plus bounded loopback fixture smoke.
scripts/check-supervisor-packaging.sh

# Clean process restart over the same read-only fixture configuration.
for attempt in 1 2; do
  target/debug/hls server \
    --live \
    --symbols @107 \
    --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson \
    --bind 127.0.0.1:0
done
```

Any failed command blocks deployment claims. A live reconnect also creates an
exact tick-level gap even when public candles are appended; the soak validator
therefore rejects the report until exact reconstruction exists or the run is
repeated without a gap.

## Required Before Deployment Support

- A supported server lifecycle with restart and recovery acceptance tests.
- Explicit persistence and recovery behavior across process restarts.
- Authentication and authorization before any non-loopback exposure.
- Structured service logs, metrics, health semantics, and alerting ownership.
- Configuration validation, resource limits, and upgrade/rollback procedures.
- Multi-hour and multi-day supervised soak reports with exact versions,
  reconnect counts, gaps, memory/CPU trends, and replay-parity evidence.
- Installation, uninstall, and incident-response documentation validated on each
  supported operating system.

Until those gates pass, use server modes only for local read-only evaluation and
do not treat them as an unattended deployment story.
