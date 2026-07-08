# Ratatui Book And Tape Depth Slice

## Intent

Move the workstation closer to a real trading terminal by making the right-side `BOOK` and `TAPE` panes carry more useful microstructure information. The current state does not expose full L2 depth or individual recent trades through `FeatureSnapshot`, so this slice must stay honest: use top-of-book, flow, BBO OFI, imbalance, spread, tradeability, resilience, and score fields already available.

## Success Criteria

- The book pane shows bid/ask price, size, notional, spread, depth, imbalance, and read-only top-of-book/proxy caveat.
- The tape pane shows selected-market flow plus ranked flow/OFI rows from screened rows.
- No synthetic L2 levels or fake trades are introduced.
- Snapshot tests prove the new labels render from fixture-backed snapshots.
- Live ingestion, recording, subscriptions, and safety boundaries remain unchanged.

## Failure Hypotheses

- Adding too much text could overflow narrow right panes.
- Using prose-heavy labels could make the pane less scannable than a trading terminal.
- If fixture data has sparse BBO/flow fields, tests should assert labels and available values without pretending missing data exists.

## Result

- Replaced the right-side `BOOK` placeholder with bid/ask price, size, notional, spread, top-of-book depth, imbalance, OFI, pressure meter, tradeability/resilience, adverse-selection proxy, and an explicit `BOOK proxy only` caveat.
- Replaced the `TAPE` placeholder with selected-symbol flow context plus a screened flow/OFI leaderboard sorted by absolute signed flow. It remains labeled as a public BBO/flow proxy, not private fills or true trade prints.
- Verification passed: `cargo fmt --check -p hls-tui`; `cargo test -p hls-tui --test ratatui_cockpit`; `cargo test -p hls-tui --test interactive_tui`; `cargo test -p hls-cli commands::live::tests::live_tui`; `cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -- -D warnings`.
- Fixture proof: `./target/debug/hls live --symbols @107 --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --metadata-file tests/fixtures/microstructure/metadata_enrichment.json --once --tui` rendered `BID`, `ASK`, `notional`, `imbalance`, `Selected flow`, `Flow leaderboard`, and `OFI`.
- Short public live top-10 smoke passed: `./target/debug/hls live --top 10 --duration-secs 8 --refresh-secs 2 --tui` exited 0 with 10 symbols, 40 subscriptions, 227 WS messages, 482 market events, 0 reconnects, and 0 data gaps.
