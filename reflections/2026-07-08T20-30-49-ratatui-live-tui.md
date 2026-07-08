# Reflection: Ratatui Live TUI Rewrite

## Success Criteria
- `hls live --tui` uses a single full-screen Ratatui workstation instead of the fixed-width string renderer.
- Existing public market-data features remain visible: screened watchlist, selected market detail, confidence, resilience, tradeability, BBO/depth/flow, metadata, why-ranked scoring, health counters, and read-only caveats.
- The TUI adapts to terminal size and uses real terminal color through Ratatui/Crossterm without polluting deterministic non-TTY output.

## Top Failure Hypotheses
1. Ratatui integration could break fixture/non-TTY command output that existing tests depend on.
2. Live rendering could regress ingestion by blocking the WebSocket loop.
3. The visual target could drift into invented trading data instead of showing implemented fields.

## Candidate Approaches
1. Keep patching the string renderer. Rejected because it cannot reliably provide adaptive full-screen layout, real styling, or rich keyboard UX.
2. Add a Ratatui cockpit renderer and route `--tui` through it while keeping non-TTY output deterministic. Chosen for incremental proof while preserving existing behavior.

## Attempt Log
- Added plan: `docs/superpowers/plans/2026-07-08-ratatui-live-tui.md`.
- Added `ratatui` dependency to the workspace and `hls-tui`/`hls-cli`.
- Created `hls_tui::ratatui_app` with `RatatuiFrameModel`, viewport/color settings, a test snapshot renderer, and a live frame renderer.
- Implemented wide/medium/narrow layouts with `WATCHLIST`, `MICROSTRUCTURE`/`DETAIL`, `CHART`, `BOOK`, `TAPE`, and status bar regions.
- Wired fixture `--once --tui`, live progress frames, and final live `--tui` output to the Ratatui renderer.
- Added alternate-screen lifecycle for real interactive terminal sessions.
- Routed terminal live runs through the Ratatui final render path as well, so TTY sessions no longer get Ratatui progress followed by the old string table.
- Made the Ratatui cockpit reflect existing UI state: selected row, view tabs, density, help overlay, and display pause.
- Removed the earlier legacy string-renderer ANSI color patch; color now belongs to the Ratatui path.
- Added real 1m candle history to the Ratatui frame model and replaced the synthetic chart with selected-symbol OHLC/volume rendering.
- Updated `LiveMarketState` to upsert current interval candles and bound per-symbol candle history, so live candle updates do not duplicate bars indefinitely.
- Added a live command deck: `/` opens a validated filter editor, `p` opens a preset editor, `s` opens a sort editor, and `t` cycles chart windows. These mutate display/screening state only; ingestion, recording, subscriptions, and read-only safety boundaries are unchanged.
- Added explicit force-color environment support for inherited monochrome shells: `HLS_FORCE_COLOR=1`, `CLICOLOR_FORCE`, or `FORCE_COLOR`.

## Closeout For This Slice
- Worked: focused Ratatui tests pass; full workspace tests/build pass; fixture `--once --tui` renders the new cockpit; short public `--top 10` smoke completed with 10 symbols, 40 subscriptions, 275 WS messages, 525 market events, zero reconnects, and zero gaps.
- Later candle-chart validation: fixture `--once --tui` rendered `CANDLES 1m  O 34.5000 H 35.2000 L 34.4000 C 35.0000 VOL 1200`; short public `--top 10 --duration-secs 8` smoke completed with 208 WS messages, 458 market events, zero reconnects, and zero gaps.
- Later control validation: focused TUI and CLI tests pass; fixture `--once --tui` rendered `chart:15m` and no `reserved` key text; invalid command submissions keep the prior request active and surface the validation error; forced-color fixture smoke emitted ANSI color escapes with `TERM=dumb` and `NO_COLOR=1`.
- Final gate: full workspace tests/build, clippy, release packaging, screenshot regeneration check, diff check, and short public top-10 smoke passed with 195 WS messages, 447 market events, zero reconnects, and zero gaps.
- Still incomplete against the full objective: pane focus, richer chart interaction, and deeper order-book/tape controls need follow-up; old string-renderer tests still exist for deterministic non-TTY output.
- Reuse: keep all future TUI work behind Ratatui snapshot tests plus one bounded public live smoke.
