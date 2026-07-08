# Ratatui Status Drilldown Reflection

## Success Criteria
- Narrow terminal status focus renders a real operational pane instead of falling back to the generic detail pane.
- Status content exposes stream state, recorder state, health counters, layout mode, controls, and read-only safety boundaries.
- Change stays presentation-only: no wallet, private stream, order route, ingestion, or recording behavior changes.

## Failure Hypotheses
- The `status` pane hotkey can focus an invisible pane on narrow layouts, confusing keyboard users.
- Health/status text could be available only in the footer, making it easy to clip on constrained terminals.
- A presentation fix could accidentally alter market-data or command-palette behavior if it touches shared state.

## Candidate Approaches
- Route `WorkstationPane::Status` in narrow drilldown mode to a dedicated operational panel.
- Inflate the existing footer with more status text.
- Reuse the generic detail pane and add status rows there.

## Attempt Log
- Chose the dedicated operational panel because it preserves the existing footer while making pane focus honest.
- Added a deterministic narrow snapshot test that focuses `WorkstationPane::Status` and asserts `[FOCUS] STATUS`, stream, recorder, health counters, pane label, and read-only safety text.
- Implemented `render_status_panel` and routed narrow status focus to it.

## Verification
- Passed: `cargo test -p hls-tui --test ratatui_cockpit narrow_cockpit_renders_status_focus_as_operational_drilldown -- --nocapture`.
- Passed: `cargo test -p hls-tui --test ratatui_cockpit --test interactive_tui`.
- Passed: `cargo clippy -p hls-tui --all-targets --all-features -- -D warnings`.
- Passed: `cargo build -p hls-cli`.
- Passed fixture smoke: `COLUMNS=120 LINES=36 ./target/debug/hls live --symbols @107 --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --metadata-file tests/fixtures/microstructure/metadata_enrichment.json --once --tui`.
- Passed public live smoke: `COLUMNS=120 LINES=36 ./target/debug/hls live --top 10 --duration-secs 8 --refresh-secs 2 --tui` with 10 symbols, 40 subscriptions, 211 WS messages, 461 market events, 0 reconnects, and 0 data gaps.
- Full `cargo test --workspace --all-features` is currently blocked by unrelated untracked `crates/hls-cli/tests/alerts_command.rs` tests expecting an `alerts` subcommand that is not wired in this branch.

## Closeout
- Dedicated status focus is now truthful in narrow adaptive layouts.
- The slice should be reusable as the place for future recorder/reconnect/latency diagnostics without bloating the footer.
