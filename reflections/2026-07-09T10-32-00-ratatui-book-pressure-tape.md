# Ratatui Book Pressure Tape

## Success Criteria
- The focused/default book pane exposes a reference-style pressure tape with bid/ask share, notional bars, queue skew, and read-only top-book wording.
- The change costs no extra vertical space, so medium and narrow layouts keep their existing adaptive coverage.
- Existing book/quote/read-only behavior remains unchanged.

## Failure Hypotheses
- Adding another book row would push current book diagnostics out of constrained panes.
- Renaming the book snapshot could break existing snapshot contracts that expect `BOOK SNAP`.
- Wider pressure text could wrap in medium terminals and make the pane feel worse.

## Candidate Approaches
- Add a new pressure tape row to the book pane.
- Upgrade the existing `BOOK SNAP` line with a `PRESSURE TAPE` marker and queue-skew context.
- Move the pressure tape into expanded-book only.

## Chosen Approach
Upgrade the existing book snapshot lines. It preserves row count and strengthens the visible book panel in the normal cockpit path.

## Validation
- `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit book_pane_renders_pressure_tape -- --nocapture` failed before implementation on missing `PRESSURE TAPE`.
- First implementation used an overlong leading marker and the full Ratatui suite caught clipping of `DEPTH CONSOLE`, `ask share`, and narrow book `imbalance`.
- Tightened implementation keeps `BOOK SNAP DEPTH CONSOLE` unchanged and moves `PRESSURE TAPE queue skew ...` onto the existing queue-map line.
- `cargo fmt --check` passed.
- `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction --test ratatui_cockpit -j 1` passed.
- `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture` passed.
- `cargo test -p hls-cli --test live_mock` passed.
- Fixture TUI smoke with `hls live --fixture-file ... --once --tui --color never` passed and found `PRESSURE TAPE`.
- `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings` passed.

## Closeout
The book pane now has a pressure-tape label and queue-skew readout in the visible cockpit path while preserving existing bid/ask share, depth-console, compact drilldown, and read-only top-book behavior.
