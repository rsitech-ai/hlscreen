# Ratatui Book Flow Mode

## Intent

Continue the unified next-gen Ratatui live workstation by making a focused pane react to existing keyboard state instead of only showing static text.

## Success Criteria

- Focusing `BOOK` and cycling to `view:flow` renders `BOOK FLOW MODE`.
- Flow mode shows top-book depth skew, bid/ask notional context, spread gate, and OFI.
- The safety boundary `Public top-book only` remains visible in medium/focused layouts.
- No wallet, private streams, signing, orders, ingestion, recording, ranking, scoring, key mapping, or screen DSL behavior changes.

## Failure Hypotheses

1. Changing BOOK rendering by view could hide the existing bid/ask share and notional behavior in overview mode.
2. Medium layouts could clip the new mode label or safety boundary.
3. Reusing the global view state might accidentally change non-BOOK panes.

## Approach

- Add one Ratatui snapshot test for focus BOOK plus `NextView`.
- Keep overview BOOK behavior unchanged.
- Make the flow mode compact-first so it fits the existing lower-right BOOK pane.

## Evidence

- Red test failed on missing `BOOK FLOW MODE`.
- Passed: `cargo test -p hls-tui --test ratatui_cockpit book_pane_flow_view_renders_depth_flow_mode`.
- Passed: `cargo fmt -p hls-tui --check`.
- Passed: `cargo test -p hls-tui --test ratatui_cockpit --test interactive_tui`.
- Passed: `cargo clippy -p hls-tui --all-targets --all-features -- -D warnings`.
- Passed: `cargo build -p hls-cli`.
- Passed: `cargo test --workspace --all-features`.
- Passed: `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Short public live top-10 smoke completed with 10 symbols, 40 subscriptions, 261 WS messages, 512 market events, 0 reconnects, and 0 data gaps while preserving the live `BOOK`, `TAPE`, `LIQUIDITY RADAR`, and `PUBLIC TRADES` panels.

## Reuse

Future pane-specific modes should reuse existing focus/view state first, then add new controls only when the current keyboard model cannot express the mode.
