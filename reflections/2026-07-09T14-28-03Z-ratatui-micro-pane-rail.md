# Reflection: Ratatui Micro Pane Rail

## Success Criteria

- The sub-20-row micro layout exposes visible pane focus controls in addition to the command rail.
- Mouse clicks on micro pane labels focus watchlist, detail, chart, book, tape, and status panes.
- Clicking the already-active micro pane label toggles zoom, matching the rest of the workstation.

## Failure Hypotheses

- The micro header line could become too long for 80-column terminals.
- Pane click mapping could steal command rail clicks because both live on the same row.
- Active-pane zoom behavior could diverge from standard header pane rails.

## Candidate Approaches

- Add a third micro header line, but that would reduce body space on already short terminals.
- Append a compact `PANES 1W 2D 3C 4B 5T 6S` rail to the existing command line.

## Chosen Approach

Append a compact pane rail to the existing micro command line and route only the pane label columns through the same focus-or-zoom behavior used by the rest of the TUI.

## Validation

- `cargo fmt --check`
- `git diff --check`
- `cargo test -p hls-tui --test ratatui_cockpit micro_cockpit -j 1 -- --nocapture`
- `cargo test -p hls-cli live_tui_mouse_clicks_visible_command_controls -j 1 -- --nocapture`
- `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit -j 1`
- `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction -j 1`
- `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`
- `CARGO_INCREMENTAL=0 cargo test -p hls-cli --test live_mock -j 1`
- `cargo build -p hls-cli`
- `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`
- `./target/debug/hls live --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --once --tui --color always --data-dir <tmp>` fixture smoke: `fixture tui smoke ok 24 lines`
