# Ratatui Market Board Reflection

## Intent

Move the unified `hls live --tui` Ratatui workstation closer to a real trading terminal by making the watchlist behave like a compact market board instead of a simple price table.

## Success Criteria

- Preserve the unified Ratatui TUI path and read-only safety boundary.
- Keep all signals sourced from existing `FeatureSnapshot` fields.
- Render rank, symbol, price, UP/DN movement, signed 30s flow, top-book depth, and quality badge in the watchlist.
- Keep the board useful in no-color terminals through text labels, not color alone.
- Verify with focused snapshot tests, scoped clippy, fixture smoke, and public top-10 live smoke.

## Failure Hypotheses

- Extra columns could make medium terminals unreadable.
- Direction glyphs could fail on some terminal fonts or no-color snapshots.
- A richer board could accidentally depend on new ingestion or private data.

## Result

Implemented a denser watchlist board in `crates/hls-tui/src/ratatui_app.rs` using only existing public market-data fields:

- `RANK`
- `PRICE`
- `UP` / `DN` / `FL` 1m movement labels
- `FLOW30`
- `DEPTH`
- `Q` / `T` / `!` quality badges

Validation passed:

- `cargo fmt -p hls-tui --check`
- `cargo test -p hls-tui --test ratatui_cockpit --test interactive_tui`
- `cargo clippy -p hls-tui --all-targets --all-features -- -D warnings`
- `cargo build -p hls-cli`
- `cargo test --workspace --all-features`
- scoped `git diff --check`
- fixture `hls live --symbols @107 ... --once --tui`
- public live top-10 8s smoke: 10 symbols, 40 subscriptions, 224 WS messages, 476 market events, 0 reconnects, 0 data gaps

Workspace-wide clippy is currently blocked by unrelated dirty `crates/hls-hyperliquid/tests/rest_metadata.rs` changes that import `parse_candle_snapshot` before the corresponding implementation is present.
