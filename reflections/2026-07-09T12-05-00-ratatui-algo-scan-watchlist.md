# Ratatui Algo Scan Watchlist

## Success Criteria
- The watchlist pane exposes an `ALGO SCAN` identity in the normal cockpit path.
- Existing row ranking, quality/explain suffixes, selected-row router, and read-only behavior remain intact.
- The marker appears in practical fixture output, not only ultra-wide screenshots.

## Failure Hypotheses
- The title could become too long and clip important range information on medium/compact panes.
- Existing exact watchlist title tests may need to follow the new wording.
- Adding another row would reduce table capacity; avoid changing row count.

## Candidate Approaches
- Add a new watchlist header row.
- Insert `ALGO SCAN` into the existing watchlist title.
- Rename table columns to more terminal-like labels.

## Chosen Approach
Insert `ALGO SCAN` into the existing title. It is row-count neutral and makes the first pane read as a scanner in normal live output.

## Validation
- Added snapshot coverage for `ALGO SCAN` in the main cockpit render.
- Updated the exact watchlist scroll-title expectation to `WATCHLIST 10/10 ALGO SCAN VIEW 05-10`.
- `cargo fmt` applied formatting.
- `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction --test ratatui_cockpit -j 1` passed.
- `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture` passed.
- `cargo test -p hls-cli --test live_mock` passed.
- `cargo build -p hls-cli` rebuilt the fixture-smoke binary.
- Fixture TUI smoke with `hls live --fixture-file ... --once --tui --color never` passed and found `ALGO SCAN`.
- `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings` passed.

## Closeout
The first visible pane now presents itself as an algorithmic scanner in normal live output while preserving table capacity, selected-row behavior, and read-only execution boundaries.
