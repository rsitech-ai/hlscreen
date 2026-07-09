# Reflection: Ratatui Header Visual Path

## Success Criteria

- Wide and medium headers visibly report the active visual/color path.
- No-color snapshots show `VISUAL plain fallback` without ANSI escapes.
- Color snapshots show `VISUAL ansi-neon active` with ANSI styling so screenshot/debug evidence is immediate.
- Narrow headers keep their existing state/control/internals contract; micro and status focus views continue to expose compact color diagnostics.
- The change does not alter market-data ingestion, scoring, recording, wallet, or order-route boundaries.

## Failure Hypotheses

- Adding a separate header line could truncate existing adaptive layout information.
- A verbose color badge could overflow narrow terminals.
- Tests might only prove bottom status-bar diagnostics, leaving the top screenshot ambiguity unresolved.

## Candidate Approaches

- Add a new visual diagnostics line to the header.
- Reuse the existing status line and inject a compact `VISUAL ...` badge.

## Chosen Approach

Inject a compact visual-path badge into the existing header status line for medium and wide screens. Keep narrow headers untouched because their three inner rows are already required for state, controls, and internals without truncating `z:book` or `v:overview` diagnostics.

## Validation

- Red check: `cargo test -p hls-tui --test ratatui_cockpit cockpit_header_renders_adaptive_layout_profile -j 1 -- --nocapture` failed before implementation on missing `VISUAL plain fallback`.
- Focused green: `cargo test -p hls-tui --test ratatui_cockpit cockpit_header_renders_adaptive_layout_profile -j 1 -- --nocapture`.
- `cargo fmt --check`
- `git diff --check`
- `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test ratatui_cockpit -j 1`
- `CARGO_INCREMENTAL=0 cargo test -p hls-tui --test workstation_interaction -j 1`
- `CARGO_INCREMENTAL=0 cargo test -p hls-cli live_tui -- --nocapture`
- `CARGO_INCREMENTAL=0 cargo test -p hls-cli --test live_mock -j 1`
- `cargo build -p hls-cli`
- `CARGO_INCREMENTAL=0 cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -j 1 -- -D warnings`
- `./target/debug/hls live --fixture-file tests/fixtures/hyperliquid/ws_mock_live.ndjson --once --tui --color always --data-dir <tmp>` fixture smoke: `fixture tui smoke ok 24 lines`
