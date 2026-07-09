# Reflection: Ratatui Arrow Pane Cycling

## Success Criteria

- Left and Right arrow keys cycle pane focus, matching common TUI operator expectations.
- Existing up/down row navigation remains unchanged.
- Command editor capture does not treat Left/Right as command text.
- Visible help and README mention arrow pane cycling alongside `[` and `]`.
- The change is display-only and does not touch market-data, wallet, private stream, or order-route code.

## Failure Hypotheses

- Left/Right could conflict with command editing if not scoped outside active command mode.
- Help copy could imply arrows always move panes while up/down still act on rows and pane-native scroll.
- Tests could cover key mapping only but miss documentation drift.

## Candidate Approaches

- Map Left/Right directly to previous/next pane.
- Map Left/Right to chart windows only when chart is focused.
- Keep only `[` and `]` and rely on docs.

## Chosen Approach

Map Left/Right to previous/next pane only when no command editor is open. Pane-native changes remain on existing keys: up/down/j/k for row/pane movement, tab for views, and `t` for chart windows.

## Validation

- Red check: `cargo test -p hls-cli live_tui_control_keys_map_to_screen_actions -j 1 -- --nocapture` failed before implementation because `Right` did not map to `NextPane`.
- Focused green checks: `cargo test -p hls-cli live_tui_control_keys_map_to_screen_actions -j 1 -- --nocapture`; `cargo test -p hls-tui --test ratatui_cockpit help_overlay -j 1 -- --nocapture`.
- Layout regressions found and fixed: long help lines clipped `z zoom/grid` and the read-only boundary on 140x40 and 72x24 renders.
- Broad checks: `cargo fmt --check`; `git diff --check`; `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit -j 1`; `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction -j 1`; `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`; `CARGO_INCREMENTAL=0 cargo test -p hls-cli --test live_mock -j 1`; `cargo build -p hls-cli`; `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`.
- Fixture smoke: `./target/debug/hls live --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --once --tui --color always` emitted truecolor Ratatui output with `layout narrow 80x24`, `resize-safe`, `ALGO SCAN`, `DETAIL`, `BBO`, and read-only markers.
