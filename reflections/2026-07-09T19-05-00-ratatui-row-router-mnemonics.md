# Ratatui Row Router Mnemonics

## Success Criteria

- The selected-row router teaches the same mnemonic pane focus keys as the top-bar desk nav.
- The expanded watchlist command center exposes row movement, detail, chart, book, tape, ops/status, and command editors without suggesting execution.
- README TTY controls document the mnemonic pane focus keys for users running from `main`.

## Failure Hypotheses

1. Adding too much row-router copy causes wide watchlist text to wrap or displace scanner context.
2. README and rendered help diverge from actual key handling.
3. New command copy accidentally sounds like order/execution control rather than read-only navigation.

## Candidate Approaches

- Update only copy/tests in existing row-router and watchlist command-center surfaces.
- Add another command panel. Rejected for this slice because the watchlist already has a row router and command center.

## Evidence

- Red: `wide_watchlist_renders_selected_row_router_strip` failed at missing `c/3 chart`; `expanded_watchlist_renders_command_center_deck` failed at missing `hotkeys j/k ent tab w/i/c/b/r/o`.
- Iteration: moving `o/6 ops` onto the second row avoided clipping in the selected-row router.
- Green focused: both watchlist row-router and expanded command-center tests pass.
- Green broad: `cargo fmt --check`; `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction --test ratatui_cockpit -j 1`; `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`; `cargo test -p hls-cli --test live_mock`; `cargo build -p hls-cli`; fixture `hls live --once --tui --color always` smoke; `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`; `git diff --check`.
