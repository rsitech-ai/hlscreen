# Reflection: Ratatui Symbol Candidate Radar

## Success Criteria

- Symbol search renders live candidate rows with rank, symbol, price, spread, flow, and read-only context before Enter.
- Suggestions are derived from the visible screened rows, not mocks or private/account data.
- The compact command palette keeps the existing suggestion line and read-only boundary visible.
- The change is display-only and does not touch wallet, private stream, order, or live ingestion behavior.

## Failure Hypotheses

- Candidate rows may overflow the command popup and hide the safety boundary.
- Formatting could duplicate existing watchlist logic badly or drift from display symbols.
- Tests could assert static copy while missing whether real live row values appear.

## Candidate Approaches

- Add a symbol-only candidate radar to the full command palette.
- Replace all command suggestions with a generic table.
- Add selectable command suggestions and completion behavior.

## Chosen Approach

Add a symbol-only candidate radar to the full command palette for the first slice. This improves the operator search experience without changing command semantics or introducing a new interaction state.

## Validation

- Red check: `cargo test -p hls-tui --test ratatui_cockpit command_palette_renders_live_symbol_suggestions -j 1 -- --nocapture` failed before implementation because `CANDIDATE RADAR` was absent.
- Focused green checks: `cargo test -p hls-tui --test ratatui_cockpit command_palette_renders_live_symbol_suggestions -j 1 -- --nocapture`; `cargo test -p hls-tui --test ratatui_cockpit command_palette -j 1 -- --nocapture`.
- Broad checks: `cargo fmt --check`; `git diff --check`; `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit -j 1`; `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction -j 1`; `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`; `CARGO_INCREMENTAL=0 cargo test -p hls-cli --test live_mock -j 1`; `cargo build -p hls-cli`; `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`.
- Fixture smoke: `./target/debug/hls live --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --once --tui --color always` emitted truecolor Ratatui output with `layout narrow 80x24`, `resize-safe`, `ALGO SCAN`, `DETAIL`, `BBO`, and read-only markers.
