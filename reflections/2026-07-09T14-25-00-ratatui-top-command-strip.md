# Ratatui Top Command Strip

## Success Criteria

- The wide Ratatui cockpit exposes a top command strip similar to a market terminal: watchlist, portfolio-risk/status, search, help, and quit.
- The portfolio label stays explicitly read-only/proxy so it does not imply wallet, positions, or execution.
- The strip reuses existing keyboard actions and does not add new command semantics.
- Validation covers the public snapshot renderer and the live `--tui` fixture path before pushing.

## Failure Hypotheses

1. Header copy becomes too long and clips core status information.
2. A portfolio label could imply wallet/private-account data unless it is marked as a read-only risk/proxy surface.
3. A cosmetic strip could drift from real keyboard actions if it names unsupported keys.

## Candidate Approaches

- Add the strip to the existing status header line on sufficiently wide terminals, avoiding fixed-height header changes.
- Keep medium/narrow terminals on the existing compact key rails to avoid wrapping.

## Execution Log

- Starting with a wide snapshot test for the command strip.
- Red check confirmed the strip was missing: `cockpit_header_renders_terminal_top_command_strip` failed on missing `TOP BAR`.
- Added a wide-only status-header strip with existing hotkeys: `WATCHLIST [1]`, `PORTFOLIO RISK [6]`, `SEARCH [/]`, `HELP [?]`, and `QUIT [q]`.
- Kept the portfolio surface labeled `read-only proxy` to avoid implying wallet/private position data.
- Validation passed:
  - `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit cockpit_header_renders_terminal_top_command_strip -- --nocapture`
  - `cargo fmt --check`
  - `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction --test ratatui_cockpit -j 1`
  - `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`
  - `cargo test -p hls-cli --test live_mock`
  - fixture `hls live --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --once --tui --color never`
  - `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`

## Closeout

- The wide cockpit now has a market-terminal-style top command strip while preserving the existing fixed-height header and adaptive lower layouts.
- No runtime action semantics, exchange access, wallet state, private streams, or execution paths changed.
