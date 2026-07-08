# Releasing

This project is pre-1.0. Use this checklist before tagging a public release.

## Pre-Release Checklist

1. Confirm scope and safety.
   - No wallet, private-key, signing, order, cancel, withdrawal, leverage, or exchange-action support.
   - No README, screenshot, or release note implies trading advice or profitability.
2. Run validation.
   ```bash
   cargo fmt --check
   cargo clippy --workspace --all-targets --all-features -- -D warnings
   cargo test --workspace --all-features
   cargo build --release --workspace --all-features
   git diff --check
   python3 scripts/generate-screenshots.py
   ```
3. Run fixture smokes.
   ```bash
   tmpdir="$(mktemp -d /tmp/hlscreen-release.XXXXXX)"
   ./target/debug/hls init --data-dir "$tmpdir"
   ./target/debug/hls doctor --data-dir "$tmpdir"
   ./target/debug/hls live --symbols @107 --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --preset thin_books --once
   ./target/debug/hls live --symbols @107 --duration-secs 15 --refresh-secs 5 --tui --record --raw --normalized --run-id release-live --data-dir "$tmpdir"
   ./target/debug/hls record --symbols @107 --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --raw --normalized --run-id release --data-dir "$tmpdir"
   ./target/debug/hls replay --data-dir "$tmpdir" --run-id release
   ```
4. Update public docs.
   - `README.md`
   - `CHANGELOG.md`
   - `docs/architecture.md`
   - `docs/data-format.md`
   - `docs/feature-definitions.md`
   - screenshots in `docs/assets/screenshots/`
5. Create and review a release PR.
6. Tag only after `main` is green.

## Tagging

```bash
git tag -a v0.1.0 -m "hlscreen v0.1.0"
git push origin v0.1.0
```

## Release Notes

Release notes should include:

- What changed.
- Validation run.
- Known limitations.
- Read-only safety statement.
- Upgrade notes or migration steps.

## Current Known Limitations

- Live WebSocket mode is bounded and read-only, reconnects/resubscribes after server disconnects, and records explicit data gaps. Automatic public REST backfill after a reconnect is not implemented yet.
- Long-running localhost HTTP serving is not implemented yet.
- True Parquet output is not implemented yet.
- This is not trading advice and does not execute orders.
