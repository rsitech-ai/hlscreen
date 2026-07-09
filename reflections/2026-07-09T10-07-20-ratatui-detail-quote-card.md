# Ratatui Detail Quote Card

## Success Criteria
- Default overview detail panel exposes a quote-card style selected-instrument snapshot without requiring pane zoom.
- Existing quote strip and read-only safety language remain visible.
- The slice is verified through Ratatui snapshot tests plus live CLI/TUI smoke before push.

## Failure Hypotheses
- Adding rows to the detail panel hides existing factor/radar context on common terminal heights.
- Long quote text wraps poorly on medium terminals and recreates the user-visible overflow problem.
- Styling work accidentally changes read-only semantics or live command behavior.

## Candidate Approaches
- Upgrade the existing quote strip into a denser quote-card line, preserving row count.
- Add a separate multi-line quote card only in expanded detail.
- Rebalance the overview detail layout height to support more rows.

## Chosen Approach
Upgrade the existing quote strip. It is the least disruptive default-cockpit change and directly improves the screenshot-like center quote area without reducing adaptive behavior.

## Validation
- `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit detail_overview_renders_quote_card -- --nocapture` failed before implementation on missing `QUOTE CARD`.
- `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction --test ratatui_cockpit -j 1` initially caught overlong quote text hiding adaptive context; the implementation was tightened.
- `cargo fmt --check` passed.
- `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction --test ratatui_cockpit -j 1` passed.
- `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture` passed.
- `cargo test -p hls-cli --test live_mock` passed.
- Fixture TUI smoke with `hls live --fixture-file ... --once --tui --color never` passed and found `QUOTE CARD`.
- `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings` passed.

## Closeout
The visible cockpit now marks the selected instrument as a quote card while preserving the compact quote strip, pair snapshot, factor/radar context, and read-only/no-wallet boundary across tested widths.
