# Ratatui Color Control Reflection

## Success Criteria
- `hls live --tui` can force color on terminals where auto-detection is wrong.
- `hls live --tui` can force no-color output for deterministic captures.
- Default behavior remains automatic and keeps existing `NO_COLOR`, `TERM=dumb`, and force-color environment semantics.
- The change stays presentation-only and does not alter public market-data ingestion, recording, screening, or order-safety boundaries.

## Failure Hypotheses
- Hidden env-only color behavior makes local runs look monochrome even when the Ratatui theme exists.
- Adding a color flag could affect only final snapshots but not live progress frames.
- Staging from the dirty primary worktree could accidentally include unrelated parquet or alert work.

## Attempt Log
- Added a behavior test for the color resolver before implementing the enum and resolver.
- Added `--color auto|always|never` to the live command and propagated the resolved mode through fixture snapshots, live progress frames, and final live snapshots.
- Moved progress rendering to a `LiveProgressContext` to avoid growing argument lists while carrying color mode.
- Used a clean auxiliary worktree from `ac9e66d` to keep the commit independent of unrelated dirty primary-worktree changes.

## Verification
- Passed in clean worktree: `cargo test -p hls-cli live_tui_`.
- Passed in clean worktree: `cargo clippy -p hls-cli --all-targets --all-features -- -D warnings`.
- Passed in clean worktree: `cargo build -p hls-cli`.
- Help output exposed `--color <COLOR>` with possible values `auto, always, never`.
- Fixture proof: `--color always` emitted ANSI escapes and `--color never` emitted no ANSI escapes.
- Public live proof: `COLUMNS=120 LINES=36 ./target/debug/hls live --top 10 --duration-secs 8 --refresh-secs 2 --tui --color never` completed with 10 symbols, 40 subscriptions, 256 WS messages, 513 market events, 0 reconnects, and 0 data gaps.

## Closeout
- The user now has a direct fix for shell-specific monochrome rendering: run `hls live --tui --color always`.
