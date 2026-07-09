# Ratatui Layout Director

## Success Criteria

- The live Ratatui header makes the active adaptive layout visible in wide, medium, and narrow terminal sizes.
- The layout rail names which panes are visible and which panes are available through focus/zoom instead of silently disappearing.
- The change keeps the existing read-only market workstation behavior and does not grow the fixed header height.
- Focused validation covers the Ratatui snapshot renderer plus the live CLI fixture path before pushing.

## Failure Hypotheses

1. Adding another header line clips existing controls because wide, medium, and narrow layouts already use all available inner rows.
2. The director text becomes too long for narrow terminals and hides the keyboard rail the user needs most.
3. The adaptive labels drift from the actual layout breakpoints, making the TUI explain the wrong screen state.

## Candidate Approaches

- Replace or augment the existing controls rail with layout director text so the fixed-height header remains stable.
- Add a separate status-bar row for layout state, but that risks compressing the body and making the cockpit less useful.

## Execution Log

- Starting with a public snapshot test across 240x48, 120x40, and 72x24 viewports.
- Red check confirmed the behavior was missing: `cockpit_header_renders_layout_director_across_viewports` failed on missing `LAYOUT DIRECTOR`.
- Implemented the director without increasing header height: narrow uses the title, medium uses compact desk/control rails, and very wide terminals expose visible/hidden pane labels in the desk rail.
- Validation passed:
  - `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit cockpit_header_renders_layout_director_across_viewports -- --nocapture`
  - `cargo fmt --check`
  - `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction --test ratatui_cockpit -j 1`
  - `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`
  - `cargo test -p hls-cli --test live_mock`
  - fixture `hls live --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --once --tui --color never`
  - `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`

## Closeout

- The adaptive layout state is now visible to the operator instead of being implicit in the screen width.
- Existing narrow hotkey and internals assertions stayed intact.
