# Ratatui Desk Navigation Rail

## Success Criteria

- The top command strip exposes all primary workstation panes, not just watchlist and status.
- The visible command surface teaches both numeric pane keys and mnemonic pane keys.
- Narrow terminals keep the compact focus rail discoverable without widening the layout.

## Failure Hypotheses

1. Wide top-bar text becomes too long and wraps into market internals.
2. Narrow control text loses critical commands while adding mnemonic hints.
3. The rendered shortcut labels drift away from the actual CLI keymap.

## Candidate Approaches

- Replace the wide `TOP BAR` copy with a full desk-nav rail using existing pane labels and read-only guard language.
- Add a separate navigation panel. Rejected for this slice because the header is already the global command surface and a new panel would cost vertical space.

## Evidence

- Red: `cockpit_header_renders_terminal_top_command_strip` failed because the wide top bar did not expose `DESK NAV`; `header_renders_keyboard_pane_hotkey_rail` failed because narrow controls did not expose mnemonic focus.
- Iteration: the first wide top-bar implementation clipped `HELP [?]`, then displaced `MARKET PULSE`; the final version renders the full desk nav on its own wide header line and gives wide headers one extra row.
- Green focused: `cockpit_header_renders_terminal_top_command_strip`, `header_renders_keyboard_pane_hotkey_rail`, `narrow_cockpit_collapses_to_watchlist_and_detail_without_tape`, `market_pulse_renders_pipeline_freshness_hud`, and `wide_cockpit_renders_all_primary_trading_workstation_regions`.
- Green broad: `cargo fmt --check`; `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction --test ratatui_cockpit -j 1`; `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`; `cargo test -p hls-cli --test live_mock`; `cargo build -p hls-cli`; fixture `hls live --once --tui --color always` smoke; `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`; `git diff --check`.
