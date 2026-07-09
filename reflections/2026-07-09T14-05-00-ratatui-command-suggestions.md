# Ratatui Command Suggestions

## Success Criteria

- The Ratatui command palette suggests valid targets from the current live/screened context instead of only showing static examples.
- Symbol search suggestions come from currently visible rows and stay read-only.
- Preset/filter/sort command targets expose practical operator suggestions without changing ingestion, ranking, or execution behavior.
- Validation covers the public snapshot renderer and the live `--tui` fixture path before pushing.

## Failure Hypotheses

1. Suggestions accidentally use raw fixture assumptions rather than the current screened rows.
2. Long suggestion copy makes the command popup less usable on medium terminals.
3. Command suggestions imply execution or recommendation semantics instead of display-only filtering/navigation.

## Candidate Approaches

- Add one `SMART SUGGESTIONS` line to the existing command palette, generated from `WorkstationCommandTarget` and current `RatatuiFrameModel`.
- Avoid changing command handling in `hls-cli`; this slice should improve interaction feedback only.

## Execution Log

- Starting with a snapshot test for symbol-search suggestions sourced from visible rows.
- Red check confirmed the command overlay had no dynamic suggestion rail: `command_palette_renders_live_symbol_suggestions` failed on missing `SMART SUGGESTIONS`.
- Added one command-palette suggestion line:
  - symbol suggestions from the current screened rows
  - preset suggestions from built-in read-only presets
  - filter/sort suggestions as explicit display-only examples
- Validation passed:
  - `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit command_palette_renders_live_symbol_suggestions -- --nocapture`
  - `cargo fmt --check`
  - `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction --test ratatui_cockpit -j 1`
  - `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`
  - `cargo test -p hls-cli --test live_mock`
  - fixture `hls live --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --once --tui --color never`
  - `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`

## Closeout

- Command search is now a live workstation interaction surface rather than a static command box.
- The slice does not alter market-data ingestion, ranking, execution, wallet behavior, or order-routing boundaries.
