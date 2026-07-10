# Quickstart: Advanced TUI Workstation

## Focused Validation

```bash
cargo test -p hls-tui --test ratatui_cockpit --test workstation_interaction
cargo test -p hls-cli --test live_mock
```

## Full Gate

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
python3 scripts/generate-screenshots.py --check
git diff --check
```

## Live Smoke

```bash
./target/debug/hls live --top 10 --duration-secs 10 --refresh-secs 2 --tui
```

Expected: clean shutdown, nonzero public WebSocket messages, zero render-induced data gaps, and a visible read-only status.

## Local Preferences Smoke

```bash
./target/debug/hls tui \
  --symbols @107 \
  --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson \
  --duration-secs 1 \
  --data-dir /tmp/hlscreen-tui-preferences
```

Expected: `/tmp/hlscreen-tui-preferences/tui-preferences.toml` is read if it
exists and rewritten on clean TUI closeout with display-only `view`, `density`,
and `chart_window` fields. It must not contain selected row, pause/help state,
commands, alerts, recording, ingestion, or execution configuration.
