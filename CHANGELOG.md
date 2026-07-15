# Changelog

All notable changes will be recorded here.

This project follows a practical changelog format and intends to use semantic versioning after the first public release.

## Unreleased

0.1.0 is the intended first public release and has not been published. The release date will be added only in the reviewed release commit.

### Added

- Public open-source readiness package: license, contribution guide, support/security/conduct docs, GitHub templates, CI, dependabot, release checklist, examples, and screenshots.
- Bounded public WebSocket live mode with heartbeat pings, all-symbol subscription budgeting, raw/normalized recording, replay verification, and a 15-minute live smoke report.
- Adaptive Ratatui workstation with keyboard/mouse navigation, differential rendering, resize-aware layouts, display-only pause, terminal diagnostics, and real PTY lifecycle coverage.
- Read-only Rust workspace for Hyperliquid spot screening and local recording.
- Public REST metadata parsing for `spotMeta` and `spotMetaAndAssetCtxs`.
- Public WebSocket fixture parsing for trades, BBO, all-mids, active asset context, and candles.
- Fixture-backed live screen, record, replay, screen rules, and health commands.
- Compressed raw public message recording, normalized replay JSONL, and local SQLite metadata registry.
- Deterministic screening DSL and built-in presets.
- Health snapshots, reconnect simulation, TUI health rendering, and read-only local API helpers.
- Pre-merge audit report and regression fixes for safety/correctness findings.

### Changed

- Hardened live ingestion with finite numeric validation, out-of-order state protection, bounded histories, market-data inactivity detection, rolling reconnect subscription limits, and confidence-aware gaps.
- Made recording identities and file registration append-only and path-safe, including symlink-aware replay containment.
- Made `--color always` override an inherited `NO_COLOR` value while `--color auto` continues to honor terminal environment policy.
