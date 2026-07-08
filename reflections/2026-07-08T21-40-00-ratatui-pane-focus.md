# Ratatui Pane Focus Slice

## Intent

Move the live Ratatui workstation closer to the requested fully interactive terminal by adding explicit pane focus. This should make the current keyboard model less flat: operators can see which pane is active, cycle panes by keyboard, and get contextual help without changing ingestion, recording, or read-only market-data boundaries.

## Success Criteria

- `WorkstationUiState` records a focused pane separately from selected market row and detail view.
- Snapshot tests prove the focused pane is visible in wide and narrow/medium renders.
- Live key handling supports pane focus cycling while command palette input keeps priority.
- No market-data subscriptions, recorder behavior, safety posture, or screen DSL semantics change.
- Validation includes focused TUI tests, CLI live key tests, clippy, formatting, fixture render, and bounded live smoke if the code path changes.

## Failure Hypotheses

- Focus state may collide with existing `Tab` view cycling and make current controls less predictable.
- Wide-only panes such as book/tape may be focused while hidden in narrow layouts, producing confusing UI.
- Styling focused panels could accidentally break no-color deterministic snapshots or narrow-width wrapping.

## Candidate Approaches

- Use a small `WorkstationPane` enum with `[`/`]` or `f` cycling while preserving `Tab` for detail views.
- Render focus as panel-title decoration and header/status metadata rather than changing layout geometry.
- Keep hidden panes focusable for state continuity but show an explicit focus label; later slices can make pane-specific actions.

## Attempt Log

- Implemented `WorkstationPane` plus `NextPane`/`PreviousPane`; chose `[` and `]` so `Tab` keeps cycling detail views.
- Rendered active pane in the header/status and applied focused border/title styling through Ratatui blocks.
- Added tests for pane cycling, command-mode priority, and focused-pane visibility in snapshots.
- One assertion that expected `[FOCUS]` inside a long chart title was too brittle because the no-color snapshot can clip title text; the stable contract is now header/status focus state plus style-level focused panels.
- Focused validation passed: `cargo fmt --check -p hls-tui -p hls-cli`; `cargo test -p hls-tui --test interactive_tui --test ratatui_cockpit`; `cargo test -p hls-cli commands::live::tests::live_tui`; `cargo clippy -p hls-tui -p hls-cli --all-targets --all-features -- -D warnings`.
- Runtime validation passed: fixture `--once --tui` rendered `pane:watchlist`, `[FOCUS] WATCHLIST`, and `focus watchlist`; short public top-10 smoke completed with 271 WS messages, 535 market events, zero reconnects, and zero gaps.
- Full workspace validation passed in the current dirty worktree: `cargo test --workspace --all-features` and `cargo build --workspace --all-features`.
