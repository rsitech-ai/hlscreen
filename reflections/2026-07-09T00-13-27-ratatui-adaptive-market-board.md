# Ratatui Adaptive Market Board Reflection

## Intent

Make the richer Ratatui watchlist actually adaptive at common terminal widths. The previous market-board slice added rank, movement, flow, depth, and quality badges, but live 120-column output showed values getting clipped. This slice makes the watchlist choose a compact column set when its panel is narrow.

## Success Criteria

- Preserve the unified Ratatui `hls live --tui` path.
- Keep the change presentation-only and read-only.
- In medium 120-column layouts, show compact headers and full movement strings such as `UP+0.57%` and `DN-1.23%`.
- Keep the wider board capable of showing the full rank/flow/depth column set.
- Verify with focused TUI tests, workspace tests, clippy, fixture smoke, and bounded public live smoke.

## Failure Hypotheses

- A fixed richer table will continue clipping in the 120-column medium layout.
- Compacting too aggressively could lose important flow context.
- Width adaptation could accidentally regress the wide workstation snapshot.

## Result

Implemented width-aware watchlist rendering in `crates/hls-tui/src/ratatui_app.rs`:

- full mode uses `RANK`, `PRICE`, `FLOW30`, and `DEPTH`;
- compact mode uses `RK`, `PX`, and `FLOW`, omitting depth to preserve full movement text;
- compact price formatting avoids wasting column width on decimals that do not fit.

Validation passed:

- `cargo fmt -p hls-tui --check`
- `cargo test -p hls-tui --test ratatui_cockpit --test interactive_tui`
- `cargo clippy -p hls-tui --all-targets --all-features -- -D warnings`
- `cargo build -p hls-cli`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- scoped `git diff --check`
- 120-column fixture TUI smoke
- 120-column public live top-10 8s smoke: 10 symbols, 40 subscriptions, 235 WS messages, 485 market events, 0 reconnects, 0 data gaps
