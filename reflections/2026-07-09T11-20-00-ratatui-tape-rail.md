# Ratatui Tape Rail

## Success Criteria
- The tape pane exposes a visible `TAPE RAIL` marker in the normal cockpit path.
- Existing selected-flow, flow pulse, public-trades, and read-only/no-fill semantics remain visible.
- The change adds no rows and preserves compact/narrow behavior.

## Failure Hypotheses
- A longer selected-flow line could clip the selected symbol in narrow focused panes.
- Moving tape labels into trade-only branches would disappear on fixture/live frames without recent prints.
- Overusing trade language could imply private fills or execution; wording must stay public/read-only.

## Candidate Approaches
- Add a new tape header row.
- Prefix the existing selected-flow line with `TAPE RAIL`.
- Only mark the trade-list header as a rail when public prints are available.

## Chosen Approach
Prefix the existing selected-flow line. It gives the pane a stronger terminal identity in all live states without changing vertical layout.

## Validation
- `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit tape_pane_renders_flow_pulse_and_net_pressure_bars -- --nocapture` failed before implementation on missing `TAPE RAIL`.
- Added `TAPE RAIL` to the existing selected-flow line only, preserving row count and `Selected flow`.
- `cargo fmt --check` passed.
- `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction --test ratatui_cockpit -j 1` passed.
- `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture` passed.
- `cargo test -p hls-cli --test live_mock` passed.
- Fixture TUI smoke with `hls live --fixture-file ... --once --tui --color never` passed and found `TAPE RAIL`.
- `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings` passed.

## Closeout
The default tape pane now reads as an explicit trade-tape rail in both flow-leaderboard and public-print states while keeping public-only/no-fill semantics intact.
