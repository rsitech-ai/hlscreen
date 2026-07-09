# Reflection: Ratatui Standard-Wide Quote Rail

## Success Criteria

- Standard-wide layouts around 160 columns show the selected-pair quote rail in the top header, matching the trading-workstation screenshot direction.
- Ultra-wide quote rail behavior remains intact.
- Medium and narrow layouts keep their existing density contracts and do not gain extra header rows.
- The rail remains read-only public BBO/flow context; no wallet, private stream, or order-route language is introduced.

## Failure Hypotheses

- Adding the rail to all non-narrow layouts could overflow medium headers.
- Standard-wide header height may not have enough inner rows if command dock is also active.
- Tests could prove only that text exists somewhere else in the detail pane rather than specifically validating header behavior.

## Candidate Approaches

- Lower the quote rail threshold from ultra-wide to all non-narrow widths.
- Add a separate compact quote rail for medium layouts.
- Add the existing full selected quote rail only for standard-wide and ultra-wide layouts where header height already has spare room.

## Chosen Approach

Render the existing selected quote rail for standard-wide and ultra-wide headers only. Keep medium and narrow unchanged, because their header rows are already fully allocated to command, layout, and internals diagnostics.

## Validation

- Red check: `cargo test -p hls-tui --test ratatui_cockpit standard_wide_header_renders_selected_quote_rail -j 1 -- --nocapture` failed before implementation on missing `SELECTED QUOTE`.
- Focused green: `cargo test -p hls-tui --test ratatui_cockpit standard_wide_header_renders_selected_quote_rail -j 1 -- --nocapture`.
- Regression green: `cargo test -p hls-tui --test ratatui_cockpit cockpit_header_renders_selected_quote_rail -j 1 -- --nocapture`.
- `cargo fmt --check`
- `git diff --check`
- `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit -j 1`
- `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction -j 1`
- `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`
- `CARGO_INCREMENTAL=0 cargo test -p hls-cli --test live_mock -j 1`
- `cargo build -p hls-cli`
- `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`
- `./target/debug/hls live --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --once --tui --color always --data-dir <tmp>` fixture smoke: `fixture tui smoke ok 24 lines`
