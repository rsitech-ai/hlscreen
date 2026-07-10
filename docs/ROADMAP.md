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
- Compressed raw capture, normalized JSONL, SQLite metadata, candle cache, and
  schema-versioned analytical Parquet export/replay.
- Deterministic replay, confidence parity, feature calculations, screening DSL,
  fee-profile assumptions, research metric/proxy labels, and local analog search.
- Adaptive Ratatui workstation with PTY cleanup tests, keyboard/mouse navigation,
  resize-aware layouts, persisted display preferences, and deterministic captures.
- Local-only alert playbooks, evaluation, cooldown suppression, and JSONL history.
- Read-only localhost HTTP routes, including an operator-terminated static loop
  and a bounded live-data preview. These are not a supported production service.
- Read-only Wasm extension contracts that deny network, filesystem, private-data,
  and trading permissions.
- Draft cargo-dist packaging and local archive/checksum smoke tests.

## Release Status

**Draft/local proof only.** Source builds, local archives, checksums, unpacked
binary smoke tests, and tag-gated workflow configuration exist. There is no reviewed `v*` release artifact publication, so public binary installation is not yet claimed.

## Next Candidate Slices

1. Reconnect recovery integration.
   - Invoke the coarse public candle adapter automatically after recorded gaps.
   - Keep missing trades/BBO unrepaired and preserve degraded confidence.
   - Add fault-injected reconnect acceptance evidence.
2. Production service lifecycle.
   - Define supported configuration, persistence/recovery, authentication,
     resource limits, graceful restart, upgrade, rollback, and incident handling.
   - Validate supervisor templates before describing them as deployment support.
3. Alert operations.
   - Add explicit scheduling, delivery, deduplication, ownership, retention, and
     escalation semantics without introducing exchange actions.
4. Evidence quality.
   - Validate canonical metric definitions against research references and data
     sufficiency requirements.
   - Add private fee-tier and realized-fill modeling only behind an explicit new
     trust boundary; keep v1 public/read-only.
   - Replace file-backed analog search with an indexed service only when scale
     evidence requires it.
5. Release and soak proof.
   - Run multi-hour and multi-day supervised public-data soaks with CPU, memory,
     latency, reconnect, gap, and replay-parity evidence.
   - Review a `v*` tag workflow, artifacts, checksums, clean-runner installation,
     and release notes before checking publication boxes.

## Explicitly Out Of Scope

- Trading execution or automated strategy recommendations.
- Wallet integration, signing, or order endpoints.
- Profitability claims.
- Silent fallback from live data to fixtures or mocks.
