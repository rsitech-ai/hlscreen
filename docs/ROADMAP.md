# Roadmap

`hlscreen` is intentionally scoped as read-only public market-data
infrastructure. The roadmap separates implemented local capabilities from
production operations and public-release proof.

## Current V1

- Public Hyperliquid REST metadata and candle-snapshot adapters.
- Public WebSocket parsing for trades, BBO, selected-symbol L2, all-mids,
  active asset context, and candles.
- Bounded live mode with heartbeat, reconnect/resubscribe, explicit data gaps,
  fail-closed recording backpressure, and all-symbol subscription budgeting.
- Compressed raw capture, normalized JSONL, SQLite metadata, and
  schema-versioned analytical Parquet event/feature export and event replay.
  A tested candle-cache adapter exists but is not wired into the current CLI or
  TUI.
- Deterministic replay, confidence parity, feature calculations, screening DSL,
  fee-profile assumptions, research metric/proxy labels, and local analog search.
- Adaptive Ratatui workstation with PTY cleanup tests, keyboard/mouse navigation,
  resize-aware layouts, persisted display preferences, and deterministic captures.
- Local-only alert playbooks, evaluation, cooldown suppression, JSONL history,
  and bounded keyboard-navigable TUI alert history.
- Read-only localhost HTTP routes, including an operator-terminated static loop
  and a bounded live-data preview. These are not a supported production service.
- Read-only Wasm extension contracts that deny network, filesystem, private-data,
  and trading permissions, plus a bounded worker ownership prerequisite that is
  not yet enabled in live rendering.
- Hardened cargo-dist candidate packaging with PR artifact builds, source
  archive, checksums, CycloneDX SBOM, auditable binaries, SHA-pinned actions,
  least-privilege permissions, cache/container and shell-injection defenses,
  automated workflow security scanning, and tag-only provenance; publication
  is unproven.
- Manual and opt-in live-closeout public candle gap coverage with durable
  partial/unrepaired evidence; original trade/BBO gaps remain degraded.

## Release Status

**Draft/local proof only.** Source builds, local archives, checksums, unpacked
binary smoke tests, and tag-gated workflow configuration exist. There is no reviewed `v*` release artifact publication, so public binary installation is not yet claimed.

## Next Candidate Slices

1. Runtime integration truth.
   - Wire configuration precedence beyond `doctor`, or keep generated config
     explicitly advisory.
   - Integrate the tested market-composite and candle-cache adapters only with
     user-visible coverage, provenance, and failure-state tests.
2. Reconnect recovery evidence.
   - Add a fault-injected reconnect acceptance run around the implemented
     opt-in coarse public candle closeout path.
   - Keep missing trades/BBO unrepaired and preserve degraded confidence.
   - Evaluate delayed public archives only as offline best-effort research data.
3. Production service lifecycle.
   - Define supported configuration, persistence/recovery, authentication,
     resource limits, graceful restart, upgrade, rollback, and incident handling.
   - Validate supervisor templates before describing them as deployment support.
4. Alert operations.
   - Add explicit scheduling, delivery, deduplication, ownership, retention, and
     escalation semantics without introducing exchange actions.
5. Evidence quality.
   - Validate canonical metric definitions against research references and data
     sufficiency requirements.
   - Replace file-backed analog search with an indexed service only when scale
     evidence requires it.
6. Release and soak proof.
   - Run multi-hour and multi-day supervised public-data soaks with CPU, memory,
     latency, reconnect, gap, and replay-parity evidence.
   - Review a `v*` tag workflow, artifacts, checksums, clean-runner installation,
     and release notes before checking publication boxes.

## Explicitly Out Of Scope

- Trading execution or automated strategy recommendations.
- Wallet integration, signing, or order endpoints.
- Private account streams, fee-tier lookup, or realized-fill modeling.
- Profitability claims.
- Silent fallback from live data to fixtures or mocks.
