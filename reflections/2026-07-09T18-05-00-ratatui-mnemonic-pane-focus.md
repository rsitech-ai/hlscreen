# Ratatui Mnemonic Pane Focus

## Success Criteria

- Operators can jump directly to each workstation pane using mnemonic keys in addition to existing `1-6` focus keys.
- Command-entry mode keeps treating those same characters as literal command input.
- Full and compact help overlays document the mnemonic pane map without adding execution/order language.

## Failure Hypotheses

1. New focus keys conflict with existing market commands.
2. Help text grows past compact terminal constraints.
3. Command-entry mode regresses by intercepting typed filter/sort/preset text.

## Candidate Approaches

- Add a small keymap extension in `key_to_workstation_action` and keep all state transitions in existing `WorkstationUiState`.
- Add a second TUI-only shortcut layer. Rejected for this slice because CLI event handling is already the public keyboard boundary.

## Evidence

- Red: `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui_control_keys_map_to_screen_actions -- --nocapture` failed at `w -> Watchlist`.
- Green focused: the same CLI keymap test passed after adding mnemonic mappings; compact/full help overlay tests passed after rendering the new key copy.
- Green broad: `cargo fmt --check`; `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction --test ratatui_cockpit -j 1`; `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`; `cargo test -p hls-cli --test live_mock`; `cargo build -p hls-cli`; fixture `hls live --once --tui --color always` smoke; `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`; `git diff --check`.
