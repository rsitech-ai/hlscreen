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
second to a 30-second cap, and includes connection establishment, outbound
writes, and rate-limit waits in the bounded duration. The loopback HTTP listener
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
