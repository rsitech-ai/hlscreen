# hlscreen Constitution

## Core Principles

### I. Read-Only Public Data Boundary

`hlscreen` is a public Hyperliquid spot market-data workstation. Features MUST NOT add wallet prompts, private account streams, signing, order placement, cancellation, withdrawal, leverage, or exchange mutation routes. Any command, plugin, API, TUI control, or background process that could be mistaken for trading execution must fail closed or remain out of scope.

### II. Replayable Evidence Before Ranking

Every ranking, alert, metric, and visual claim must be reproducible from recorded public data or deterministic fixtures. Raw public captures, normalized replay events, confidence state, and score explanations are first-class evidence. If a window is incomplete, stale, sparse, or affected by a gap, the UI and machine-readable outputs must say so.

### III. Live Truth Over Mock Convenience

Fixture data is allowed for tests and screenshots only when it is explicitly labeled. Production paths must use real public REST/WebSocket data, preserve exchange and receive timestamps, and avoid fabricated market fields. Proxies such as BBO-only OFI, liquidity-cost, or adverse-selection measures must be named as proxies.

### IV. Operator Safety And Observability

Long-running or live-data features must expose health, latency, reconnect, queue, data-gap, and confidence signals with low-cardinality metrics. Backpressure, unsupported output formats, invalid filters, schema drift, private-channel requests, and replay parity drift must fail loudly with actionable errors.

### V. Open-Source Reproducibility

The project must remain buildable from a clean checkout with documented commands, pinned toolchain expectations, generated screenshots, release packaging evidence, and contribution guidance. Public docs must distinguish implemented behavior, partial behavior, planned work, and explicit non-goals.

## Engineering Constraints

- Rust workspace boundaries stay explicit: shared contracts in `hls-core`, public exchange adapters in `hls-hyperliquid`, local recording/replay in `hls-store`, feature math in `hls-features`, screening in `hls-screen`, presentation in `hls-tui`, CLI orchestration in `hls-cli`, and read-only route helpers in `hls-server`.
- TUI work must keep ingestion non-blocking and preserve deterministic non-TTY output for tests and screenshots.
- Storage changes must include schema/version notes, migration behavior, replay compatibility checks, and failure modes for unsupported historical data.
- Plugin or extension work must default to no network, no filesystem, no credentials, no private account data, no state mutation, and no execution capability.
- Release work must be proven by dry-run/build evidence before docs claim installability through a published channel.

## Validation Gates

- Run the narrowest relevant focused tests first, then the full workspace gate before merge: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features`, `cargo build --workspace --all-features`, `scripts/check-release-packaging.sh`, `python3 scripts/generate-screenshots.py --check`, and `git diff --check` when those gates apply.
- Live-data claims require bounded public smoke evidence with symbol count, subscription count, WebSocket message count, normalized event count, reconnects, data gaps, and clean shutdown.
- Docs and roadmap updates must be part of the same change whenever behavior or readiness changes.

## Governance

This constitution guides Spec Kit plans, roadmap packages, and implementation reviews. Changes require a documented rationale in the relevant feature plan and must not weaken the read-only capital boundary. Any feature that touches live ingestion, storage/replay, plugins, release distribution, or operator health must pass the validation gates above or remain explicitly incomplete.

**Version**: 1.0.0 | **Ratified**: 2026-07-08 | **Last Amended**: 2026-07-08
