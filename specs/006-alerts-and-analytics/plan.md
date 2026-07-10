# Implementation Plan: Alerts And Analytics

**Branch**: `006-alerts-and-analytics` | **Date**: 2026-07-08 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/006-alerts-and-analytics/spec.md`

## Summary

Add the next analytics layer: a local alert-evaluator prototype, local historical analog search, research metric proxies, local fee-assumption tradeability, and sandboxed read-only plugin runtime execution.

## Technical Context

**Language/Version**: Rust stable, edition 2024.

**Primary Dependencies**: Existing screen DSL, replay/store, feature engine, extension manifest models, and Wasmtime for bounded row-annotation execution.

**Storage**: Local alert event logs, analog indexes or fixture packs, metric definitions, fee profile config, plugin manifests.

**Testing**: Replay alert fixtures, analog fixtures, metric formula tests, plugin permission/runtime tests.

**Target Platform**: Local macOS/Linux terminal use.

**Project Type**: Rust CLI/TUI analytics and extension runtime feature.

**Performance Goals**: Alert evaluation and metric updates must not block live ingestion; plugin execution must have bounded time and output.

**Constraints**: No exchange actions, no private account dependency, no production-canonical metric claims, no unsafe plugin capabilities by default.

**Scale/Scope**: Local alerts and local historical data. Hosted alerts/team collaboration are out of scope.

## Constitution Check

- **Read-only public data boundary**: PASS with strict alert/plugin limits.
- **Replayable evidence before ranking**: PASS. Every output must be replay-testable.
- **Live truth over mock convenience**: PASS. Metrics must expose unavailable/proxy states.
- **Operator safety and observability**: PASS. Cooldowns, confidence, and bounded plugin execution are required.
- **Open-source reproducibility**: PASS. Fixtures and formulas are required.

## Project Structure

```text
crates/hls-core/src/
crates/hls-features/src/
crates/hls-store/src/
crates/hls-screen/src/
crates/hls-tui/src/
crates/hls-cli/src/commands/
tests/fixtures/microstructure/
docs/feature-definitions.md
docs/extensions.md
```

## Phase 0: Research Summary

- Treat alerts as local events and UI/log outputs only.
- Use local replay data for analog search before adding indexes.
- Keep implemented public-data formulas labeled as research proxies until sampling, normalization, benchmark, and operational validation justify a separate canonical contract.
- Use Wasmtime for the first bounded row-annotation runtime; keep broader plugin ecosystem work separate.

## Phase 1: Design Summary

Alert, analog, metric, fee, and plugin outputs all share the same evidence rule: replayable, confidence-aware, and read-only. The runtime must fail closed on unsafe plugin capabilities and insufficient data.

## Complexity Tracking

Plugin runtime introduces sandbox complexity and is limited to strict, tested, read-only row annotations in this slice.
