# Ratatui Pane Hotkeys Slice

## Intent

Make the focused-pane model usable at trading-terminal speed. Pane cycling and mouse focus already exist, but direct keyboard pane focus is the faster path, especially after narrow terminals started using focus as drilldown.

## Success Criteria

- Number keys `1` through `6` focus watchlist, detail, chart, book, tape, and status.
- Number keys still type into the command palette when a filter/preset/sort editor is open.
- Ratatui header/help advertises the direct pane shortcuts.
- The change does not touch ingestion, recording, private streams, wallets, or order routes.

## Failure Hypotheses

- Digit hotkeys could steal numeric input from command entry.
- Header/help text could overflow smaller panels.
- CLI key mapping tests could pass while the Ratatui screen remains undiscoverable.

## Result

- Added direct pane hotkey mapping for `1` through `6` in live TTY input handling.
- Kept command mode priority: digits remain `CommandChar` while the command palette is open.
- Updated header/help copy to advertise `1-6` pane focus.
- Verification passed: `cargo fmt --check -p hls-tui -p hls-cli`; `cargo test -p hls-tui --test ratatui_cockpit --test interactive_tui`; `cargo test -p hls-cli commands::live::tests::live_tui`; `cargo clippy -p hls-tui --all-targets --all-features -- -D warnings`; `cargo clippy -p hls-cli --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `git diff --check`.
- Fixture proof: `./target/debug/hls live --symbols @107 --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --metadata-file tests/fixtures/microstructure/metadata_enrichment.json --once --tui` rendered the updated `keys: j/k 1-6` legend.
- Short public live top-10 smoke passed: `./target/debug/hls live --top 10 --duration-secs 8 --refresh-secs 2 --tui` exited 0 with 10 symbols, 40 subscriptions, 279 WS messages, 529 market events, 0 reconnects, and 0 data gaps.
