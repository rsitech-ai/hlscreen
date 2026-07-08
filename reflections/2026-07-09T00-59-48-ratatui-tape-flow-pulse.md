# Ratatui Tape Flow Pulse

## Intent

Continue the unified next-gen Ratatui live workstation by making the `TAPE` pane visually useful in the same way the book pane now is, without changing ingestion, ranking, recording, or read-only safety boundaries.

## Success Criteria

- `TAPE` pane renders a selected-symbol flow pulse and aggregate net-pressure bar.
- 120-column live layout keeps the tape safety label visible.
- Wider/focused layouts still show a richer flow leaderboard.
- Fixture and short public live top-10 runs exercise the actual `hls live --tui` path.

## Failure Hypotheses

1. New visual tape lines wrap in the lower right rail and hide safety context.
2. The tape starts implying private fills instead of public BBO/flow proxies.
3. Adaptive changes regress narrow pane focus or existing keyboard-state tests.

## Approach

- Add a behavior-first Ratatui test for `FLOW pulse`, `net pressure`, and the tape proxy label.
- Add a 120-column regression test after live smoke showed the safety line could be clipped by wrapping.
- Make `render_tape` pass both content height and width into `tape_lines`.
- Use shorter centered signed-flow bars and compact leader rows in constrained panels.

## Evidence

- Red test first failed on missing `FLOW pulse`.
- After implementation, live 120-column smoke showed the safety label could be clipped; added a regression test that failed on missing `Tape proxy only`.
- Compact mode fixed the failure by reserving a visible safety line and omitting the wide `ret1m/rv/spread` tape detail line only in constrained panels.
- Passed: `cargo test -p hls-tui --test ratatui_cockpit --test interactive_tui`.
- Passed: `cargo clippy -p hls-tui --all-targets --all-features -- -D warnings`.
- Passed: `cargo build -p hls-cli`.
- Passed: `cargo test --workspace --all-features`.
- Passed: `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Fixture proof rendered `FLOW pulse`, `net pressure`, and `Tape proxy only | public flow`.
- Short public live top-10 smoke completed with 10 symbols, 40 subscriptions, 246 WS messages, 517 market events, 0 reconnects, and 0 data gaps.

## Reuse

For future Ratatui polish, validate both panel height and panel width. A line can pass snapshot tests at wide sizes while still wrapping in the live 120-column lower rail.
