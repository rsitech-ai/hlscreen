# Changelog

All notable changes will be recorded here.

This project follows a practical changelog format and intends to use semantic versioning after the first public release.

## Unreleased

### Added

- Public open-source readiness package: license, contribution guide, support/security/conduct docs, GitHub templates, CI, dependabot, release checklist, examples, and screenshots.

## 0.1.0 - 2026-07-08

### Added

- Read-only Rust workspace for Hyperliquid spot screening and local recording.
- Public REST metadata parsing for `spotMeta` and `spotMetaAndAssetCtxs`.
- Public WebSocket fixture parsing for trades, BBO, all-mids, active asset context, and candles.
- Fixture-backed live screen, record, replay, screen rules, and health commands.
- Compressed raw public message recording, normalized replay JSONL, and local SQLite metadata registry.
- Deterministic screening DSL and built-in presets.
- Health snapshots, reconnect simulation, TUI health rendering, and read-only local API helpers.
- Pre-merge audit report and regression fixes for safety/correctness findings.
