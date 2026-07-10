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
one full API snapshot recomputation every 250 ms. `--refresh-secs` remains the
slower health refresh cadence; the final state is also published at shutdown.

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
