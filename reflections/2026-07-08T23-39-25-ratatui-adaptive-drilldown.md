# Ratatui Adaptive Drilldown Slice

## Intent

Make the unified Ratatui cockpit more adaptive without widening the market-data or safety surface. Medium terminals should keep the primary workstation panels visible, and narrow terminals should let keyboard pane focus reveal panels that cannot fit side-by-side.

## Success Criteria

- Medium-width snapshots render watchlist, microstructure, candles, book, and tape together.
- Narrow snapshots keep the default watchlist/detail collapse, but focused `BOOK` and `TAPE` panes render as the lower drilldown panel.
- Existing keyboard focus semantics stay unchanged.
- No ingestion, subscription, recording, private-stream, wallet, or order-route behavior changes.

## Failure Hypotheses

- Adding book/tape to medium layouts could starve the chart height.
- Narrow drilldown could break the existing default narrow collapse expected by tests.
- Long book/tape caveat text could wrap or clip in small terminals, so tests should prove stable labels rather than exact prose placement.

## Result

- Medium layout now splits the right-side content vertically into microstructure, chart, and compact book/tape panels.
- Narrow layout now routes the lower panel through focused pane state: chart, book, and tape can be reached by pane focus while the default focus still shows detail.
- Verification passed: `cargo fmt --check -p hls-tui`; `cargo test -p hls-tui --test ratatui_cockpit`; `cargo test -p hls-tui --test interactive_tui`; `cargo test -p hls-cli commands::live::tests::live_tui`; `cargo clippy -p hls-tui --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-features`; `cargo build --workspace --all-features`; `git diff --check`.
- Fixture proof: `./target/debug/hls live --symbols @107 --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --metadata-file tests/fixtures/microstructure/metadata_enrichment.json --once --tui` rendered the full cockpit with book and tape.
- Short public live top-10 smoke passed: `./target/debug/hls live --top 10 --duration-secs 8 --refresh-secs 2 --tui` exited 0 with 10 symbols, 40 subscriptions, 226 WS messages, 477 market events, 0 reconnects, and 0 data gaps.
- Local caveat: `cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -- -D warnings` is blocked in this dirty worktree by unrelated `hls-store/src/parquet.rs` `clippy::useless_format` errors.
