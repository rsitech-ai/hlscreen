# Ratatui Exec Guard Rail

## Success Criteria
- The top command strip exposes an explicit `EXEC GUARD` marker in the normal wide cockpit path.
- Existing top-bar navigation commands remain visible: watchlist, portfolio risk, search, help, quit.
- Read-only wording remains present and no execution/wallet/order route is introduced.

## Failure Hypotheses
- The wider top command strip may clip `QUIT [q]` at 240 columns.
- Rewording safety copy could weaken the existing read-only signal.
- Putting guard text in the status/action bar could crowd the neon and theme rails.

## Candidate Approaches
- Add a new header row for execution guard state.
- Add guard wording to the existing top command strip.
- Add guard wording to the bottom action strip.

## Chosen Approach
Use the existing top command strip and keep row count unchanged. This is the strongest visible location for operator safety mode without competing with market ticker/status content.

## Validation
- `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit cockpit_header_renders_terminal_top_command_strip -- --nocapture` failed before implementation on missing `EXEC GUARD`.
- Added `EXEC GUARD read-only proxy` to the ultra-wide top command strip.
- Fixture smoke at the normal 160-column output showed the top command strip is not visible there, so the guard was also added to the desk rail as `EXEC GUARD read-only`.
- `cargo fmt --check` passed.
- `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction --test ratatui_cockpit -j 1` passed.
- Rebuilt `hls-cli`; fixture TUI smoke with `hls live --fixture-file ... --once --tui --color never` passed and found `EXEC GUARD`.
- `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture` passed.
- `cargo test -p hls-cli --test live_mock` passed.
- `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings` passed.

## Closeout
The cockpit now exposes an explicit execution guard in both the wide top command strip and the normal desk rail, keeping read-only safety visible across practical terminal widths.
