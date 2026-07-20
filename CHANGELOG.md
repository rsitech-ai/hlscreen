# Changelog

All notable changes will be recorded here.

This project follows a practical changelog format and intends to use semantic versioning after the first public release.

## Unreleased

## 0.1.1 - 2026-07-20

### Changed

- Removed development-assistant working material from the repository and its
  published Git history: maintainer plan/memory/task journals and their
  directories (`memory/`, `plans/`, `reflections/`), assistant lesson stores
  and planning notes (`docs/agent-memory/`, `docs/superpowers/`), vendored
  Spec Kit workflow tooling (`.specify/`, `.agents/`, `specs/`,
  `third_party/spec-kit/`), and internal release dossiers and session audit
  reports (`docs/reports/` and the internal open-source audit and checklist
  documents). Product source, tests, fixtures, public documentation, and
  release evidence are unchanged.
- Regenerated `THIRD_PARTY_LICENSES.txt` and `THIRD_PARTY_NOTICES.md` without
  the removed vendored Spec Kit attribution; all Rust dependency licenses and
  packaged notices are preserved.
- Signed the Apple Silicon release binary with the maintainer's
  `Developer ID Application` identity (Team `2NY8A789TN`) with hardened
  runtime. The archive is still not notarized, and documentation states that
  boundary explicitly.
- Retired the internal hosted-surface publication gate scripts; the public
  readiness, history secret-scan, packaging, and artifact smoke gates remain.

## 0.1.0 - 2026-07-20

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
- Schema-versioned event and feature/confidence Parquet exports with manifests,
  local analog indexes, bounded local alert history, and supervisor smoke checks.
- A complete help contract for every public CLI command and option.
- Canonical RSI Tech organization ownership and public release metadata.
- Apache-2.0 project licensing, Rafal Sikora copyright attribution, and RSI
  Tech maintainer, website, and contact metadata.

### Changed

- Normalized public human Git history to the approved Rafal Sikora GitHub
  no-reply identity while preserving GitHub system committer metadata.
- Documented the Apple Silicon binary's linker-generated ad hoc signature
  without implying Developer ID signing or notarization.
- Hardened live ingestion with finite numeric validation, out-of-order state protection, bounded histories, market-data inactivity detection, rolling reconnect subscription limits, and confidence-aware gaps.
- Disabled public REST redirects, capped successful REST bodies at 8 MiB, and
  limited cleartext WebSocket URLs to loopback fixtures.
- Replaced release-workflow pipe-to-shell installers with version-locked Cargo
  registry builds and bound soak evidence to exact runtime-source and binary
  SHA-256 digests.
- Kept the read-only safety and quit controls visible in short help overlays.
- Made recording identities and file registration append-only and path-safe, including symlink-aware replay containment.
- Made `--color always` override an inherited `NO_COLOR` value while `--color auto` continues to honor terminal environment policy.
