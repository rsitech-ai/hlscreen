# Reflection: Ratatui Watchlist Context Rail

## Success Criteria

- Standard-wide layouts render selected-row context under the watchlist even when the left column is narrower than the expanded watchlist layout.
- The context rail surfaces row router, scanner leaders, and read-only action context without requiring pane expansion.
- Narrow/short layouts remain compact and do not lose the primary watchlist.
- The change is display-only and uses existing screened public rows.

## Failure Hypotheses

- Lowering the width threshold could crowd the standard-wide watchlist table.
- Long context text could wrap too aggressively in the left column.
- Existing narrow layouts could start rendering too much context if the height guard is too loose.

## Candidate Approaches

- Lower the row-router width threshold for non-compact watchlist panes.
- Add a separate left-column mini detail pane.
- Only render row context when the watchlist is expanded.

## Chosen Approach

Lower the row-router threshold for non-compact panes while keeping the existing height guard. This fills otherwise blank vertical space in standard-wide layouts without changing input behavior or market-data semantics.

## Validation

- Red check: `cargo test -p hls-tui --test ratatui_cockpit standard_wide_watchlist_keeps_row_context_rail -j 1 -- --nocapture` failed before implementation because `ROW ROUTER` was absent at `220x56`.
- Focused green checks: `cargo test -p hls-tui --test ratatui_cockpit standard_wide_watchlist_keeps_row_context_rail -j 1 -- --nocapture`; `cargo test -p hls-tui --test ratatui_cockpit wide_watchlist_renders_dynamic_scanner_rail -j 1 -- --nocapture`.
- Broad checks: `cargo fmt --check`; `git diff --check`; `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit -j 1`; `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction -j 1`; `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`; `CARGO_INCREMENTAL=0 cargo test -p hls-cli --test live_mock -j 1`; `cargo build -p hls-cli`; `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`.
- Fixture smoke: `./target/debug/hls live --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --once --tui --color always` emitted truecolor Ratatui output with `layout narrow 80x24`, `resize-safe`, `ALGO SCAN`, `DETAIL`, `BBO`, and read-only markers.
